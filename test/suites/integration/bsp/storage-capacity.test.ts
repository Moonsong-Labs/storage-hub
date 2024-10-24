import assert from "node:assert";
import {
  bspKey,
  describeBspNet,
  type EnrichedBspApi,
  ferdie,
  skipBlocksToMinChangeTime,
  sleep
} from "../../../util";

describeBspNet("BSPNet: Validating max storage", ({ before, it, createUserApi, createBspApi }) => {
  let userApi: EnrichedBspApi;
  let bspApi: EnrichedBspApi;

  before(async () => {
    userApi = await createUserApi();
    bspApi = await createBspApi();
  });

  it("Unregistered accounts fail when changing capacities", async () => {
    const totalCapacityBefore = await userApi.query.providers.totalBspsCapacity();
    const bspCapacityBefore = await userApi.query.providers.backupStorageProviders(
      bspApi.shConsts.DUMMY_BSP_ID
    );
    assert.ok(bspCapacityBefore.unwrap().capacity.eq(totalCapacityBefore));

    const { events, extSuccess } = await userApi.sealBlock(
      userApi.tx.providers.changeCapacity(bspApi.shConsts.CAPACITY[1024]),
      ferdie
    );
    assert.strictEqual(extSuccess, false);

    await userApi.block.skip(20);
    const [eventInfo, _eventError] = userApi.assert.fetchEventData(
      userApi.events.system.ExtrinsicFailed,
      events
    );

    const providersPallet = userApi.runtimeMetadata.asLatest.pallets.find(
      (pallet) => pallet.name.toString() === "Providers"
    );
    const notRegisteredErrorIndex = userApi.errors.providers.NotRegistered.meta.index.toNumber();
    assert.strictEqual(eventInfo.asModule.index.toNumber(), providersPallet?.index.toNumber());
    assert.strictEqual(eventInfo.asModule.error[0], notRegisteredErrorIndex);

    const totalCapacityAfter = await userApi.query.providers.totalBspsCapacity();
    const bspCapacityAfter = await userApi.query.providers.backupStorageProviders(
      bspApi.shConsts.DUMMY_BSP_ID
    );
    assert.ok(bspCapacityAfter.unwrap().capacity.eq(totalCapacityBefore));
    assert.ok(totalCapacityAfter.eq(totalCapacityBefore));
  });

  it("Change capacity ext called before volunteering for file size greater than available capacity", async () => {
    const capacityUsed = (
      await userApi.query.providers.backupStorageProviders(bspApi.shConsts.DUMMY_BSP_ID)
    )
      .unwrap()
      .capacityUsed.toNumber();
    await userApi.block.skipToMinChangeTime();
    const minCapacity = userApi.consts.providers.spMinCapacity.toNumber();
    const newCapacity = Math.max(minCapacity, capacityUsed + 1);

    // Set BSP's available capacity to 0 to force the BSP to increase its capacity before volunteering for the storage request.
    const { extSuccess } = await userApi.sealBlock(
      userApi.tx.providers.changeCapacity(newCapacity),
      bspKey
    );
    assert.strictEqual(extSuccess, true);

    const source = "res/cloud.jpg";
    const location = "test/cloud.jpg";
    const bucketName = "toobig-1";
    await userApi.file.newStorageRequest(source, location, bucketName);

    //To allow for BSP to react to request
    await sleep(500);

    // Skip block height until BSP sends a call to change capacity.
    await skipBlocksToMinChangeTime(userApi);
    // Allow BSP enough time to send call to change capacity.
    await sleep(500);
    // Assert BSP has sent a call to increase its capacity.
    await userApi.assert.extrinsicPresent({
      module: "providers",
      method: "changeCapacity",
      checkTxPool: true
    });

    await userApi.sealBlock();

    // Assert that the capacity has changed.
    await userApi.assert.eventPresent("providers", "CapacityChanged");

    // Allow BSP enough time to send call to volunteer for the storage request.
    await sleep(500);

    // Assert that the BSP has send a call to volunteer for the storage request.
    await userApi.assert.extrinsicPresent({
      module: "fileSystem",
      method: "bspVolunteer",
      checkTxPool: true
    });

    await userApi.sealBlock();

    const updatedCapacity = BigInt(bspApi.shConsts.JUMP_CAPACITY_BSP + newCapacity);
    const bspCapacityAfter = await userApi.query.providers.backupStorageProviders(
      bspApi.shConsts.DUMMY_BSP_ID
    );
    assert.strictEqual(bspCapacityAfter.unwrap().capacity.toBigInt(), updatedCapacity);

    // Assert that the BSP was accepted as a volunteer.
    await userApi.assert.eventPresent("fileSystem", "AcceptedBspVolunteer");
  });

  it("Total capacity updated when single BSP capacity updated", async () => {
    const newCapacity =
      BigInt(Math.floor(Math.random() * 1000 * 1024 * 1024)) + bspApi.shConsts.CAPACITY_512;

    // Skip block height past threshold
    await skipBlocksToMinChangeTime(userApi);

    await userApi.sealBlock(userApi.tx.providers.changeCapacity(newCapacity), bspKey);

    const totalCapacityAfter = await userApi.query.providers.totalBspsCapacity();
    const bspCapacityAfter = await userApi.query.providers.backupStorageProviders(
      bspApi.shConsts.DUMMY_BSP_ID
    );
    assert.strictEqual(bspCapacityAfter.unwrap().capacity.toBigInt(), newCapacity);
    assert.strictEqual(totalCapacityAfter.toBigInt(), newCapacity);
  });

  it("Test BSP storage size can not be decreased below used", async () => {
    const source = "res/adolphus.jpg";
    const location = "test/adolphus.jpg";
    const bucketName = "nothingmuch-2";
    await userApi.file.newStorageRequest(source, location, bucketName);

    await userApi.wait.bspVolunteer();
    await userApi.wait.bspStored();

    // Skip block height past threshold
    await userApi.block.skipToMinChangeTime();

    const { events, extSuccess } = await userApi.sealBlock(
      userApi.tx.providers.changeCapacity(2n),
      bspKey
    );
    assert.strictEqual(extSuccess, false);
    const [eventInfo, _eventError] = userApi.assert.fetchEventData(
      userApi.events.system.ExtrinsicFailed,
      events
    );

    const providersPallet = userApi.runtimeMetadata.asLatest.pallets.find(
      (pallet) => pallet.name.toString() === "Providers"
    );
    const newCapacityLessThanUsedStorageErrorIndex =
      userApi.errors.providers.NewCapacityLessThanUsedStorage.meta.index.toNumber();
    assert.strictEqual(eventInfo.asModule.index.toNumber(), providersPallet?.index.toNumber());
    assert.strictEqual(eventInfo.asModule.error[0], newCapacityLessThanUsedStorageErrorIndex);
  });

  it("Test BSP storage size increased twice in the same increasing period (check for race condition)", async () => {
    const capacityUsed = (
      await userApi.query.providers.backupStorageProviders(bspApi.shConsts.DUMMY_BSP_ID)
    )
      .unwrap()
      .capacityUsed.toNumber();
    await userApi.block.skipToMinChangeTime();
    const minCapacity = userApi.consts.providers.spMinCapacity.toNumber();
    const newCapacity = Math.max(minCapacity, capacityUsed + 1);

    // Set BSP's available capacity to 0 to force the BSP to increase its capacity before volunteering for the storage request.
    const { extSuccess } = await userApi.sealBlock(
      userApi.tx.providers.changeCapacity(newCapacity),
      bspKey
    );
    assert.strictEqual(extSuccess, true);

    // First storage request
    const source1 = "res/cloud.jpg";
    const location1 = "test/cloud.jpg";
    const bucketName1 = "bucket-1";
    await userApi.file.newStorageRequest(source1, location1, bucketName1);

    // Second storage request
    const source2 = "res/adolphus.jpg";
    const location2 = "test/adolphus.jpg";
    const bucketName2 = "bucket-2";
    await userApi.file.newStorageRequest(source2, location2, bucketName2);

    //To allow for BSP to react to request
    await sleep(500);

    await userApi.block.skipToMinChangeTime();

    // Allow BSP enough time to send call to change capacity.
    await sleep(500);

    // Assert BSP has sent a call to increase its capacity.
    await bspApi.assert.extrinsicPresent({
      module: "providers",
      method: "changeCapacity",
      checkTxPool: true
    });

    await userApi.sealBlock();

    // Assert that the capacity has changed.
    await userApi.assert.eventPresent("providers", "CapacityChanged");

    const updatedCapacity = BigInt(userApi.shConsts.JUMP_CAPACITY_BSP + newCapacity);
    const bspCapacityAfter = await userApi.query.providers.backupStorageProviders(
      bspApi.shConsts.DUMMY_BSP_ID
    );
    assert.strictEqual(bspCapacityAfter.unwrap().capacity.toBigInt(), updatedCapacity);
  });
});
