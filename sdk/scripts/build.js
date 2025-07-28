#!/usr/bin/env node
import { build } from 'esbuild';
import { join } from 'node:path';

const packageRoot = process.cwd();

await build({
    entryPoints: [join(packageRoot, 'src', 'index.ts')],
    outfile: join(packageRoot, 'dist', 'index.js'),
    bundle: true,
    sourcemap: true,
    minify: true,
    target: ['es2022'],
    platform: 'node',
    external: ['node:*'],
    format: 'esm',
});

console.log('Build completed'); 