import { createRequire } from 'node:module';
const require = createRequire(import.meta.url);
// eslint-disable-next-line @typescript-eslint/consistent-type-imports
const wasm = require('../wasm/pkg') as typeof import('../wasm/pkg/storagehub_wasm.js');

export const { add } = wasm;
