import { expect } from "expect";
import { after, before, describe, it } from "node:test";
import {
  alice,
  type StartedTestContainer,
  createSr25519Account,
  devnodeSetup,
  UNIT,
  type ExtendedApiPromise,
  bob,
} from "../../../util";

describe("Balances Pallet: Basic", () => {
  let container: StartedTestContainer;
  let api: ExtendedApiPromise;

  before(async () => {
    const { extendedApi, runningContainer } = await devnodeSetup({
      // keepOpen: true,
    });
    api = extendedApi;
    container = runningContainer;
  });

  // TODO: Clear this up automatically
  after(async () => {
    await api.disconnect();
    await container.stop();
  });

  it("Can query balance", async () => {
    const {
      data: { free },
    } = await api.query.system.account(alice.address);
    console.log("Alice balance: ", free.toHuman());
    expect(free.toBigInt()).toBeGreaterThan(0n);
  });

  it("Can send balance to another account", async () => {
    const { address: randomId } = await createSr25519Account();
    const amount = 10n * UNIT;
    const {
      data: { free: balBefore },
    } = await api.query.system.account(randomId);
    expect(balBefore.toBigInt()).toBe(0n);

    console.log(`Sending balance to ${randomId}`);
    await api.tx.balances.transferAllowDeath(randomId, amount).signAndSend(alice);

    await api.createBlock();

    const {
      data: { free: balAfter },
    } = await api.query.system.account(randomId);
    expect(balAfter.toBigInt()).toBe(amount);
  });

  it("Can display total issuance", { only: true }, async () => {
    const accountEntries = await api.query.system.account.entries();
    const balancesTotal = accountEntries.reduce(
      (acc, [, { data }]) => acc + data.free.toBigInt() + data.reserved.toBigInt(),
      0n
    );
    const totalSupply = await api.query.balances.totalIssuance();

    expect(balancesTotal).toBe(totalSupply.toBigInt());
  });

  it("SetBalance fails when called without sudo", { only: true }, async () => {
    const { address: randomId } = await createSr25519Account();
    const {
      data: { free: balBefore },
    } = await api.query.system.account(randomId);

    await api.tx.balances.forceSetBalance(bob.address, 1337n).signAndSend(alice);
    await api.createBlock();

    const {
      data: { free: balAfter },
    } = await api.query.system.account(randomId);

    expect(balBefore.sub(balAfter).toNumber()).toBe(0);
  });

  it("SetBalance passes when called with sudo", { only: true }, async () => {
    const { address: randomId } = await createSr25519Account();

    const call = api.tx.balances.forceSetBalance(randomId, UNIT);
    await api.tx.sudo.sudo(call).signAndSend(alice);
    await api.createBlock();

    const {
      data: { free: balAfter },
    } = await api.query.system.account(randomId);

    expect(balAfter.toBigInt()).toBe(UNIT);
  });
});
