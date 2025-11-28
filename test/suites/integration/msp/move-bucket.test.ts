import assert, { strictEqual } from "node:assert";
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
  "MSP moves bucket to another MSP",
  { initialised: false, indexer: true },
  ({ before, createMsp1Api, createMsp2Api, it, createUserApi }) => {
    let userApi: EnrichedBspApi;
    let msp1Api: EnrichedBspApi;
    let msp2Api: EnrichedBspApi;
    const source = ["res/whatsup.jpg", "res/adolphus.jpg", "res/smile.jpg"];
    const destination = ["test/whatsup.jpg", "test/adolphus.jpg", "test/smile.jpg"];
    const bucketName = "nothingmuch-3";
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

      // Create a new bucket where all the files will be stored.
      const newBucketEventEvent = await userApi.createBucket(bucketName, valuePropId);
      const newBucketEventDataBlob =
        userApi.events.fileSystem.NewBucket.is(newBucketEventEvent) && newBucketEventEvent.data;

      if (!newBucketEventDataBlob) {
        throw new Error("NewBucket event data does not match expected type");
      }
      bucketId = newBucketEventDataBlob.bucketId.toString();

      // Seal block with 3 storage requests.
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
            {
              Custom: 2
            }
          )
        );
      }
      await userApi.block.seal({ calls: txs, signer: shUser });
    });

    it("MSP 1 receives files from user and accepts them", async () => {
      const originalRoot = await msp1Api.rpc.storagehubclient.getForestRoot(bucketId);

      // Get the events of the storage requests to extract the file keys and check
      // that the MSP received them.
      const events = await userApi.assert.eventMany("fileSystem", "NewStorageRequest");
      const matchedEvents = events.filter((e) =>
        userApi.events.fileSystem.NewStorageRequest.is(e.event)
      );
      if (matchedEvents.length !== source.length) {
        throw new Error(`Expected ${source.length} NewStorageRequest events`);
      }

      // Allow time for the MSP to receive and store the files from the user
      await waitFor({
        lambda: async () => {
          try {
            // Check if the MSP received the files.
            const fileKeys: string[] = [];

            for (const e of matchedEvents) {
              const newStorageRequestDataBlob =
                userApi.events.fileSystem.NewStorageRequest.is(e.event) && e.event.data;

              if (!newStorageRequestDataBlob) {
                throw new Error("Event doesn't match NewStorageRequest type");
              }

              const result = await msp1Api.rpc.storagehubclient.isFileInFileStorage(
                newStorageRequestDataBlob.fileKey
              );

              if (!result.isFileFound) {
                throw new Error(
                  `File not found in storage for ${newStorageRequestDataBlob.location.toHuman()}`
                );
              }

              fileKeys.push(newStorageRequestDataBlob.fileKey.toString());
            }

            allBucketFiles.push(...new Set(allBucketFiles));
            return true;
          } catch {
            return false;
          }
        }
      });

      // Seal block containing the MSP's first response.
      // MSPs batch responses to achieve higher throughput in periods of high demand. But they
      // also prioritise a fast response, so if the Forest Write Lock is available, it will send
      // the first response it can immediately.
      await userApi.wait.mspResponseInTxPool();
      await userApi.block.seal();

      // Give time for the MSP to update the local forest root.
      await waitFor({
        lambda: async () =>
          (await msp1Api.rpc.storagehubclient.getForestRoot(bucketId)).toHex() !==
          originalRoot.toHex()
      });

      // Check that the local forest root is updated, and matches th on-chain root.
      const localBucketRoot = await msp1Api.rpc.storagehubclient.getForestRoot(bucketId);

      const { event: bucketRootChangedEvent } = await userApi.assert.eventPresent(
        "providers",
        "BucketRootChanged"
      );
      const bucketRootChangedDataBlob =
        userApi.events.providers.BucketRootChanged.is(bucketRootChangedEvent) &&
        bucketRootChangedEvent.data;
      if (!bucketRootChangedDataBlob) {
        throw new Error("Expected BucketRootChanged event but received event of different type");
      }

      strictEqual(bucketRootChangedDataBlob.newRoot.toString(), localBucketRoot.toString());

      // The MSP should have accepted exactly one file.
      // Register how many were accepted in the last block sealed.
      const acceptedFileKeys: string[] = [];
      const mspAcceptedStorageRequestEvents = await userApi.assert.eventMany(
        "fileSystem",
        "MspAcceptedStorageRequest"
      );
      for (const e of mspAcceptedStorageRequestEvents) {
        const mspAcceptedStorageRequestDataBlob =
          userApi.events.fileSystem.MspAcceptedStorageRequest.is(e.event) && e.event.data;
        if (mspAcceptedStorageRequestDataBlob) {
          acceptedFileKeys.push(mspAcceptedStorageRequestDataBlob.fileKey.toString());
        }
      }
      assert(
        acceptedFileKeys.length === 1,
        "Expected 1 file key accepted in first block after storage requests"
      );

      // An additional block needs to be sealed to accept the rest of the files.
      // There should be a pending transaction to accept the rest of the files.
      await userApi.wait.mspResponseInTxPool();
      await userApi.block.seal();

      // Give time for the MSP to update the local forest root.
      await waitFor({
        lambda: async () =>
          (await msp1Api.rpc.storagehubclient.getForestRoot(bucketId)).toHex() !==
          localBucketRoot.toHex()
      });

      // Check that the local forest root is updated, and matches th on-chain root.
      const localBucketRoot2 = await msp1Api.rpc.storagehubclient.getForestRoot(bucketId);

      const { event: bucketRootChangedEvent2 } = await userApi.assert.eventPresent(
        "providers",
        "BucketRootChanged"
      );
      const bucketRootChangedDataBlob2 =
        userApi.events.providers.BucketRootChanged.is(bucketRootChangedEvent2) &&
        bucketRootChangedEvent2.data;
      if (!bucketRootChangedDataBlob2) {
        throw new Error("Expected BucketRootChanged event but received event of different type");
      }

      strictEqual(bucketRootChangedDataBlob2.newRoot.toString(), localBucketRoot2.toString());

      // The MSP should have accepted at least one file.
      // Register how many were accepted in the last block sealed.
      const mspAcceptedStorageRequestEvents2 = await userApi.assert.eventMany(
        "fileSystem",
        "MspAcceptedStorageRequest"
      );
      for (const e of mspAcceptedStorageRequestEvents2) {
        const mspAcceptedStorageRequestDataBlob =
          userApi.events.fileSystem.MspAcceptedStorageRequest.is(e.event) && e.event.data;
        if (mspAcceptedStorageRequestDataBlob) {
          acceptedFileKeys.push(mspAcceptedStorageRequestDataBlob.fileKey.toString());
        }
      }

      // Now for sure, the total number of accepted files should be `source.length`.
      assert(acceptedFileKeys.length === source.length, `Expected ${source.length} file keys`);

      // And they should be in the Forest storage of the MSP, in the Forest corresponding
      // to the bucket ID.
      for (const fileKey of acceptedFileKeys) {
        const isFileInForest = await msp1Api.rpc.storagehubclient.isFileInForest(bucketId, fileKey);
        assert(isFileInForest.isTrue, "File is not in forest");
        allBucketFiles.push(fileKey);
      }

      // Wait for the BSPs to volunteer and confirm storing the files so the storage requests get fulfilled.
      for (const fileKey of acceptedFileKeys) {
        await userApi.wait.storageRequestNotOnChain(fileKey);
      }
    });

    it("User moves bucket to second MSP", async () => {
      // Get the value propositions of the second MSP to use, and use the first one (can be any).
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

      // Finalising the block in the BSP node as well, to trigger the reorg in the BSP node too.
      const finalisedBlockHash = await userApi.rpc.chain.getFinalizedHead();

      // Wait for BSP node to have imported the finalised block built by the user node.
      await msp2Api.wait.blockImported(finalisedBlockHash.toString());
      await msp2Api.block.finaliseBlock(finalisedBlockHash.toString());

      await userApi.wait.waitForTxInPool({
        module: "fileSystem",
        method: "mspRespondMoveBucketRequest"
      });

      const { events } = await userApi.block.seal();

      assertEventPresent(userApi, "fileSystem", "MoveBucketAccepted", events);

      // Wait for all files to be in the Forest of the second MSP.
      await waitFor({
        lambda: async () => {
          for (const fileKey of allBucketFiles) {
            const isFileInForest = await msp2Api.rpc.storagehubclient.isFileInForest(
              bucketId,
              fileKey
            );
            if (!isFileInForest.isTrue) {
              return false;
            }
          }
          return true;
        },
        iterations: 100,
        delay: 1000
      });
    });
  }
);
