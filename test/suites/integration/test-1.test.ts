import "core-js/stable";
import { strictEqual } from "node:assert";
import { randomInt } from "node:crypto";
import { describe, it } from "node:test";

describe("Suite 1", () => {
  it("Test 1.1", () => {
    strictEqual(true, true);
  });

  it("Test 1.2", async () => {
    const period = randomInt(0, 10) * 10;
    await new Promise((resolve) => setTimeout(resolve, period));
    strictEqual(true, true);
  });

  it("Test 1.3", async () => {
    const period = randomInt(0, 100) * 10;
    await new Promise((resolve) => setTimeout(resolve, period));
    strictEqual(true, true);
  });
  it("Test 1.4", async () => {
    const period = randomInt(0, 100) * 10;
    await new Promise((resolve) => setTimeout(resolve, period));
    strictEqual(true, true);
  });
  it("Test 1.5", async () => {
    const period = randomInt(0, 100) * 10;
    await new Promise((resolve) => setTimeout(resolve, period));
    strictEqual(true, true);
  });
});
