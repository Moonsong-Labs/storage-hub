import { expect } from "expect";
import { randomInt } from "node:crypto";
import { describe, it } from "node:test";

describe("Suite 3", () => {
  it("Test 3.1", () => {
    expect(true).toBe(true);
  });

  it("Test 3.2", async () => {
    const period = randomInt(0, 100) * 10;
    await new Promise((resolve) => setTimeout(resolve, period));
    expect(true).toBe(true);
  });

  it("Test 3.3", async () => {
    const period = randomInt(0, 100) * 10;
    await new Promise((resolve) => setTimeout(resolve, period));
    expect(true).toBe(true);
  });
  it("Test 3.4", async () => {
    const period = randomInt(0, 100) * 10;
    await new Promise((resolve) => setTimeout(resolve, period));
    expect(true).toBe(true);
  });
  it("Test 3.5", async () => {
    const period = randomInt(0, 100) * 10;
    await new Promise((resolve) => setTimeout(resolve, period));
    expect(true).toBe(true);
  });
});
