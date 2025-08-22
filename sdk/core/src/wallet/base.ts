/**
 * Base abstraction for wallet integrations.
 *
 * Any concrete wallet (e.g. a browser extension wallet, a hardware wallet or a
 * mobile-SDK based wallet) must extend this class and implement the methods
 * for retrieving the active account address, sending transactions, and
 * signing arbitrary messages.
 */
import type { TransactionRequest } from 'ethers';

export abstract class WalletBase {
  /**
   * Return the public address for the currently selected account.
   *
   * Implementations may need to prompt the user to unlock the wallet or to
   * choose an account if more than one is available.
   */
  public abstract getAddress(): Promise<string>;

  /**
   * Send a transaction through the wallet and return the transaction hash.
   *
   * This is the primary operation for most EIP-1193 compatible wallets which
   * do not support producing detached transaction signatures.
   */
  public abstract sendTransaction(tx: TransactionRequest): Promise<string>;

  /**
   * Sign an arbitrary message and return the signature.
   *
   * This is commonly used for off-chain authentication flows (e.g. signing a
   * nonce) or for verifying ownership of an address.
   *
   * @param msg The message to sign, either as raw bytes (`Uint8Array`) or a
   *            regular UTF-8 string.
   * @returns   A signature string, typically hex-encoded.
   */
  public abstract signMessage(msg: Uint8Array | string): Promise<string>;
}
