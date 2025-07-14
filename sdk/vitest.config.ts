import { defineConfig } from 'vitest/config';

// TODO: set coverage to 95% according to specs
const COVERAGE_THRESHOLD = 0;

export default defineConfig({
    test: {
        environment: 'node',
        globals: true,
        coverage: {
            provider: 'v8',
            exclude: ['scripts/**', 'wasm/pkg/**'],
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