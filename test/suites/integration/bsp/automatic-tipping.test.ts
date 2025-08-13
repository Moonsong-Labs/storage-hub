import { assertDockerLog, describeBspNet, waitFor, type EnrichedBspApi } from "../../../util";

describeBspNet(
  "BSP Automatic Tipping",
  { extrinsicRetryTimeout: 2 },
  ({ before, it, createUserApi }) => {
    let userApi: EnrichedBspApi;

    before(async () => {
      userApi = await createUserApi();
    });

    it("Confirm storing failure results in increased tip", async () => {
      await userApi.file.createBucketAndSendNewStorageRequest(
        "res/whatsup.jpg",
        "test/whatsup.jpg",
        "nothingmuch-2"
      );
      await userApi.wait.bspVolunteer(1);

      await userApi.wait.bspStored({
        expectedExts: 1,
        timeoutMs: 12000,
        sealBlock: false
      });

      await assertDockerLog("storage-hub-sh-bsp-1", "attempt #1", 40000);

      await assertDockerLog("storage-hub-sh-bsp-1", "attempt #2", 40000);

      await assertDockerLog("storage-hub-sh-bsp-1", "attempt #3", 40000);

      await assertDockerLog(
        "storage-hub-sh-bsp-1",
        "Failed to confirm file after 3 retries: Exhausted retry strategy",
        40000
      );

      await waitFor({
        lambda: async () => {
          const confirmStoringMatch = await userApi.assert.extrinsicPresent({
            method: "bspConfirmStoring",
            module: "fileSystem",
            checkTxPool: true,
            assertLength: 1
          });
          const txPool = await userApi.rpc.author.pendingExtrinsics();
          const tip = txPool[confirmStoringMatch[0].extIndex].tip.toBigInt();
          const nonce = txPool[confirmStoringMatch[0].extIndex].nonce;
          return tip > 0 && nonce.toNumber() === 1;
        },
        iterations: 100,
        delay: 100
      });
    });
  }
);
