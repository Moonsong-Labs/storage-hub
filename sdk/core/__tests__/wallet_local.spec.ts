import { describe, it, expect } from 'vitest';
import { LocalWallet } from '../src/wallet/local.js';
import { verifyMessage } from 'ethers';

describe('LocalWallet', () => {
    // NOTE: You can compute the private key, public key and address using https://iancoleman.io/bip39/
    // ---- Test data ----
    // 12-word mnemonic and its first account (m/44'/60'/0'/0/0)
    const TEST_MNEMONIC_12 = 'test test test test test test test test test test test junk';
    const TEST_PRIVATE_KEY = '0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80';
    const TEST_ADDRESS_12 = '0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266';
    // 24-word mnemonic and its first account (m/44'/60'/0'/0/0)
    const TEST_MNEMONIC_24 = 'abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art';
    const TEST_ADDRESS_24 = '0xF278cF59F82eDcf871d630F28EcC8056f25C1cdb';


    describe('Creation', () => {
        it('should create a wallet from a private key', async () => {
            const wallet = LocalWallet.fromPrivateKey(TEST_PRIVATE_KEY);
            expect(wallet).toBeInstanceOf(LocalWallet);
            expect(await wallet.getAddress()).toBe(TEST_ADDRESS_12);
        });

        it('should create a wallet from a 12-word mnemonic', async () => {
            const wallet = LocalWallet.fromMnemonic(TEST_MNEMONIC_12);
            expect(wallet).toBeInstanceOf(LocalWallet);
            expect(await wallet.getAddress()).toBe(TEST_ADDRESS_12);
        });

        it('should create a wallet from a 24-word mnemonic', async () => {
            const wallet24 = LocalWallet.fromMnemonic(TEST_MNEMONIC_24);
            expect(wallet24).toBeInstanceOf(LocalWallet);
            expect(await wallet24.getAddress()).toBe(TEST_ADDRESS_24);
        });

        it('should create a random wallet', async () => {
            const wallet = LocalWallet.createRandom();
            const address = await wallet.getAddress();
            expect(address).toBeTypeOf('string');
            expect(address).toMatch(/^0x[a-fA-F0-9]{40}$/);
        });
    });

    describe('Failure Cases', () => {
        it('should throw an error for a private key with invalid length (short)', () => {
            const shortKey = '0x1234';
            expect(() => LocalWallet.fromPrivateKey(shortKey)).toThrow();
        });

        it('should throw an error for a private key with invalid length (long)', () => {
            const longKey = TEST_PRIVATE_KEY + 'ff';
            expect(() => LocalWallet.fromPrivateKey(longKey)).toThrow();
        });

        it('should throw an error for an invalid mnemonic', () => {
            const invalidMnemonic = 'not a valid mnemonic phrase';
            expect(() => LocalWallet.fromMnemonic(invalidMnemonic)).toThrow();
        });
    });

    describe('Signing', () => {
        it('should sign a message and verify it', async () => {
            const wallet = LocalWallet.fromPrivateKey(TEST_PRIVATE_KEY);
            const message = 'Hello StorageHub';
            const signature = await wallet.signMessage(message);

            // ethers.verifyMessage returns the address that signed the message
            const recovered = verifyMessage(message, signature);
            expect(recovered.toLowerCase()).toBe(TEST_ADDRESS_12.toLowerCase());
        });

        it('should sign a transaction and verify the from address', async () => {

            const wallet = LocalWallet.fromPrivateKey(TEST_PRIVATE_KEY);

            // use ethers v6 public API (Transaction.unsignedSerialized)
            // eslint-disable-next-line @typescript-eslint/ban-ts-comment
            // @ts-ignore - dynamic import to avoid ESM typing issues in vitest
            const { Transaction, getBytes, parseUnits } = await import('ethers');

            const unsignedTx = {
                to: TEST_ADDRESS_12,
                nonce: 0,
                gasPrice: parseUnits('1', 'gwei'),
                gasLimit: 21_000,
                value: parseUnits('0.001', 'ether'),
                chainId: 1,
            };

            const txObj = Transaction.from(unsignedTx);
            const rawUnsigned = getBytes(txObj.unsignedSerialized);

            const signedTxHex = await wallet.signTxn(rawUnsigned);

            const parsed = Transaction.from(signedTxHex);
            expect(parsed.from?.toLowerCase()).toBe(TEST_ADDRESS_12.toLowerCase());
        });
    });
}); 