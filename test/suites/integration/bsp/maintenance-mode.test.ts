import { strictEqual } from "node:assert";
import { bspTwoKey, describeBspNet, type EnrichedBspApi, ShConsts } from "../../../util";

await describeBspNet(
  "BSPNet: Maintenance Mode Test",
  ({ before, it, createUserApi, createApi }) => {
    let userApi: EnrichedBspApi;
    let maintenanceBspApi: EnrichedBspApi;

    before(async () => {
      userApi = await createUserApi();

      // 1 block to maxthreshold (i.e. instant acceptance)
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
    });

    it("BSP in maintenance mode does not execute actions after block imports", async () => {
      // Onboard a BSP in maintenance mode
      const { rpcPort: maintenanceBspRpcPort } = await userApi.docker.onboardBsp({
        bspSigner: bspTwoKey,
        name: "sh-bsp-maintenance",
        bspId: ShConsts.BSP_TWO_ID,
        additionalArgs: [
          "--keystore-path=/keystore/bsp-two",
          "--maintenance-mode",
          "--extrinsic-retry-timeout=60"
        ],
        waitForIdle: true
      });

      console.log("BSP in maintenance mode started");

      // Connect to the BSP in maintenance mode
      maintenanceBspApi = await createApi(`ws://127.0.0.1:${maintenanceBspRpcPort}`);

      // Issue a storage request. The maintenance mode BSP should not volunteer for it since it won't process the request
      await userApi.file.createBucketAndSendNewStorageRequest(
        "res/adolphus.jpg",
        "cat/adolphus.jpg",
        "maintenance-mode-test"
      );

      // Wait for the two BSPs to volunteer. This will throw since the maintenance mode BSP won't process the request
      await userApi.assert
        .extrinsicPresent({
          module: "fileSystem",
          method: "bspVolunteer",
          checkTxPool: true,
          assertLength: 2,
          timeout: 15000
        })
        .then(
          (extrinsicArray) => {
            console.log("Extrinsic array:", extrinsicArray);
            throw new Error(
              "Expected assertion to fail because maintenance mode BSP should not volunteer"
            );
          },
          (error) => {
            console.log(
              "Expected error (maintenance BSP not volunteering):",
              error instanceof Error ? error.message : String(error)
            );
          }
        );

      // Verify that RPC calls still work on the maintenance BSP:
      // This should work even in maintenance mode
      const result = await maintenanceBspApi.rpc.storagehubclient.isFileInFileStorage(
        "0x0000000000000000000000000000000000000000000000000000000000000000"
      );

      // The specific result doesn't matter - what matters is that the call worked and didn't throw
      strictEqual(result !== undefined, true, "RPC calls should still work in maintenance mode");

      // Disconnect the maintenance BSP
      await userApi.docker.stopContainer("sh-bsp-maintenance");
      await maintenanceBspApi.disconnect();
    });
  }
);
