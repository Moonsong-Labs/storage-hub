import { defineConfig } from 'vitest/config';
import { fileURLToPath } from 'node:url';
import { resolve } from 'node:path';

// TODO: set coverage to 95% according to specs
const COVERAGE_THRESHOLD = 0;

export default defineConfig({
    resolve: {
        alias: {
            '@storagehub/wasm': resolve(fileURLToPath(new URL('.', import.meta.url)), 'core/wasm/pkg'),
        },
    },
    test: {
        environment: 'node',
        globals: true,
        watch: false,
        coverage: {
            provider: 'v8',
            exclude: ['scripts/**', '**/wasm/pkg/**'],
            reporter: ['text', 'html'],
            all: true,
            thresholds: {
                statements: COVERAGE_THRESHOLD,
                branches: COVERAGE_THRESHOLD,
                functions: COVERAGE_THRESHOLD,
                lines: COVERAGE_THRESHOLD,
            },
        },
    },
});