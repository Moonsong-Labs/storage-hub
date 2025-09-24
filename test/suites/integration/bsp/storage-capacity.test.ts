import assert from "node:assert";
import {
  addBsp,
  BspNetTestApi,
  bspKey,
  bspTwoKey,
  describeBspNet,
  type EnrichedBspApi,
  ferdie,
  ShConsts,
  sleep
} from "../../../util";

await describeBspNet("BSPNet: Change capacity tests.", ({ before, it, createUserApi }) => {
  let userApi: EnrichedBspApi;

  before(async () => {
    userApi = await createUserApi();
  });

  it("An unregistered account (not BSP nor MSP) can't change its capacity.", async () => {
    // Get the total network BSP capacity and the DUMMY_BSP's capacity before doing anything.
    // This the DUMMY_BSP is the only one active, they should match.
    const totalCapacityBefore = await userApi.query.providers.totalBspsCapacity();
    const bspCapacityBefore = await userApi.query.providers.backupStorageProviders(
      userApi.shConsts.DUMMY_BSP_ID
    );
    assert.ok(bspCapacityBefore.unwrap().capacity.eq(totalCapacityBefore));

    // Seal a block with a change capacity extrinsic from an unregistered account.
    const { events, extSuccess } = await userApi.block.seal({
      calls: [userApi.tx.providers.changeCapacity(userApi.shConsts.CAPACITY[1024])],
      signer: ferdie
    });

    // The extrinsic should have failed.
    assert.strictEqual(extSuccess, false);

    // Get the event of the extrinsic failure.
    const {
      data: { dispatchError: eventInfo }
    } = userApi.assert.fetchEvent(userApi.events.system.ExtrinsicFailed, events);

    // Ensure it failed with the correct error.
    const providersPallet = userApi.runtimeMetadata.asLatest.pallets.find(
      (pallet) => pallet.name.toString() === "Providers"
    );
    const notRegisteredErrorIndex = userApi.errors.providers.NotRegistered.meta.index.toNumber();
    assert.strictEqual(eventInfo.asModule.index.toNumber(), providersPallet?.index.toNumber());
    assert.strictEqual(eventInfo.asModule.error[0], notRegisteredErrorIndex);

    // Ensure neither the total capacity nor the DUMMY_BSP's capacity changed.
    const totalCapacityAfter = await userApi.query.providers.totalBspsCapacity();
    const bspCapacityAfter = await userApi.query.providers.backupStorageProviders(
      userApi.shConsts.DUMMY_BSP_ID
    );
    assert.ok(bspCapacityAfter.unwrap().capacity.eq(totalCapacityBefore));
    assert.ok(totalCapacityAfter.eq(totalCapacityBefore));
  });

  it("BSP changes its capacity if not enough before volunteering for a storage request.", async () => {
    // Set up 1 block to max volunteer threshold (i.e. instant acceptance)
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

    // Skip blocks until the BSP can change its capacity.
    await userApi.block.skipUntilBspCanChangeCapacity(userApi.shConsts.DUMMY_BSP_ID);

    // Get the current used capacity of the DUMMY_BSP and the minimum capacity that a BSP can have.
    const capacityUsed = (
      await userApi.query.providers.backupStorageProviders(userApi.shConsts.DUMMY_BSP_ID)
    )
      .unwrap()
      .capacityUsed.toNumber();
    const minCapacity = userApi.consts.providers.spMinCapacity.toNumber();

    // The capacity to set for the BSP will be the maximum of the minimum capacity and the current used capacity.
    const newCapacity = Math.max(minCapacity, capacityUsed);

    // Set BSP's available capacity (total - used) to 0 to force the BSP to increase its capacity before volunteering for the storage request.
    await userApi.wait.waitForAvailabilityToSendTx(bspKey.address.toString());
    const { extSuccess } = await userApi.block.seal({
      calls: [userApi.tx.providers.changeCapacity(newCapacity)],
      signer: bspKey
    });
    assert.strictEqual(extSuccess, true);

    // Issue a new storage request.
    const source = "res/cloud.jpg";
    const location = "test/cloud.jpg";
    const bucketName = "toobig-1";
    await userApi.file.createBucketAndSendNewStorageRequest(source, location, bucketName);

    // Wait until the BSP detects that it has to increase its capacity to be able to volunteer.
    await userApi.docker.waitForLog({
      containerName: "storage-hub-sh-bsp-1",
      searchString: "Insufficient storage capacity to volunteer for file key"
    });

    // Skip blocks until the BSP can change its capacity.
    await userApi.block.skipUntilBspCanChangeCapacity(userApi.shConsts.DUMMY_BSP_ID);

    // Assert that the BSP has sent the extrinsic to increase its capacity.
    await userApi.assert.extrinsicPresent({
      module: "providers",
      method: "changeCapacity",
      checkTxPool: true
    });

    // Seal the block to increase the capacity of the BSP so it can volunteer.
    await userApi.block.seal();

    // Assert that the event of the capacity change was emitted.
    await userApi.assert.eventPresent("providers", "CapacityChanged");

    // Assert that the BSP correctly increased its capacity by the `JUMP_CAPACITY_BSP` amount.
    const updatedCapacity = BigInt(userApi.shConsts.JUMP_CAPACITY_BSP + newCapacity);
    const bspCapacityAfter = await userApi.query.providers.backupStorageProviders(
      userApi.shConsts.DUMMY_BSP_ID
    );
    assert.strictEqual(bspCapacityAfter.unwrap().capacity.toBigInt(), updatedCapacity);

    // Assert that the BSP has sent the extrinsic to volunteer for the storage request.
    await userApi.assert.extrinsicPresent({
      module: "fileSystem",
      method: "bspVolunteer",
      checkTxPool: true
    });

    // Seal the block to volunteer the BSP for the storage request.
    await userApi.block.seal();

    // Assert that the BSP was accepted as a volunteer.
    await userApi.assert.eventPresent("fileSystem", "AcceptedBspVolunteer");
  });

  it("Total BSP capacity of the network is updated when a single BSP changes its capacity.", async () => {
    // Calculate a random new capacity for the BSP.
    const newCapacity =
      BigInt(Math.floor(Math.random() * 1000 * 1024 * 1024)) + userApi.shConsts.CAPACITY_512;

    // Skip blocks until the BSP can change its capacity.
    await userApi.block.skipUntilBspCanChangeCapacity(userApi.shConsts.DUMMY_BSP_ID);

    // Seal the block with the extrinsic to change the capacity of the BSP.
    await userApi.wait.waitForAvailabilityToSendTx(bspKey.address.toString());
    await userApi.block.seal({
      calls: [userApi.tx.providers.changeCapacity(newCapacity)],
      signer: bspKey
    });

    // Ensure that the new capacity was set correctly and the total BSP capacity of the network was updated.
    const totalCapacityAfter = await userApi.query.providers.totalBspsCapacity();
    const bspCapacityAfter = await userApi.query.providers.backupStorageProviders(
      userApi.shConsts.DUMMY_BSP_ID
    );
    assert.strictEqual(bspCapacityAfter.unwrap().capacity.toBigInt(), newCapacity);
    assert.strictEqual(totalCapacityAfter.toBigInt(), newCapacity);
  });

  it("Total capacity of a BSP can't go under its used capacity.", async () => {
    // Check if the current used capacity of the BSP is greater than the minimum capacity a BSP can have.
    // If it's not, issue a new storage request and make the BSP volunteer and confirm it.
    let capacityUsed = (
      await userApi.query.providers.backupStorageProviders(userApi.shConsts.DUMMY_BSP_ID)
    )
      .unwrap()
      .capacityUsed.toNumber();
    const minCapacity = userApi.consts.providers.spMinCapacity.toNumber();
    if (capacityUsed <= minCapacity) {
      // Get the current available capacity of the BSP (total - used).
      const totalBspCapacity = (
        await userApi.query.providers.backupStorageProviders(userApi.shConsts.DUMMY_BSP_ID)
      )
        .unwrap()
        .capacity.toNumber();
      const availableCapacity = totalBspCapacity - capacityUsed;

      // Issue a new storage request.
      const source = "res/adolphus.jpg";
      const location = "test/adolphus.jpg";
      const bucketName = "nothingmuch-2";
      const { fileSize } = await userApi.file.createBucketAndSendNewStorageRequest(
        source,
        location,
        bucketName
      );

      // Ensure the BSP has enough available capacity to store the file.
      assert(fileSize <= availableCapacity, "BSP doesn't have enough available capacity.");

      // Wait until the BSP volunteers for the storage request.
      await userApi.wait.bspVolunteer(1);

      // Assert that the BSP was accepted as a volunteer.
      await userApi.assert.eventPresent("fileSystem", "AcceptedBspVolunteer");

      // Wait until the BSP stores the file.
      const bspAddress = userApi.createType("Address", bspKey.address);
      await userApi.wait.bspStored({ expectedExts: 1, bspAccount: bspAddress });

      // Update the used capacity of the BSP.
      capacityUsed = (
        await userApi.query.providers.backupStorageProviders(userApi.shConsts.DUMMY_BSP_ID)
      )
        .unwrap()
        .capacityUsed.toNumber();
    }

    // Skip blocks until the BSP can change its capacity.
    await userApi.block.skipUntilBspCanChangeCapacity(userApi.shConsts.DUMMY_BSP_ID);

    // Try to change the BSP's capacity to something that's smaller than the used capacity.
    await userApi.wait.waitForAvailabilityToSendTx(bspKey.address.toString());
    const { events, extSuccess } = await userApi.block.seal({
      calls: [userApi.tx.providers.changeCapacity(capacityUsed - 1)],
      signer: bspKey
    });

    // The extrinsic should have failed.
    assert.strictEqual(extSuccess, false);

    // Get the event of the extrinsic failure.
    const {
      data: { dispatchError: eventInfo }
    } = userApi.assert.fetchEvent(userApi.events.system.ExtrinsicFailed, events);

    // Ensure it failed with the correct error.
    const providersPallet = userApi.runtimeMetadata.asLatest.pallets.find(
      (pallet) => pallet.name.toString() === "Providers"
    );
    const newCapacityLessThanUsedStorageErrorIndex =
      userApi.errors.providers.NewCapacityLessThanUsedStorage.meta.index.toNumber();
    assert.strictEqual(eventInfo.asModule.index.toNumber(), providersPallet?.index.toNumber());
    assert.strictEqual(eventInfo.asModule.error[0], newCapacityLessThanUsedStorageErrorIndex);
  });

  it("Required capacity over available capacity gets accumulated and changed at once if trying to volunteer to multiple storage requests.", async () => {
    // Skip blocks until the BSP can change its capacity.
    await userApi.block.skipUntilBspCanChangeCapacity(userApi.shConsts.DUMMY_BSP_ID);

    // Get the current used capacity of the DUMMY_BSP and the minimum capacity that a BSP can have.
    const capacityUsed = (
      await userApi.query.providers.backupStorageProviders(userApi.shConsts.DUMMY_BSP_ID)
    )
      .unwrap()
      .capacityUsed.toNumber();
    const minCapacity = userApi.consts.providers.spMinCapacity.toNumber();

    // The capacity to set for the BSP will be the maximum of the minimum capacity and the current used capacity.
    const newCapacity = Math.max(minCapacity, capacityUsed);

    // Set BSP's available capacity (total - used) to 0 to force the BSP to increase its capacity before volunteering for a storage request.
    await userApi.wait.waitForAvailabilityToSendTx(bspKey.address.toString());
    const { extSuccess } = await userApi.block.seal({
      calls: [userApi.tx.providers.changeCapacity(newCapacity)],
      signer: bspKey
    });
    assert.strictEqual(extSuccess, true);

    // Issue two storage requests.
    const source1 = "res/cloud.jpg";
    const location1 = "test/cloud.jpg";
    const bucketName1 = "bucket-1";
    await userApi.file.createBucketAndSendNewStorageRequest(source1, location1, bucketName1);

    const source2 = "res/adolphus.jpg";
    const location2 = "test/adolphus.jpg";
    const bucketName2 = "bucket-2";
    await userApi.file.createBucketAndSendNewStorageRequest(source2, location2, bucketName2);

    // Wait until the BSP first detects that it has to increase its capacity to be able to volunteer.
    await userApi.docker.waitForLog({
      containerName: "storage-hub-sh-bsp-1",
      searchString: "Insufficient storage capacity to volunteer for file key"
    });

    // Wait a bit more so the BSP detects the second file too.
    // TODO: If needed, replace this with a more reliable way to wait for the second file.
    await sleep(1000);

    // Skip blocks until the BSP can change its capacity.
    await userApi.block.skipUntilBspCanChangeCapacity(userApi.shConsts.DUMMY_BSP_ID);

    // Assert that the BSP has sent the extrinsic to increase its capacity.
    await userApi.assert.extrinsicPresent({
      module: "providers",
      method: "changeCapacity",
      checkTxPool: true
    });

    // Seal the block to increase the capacity of the BSP so it can volunteer.
    await userApi.block.seal();

    // Assert that the event of the capacity change was emitted.
    await userApi.assert.eventPresent("providers", "CapacityChanged");

    // Assert that the BSP correctly increased its capacity by the `JUMP_CAPACITY_BSP` amount.
    const updatedCapacity = BigInt(userApi.shConsts.JUMP_CAPACITY_BSP + newCapacity);
    const bspCapacityAfter = await userApi.query.providers.backupStorageProviders(
      userApi.shConsts.DUMMY_BSP_ID
    );
    assert.strictEqual(bspCapacityAfter.unwrap().capacity.toBigInt(), updatedCapacity);

    // Assert that the BSP has sent two extrinsics to volunteer for both storage requests.
    await userApi.wait.bspVolunteer(2);

    // Assert that the BSP was accepted as a volunteer.
    const acceptedBspVolunteerEvents = await userApi.assert.eventMany(
      "fileSystem",
      "AcceptedBspVolunteer"
    );
    assert(acceptedBspVolunteerEvents.length === 2, "BSP wasn't accepted as a volunteer twice.");

    // Wait until the BSP stores both files
    const bspAddress = userApi.createType("Address", bspKey.address);
    await userApi.wait.bspStored({ expectedExts: 1, bspAccount: bspAddress });
  });

  it("BSP does not increase its capacity over its configured maximum (and skips volunteering if that would be needed).", async () => {
    // Max storage capacity such that the BSP can store one of the files we will request but no more.
    const MAX_STORAGE_CAPACITY = 416600;

    // Add a second BSP with the configured maximum storage capacity limit.
    const { rpcPort } = await addBsp(userApi, bspTwoKey, userApi.accounts.sudo, {
      name: "sh-bsp-two",
      bspId: ShConsts.BSP_TWO_ID,
      maxStorageCapacity: MAX_STORAGE_CAPACITY,
      initialCapacity: BigInt(MAX_STORAGE_CAPACITY),
      additionalArgs: ["--keystore-path=/keystore/bsp-two"]
    });
    await userApi.assert.eventPresent("providers", "BspSignUpSuccess");

    // Wait until the new BSP catches up to the chain tip.
    const bspTwoApi = await BspNetTestApi.create(`ws://127.0.0.1:${rpcPort}`);
    await userApi.wait.nodeCatchUpToChainTip(bspTwoApi);

    // Stop the other BSP so it doesn't volunteer for the files.
    await userApi.docker.pauseContainer("storage-hub-sh-bsp-1");

    // Issue the first storage request. The new BSP should have enough capacity to volunteer for it.
    const source1 = "res/cloud.jpg";
    const location1 = "test/cloud.jpg";
    const bucketName1 = "kek1";
    const fileMetadata = await userApi.file.createBucketAndSendNewStorageRequest(
      source1,
      location1,
      bucketName1
    );

    // Check at which tick the new BSP can volunteer for the file.
    // Note: since we set up the network to have instant acceptance, the new BSP should be able to volunteer immediately
    // but we still check to be sure.
    const bspVolunteerTick = (
      await userApi.call.fileSystemApi.queryEarliestFileVolunteerTick(
        ShConsts.BSP_TWO_ID,
        fileMetadata.fileKey
      )
    ).asOk.toNumber();

    // If the BSP can't volunteer yet, skips blocks until it can.
    if ((await userApi.rpc.chain.getHeader()).number.toNumber() < bspVolunteerTick - 1) {
      // If a BSP can volunteer in tick X, it sends the extrinsic once it imports block with tick X - 1, so it gets included directly in tick X
      await userApi.block.skipTo(bspVolunteerTick - 1);
    }

    // Wait until the BSP volunteers for the file.
    await userApi.wait.bspVolunteer(1);

    // Wait until the BSP confirms storing the file.
    const bspTwpAddress = userApi.createType("Address", bspTwoKey.address);
    await userApi.wait.bspStored({ expectedExts: 1, bspAccount: bspTwpAddress });

    // Issue the second storage request. The BSP shouldn't be able to volunteer for this one since
    // it would have to increase its capacity over its configured maximum.
    const source2 = "res/adolphus.jpg";
    const location2 = "test/adolphus.jpg";
    const bucketName2 = "kek2";
    const fileMetadata2 = await userApi.file.createBucketAndSendNewStorageRequest(
      source2,
      location2,
      bucketName2
    );

    // Check at which tick the new BSP can volunteer for the file.
    // Note: since we set up the network to have instant acceptance, the new BSP should be able to volunteer immediately
    // but we still check to be sure.
    const bspVolunteerTick2 = (
      await userApi.call.fileSystemApi.queryEarliestFileVolunteerTick(
        ShConsts.BSP_TWO_ID,
        fileMetadata2.fileKey
      )
    ).asOk.toNumber();

    // If the BSP can't volunteer yet, skips blocks until it can.
    if ((await userApi.rpc.chain.getHeader()).number.toNumber() < bspVolunteerTick2) {
      // If a BSP can volunteer in tick X, it sends the extrinsic once it imports block with tick X - 1, so it gets included directly in tick X
      await userApi.block.skipTo(bspVolunteerTick2 - 1);
    }

    // The BSP should not volunteer for the second file. To check this we check that the wait for
    // the BSP volunteer times out and throws.
    assert.rejects(userApi.wait.bspVolunteer(1));

    // Check that the BSP's capacity used is equal to the first file's size
    const bspTwo = (
      await userApi.query.providers.backupStorageProviders(ShConsts.BSP_TWO_ID)
    ).unwrap();
    assert.equal(
      bspTwo.capacityUsed.toNumber(),
      fileMetadata.fileSize,
      "Used capacity is still equal to the first file's size"
    );

    // Disconnect and stop the new BSP.
    await userApi.docker.stopContainer("sh-bsp-two");
    await bspTwoApi.disconnect();
  });
});
