import assert, { strictEqual } from "node:assert";
import { u8aToHex } from "@polkadot/util";
import { decodeAddress } from "@polkadot/util-crypto";
import { type EnrichedBspApi, bspKey, describeBspNet, shUser, waitFor } from "../../../util";

describeBspNet("Single BSP Volunteering", ({ before, createBspApi, it, createUserApi }) => {
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

  it("Volunteer for multiple files and delete them", async () => {
    // Set the tick range to maximum threshold parameter to 1 (immediately accept)
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

    const newBucketEventEvent = await userApi.createBucket(bucketName);
    const newBucketEventDataBlob =
      userApi.events.fileSystem.NewBucket.is(newBucketEventEvent) && newBucketEventEvent.data;

    assert(newBucketEventDataBlob, "Event doesn't match Type");

    const files = [];
    const txs = [];
    const ownerHex = u8aToHex(decodeAddress(userApi.shConsts.NODE_INFOS.user.AddressId)).slice(2);
    for (let i = 0; i < source.length; i++) {
      const {
        file_metadata: { location, fingerprint, file_size }
      } = await userApi.rpc.storagehubclient.loadFileInStorage(
        source[i],
        destination[i],
        ownerHex,
        newBucketEventDataBlob.bucketId
      );

      files.push({ fingerprint, file_size, location });
      txs.push(
        userApi.tx.fileSystem.issueStorageRequest(
          newBucketEventDataBlob.bucketId,
          location,
          fingerprint,
          file_size,
          userApi.shConsts.DUMMY_MSP_ID,
          [userApi.shConsts.NODE_INFOS.user.expectedPeerId],
          {
            Custom: 1
          }
        )
      );
    }

    await userApi.block.seal({ calls: txs, signer: shUser });

    // Get the new storage request events, making sure we have 3
    const storageRequestEvents = await userApi.assert.eventMany("fileSystem", "NewStorageRequest");
    strictEqual(storageRequestEvents.length, 3);

    // Get the file keys from the storage request events
    const fileKeys = storageRequestEvents.map((event) => {
      const dataBlob =
        userApi.events.fileSystem.NewStorageRequest.is(event.event) && event.event.data;
      assert(dataBlob, "Event doesn't match Type");
      return dataBlob.fileKey;
    });

    // Wait for the BSP to volunteer
    await userApi.wait.bspVolunteer(source.length);
    for (const fileKey of fileKeys) {
      await bspApi.wait.fileStorageComplete(fileKey);
    }

    // Waiting for a confirmation of the first file to be stored
    await userApi.wait.bspStored({ expectedExts: 1 });

    // Here we expect the 2 others files to be batched
    await userApi.wait.bspStored({ expectedExts: 1 });

    // Wait for BSP to update its local Forest root before starting to generate the inclusion proofs
    await waitFor({
      lambda: async () => {
        let isRootUpdatedWithAllNewFiles = true;
        for (const fileKey of fileKeys) {
          const isFileInForest = await bspApi.rpc.storagehubclient.isFileInForest(null, fileKey);
          isRootUpdatedWithAllNewFiles = isRootUpdatedWithAllNewFiles && isFileInForest.isTrue;
        }
        return isRootUpdatedWithAllNewFiles;
      }
    });

    const stopStroringTxs = [];
    for (let i = 0; i < fileKeys.length; i++) {
      const inclusionForestProof = await bspApi.rpc.storagehubclient.generateForestProof(null, [
        fileKeys[i]
      ]);
      stopStroringTxs.push(
        userApi.tx.fileSystem.bspRequestStopStoring(
          fileKeys[i],
          newBucketEventDataBlob.bucketId,
          files[i].location,
          userApi.shConsts.NODE_INFOS.user.AddressId,
          files[i].fingerprint,
          files[i].file_size,
          false,
          inclusionForestProof.toString()
        )
      );
    }

    await userApi.block.seal({ calls: stopStroringTxs, signer: bspKey });
    const stopStoringEvents = await userApi.assert.eventMany(
      "fileSystem",
      "BspRequestedToStopStoring"
    );
    strictEqual(stopStoringEvents.length, fileKeys.length);

    // Wait enough blocks for the deletion to be allowed.
    const currentBlock = await userApi.rpc.chain.getBlock();
    const currentBlockNumber = currentBlock.block.header.number.toNumber();
    const minWaitForStopStoring = (
      await userApi.query.parameters.parameters({
        RuntimeConfig: {
          MinWaitForStopStoring: null
        }
      })
    )
      .unwrap()
      .asRuntimeConfig.asMinWaitForStopStoring.toNumber();
    const cooldown = currentBlockNumber + minWaitForStopStoring;
    await userApi.block.skipTo(cooldown);

    for (let i = 0; i < fileKeys.length; i++) {
      const inclusionForestProof = await bspApi.rpc.storagehubclient.generateForestProof(null, [
        fileKeys[i]
      ]);
      await userApi.block.seal({
        calls: [
          userApi.tx.fileSystem.bspConfirmStopStoring(fileKeys[i], inclusionForestProof.toString())
        ],
        signer: bspKey
      });

      // Check for the confirm stopped storing event.
      await userApi.assert.eventPresent("fileSystem", "BspConfirmStoppedStoring");

      // Wait for BSP to update its local Forest root as a consequence of the confirmed stop storing extrinsic.
      await waitFor({
        lambda: async () => {
          const isFileInForest = await bspApi.rpc.storagehubclient.isFileInForest(
            null,
            fileKeys[i]
          );
          return isFileInForest.isFalse;
        }
      });
    }
  });

  it(
    "Volunteer for multiple files and delete them (failing to batch when confirming)",
    { skip: "cannot store files again after they have been deleted once" },
    async () => {
      const source = ["res/whatsup.jpg", "res/adolphus.jpg", "res/cloud.jpg"];
      const destination = ["test/whatsup.jpg", "test/adolphus.jpg", "test/cloud.jpg"];
      const bucketName = "something-4";

      const newBucketEventEvent = await userApi.createBucket(bucketName);
      const newBucketEventDataBlob =
        userApi.events.fileSystem.NewBucket.is(newBucketEventEvent) && newBucketEventEvent.data;

      assert(newBucketEventDataBlob, "Event doesn't match Type");

      const files = [];
      const txs = [];
      const ownerHex3 = u8aToHex(decodeAddress(userApi.shConsts.NODE_INFOS.user.AddressId)).slice(
        2
      );
      for (let i = 0; i < source.length; i++) {
        const {
          file_metadata: { location, fingerprint, file_size }
        } = await userApi.rpc.storagehubclient.loadFileInStorage(
          source[i],
          destination[i],
          ownerHex3,
          newBucketEventDataBlob.bucketId
        );

        files.push({ fingerprint, file_size, location });
        txs.push(
          userApi.tx.fileSystem.issueStorageRequest(
            newBucketEventDataBlob.bucketId,
            location,
            fingerprint,
            file_size,
            userApi.shConsts.DUMMY_MSP_ID,
            [userApi.shConsts.NODE_INFOS.user.expectedPeerId],
            {
              Custom: 1
            }
          )
        );
      }

      await userApi.block.seal({ calls: txs, signer: shUser });

      // Get the new storage request events, making sure we have 3
      const storageRequestEvents = await userApi.assert.eventMany(
        "fileSystem",
        "NewStorageRequest"
      );
      strictEqual(storageRequestEvents.length, 3);

      // Get the file keys from the storage request events
      const fileKeys = storageRequestEvents.map((event) => {
        const dataBlob =
          userApi.events.fileSystem.NewStorageRequest.is(event.event) && event.event.data;
        assert(dataBlob, "Event doesn't match Type");
        return dataBlob.fileKey;
      });

      // Wait for the BSP to volunteer
      await userApi.wait.bspVolunteer(source.length);
      for (const fileKey of fileKeys) {
        await bspApi.wait.fileStorageComplete(fileKey);
      }

      // Waiting for a confirmation of the first file to be stored
      await userApi.wait.bspStored({ expectedExts: 1 });

      // Here we expect the 2 others files to be batched
      await userApi.wait.bspStored({ expectedExts: 1 });

      // Wait for BSP to update its local Forest root before starting to generate the inclusion proofs
      await waitFor({
        lambda: async () => {
          let isRootUpdatedWithAllNewFiles = true;
          for (const fileKey of fileKeys) {
            const isFileInForest = await bspApi.rpc.storagehubclient.isFileInForest(null, fileKey);
            isRootUpdatedWithAllNewFiles = isRootUpdatedWithAllNewFiles && isFileInForest.isTrue;
          }
          return isRootUpdatedWithAllNewFiles;
        }
      });

      const stopStroringTxs = [];
      for (let i = 0; i < fileKeys.length; i++) {
        const inclusionForestProof = await bspApi.rpc.storagehubclient.generateForestProof(null, [
          fileKeys[i]
        ]);
        stopStroringTxs.push(
          userApi.tx.fileSystem.bspRequestStopStoring(
            fileKeys[i],
            newBucketEventDataBlob.bucketId,
            files[i].location,
            userApi.shConsts.NODE_INFOS.user.AddressId,
            files[i].fingerprint,
            files[i].file_size,
            false,
            inclusionForestProof.toString()
          )
        );
      }

      await userApi.block.seal({ calls: stopStroringTxs, signer: bspKey });
      const stopStoringEvents = await userApi.assert.eventMany(
        "fileSystem",
        "BspRequestedToStopStoring"
      );
      strictEqual(stopStoringEvents.length, fileKeys.length);

      // Wait enough blocks for the deletion to be allowed.
      const currentBlock = await userApi.rpc.chain.getBlock();
      const currentBlockNumber = currentBlock.block.header.number.toNumber();
      const minWaitForStopStoring = (
        await userApi.query.parameters.parameters({
          RuntimeConfig: {
            MinWaitForStopStoring: null
          }
        })
      )
        .unwrap()
        .asRuntimeConfig.asMinWaitForStopStoring.toNumber();
      const cooldown = currentBlockNumber + minWaitForStopStoring;
      await userApi.block.skipTo(cooldown);

      // Batching the delete confirmation should fail because of the wrong inclusionForestProof for extrinsinc 2 and 3
      const confirmStopStoringTxs = [];
      for (let i = 0; i < fileKeys.length; i++) {
        const inclusionForestProof = await bspApi.rpc.storagehubclient.generateForestProof(null, [
          fileKeys[i]
        ]);
        confirmStopStoringTxs.push(
          userApi.tx.fileSystem.bspConfirmStopStoring(fileKeys[i], inclusionForestProof.toString())
        );

        // Check for the confirm stopped storing event.
        await userApi.assert.eventPresent("fileSystem", "BspConfirmStoppedStoring");

        // Wait for BSP to update its local Forest root as a consequence of the confirmed stop storing extrinsic.
        await waitFor({
          lambda: async () => {
            const isFileInForest = await bspApi.rpc.storagehubclient.isFileInForest(
              null,
              fileKeys[i]
            );
            return isFileInForest.isFalse;
          }
        });
      }

      await userApi.block.seal({ calls: confirmStopStoringTxs, signer: bspKey });

      // Check for the confirm stopped storing event.
      const confirmStopStoringEvents = await userApi.assert.eventMany(
        "fileSystem",
        "BspConfirmStoppedStoring"
      );

      assert(
        confirmStopStoringEvents.length === 1,
        "two of the extrinsincs should fail because of wrong inclusion proof"
      );
    }
  );
});
