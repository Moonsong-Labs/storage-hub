import { describeBspNet, type EnrichedBspApi } from "../../../util";
import { alith } from "../../../util/evmNet/keyring";

await describeBspNet(
  "Solochain EVM ETH Transfer",
  { initialised: true, networkConfig: "standard", runtimeType: "solochain", keepAlive: true },
  ({ before, it, createUserApi }) => {
    let userApi: EnrichedBspApi;

    before(async () => {
      userApi = await createUserApi();
    });

    it("Can send ETH to the BSP", async () => {
      const tx = await userApi.tx.ethereum.transact(
        "0x0101010101010101010101010101010101010101010101010101010101010101"
      );
      await tx.signAndSend(alith);
      await userApi.wait.bspVolunteer(1);
      await userApi.wait.bspStored({ expectedExts: 1, timeoutMs: 12000, sealBlock: false });
    });
  }
);
