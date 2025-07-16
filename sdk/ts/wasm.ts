import { createRequire } from 'node:module';
const require = createRequire(import.meta.url);
// eslint-disable-next-line @typescript-eslint/consistent-type-imports
import * as wasm from '../wasm/pkg';

export const { add } = wasm;
