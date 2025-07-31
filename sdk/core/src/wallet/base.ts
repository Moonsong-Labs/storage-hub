/**
 * Base abstraction for wallet integrations.
 *
 * Any concrete wallet (e.g. a browser extension wallet, a hardware wallet or a
 * mobile-SDK based wallet) must extend this class and implement the methods
 * for retrieving the active account address and for signing transactions or
 * arbitrary messages.
 */
export abstract class WalletBase {
  /**
   * Return the public address for the currently selected account.
   *
   * Implementations may need to prompt the user to unlock the wallet or to
   * choose an account if more than one is available.
   */
  public abstract getAddress(): Promise<string>;

  /**
   * Sign a blockchain transaction and return the resulting signature.
   *
   * @param tx  Raw transaction payload as a `Uint8Array`. The exact encoding
   *            depends on the target network and should match what the wallet
   *            expects (for example an RLP-encoded Ethereum transaction).
   * @returns   A signature string, typically hex-encoded.
   */
  public abstract signTxn(tx: Uint8Array): Promise<string>;

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
