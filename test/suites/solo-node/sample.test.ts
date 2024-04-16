import { expect, test, describe } from "bun:test";

// TODO: When storage-hub can be run as a solo node with --dev, we can add tests here

describe("Sample test suite", () => {
  test("2 + 2", () => {
    expect(2 + 2).toBe(4);
  });

  test("truthy", () => {
    expect(1 === Number("1")).toBeTrue;
  });
});
