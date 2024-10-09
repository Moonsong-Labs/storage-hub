import { describeBspNet, sleep, type EnrichedBspApi } from "../../../util";
import { assert } from "node:console";

describeBspNet(
  "BSP Automatic Tipping",
  { extrinsicRetryTimeout: 2 },
  ({ before, it, createUserApi }) => {
    let api: EnrichedBspApi;

    before(async () => {
      api = await createUserApi();
    });

    it("Confirm storing failure results in increased tip", async () => {
      await api.file.newStorageRequest("res/whatsup.jpg", "test/whatsup.jpg", "nothingmuch-2");
      await api.wait.bspVolunteer();

      // Wait for the bsp to send the first confirm storing extrinsic
      await api.wait.bspStoredInTxPool();

      // Wait for the bsp to send all the confirm retries
      await sleep(6000);
      await api.wait.bspStoredInTxPool(4);

      // We get the confirm storing pending extrinsics to get their extrinsic index
      const confirmStoringPendingMatches = await api.assert.extrinsicPresent({
        method: "bspConfirmStoring",
        module: "fileSystem",
        checkTxPool: true,
        assertLength: 4
      });

      const txPool = await api.rpc.author.pendingExtrinsics();

      const confirmStoringPendingExts = confirmStoringPendingMatches.map(
        (match) => txPool[match.extIndex]
      );

      for (let i = 1; i < confirmStoringPendingExts.length; ++i) {
        assert(
          confirmStoringPendingExts[i].tip.toBigInt() >
            confirmStoringPendingExts[i - 1].tip.toBigInt(),
          "Tip should increase with each retry"
        );
      }
    });
  }
);
