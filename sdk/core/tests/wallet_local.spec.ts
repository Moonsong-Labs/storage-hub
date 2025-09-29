import { describe, it, expect } from "vitest";
import { LocalWallet } from "../src/wallet/local.js";
import { WalletError } from "../src/wallet/errors.js";
import {
  TEST_MNEMONIC_12,
  TEST_PRIVATE_KEY_12,
  TEST_ADDRESS_12,
  TEST_MNEMONIC_24,
  TEST_ADDRESS_24
} from "./consts.js";
import { verifyMessage, Transaction, getBytes, parseUnits } from "ethers";

describe("LocalWallet", () => {
  describe("Creation", () => {
    it("should create a wallet from a private key", async () => {
      const wallet = LocalWallet.fromPrivateKey(TEST_PRIVATE_KEY_12);
      expect(wallet).toBeInstanceOf(LocalWallet);
      expect(await wallet.getAddress()).toBe(TEST_ADDRESS_12);
    });

    it("should create a wallet from a 12-word mnemonic", async () => {
      const wallet = LocalWallet.fromMnemonic(TEST_MNEMONIC_12);
      expect(wallet).toBeInstanceOf(LocalWallet);
      expect(await wallet.getAddress()).toBe(TEST_ADDRESS_12);
    });

    it("should create a wallet from a 24-word mnemonic", async () => {
      const wallet24 = LocalWallet.fromMnemonic(TEST_MNEMONIC_24);
      expect(wallet24).toBeInstanceOf(LocalWallet);
      expect(await wallet24.getAddress()).toBe(TEST_ADDRESS_24);
    });

    it("should create a random wallet", async () => {
      const wallet = LocalWallet.createRandom();
      const address = await wallet.getAddress();
      expect(address).toBeTypeOf("string");
      expect(address).toMatch(/^0x[a-fA-F0-9]{40}$/);
    });
  });

  describe("Failure Cases", () => {
    it("should throw WalletError InvalidPrivateKey for a private key with invalid length (short)", () => {
      expect.assertions(2);
      const shortKey = "0x1234";
      try {
        LocalWallet.fromPrivateKey(shortKey);
      } catch (e) {
        expect(e).toBeInstanceOf(WalletError);
        expect((e as WalletError).code).toBe("InvalidPrivateKey");
      }
    });

    it("should throw WalletError InvalidPrivateKey for a private key with invalid length (long)", () => {
      expect.assertions(2);
      const longKey = `${TEST_PRIVATE_KEY_12}ff`;
      try {
        LocalWallet.fromPrivateKey(longKey);
      } catch (e) {
        expect(e).toBeInstanceOf(WalletError);
        expect((e as WalletError).code).toBe("InvalidPrivateKey");
      }
    });

    it("should throw WalletError InvalidMnemonic for an invalid mnemonic", () => {
      expect.assertions(2);
      const invalidMnemonic = "not a valid mnemonic phrase";
      try {
        LocalWallet.fromMnemonic(invalidMnemonic);
      } catch (e) {
        expect(e).toBeInstanceOf(WalletError);
        expect((e as WalletError).code).toBe("InvalidMnemonic");
      }
    });
  });

  describe("Signing", () => {
    it("should sign a message and verify it", async () => {
      const wallet = LocalWallet.fromPrivateKey(TEST_PRIVATE_KEY_12);
      const message = "Hello StorageHub";
      const signature = await wallet.signMessage(message);

      // ethers.verifyMessage returns the address that signed the message
      const recovered = verifyMessage(message, signature);
      expect(recovered.toLowerCase()).toBe(TEST_ADDRESS_12.toLowerCase());
    });

    it("should sign a transaction and verify the from address", async () => {
      const wallet = LocalWallet.fromPrivateKey(TEST_PRIVATE_KEY_12);

      const unsignedTx = {
        to: TEST_ADDRESS_24,
        nonce: 0,
        gasPrice: parseUnits("1", "gwei"),
        gasLimit: 21_000,
        value: parseUnits("0.001", "ether"),
        chainId: 1
      };

      const txObj = Transaction.from(unsignedTx);
      const rawUnsigned = getBytes(txObj.unsignedSerialized);

      const signedTxHex = await wallet.signTransaction(rawUnsigned);

      const parsed = Transaction.from(signedTxHex);
      expect(parsed.from?.toLowerCase()).toBe(TEST_ADDRESS_12.toLowerCase());
    });
  });
});
