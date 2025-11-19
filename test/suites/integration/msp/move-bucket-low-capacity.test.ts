import assert, { strictEqual } from "node:assert";
import type { EventRecord } from "@polkadot/types/interfaces";
import { u8aToHex } from "@polkadot/util";
import { decodeAddress } from "@polkadot/util-crypto";
import {
  addMspContainer,
  assertEventPresent,
  bspThreeKey,
  bspTwoKey,
  describeMspNet,
  type EnrichedBspApi,
  getContainerIp,
  mspThreeKey,
  ShConsts,
  shUser,
  sleep
} from "../../../util";

await describeMspNet(
  "MSP rejects bucket move requests due to low capacity",
  { initialised: false, indexer: true },
  ({ before, after, createMsp1Api, it, createUserApi, createApi }) => {
    let userApi: EnrichedBspApi;
    let mspApi: EnrichedBspApi;
    let msp3Api: EnrichedBspApi;

    const source = ["res/cloud.jpg", "res/smile.jpg", "res/whatsup.jpg"];
    const destination = ["test/cloud.jpg", "test/smile.jpg", "test/whatsup.jpg"];
    const bucketName = "move-bucket";
    let bucketId: string;
    const allBucketFiles: string[] = [];

    before(async () => {
      userApi = await createUserApi();
      const maybeMspApi = await createMsp1Api();
      if (!maybeMspApi) {
        throw new Error("Failed to create MSP API");
      }
      mspApi = maybeMspApi;
    });

    after(async () => {
      msp3Api.disconnect();
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

      const mspNodePeerId = await mspApi.rpc.system.localPeerId();
      strictEqual(mspNodePeerId.toString(), userApi.shConsts.NODE_INFOS.msp1.expectedPeerId);
    });

    it("Add 2 more BSPs (3 total) and set the replication target to 2", async () => {
      // Replicate to 2 BSPs, 5 blocks to maxthreshold
      const newRuntimeParameter = {
        RuntimeConfig: {
          MaxReplicationTarget: [null, 2],
          TickRangeToMaximumThreshold: [null, 5]
        }
      };
      await userApi.block.seal({
        calls: [userApi.tx.sudo.sudo(userApi.tx.parameters.setParameter(newRuntimeParameter))]
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

    it("Add new MSP with low capacity", async () => {
      const { containerName, p2pPort, peerId, rpcPort } = await addMspContainer({
        name: "storage-hub-sh-msp-sleepy",
        additionalArgs: [
          "--keystore-path=/keystore/msp-three",
          `--max-storage-capacity=${1024 * 1024}`,
          `--jump-capacity=${1024 * 1024}`,
          "--msp-charging-period=12"
        ]
      });

      await userApi.docker.waitForLog({
        containerName: "storage-hub-sh-msp-sleepy",
        searchString: "💤 Idle",
        timeout: 15000
      });

      msp3Api = await createApi(`ws://127.0.0.1:${rpcPort}`);

      // Give it some balance.
      const amount = 10000n * 10n ** 12n;
      await userApi.block.seal({
        calls: [
          userApi.tx.sudo.sudo(userApi.tx.balances.forceSetBalance(mspThreeKey.address, amount))
        ]
      });

      const mspIp = await getContainerIp(containerName);
      const multiAddressMsp = `/ip4/${mspIp}/tcp/${p2pPort}/p2p/${peerId}`;
      await userApi.block.seal({
        calls: [
          userApi.tx.sudo.sudo(
            userApi.tx.providers.forceMspSignUp(
              mspThreeKey.address,
              mspThreeKey.publicKey,
              userApi.shConsts.CAPACITY_512,
              [multiAddressMsp],
              100 * 1024 * 1024,
              "Terms of Service...",
              9999999,
              mspThreeKey.address
            )
          )
        ]
      });
    });

    it("User submits 3 storage requests in the same bucket for first MSP", async () => {
      // Get value propositions from the MSP to use, and use the first one (can be any).
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
      // Get the events of the storage requests to extract the file keys and check
      // that the MSP received them.
      const events = await userApi.assert.eventMany("fileSystem", "NewStorageRequest");
      const matchedEvents = events.filter((e: EventRecord) =>
        userApi.events.fileSystem.NewStorageRequest.is(e.event)
      );
      if (matchedEvents.length !== source.length) {
        throw new Error(`Expected ${source.length} NewStorageRequest events`);
      }

      // Allow time for the MSP to receive and store the files from the user
      await sleep(3000);

      // Check if the MSP received the files.
      for (const e of matchedEvents) {
        const newStorageRequestDataBlob =
          userApi.events.fileSystem.NewStorageRequest.is(e.event) && e.event.data;

        if (!newStorageRequestDataBlob) {
          throw new Error("Event doesn't match NewStorageRequest type");
        }

        const result = await mspApi.rpc.storagehubclient.isFileInFileStorage(
          newStorageRequestDataBlob.fileKey
        );

        if (!result.isFileFound) {
          throw new Error(
            `File not found in storage for ${newStorageRequestDataBlob.location.toHuman()}`
          );
        }

        allBucketFiles.push(newStorageRequestDataBlob.fileKey.toString());
      }

      // Seal block containing the MSP's first response.
      await userApi.wait.mspResponseInTxPool();
      await userApi.block.seal();

      // Give time for the MSP to update the local forest root.
      await sleep(1000);

      // Check that the local forest root is updated, and matches the on-chain root.
      const localBucketRoot = await mspApi.rpc.storagehubclient.getForestRoot(bucketId);

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
      await sleep(1000);

      // Check that the local forest root is updated, and matches the on-chain root.
      const localBucketRoot2 = await mspApi.rpc.storagehubclient.getForestRoot(bucketId);

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
        const isFileInForest = await mspApi.rpc.storagehubclient.isFileInForest(bucketId, fileKey);
        assert(isFileInForest.isTrue, "File is not in forest");
        allBucketFiles.push(fileKey);
      }

      // Seal more blocks until the storage request is fulfilled.
      let hasStorageRequests = true;
      let iterations = 0;
      const maxIterations = 60; // Max 60 iterations (30 seconds at 500ms per iteration)

      while (hasStorageRequests && iterations < maxIterations) {
        hasStorageRequests = false;
        for (const fileKey of acceptedFileKeys) {
          const storageRequest = await userApi.query.fileSystem.storageRequests(fileKey);
          if (storageRequest && !storageRequest.isEmpty) {
            hasStorageRequests = true;
            break;
          }
        }

        if (hasStorageRequests) {
          await sleep(500);
          const block = await userApi.block.seal();
          await userApi.rpc.engine.finalizeBlock(block.blockReceipt.blockHash);
          iterations++;
        }
      }

      if (iterations >= maxIterations) {
        throw new Error(`Storage requests not fulfilled after ${maxIterations} iterations`);
      }
    });

    it("New MSP rejects move request due to low capacity", async () => {
      const valueProps = await userApi.call.storageProvidersApi.queryValuePropositionsForMsp(
        mspThreeKey.publicKey
      );
      const valuePropId = valueProps[0].id;

      // User requests to move bucket to second MSP
      const requestMoveBucketResult = await userApi.block.seal({
        calls: [
          userApi.tx.fileSystem.requestMoveBucket(bucketId, mspThreeKey.publicKey, valuePropId)
        ],
        signer: shUser
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
      await msp3Api.wait.blockImported(finalisedBlockHash.toString());
      await msp3Api.block.finaliseBlock(finalisedBlockHash.toString());

      // Wait for the rejection response from Sleepy
      await userApi.wait.waitForTxInPool({
        module: "fileSystem",
        method: "mspRespondMoveBucketRequest"
      });

      const { events } = await userApi.block.seal();

      // Verify that the move request was rejected
      assertEventPresent(userApi, "fileSystem", "MoveBucketRejected", events);
    });
  }
);
