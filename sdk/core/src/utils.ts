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
 * Checks if a string is a valid 0x-prefixed hex string.
 *
 * Mirrors this SDK's parsing expectations (`hexToBytes`):
 * - must start with lowercase `0x`
 * - must contain at least 1 byte (not just `0x`)
 * - must have an even number of hex characters
 * - must contain only [0-9a-fA-F]
 */
export function isHexString(v: string): v is `0x${string}` {
  if (!v.startsWith("0x")) return false;
  if (v.length === 2) return false; // bare "0x"

  const hex = v.slice(2);
  if (hex.length % 2 !== 0) return false;
  return /^[0-9a-fA-F]+$/.test(hex);
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
