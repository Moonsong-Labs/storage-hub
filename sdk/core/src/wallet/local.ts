import {
  Wallet as EthersWallet,
  Transaction,
  hexlify,
  type Provider,
  type TransactionRequest,
} from 'ethers';
import { WalletBase } from './base.js';
import { WalletError } from './errors.js';

/**
 * A local, in-memory wallet implementation.
 *
 * @warning This class is intended for development and testing purposes only.
 * It manages private keys in memory and is not suitable for production use
 * where secure key management is required.
 */
export class LocalWallet extends WalletBase {
  private constructor(
    private readonly wallet: EthersWallet,
    private readonly provider?: Provider,
  ) {
    super();
  }

  /**
   * Create an instance from an existing private key.
   *
   * @param privateKey - A 0x-prefixed hex string containing the private key.
   * @returns A new `LocalWallet` that can sign on behalf of the keyʼs address.
   */
  public static fromPrivateKey(privateKey: string, provider?: Provider): LocalWallet {
    // Validate early to provide a stable error type regardless of ethers internals
    const isHex = /^0x[0-9a-fA-F]{64}$/.test(privateKey);
    if (!isHex) throw new WalletError('InvalidPrivateKey');

    try {
      return new LocalWallet(new EthersWallet(privateKey, provider), provider);
    } catch {
      throw new WalletError('InvalidPrivateKey');
    }
  }

  /**
   * Create an instance from a BIP-39 mnemonic phrase.
   *
   * @param mnemonic - The 12/24-word mnemonic phrase.
   * @returns A new `LocalWallet` bound to the first account derived from the
   *          mnemonic.
   */
  public static fromMnemonic(mnemonic: string, provider?: Provider): LocalWallet {
    try {
      const wallet = EthersWallet.fromPhrase(mnemonic);
      return new LocalWallet(new EthersWallet(wallet.privateKey, provider), provider);
    } catch (err) {
      throw new WalletError('InvalidMnemonic');
    }
  }

  /**
   * Generate a brand-new keypair on the fly.
   *
   * @returns A freshly generated `LocalWallet` with a random private key.
   */
  public static createRandom(provider?: Provider): LocalWallet {
    const wallet = EthersWallet.createRandom();
    return new LocalWallet(new EthersWallet(wallet.privateKey, provider), provider);
  }

  /** @inheritdoc */
  public getAddress(): Promise<string> {
    return Promise.resolve(this.wallet.address);
  }

  /** @inheritdoc */
  public signTransaction(tx: Uint8Array): Promise<string> {
    const hexTx = hexlify(tx);
    return this.wallet.signTransaction(Transaction.from(hexTx));
  }

  /** @inheritdoc */
  public async sendTransaction(tx: TransactionRequest): Promise<string> {
    if (!this.provider) {
      throw new Error('No provider configured for LocalWallet; cannot send transaction');
    }
    const connected = this.wallet.connect(this.provider);
    const response = await connected.sendTransaction(tx);
    return response.hash;
  }

  /** @inheritdoc */
  public signMessage(msg: Uint8Array | string): Promise<string> {
    return this.wallet.signMessage(msg);
  }
}
