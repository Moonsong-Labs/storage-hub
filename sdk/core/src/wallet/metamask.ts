import { WalletBase } from './base.js';
import { BrowserProvider, type TransactionRequest, type Eip1193Provider } from 'ethers';

declare global {
  // Expose the injected provider placed on the window object by MetaMask.
  // We type it as unknown because the exact shape is library-specific and we
  // interact with it exclusively through ethers' BrowserProvider wrapper.
  interface Window {
    ethereum?: unknown;
  }
}

/**
 * Wallet integration for the MetaMask browser extension.
 *
 * It fulfils the minimal `WalletBase` contract (fetching the current address
 * and signing arbitrary messages). MetaMask **cannot** sign a raw transaction
 * without also broadcasting it, therefore {@link signTxn} intentionally throws
 * and consumers should use {@link sendTransaction} instead.
 */
export class MetamaskWallet extends WalletBase {
  private constructor(private readonly provider: BrowserProvider) {
    super();
  }

  /**
   * Request connection to MetaMask and create a new `MetamaskWallet`.
   *
   * Internally this triggers the extension UI via `eth_requestAccounts` which
   * asks the user to authorise account access.
   *
   * @throws If no injected provider is found (MetaMask not installed).
   */
  public static async connect(): Promise<MetamaskWallet> {
    if (typeof window.ethereum === 'undefined') {
      throw new Error('Metamask provider not found. Please install Metamask.');
    }

    const provider = new BrowserProvider(window.ethereum as Eip1193Provider);
    // Prompt the user to connect (select account)
    await provider.send('eth_requestAccounts', []);
    return new MetamaskWallet(provider);
  }

  /** @inheritdoc */
  public async getAddress(): Promise<string> {
    const signer = await this.provider.getSigner();
    return signer.getAddress();
  }

  /** @inheritdoc */
  public async sendTransaction(tx: TransactionRequest): Promise<string> {
    const signer = await this.provider.getSigner();
    // Keep only meaningful fields; let MetaMask fill in nonce/gas/chainId
    const txRequest: Partial<TransactionRequest> = {};
    if (tx.to) txRequest.to = tx.to;
    if (tx.data && tx.data !== '0x') txRequest.data = tx.data;
    if (tx.value && tx.value !== 0n) txRequest.value = tx.value;
    if (tx.gasLimit && tx.gasLimit !== 0n) txRequest.gasLimit = tx.gasLimit;
    const response = await signer.sendTransaction(txRequest);
    return response.hash;
  }

  /** @inheritdoc */
  public async signMessage(msg: Uint8Array | string): Promise<string> {
    const signer = await this.provider.getSigner();
    return signer.signMessage(msg);
  }
}
