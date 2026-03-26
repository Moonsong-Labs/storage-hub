import assert, { strictEqual } from "node:assert";
import {
  addBsp,
  BspNetTestApi,
  bspDownKey,
  bspThreeKey,
  bspTwoKey,
  describeBspNet,
  type EnrichedBspApi,
  ShConsts
} from "../../../util";

await describeBspNet(
  "BSPNet: BSP Volunteering Thresholds",
  {
    initialised: false,
    bspStartingWeight: 100n,
    networkConfig: "standard"
  },
  ({ before, it, createUserApi, createBspApi }) => {
    let userApi: EnrichedBspApi;
    let bspApi: EnrichedBspApi;

    before(async () => {
      userApi = await createUserApi();
      bspApi = await createBspApi();
      const maxReplicationTargetRuntimeParameter = {
        RuntimeConfig: {
          MaxReplicationTarget: [null, 1]
        }
      };
      const tickRangeToMaximumThresholdRuntimeParameter = {
        RuntimeConfig: {
          TickRangeToMaximumThreshold: [null, 1]
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
    });

    it("Can set parameters of the file-system pallet", async () => {
      const maxReplicationTargetRuntimeParameter = {
        RuntimeConfig: {
          MaxReplicationTarget: [null, 87]
        }
      };
      const tickRangeToMaximumThresholdRuntimeParameter = {
        RuntimeConfig: {
          TickRangeToMaximumThreshold: [null, 200]
        }
      };
      const { extSuccess } = await userApi.block.seal({
        calls: [
          userApi.tx.sudo.sudo(
            userApi.tx.parameters.setParameter(maxReplicationTargetRuntimeParameter)
          )
        ]
      });
      strictEqual(extSuccess, true, "Extrinsic should be successful");
      const { extSuccess: extSuccessTwo } = await userApi.block.seal({
        calls: [
          userApi.tx.sudo.sudo(
            userApi.tx.parameters.setParameter(tickRangeToMaximumThresholdRuntimeParameter)
          )
        ]
      });
      strictEqual(extSuccessTwo, true, "Extrinsic should be successful");

      strictEqual(
        (
          await userApi.query.parameters.parameters({
            RuntimeConfig: {
              TickRangeToMaximumThreshold: null
            }
          })
        )
          .unwrap()
          .asRuntimeConfig.asTickRangeToMaximumThreshold.toNumber(),
        200,
        "Threshold should have changed"
      );

      strictEqual(
        (
          await userApi.query.parameters.parameters({
            RuntimeConfig: {
              MaxReplicationTarget: null
            }
          })
        )
          .unwrap()
          .asRuntimeConfig.asMaxReplicationTarget.toNumber(),
        87,
        "Max replication target should have changed"
      );
    });

    it("Shouldn't be able to set parameters without sudo", async () => {
      const maxReplicationTargetRuntimeParameter = {
        RuntimeConfig: {
          MaxReplicationTarget: [null, 13]
        }
      };
      const tickRangeToMaximumThresholdRuntimeParameter = {
        RuntimeConfig: {
          TickRangeToMaximumThreshold: [null, 37]
        }
      };
      const { extSuccess } = await userApi.block.seal({
        calls: [userApi.tx.parameters.setParameter(maxReplicationTargetRuntimeParameter)]
      });
      strictEqual(extSuccess, false, "Extrinsic should be unsuccessful");
      const { extSuccess: extSuccessTwo } = await userApi.block.seal({
        calls: [userApi.tx.parameters.setParameter(tickRangeToMaximumThresholdRuntimeParameter)]
      });
      strictEqual(extSuccessTwo, false, "Extrinsic should be unsuccessful");

      const { data } = await userApi.assert.eventPresent("system", "ExtrinsicFailed");
      const error = data[0].toString();
      strictEqual(error, "BadOrigin", "Extrinsic should fail with BadOrigin");

      strictEqual(
        (
          await userApi.query.parameters.parameters({
            RuntimeConfig: {
              TickRangeToMaximumThreshold: null
            }
          })
        )
          .unwrap()
          .asRuntimeConfig.asTickRangeToMaximumThreshold.toNumber(),
        200,
        "Threshold should not have changed"
      );
      strictEqual(
        (
          await userApi.query.parameters.parameters({
            RuntimeConfig: {
              MaxReplicationTarget: null
            }
          })
        )
          .unwrap()
          .asRuntimeConfig.asMaxReplicationTarget.toNumber(),
        87,
        "Max replication target should not have changed"
      );
    });

    it("Reputation increased on successful storage", { skip: "Not Implemented" }, async () => {
      const repBefore = (
        await userApi.query.providers.backupStorageProviders(ShConsts.DUMMY_BSP_ID)
      )
        .unwrap()
        .reputationWeight.toBigInt();
      await userApi.file.createBucketAndSendNewStorageRequest(
        "res/cloud.jpg",
        "test/cloud.jpg",
        "bucket-1"
      );
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
      let bspDownApi: EnrichedBspApi | undefined;
      try {
        const basicReplicationTargetRuntimeParameter = {
          RuntimeConfig: {
            BasicReplicationTarget: [null, 5]
          }
        };
        await userApi.block.seal({
          calls: [
            userApi.tx.sudo.sudo(
              userApi.tx.parameters.setParameter(basicReplicationTargetRuntimeParameter)
            )
          ]
        });

        const tickToMaximumThresholdRuntimeParameter = {
          RuntimeConfig: {
            TickRangeToMaximumThreshold: [null, 500]
          }
        };
        await userApi.block.seal({
          calls: [
            userApi.tx.sudo.sudo(
              userApi.tx.parameters.setParameter(tickToMaximumThresholdRuntimeParameter)
            )
          ]
        });

        const storageRequestTtlRuntimeParameter = {
          RuntimeConfig: {
            StorageRequestTtl: [null, 550]
          }
        };
        await userApi.block.seal({
          calls: [
            userApi.tx.sudo.sudo(
              userApi.tx.parameters.setParameter(storageRequestTtlRuntimeParameter)
            )
          ]
        });

        // Create a new BSP and onboard with no reputation
        const { rpcPort } = await addBsp(userApi, bspDownKey, userApi.accounts.sudo, {
          name: "sh-bsp-down",
          bspId: ShConsts.BSP_DOWN_ID,
          additionalArgs: ["--keystore-path=/keystore/bsp-down"],
          bspStartingWeight: 1n
        });
        bspDownApi = await BspNetTestApi.create(`ws://127.0.0.1:${rpcPort}`);

        // Wait for it to catch up to the tip of the chain
        await userApi.wait.nodeCatchUpToChainTip(bspDownApi);

        const { fileKey } = await userApi.file.createBucketAndSendNewStorageRequest(
          "res/whatsup.jpg",
          "test/whatsup.jpg",
          "bucket-1"
        );

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

        const currentTick = (await userApi.call.proofsDealerApi.getCurrentTick()).toNumber();
        assert(
          currentTick === normalReputationVolunteerTick,
          "The BSP with high reputation should be able to volunteer immediately"
        );
        assert(
          currentTick < lowReputationVolunteerTick,
          "The volunteer tick for the low reputation BSP should be in the future"
        );

        // Checking volunteering and confirming for the high reputation BSP
        await userApi.wait.bspVolunteer(1);
        await bspApi.wait.fileStorageComplete(fileKey);
        await userApi.wait.bspStored({ expectedExts: 1 });

        // Checking volunteering and confirming for the low reputation BSP
        // If a BSP can volunteer in tick X, it sends the extrinsic once it imports block with tick X - 1, so it gets included directly in tick X
        await userApi.block.skipTo(lowReputationVolunteerTick - 1);

        // Wait for the BSP to catch up to the new block height after skipping
        await userApi.wait.nodeCatchUpToChainTip(bspDownApi);

        await userApi.wait.bspVolunteer(1);
        const matchedEvents = await userApi.assert.eventMany("fileSystem", "AcceptedBspVolunteer"); // T1

        // Check that it is in fact the BSP with low reputation that just volunteered
        const filtered = matchedEvents.filter(
          ({ event }) =>
            (userApi.events.fileSystem.AcceptedBspVolunteer.is(event) &&
              event.data.bspId.toString()) === ShConsts.BSP_DOWN_ID
        );

        assert(
          filtered.length === 1,
          "Zero reputation BSP should be able to volunteer and be accepted"
        );
      } finally {
        if (bspDownApi) {
          await bspDownApi.disconnect();
        }
        await userApi.docker.stopContainer("sh-bsp-down");
      }
    });

    it("BSP two eventually volunteers after threshold curve is met", async () => {
      let bspTwoApi: EnrichedBspApi | undefined;
      try {
        const basicReplicationTargetRuntimeParameter = {
          RuntimeConfig: {
            BasicReplicationTarget: [null, 2]
          }
        };
        await userApi.block.seal({
          calls: [
            userApi.tx.sudo.sudo(
              userApi.tx.parameters.setParameter(basicReplicationTargetRuntimeParameter)
            )
          ]
        });

        const tickToMaximumThresholdRuntimeParameter = {
          RuntimeConfig: {
            TickRangeToMaximumThreshold: [null, 20]
          }
        };
        await userApi.block.seal({
          calls: [
            userApi.tx.sudo.sudo(
              userApi.tx.parameters.setParameter(tickToMaximumThresholdRuntimeParameter)
            )
          ]
        });

        // Add the second BSP
        const { rpcPort } = await addBsp(userApi, bspTwoKey, userApi.accounts.sudo, {
          name: "sh-bsp-two",
          bspId: ShConsts.BSP_TWO_ID,
          additionalArgs: ["--keystore-path=/keystore/bsp-two"]
        });
        bspTwoApi = await BspNetTestApi.create(`ws://127.0.0.1:${rpcPort}`);

        // Wait for it to catch up to the tip of the chain
        await userApi.wait.nodeCatchUpToChainTip(bspTwoApi);

        // Create a new storage request
        const { fileKey } = await userApi.file.createBucketAndSendNewStorageRequest(
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

        assert(bsp1VolunteerTick < bsp2VolunteerTick, "BSP one should be able to volunteer first");
        const currentBlockNumber = (await userApi.rpc.chain.getHeader()).number.toNumber();
        assert(
          currentBlockNumber === bsp1VolunteerTick,
          "BSP one should be able to volunteer immediately"
        );

        await userApi.wait.bspVolunteer(1);
        await bspApi.wait.fileStorageComplete(fileKey);
        await userApi.wait.bspStored({ expectedExts: 1 });

        // Then wait for the second BSP to volunteer and confirm storing the file
        // If a BSP can volunteer in tick X, it sends the extrinsic once it imports block with tick X - 1, so it gets included directly in tick X
        await userApi.block.skipTo(bsp2VolunteerTick - 1);

        // Wait for BSP two to catch up to the new block height after skipping
        await userApi.wait.nodeCatchUpToChainTip(bspTwoApi);

        await userApi.wait.bspVolunteer(1);
        await bspTwoApi.wait.fileStorageComplete(fileKey);
        await userApi.wait.bspStored({ expectedExts: 1 });
      } finally {
        if (bspTwoApi) {
          await bspTwoApi.disconnect();
        }
        await userApi.docker.stopContainer("sh-bsp-two");
      }
    });

    it("BSP with reputation is prioritised", async () => {
      let bspThreeApi: EnrichedBspApi | undefined;
      try {
        // Add a new, high reputation BSP
        const { rpcPort } = await addBsp(userApi, bspThreeKey, userApi.accounts.sudo, {
          name: "sh-bsp-three",
          bspId: ShConsts.BSP_THREE_ID,
          additionalArgs: ["--keystore-path=/keystore/bsp-three"],
          bspStartingWeight: 800_000_000n
        });
        bspThreeApi = await BspNetTestApi.create(`ws://127.0.0.1:${rpcPort}`);

        // Wait for it to catch up to the top of the chain
        await userApi.wait.nodeCatchUpToChainTip(bspThreeApi);

        // Set max replication target and tick to maximum threshold to small numbers
        const maxReplicationTargetRuntimeParameter = {
          RuntimeConfig: {
            MaxReplicationTarget: [null, 5]
          }
        };
        // In order to test the reputation prioritisation, we need to set the tick to maximum
        // threshold to a high enough number such that
        // highReputationBspVolunteerTick - initialBspVolunteerTick > 2 (not 1!).
        const tickRangeToMaximumThresholdRuntimeParameter = {
          RuntimeConfig: {
            TickRangeToMaximumThreshold: [null, 9001]
          }
        };
        const storageRequestTtlRuntimeParameter = {
          RuntimeConfig: {
            StorageRequestTtl: [null, 110]
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
        await userApi.block.seal({
          calls: [
            userApi.tx.sudo.sudo(
              userApi.tx.parameters.setParameter(storageRequestTtlRuntimeParameter)
            )
          ]
        });

        // Create a new storage request
        const { fileKey } = await userApi.file.createBucketAndSendNewStorageRequest(
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
        const currentBlockNumber = (await userApi.rpc.chain.getHeader()).number.toNumber();
        assert(
          currentBlockNumber === highReputationBspVolunteerTick,
          "BSP with high reputation should be able to volunteer immediately"
        );

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
      } finally {
        if (bspThreeApi) {
          await bspThreeApi.disconnect();
        }
        await userApi.docker.stopContainer("sh-bsp-three");
      }
    });

    it(
      "BSP two cannot spam the chain to volunteer first",
      {
        skip: "Test takes way to long to run. This test actually spams the chain with transactions, unskip it if you want to run it."
      },
      async () => {
        const basicReplicationTargetRuntimeParameter = {
          RuntimeConfig: {
            BasicReplicationTarget: [null, 2]
          }
        };
        await userApi.block.seal({
          calls: [
            userApi.tx.sudo.sudo(
              userApi.tx.parameters.setParameter(basicReplicationTargetRuntimeParameter)
            )
          ]
        });

        const tickToMaximumThresholdRuntimeParameter = {
          RuntimeConfig: {
            TickRangeToMaximumThreshold: [null, 50]
          }
        };
        await userApi.block.seal({
          calls: [
            userApi.tx.sudo.sudo(
              userApi.tx.parameters.setParameter(tickToMaximumThresholdRuntimeParameter)
            )
          ]
        });

        const { fileKey } = await userApi.file.createBucketAndSendNewStorageRequest(
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
          await userApi.block.skipTo(bsp2VolunteerTick, {
            spam: true,
            verbose: true
          });
        }

        const tickAfterSpamResult = (
          await userApi.call.proofsDealerApi.getCurrentTick()
        ).toNumber();

        assert(
          tickAfterSpamResult < bsp2VolunteerTick,
          "BSP two should not be able to spam the chain and reach his threshold to volunteer"
        );

        await userApi.docker.stopContainer("sh-bsp-two");
      }
    );
  }
);
