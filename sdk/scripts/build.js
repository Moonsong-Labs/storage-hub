#!/usr/bin/env node
import { build } from 'esbuild';
import { join } from 'node:path';
import { existsSync } from 'node:fs';

const packageRoot = process.cwd();

const builds = [];

// Default module (browser-targeted)
builds.push(build({
    entryPoints: [join(packageRoot, 'src', 'index.ts')],
    outfile: join(packageRoot, 'dist', 'index.js'),
    bundle: true,
    sourcemap: true,
    minify: true,
    target: ['es2022'],
    platform: 'browser',
    external: ['ethers', '@polkadot/*', 'bn.js'],
    format: 'esm',
}));

// Optional browser entry with auto-init
const browserEntry = join(packageRoot, 'src', 'entry.browser.ts');
if (existsSync(browserEntry)) {
    builds.push(build({
        entryPoints: [browserEntry],
        outfile: join(packageRoot, 'dist', 'index.browser.js'),
        bundle: true,
        sourcemap: true,
        minify: true,
        target: ['es2022'],
        platform: 'browser',
        external: ['ethers', '@polkadot/*', 'bn.js'],
        format: 'esm',
    }));
}

// Optional node entry with auto-init
const nodeEntry = join(packageRoot, 'src', 'entry.node.ts');
if (existsSync(nodeEntry)) {
    builds.push(build({
        entryPoints: [nodeEntry],
        outfile: join(packageRoot, 'dist', 'index.node.js'),
        bundle: true,
        sourcemap: true,
        minify: true,
        target: ['es2022'],
        platform: 'node',
        conditions: ['node', 'import', 'default'],
        external: ['ethers', '@polkadot/*', 'bn.js'],
        format: 'esm',
    }));
}

await Promise.all(builds);

console.log('Build completed');