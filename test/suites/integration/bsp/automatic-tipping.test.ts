import {
  assertDockerLog,
  assertExtrinsicPresent,
  describeBspNet,
  type EnrichedBspApi,
  waitFor
} from "../../../util";

await describeBspNet(
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
      await userApi.wait.mspResponseInTxPool(1);
      await userApi.wait.bspVolunteer(1);

      await userApi.wait.bspStored({
        expectedExts: 1,
        timeoutMs: 12000,
        sealBlock: false
      });

      await assertDockerLog("storage-hub-sh-bsp-1", "attempt 1/", 40000);

      await assertDockerLog("storage-hub-sh-bsp-1", "attempt 2/", 40000);

      await assertDockerLog("storage-hub-sh-bsp-1", "attempt 3/", 40000);

      await assertDockerLog(
        "storage-hub-sh-bsp-1",
        "Failed to confirm file after 3 retries: Exhausted retry strategy",
        40000
      );

      await waitFor({
        lambda: async () => {
          // Find all bspConfirmStoring extrinsics in the pool
          let matches: { module: string; method: string; extIndex: number }[];
          try {
            matches = await assertExtrinsicPresent(userApi, {
              module: "fileSystem",
              method: "bspConfirmStoring",
              checkTxPool: true
            });
          } catch {
            // No matching extrinsics found yet
            return false;
          }

          // Get the actual extrinsics from the pool using the extIndex
          const txPool = await userApi.rpc.author.pendingExtrinsics();

          // With the requeue mechanism, extrinsics accumulate across retry cycles:
          // - Within a retry cycle: same nonce is reused, so transactions replace each other
          // - Between retry cycles: new nonce is assigned, so a new extrinsic is added
          // We verify the tipping mechanism works by checking any extrinsic has tip > 0.
          const hasExtrinsicWithTip = matches.some(
            (match) => txPool[match.extIndex].tip.toBigInt() > 0n
          );
          return hasExtrinsicWithTip;
        },
        iterations: 100,
        delay: 100
      });
    });
  }
);
