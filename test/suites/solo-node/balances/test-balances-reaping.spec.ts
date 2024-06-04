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
} from "../../../util";

describe("Balances Pallet: Reaping", () => {
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
});
