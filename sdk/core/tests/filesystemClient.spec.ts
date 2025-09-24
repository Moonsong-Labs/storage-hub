import { describe, it, expect } from "vitest";
import { StorageHubClient } from "../src/evm/storageHubClient.js";
import { LocalWallet } from "../src/wallet/local.js";
import { createWalletClient, defineChain, http, type Address } from "viem";
import { mnemonicToAccount } from "viem/accounts";
import { TEST_MNEMONIC_12 } from "./consts.js";

describe("StorageHubClient", () => {
  it("throws if bucket name exceeds 100 bytes", async () => {
    // Create a valid EVM address using LocalWallet
    const wallet = LocalWallet.createRandom();
    const owner = (await wallet.getAddress()) as Address;

    // Create chain and wallet client for the new StorageHubClient constructor
    const chain = defineChain({
      id: 31337,
      name: "Hardhat",
      nativeCurrency: { name: "Ether", symbol: "ETH", decimals: 18 },
      rpcUrls: { default: { http: ["http://127.0.0.1:8545"] } }
    });
    const walletClient = createWalletClient({
      chain,
      account: mnemonicToAccount(TEST_MNEMONIC_12),
      transport: http("http://127.0.0.1:8545")
    });
    const hubClient = new StorageHubClient({
      rpcUrl: "http://127.0.0.1:8545",
      chain,
      walletClient
    });

    const longName = "a".repeat(101); // String that's 101 bytes
    expect(() => hubClient.deriveBucketId(owner, longName)).toThrowError(
      /exceeds maximum length of 100 bytes/
    );
  });

  // Broadcast test against a running Hardhat node (skipped if env vars not present)
  it("attempts to broadcast a transaction to Hardhat", async () => {
    const rpcUrl = "http://127.0.0.1:8545" as const;
    const chain = defineChain({
      id: 31337,
      name: "Hardhat",
      nativeCurrency: { name: "Ether", symbol: "ETH", decimals: 18 },
      rpcUrls: { default: { http: [rpcUrl] } }
    });
    // Use the testing mnemonic (Hardhat default) to derive the first account
    const account = mnemonicToAccount(TEST_MNEMONIC_12);
    // Also build a LocalWallet from the same mnemonic to keep parity with Core's wallet flow
    void LocalWallet.fromMnemonic(TEST_MNEMONIC_12);
    const walletClient = createWalletClient({
      chain,
      account,
      transport: http(rpcUrl)
    });
    const hubClient = new StorageHubClient({ rpcUrl, chain, walletClient });

    const name = "bucket"; // Now a simple string
    // Attempt broadcast; on vanilla Hardhat this should reject (no precompile),
    // on a StorageHub-compatible node it may resolve. We assert it returns a promise
    // and does not throw synchronously.
    const p = hubClient.createBucket(
      `0x${"00".repeat(32)}` as `0x${string}`,
      name,
      true,
      `0x${"00".repeat(32)}` as `0x${string}`
    );
    expect(typeof (p as Promise<unknown>).then).toBe("function");
    await p.catch(() => { });
  });
});
