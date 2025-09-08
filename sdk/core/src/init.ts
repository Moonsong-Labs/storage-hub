let initPromise: Promise<void> | null = null;

/**
 * Decode a base64 string into bytes in both Node.js and browsers without relying
 * on bundler-specific polyfills.
 *
 * Rationale:
 * - In Node.js, `Buffer.from(b64, 'base64')` is the most reliable and efficient
 *   way to get bytes. `Buffer` is not available in some browser runtimes, so we
 *   feature-detect it first.
 * - In browsers, prefer `atob` and manually construct a `Uint8Array` to avoid
 *   importing additional helpers or shims. This keeps the bundle small and
 *   compatible with environments where Node globals are not present.
 * - If neither mechanism is available (very restricted runtimes), we throw a
 *   descriptive error so the caller can surface a clear message to users.
 */
function decodeBase64ToBytes(b64: string): Uint8Array {
  // Prefer Node's Buffer when available (Buffer extends Uint8Array)
  if (
    typeof Buffer !== 'undefined' &&
    typeof (Buffer as unknown as { from?: (s: string, enc: 'base64') => Uint8Array }).from ===
      'function'
  ) {
    const buf = (Buffer as unknown as { from: (s: string, enc: 'base64') => Uint8Array }).from(
      b64,
      'base64',
    );
    return new Uint8Array(buf);
  }
  // Browser fallback using atob
  const atobFn: ((s: string) => string) | undefined = (
    globalThis as unknown as {
      atob?: (s: string) => string;
    }
  ).atob;
  if (!atobFn) {
    throw new Error('Base64 decoder not available');
  }
  const binary = atobFn(b64);
  const out = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) out[i] = binary.charCodeAt(i);
  return out;
}

export async function initWasm(): Promise<void> {
  if (initPromise) {
    // If initialization is already in progress, wait for the same Promise
    return initPromise;
  }

  // Create the initialization Promise and store it
  initPromise = (async () => {
    // Import the wasm glue dynamically by URL so bundlers don't inline it
    const wasmInit = (await import('../wasm/pkg/storagehub_wasm.js')).default;

    const mod = (await import('./_wasm_embed.js')) as { WASM_BASE64?: unknown };
    const b64 = typeof mod.WASM_BASE64 === 'string' ? (mod.WASM_BASE64 as string) : undefined;
    if (!b64 || b64.length === 0) {
      throw new Error('Embedded WASM is missing or empty. Ensure build generated _wasm_embed.ts.');
    }
    const bytes = decodeBase64ToBytes(b64);
    await wasmInit(bytes);
    return;
  })();

  return initPromise;
}
