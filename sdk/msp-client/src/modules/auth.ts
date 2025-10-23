import type { NonceResponse, Session, UserInfo, AuthStatus } from "../types.js";
import { AuthState } from "../types.js";
import { getAddress, type WalletClient } from "viem";
import { ModuleBase } from "../base.js";

export class AuthModule extends ModuleBase {
  /**
   * Request nonce for SIWE.
   * - Input: EVM `address`, `chainId`.
   * - Output: message to sign.
   */
  private getNonce(address: string, chainId: number, signal?: AbortSignal): Promise<NonceResponse> {
    return this.ctx.http.post<NonceResponse>("/auth/nonce", {
      body: { address, chainId },
      headers: { "Content-Type": "application/json" },
      ...(signal ? { signal } : {})
    });
  }

  /**
   * Verify SIWE signature.
   * - Persists `session` in context on success.
   */
  private async verify(message: string, signature: string, signal?: AbortSignal): Promise<Session> {
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
  async SIWE(wallet: WalletClient, signal?: AbortSignal): Promise<Session> {
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
    const { message } = await this.getNonce(address, chainId, signal);

    // Sign using the active account resolved above (string or Account object)
    const signature = await wallet.signMessage({ account, message });

    return this.verify(message, signature, signal);
  }

  /**
   * Fetch authenticated user's profile.
   * - Requires valid `session` (Authorization header added automatically).
   */
  getProfile(signal?: AbortSignal): Promise<UserInfo> {
    const headers = this.withAuth();
    return this.ctx.http.get<UserInfo>("/auth/profile", {
      ...(headers ? { headers } : {}),
      ...(signal ? { signal } : {})
    });
  }

  /**
   * Determine auth status by checking token presence and profile reachability.
   */
  async getAuthStatus(): Promise<AuthStatus> {
    const headers = this.withAuth();
    const hasAuth = !!headers && "Authorization" in headers;
    if (!hasAuth) {
      return { status: AuthState.NotAuthenticated };
    }
    const profile = await this.getProfile().catch((err: any) =>
      err?.response?.status === 401 ? null : Promise.reject(err)
    );
    return profile ? { status: AuthState.Authenticated } : { status: AuthState.TokenExpired };
  }
}
