import { describe, it, expect } from 'vitest';
import { randomBytes } from 'node:crypto';
import { hexToBytes, equalBytes, bytesToHex } from '@noble/ciphers/utils.js';

import { AAD, BaseNonce, DEK, IKM, Nonce, Salt } from '../src/encryption/types.js';
import { privateKeyToAccount } from 'viem/accounts';
import { createWalletClient, defineChain, http } from 'viem';
import { TEST_PRIVATE_KEY_12 } from './consts.js';
import { ensure0xPrefix } from '../src/utils.js';

describe('encryption types sanity check', () => {
  function u64be(n: number): Uint8Array {
    const b = new Uint8Array(8);
    new DataView(b.buffer).setBigUint64(0, BigInt(n));
    return b;
  }

  it('DEK/Nonce/AAD constructors: ok + error + unwrap', () => {
    // ok paths
    const dekRes = DEK.fromBytes(randomBytes(32));
    expect(dekRes.ok).toBe(true);
    const dek = dekRes.unwrap();
    expect(dek).toBeInstanceOf(Uint8Array);
    expect(dek.length).toBe(32);

    const nonceRes = Nonce.fromBytes(randomBytes(12));
    expect(nonceRes.ok).toBe(true);
    const nonce = nonceRes.unwrap();
    expect(nonce).toBeInstanceOf(Uint8Array);
    expect(nonce.length).toBe(12);

    const chunkIndex = 0;
    const aadRes = AAD.fromBytes(u64be(chunkIndex));
    expect(aadRes.ok).toBe(true);
    const aad = aadRes.unwrap();
    expect(aad).toBeInstanceOf(Uint8Array);
    expect(aad.length).toBe(8);

    // error paths
    const dekBad = DEK.fromBytes(randomBytes(31));
    expect(dekBad.ok).toBe(false);
    expect(dekBad.error).toBeInstanceOf(Error);
    expect(() => dekBad.unwrap()).toThrow();

    const nonceBad = Nonce.fromBytes(randomBytes(11));
    expect(nonceBad.ok).toBe(false);
    expect(nonceBad.error).toBeInstanceOf(Error);
    expect(() => nonceBad.unwrap()).toThrow();
  });
});

describe('DEK derivation sanity check', () => {
  it('derives a 32-byte DEK from password (IKM from password)', () => {
    const password = 'correct horse battery staple';
    const ikmRes = IKM.fromPassword(password);
    expect(ikmRes.ok).toBe(true);
    const ikm = ikmRes.unwrap();

    const salt = Salt.fromBytes(hexToBytes('11'.repeat(32))).unwrap();
    const dekRes1 = DEK.derive(ikm, salt);
    expect(dekRes1.ok).toBe(true);
    const dek1 = dekRes1.unwrap();
    expect(dek1.length).toBe(32);

    const dekRes2 = DEK.derive(ikm, salt);
    expect(dekRes2.ok).toBe(true);
    const dek2 = dekRes2.unwrap();
    expect(equalBytes(dek1, dek2)).toBe(true);

    // Error path: password too short
    const short = IKM.fromPassword('short');
    expect(short.ok).toBe(false);
    expect(() => short.unwrap()).toThrow();
  });

  it('derives a 32-byte DEK from signature (IKM from signature)', () => {
    const signature = (`0x${'a1'.repeat(65)}`) as `0x${string}`; // emulate signature hex
    const ikmRes = IKM.fromSignature(signature);
    expect(ikmRes.ok).toBe(true);
    const ikm = ikmRes.unwrap();

    const salt = Salt.fromBytes(hexToBytes('22'.repeat(32))).unwrap();
    const dekRes1 = DEK.derive(ikm, salt);
    expect(dekRes1.ok).toBe(true);
    const dek1 = dekRes1.unwrap();
    expect(dek1.length).toBe(32);

    // Error path: signature must be a valid hex string
    const badSig = IKM.fromSignature('not-bytes' as any);
    expect(badSig.ok).toBe(false);
    expect(() => badSig.unwrap()).toThrow();
  });
});

describe('BaseNonce + chunked file nonces', () => {
  it('derives identical per-chunk nonces for same BaseNonce', () => {
    // Deterministic base nonce bytes (12 bytes)
    const baseBytes = hexToBytes('ab'.repeat(12));

    const base1 = BaseNonce.fromBytes(baseBytes).unwrap();
    const base2 = BaseNonce.fromBytes(baseBytes).unwrap();

    const derived1: Uint8Array[] = [];
    const derived2: Uint8Array[] = [];

    // Simulate chunked file: derive a nonce per chunk index
    for (let chunkIndex = 0; chunkIndex < 10; chunkIndex++) {
      const n1 = base1.getNonce(chunkIndex);
      expect(n1.ok).toBe(true);
      derived1.push(n1.unwrap());

      const n2 = base2.getNonce(chunkIndex);
      expect(n2.ok).toBe(true);
      derived2.push(n2.unwrap());
    }

    expect(derived1.length).toBe(derived2.length);
    for (let i = 0; i < derived1.length; i++) {
      const a = derived1[i];
      const b = derived2[i];
      if (!a || !b) throw new Error('missing derived nonce');
      expect(equalBytes(a, b)).toBe(true);
    }

    // sanity: different chunk indices should generally produce different nonces
    const d0 = derived1[0];
    const d1 = derived1[1];
    if (!d0 || !d1) throw new Error('missing derived nonce');
    expect(equalBytes(d0, d1)).toBe(false);
  });
});

describe('Generate DEK and base Nonce', () => {
  it('Generate DEK & Nonce from Signature', async () => {
    const rpcUrl = "http://127.0.0.1:8545" as const;
    const chain = defineChain({
      id: 31337,
      name: "Hardhat",
      nativeCurrency: { name: "Ether", symbol: "ETH", decimals: 18 },
      rpcUrls: { default: { http: [rpcUrl] } }
    });

    const account = privateKeyToAccount(TEST_PRIVATE_KEY_12);
    const walletClient = createWalletClient({ chain, account, transport: http(rpcUrl) });

    const fingerprint = "Fingerprint from unencrypted file";
    const message = `Sign this message to generate the encryption key for filekey: ${fingerprint}`;
    const signature = await walletClient.signMessage({
      account,
      message,
    });

    // Make sure to use deterministic ECDSA (e.g. RFC 6979)
    const expectedSignature = "0xf57db1fe2cbf3fe9ba789975126bcdf679394a70363758e429271b0323fc85f265db0fb27a66b8457c5b1138747a5ebefcd5756e8bcabe110c24cffab6ff118b1b";
    expect(signature).toBe(expectedSignature);

    // Normalize input
    const ikm = IKM.fromSignature(signature).unwrap();
    const salt = Salt.fromBytes(hexToBytes('33'.repeat(32))).unwrap();

    // Compute DEK and Base nonce (salted HKDF)
    const dek = DEK.derive(ikm, salt).unwrap();
    expect(dek.length).toBe(32);

    const baseNonce = BaseNonce.derive(ikm, salt).unwrap();
    expect(baseNonce.bytes.length).toBe(12);

    // Basic nonce sanity: chunk 0 nonce equals base bytes; later nonces differ
    const n0 = baseNonce.getNonce(0).unwrap();
    expect(ensure0xPrefix(bytesToHex(n0))).toBe(ensure0xPrefix(bytesToHex(baseNonce.bytes)));
    const n1 = baseNonce.getNonce(1).unwrap();
    expect(equalBytes(n0, n1)).toBe(false);

  }, 30_000);

  it('Generate DEK & Nonce from Password', async () => {
    // Normalize input
    const ikm = IKM.fromPassword("User sets some really strong password").unwrap()
    const salt = Salt.fromBytes(hexToBytes('44'.repeat(32))).unwrap();

    // Compute DEK and Base nonce (salted HKDF)
    const dek = DEK.derive(ikm, salt).unwrap();
    expect(dek.length).toBe(32);

    const baseNonce = BaseNonce.derive(ikm, salt).unwrap();
    expect(baseNonce.bytes.length).toBe(12);

    const n0 = baseNonce.getNonce(0).unwrap();
    expect(ensure0xPrefix(bytesToHex(n0))).toBe(ensure0xPrefix(bytesToHex(baseNonce.bytes)));
    const n1 = baseNonce.getNonce(1).unwrap();
    expect(equalBytes(n0, n1)).toBe(false);

  }, 30_000);
});