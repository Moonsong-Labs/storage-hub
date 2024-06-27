import _ from "lodash";
import {
  alice,
  bsp,
  CAPACITY_512,
  collator,
  DUMMY_MSP_ID,
  getZombieClients,
  sendTransaction,
  shUser,
  VALUE_PROP,
  waitForChain,
  waitForRandomness,
  ZOMBIE_RELAY_URL,
  ZOMBIE_SH_URL,
} from "../util";

const idealExecutorParams = [
  { maxMemoryPages: 8192 },
  { pvfExecTimeout: ["Backing", 2500] },
  { pvfExecTimeout: ["Approval", 15000] },
];

async function main() {
  await using resources = await getZombieClients({
    relayWs: ZOMBIE_RELAY_URL,
    shWs: ZOMBIE_SH_URL,
  });

  await waitForChain(resources.relayApi);

  // Check if executor parameters are set
  const { executorParams } = (await resources.relayApi.query.configuration.activeConfig()).toJSON();
  if (_.isEqual(executorParams, idealExecutorParams)) {
    console.log("Executor parameters are already set to ideal values ✅");
  } else {
    const setConfig = resources.relayApi.tx.configuration.setExecutorParams(
      // @ts-expect-error - ApiAugment issue
      idealExecutorParams
    );

    // Setting Async Config
    process.stdout.write("Setting Executor Parameters config for relay chain... ");
    await sendTransaction(resources.relayApi.tx.sudo.sudo(setConfig));
    process.stdout.write("✅\n");
  }

  await waitForChain(resources.storageApi);

  // Settings Balances
  const {
    data: { free },
  } = await resources.storageApi.query.system.account(bsp.address);

  if (free.toBigInt() < 1_000_000_000_000n) {
    const setBal = resources.storageApi.tx.balances.forceSetBalance(
      bsp.address,
      1000_000_000_000_000_000n
    );
    const setBal2 = resources.storageApi.tx.balances.forceSetBalance(
      collator.address,
      1000_000_000_000_000_000n
    );

    const setBal3 = resources.storageApi.tx.balances.forceSetBalance(
      shUser.address,
      1000_000_000_000_000_000n
    );

    process.stdout.write("Using sudo to increase BSP account balance... ");

    const { nonce } = await resources.storageApi.query.system.account(alice.address);

    const tx1 = sendTransaction(resources.storageApi.tx.sudo.sudo(setBal), {
      nonce: nonce.toNumber(),
    });
    const tx2 = sendTransaction(resources.storageApi.tx.sudo.sudo(setBal2), {
      nonce: nonce.toNumber() + 1,
    });
    const tx3 = sendTransaction(resources.storageApi.tx.sudo.sudo(setBal3), {
      nonce: nonce.toNumber() + 2,
    });

    await Promise.all([tx1, tx2, tx3]);

    process.stdout.write("✅\n");
    const {
      data: { free },
    } = await resources.storageApi.query.system.account(bsp.address);

    console.log(
      `BSP account balance reset by sudo, new free is ${free.toBigInt() / 10n ** 12n} balance ✅`
    );
  } else {
    console.log(`BSP account balance is  already ${free.toBigInt() / 10n ** 12n} balance ✅`);
  }

  // This is the fingerprint of  "../resource/whatsup.jpg" which is adjusted for native runner
  const bspId = "0x002aaf768af5b738eea96084f10dac7ad4f6efa257782bdb9823994ffb233300";

  // Enrolling BSP
  const bspMultiAddress = (await resources.storageApi.rpc.system.localListenAddresses()).find(
    (peer) => peer.includes("127.0.0.1")
  );

  if (!bspMultiAddress) {
    throw new Error("BSP MultiAddress not found");
  }

  // TODO: Remove Sudo and use proper flow
  process.stdout.write(`Forcing BSP sign up for ${bsp.address} ...`);
  await sendTransaction(
    resources.storageApi.tx.providers.forceBspSignUp(
      bsp.address,
      bspId,
      CAPACITY_512,
      [bspMultiAddress.toString()],
      bsp.address
    )
  );
  process.stdout.write("✅\n");

  // TODO: Remove Sudo and use proper flow
  process.stdout.write(`Forcing BSP sign up for ${bsp.address} ...`);
  await sendTransaction(
    resources.storageApi.tx.sudo.sudo(
      resources.storageApi.tx.providers.forceMspSignUp(
        alice.address,
        DUMMY_MSP_ID,
        CAPACITY_512,
        [bspMultiAddress.toString()],
        {
          identifier: VALUE_PROP,
          dataLimit: 500,
          protocols: ["https", "ssh", "telnet"],
        },
        alice.address
      )
    )
  );
  process.stdout.write("✅\n");

  // Confirm providers added
  const bsps = await resources.storageApi.query.providers.backupStorageProviders.entries();

  const msps = await resources.storageApi.query.providers.mainStorageProviders.entries();

  if (bsps.length === 1 && msps.length === 1) {
    console.log("💫 Providers added correctly");
  } else {
    console.error("🪦 Providers not added correctly");
    console.error("BSps: ", bsps);
    console.error("MSps: ", msps);
  }
}

main();
