import { describe, it, before, after } from "node:test";
import { expect } from "expect";
import {Sr25519Account} from "@unique-nft/sr25519"
import {
  GenericContainer,
  type StartedTestContainer,
  Wait,
} from "testcontainers";
import { createClient, type PolkadotClient, type TypedApi } from "polkadot-api";
import { WebSocketProvider } from "polkadot-api/ws-provider/node";
import { MultiAddress, storagehub } from "@polkadot-api/descriptors";
import { accounts, getEd25519Account } from "../../util";
import { getPolkadotSigner } from "polkadot-api/signer";


// TODO: Run typegen against a local dev first
// do this via creating a new script

describe("Sample Dev Node suite", () => {
  let container: StartedTestContainer;
  let client: PolkadotClient;
  let api: TypedApi<typeof storagehub>;
  // biome-ignore lint/suspicious/noExplicitAny: <explanation>
  let runtime: any;

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

    // const connectString = `ws://${container.getHost()}:${container.getMappedPort(
    //   9944
    // )}`;

    const connectString = "ws://localhost:9944";

    process.stdout.write(`Connecting APIs at ${connectString}... `);
    client = createClient(WebSocketProvider(connectString));
    api = client.getTypedApi(storagehub);
    runtime = await api.runtime.latest();
    process.stdout.write("✅\n");
  });

  after(async () => {
    await new Promise((resolve) => setTimeout(resolve, 30_000));
    client.destroy();
    await container.stop();
  });

  it("Can query balance", async () => {
    const balance = (
      await api.query.System.Account.getValue(accounts.alice.sr25519.id)
    ).data.free;
    console.log("Alice balance: ", balance);
    expect(balance).toBeGreaterThan(0);
  });

  it("Can send balance to another account", async () => {
    const account =Sr25519Account.fromUri("//Alice")

    const signer  = getPolkadotSigner(account.publicKey, "Sr25519", async(input)=> account.sign(input))
    const amount = 1_000_000n;
    // const { id: randomId } = await getEd25519Account();
    const randomId = accounts.bob.sr25519.id
    console.log(`Sending balance to ${randomId}`);
    const {
      data: { free: balBefore },
    } = await api.query.System.Account.getValue(randomId);
    // expect(balBefore).toBe(0n);

    const tx = await api.tx.Balances.transfer_allow_death({
      dest: MultiAddress.Id(randomId),
      value: 3n,
    }).sign(accounts.alice.sr25519.signer);

    console.log("Sending tx: ", tx);

    await client.submit(tx);

    const {
      data: { free: balAfter },
    } = await api.query.System.Account.getValue(randomId);

    expect(balAfter).toBe(amount);
  });
});
