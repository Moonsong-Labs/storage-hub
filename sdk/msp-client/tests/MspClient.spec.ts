import { MspClient } from "../src/index.js";
import { describe, expect, it } from "vitest";

// Dummy test to satisfy Vitest until real client-side tests are implemented.
describe("MspClient", () => {
  it("connect() should return an MspClient instance", async () => {
    const sessionProvider = () => undefined;
    const client = await MspClient.connect({ baseUrl: "http://localhost" }, sessionProvider);
    expect(client).toBeInstanceOf(MspClient);
  });
});
