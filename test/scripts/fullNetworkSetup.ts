import { MultiAddress } from "@polkadot-api/descriptors";
import { SH_BSP, accounts, getZombieClients, waitForChain } from "../util";
import { Binary } from "polkadot-api";

const { relayClient, relayApi, shClient, storageApi } = await getZombieClients({
  relayWs: "ws://127.0.0.1:31000",
  shWs: "ws://127.0.0.1:33000",
});

async function main() {
  await  waitForChain(relayClient)

  // Increasing times for the relay chain
  const setConfig = relayApi.tx.Configuration.set_executor_params({
    new: [
      { type: "MaxMemoryPages", value: 8192 },
      { type: "PvfExecTimeout", value: [{type: "Backing"}, 2500n]},
      { type: "PvfExecTimeout", value: [{type: "Approval"}, 15000n]},
    ],
  }).decodedCall;

  // Setting Async Config
  process.stdout.write("Setting Executor Parameters config for relay chain... ");
  await relayApi.tx.Sudo.sudo({ call: setConfig }).signAndSubmit(
    accounts.alice.sr25519.signer
  );
  process.stdout.write("✅\n");

  await waitForChain(shClient)
  // Settings Balances
  const {
    data: { free },
  } = await storageApi.query.System.Account.getValue(SH_BSP);

  if (free < 1000_000_000_000_000n) {
    const setbalance = storageApi.tx.Balances.force_set_balance({
      who: MultiAddress.Id(SH_BSP),
      new_free: 1000_000_000_000_000n,
    }).decodedCall;

    process.stdout.write("Using sudo to increase BSP account balance... ");
    await storageApi.tx.Sudo.sudo({ call: setbalance }).signAndSubmit(
      accounts.alice.sr25519.signer
    );

    process.stdout.write("✅\n");
    const {
      data: { free },
    } = await storageApi.query.System.Account.getValue(SH_BSP);

    console.log(
      `BSP account balance reset by sudo, new free is ${
        free / 10n ** 12n
      } balance ✅`
    );
  } else {
    console.log(`BSP account balance is  already ${free / 10n ** 12n} balance ✅`);
  }
  // Enrolling BSP
  const string =
    "0x8e6a748e6d787260f47f61df1e2cac065db8c1d41428eb178102177876071c6b";
  const buffer = Buffer.from(string, "utf8");
  const uint8Array = new Uint8Array(buffer);

  process.stdout.write(`Requesting sign up for ${SH_BSP} ...`);
  await storageApi.tx.Providers.request_bsp_sign_up({
    capacity: 5000000,
    multiaddresses: [new Binary(uint8Array)],
  }).signAndSubmit(accounts.alice.sr25519.signer);
  process.stdout.write("✅\n");

  // Wait for randomness (9 blocks)
  // TODO: Wait for 9 blocks, not 1 minute
  process.stdout.write("Waiting for randomness ...");
  await waitForChain(shClient, 60000);
  process.stdout.write("✅\n");


  // Confirm sign up
  console.log(`Confirming sign up for ${SH_BSP} ...`);
  await storageApi.tx.Providers.confirm_sign_up({
    provider_account: accounts.bsp.sr25519.id,
  }).signAndSubmit(accounts.alice.sr25519.signer);
  process.stdout.write("✅\n");

  // TODO: ERROR: Error thrown when a user tries to confirm a sign up that was not requested previously.


  // Confirm providers added
  const providers =
    await storageApi.query.Providers.BackupStorageProviders.getEntries();

  console.log(providers);
}

main().finally(() => {
  shClient.destroy();
  relayClient.destroy();
});
