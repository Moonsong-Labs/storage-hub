import assert from "node:assert";
import {
  bspKey,
  describeBspNet,
  type EnrichedBspApi,
  ferdie,
  skipBlocksToMinChangeTime,
  sleep
} from "../../../util";

describeBspNet(
  "BSPNet: Validating max storage",
  ({ before, it, createUserApi }) => {
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
      const [eventInfo, _eventError] = api.assert.fetchEventData(
        api.events.system.ExtrinsicFailed,
        events
      );
      assert.strictEqual(eventInfo.asModule.index.toNumber(), 40); // providers
      assert.strictEqual(eventInfo.asModule.error.toHex(), "0x0f000000"); // NotRegistered

      const totalCapacityAfter = await api.query.providers.totalBspsCapacity();
      const bspCapacityAfter = await api.query.providers.backupStorageProviders(
        api.shConsts.DUMMY_BSP_ID
      );
      assert.ok(bspCapacityAfter.unwrap().capacity.eq(totalCapacityBefore));
      assert.ok(totalCapacityAfter.eq(totalCapacityBefore));
    });

    it("Change capacity ext called before volunteering for file size greater than available capacity", async () => {
      const capacityUsed = (
        await api.query.providers.backupStorageProviders(api.shConsts.DUMMY_BSP_ID)
      )
        .unwrap()
        .capacityUsed.toNumber();
      await api.block.skipToMinChangeTime();
      const minCapacity = api.consts.providers.spMinCapacity.toNumber();
      const newCapacity = Math.max(minCapacity, capacityUsed + 1);

      // Set BSP's available capacity to 0 to force the BSP to increase its capacity before volunteering for the storage request.
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

      // Skip block height until BSP sends a call to change capacity.
      await skipBlocksToMinChangeTime(api);
      // Allow BSP enough time to send call to change capacity.
      await sleep(500);
      // Assert BSP has sent a call to increase its capacity.
      await api.assert.extrinsicPresent({
        module: "providers",
        method: "changeCapacity",
        checkTxPool: true
      });

      await api.sealBlock();

      // Assert that the capacity has changed.
      api.assert.eventPresent("providers", "CapacityChanged", await api.query.system.events());

      // Allow BSP enough time to send call to volunteer for the storage request.
      await sleep(500);

      // Assert that the BSP has send a call to volunteer for the storage request.
      await api.assert.extrinsicPresent({
        module: "fileSystem",
        method: "bspVolunteer",
        checkTxPool: true
      });

      await api.sealBlock();

      // Assert that the BSP was accepted as a volunteer.
      api.assert.eventPresent(
        "fileSystem",
        "AcceptedBspVolunteer",
        await api.query.system.events()
      );
    });

    it("Total capacity updated when single BSP capacity updated", async () => {
      const newCapacity =
        BigInt(Math.floor(Math.random() * 1000 * 1024 * 1024)) + api.shConsts.CAPACITY_512;

      // Skip block height past threshold
      await skipBlocksToMinChangeTime(api);

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

      const { events, extSuccess } = await api.sealBlock(
        api.tx.providers.changeCapacity(2n),
        bspKey
      );
      assert.strictEqual(extSuccess, false);
      const [eventInfo, _eventError] = api.assert.fetchEventData(
        api.events.system.ExtrinsicFailed,
        events
      );
      assert.strictEqual(eventInfo.asModule.index.toNumber(), 40); // providers
      assert.strictEqual(eventInfo.asModule.error.toHex(), "0x0b000000"); // NewCapacityLessThanUsedStorage
    });
  }
);
