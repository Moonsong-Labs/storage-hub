import { WalletBase } from './base.js';
import { BrowserProvider, type Eip1193Provider, type TransactionRequest } from 'ethers';

declare global {
  /**
   * EIP-1193 injected provider placed on the window object by browser wallets.
   * The exact shape is library-specific; we wrap it via ethers' BrowserProvider.
   */
  interface Window {
    ethereum?: unknown;
  }
}

/**
 * Generic wallet integration for any EIP-1193 compliant injected provider.
 *
 * Implements the minimal `WalletBase` contract (fetching the current address,
 * sending transactions, and signing arbitrary messages) using ethers v6.
 */
export class Eip1193Wallet extends WalletBase {
  private constructor(private readonly provider: BrowserProvider) {
    super();
  }

  /**
   * Create a wallet from an existing EIP-1193 provider instance.
   */
  public static fromProvider(provider: Eip1193Provider): Eip1193Wallet {
    return new Eip1193Wallet(new BrowserProvider(provider));
  }

  /**
   * Request connection to the injected provider at `window.ethereum` and
   * create a new `Eip1193Wallet`.
   *
   * Internally this triggers the extension UI via `eth_requestAccounts` which
   * asks the user to authorise account access.
   *
   * @throws If no injected provider is found.
   */
  public static async connect(): Promise<Eip1193Wallet> {
    if (typeof window.ethereum === 'undefined') {
      throw new Error('EIP-1193 provider not found. Please install a compatible wallet.');
    }

    const provider = new BrowserProvider(window.ethereum as Eip1193Provider);
    await provider.send('eth_requestAccounts', []);
    return new Eip1193Wallet(provider);
  }

  /** @inheritdoc */
  public async getAddress(): Promise<string> {
    const signer = await this.provider.getSigner();
    return signer.getAddress();
  }

  /** @inheritdoc */
  public async sendTransaction(tx: TransactionRequest): Promise<string> {
    const signer = await this.provider.getSigner();
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
