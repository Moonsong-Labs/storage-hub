/**
 * Utility functions for StorageHub SDK
 */

/**
 * Ensures a hex string has the '0x' prefix
 * @param hex - The hex string to process
 * @returns The hex string with '0x' prefix
 */
export function ensure0xPrefix(hex: string): `0x${string}` {
  return hex.startsWith("0x") ? (hex as `0x${string}`) : (`0x${hex}` as `0x${string}`);
}

/**
 * Removes the '0x' prefix from a hex string if present
 * @param hex - The hex string to process
 * @returns The hex string without '0x' prefix
 */
export function removeHexPrefix(hex: string): string {
  return hex.startsWith("0x") ? hex.slice(2) : hex;
}

/**
 * Validates a 0x-prefixed hex string has the expected byte length.
 * @param value - Hex string to validate
 * @param expectedLength - Expected byte length (excluding 0x prefix)
 * @param errorMessage - Error message thrown when validation fails
 * @throws Error if the input is not 0x-prefixed hex with the expected byte length
 */
export function assert0xString(
  value: string,
  expectedLength: number,
  errorMessage: string
): asserts value is `0x${string}` {
  if (expectedLength <= 0) {
    throw new Error("Expected length must be a positive integer");
  }

  const expectedHexLength = expectedLength * 2;
  const regex = new RegExp(`^0x[0-9a-fA-F]{${expectedHexLength}}$`);
  if (!regex.test(value)) {
    throw new Error(errorMessage);
  }
}

/**
 * Converts a hex string to Uint8Array
 * @param hex - The hex string to convert (with or without 0x prefix)
 * @returns Uint8Array representation
 */
/**
 * Safely parse a date string, validating that it results in a valid Date object.
 * @param dateString - The date string to parse (expected to be an ISO timestamp)
 * @returns A valid Date object
 * @throws Error if the date string is invalid or results in an Invalid Date
 */
export function parseDate(dateString: string): Date {
  if (typeof dateString !== "string" || dateString.trim() === "") {
    throw new Error(
      `Invalid date string: expected non-empty string, got ${JSON.stringify(dateString)}`
    );
  }

  const date = new Date(dateString);
  if (Number.isNaN(date.getTime())) {
    throw new Error(`Invalid date string: "${dateString}" cannot be parsed as a valid date`);
  }

  return date;
}

export function hexToBytes(hex: string): Uint8Array {
  if (!hex) {
    throw new Error("Hex string cannot be empty");
  }

  const cleanHex = removeHexPrefix(hex);

  if (!cleanHex) {
    throw new Error("Hex string cannot be empty");
  }

  if (cleanHex.length % 2 !== 0) {
    throw new Error("Hex string must have an even number of characters");
  }

  if (!/^[0-9a-fA-F]*$/.test(cleanHex)) {
    throw new Error("Hex string contains invalid characters");
  }

  return new Uint8Array(cleanHex.match(/.{2}/g)?.map((byte) => Number.parseInt(byte, 16)) || []);
}
