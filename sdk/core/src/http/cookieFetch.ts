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
export function createCookieFetch(baseFetch: typeof fetch): typeof fetch {
  let cookies: string[] = [];

  return async (input, init) => {
    const headers = new Headers(init?.headers);
    if (cookies.length > 0) {
      headers.set("Cookie", cookies.join("; "));
    }

    const res = await baseFetch(input, { ...init, headers });

    const setCookie = res.headers.getSetCookie?.();
    if (setCookie?.length) {
      cookies = setCookie.map((c) => c.split(";", 1)[0]!);
    }

    return res;
  };
}
