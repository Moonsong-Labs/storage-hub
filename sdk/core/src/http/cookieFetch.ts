/**
 * Creates a fetch wrapper that automatically captures `Set-Cookie` response
 * headers and sends them back as `Cookie` on subsequent requests.
 *
 * Browsers handle cookies natively, but Node.js `fetch` ignores them entirely.
 * When a load-balancer uses cookie-based affinity (sticky sessions), this
 * wrapper ensures consecutive requests are routed to the same backend instance.
 *
 * Each invocation returns an independent cookie store, so separate clients
 * maintain separate affinity.
 */
export type FetchLike = (
  input: RequestInfo | URL,
  init?: RequestInit
) => Promise<Response>;

export function createCookieFetch(baseFetch: FetchLike): FetchLike {
  const cookieJar = new Map<string, string>();

  return async (input, init) => {
    const headers = new Headers(init?.headers);
    if (cookieJar.size > 0) {
      headers.set("Cookie", [...cookieJar.values()].join("; "));
    }

    const res = await baseFetch(input, { ...init, headers });

    const setCookie = res.headers.getSetCookie?.();
    if (setCookie?.length) {
      for (const c of setCookie) {
        const pair = c.split(";", 1)[0] ?? "";
        const name = pair.split("=", 1)[0] ?? "";
        cookieJar.set(name, pair);
      }
    }

    return res;
  };
}
