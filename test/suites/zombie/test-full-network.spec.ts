import { test, describe, after } from "node:test";
import { expect } from "expect";
import {
  alice,
  bob,
  bsp,
  charlie,
  collator,
  createSr25519Account,
  dave,
  eve,
  ferdie,
  getZombieClients,
  sendTransaction,
} from "../../util";

describe("Full Network Suite", { concurrency: 2 }, async () => {
  const { relayApi, storageApi } = await getZombieClients({
    relayWs: "ws://127.0.0.1:31000",
    shWs: "ws://127.0.0.1:32000",
  });

  after(() => {
    relayApi.disconnect();
    storageApi.disconnect();
  });

  describe("Relay Tests", async () => {
    test("Check RelayChain RT Version", async () => {
      const { specName, specVersion } = relayApi.consts.system.version;
      expect(specName.toString()).toBe("rococo");
      expect(specVersion.toNumber()).toBeGreaterThanOrEqual(1008000);
    });

    test("Check sr25519 keyring is correct", async () => {
      expect(alice.address).toBe("5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY");
      expect(bob.address).toBe("5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty");
    });

    test("Check test accounts have balance", async () => {
      const promises = [alice, bob, charlie, dave, eve, ferdie].map(async (signer) => {
        const {
          data: { free },
        } = await relayApi.query.system.account(signer.address);
        console.log(`✅ Account ${signer.address} ${signer.meta.name} has ${free} balance`);

        return { free, address: signer.address, name: signer.meta.name };
      });

      const failures = (await Promise.all(promises)).filter(({ free }) => free.toBigInt() < 1n);

      for (const { address, name } of failures) {
        console.error(`❌ Account ${address} ${name}  has no balance!`);
      }

      expect(failures).toHaveLength(0);
    });

    test("Send bal transfer on relaychain", { timeout: 60_000 }, async () => {
      const amount = 1_000_000_000n;
      const { address: randomId } = await createSr25519Account();
      console.log(`Sending balance to ${randomId}`);

      await sendTransaction(relayApi.tx.balances.transferAllowDeath(randomId, amount));

      const {
        data: { free: balAfter },
      } = await relayApi.query.system.account(randomId);

      expect(balAfter.toBigInt()).toBe(amount);
      console.log(`✅ Account ${randomId} has ${balAfter} balance`);
    });
  });

  describe("StorageHub", async () => {
    test("Check StorageHub RT Version", async () => {
      const { specName, specVersion } = storageApi.consts.system.version;
      expect(specName.toString()).toBe("storage-hub-runtime");
      expect(specVersion.toNumber()).toBeGreaterThanOrEqual(1);
    });

    test("Check test accounts have balance", async () => {
      const promises = [alice, bob, charlie, dave, eve, ferdie, bsp, collator].map(
        async (signer) => {
          const {
            data: { free },
          } = await storageApi.query.system.account(signer.address);
          console.log(`✅ Account ${signer.address} ${signer.meta.name} has ${free} balance`);

          return { free, address: signer.address, name: signer.meta.name };
        }
      );

      const failures = (await Promise.all(promises)).filter(({ free }) => free.toBigInt() < 1n);

      for (const { address, name } of failures) {
        console.error(`❌ Account ${address} ${name}  has no balance!`);
      }

      expect(failures).toHaveLength(0);
    });

    test("Send bal transfer on storagehub", { timeout: 120_000 }, async () => {
      const amount = 1_000_000_000n;
      const { address: randomId } = await createSr25519Account();
      console.log(`Sending balance to ${randomId}`);

      await sendTransaction(storageApi.tx.balances.transferAllowDeath(randomId, amount));

      const {
        data: { free: balAfter },
      } = await storageApi.query.system.account(randomId);

      expect(balAfter.toBigInt()).toBe(amount);
      console.log(`✅ Account ${randomId} has ${balAfter} balance`);
    });
  });
});
