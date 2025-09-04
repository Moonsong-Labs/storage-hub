let initPromise: Promise<void> | null = null;

export async function initWasm(): Promise<void> {
  if (initPromise) {
    // If initialization is already in progress, wait for the same Promise
    return initPromise;
  }

  // Create the initialization Promise and store it
  initPromise = (async () => {
    // Import the wasm glue dynamically by URL so bundlers don't inline it
    const wasmInit = (await import('../wasm/pkg/storagehub_wasm.js')).default;

    const wasmUrl = new URL('../wasm/pkg/storagehub_wasm_bg.wasm', import.meta.url);
    const isNode = typeof process !== 'undefined' && !!process.versions && !!process.versions.node;
    if (isNode) {
      // Node.js: read WASM bytes and pass as ArrayBuffer
      const fsMod = 'node:fs/promises';
      const { readFile } = await import(fsMod);
      const buf = await readFile(wasmUrl);
      await wasmInit(buf);
    } else {
      // Browser / Edge runtime: embed fallback so apps need no configuration
      try {
        const mod = (await import('./_wasm_embed.js')) as { WASM_BASE64?: unknown };
        const b64 = typeof mod.WASM_BASE64 === 'string' ? (mod.WASM_BASE64 as string) : undefined;
        if (b64 && b64.length > 0) {
          const bytes = Uint8Array.from(atob(b64), (c) => c.charCodeAt(0));
          await wasmInit(bytes);
          return;
        }
        throw new Error('No embedded WASM');
      } catch {
        // Fallback to public path if embed not present
        const overrideVal = (globalThis as Record<string, unknown>)[
          '__STORAGEHUB_WASM_PUBLIC_PATH__'
        ];
        const override = typeof overrideVal === 'string' ? (overrideVal as string) : undefined;
        await wasmInit(override ?? '/wasm/storagehub_wasm_bg.wasm');
      }
    }
  })();

  return initPromise;
}
