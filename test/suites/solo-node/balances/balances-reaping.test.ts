import { strictEqual } from "node:assert";
import { after, before, describe, it } from "node:test";
import {
  alice,
  createSr25519Account,
  UNIT,
  eve,
  ferdie,
  type ExtendedApiPromise,
  ROUGH_TRANSFER_FEE,
  DevTestContext
} from "../../../util";

// TODO: Figure out why only isn't working here
describe("Balances Pallet: Reaping", async () => {
  const context = new DevTestContext({
    // printLogs: true,
    // keepOpen: true,
  });
  let api: ExtendedApiPromise;

  before(async () => {
    api = await context.initialize();
  });

  after(async () => {
    await context.dispose();
  });

  it("Can transfer full balance to another account with reap", async () => {
    const { address: randomId } = await createSr25519Account();
    await api.tx.balances.transferAll(randomId, false).signAndSend(eve);
    await api.createBlock();

    const {
      data: { free: eveBal }
    } = await api.query.system.account(eve.address);
    strictEqual(eveBal.toBigInt(), 0n);

    const {
      data: { free: randomBal }
    } = await api.query.system.account(randomId);
    strictEqual(randomBal.toBigInt() > 0n, true);
  });

  it("Can transfer full balance to another account without reap", async () => {
    const { address: randomId } = await createSr25519Account();
    await api.tx.balances.transferAll(randomId, true).signAndSend(ferdie);
    await api.createBlock();
    const {
      data: { free: ferdieBal }
    } = await api.query.system.account(ferdie.address);
    strictEqual(ferdieBal.toBigInt() > 0n, true);

    const {
      data: { free: randomBal }
    } = await api.query.system.account(randomId);
    strictEqual(randomBal.toBigInt() > 0n, true);
  });

  it("Bal below ED kills account", async () => {
    const randomAccount = await createSr25519Account();
    const amount = 10n * UNIT;

    await api.tx.balances.transferAllowDeath(randomAccount.address, amount).signAndSend(alice);
    await api.createBlock();

    const {
      data: { free: balAvail }
    } = await api.query.system.account(randomAccount.address);
    console.log(balAvail.toHuman());

    await api.tx.balances
      .transferAllowDeath(alice.address, balAvail.toBigInt() - ROUGH_TRANSFER_FEE - 10000n)
      .signAndSend(randomAccount);
    await api.createBlock();

    const {
      data: { free: randBal }
    } = await api.query.system.account(randomAccount.address);

    strictEqual(randBal.toBigInt(), 0n);
  });
});
