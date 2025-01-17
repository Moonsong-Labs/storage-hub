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
      // Make a storage request and wait for the bsp to volunteer
      await userApi.file.createBucketAndSendNewStorageRequest(
        "res/whatsup.jpg",
        "test/whatsup.jpg",
        "nothingmuch-2"
      );
      await userApi.wait.bspVolunteer(1);

      // Wait for the bsp to send all the confirm retries
      await userApi.wait.bspStoredInTxPool({ expectedExts: 4, timeoutMs: 12000 });

      // We get the confirm storing pending extrinsics to get their extrinsic index
      const confirmStoringPendingMatches = await userApi.assert.extrinsicPresent({
        method: "bspConfirmStoring",
        module: "fileSystem",
        checkTxPool: true,
        assertLength: 4
      });

      const txPool = await userApi.rpc.author.pendingExtrinsics();

      const tips = confirmStoringPendingMatches.map(
        (match) => txPool[match.extIndex].tip.toBigInt()
      );

      // Log all tips first
      console.log(
        "All tips:",
        tips
      );

      const isIncreasing = tips
        .slice(1)
        .every((current, i) => current > tips[i]);

      assert(isIncreasing, "Tip should increase with each retry");
    });
  }
);
