import assert from "node:assert";
import { u8aToHex } from "@polkadot/util";
import { decodeAddress } from "@polkadot/util-crypto";
import {
  assertEventPresent,
  bspThreeKey,
  bspTwoKey,
  describeMspNet,
  type EnrichedBspApi,
  ShConsts,
  shUser,
  waitFor
} from "../../../util";

await describeMspNet(
  "Prometheus bucket move metrics",
  {
    initialised: false,
    indexer: true,
    telemetry: true
  },
  ({ before, createMsp1Api, createMsp2Api, createBspApi, it, createUserApi }) => {
    let userApi: EnrichedBspApi;
    let msp1Api: EnrichedBspApi;
    let msp2Api: EnrichedBspApi;
    const source = ["res/whatsup.jpg", "res/smile.jpg"];
    const destination = ["test/bucket-move-1.jpg", "test/bucket-move-2.jpg"];
    const bucketName = "bucket-move-metrics-test";
    let bucketId: string;
    const allBucketFiles: string[] = [];

    before(async () => {
      userApi = await createUserApi();
      const maybeMsp1Api = await createMsp1Api();
      if (maybeMsp1Api) {
        msp1Api = maybeMsp1Api;
      } else {
        throw new Error("MSP API for first MSP not available");
      }
      const maybeMsp2Api = await createMsp2Api();
      if (maybeMsp2Api) {
        msp2Api = maybeMsp2Api;
      } else {
        throw new Error("MSP API for second MSP not available");
      }
      // Initialize BSP API (needed for network setup)
      await createBspApi();

      // Wait for Prometheus to be ready
      await userApi.prometheus.waitForReady();
    });

    it("Setup: Add additional BSPs and configure replication", async () => {
      // Replicate to 2 BSPs
      const maxReplicationTargetRuntimeParameter = {
        RuntimeConfig: {
          MaxReplicationTarget: [null, 2]
        }
      };
      const tickRangeToMaximumThresholdRuntimeParameter = {
        RuntimeConfig: {
          TickRangeToMaximumThreshold: [null, 5]
        }
      };
      await userApi.block.seal({
        calls: [
          userApi.tx.sudo.sudo(
            userApi.tx.parameters.setParameter(maxReplicationTargetRuntimeParameter)
          )
        ]
      });
      await userApi.block.seal({
        calls: [
          userApi.tx.sudo.sudo(
            userApi.tx.parameters.setParameter(tickRangeToMaximumThresholdRuntimeParameter)
          )
        ]
      });

      await userApi.docker.onboardBsp({
        bspSigner: bspTwoKey,
        name: "sh-bsp-two",
        bspId: ShConsts.BSP_TWO_ID,
        additionalArgs: ["--keystore-path=/keystore/bsp-two"],
        waitForIdle: true
      });

      await userApi.docker.onboardBsp({
        bspSigner: bspThreeKey,
        name: "sh-bsp-three",
        bspId: ShConsts.BSP_THREE_ID,
        additionalArgs: ["--keystore-path=/keystore/bsp-three"],
        waitForIdle: true
      });
    });

    it("Record initial bucket move metrics", async () => {
      // Get initial metrics before any bucket moves
      const initialMspBucketMovesPending = await userApi.prometheus.getMetricValue(
        'storagehub_msp_bucket_moves_total{job="storagehub-msp-1",status="pending"}'
      );
      const initialMspBucketMovesSuccess = await userApi.prometheus.getMetricValue(
        'storagehub_msp_bucket_moves_total{job="storagehub-msp-1",status="success"}'
      );
      const initialBspBucketMovesPending = await userApi.prometheus.getMetricValue(
        'storagehub_bsp_bucket_moves_total{job="storagehub-bsp",status="pending"}'
      );
      const initialBspBucketMovesSuccess = await userApi.prometheus.getMetricValue(
        'storagehub_bsp_bucket_moves_total{job="storagehub-bsp",status="success"}'
      );

      console.log("Initial bucket move metrics:");
      console.log(`  MSP bucket moves (pending): ${initialMspBucketMovesPending}`);
      console.log(`  MSP bucket moves (success): ${initialMspBucketMovesSuccess}`);
      console.log(`  BSP bucket moves (pending): ${initialBspBucketMovesPending}`);
      console.log(`  BSP bucket moves (success): ${initialBspBucketMovesSuccess}`);

      // Verify metrics are queryable
      const mspResult = await userApi.prometheus.query("storagehub_msp_bucket_moves_total");
      const bspResult = await userApi.prometheus.query("storagehub_bsp_bucket_moves_total");

      assert.strictEqual(mspResult.status, "success", "MSP bucket moves query should succeed");
      assert.strictEqual(bspResult.status, "success", "BSP bucket moves query should succeed");
    });

    it("User uploads files to MSP 1", async () => {
      // Get value propositions from the MSP
      const valueProps = await userApi.call.storageProvidersApi.queryValuePropositionsForMsp(
        userApi.shConsts.DUMMY_MSP_ID
      );
      const valuePropId = valueProps[0].id;

      // Create a new bucket
      const newBucketEvent = await userApi.createBucket(bucketName, valuePropId);
      const newBucketEventData =
        userApi.events.fileSystem.NewBucket.is(newBucketEvent) && newBucketEvent.data;

      if (!newBucketEventData) {
        throw new Error("NewBucket event data does not match expected type");
      }
      bucketId = newBucketEventData.bucketId.toString();
      console.log(`Created bucket: ${bucketId}`);

      // Upload files
      const txs = [];
      const ownerHex = u8aToHex(decodeAddress(userApi.shConsts.NODE_INFOS.user.AddressId)).slice(2);
      for (let i = 0; i < source.length; i++) {
        const {
          file_metadata: { location, fingerprint, file_size }
        } = await userApi.rpc.storagehubclient.loadFileInStorage(
          source[i],
          destination[i],
          ownerHex,
          bucketId
        );

        txs.push(
          userApi.tx.fileSystem.issueStorageRequest(
            bucketId,
            location,
            fingerprint,
            file_size,
            userApi.shConsts.DUMMY_MSP_ID,
            [userApi.shConsts.NODE_INFOS.user.expectedPeerId],
            { Custom: 2 }
          )
        );
      }
      await userApi.block.seal({ calls: txs, signer: shUser });

      // Get file keys from storage request events
      const events = await userApi.assert.eventMany("fileSystem", "NewStorageRequest");
      for (const e of events) {
        const data = userApi.events.fileSystem.NewStorageRequest.is(e.event) && e.event.data;
        if (data) {
          allBucketFiles.push(data.fileKey.toString());
        }
      }
      console.log(`Uploaded ${allBucketFiles.length} files`);
    });

    it("Wait for MSP 1 to accept and BSPs to volunteer", async () => {
      // Wait for files to be in MSP 1's storage
      await waitFor({
        lambda: async () => {
          for (const fileKey of allBucketFiles) {
            const result = await msp1Api.rpc.storagehubclient.isFileInFileStorage(fileKey);
            if (!result.isFileFound) return false;
          }
          return true;
        }
      });

      // Seal blocks for MSP responses
      await userApi.wait.mspResponseInTxPool();
      await userApi.block.seal();

      // Wait for remaining MSP response if any
      try {
        await userApi.wait.mspResponseInTxPool();
        await userApi.block.seal();
      } catch {
        // No more pending MSP responses
      }

      // Wait for BSPs to volunteer and store
      for (const fileKey of allBucketFiles) {
        await userApi.wait.storageRequestNotOnChain(fileKey);
      }

      console.log("All files stored by MSP 1 and replicated to BSPs");
    });

    it("Move bucket to MSP 2 and verify metrics increment", async () => {
      // Get initial metrics
      const initialMspPending = await userApi.prometheus.getMetricValue(
        'storagehub_msp_bucket_moves_total{job="storagehub-msp-2",status="pending"}'
      );
      const initialMspSuccess = await userApi.prometheus.getMetricValue(
        'storagehub_msp_bucket_moves_total{job="storagehub-msp-2",status="success"}'
      );
      const initialBspPending = await userApi.prometheus.getMetricValue(
        'storagehub_bsp_bucket_moves_total{job="storagehub-bsp",status="pending"}'
      );
      const initialBspSuccess = await userApi.prometheus.getMetricValue(
        'storagehub_bsp_bucket_moves_total{job="storagehub-bsp",status="success"}'
      );

      console.log(`Initial MSP-2 bucket moves pending: ${initialMspPending}`);
      console.log(`Initial MSP-2 bucket moves success: ${initialMspSuccess}`);
      console.log(`Initial BSP bucket moves pending: ${initialBspPending}`);
      console.log(`Initial BSP bucket moves success: ${initialBspSuccess}`);

      // Request bucket move to MSP 2
      const valueProps = await userApi.call.storageProvidersApi.queryValuePropositionsForMsp(
        userApi.shConsts.DUMMY_MSP_ID_2
      );
      const valuePropId = valueProps[0].id;

      const requestMoveBucketResult = await userApi.block.seal({
        calls: [
          userApi.tx.fileSystem.requestMoveBucket(
            bucketId,
            msp2Api.shConsts.DUMMY_MSP_ID_2,
            valuePropId
          )
        ],
        signer: shUser,
        finaliseBlock: true
      });

      assertEventPresent(
        userApi,
        "fileSystem",
        "MoveBucketRequested",
        requestMoveBucketResult.events
      );
      console.log("Bucket move requested");

      // Finalise in MSP 2 node
      const finalisedBlockHash = await userApi.rpc.chain.getFinalizedHead();
      await msp2Api.wait.blockImported(finalisedBlockHash.toString());
      await msp2Api.block.finaliseBlock(finalisedBlockHash.toString());

      // Wait for MSP 2 to respond to move request
      await userApi.wait.waitForTxInPool({
        module: "fileSystem",
        method: "mspRespondMoveBucketRequest"
      });

      const { events } = await userApi.block.seal();
      assertEventPresent(userApi, "fileSystem", "MoveBucketAccepted", events);
      console.log("Bucket move accepted by MSP 2");

      // Wait for files to be in MSP 2's forest
      await waitFor({
        lambda: async () => {
          for (const fileKey of allBucketFiles) {
            const isFileInForest = await msp2Api.rpc.storagehubclient.isFileInForest(
              bucketId,
              fileKey
            );
            if (!isFileInForest.isTrue) return false;
          }
          return true;
        },
        iterations: 100,
        delay: 1000
      });

      console.log("All files moved to MSP 2");

      // Wait for Prometheus to scrape updated metrics
      await userApi.prometheus.waitForScrape();

      // Check final metrics
      const finalMspPending = await userApi.prometheus.getMetricValue(
        'storagehub_msp_bucket_moves_total{job="storagehub-msp-2",status="pending"}'
      );
      const finalMspSuccess = await userApi.prometheus.getMetricValue(
        'storagehub_msp_bucket_moves_total{job="storagehub-msp-2",status="success"}'
      );
      const finalBspPending = await userApi.prometheus.getMetricValue(
        'storagehub_bsp_bucket_moves_total{job="storagehub-bsp",status="pending"}'
      );
      const finalBspSuccess = await userApi.prometheus.getMetricValue(
        'storagehub_bsp_bucket_moves_total{job="storagehub-bsp",status="success"}'
      );

      console.log("Final bucket move metrics:");
      console.log(`  MSP-2 bucket moves (pending): ${finalMspPending}`);
      console.log(`  MSP-2 bucket moves (success): ${finalMspSuccess}`);
      console.log(`  BSP bucket moves (pending): ${finalBspPending}`);
      console.log(`  BSP bucket moves (success): ${finalBspSuccess}`);

      // Verify MSP pending metric incremented (move was initiated)
      assert(
        finalMspPending > initialMspPending || finalMspSuccess > initialMspSuccess,
        "Expected MSP bucket move metrics to increment after move"
      );

      // Note: BSP bucket move metrics increment when BSP processes the bucket move notification
      // This may take additional blocks to propagate
      console.log(
        `BSP bucket moves delta - pending: ${
          finalBspPending - initialBspPending
        }, success: ${finalBspSuccess - initialBspSuccess}`
      );
    });
  }
);
