import { Wallet as EthersWallet, Transaction, hexlify } from 'ethers';
import { WalletBase } from './base.js';

/**
 * A local, in-memory wallet implementation.
 *
 * @warning This class is intended for development and testing purposes only.
 * It manages private keys in memory and is not suitable for production use
 * where secure key management is required.
 */
export class LocalWallet extends WalletBase {
  private constructor(private readonly wallet: EthersWallet) {
    super();
  }

  /**
   * Create an instance from an existing private key.
   *
   * @param privateKey - A 0x-prefixed hex string containing the private key.
   * @returns A new `LocalWallet` that can sign on behalf of the keyʼs address.
   */
  public static fromPrivateKey(privateKey: string): LocalWallet {
    return new LocalWallet(new EthersWallet(privateKey));
  }

  /**
   * Create an instance from a BIP-39 mnemonic phrase.
   *
   * @param mnemonic - The 12/24-word mnemonic phrase.
   * @returns A new `LocalWallet` bound to the first account derived from the
   *          mnemonic.
   */
  public static fromMnemonic(mnemonic: string): LocalWallet {
    const wallet = EthersWallet.fromPhrase(mnemonic);
    return new LocalWallet(new EthersWallet(wallet.privateKey));
  }

  /**
   * Generate a brand-new keypair on the fly.
   *
   * @returns A freshly generated `LocalWallet` with a random private key.
   */
  public static createRandom(): LocalWallet {
    const wallet = EthersWallet.createRandom();
    return new LocalWallet(new EthersWallet(wallet.privateKey));
  }

  /** @inheritdoc */
  public getAddress(): Promise<string> {
    return Promise.resolve(this.wallet.address);
  }

  /** @inheritdoc */
  public signTxn(tx: Uint8Array): Promise<string> {
    const hexTx = hexlify(tx);
    return this.wallet.signTransaction(Transaction.from(hexTx));
  }

  /** @inheritdoc */
  public signMessage(msg: Uint8Array | string): Promise<string> {
    return this.wallet.signMessage(msg);
  }
}
