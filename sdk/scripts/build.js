#!/usr/bin/env node
import { build } from 'esbuild';
import { join } from 'node:path';

const root = new URL('..', import.meta.url).pathname;

await build({
    entryPoints: [join(root, 'ts', 'index.ts')],
    outfile: join(root, 'dist', 'index.js'),
    bundle: true,
    sourcemap: true,
    minify: true,
    target: ['es2022'],
    platform: 'node',
    external: ['node:*'],
    format: 'esm',
});

console.log('Build completed'); 