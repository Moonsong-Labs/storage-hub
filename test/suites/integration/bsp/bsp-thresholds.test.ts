import assert, { strictEqual } from "node:assert";
import {
  addBsp,
  bspDownKey,
  bspDownSeed,
  bspThreeKey,
  bspThreeSeed,
  bspTwoKey,
  bspTwoSeed,
  describeBspNet,
  type EnrichedBspApi,
  ShConsts
} from "../../../util";

describeBspNet(
  "BSPNet: BSP Volunteering Thresholds",
  { initialised: false },
  ({ before, it, createUserApi, beforeEach }) => {
    let api: EnrichedBspApi;

    before(async () => {
      api = await createUserApi();
    });

    beforeEach(async () => {
      await api.sealBlock(api.tx.sudo.sudo(api.tx.fileSystem.setGlobalParameters(1, 1)));
    });

    it("Can set params with setGlobalParams", async () => {
      const { extSuccess } = await api.sealBlock(
        api.tx.sudo.sudo(api.tx.fileSystem.setGlobalParameters(87, 200))
      );

      strictEqual(extSuccess, true, "Extrinsic should be successful");

      strictEqual(
        (await api.query.fileSystem.blockRangeToMaximumThreshold()).toNumber(),
        200,
        "Threshold should have changed"
      );
      strictEqual(
        (await api.query.fileSystem.replicationTarget()).toNumber(),
        87,
        "Replication Target should have changed"
      );
    });

    it("Shouldn't be able to setGlobalParams without sudo", async () => {
      const { extSuccess, events } = await api.sealBlock(
        api.tx.fileSystem.setGlobalParameters(13, 37)
      );

      strictEqual(extSuccess, false, "Extrinsic should be unsuccessful");
      const { data } = api.assert.eventPresent("system", "ExtrinsicFailed", events);
      const error = data[0].toString();
      strictEqual(error, "BadOrigin", "Extrinsic should fail with BadOrigin");

      strictEqual(
        (await api.query.fileSystem.blockRangeToMaximumThreshold()).toNumber(),
        1,
        "Threshold should not have changed"
      );
      strictEqual(
        (await api.query.fileSystem.replicationTarget()).toNumber(),
        1,
        "Replication Target should not have changed"
      );
    });

    it("Reputation increased on successful storage", { skip: "Not Implemented" }, async () => {
      const repBefore = (await api.query.providers.backupStorageProviders(ShConsts.DUMMY_BSP_ID))
        .unwrap()
        .reputationWeight.toBigInt();
      await api.file.newStorageRequest("res/cloud.jpg", "test/cloud.jpg", "bucket-1");
      await api.wait.bspVolunteer();
      await api.wait.bspStored();

      const repAfter = (await api.query.providers.backupStorageProviders(ShConsts.DUMMY_BSP_ID))
        .unwrap()
        .reputationWeight.toBigInt();

      assert(
        repAfter > repBefore,
        "Reputation should increase after successful storage request fufilled"
      );
      console.log(`Reputation increased from ${repBefore} to ${repAfter}`);
    });

    it("zero reputation can still volunteer and be accepted", async () => {
      // Create a new BSP and onboard with no reputation
      await addBsp(api, bspDownKey, {
        name: "sh-bsp-down",
        bspKeySeed: bspDownSeed,
        bspId: ShConsts.BSP_DOWN_ID,
        additionalArgs: ["--keystore-path=/keystore/bsp-down"],
        bspStartingWeight: 0n
      });
      await api.sealBlock(api.tx.sudo.sudo(api.tx.fileSystem.setGlobalParameters(5, 1)));

      await api.file.newStorageRequest("res/smile.jpg", "test/smile.jpg", "bucket-1"); // T0
      await api.wait.bspVolunteer();

      const events = await api.query.system.events();
      const matchedEvents = await api.assert.eventMany(
        "fileSystem",
        "AcceptedBspVolunteer",
        events
      ); // T1

      assert(matchedEvents.length === 2, "Multiple BSPs should be able to volunteer");

      const filtered = matchedEvents.filter(
        ({ event }) =>
          (api.events.fileSystem.AcceptedBspVolunteer.is(event) && event.data.bspId.toString()) ===
          ShConsts.BSP_DOWN_ID
      );

      assert(
        filtered.length === 1,
        "Zero reputation BSP should be able to volunteer and be accepted"
      );
      await api.docker.stopBspContainer("sh-bsp-down");
    });

    // Not sure if this is good test? the times are very dependent on threshold created
    it("BSP two eventually volunteers after threshold curve is met", async () => {
      await api.sealBlock(api.tx.sudo.sudo(api.tx.fileSystem.setGlobalParameters(2, 10)));

      await addBsp(api, bspTwoKey, {
        name: "sh-bsp-two",
        bspKeySeed: bspTwoSeed,
        bspId: ShConsts.BSP_TWO_ID,
        additionalArgs: ["--keystore-path=/keystore/bsp-two"]
      });

      await api.file.newStorageRequest("res/cloud.jpg", "test/cloud.jpg", "bucket-2"); // T0
      await api.sealBlock(); // T1
      await api.sealBlock(); // T2
      await api.wait.bspVolunteer(); // T3
      await api.wait.bspStored(); // T4

      await api.sealBlock(); // T5
      await api.wait.bspVolunteer(); // T6
      await api.wait.bspStored(); // T7

      await api.docker.stopBspContainer("sh-bsp-two");
    });

    it("BSP with reputation is prioritised", async () => {
      await addBsp(api, bspThreeKey, {
        name: "sh-bsp-three",
        bspKeySeed: bspThreeSeed,
        bspId: ShConsts.BSP_THREE_ID,
        additionalArgs: ["--keystore-path=/keystore/bsp-three"],
        bspStartingWeight: 800_000n
      });

      // Set global params to small numbers
      await api.sealBlock(api.tx.sudo.sudo(api.tx.fileSystem.setGlobalParameters(2, 10)));

      // Create a new storage request
      await api.file.newStorageRequest("res/adolphus.jpg", "test/adolphus.jpg", "bucket-3"); // T0

      await api.wait.bspVolunteer();
      const events = await api.query.system.events();
      const matchedEvents = await api.assert.eventMany(
        "fileSystem",
        "AcceptedBspVolunteer",
        events
      ); // T1

      const filtered = matchedEvents.filter(
        ({ event }) =>
          (api.events.fileSystem.AcceptedBspVolunteer.is(event) && event.data.bspId.toString()) ===
          ShConsts.BSP_THREE_ID
      );

      // Verify that the BSP with reputation is prioritised over the zero reputation
      assert(filtered.length === 1, "BSP with reputation should be prioritised");
      await api.docker.stopBspContainer("sh-bsp-three");
    });
  }
);
