import { expect, test, describe } from "bun:test";

describe("Sample test suite", () => {
  test("2 + 2", () => {
    expect(2 + 2).toBe(4);
  });

  test("truthy", () => {
    expect(1 === Number("1")).toBeTrue;
  });
});
