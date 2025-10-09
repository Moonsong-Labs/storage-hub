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
    this.ctx.session = session;
    return session;
  }

  /**
   * Full SIWE flow using a `WalletClient`.
   * - Derives address, fetches nonce, signs message, verifies and stores session.
   */
  async SIWE(wallet: WalletClient, signal?: AbortSignal): Promise<void> {
    const rawAddress = (await wallet.getAddresses())?.[0];
    if (!rawAddress) throw new Error("No wallet addresses found");
    if (!wallet.account) throw new Error("Wallet client has no active account");

    // Get the checksummed address
    const address = getAddress(rawAddress);
    const chainId = await wallet.getChainId();
    const { message } = await this.getNonce(address, chainId, signal);
    const signature = await wallet.signMessage({ account: wallet.account, message });

    this.ctx.session = await this.verify(message, signature, signal);
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
    if (!this.ctx.session?.token) {
      return { status: AuthState.NotAuthenticated };
    }
    const profile = await this.getProfile().catch((err: any) =>
      err?.response?.status === 401 ? null : Promise.reject(err)
    );
    return profile ? { status: AuthState.Authenticated } : { status: AuthState.TokenExpired };
  }
}
