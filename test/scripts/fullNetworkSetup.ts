import _ from "lodash";
import {
  alice,
  bsp,
  collator,
  getZombieClients,
  sendTransaction,
  waitForChain,
  waitForRandomness,
} from "../util";

const idealExecutorParams = [
  { maxMemoryPages: 8192 },
  { pvfExecTimeout: ["Backing", 2500] },
  { pvfExecTimeout: ["Approval", 15000] },
];

await using resources = await getZombieClients({
  relayWs: "ws://127.0.0.1:31000",
  // relayWs: "wss://rococo-rpc.polkadot.io",
  shWs: "ws://127.0.0.1:32000",
});

async function main() {
  const { storageApi, relayApi } = resources;

  await waitForChain(relayApi);

  // Check if executor parameters are set
  const { executorParams } = (await relayApi.query.configuration.activeConfig()).toJSON();
  if (_.isEqual(executorParams, idealExecutorParams)) {
    console.log("Executor parameters are already set to ideal values ✅");
  } else {
    const setConfig = relayApi.tx.configuration.setExecutorParams([
      // @ts-expect-error - ApiAugment not ready yet for SH
      { maxMemoryPages: 8192 },
      // @ts-expect-error - ApiAugment not ready yet for SH
      { pvfExecTimeout: ["Backing", 2500] },
      // @ts-expect-error - ApiAugment not ready yet for SH
      { pvfExecTimeout: ["Approval", 15000] },
    ]);

    // Setting Async Config
    process.stdout.write("Setting Executor Parameters config for relay chain... ");
    await sendTransaction(relayApi.tx.sudo.sudo(setConfig));
    process.stdout.write("✅\n");
  }

  await waitForChain(storageApi);

  // Settings Balances
  const {
    data: { free },
  } = await storageApi.query.system.account(bsp.address);

  if (free.toBigInt() < 1_000_000_000_000n) {
    const setBal = storageApi.tx.balances.forceSetBalance(bsp.address, 1000_000_000_000_000_000n);
    const setBal2 = storageApi.tx.balances.forceSetBalance(
      collator.address,
      1000_000_000_000_000_000n
    );

    process.stdout.write("Using sudo to increase BSP account balance... ");

    const { nonce } = await storageApi.query.system.account(alice.address);

    const tx1 = sendTransaction(storageApi.tx.sudo.sudo(setBal), {
      nonce: nonce.toNumber(),
    });
    const tx2 = sendTransaction(storageApi.tx.sudo.sudo(setBal2), {
      nonce: nonce.toNumber() + 1,
    });

    await Promise.all([tx1, tx2]);

    process.stdout.write("✅\n");
    const {
      data: { free },
    } = await storageApi.query.system.account(bsp.address);

    console.log(
      `BSP account balance reset by sudo, new free is ${free.toBigInt() / 10n ** 12n} balance ✅`
    );
  } else {
    console.log(`BSP account balance is  already ${free.toBigInt() / 10n ** 12n} balance ✅`);
  }

  // Enrolling BSP
  const string = "0x8e6a748e6d787260f47f61df1e2cac065db8c1d41428eb178102177876071c6b";
  const buffer = Buffer.from(string, "utf8");
  const uint8Array = new Uint8Array(buffer);

  process.stdout.write(`Requesting sign up for ${bsp.address} ...`);
  await sendTransaction(
    // @ts-expect-error - ApiAugment not ready yet for SH
    storageApi.tx.providers.requestBspSignUp(5000000, [uint8Array], bsp.address),
    {
      signer: bsp,
    }
  );
  process.stdout.write("✅\n");

  await waitForRandomness(storageApi);

  // Confirm sign up
  process.stdout.write(`Confirming sign up for ${bsp.address} ...`);
  // @ts-expect-error - ApiAugment not ready yet for SH
  await sendTransaction(storageApi.tx.providers.confirmSignUp(bsp.address), {
    signer: bsp,
  });
  process.stdout.write("✅\n");

  // TODO: ERROR: Error thrown when a user tries to confirm a sign up that was not requested previously.

  // Confirm providers added
  // @ts-expect-error - ApiAugment not ready yet for SH
  const providers = await storageApi.query.providers.backupStorageProviders.entries();

  if (providers.length === 1) {
    console.log("💫 Provider added correctly");
  } else {
    console.error("🪦 Provider not added correctly");
  }
}

main().finally(() => {
  try {
    resources.storageApi.disconnect();
    resources.relayApi.disconnect();
  } catch (e) {
    // ignore
  }
});
