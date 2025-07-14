import { createRequire } from 'node:module';
const require = createRequire(import.meta.url);
// eslint-disable-next-line @typescript-eslint/consistent-type-imports
const wasm = require('@storagehub/wasm') as typeof import('../pkg/storagehub_wasm');

export const { add } = wasm; 