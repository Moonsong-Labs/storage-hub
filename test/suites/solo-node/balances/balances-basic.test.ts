import { after, before, describe, it } from "node:test";
import {
  alice,
  createSr25519Account,
  UNIT,
  type ExtendedApiPromise,
  bob,
  DevTestContext
} from "../../../util";
import { strictEqual } from "node:assert";

describe("Balances Pallet: Basic", {}, async () => {
  const context = new DevTestContext({});
  let api: ExtendedApiPromise;

  before(async () => {
    api = await context.initialize();
  });

  after(async () => {
    await context.dispose();
  });

  it("Can query balance", async () => {
    const {
      data: { free }
    } = await api.query.system.account(alice.address);
    console.log("Alice balance: ", free.toHuman());
    strictEqual(free.toBigInt() > 0n, true);
  });

  it("Can send balance to another account", async () => {
    const { address: randomId } = await createSr25519Account();
    const amount = 10n * UNIT;
    const {
      data: { free: balBefore }
    } = await api.query.system.account(randomId);
    strictEqual(balBefore.toBigInt(), 0n);

    console.log(`Sending balance to ${randomId}`);
    await api.tx.balances.transferAllowDeath(randomId, amount).signAndSend(alice);

    await api.createBlock();

    const {
      data: { free: balAfter }
    } = await api.query.system.account(randomId);
    strictEqual(balAfter.toBigInt(), amount);
  });

  it("Can display total issuance", async () => {
    const accountEntries = await api.query.system.account.entries();
    const balancesTotal = accountEntries.reduce(
      (acc, [, { data }]) => acc + data.free.toBigInt() + data.reserved.toBigInt(),
      0n
    );
    const totalSupply = await api.query.balances.totalIssuance();

    strictEqual(balancesTotal, totalSupply.toBigInt());
  });

  it("SetBalance fails when called without sudo", async () => {
    const { address: randomId } = await createSr25519Account();
    const {
      data: { free: balBefore }
    } = await api.query.system.account(randomId);

    await api.tx.balances.forceSetBalance(bob.address, 1337n).signAndSend(alice);
    await api.createBlock();

    const {
      data: { free: balAfter }
    } = await api.query.system.account(randomId);

    strictEqual(balBefore.sub(balAfter).toNumber(), 0);
  });

  it("SetBalance passes when called with sudo", async () => {
    const { address: randomId } = await createSr25519Account();

    const call = api.tx.balances.forceSetBalance(randomId, UNIT);
    await api.tx.sudo.sudo(call).signAndSend(alice);
    await api.createBlock();

    const {
      data: { free: balAfter }
    } = await api.query.system.account(randomId);

    strictEqual(balAfter.toBigInt(), UNIT);
  });
});
