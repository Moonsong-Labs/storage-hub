import { expect } from "expect";
import { after, before, describe, it } from "node:test";
import {
  alice,
  type StartedTestContainer,
  createSr25519Account,
  devnodeSetup,
  UNIT,
  eve,
  ferdie,
  type ExtendedApiPromise,
  ROUGH_TRANSFER_FEE,
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

  it("Can transfer full balance to another account with reap", async () => {
    const { address: randomId } = await createSr25519Account();
    await api.tx.balances.transferAll(randomId, false).signAndSend(eve);
    await api.createBlock();

    const {
      data: { free: eveBal },
    } = await api.query.system.account(eve.address);
    expect(eveBal.toBigInt()).toBe(0n);

    const {
      data: { free: randomBal },
    } = await api.query.system.account(randomId);
    expect(randomBal.toBigInt()).toBeGreaterThan(0n);
  });

  it("Can transfer full balance to another account without reap", async () => {
    const { address: randomId } = await createSr25519Account();
    await api.tx.balances.transferAll(randomId, true).signAndSend(ferdie);
    await api.createBlock();
    const {
      data: { free: ferdieBal },
    } = await api.query.system.account(ferdie.address);
    expect(ferdieBal.toBigInt()).toBeGreaterThan(0n);

    const {
      data: { free: randomBal },
    } = await api.query.system.account(randomId);
    expect(randomBal.toBigInt()).toBeGreaterThan(0n);
  });

  it("Bal below ED kills account", async () => {
    const randomAccount = await createSr25519Account();
    const amount = 10n * UNIT;

    await api.tx.balances.transferAllowDeath(randomAccount.address, amount).signAndSend(alice);
    await api.createBlock();

    const {
      data: { free: balAvail },
    } = await api.query.system.account(randomAccount.address);
    console.log(balAvail.toHuman());

    await api.tx.balances
      .transferAllowDeath(alice.address, balAvail.toBigInt() - ROUGH_TRANSFER_FEE - 10000n)
      .signAndSend(randomAccount);
    await api.createBlock();

    const {
      data: { free: randBal },
    } = await api.query.system.account(randomAccount.address);

    expect(randBal.toBigInt()).toBe(0n);
  });

  it("SetBalance fails when called without sudo", { only: true }, async () => {
    const {
      data: { free: balBefore },
    } = await api.query.system.account(bob.address);

    await api.tx.balances.forceSetBalance(bob.address, 1337n).signAndSend(alice);
    await api.createBlock();

    const {
      data: { free: balAfter },
    } = await api.query.system.account(bob.address);

    expect(balBefore.sub(balAfter).toNumber()).toBe(0);
  });

  // set balance sudo pass
});
