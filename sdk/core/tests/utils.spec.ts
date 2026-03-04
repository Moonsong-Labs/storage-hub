import { describe, it, expect } from "vitest";
import { ensure0xPrefix, removeHexPrefix, assert0xString, hexToBytes } from "../src/utils.js";

describe("Hex Utility Functions", () => {
  describe("ensure0xPrefix", () => {
    it("should add 0x prefix to hex string without prefix", () => {
      expect(ensure0xPrefix("abc123")).toBe("0xabc123");
      expect(ensure0xPrefix("deadbeef")).toBe("0xdeadbeef");
      expect(ensure0xPrefix("")).toBe("0x");
    });

    it("should preserve 0x prefix if already present", () => {
      expect(ensure0xPrefix("0xabc123")).toBe("0xabc123");
      expect(ensure0xPrefix("0xdeadbeef")).toBe("0xdeadbeef");
      expect(ensure0xPrefix("0x")).toBe("0x");
    });

    it("should return correct TypeScript type", () => {
      const result = ensure0xPrefix("abc123");
      // This test ensures the return type is `0x${string}`
      expect(result).toMatch(/^0x/);
    });
  });

  describe("removeHexPrefix", () => {
    it("should remove 0x prefix from hex string", () => {
      expect(removeHexPrefix("0xabc123")).toBe("abc123");
      expect(removeHexPrefix("0xdeadbeef")).toBe("deadbeef");
      expect(removeHexPrefix("0x")).toBe("");
    });

    it("should leave hex string unchanged if no prefix", () => {
      expect(removeHexPrefix("abc123")).toBe("abc123");
      expect(removeHexPrefix("deadbeef")).toBe("deadbeef");
      expect(removeHexPrefix("")).toBe("");
    });
  });

  describe("hexToBytes", () => {
    it("should convert hex string with 0x prefix to Uint8Array", () => {
      const result = hexToBytes("0xabc123");
      expect(result).toBeInstanceOf(Uint8Array);
      expect(Array.from(result)).toEqual([0xab, 0xc1, 0x23]);
    });

    it("should convert hex string without 0x prefix to Uint8Array", () => {
      const result = hexToBytes("abc123");
      expect(result).toBeInstanceOf(Uint8Array);
      expect(Array.from(result)).toEqual([0xab, 0xc1, 0x23]);
    });

    it("should handle empty hex string", () => {
      expect(() => hexToBytes("")).toThrow("Hex string cannot be empty");
      expect(() => hexToBytes("0x")).toThrow("Hex string cannot be empty");
    });

    it("should handle uppercase hex strings", () => {
      const result = hexToBytes("0xDEADBEEF");
      expect(Array.from(result)).toEqual([0xde, 0xad, 0xbe, 0xef]);
    });

    it("should handle mixed case hex strings", () => {
      const result = hexToBytes("0xDeAdBeEf");
      expect(Array.from(result)).toEqual([0xde, 0xad, 0xbe, 0xef]);
    });

    it("should throw error for odd-length hex strings", () => {
      expect(() => hexToBytes("0xabc")).toThrow(
        "Hex string must have an even number of characters"
      );
      expect(() => hexToBytes("abc")).toThrow("Hex string must have an even number of characters");
    });

    it("should throw error for invalid hex characters", () => {
      expect(() => hexToBytes("0xabcg")).toThrow("Hex string contains invalid characters");
      expect(() => hexToBytes("abcg")).toThrow("Hex string contains invalid characters");
      expect(() => hexToBytes("0x123z")).toThrow("Hex string contains invalid characters");
    });

    it("should handle special characters that might look like hex", () => {
      expect(() => hexToBytes("0xabco")).toThrow("Hex string contains invalid characters");
      expect(() => hexToBytes("0x123!")).toThrow("Hex string contains invalid characters");
    });
  });

  describe("assert0xString", () => {
    it("should accept valid 0x-prefixed hex with expected byte length", () => {
      expect(() =>
        assert0xString(
          "0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
          32,
          "Invalid hex"
        )
      ).not.toThrow();
    });

    it("should throw if input is not 0x-prefixed", () => {
      expect(() =>
        assert0xString(
          "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
          32,
          "Invalid hex"
        )
      ).toThrow("Invalid hex");
    });

    it("should throw if input length does not match expected bytes", () => {
      expect(() => assert0xString("0x0123456789abcdef", 32, "Invalid hex length")).toThrow(
        "Invalid hex length"
      );
    });

    it("should throw if input contains non-hex characters", () => {
      expect(() =>
        assert0xString(
          "0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdeg",
          32,
          "Invalid hex chars"
        )
      ).toThrow("Invalid hex chars");
    });

    it("should throw for invalid expected byte length argument", () => {
      expect(() => assert0xString("0x12", 0, "Invalid hex")).toThrow(
        "Expected length must be a positive integer"
      );
      expect(() => assert0xString("0x12", -1, "Invalid hex")).toThrow(
        "Expected length must be a positive integer"
      );
    });
  });
});
