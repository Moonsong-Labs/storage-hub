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
   * Verify a Sign-In with Ethereum (SIWE) signature.
   *
   * **Advanced use only:** Most users should use the `SIWE()` method instead, which handles the complete authentication flow automatically. This method is exposed only for custom authentication flows.
   *
   * **Important:** You must store the returned Session object and provide it via the `sessionProvider` function passed to `MspClient.connect()`.
   * The session is not automatically persisted - you are responsible for managing session storage and ensuring your `sessionProvider` returns it for subsequent authenticated requests.
   *
   * @param message - The SIWE challenge message received from `getNonce()`.
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
    // Resolve the current active account from the WalletClient.
    // - Browser wallets (e.g., MetaMask) surface the user-selected address here.
    // - Viem/local wallets must set `wallet.account` explicitly before calling.
    const account = wallet.account;
    const resolvedAddress = typeof account === "string" ? account : account?.address;
    if (!resolvedAddress || !account) {
      throw new Error(
        "Wallet client has no active account; set wallet.account before calling SIWE"
      );
    }
    // Get the checksummed address
    const address = getAddress(resolvedAddress);
    const chainId = await wallet.getChainId();
    const { message } = await this.getNonce(address, chainId, domain, uri, signal);

    // Sign using the active account resolved above (string or Account object)
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
    throw lastError instanceof Error ? lastError : new Error("SIWE verification failed");
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
