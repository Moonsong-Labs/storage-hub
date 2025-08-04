import { defineConfig } from 'vite';
import { resolve as resolvePath } from 'path';

export default defineConfig({
    root: __dirname,
    publicDir: false,
    server: {
        port: 5173,
        open: false,
    },
    resolve: {
        alias: {
            '@storagehub-sdk/core': resolvePath(__dirname, '../../core/src'),
            '@storagehub/wasm': resolvePath(__dirname, '../../core/wasm/pkg'),
        },
    },
    build: {
        outDir: 'dist',
        emptyOutDir: true,
    },
});