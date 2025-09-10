import { isDeepStrictEqual } from "node:util";
import {
  alice,
  bspKey,
  collator,
  getZombieClients,
  sendTransaction,
  waitForChain,
  waitForRandomness
} from "../util";

const idealExecutorParams = [
  { maxMemoryPages: 8192 },
  { pvfExecTimeout: ["Backing", 2500] },
  { pvfExecTimeout: ["Approval", 15000] }
];

async function main() {
  await using resources = await getZombieClients({
    relayWs: "ws://127.0.0.1:31000",
    // relayWs: "wss://rococo-rpc.polkadot.io",
    shWs: "ws://127.0.0.1:32000"
  });

  await waitForChain(resources.relayApi);

  // Check if executor parameters are set
  // @ts-expect-error mute anyJson error
  const { executorParams } = (await resources.relayApi.query.configuration.activeConfig()).toJSON();
  if (isDeepStrictEqual(executorParams, idealExecutorParams)) {
    console.log("Executor parameters are already set to ideal values ✅");
  } else {
    const setConfig = resources.relayApi.tx.configuration.setExecutorParams(idealExecutorParams);

    // Setting Async Config
    process.stdout.write("Setting Executor Parameters config for relay chain... ");
    await sendTransaction(resources.relayApi.tx.sudo.sudo(setConfig));
    process.stdout.write("✅\n");
  }

  await waitForChain(resources.storageApi);

  // Settings Balances
  const {
    data: { free }
  } = await resources.storageApi.query.system.account(bspKey.address);

  if (free.toBigInt() < 1_000_000_000_000n) {
    const setBal = resources.storageApi.tx.balances.forceSetBalance(
      bspKey.address,
      1000_000_000_000_000_000n
    );
    const setBal2 = resources.storageApi.tx.balances.forceSetBalance(
      collator.address,
      1000_000_000_000_000_000n
    );

    process.stdout.write("Using sudo to increase BSP account balance... ");

    const { nonce } = await resources.storageApi.query.system.account(alice.address);

    const tx1 = sendTransaction(resources.storageApi.tx.sudo.sudo(setBal), {
      nonce: nonce.toNumber()
    });
    const tx2 = sendTransaction(resources.storageApi.tx.sudo.sudo(setBal2), {
      nonce: nonce.toNumber() + 1
    });

    await Promise.all([tx1, tx2]);

    process.stdout.write("✅\n");
    const {
      data: { free }
    } = await resources.storageApi.query.system.account(bspKey.address);

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

  process.stdout.write(`Requesting sign up for ${bspKey.address} ...`);
  await sendTransaction(
    resources.storageApi.tx.providers.requestBspSignUp(5000000, [uint8Array], bspKey.address),
    {
      signer: bspKey
    }
  );
  process.stdout.write("✅\n");

  await waitForRandomness(resources.storageApi);

  // Confirm sign up
  process.stdout.write(`Confirming sign up for ${bspKey.address} ...`);
  await sendTransaction(resources.storageApi.tx.providers.confirmSignUp(bspKey.address), {
    signer: bspKey
  });
  process.stdout.write("✅\n");

  // Confirm providers added
  const providers = await resources.storageApi.query.providers.backupStorageProviders.entries();

  if (providers.length === 1) {
    console.log("💫 Provider added correctly");
  } else {
    console.error("🪦 Provider not added correctly");
  }
}

await await main();
