import { existsSync } from "node:fs";
import { resolve } from "node:path";
import { ok } from "node:assert";

/**
 * WebSocket URI of a live chain running the StorageHub runtime.
 *
 * Required. Set via the WS_URI environment variable:
 *   WS_URI=wss://your-chain-endpoint:443 pnpm test:migrations
 */
const wsUri = process.env.WS_URI;
if (!wsUri) {
  throw new Error(
    "WS_URI environment variable is required.\n" +
      "Set it to the WebSocket endpoint of a live chain running the StorageHub runtime.\n" +
      "Example: WS_URI=wss://services.datahaven-testnet.network/testnet pnpm test:migrations"
  );
}
export const WS_URI: string = wsUri;

/**
 * Path to the compiled StorageHub parachain runtime WASM blob.
 *
 * Defaults to the standard cargo release build output. Override via WASM_PATH env var.
 */
export const WASM_PATH: string = resolve(
  process.env.WASM_PATH ??
    "../target/release/wbuild/sh-parachain-runtime/sh_parachain_runtime.compact.compressed.wasm"
);

/**
 * Assert that the WASM file exists at WASM_PATH.
 * Call in test `before()` hooks or prerequisite tests.
 *
 * @param buildHint - Optional build command to show in the error message.
 */
export function assertWasmExists(buildHint?: string): void {
  const hint = buildHint ?? "cargo build --release -p sh-parachain-runtime";
  ok(existsSync(WASM_PATH), `WASM not found at ${WASM_PATH}. Build with: ${hint}`);
}
