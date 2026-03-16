/**
 * Dynamic identity generation for network nodes.
 *
 * Generates cryptographic identities (seeds, keypairs, node keys) for dynamically
 * created network nodes. Handles key injection via RPC and provider ID retrieval
 * after on-chain registration.
 */

import { randomBytes } from "node:crypto";
import type { ApiPromise } from "@polkadot/api";
import { Keyring } from "@polkadot/api";
import type { KeyringPair } from "@polkadot/keyring/types";
import { sleep } from "../../timer";

/** Hex string type for provider IDs */
export type HexString = `0x${string}`;

/**
 * Generated identity for a node before on-chain registration.
 *
 * The lifecycle is:
 * 1. Generate identity (seed, keyring, nodeKey)
 * 2. Start container with nodeKey
 * 3. Call insertBcsvKeys RPC to inject seed → sets publicKey
 * 4. Register provider on-chain (forceBspSignUp/forceMspSignUp)
 * 5. Call getProviderId RPC → sets providerId
 */
export interface GeneratedIdentity {
  /** Seed phrase for the identity (e.g., "//BSP-Dynamic-0") */
  seed: string;
  /** Polkadot keyring pair for signing transactions */
  keyring: KeyringPair;
  /** 32-byte hex string for p2p node key */
  nodeKey: string;
  /** Public key returned from insertBcsvKeys RPC (set after key injection) */
  publicKey?: string;
  /** Provider ID returned from getProviderId RPC (set after registration) */
  providerId?: string;
}

/**
 * Generates a deterministic identity for a node.
 *
 * @param type - The type of node ("bsp" | "msp" | "fisherman" | "user")
 * @param index - The index of this node (0-based)
 * @returns Generated identity with seed, keyring, and nodeKey
 *
 * @example
 * ```ts
 * const identity = generateNodeIdentity("bsp", 0);
 * // identity.seed = "//BSP-Dynamic-0"
 * // identity.nodeKey = "0x..." (32 random bytes)
 * ```
 */
export function generateNodeIdentity(
  type: "bsp" | "msp" | "fisherman" | "user",
  index: number
): GeneratedIdentity {
  const seed = `//${type.toUpperCase()}-Dynamic-${index}`;
  const keyring = new Keyring({ type: "sr25519" });
  const pair = keyring.addFromUri(seed);
  const nodeKey = `0x${randomBytes(32).toString("hex")}`;

  return { seed, keyring: pair, nodeKey };
}

/**
 * Injects BCSV keys into a running node via RPC.
 *
 * Uses the `storagehubclient.insertBcsvKeys` RPC to inject cryptographic keys
 * into the node's keystore. Retries on failure up to maxRetries times.
 *
 * @param api - Connected API instance for the node
 * @param identity - The identity to inject (must have seed)
 * @param maxRetries - Maximum number of retry attempts (default: 3)
 * @throws Error if all retries fail
 *
 * @example
 * ```ts
 * await injectKeys(api, identity);
 * console.log(identity.publicKey); // Now populated with result from RPC
 * ```
 */
export async function injectKeys(
  api: ApiPromise,
  identity: GeneratedIdentity,
  maxRetries = 3
): Promise<void> {
  for (let i = 0; i < maxRetries; i++) {
    try {
      const publicKey = await api.rpc.storagehubclient.insertBcsvKeys(identity.seed);
      identity.publicKey = publicKey.toString();
      return;
    } catch (err) {
      if (i === maxRetries - 1) {
        throw new Error(
          `Failed to inject keys for ${identity.seed} after ${maxRetries} retries: ${err}`
        );
      }
      // Exponential backoff
      await sleep(2 ** i * 1000);
    }
  }
}

/**
 * Fetches the provider ID from a registered node via RPC.
 *
 * Must be called AFTER the provider has been registered on-chain via
 * forceBspSignUp or forceMspSignUp. Uses the `storagehubclient.getProviderId`
 * RPC to retrieve the derived provider ID.
 *
 * @param api - Connected API instance for the node
 * @param identity - The identity to fetch provider ID for
 * @throws Error if provider ID is not found (provider not registered)
 *
 * @example
 * ```ts
 * // After registration:
 * await api.tx.sudo.sudo(api.tx.providers.forceBspSignUp(...)).signAndSend(...)
 * // Now fetch provider ID:
 * await fetchProviderId(api, identity);
 * console.log(identity.providerId); // Now populated
 * ```
 */
export async function fetchProviderId(api: ApiPromise, identity: GeneratedIdentity): Promise<void> {
  const providerId = await api.rpc.storagehubclient.getProviderId();
  if (!providerId || providerId.toString() === "null") {
    throw new Error(`Provider ID not found for ${identity.seed} after registration`);
  }
  identity.providerId = providerId.toString();
}
