import assert, { strictEqual } from "node:assert";
import {
  addBsp,
  bspDownKey,
  bspDownSeed,
  BspNetTestApi,
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
  { initialised: false, bspStartingWeight: 5n, networkConfig: "standard" },
  ({ before, it, createUserApi, createBspApi, beforeEach }) => {
    let userApi: EnrichedBspApi;
    let bspApi: EnrichedBspApi;

    before(async () => {
      userApi = await createUserApi();
      bspApi = await createBspApi();
    });

    beforeEach(async () => {
      await userApi.sealBlock(
        userApi.tx.sudo.sudo(userApi.tx.fileSystem.setGlobalParameters(1, 1))
      );
    });

    it("Can set params with setGlobalParams", async () => {
      const { extSuccess } = await userApi.sealBlock(
        userApi.tx.sudo.sudo(userApi.tx.fileSystem.setGlobalParameters(87, 200))
      );

      strictEqual(extSuccess, true, "Extrinsic should be successful");

      strictEqual(
        (await userApi.query.fileSystem.tickRangeToMaximumThreshold()).toNumber(),
        200,
        "Threshold should have changed"
      );
      strictEqual(
        (await userApi.query.fileSystem.replicationTarget()).toNumber(),
        87,
        "Replication Target should have changed"
      );
    });

    it("Shouldn't be able to setGlobalParams without sudo", async () => {
      const { extSuccess } = await userApi.sealBlock(
        userApi.tx.fileSystem.setGlobalParameters(13, 37)
      );

      strictEqual(extSuccess, false, "Extrinsic should be unsuccessful");
      const { data } = await userApi.assert.eventPresent("system", "ExtrinsicFailed");
      const error = data[0].toString();
      strictEqual(error, "BadOrigin", "Extrinsic should fail with BadOrigin");

      strictEqual(
        (await userApi.query.fileSystem.tickRangeToMaximumThreshold()).toNumber(),
        1,
        "Threshold should not have changed"
      );
      strictEqual(
        (await userApi.query.fileSystem.replicationTarget()).toNumber(),
        1,
        "Replication Target should not have changed"
      );
    });

    it("Reputation increased on successful storage", { skip: "Not Implemented" }, async () => {
      const repBefore = (
        await userApi.query.providers.backupStorageProviders(ShConsts.DUMMY_BSP_ID)
      )
        .unwrap()
        .reputationWeight.toBigInt();
      await userApi.file.newStorageRequest("res/cloud.jpg", "test/cloud.jpg", "bucket-1");
      await userApi.wait.bspVolunteer();
      await userApi.wait.bspStored();

      const repAfter = (await userApi.query.providers.backupStorageProviders(ShConsts.DUMMY_BSP_ID))
        .unwrap()
        .reputationWeight.toBigInt();

      assert(
        repAfter > repBefore,
        "Reputation should increase after successful storage request fufilled"
      );
      console.log(`Reputation increased from ${repBefore} to ${repAfter}`);
    });

    it("lower reputation can still volunteer and be accepted", async () => {
      await userApi.sealBlock(
        userApi.tx.sudo.sudo(userApi.tx.fileSystem.setGlobalParameters(5, 10))
      );

      // Create a new BSP and onboard with no reputation
      await addBsp(userApi, bspDownKey, {
        name: "sh-bsp-down",
        bspKeySeed: bspDownSeed,
        bspId: ShConsts.BSP_DOWN_ID,
        additionalArgs: ["--keystore-path=/keystore/bsp-down"],
        bspStartingWeight: 1n
      });

      const { fileKey } = await userApi.file.newStorageRequest(
        "res/smile.jpg",
        "test/smile.jpg",
        "bucket-1"
      ); // T0

      const lowReputationVolunteerTick = (
        await userApi.call.fileSystemApi.queryEarliestFileVolunteerTick(
          ShConsts.BSP_DOWN_ID,
          fileKey
        )
      ).asOk.toNumber();

      const normalReputationVolunteerTick = (
        await userApi.call.fileSystemApi.queryEarliestFileVolunteerTick(
          ShConsts.DUMMY_BSP_ID,
          fileKey
        )
      ).asOk.toNumber();

      if ((await userApi.rpc.chain.getHeader()).number.toNumber() < lowReputationVolunteerTick) {
        await userApi.block.skipTo(lowReputationVolunteerTick);
      }

      if (normalReputationVolunteerTick < lowReputationVolunteerTick) {
        await userApi.wait.bspVolunteer();
      } else {
        await userApi.wait.bspVolunteer(2);
      }
      const matchedEvents = await userApi.assert.eventMany("fileSystem", "AcceptedBspVolunteer"); // T1

      const filtered = matchedEvents.filter(
        ({ event }) =>
          (userApi.events.fileSystem.AcceptedBspVolunteer.is(event) &&
            event.data.bspId.toString()) === ShConsts.BSP_DOWN_ID
      );

      assert(
        filtered.length === 1,
        "Zero reputation BSP should be able to volunteer and be accepted"
      );
      await userApi.docker.stopBspContainer("sh-bsp-down");
    });

    it("BSP two eventually volunteers after threshold curve is met", async () => {
      await userApi.sealBlock(
        userApi.tx.sudo.sudo(userApi.tx.fileSystem.setGlobalParameters(2, 20))
      );

      // Add the second BSP
      const { rpcPort } = await addBsp(userApi, bspTwoKey, {
        name: "sh-bsp-two",
        bspKeySeed: bspTwoSeed,
        bspId: ShConsts.BSP_TWO_ID,
        additionalArgs: ["--keystore-path=/keystore/bsp-two"]
      });
      const bspTwoApi = await BspNetTestApi.create(`ws://127.0.0.1:${rpcPort}`);

      // Wait for it to catch up to the tip of the chain
      await userApi.wait.bspCatchUpToChainTip(bspTwoApi);

      // Create a new storage request
      const { fileKey } = await userApi.file.newStorageRequest(
        "res/cloud.jpg",
        "test/cloud.jpg",
        "bucket-2"
      );

      // Check where the BSPs would be allowed to volunteer for it
      const bsp1VolunteerTick = (
        await userApi.call.fileSystemApi.queryEarliestFileVolunteerTick(
          ShConsts.DUMMY_BSP_ID,
          fileKey
        )
      ).asOk.toNumber();
      const bsp2VolunteerTick = (
        await userApi.call.fileSystemApi.queryEarliestFileVolunteerTick(
          ShConsts.BSP_TWO_ID,
          fileKey
        )
      ).asOk.toNumber();

      if (bsp1VolunteerTick < bsp2VolunteerTick) {
        // If the first BSP can volunteer first, wait for it to volunteer and confirm storing the file
        if ((await userApi.rpc.chain.getHeader()).number.toNumber() < bsp1VolunteerTick) {
          await userApi.block.skipTo(bsp1VolunteerTick);
        }
        await userApi.wait.bspVolunteer(1);
        await bspApi.wait.bspFileStorageComplete(fileKey);
        await userApi.wait.bspStored(1);

        // Then wait for the second BSP to volunteer and confirm storing the file
        if ((await userApi.rpc.chain.getHeader()).number.toNumber() < bsp2VolunteerTick) {
          await userApi.block.skipTo(bsp2VolunteerTick);
        }
        await userApi.wait.bspVolunteer(1);
        await bspTwoApi.wait.bspFileStorageComplete(fileKey);
        await userApi.wait.bspStored(1);
      } else if (bsp1VolunteerTick > bsp2VolunteerTick) {
        // If the second BSP can volunteer first, wait for it to volunteer and confirm storing the file
        if ((await userApi.rpc.chain.getHeader()).number.toNumber() < bsp2VolunteerTick) {
          await userApi.block.skipTo(bsp2VolunteerTick);
        }
        await userApi.wait.bspVolunteer(1);
        await bspTwoApi.wait.bspFileStorageComplete(fileKey);
        await userApi.wait.bspStored(1);

        // Then wait for the first BSP to volunteer and confirm storing the file
        if ((await userApi.rpc.chain.getHeader()).number.toNumber() < bsp1VolunteerTick) {
          await userApi.block.skipTo(bsp1VolunteerTick);
        }
        await userApi.wait.bspVolunteer(1);
        await bspApi.wait.bspFileStorageComplete(fileKey);
        await userApi.wait.bspStored(1);
      } else {
        // If both BSPs can volunteer at the same time, advance to the tick where both can volunteer
        if ((await userApi.rpc.chain.getHeader()).number.toNumber() < bsp1VolunteerTick) {
          await userApi.block.skipTo(bsp1VolunteerTick);
        }
        // And wait for them to volunteer and confirm storing the file
        await userApi.wait.bspVolunteer(2);
        await bspApi.wait.bspFileStorageComplete(fileKey);
        await bspTwoApi.wait.bspFileStorageComplete(fileKey);
        await userApi.wait.bspStored(2);
      }

      await bspTwoApi.disconnect();
      await userApi.docker.stopBspContainer("sh-bsp-two");
    });

    it("BSP with reputation is prioritised", async () => {
      // Add a new, high reputation BSP
      const { rpcPort } = await addBsp(userApi, bspThreeKey, {
        name: "sh-bsp-three",
        bspKeySeed: bspThreeSeed,
        bspId: ShConsts.BSP_THREE_ID,
        additionalArgs: ["--keystore-path=/keystore/bsp-three"],
        bspStartingWeight: 800_000_000n
      });
      const bspThreeApi = await BspNetTestApi.create(`ws://127.0.0.1:${rpcPort}`);

      // Wait for it to catch up to the top of the chain
      await userApi.wait.bspCatchUpToChainTip(bspThreeApi);

      // Set global params to small numbers
      await userApi.sealBlock(
        userApi.tx.sudo.sudo(userApi.tx.fileSystem.setGlobalParameters(5, 100))
      );

      // Create a new storage request
      const { fileKey } = await userApi.file.newStorageRequest(
        "res/adolphus.jpg",
        "test/adolphus.jpg",
        "bucket-4"
      ); // T0

      // Query the earliest volunteer tick for the dummy BSP and the new BSP
      const initialBspVolunteerTick = (
        await userApi.call.fileSystemApi.queryEarliestFileVolunteerTick(
          ShConsts.DUMMY_BSP_ID,
          fileKey
        )
      ).asOk.toNumber();
      const highReputationBspVolunteerTick = (
        await userApi.call.fileSystemApi.queryEarliestFileVolunteerTick(
          ShConsts.BSP_THREE_ID,
          fileKey
        )
      ).asOk.toNumber();

      // Ensure that the new BSP should be able to volunteer first
      assert(
        highReputationBspVolunteerTick < initialBspVolunteerTick,
        "New BSP should be able to volunteer first"
      );

      // Advance to the tick where the new BSP can volunteer
      if (
        (await userApi.rpc.chain.getHeader()).number.toNumber() < highReputationBspVolunteerTick
      ) {
        await userApi.block.skipTo(highReputationBspVolunteerTick);
      }

      // Wait until the new BSP volunteers
      await userApi.wait.bspVolunteer(1);
      const matchedEvents = await userApi.assert.eventMany("fileSystem", "AcceptedBspVolunteer"); // T1

      const filtered = matchedEvents.filter(
        ({ event }) =>
          (userApi.events.fileSystem.AcceptedBspVolunteer.is(event) &&
            event.data.bspId.toString()) === ShConsts.BSP_THREE_ID
      );

      // Verify that the BSP with reputation is prioritised over the lower reputation BSPs
      assert(filtered.length === 1, "BSP with reputation should be prioritised");
      await userApi.docker.stopBspContainer("sh-bsp-three");
    });

    it(
      "BSP two cannot spam the chain to volunteer first",
      {
        skip: "Test takes way to long to run. This test actually spams the chain with transactions, unskip it if you want to run it."
      },
      async () => {
        await userApi.sealBlock(
          userApi.tx.sudo.sudo(userApi.tx.fileSystem.setGlobalParameters(2, 50))
        );

        const { fileKey } = await userApi.file.newStorageRequest(
          "res/cloud.jpg",
          "test/cloud.jpg",
          "bucket-3"
        ); // T0
        const bsp1VolunteerTick = (
          await userApi.call.fileSystemApi.queryEarliestFileVolunteerTick(
            ShConsts.DUMMY_BSP_ID,
            fileKey
          )
        ).asOk.toNumber();
        const bsp2VolunteerTick = (
          await userApi.call.fileSystemApi.queryEarliestFileVolunteerTick(
            ShConsts.BSP_TWO_ID,
            fileKey
          )
        ).asOk.toNumber();

        assert(bsp1VolunteerTick < bsp2VolunteerTick, "BSP one should be able to volunteer first");

        // BSP two tries to spam the chain to advance until it can volunteer
        if ((await userApi.rpc.chain.getHeader()).number.toNumber() !== bsp2VolunteerTick) {
          await userApi.block.skipTo(bsp2VolunteerTick, { spam: true, verbose: true });
        }

        const tickAfterSpamResult = (
          await userApi.call.proofsDealerApi.getCurrentTick()
        ).toNumber();

        assert(
          tickAfterSpamResult < bsp2VolunteerTick,
          "BSP two should not be able to spam the chain and reach his threshold to volunteer"
        );

        await userApi.docker.stopBspContainer("sh-bsp-two");
      }
    );
  }
);
