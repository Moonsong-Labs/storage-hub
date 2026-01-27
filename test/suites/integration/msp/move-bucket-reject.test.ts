import { strictEqual } from "node:assert";
import Docker from "dockerode";
import {
  assertEventPresent,
  bspThreeKey,
  bspTwoKey,
  createSqlClient,
  describeMspNet,
  type EnrichedBspApi,
  ShConsts,
  shUser,
  sleep
} from "../../../util";

await describeMspNet(
  "MSP rejects bucket move requests",
  { initialised: false, indexer: true },
  ({ before, createMsp1Api, createMsp2Api, it, createUserApi }) => {
    let userApi: EnrichedBspApi;
    let msp1Api: EnrichedBspApi;
    let msp2Api: EnrichedBspApi;
    const source = ["res/whatsup.jpg", "res/adolphus.jpg", "res/smile.jpg"];
    const destination = ["test/whatsup.jpg", "test/adolphus.jpg", "test/smile.jpg"];
    const bucketName = "reject-move-bucket";
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
    });

    it("postgres DB is ready", async () => {
      await userApi.docker.waitForLog({
        containerName: userApi.shConsts.NODE_INFOS.indexerDb.containerName,
        searchString: "database system is ready to accept connections",
        timeout: 5000
      });
    });

    it("Network launches and can be queried", async () => {
      const userNodePeerId = await userApi.rpc.system.localPeerId();
      strictEqual(userNodePeerId.toString(), userApi.shConsts.NODE_INFOS.user.expectedPeerId);

      const mspNodePeerId = await msp1Api.rpc.system.localPeerId();
      strictEqual(mspNodePeerId.toString(), userApi.shConsts.NODE_INFOS.msp1.expectedPeerId);
    });

    it("Add 2 more BSPs (3 total) and set the replication target to 2", async () => {
      // Replicate to 2 BSPs, 5 blocks to maxthreshold
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

    it("User submits 3 storage requests in the same bucket for first MSP", async () => {
      // Get value propositions form the MSP to use, and use the first one (can be any).
      const valueProps = await userApi.call.storageProvidersApi.queryValuePropositionsForMsp(
        userApi.shConsts.DUMMY_MSP_ID
      );
      const valuePropId = valueProps[0].id;

      const batchResult = await userApi.file.batchStorageRequests({
        files: source.map((src, i) => ({
          source: src,
          destination: destination[i],
          bucketIdOrName: bucketName,
          replicationTarget: 2
        })),
        mspId: userApi.shConsts.DUMMY_MSP_ID,
        valuePropId,
        owner: shUser,
        bspApis: undefined, // No BSP needed for this test
        mspApi: msp1Api
      });

      // Extract bucket ID and file keys from the batch result
      bucketId = batchResult.bucketIds[0];
      allBucketFiles.push(...batchResult.fileKeys);

      // Seal 5 more blocks to pass maxthreshold and ensure completed upload requests
      for (let i = 0; i < 5; i++) {
        await sleep(500);
        const block = await userApi.block.seal();
        await userApi.rpc.engine.finalizeBlock(block.blockReceipt.blockHash);
      }
    });

    it("MSP 2 rejects move request when indexer postgres DB is down", async () => {
      // Pause the postgres container - this preserves the state
      const docker = new Docker();
      const postgresContainer = docker.getContainer(
        userApi.shConsts.NODE_INFOS.indexerDb.containerName
      );
      await postgresContainer.pause();

      const valueProps = await userApi.call.storageProvidersApi.queryValuePropositionsForMsp(
        userApi.shConsts.DUMMY_MSP_ID_2
      );
      const valuePropId = valueProps[0].id;

      // User requests to move bucket to second MSP
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

      // Finalising the block in the BSP node as well, to trigger the reorg in the BSP node too.
      const finalisedBlockHash = await userApi.rpc.chain.getFinalizedHead();

      // Wait for BSP node to have imported the finalised block built by the user node.
      await msp2Api.wait.blockImported(finalisedBlockHash.toString());
      await msp2Api.block.finaliseBlock(finalisedBlockHash.toString());

      // Wait for the rejection response from MSP2
      await userApi.wait.waitForTxInPool({
        module: "fileSystem",
        method: "mspRespondMoveBucketRequest",
        expectedEvent: "MoveBucketRejected",
        timeout: 45000,
        shouldSeal: true
      });

      // Resume postgres
      await postgresContainer.unpause();

      await userApi.docker.waitForLog({
        containerName: userApi.shConsts.NODE_INFOS.indexerDb.containerName,
        searchString: "database system is ready to accept connections",
        timeout: 5000
      });
    });

    it("MSP 2 rejects move request when indexer data is corrupted", async () => {
      // Delete all entries from bsp_file table to corrupt the replication data
      const sql = createSqlClient();
      await sql`DELETE FROM bsp_file`;
      await sql.end();

      const valueProps = await userApi.call.storageProvidersApi.queryValuePropositionsForMsp(
        userApi.shConsts.DUMMY_MSP_ID_2
      );
      const valuePropId = valueProps[0].id;

      // User requests to move bucket to second MSP
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

      // Finalising the block in the BSP node as well, to trigger the reorg in the BSP node too.
      const finalisedBlockHash = await userApi.rpc.chain.getFinalizedHead();

      // Wait for MSP2 node to have imported the finalised block built by the user node.
      await msp2Api.wait.blockImported(finalisedBlockHash.toString());
      await msp2Api.block.finaliseBlock(finalisedBlockHash.toString());

      // Wait for the rejection response from MSP2
      await userApi.wait.waitForTxInPool({
        module: "fileSystem",
        method: "mspRespondMoveBucketRequest",
        expectedEvent: "MoveBucketRejected",
        shouldSeal: true
      });
    });
  }
);
