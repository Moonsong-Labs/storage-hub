let initialized = false;

export async function initWasm(): Promise<void> {
  if (initialized) return;

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

  initialized = true;
}
