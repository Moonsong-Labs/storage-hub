import assert from "node:assert";
import { bspKey, describeBspNet, type EnrichedBspApi, ferdie, sleep } from "../../../util";

describeBspNet("BSPNet: Validating max storage", ({ before, it, createUserApi }) => {
  let api: EnrichedBspApi;

  before(async () => {
    api = await createUserApi();
  });

  it("Unregistered accounts fail when changing capacities", async () => {
    const totalCapacityBefore = await api.query.providers.totalBspsCapacity();
    const bspCapacityBefore = await api.query.providers.backupStorageProviders(
      api.shConsts.DUMMY_BSP_ID
    );
    assert.ok(bspCapacityBefore.unwrap().capacity.eq(totalCapacityBefore));

    const { events, extSuccess } = await api.sealBlock(
      api.tx.providers.changeCapacity(api.shConsts.CAPACITY[1024]),
      ferdie
    );
    assert.strictEqual(extSuccess, false);

    await api.block.skip(20);
    const [eventInfo, _eventError] = api.assert.fetchEvent(
      api.events.system.ExtrinsicFailed,
      events
    );
    assert.strictEqual(eventInfo.asModule.index.toNumber(), 40); // providers
    assert.strictEqual(eventInfo.asModule.error.toHex(), "0x0f000000"); // NotRegistered

    api.rpc.storagehubclient.getForestRoot();

    const totalCapacityAfter = await api.query.providers.totalBspsCapacity();
    const bspCapacityAfter = await api.query.providers.backupStorageProviders(
      api.shConsts.DUMMY_BSP_ID
    );
    assert.ok(bspCapacityAfter.unwrap().capacity.eq(totalCapacityBefore));
    assert.ok(totalCapacityAfter.eq(totalCapacityBefore));
  });

  it(
    "Maxed out storages not volunteered",
    { skip: "Capacity check not Implemented yet" },
    async () => {
      const capacityUsed = (
        await api.query.providers.backupStorageProviders(api.shConsts.DUMMY_BSP_ID)
      )
        .unwrap()
        .capacityUsed.toNumber();
      await api.block.skipToMinChangeTime();
      const minCapacity = api.consts.providers.spMinCapacity.toNumber();
      const newCapacity = Math.max(minCapacity, capacityUsed + 1);

      const { extSuccess } = await api.sealBlock(
        api.tx.providers.changeCapacity(newCapacity),
        bspKey
      );
      assert.strictEqual(extSuccess, true);

      const source = "res/cloud.jpg";
      const location = "test/cloud.jpg";
      const bucketName = "toobig-1";
      await api.file.newStorageRequest(source, location, bucketName);

      //To allow for BSP to react to request
      await sleep(500);
      await assert.rejects(
        async () => {
          api.assert.extrinsicPresent({
            module: "fileSystem",
            method: "bspVolunteer",
            checkTxPool: true,
            skipSuccessCheck: true
          });
        },
        /No events matching system\.ExtrinsicSuccess/,
        "BSP should not have volunteered to a file that's too big"
      );
    }
  );

  it("Total capacity updated when single BSP capacity updated", async () => {
    const newCapacity =
      BigInt(Math.floor(Math.random() * 1000 * 1024 * 1024)) + api.shConsts.CAPACITY_512;

    await api.sealBlock(api.tx.providers.changeCapacity(newCapacity), bspKey);

    const totalCapacityAfter = await api.query.providers.totalBspsCapacity();
    const bspCapacityAfter = await api.query.providers.backupStorageProviders(
      api.shConsts.DUMMY_BSP_ID
    );
    assert.strictEqual(bspCapacityAfter.unwrap().capacity.toBigInt(), newCapacity);
    assert.strictEqual(totalCapacityAfter.toBigInt(), newCapacity);
  });

  it("Test BSP storage size can not be decreased below used", async () => {
    const source = "res/adolphus.jpg";
    const location = "test/adolphus.jpg";
    const bucketName = "nothingmuch-2";
    await api.file.newStorageRequest(source, location, bucketName);

    await api.wait.bspVolunteer();
    await api.wait.bspStored();

    // Skip block height past threshold
    await api.block.skipToMinChangeTime();

    const { events, extSuccess } = await api.sealBlock(api.tx.providers.changeCapacity(2n), bspKey);
    assert.strictEqual(extSuccess, false);
    const [eventInfo, _eventError] = api.assert.fetchEvent(
      api.events.system.ExtrinsicFailed,
      events
    );
    assert.strictEqual(eventInfo.asModule.index.toNumber(), 40); // providers
    assert.strictEqual(eventInfo.asModule.error.toHex(), "0x0b000000"); // NewCapacityLessThanUsedStorage
  });
});
