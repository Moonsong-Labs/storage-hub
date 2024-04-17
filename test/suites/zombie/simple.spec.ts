import { test, describe, expect } from "bun:test";
import { createClient } from "polkadot-api";
import { WebSocketProvider } from "polkadot-api/ws-provider/node";
import { relaychain, storagehub, MultiAddress } from "@polkadot-api/descriptors";
import { accounts } from "../../util";

type TypesBundle = typeof relaychain | typeof storagehub;

const getClient = async (endpoint: string, typesBundle: TypesBundle) => {
  const relayClient = createClient(WebSocketProvider(endpoint));
  const api = relayClient.getTypedApi(typesBundle);
  const rt = await api.runtime.latest();
  return { api, rt };
};

describe("Simple zombieTest", async () => {
  const { api: relayApi, rt: relayRT } = await getClient("ws://127.0.0.1:39459", relaychain);
  const { api: storageApi, rt: storageRT } = await getClient("ws://127.0.0.1:42933", storagehub);

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
        await relayApi.tx.Balances.transfer_allow_death({
          dest: MultiAddress.Id(accounts.bob.sr25519.id),
          value: 1337n,
        }).signAndSubmit(accounts.alice.sr25519.signer);
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
        await storageApi.tx.Balances.transfer_allow_death({
          dest: MultiAddress.Id(accounts.bob.sr25519.id),
          value: 1337n,
        }).signAndSubmit(accounts.alice.sr25519.signer);
      },
      { timeout: 60_000 }
    );
  });
});
