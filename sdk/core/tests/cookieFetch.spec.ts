import { describe, it, expect, vi } from "vitest";
import { createCookieFetch } from "../src/http/cookieFetch.js";

/**
 * Returns a vi.fn() mock that behaves like `fetch`, always responding with
 * 200 OK and the given `Set-Cookie` headers. Every response is identical,
 * so this is best for tests where the server reply doesn't need to change
 * between calls.
 */
function mockFetch(setCookieHeaders: string[] = []): typeof fetch {
  return vi.fn(async () => {
    const headers = new Headers();
    headers.append("Content-Type", "application/json");
    for (const sc of setCookieHeaders) {
      headers.append("Set-Cookie", sc);
    }
    return new Response("{}", { status: 200, headers });
  });
}

/**
 * Extracts the `Cookie` request header that `createCookieFetch` attached
 * to the Nth call (0-based) of the given mock. Returns `null` when no
 * `Cookie` header was set (i.e. the jar was empty at that point).
 */
function getCookieHeader(mock: ReturnType<typeof vi.fn>, callIndex: number): string | null {
  const call = mock.mock.calls[callIndex] as [RequestInfo, RequestInit | undefined];
  const headers = call[1]?.headers as Headers;
  return headers?.get("Cookie") ?? null;
}

describe("createCookieFetch", () => {
  // The most basic scenario: the first response sets a cookie via Set-Cookie,
  // and the wrapper should attach it as a Cookie header on the subsequent request.
  // The first request has no cookies yet, so its Cookie header should be null.
  it("should capture Set-Cookie and send it on the next request", async () => {
    const base = mockFetch(["session=abc123"]);
    const fetchWithCookies = createCookieFetch(base);

    await fetchWithCookies("https://example.com/first");
    await fetchWithCookies("https://example.com/second");

    // First request: jar is empty, no Cookie header sent.
    expect(getCookieHeader(base, 0)).toBeNull();
    // Second request: jar now contains the cookie from the first response.
    expect(getCookieHeader(base, 1)).toBe("session=abc123");
  });

  // A single HTTP response can include multiple Set-Cookie headers (one per cookie).
  // The wrapper must store all of them and send them back joined with "; " in
  // a single Cookie header, which is the standard format for the Cookie header.
  it("should handle multiple Set-Cookie headers in a single response", async () => {
    const base = mockFetch(["session=abc123", "affinity=node-1"]);
    const fetchWithCookies = createCookieFetch(base);

    await fetchWithCookies("https://example.com/first");
    await fetchWithCookies("https://example.com/second");

    const cookie = getCookieHeader(base, 1);
    // Both cookies should be present in the outgoing Cookie header.
    expect(cookie).toContain("session=abc123");
    expect(cookie).toContain("affinity=node-1");
    // They should be joined with "; " as per the Cookie header spec.
    expect(cookie).toBe("session=abc123; affinity=node-1");
  });

  // If a response doesn't include Set-Cookie, the jar should remain untouched.
  // Here the first response sets a cookie, but the second response has none.
  // Both the second and third requests should still carry the original cookie.
  it("should preserve existing cookies when response has no Set-Cookie", async () => {
    const withCookie = mockFetch(["session=abc123"]);
    const withoutCookie = mockFetch([]);

    let callCount = 0;
    const base: typeof fetch = async (input, init) => {
      callCount++;
      // Only the first response sets a cookie; subsequent ones are cookie-free.
      if (callCount === 1) return withCookie(input, init);
      return withoutCookie(input, init);
    };
    const spy = vi.fn(base);
    const fetchWithCookies = createCookieFetch(spy);

    await fetchWithCookies("https://example.com/sets-cookie");
    await fetchWithCookies("https://example.com/no-cookie");
    await fetchWithCookies("https://example.com/third");

    // Cookie persists even though the server stopped sending Set-Cookie.
    expect(getCookieHeader(spy, 1)).toBe("session=abc123");
    expect(getCookieHeader(spy, 2)).toBe("session=abc123");
  });

  // Set-Cookie headers contain attributes after the first ";", e.g.
  // "session=abc123; Path=/; HttpOnly; Secure". Only the "name=value" portion
  // before the first ";" should be stored and sent back in Cookie headers.
  it("should strip cookie attributes (Path, HttpOnly, etc.)", async () => {
    const base = mockFetch(["session=abc123; Path=/; HttpOnly; Secure; SameSite=Strict"]);
    const fetchWithCookies = createCookieFetch(base);

    await fetchWithCookies("https://example.com/first");
    await fetchWithCookies("https://example.com/second");

    // Only "session=abc123" should remain; all attributes must be stripped.
    expect(getCookieHeader(base, 1)).toBe("session=abc123");
  });

  // `Headers.getSetCookie()` is available since Node 20, and we target >=22,
  // so this scenario should not arise in practice. Still, the wrapper guards
  // with optional chaining (`?.`), and this test verifies it degrades
  // gracefully rather than throwing if the method is ever missing.
  it("should gracefully handle environments where getSetCookie is undefined", async () => {
    const base = vi.fn(async () => {
      const headers = new Headers();
      // Simulate a runtime that doesn't support getSetCookie.
      Object.defineProperty(headers, "getSetCookie", { value: undefined });
      return new Response("{}", { status: 200, headers });
    });
    const fetchWithCookies = createCookieFetch(base as unknown as typeof fetch);

    await fetchWithCookies("https://example.com/first");
    await fetchWithCookies("https://example.com/second");

    // No crash, and no cookies are sent since none could be captured.
    expect(getCookieHeader(base, 1)).toBeNull();
  });

  // Each call to createCookieFetch returns a wrapper with its own cookie jar.
  // Two separate wrappers must not share state — this is important so that
  // different SDK clients maintain independent sticky-session affinity.
  it("should maintain independent cookie stores per invocation", async () => {
    const baseA = mockFetch(["session=aaa"]);
    const baseB = mockFetch(["session=bbb"]);

    const fetchA = createCookieFetch(baseA);
    const fetchB = createCookieFetch(baseB);

    await fetchA("https://example.com/a");
    await fetchB("https://example.com/b");

    await fetchA("https://example.com/a2");
    await fetchB("https://example.com/b2");

    // Each wrapper only sees its own cookies, no cross-contamination.
    expect(getCookieHeader(baseA, 1)).toBe("session=aaa");
    expect(getCookieHeader(baseB, 1)).toBe("session=bbb");
  });

  // The current implementation uses `headers.set("Cookie", ...)`, which
  // replaces any Cookie header the caller may have passed in `init.headers`.
  // This test documents that behavior. A future improvement could merge
  // caller-supplied cookies with the jar instead of overwriting them.
  it("should overwrite caller-supplied Cookie header with jar cookies", async () => {
    const base = mockFetch(["session=from-server"]);
    const fetchWithCookies = createCookieFetch(base);

    await fetchWithCookies("https://example.com/first");
    // Caller explicitly passes a Cookie header, but the jar overwrites it.
    await fetchWithCookies("https://example.com/second", {
      headers: { Cookie: "caller=original" }
    });

    // The jar cookie wins; the caller's "caller=original" is lost.
    expect(getCookieHeader(base, 1)).toBe("session=from-server");
  });

  // Different endpoints may set different cookies across separate responses
  // (e.g. /login sets "session", /verify sets "affinity"). The jar must
  // accumulate cookies by name so that later requests carry all of them.
  // This is the key scenario that the Map-based jar was designed to fix.
  it("should merge cookies by name across multiple responses", async () => {
    let callCount = 0;
    const base = vi.fn(async (_input: RequestInfo | URL, _init?: RequestInit) => {
      callCount++;
      const headers = new Headers();
      if (callCount === 1) {
        headers.append("Set-Cookie", "session=abc123");
      } else if (callCount === 2) {
        headers.append("Set-Cookie", "affinity=node-1");
      }
      return new Response("{}", { status: 200, headers });
    }) as unknown as typeof fetch;

    const fetchWithCookies = createCookieFetch(base);

    await fetchWithCookies("https://example.com/login");
    await fetchWithCookies("https://example.com/verify");
    await fetchWithCookies("https://example.com/data");

    // By the third request both cookies from previous responses must be present.
    const cookie = getCookieHeader(base as unknown as ReturnType<typeof vi.fn>, 2);
    expect(cookie).toContain("session=abc123");
    expect(cookie).toContain("affinity=node-1");
  });

  // When the server sends a Set-Cookie with the same name but a different value
  // (e.g. session rotation), the jar should update in place rather than keeping
  // the stale value or duplicating the cookie name.
  it("should update a cookie value when the server sends a new value for the same name", async () => {
    let callCount = 0;
    const base = vi.fn(async (_input: RequestInfo | URL, _init?: RequestInit) => {
      callCount++;
      const headers = new Headers();
      if (callCount === 1) {
        headers.append("Set-Cookie", "session=old-value");
      } else if (callCount === 2) {
        headers.append("Set-Cookie", "session=new-value");
      }
      return new Response("{}", { status: 200, headers });
    }) as unknown as typeof fetch;

    const fetchWithCookies = createCookieFetch(base);

    await fetchWithCookies("https://example.com/first");
    await fetchWithCookies("https://example.com/second");
    await fetchWithCookies("https://example.com/third");

    // The jar should contain only the latest value for "session".
    const cookie = getCookieHeader(base as unknown as ReturnType<typeof vi.fn>, 2);
    expect(cookie).toBe("session=new-value");
  });
});
