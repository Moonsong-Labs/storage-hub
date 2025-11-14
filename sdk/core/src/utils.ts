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
 * Converts a hex string to Uint8Array
 * @param hex - The hex string to convert (with or without 0x prefix)
 * @returns Uint8Array representation
 */
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
