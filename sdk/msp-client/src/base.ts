import type { MspClientContext } from "./context.js";

export abstract class ModuleBase {
  protected readonly ctx: MspClientContext;

  constructor(ctx: MspClientContext) {
    this.ctx = ctx;
  }

  protected withAuth(headers?: Record<string, string>): Record<string, string> | undefined {
    const token = this.ctx.session?.token;
    return token ? { ...(headers ?? {}), Authorization: `Bearer ${token}` } : headers;
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
