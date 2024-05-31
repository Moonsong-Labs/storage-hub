import "@polkadot/api-augment";
import { after, before, describe, it } from "node:test";
import { expect } from "expect";
import {
  GenericContainer,
  type StartedTestContainer,
  Wait,
} from "testcontainers";
import { ApiPromise, Keyring, WsProvider } from "@polkadot/api";
import { accounts, getEd25519Account } from "../../util";

const UNIT = 1_000_000_000_000n;
describe("Sample Dev Node suite", () => {
  let container: StartedTestContainer;
  let api: ApiPromise;

  before(async () => {
    process.stdout.write("Starting container... ");
    container = await new GenericContainer("storage-hub:local")
      .withExposedPorts(9944)
      .withCommand([
        "--dev",
        "--rpc-cors=all",
        "--no-hardware-benchmarks",
        "--no-telemetry",
        "--no-prometheus",
        "--unsafe-rpc-external",
        "--sealing=instant",
      ])
      // replace with a health check
      .withWaitStrategy(Wait.forLogMessage("Development Service Ready"))
      // .withLogConsumer((stream) => {
      //   stream.on("data", (line) => console.log(line));
      //   stream.on("err", (line) => console.error(line));
      //   stream.on("end", () => console.log("Stream closed"));
      // })
      .start();
    process.stdout.write("✅\n");

    const connectString = `ws://${container.getHost()}:${container.getMappedPort(
      9944
    )}`;
    process.stdout.write(`Connecting APIs at ${connectString}... `);
    api = await ApiPromise.create({ provider: new WsProvider(connectString) });
    process.stdout.write("✅\n");
  });

  after(async () => {
    await api.disconnect();
    await container.stop();
  });

  it("Can query balance", async () => {
    const {
      data: { free },
    } = await api.query.system.account(accounts.alice.sr25519.id);
    console.log("Alice balance: ", free.toHuman());
    expect(free.toBigInt()).toBeGreaterThan(0n);
  });

  it("Can send balance to another account", async () => {
    const keyring = new Keyring({ type: "sr25519" });
    const alice = keyring.addFromUri("//Alice", { name: "Alice default" });
    const { id: randomId } = await getEd25519Account();
    const amount = 10n * UNIT;

    const {
      data: { free: balBefore },
    } = await api.query.system.account(randomId);
    expect(balBefore.toBigInt()).toBe(0n);

    console.log(`Sending balance to ${randomId}`);
    await api.tx.balances
      .transferAllowDeath(randomId, amount)
      .signAndSend(alice);

    const {
      data: { free: balAfter },
    } = await api.query.system.account(randomId);
    expect(balAfter.toBigInt()).toBe(amount);
  });
});

// const account = Sr25519Account.fromUri("//Alice");
// const signer = getPolkadotSigner(
//   account.publicKey,
//   "Sr25519",
//   async (input) => account.sign(input)
// );
// // const signedMsg = signer.sign("0x3902840088dc3417d5058ec4b4503e0c12ea1a0a89be200fe98922423d4334014fa6b0ee004598dcc610b1f8f3854fb533c6dda026e070c6df5f6dbdcfc37cd9056acd85a5fcee0d1df7acdfebbb810cf074a3e69363b535f093ac014b922ea4bddbf9a90f030000000a00008eaf04151687736326c9fea17e25fc5287613693c912909cb226aa4794f26a4802093d00")
// const amount = 1_000_000n;
// // const { id: randomId } = await getEd25519Account();
// const randomId = accounts.bob.sr25519.id;
// console.log(`Sending balance to ${randomId}`);
// const {
//   data: { free: balBefore },
// } = await api.query.System.Account.getValue(randomId);
// // expect(balBefore).toBe(0n);

// console.log(`Alice id is ${accounts.alice.sr25519.id}`);
// const tx = api.tx.Balances.transfer_allow_death({
//   dest: MultiAddress.Id(randomId),
//   value: amount,
// });
// console.log((await tx.getEncodedData()).asHex());

// const calldata= "0x0a00008eaf04151687736326c9fea17e25fc5287613693c912909cb226aa4794f26a4802093d00"
// console.log(signer.sign())

// const signed_tx = await tx.sign(signer);

// console.log(signed_tx);

// await new Promise((resolve) => setTimeout(resolve, 1000_000));
// await client.submit(signed_tx);

// const {
//   data: { free: balAfter },
// } = await api.query.System.Account.getValue(randomId);

// expect(balAfter).toBe(amount);
