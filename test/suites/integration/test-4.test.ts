import { strictEqual } from "node:assert";
import { randomInt } from "node:crypto";
import { describe, it } from "node:test";

describe("Suite 4", () => {
  it("Test 4.1", () => {
    strictEqual(true, true);
  });

  it("Test 4.2", async () => {
    const period = randomInt(0, 100) * 10;
    await new Promise((resolve) => setTimeout(resolve, period));
    strictEqual(true, true);
  });

  it("Test 4.3", async () => {
    const period = randomInt(0, 100) * 10;
    await new Promise((resolve) => setTimeout(resolve, period));
    strictEqual(true, true);
  });
  it("Test 4.4", async () => {
    const period = randomInt(0, 100) * 10;
    await new Promise((resolve) => setTimeout(resolve, period));
    strictEqual(true, true);
  });
  it("Test 4.5", async () => {
    const period = randomInt(0, 100) * 10;
    await new Promise((resolve) => setTimeout(resolve, period));
    strictEqual(true, true);
  });
});
