import { MspClient } from "../src/index.js";
import { describe, expect, it } from "vitest";
import type { Session } from "../src/types.js";

describe("MspClient", () => {
  it("should connect without sessionProvider", async () => {
    const client = await MspClient.connect({
      baseUrl: "http://localhost:8080"
    });
    expect(client).toBeInstanceOf(MspClient);
  });

  it("should allow updating sessionProvider", async () => {
    let authHeaders: Record<string, string> | undefined = undefined;
    const mockFetch = async (url: string | URL, init?: RequestInit) => {
      if (typeof url === "string" && url.includes("/auth/profile")) {
        authHeaders = (init?.headers as Record<string, string>) || undefined;
        return new Response(JSON.stringify({ address: "0x123", ens: "user.eth" }), {
          status: 200,
          headers: { "Content-Type": "application/json" }
        });
      }
      return new Response("Not Found", { status: 404 });
    };

    const client = await MspClient.connect({
      baseUrl: "http://localhost:8080",
      fetchImpl: mockFetch as typeof fetch
    });

    const session: Session = {
      token: "test-token",
      user: { address: "0x123" }
    };
    client.setSessionProvider(async () => session);

    await client.auth.getProfile();
    expect(authHeaders).toHaveProperty("Authorization");
    expect(authHeaders!["Authorization"]).toBe("Bearer test-token");
  });
});
