import type { NonceResponse, Session, UserInfo } from "../types.js";
import { getAddress, type WalletClient } from "viem";
import { ModuleBase } from "../base.js";

const DEFAULT_SIWE_VERIFY_RETRY_ATTEMPS = 10;
const DEFAULT_SIWE_VERIFY_BACKOFF_MS = 100;

export class AuthModule extends ModuleBase {
  /**
   * Request nonce for SIWE.
   * - Input: EVM `address`, `chainId`.
   * - Output: message to sign.
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
   * Verify SIWE signature.
   * - Persists `session` in context on success.
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
   * Full SIWE flow using a `WalletClient`.
   * - Derives address, fetches nonce, signs message, verifies and stores session.
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
