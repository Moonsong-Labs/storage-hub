import assert from "node:assert";
import { JsonRpcProvider, Wallet } from "ethers";
import { describeBspNet, type EnrichedBspApi, ShConsts, waitFor } from "../../../util";
import { ALITH_PRIVATE_KEY, ETH_BSP_ADDRESS } from "../../../util/evmNet/keyring";

await describeBspNet(
  "Solochain EVM ETH Transfer",
  { initialised: false, networkConfig: "standard", runtimeType: "solochain", keepAlive: false },
  ({ before, it, createUserApi }) => {
    let userApi: EnrichedBspApi;

    before(async () => {
      userApi = await createUserApi();
    });

    it("Can send ETH to the BSP", async () => {
      const rpcUrl = `http://127.0.0.1:${ShConsts.NODE_INFOS.user.port}`;
      const provider = new JsonRpcProvider(rpcUrl);

      const sender = new Wallet(ALITH_PRIVATE_KEY, provider);
      const recipient = ETH_BSP_ADDRESS;

      const balanceSenderBefore = await provider.getBalance(await sender.getAddress());
      const balanceRecipientBefore = await provider.getBalance(recipient);

      // Estimate a safe value that leaves enough room for gas
      const fee = await provider.getFeeData();
      const gasLimit = 21000n;
      const gasPriceForCalc = fee.gasPrice ?? fee.maxFeePerGas ?? 1n;
      const maxSendable = balanceSenderBefore - gasPriceForCalc * gasLimit;
      assert(maxSendable > 0n, "Sender has insufficient balance to cover gas");

      const value = maxSendable / 100n > 0n ? maxSendable / 100n : 1n; // send ~1% or 1 wei
      console.log("HELLO THERE: value", value);

      const tx = await sender.sendTransaction(
        fee.maxFeePerGas && fee.maxPriorityFeePerGas
          ? {
              to: recipient,
              value,
              gasLimit,
              maxFeePerGas: fee.maxFeePerGas,
              maxPriorityFeePerGas: fee.maxPriorityFeePerGas
            }
          : { to: recipient, value, gasLimit, gasPrice: fee.gasPrice ?? gasPriceForCalc }
      );

      // Manual sealing is enabled; mine a block so the tx gets included
      await userApi.block.seal();

      const receipt = await tx.wait();
      assert(receipt?.status === 1, "Transaction failed");

      await waitFor({
        lambda: async () => {
          // Query the balance of the sender and recipient
          const balanceSenderAfter = await provider.getBalance(await sender.getAddress());
          const balanceRecipientAfter = await provider.getBalance(recipient);

          // Check if the balance of the recipient has increased and the balance of the sender has decreased
          return (
            balanceRecipientAfter > balanceRecipientBefore &&
            balanceSenderAfter < balanceSenderBefore
          );
        }
      });
    });
  }
);
