let initPromise: Promise<void> | null = null;

export async function initWasm(): Promise<void> {
  if (initPromise) {
    // If initialization is already in progress, wait for the same Promise
    return initPromise;
  }

  // Create the initialization Promise and store it
  initPromise = (async () => {
    // Import the web-style init function
    const wasmInit = (await import('../wasm/pkg/storagehub_wasm.js')).default;

    const wasmUrl = new URL('../wasm/pkg/storagehub_wasm_bg.wasm', import.meta.url);
    if (typeof window === 'undefined') {
      // Node.js: read WASM bytes and pass as ArrayBuffer
      const fsMod = 'node:fs/promises';
      const { readFile } = await import(fsMod);
      const buf = await readFile(wasmUrl);
      await wasmInit(buf);
    } else {
      // Browser: pass URL or fetch promise
      await wasmInit(wasmUrl.href);
    }
  })();

  return initPromise;
}
