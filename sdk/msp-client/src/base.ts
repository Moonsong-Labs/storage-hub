import type { MspClientContext } from "./context.js";
import type { SessionProvider } from "./types.js";

/**
 * Shared reference to sessionProvider so all modules use the same instance.
 */
type SessionProviderRef = { current: SessionProvider };

export abstract class ModuleBase {
  protected readonly ctx: MspClientContext;
  protected readonly sessionProviderRef: SessionProviderRef;

  constructor(ctx: MspClientContext, sessionProviderRef: SessionProviderRef) {
    this.ctx = ctx;
    this.sessionProviderRef = sessionProviderRef;
  }

  protected async withAuth(
    headers?: Record<string, string>
  ): Promise<Record<string, string> | undefined> {
    const session = await this.sessionProviderRef.current();
    const token = session?.token;
    if (!token) return headers;
    return headers
      ? { ...headers, Authorization: `Bearer ${token}` }
      : { Authorization: `Bearer ${token}` };
  }

  /**
   * Normalize a user-provided path for HTTP query usage.
   * - Removes all leading '/' characters to avoid double slashes in URLs.
   * - Collapses any repeated slashes in the middle or at the end to a single '/'.
   * Examples:
   *   "/foo/bar"  -> "foo/bar"
   *   "///docs"   -> "docs"
   *   "foo//bar"  -> "foo/bar"
   *   "///a//b///" -> "a/b/"
   *   "foo/bar"   -> "foo/bar" (unchanged)
   *   "/"         -> ""
   */
  protected normalizePath(path: string): string {
    // Drop leading slashes (offset === 0), collapse others to '/'
    return path.replace(/^\/+|\/{2,}/g, (_m, offset: number) => (offset === 0 ? "" : "/"));
  }
}
