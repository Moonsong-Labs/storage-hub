import assert, { notEqual, strictEqual } from "node:assert";
import { u8aToHex } from "@polkadot/util";
import { decodeAddress } from "@polkadot/util-crypto";
import {
  bspTwoKey,
  describeBspNet,
  type EnrichedBspApi,
  type FileMetadata,
  shUser,
  waitFor
} from "../../../util";

await describeBspNet(
  "BSPNet: Single BSP Volunteering",
  { initialised: false },
  ({ before, createBspApi, it, createUserApi }) => {
    let userApi: EnrichedBspApi;
    let bspApi: EnrichedBspApi;

    const source = "res/whatsup.jpg";
    const destination = "test/whatsup.jpg";
    const bucketName = "nothingmuch-2";

    let fileMetadata: FileMetadata;

    before(async () => {
      userApi = await createUserApi();
      bspApi = await createBspApi();
    });

    it("Network launches and can be queried", async () => {
      const userNodePeerId = await userApi.rpc.system.localPeerId();
      strictEqual(userNodePeerId.toString(), userApi.shConsts.NODE_INFOS.user.expectedPeerId);

      const bspNodePeerId = await bspApi.rpc.system.localPeerId();
      strictEqual(bspNodePeerId.toString(), userApi.shConsts.NODE_INFOS.bsp.expectedPeerId);
    });

    it("BSP volunteers when a new storage request is issued via issueStorageRequest", async () => {
      // Get the initial forest root of the BSP and ensure it matches the root of an empty forest
      const initialBspForestRoot = await bspApi.rpc.storagehubclient.getForestRoot(null);
      strictEqual(
        initialBspForestRoot.toString(),
        "0x03170a2e7597b7b7e3d84c05391d139a62b157e78786d8c082f29dcf4c111314"
      );

      // Create a new bucket and issue a storage request for it
      fileMetadata = await userApi.file.createBucketAndSendNewStorageRequest(
        source,
        destination,
        bucketName,
        null,
        null,
        null,
        1
      );

      // Wait for the BSP volunteer transaction to be in the transaction pool
      await userApi.wait.bspVolunteerInTxPool(1);

      // Seal the block with the BSP volunteer transaction
      await userApi.block.seal();

      // Assert that the `AcceptedBspVolunteer` event was emitted for the correct file
      const {
        data: {
          bspId: resBspId,
          bucketId: resBucketId,
          location: resLoc,
          fingerprint: resFinger,
          multiaddresses: resMulti,
          size_: resSize
        }
      } = userApi.assert.fetchEvent(
        userApi.events.fileSystem.AcceptedBspVolunteer,
        await userApi.query.system.events()
      );

      strictEqual(resBspId.toHuman(), userApi.shConsts.DUMMY_BSP_ID);
      strictEqual(resBucketId.toString(), fileMetadata.bucketId);
      strictEqual(resLoc.toHuman(), destination);
      strictEqual(resFinger.toString(), fileMetadata.fingerprint.toString());
      strictEqual(resMulti.length, 1);
      strictEqual(
        (resMulti[0].toHuman() as string).includes(userApi.shConsts.NODE_INFOS.bsp.expectedPeerId),
        true
      );
      strictEqual(resSize.toNumber(), fileMetadata.fileSize);

      // Wait for the BSP to receive the file and store it in its file storage
      await waitFor({
        lambda: async () =>
          (await bspApi.rpc.storagehubclient.isFileInFileStorage(fileMetadata.fileKey)).isFileFound
      });

      // Wait for the BSP to confirm storing the file via the `bspConfirmStoring` extrinsic
      await userApi.wait.bspStored({
        expectedExts: 1,
        timeoutMs: 12000,
        sealBlock: false
      });

      // Ensure the BSP confirm storing transaction is in the pool
      await userApi.assert.extrinsicPresent({
        method: "bspConfirmStoring",
        module: "fileSystem",
        checkTxPool: true,
        assertLength: 1
      });

      // Seal the block with the `bspConfirmStoring` extrinsic
      await userApi.block.seal();

      // Assert that the `BspConfirmedStoring` event was emitted for the correct file and BSP
      const {
        data: {
          bspId: bspConfirmRes_bspId,
          confirmedFileKeys: bspConfirmRes_fileKeys,
          newRoot: bspConfirmRes_newRoot
        }
      } = userApi.assert.fetchEvent(
        userApi.events.fileSystem.BspConfirmedStoring,
        await userApi.query.system.events()
      );

      strictEqual(bspConfirmRes_bspId.toHuman(), userApi.shConsts.DUMMY_BSP_ID);
      strictEqual(bspConfirmRes_fileKeys.length, 1);
      strictEqual(bspConfirmRes_fileKeys[0][0].toString(), fileMetadata.fileKey.toString());

      // Get the new forest root of the BSP and ensure it matches the root of the forest after the file was confirmed
      await waitFor({
        lambda: async () =>
          (await bspApi.rpc.storagehubclient.getForestRoot(null)).toHex() !==
          initialBspForestRoot.unwrap().toHex()
      });
      const bspForestRootAfterConfirm = await bspApi.rpc.storagehubclient.getForestRoot(null);
      strictEqual(bspForestRootAfterConfirm.toString(), bspConfirmRes_newRoot.toString());
      notEqual(bspForestRootAfterConfirm.toString(), initialBspForestRoot.toString());

      // Check that the file received by the BSP can be saved into its disk and passes integrity checks
      const saveFileToDisk = await bspApi.rpc.storagehubclient.saveFileToDisk(
        bspConfirmRes_fileKeys[0][0],
        "/storage/test/whatsup.jpg"
      );
      assert(saveFileToDisk.isSuccess);
      const sha = await userApi.docker.checkFileChecksum("test/whatsup.jpg");
      strictEqual(sha, userApi.shConsts.TEST_ARTEFACTS["res/whatsup.jpg"].checksum);

      // Check if the storage request is still on-chain (MSP may have already accepted it in a previous block)
      const storageRequest = await userApi.query.fileSystem.storageRequests(fileMetadata.fileKey);

      if (storageRequest.isSome) {
        // Storage request still exists, wait for MSP to accept it
        await userApi.wait.mspResponseInTxPool(1);

        // Seal the block with the MSP acceptance transaction
        await userApi.block.seal();

        // Verify that the storage request is no longer on-chain
        const storageRequestAfterMspAcceptance = await userApi.query.fileSystem.storageRequests(
          fileMetadata.fileKey
        );
        assert(
          storageRequestAfterMspAcceptance.isNone,
          "Storage request should be removed from chain after MSP acceptance"
        );
      }
    });

    it("BSP skips volunteering for the same file it's already storing", async () => {
      // Issue a new storage request to the BSP for the same file key
      await userApi.block.seal({
        calls: [
          userApi.tx.fileSystem.issueStorageRequest(
            fileMetadata.bucketId,
            fileMetadata.location,
            fileMetadata.fingerprint,
            fileMetadata.fileSize,
            userApi.shConsts.DUMMY_MSP_ID,
            [userApi.shConsts.NODE_INFOS.user.expectedPeerId],
            {
              Custom: 1
            }
          )
        ],
        signer: shUser
      });

      await userApi.assert.eventPresent("fileSystem", "NewStorageRequestV2");

      // The BSP should skip volunteering for the same file it's already storing
      await bspApi.docker.waitForLog({
        containerName: "storage-hub-sh-bsp-1",
        searchString: "Skipping file key",
        timeout: 15000
      });
    });
  }
);

await describeBspNet(
  "BSPNet: Initial volunteer tick is different for different BSPs and stays constant over time",
  {
    initialised: false,
    bspStartingWeight: 1n
  },
  ({ before, createBspApi, it, createUserApi }) => {
    let userApi: EnrichedBspApi;
    let bspApi: EnrichedBspApi;

    const source = "res/adolphus.jpg";
    const destination = "test/adolphus.jpg";
    const bucketName = "initial-volunteer-tick-test";

    let fileMetadata: FileMetadata;
    let bspId: string;
    let bsp2Id: string;

    before(async () => {
      userApi = await createUserApi();
      bspApi = await createBspApi();

      // Set TickRangeToMaximumThreshold to 1000 to ensure BSPs need to wait
      const tickToMaximumThresholdRuntimeParameter = {
        RuntimeConfig: {
          TickRangeToMaximumThreshold: [null, 1000]
        }
      };
      await userApi.block.seal({
        calls: [
          userApi.tx.sudo.sudo(
            userApi.tx.parameters.setParameter(tickToMaximumThresholdRuntimeParameter)
          )
        ]
      });

      // Get the BSP IDs for later use
      bspId = userApi.shConsts.DUMMY_BSP_ID;
      bsp2Id = userApi.shConsts.BSP_TWO_ID;

      // Add the second BSP with weight=1
      await userApi.docker.onboardBsp({
        bspSigner: bspTwoKey,
        name: "sh-bsp-two",
        bspId: bsp2Id,
        additionalArgs: ["--keystore-path=/keystore/bsp-two"],
        waitForIdle: true,
        bspStartingWeight: 1n
      });
    });

    it("Network launches and can be queried", async () => {
      const userNodePeerId = await userApi.rpc.system.localPeerId();
      strictEqual(userNodePeerId.toString(), userApi.shConsts.NODE_INFOS.user.expectedPeerId);

      const bspNodePeerId = await bspApi.rpc.system.localPeerId();
      strictEqual(bspNodePeerId.toString(), userApi.shConsts.NODE_INFOS.bsp.expectedPeerId);
    });

    it("Earliest volunteer tick is stable and deterministic for both BSPs", async () => {
      // Stop the BSPs so they don't volunteer for the file
      await userApi.docker.stopContainer("sh-bsp-1");
      await userApi.docker.stopContainer("sh-bsp-two");

      // Create a new bucket and issue a storage request with replication target = 1
      fileMetadata = await userApi.file.createBucketAndSendNewStorageRequest(
        source,
        destination,
        bucketName,
        null,
        null,
        null,
        1
      );

      // Get the current tick for reference
      const initialTick = (await userApi.call.proofsDealerApi.getCurrentTick()).toNumber();

      // Query the earliest volunteer tick for both BSPs
      const bsp1EarliestTick = (
        await userApi.call.fileSystemApi.queryEarliestFileVolunteerTick(bspId, fileMetadata.fileKey)
      ).asOk.toNumber();
      const bsp2EarliestTick = (
        await userApi.call.fileSystemApi.queryEarliestFileVolunteerTick(
          bsp2Id,
          fileMetadata.fileKey
        )
      ).asOk.toNumber();

      // The two BSPs should have different thresholds so they should have different earliest volunteer ticks.
      // We know that the BSP 2 can't volunteer immediately because the BSP IDs and the file key are
      // deterministic in this test.
      notEqual(
        bsp1EarliestTick,
        bsp2EarliestTick,
        "Different BSPs should have different earliest volunteer ticks (based on their thresholds)"
      );

      // The BSP 1 should be able to volunteer immediately. Again, we know this is the case because
      // the BSP IDs and the file key are deterministic in this test.
      strictEqual(bsp1EarliestTick, initialTick, "BSP1 should be able to volunteer immediately");

      // Seal a few blocks and query again - values should remain stable
      await userApi.block.seal();
      await userApi.block.seal();
      await userApi.block.seal();

      // Get the current tick after advancing time
      const tickAfterAdvance = (await userApi.call.proofsDealerApi.getCurrentTick()).toNumber();

      // Query both BSPs after advancing time
      // The earliest volunteer tick should remain stable (except if the BSP was already eligible or
      // became eligible in the meantime, in which case it should have increased to the current tick)
      const bsp1EarliestTickAfterAdvance = (
        await userApi.call.fileSystemApi.queryEarliestFileVolunteerTick(bspId, fileMetadata.fileKey)
      ).asOk.toNumber();
      const bsp2EarliestTickAfterAdvance = (
        await userApi.call.fileSystemApi.queryEarliestFileVolunteerTick(
          bsp2Id,
          fileMetadata.fileKey
        )
      ).asOk.toNumber();

      // Check if BSPs are now eligible (current tick >= their earliest tick)
      if (tickAfterAdvance >= bsp1EarliestTick) {
        assert(
          bsp1EarliestTickAfterAdvance === tickAfterAdvance,
          "If the BSP is eligible, its earliest volunteer tick should be the same as the current tick"
        );
      } else {
        strictEqual(
          bsp1EarliestTickAfterAdvance,
          bsp1EarliestTick,
          "If the BSP is not yet eligible, its earliest volunteer tick should remain stable"
        );
      }

      if (tickAfterAdvance >= bsp2EarliestTick) {
        assert(
          bsp2EarliestTickAfterAdvance === tickAfterAdvance,
          "If the BSP is eligible, its earliest volunteer tick should be the same as the current tick"
        );
      } else {
        strictEqual(
          bsp2EarliestTickAfterAdvance,
          bsp2EarliestTick,
          "If the BSP is not yet eligible, its earliest volunteer tick should remain stable"
        );
      }
    });
  }
);

await describeBspNet(
  "BSPNet: Single BSP multi-volunteers",
  { initialised: false },
  ({ before, createBspApi, createUserApi, it }) => {
    let userApi: EnrichedBspApi;
    let bspApi: EnrichedBspApi;

    before(async () => {
      userApi = await createUserApi();
      bspApi = await createBspApi();
    });

    it("Network launches and can be queried", async () => {
      const userNodePeerId = await userApi.rpc.system.localPeerId();
      strictEqual(userNodePeerId.toString(), userApi.shConsts.NODE_INFOS.user.expectedPeerId);

      const bspNodePeerId = await bspApi.rpc.system.localPeerId();
      strictEqual(bspNodePeerId.toString(), userApi.shConsts.NODE_INFOS.bsp.expectedPeerId);
    });

    it("BSP volunteers and confirms storing multiple files with batching", async () => {
      // Get the initial forest root of the BSP and ensure it matches the root of an empty forest
      const initialBspForestRoot = await bspApi.rpc.storagehubclient.getForestRoot(null);
      strictEqual(
        initialBspForestRoot.toString(),
        "0x03170a2e7597b7b7e3d84c05391d139a62b157e78786d8c082f29dcf4c111314"
      );

      // Set tick range to 1 block for instant BSP acceptance (bypasses threshold checks)
      const tickToMaximumThresholdRuntimeParameter = {
        RuntimeConfig: {
          TickRangeToMaximumThreshold: [null, 1]
        }
      };
      await userApi.block.seal({
        calls: [
          userApi.tx.sudo.sudo(
            userApi.tx.parameters.setParameter(tickToMaximumThresholdRuntimeParameter)
          )
        ]
      });

      const source = ["res/whatsup.jpg", "res/adolphus.jpg", "res/cloud.jpg"];
      const destination = ["test/whatsup.jpg", "test/adolphus.jpg", "test/cloud.jpg"];
      const bucketName = "something-3";

      // Create a new bucket for the storage requests
      const newBucketEventEvent = await userApi.createBucket(bucketName);
      const newBucketEventDataBlob =
        userApi.events.fileSystem.NewBucket.is(newBucketEventEvent) && newBucketEventEvent.data;

      assert(newBucketEventDataBlob, "Event doesn't match Type");

      // Prepare multiple storage request transactions for all files
      const txs = [];
      const ownerHex2 = u8aToHex(decodeAddress(userApi.shConsts.NODE_INFOS.user.AddressId)).slice(
        2
      );
      for (let i = 0; i < source.length; i++) {
        const {
          file_metadata: { location, fingerprint, file_size }
        } = await userApi.rpc.storagehubclient.loadFileInStorage(
          source[i],
          destination[i],
          ownerHex2,
          newBucketEventDataBlob.bucketId
        );

        txs.push(
          userApi.tx.fileSystem.issueStorageRequest(
            newBucketEventDataBlob.bucketId,
            location,
            fingerprint,
            file_size,
            userApi.shConsts.DUMMY_MSP_ID,
            [userApi.shConsts.NODE_INFOS.user.expectedPeerId],
            {
              Basic: null
            }
          )
        );
      }

      // Seal a block with all storage request transactions
      await userApi.block.seal({ calls: txs, signer: shUser });

      // Verify that all three storage requests were created
      const storageRequestEvents = await userApi.assert.eventMany(
        "fileSystem",
        "NewStorageRequestV2"
      );
      strictEqual(storageRequestEvents.length, 3);

      // Get the file keys from the storage request events
      const fileKeys = storageRequestEvents.map((event) => {
        const dataBlob =
          userApi.events.fileSystem.NewStorageRequestV2.is(event.event) && event.event.data;
        if (!dataBlob) {
          throw new Error("Event doesn't match Type");
        }
        return dataBlob.fileKey;
      });

      // Wait for the BSP volunteer transactions to be in the transaction pool
      await userApi.wait.bspVolunteerInTxPool(source.length);

      // Seal the block with the BSP volunteer transactions
      await userApi.block.seal();

      // Wait for the BSP to receive and store all files in its file storage
      for (let i = 0; i < source.length; i++) {
        const fileKey = fileKeys[i];
        await waitFor({
          lambda: async () =>
            (await bspApi.rpc.storagehubclient.isFileInFileStorage(fileKey)).isFileFound
        });
      }

      // Wait for the BSP to confirm storing the files via the `bspConfirmStoring` extrinsic
      // The first file to be completed will immediately acquire the forest write lock
      // and send the `bspConfirmStoring` extrinsic. The other two files will be queued.
      await userApi.wait.bspStored({
        expectedExts: 1,
        timeoutMs: 12000,
        sealBlock: false
      });

      // Ensure the BSP confirm storing transaction is in the pool
      await userApi.assert.extrinsicPresent({
        method: "bspConfirmStoring",
        module: "fileSystem",
        checkTxPool: true,
        assertLength: 1
      });

      // Seal the block with the `bspConfirmStoring` extrinsic
      await userApi.block.seal();

      // Assert that the first `BspConfirmedStoring` event was emitted for the correct BSP and file
      const {
        data: {
          bspId: bspConfirmRes_bspId,
          confirmedFileKeys: bspConfirmRes_fileKeys,
          newRoot: bspConfirmRes_newRoot
        }
      } = userApi.assert.fetchEvent(
        userApi.events.fileSystem.BspConfirmedStoring,
        await userApi.query.system.events()
      );

      strictEqual(bspConfirmRes_bspId.toHuman(), userApi.shConsts.DUMMY_BSP_ID);
      strictEqual(bspConfirmRes_fileKeys.length, 1);

      // Wait for the BSP to process the BspConfirmedStoring event and update its forest root
      await waitFor({
        lambda: async () =>
          (await bspApi.rpc.storagehubclient.getForestRoot(null)).toHex() !==
          initialBspForestRoot.unwrap().toHex()
      });
      const bspForestRootAfterConfirm = await bspApi.rpc.storagehubclient.getForestRoot(null);
      strictEqual(bspForestRootAfterConfirm.toString(), bspConfirmRes_newRoot.toString());
      notEqual(bspForestRootAfterConfirm.toString(), initialBspForestRoot.toString());

      // After the previous block is processed by the BSP, the forest write lock is released and
      // the other pending `bspConfirmStoring` extrinsics are processed and batched into one extrinsic
      await userApi.wait.bspStored({
        expectedExts: 1,
        timeoutMs: 12000,
        sealBlock: false
      });

      // Ensure the batched BSP confirm storing transaction is in the pool
      await userApi.assert.extrinsicPresent({
        method: "bspConfirmStoring",
        module: "fileSystem",
        checkTxPool: true,
        assertLength: 1
      });

      // Seal the block with the batched `bspConfirmStoring` extrinsic
      await userApi.block.seal();

      // Assert that the second `BspConfirmedStoring` event contains the batched files
      const {
        data: {
          bspId: bspConfirm2Res_bspId,
          confirmedFileKeys: bspConfirm2Res_fileKeys,
          newRoot: bspConfirm2Res_newRoot
        }
      } = userApi.assert.fetchEvent(
        userApi.events.fileSystem.BspConfirmedStoring,
        await userApi.query.system.events()
      );

      strictEqual(bspConfirm2Res_bspId.toHuman(), userApi.shConsts.DUMMY_BSP_ID);
      strictEqual(bspConfirm2Res_fileKeys.length, 2);

      // Wait for the BSP to process the second batch and update its forest root
      await waitFor({
        lambda: async () =>
          (await bspApi.rpc.storagehubclient.getForestRoot(null)).toHex() !==
          bspForestRootAfterConfirm.toHex()
      });

      const bspForestRootAfterConfirm2 = await bspApi.rpc.storagehubclient.getForestRoot(null);
      strictEqual(bspForestRootAfterConfirm2.toString(), bspConfirm2Res_newRoot.toString());
      notEqual(bspForestRootAfterConfirm2.toString(), bspForestRootAfterConfirm.toString());
    });
  }
);
