import type { NonceResponse, Session, UserInfo } from "../types.js";
import { getAddress, type WalletClient } from "viem";
import { ModuleBase } from "../base.js";

const DEFAULT_SIWE_VERIFY_RETRY_ATTEMPS = 10;
const DEFAULT_SIWE_VERIFY_BACKOFF_MS = 100;

export class AuthModule extends ModuleBase {
  /**
   * Request a nonce (challenge message) for Sign-In with Ethereum (SIWE).
   *
   * **Advanced use only:** Most users should use the `SIWE()` method instead, which handles the complete authentication flow automatically. This method is exposed only for custom authentication flows.
   *
   * **Important:** The challenge message expires after a short time (typically 5 minutes). You must call `verify()` with a valid signature before expiration.
   *
   * @param address - The Ethereum address requesting authentication (checksummed format recommended).
   * @param chainId - The chain ID the user is connected to.
   * @param domain - The domain (host[:port]) for the SIWE message (e.g., "datahaven.app" or "localhost:3000").
   * @param uri - The full URI of your application (e.g., "https://datahaven.app" or "http://localhost:3000").
   * @param signal - Optional AbortSignal for request cancellation.
   * @returns A promise resolving to the SIWE challenge message to be signed.
   */
  public getNonce(
    address: string,
    chainId: number,
    domain: string,
    uri: string,
    signal?: AbortSignal
  ): Promise<NonceResponse> {
    return this.ctx.http.post<NonceResponse>("/auth/nonce", {
      body: {
        address,
        chainId,
        domain,
        uri
      },
      headers: { "Content-Type": "application/json" },
      ...(signal ? { signal } : {})
    });
  }

  /**
   * Request a message (challenge) for Sign-In with X (SIWX) using CAIP-122 standard.
   *
   * **Advanced use only:** Most users should use the `SIWX()` method instead, which handles the complete authentication flow automatically. This method is exposed only for custom authentication flows.
   *
   * This method follows the CAIP-122 standard for chain-agnostic authentication.
   * The message uses CAIP-10 format for addresses (e.g., `eip155:55931:0x...`).
   *
   * **Important:** The challenge message expires after a short time (typically 5 minutes).
   * You must call `verify()` with a valid signature before expiration.
   *
   * **Note:** According to CAIP-122, the domain is extracted from the URI automatically.
   * You do not need to provide the domain separately - it will be extracted from the URI.
   *
   * @param address - The blockchain address requesting authentication (checksummed format recommended).
   * @param chainId - The chain ID the user is connected to.
   * @param uri - The full URI of your dApp (e.g., "https://datahaven.app"). This should be the dApp URL, not the MSP API URL.
   *   The domain will be automatically extracted from this URI per CAIP-122 specification.
   * @param signal - Optional AbortSignal for request cancellation.
   * @returns A promise resolving to the CAIP-122 challenge message to be signed.
   */
  public getMessage(
    address: string,
    chainId: number,
    uri: string,
    signal?: AbortSignal
  ): Promise<NonceResponse> {
    return this.ctx.http.post<NonceResponse>("/auth/message", {
      body: {
        address,
        chainId,
        uri
      },
      headers: { "Content-Type": "application/json" },
      ...(signal ? { signal } : {})
    });
  }

  /**
   * Verify a Sign-In signature (works with both SIWE and SIWX/CAIP-122 messages).
   *
   * **Advanced use only:** Most users should use the `SIWE()` or `SIWX()` methods instead, which handle the complete authentication flow automatically. This method is exposed only for custom authentication flows.
   *
   * **Important:** You must store the returned Session object and provide it via the `sessionProvider` function passed to `MspClient.connect()`.
   * The session is not automatically persisted - you are responsible for managing session storage and ensuring your `sessionProvider` returns it for subsequent authenticated requests.
   *
   * @param message - The challenge message received from `getNonce()` (SIWE) or `getMessage()` (CAIP-122).
   * @param signature - The signature of the message signed by the user's wallet.
   * @param signal - Optional AbortSignal for request cancellation.
   * @returns A promise resolving to a Session object that you must store and provide via your sessionProvider.
   */
  public async verify(message: string, signature: string, signal?: AbortSignal): Promise<Session> {
    const session = await this.ctx.http.post<Session>("/auth/verify", {
      body: { message, signature },
      headers: { "Content-Type": "application/json" },
      ...(signal ? { signal } : {})
    });

    return session;
  }

  /**
   * Complete Sign-In with Ethereum (SIWE) authentication flow using a `WalletClient`.
   *
   * This is the recommended method for authentication. It handles the complete flow automatically:
   * derives the wallet address, fetches a nonce, prompts the user to sign the message, verifies the signature,
   * and returns a session token.
   *
   * **Important:** You must store the returned Session object and provide it via the `sessionProvider` function
   * passed to `MspClient.connect()`. The session is not automatically persisted - you are responsible for managing
   * session storage and ensuring your `sessionProvider` returns it for subsequent authenticated requests.
   *
   * **Note:** This method includes automatic retry logic for verification requests (default: 10 attempts with 100ms backoff).
   * The retry behavior can be customized via the `retry` parameter.
   *
   * @param wallet - The Viem `WalletClient` instance. Must have an active account set (`wallet.account`).
   *   - Browser wallets (e.g., MetaMask) automatically surface the user-selected address.
   *   - Viem/local wallets must set `wallet.account` explicitly before calling.
   * @param domain - The domain (host[:port]) for the SIWE message (e.g., "datahaven.app" or "localhost:3000").
   * @param uri - The full URI of your application (e.g., "https://datahaven.app" or "http://localhost:3000").
   * @param retry - Number of retry attempts for verification requests (default: 10).
   * @param signal - Optional AbortSignal for request cancellation.
   * @returns A promise resolving to a Session object that you must store and provide via your sessionProvider.
   */
  async SIWE(
    wallet: WalletClient,
    domain: string,
    uri: string,
    retry = DEFAULT_SIWE_VERIFY_RETRY_ATTEMPS,
    signal?: AbortSignal
  ): Promise<Session> {
    const { account, address, chainId } = await this.resolveAccount(wallet, "SIWE");
    const { message } = await this.getNonce(address, chainId, domain, uri, signal);
    return this.signAndVerifyWithRetry(wallet, account, message, retry, signal, "SIWE");
  }

  /**
   * Complete Sign-In with X (SIWX) authentication flow using CAIP-122 standard and a `WalletClient`.
   *
   * This is the recommended method for CAIP-122 authentication. It handles the complete flow automatically:
   * derives the wallet address, fetches a CAIP-122 message, prompts the user to sign the message, verifies the signature,
   * and returns a session token.
   *
   * **Important:** You must store the returned Session object and provide it via the `sessionProvider` function
   * passed to `MspClient.connect()`. The session is not automatically persisted - you are responsible for managing
   * session storage and ensuring your `sessionProvider` returns it for subsequent authenticated requests.
   *
   * **Note:** This method includes automatic retry logic for verification requests (default: 10 attempts with 100ms backoff).
   * The retry behavior can be customized via the `retry` parameter.
   *
   * **CAIP-122:** Unlike SIWE, this method does not require a `domain` parameter. The domain is automatically extracted
   * from the `uri` parameter on the backend per CAIP-122 specification.
   *
   * @param wallet - The Viem `WalletClient` instance. Must have an active account set (`wallet.account`).
   *   - Browser wallets (e.g., MetaMask) automatically surface the user-selected address.
   *   - Viem/local wallets must set `wallet.account` explicitly before calling.
   * @param uri - The full URI of your dApp (e.g., "https://datahaven.app"). This should be the dApp URL, not the MSP API URL.
   *   The domain will be automatically extracted from this URI per CAIP-122 specification.
   * @param retry - Number of retry attempts for verification requests (default: 10).
   * @param signal - Optional AbortSignal for request cancellation.
   * @returns A promise resolving to a Session object that you must store and provide via your sessionProvider.
   */
  async SIWX(
    wallet: WalletClient,
    uri: string,
    retry = DEFAULT_SIWE_VERIFY_RETRY_ATTEMPS,
    signal?: AbortSignal
  ): Promise<Session> {
    const { account, address, chainId } = await this.resolveAccount(wallet, "SIWX");
    const { message } = await this.getMessage(address, chainId, uri, signal);
    return this.signAndVerifyWithRetry(wallet, account, message, retry, signal, "SIWX");
  }

  /**
   * Resolves and validates the account from a WalletClient.
   *
   * @param wallet - The Viem WalletClient instance.
   * @param methodName - The name of the calling method (for error messages).
   * @returns An object containing the account, checksummed address, and chainId.
   * @throws Error if the wallet has no active account.
   */
  private async resolveAccount(
    wallet: WalletClient,
    methodName: string
  ): Promise<{ account: NonNullable<WalletClient["account"]>; address: string; chainId: number }> {
    const account = wallet.account;
    const resolvedAddress = typeof account === "string" ? account : account?.address;
    if (!resolvedAddress || !account) {
      throw new Error(
        `Wallet client has no active account; set wallet.account before calling ${methodName}`
      );
    }
    const address = getAddress(resolvedAddress);
    const chainId = await wallet.getChainId();
    return { account, address, chainId };
  }

  /**
   * Signs a message and verifies it with retry logic.
   *
   * @param wallet - The Viem WalletClient instance.
   * @param account - The account to sign with.
   * @param message - The message to sign.
   * @param retry - Number of retry attempts for verification.
   * @param signal - Optional AbortSignal for request cancellation.
   * @param methodName - The name of the calling method (for error messages).
   * @returns A promise resolving to a Session object.
   */
  private async signAndVerifyWithRetry(
    wallet: WalletClient,
    account: NonNullable<WalletClient["account"]>,
    message: string,
    retry: number,
    signal: AbortSignal | undefined,
    methodName: string
  ): Promise<Session> {
    const signature = await wallet.signMessage({ account, message });

    // TODO: remove the retry logic once the backend is fixed.
    let lastError: unknown;
    for (let attemptIndex = 0; attemptIndex < retry; attemptIndex++) {
      try {
        return await this.verify(message, signature, signal);
      } catch (err) {
        lastError = err;
        await this.delay(DEFAULT_SIWE_VERIFY_BACKOFF_MS);
      }
    }
    throw lastError instanceof Error ? lastError : new Error(`${methodName} verification failed`);
  }

  private async delay(ms: number): Promise<void> {
    await new Promise((resolve) => setTimeout(resolve, ms));
  }

  /**
   * Fetch authenticated user's profile.
   * - Requires valid `session` (Authorization header added automatically).
   */
  async getProfile(signal?: AbortSignal): Promise<UserInfo> {
    const headers = await this.withAuth();
    return this.ctx.http.get<UserInfo>("/auth/profile", {
      ...(headers ? { headers } : {}),
      ...(signal ? { signal } : {})
    });
  }
}
