import { test, describe, expect, beforeAll } from "bun:test";
import { MultiAddress } from "@polkadot-api/descriptors";
import { accounts, getSr25519Account, getZombieClients, waitForChain } from "../../util";

describe("Simple zombieTest", async () => {
  const { relayApi, relayClient, relayRT, shClient, storageApi, storageRT } =
    await getZombieClients();

  beforeAll(async () => {
    await Promise.all([waitForChain(shClient), waitForChain(relayClient)]);
  });

  describe("Relay", async () => {
    test("Check RelayChain RT Version", async () => {
      const { spec_name, spec_version } = relayApi.constants.System.Version(relayRT);
      expect(spec_name).toBe("rococo");
      expect(spec_version).toBeGreaterThanOrEqual(1008000);
    });

    test("Check sr25519 keyring is correct", async () => {
      expect(accounts.alice.sr25519.id).toBe("5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY");
      expect(accounts.bob.sr25519.id).toBe("5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty");
    });

    test("Check test accounts have balance", async () => {
      const promises = Object.entries(accounts).map(async ([account, signers]) => {
        const {
          data: { free },
        } = await relayApi.query.System.Account.getValue(signers.sr25519.id);
        console.log(`✅ Account ${account} has ${free} balance`);
        return { account, free: free.valueOf() };
      });

      const failures = (await Promise.all(promises)).filter(({ free }) => free < 1n);
      for (const { account } of failures) {
        console.error(`❌ Account ${account} has no balance!`);
      }

      expect(failures).toHaveLength(0);
    });

    test(
      "Send bal transfer on relaychain",
      async () => {
        const amount = 1_000_000_000n;
        const { id: randomId } = await getSr25519Account();
        console.log(`Sending balance to ${randomId}`);

        await relayApi.tx.Balances.transfer_allow_death({
          dest: MultiAddress.Id(randomId),
          value: amount,
        }).signAndSubmit(accounts.alice.sr25519.signer);

        const {
          data: { free: balAfter },
        } = await relayApi.query.System.Account.getValue(randomId);

        expect(balAfter).toBe(amount);
        console.log(`✅ Account ${randomId} has ${balAfter} balance`);
      },
      { timeout: 60_000 }
    );
  });

  describe("StorageHub", async () => {
    test("Check StorageHub RT Version", async () => {
      const { spec_name, spec_version } = storageApi.constants.System.Version(storageRT);
      expect(spec_name).toBe("storage-hub-runtime");
      expect(spec_version).toBeGreaterThanOrEqual(1);
    });

    test("Check test accounts have balance", async () => {
      const promises = Object.entries(accounts).map(async ([account, signers]) => {
        const {
          data: { free },
        } = await storageApi.query.System.Account.getValue(signers.sr25519.id);
        console.log(`✅ Account ${account} has ${free} balance`);
        return { account, free: free.valueOf() };
      });

      const failures = (await Promise.all(promises)).filter(({ free }) => free < 1n);
      for (const { account } of failures) {
        console.error(`❌ Account ${account} has no balance!`);
      }

      expect(failures).toHaveLength(0);
    });

    test(
      "Send bal transfer on storagehub",
      async () => {
        const amount = 1_000_000_000n;
        const { id: randomId } = await getSr25519Account();
        console.log(`Sending balance to ${randomId}`);

        await storageApi.tx.Balances.transfer_allow_death({
          dest: MultiAddress.Id(randomId),
          value: amount,
        }).signAndSubmit(accounts.alice.sr25519.signer);

        const {
          data: { free: balAfter },
        } = await storageApi.query.System.Account.getValue(randomId);

        expect(balAfter).toBe(amount);
        console.log(`✅ Account ${randomId} has ${balAfter} balance`);
      },
      { timeout: 120_000 }
    );
  });
});
