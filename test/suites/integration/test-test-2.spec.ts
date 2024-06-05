import { expect } from "expect";
import { randomInt } from "node:crypto";
import { describe, it } from "node:test";

describe("Suite 2", { only: true }, async () => {
  it("Test 2.1", () => {
    expect(true).toBe(true);
  });

  it("Test 2.2", { only: true }, async () => {
    const period = randomInt(0, 100) * 10;
    await new Promise((resolve) => setTimeout(resolve, period));
    expect(true).toBe(true);
  });

  it("Test 2.3", async () => {
    const period = randomInt(0, 100) * 10;
    await new Promise((resolve) => setTimeout(resolve, period));
    expect(true).toBe(true);
  });
  it("Test 2.4", async () => {
    const period = randomInt(0, 100) * 10;
    await new Promise((resolve) => setTimeout(resolve, period));
    expect(true).toBe(true);
  });
  it("Test 2.5", async () => {
    const period = randomInt(0, 100) * 10;
    await new Promise((resolve) => setTimeout(resolve, period));
    expect(true).toBe(true);
  });
});
