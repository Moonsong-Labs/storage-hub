import { readFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

import initWasm from '@storagehub/wasm';

// Resolve the path to the *.wasm file relative to the compiled JS wrapper
const pkgDir = resolve(dirname(fileURLToPath(import.meta.url)), 'core/wasm/pkg');
const wasmPath = resolve(pkgDir, 'storagehub_wasm_bg.wasm');

// Vitest executes setup synchronously, so we need to block until the WASM module
// is fully initialised before any tests run.
await initWasm({ module_or_path: readFileSync(wasmPath) });
