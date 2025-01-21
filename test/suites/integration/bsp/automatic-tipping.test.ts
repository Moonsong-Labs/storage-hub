import { describeBspNet, type EnrichedBspApi } from "../../../util";
import assert from "node:assert";

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

      await userApi.wait.bspStoredInTxPool({ expectedExts: 4, timeoutMs: 12000 });

      const confirmStoringPendingMatches = await userApi.assert.extrinsicPresent({
        method: "bspConfirmStoring",
        module: "fileSystem",
        checkTxPool: true,
        assertLength: 4
      });

      const txPool = await userApi.rpc.author.pendingExtrinsics();

      const tips = confirmStoringPendingMatches.map((match) =>
        txPool[match.extIndex].tip.toBigInt()
      );
      const isIncreasing = tips.slice(1).every((current, i) => current > tips[i]);

      assert(isIncreasing, "Tip should increase with each retry");
    });
  }
);
