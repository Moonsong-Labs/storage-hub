import { MultiAddress } from "@polkadot-api/descriptors";
import { accounts, getZombieClients, waitForChain, waitForRandomness } from "../util";
import { Binary } from "polkadot-api";
import { isEqual } from "lodash";

const idealExecutorParams = [
  {
    type: "MaxMemoryPages",
    value: 8192,
  },
  {
    type: "PvfExecTimeout",
    value: [
      {
        type: "Backing",
        value: undefined,
      },
      2500n,
    ],
  },
  {
    type: "PvfExecTimeout",
    value: [
      {
        type: "Approval",
        value: undefined,
      },
      15000n,
    ],
  },
];

const { relayClient, relayApi, shClient, storageApi } = await getZombieClients({
  relayWs: "ws://127.0.0.1:31000",
  // relayWs: "wss://rococo-rpc.polkadot.io",
  shWs: "ws://127.0.0.1:32000",
});

async function main() {
  await waitForChain(relayClient);

  // Check if executor parameters are set

  const { executor_params } = await relayApi.query.Configuration.ActiveConfig.getValue();

  if (isEqual(executor_params, idealExecutorParams)) {
    console.log("Executor parameters are already set to ideal values ✅");
  } else {
    // Increasing times for the relay chain
    const setConfig = relayApi.tx.Configuration.set_executor_params({
      new: [
        { type: "MaxMemoryPages", value: 8192 },
        {
          type: "PvfExecTimeout",
          value: [{ type: "Backing", value: undefined }, 2500n],
        },
        {
          type: "PvfExecTimeout",
          value: [{ type: "Approval", value: undefined }, 15000n],
        },
      ],
    }).decodedCall;

    // Setting Async Config
    process.stdout.write("Setting Executor Parameters config for relay chain... ");
    await relayApi.tx.Sudo.sudo({ call: setConfig }).signAndSubmit(accounts.alice.sr25519.signer);
    process.stdout.write("✅\n");
  }

  await waitForChain(shClient);

  // Settings Balances
  const {
    data: { free },
  } = await storageApi.query.System.Account.getValue(accounts.bsp.sr25519.id);

  if (free < 1_000_000_000_000n) {
    const setbalance = storageApi.tx.Balances.force_set_balance({
      who: MultiAddress.Id(accounts.bsp.sr25519.id),
      new_free: 1000_000_000_000_000_000n,
    }).decodedCall;

    const setbalance2 = storageApi.tx.Balances.force_set_balance({
      who: MultiAddress.Id(accounts.collator.sr25519.id),
      new_free: 1000_000_000_000_000_000n,
    }).decodedCall;

    process.stdout.write("Using sudo to increase BSP account balance... ");

    const { nonce } = await storageApi.query.System.Account.getValue(accounts.alice.sr25519.id);
    const tx1 = storageApi.tx.Sudo.sudo({ call: setbalance }).signAndSubmit(
      accounts.alice.sr25519.signer,
      { nonce }
    );

    const tx2 = storageApi.tx.Sudo.sudo({ call: setbalance2 }).signAndSubmit(
      accounts.alice.sr25519.signer,
      { nonce: nonce + 1 }
    );

    await Promise.all([tx1, tx2]);

    process.stdout.write("✅\n");
    const {
      data: { free },
    } = await storageApi.query.System.Account.getValue(accounts.bsp.sr25519.id);

    console.log(`BSP account balance reset by sudo, new free is ${free / 10n ** 12n} balance ✅`);
  } else {
    console.log(`BSP account balance is  already ${free / 10n ** 12n} balance ✅`);
  }
  // Enrolling BSP
  const string = "0x8e6a748e6d787260f47f61df1e2cac065db8c1d41428eb178102177876071c6b";
  const buffer = Buffer.from(string, "utf8");
  const uint8Array = new Uint8Array(buffer);

  process.stdout.write(`Requesting sign up for ${accounts.bsp.sr25519.id} ...`);
  await storageApi.tx.Providers.request_bsp_sign_up({
    capacity: 5000000,
    multiaddresses: [new Binary(uint8Array)],
  }).signAndSubmit(accounts.bsp.sr25519.signer);
  process.stdout.write("✅\n");

  await waitForRandomness(storageApi);

  // Confirm sign up
  process.stdout.write(`Confirming sign up for ${accounts.bsp.sr25519.id} ...`);
  await storageApi.tx.Providers.confirm_sign_up({
    provider_account: accounts.bsp.sr25519.id,
  }).signAndSubmit(accounts.bsp.sr25519.signer);
  process.stdout.write("✅\n");

  // TODO: ERROR: Error thrown when a user tries to confirm a sign up that was not requested previously.

  // Confirm providers added
  const providers = await storageApi.query.Providers.BackupStorageProviders.getEntries();

  if (providers.length === 1) {
    console.log("💫 Provider added correctly");
  } else {
    console.error("🪦 Provider not added correctly");
  }
}

main().finally(() => {
  shClient.destroy();
  relayClient.destroy();
});
