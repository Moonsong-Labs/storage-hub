#!/usr/bin/env node
import { build, context } from 'esbuild';
import { join } from 'node:path';
import { existsSync, rmSync, readFileSync } from 'node:fs';
import { exec as _exec } from 'node:child_process';
import { promisify } from 'node:util';

const exec = promisify(_exec);

function getPackageJson(packageRoot) {
  const pkgPath = join(packageRoot, 'package.json');
  return JSON.parse(readFileSync(pkgPath, 'utf-8'));
}

function computeExternalDeps(pkgJson, { isCorePackage }) {
  const deps = new Set([
    'ethers',
    '@polkadot/*',
    'bn.js',
  ]);

  for (const key of ['dependencies', 'peerDependencies', 'optionalDependencies']) {
    const section = pkgJson[key];
    if (section && typeof section === 'object') {
      for (const name of Object.keys(section)) {
        deps.add(name);
      }
    }
  }

  if (isCorePackage) {
    // Ensure runtime WASM loader stays external and on-disk
    deps.add('../wasm/pkg/*');
    deps.add('./wasm/pkg/*');
    deps.add('wasm/pkg/*');
  }

  return Array.from(deps);
}

async function runWasmBuildIfNeeded(packageRoot, { withWasm }) {
  if (!withWasm) return;
  const wasmDir = join(packageRoot, 'wasm');
  const cargoToml = join(wasmDir, 'Cargo.toml');
  if (!existsSync(cargoToml)) return;

  const cmd = 'wasm-pack build ./wasm --target web --release --out-dir pkg && rm -f ./wasm/pkg/package.json ./wasm/pkg/.gitignore';
  await exec(cmd, { cwd: packageRoot });
}

export async function runBuild({ withWasm = false, watch = false } = {}) {
  const packageRoot = process.cwd();
  const pkgJson = getPackageJson(packageRoot);
  const isCorePackage = pkgJson.name === '@storagehub-sdk/core';

  await runWasmBuildIfNeeded(packageRoot, { withWasm });

  // Clean dist to avoid stale artifacts
  const distDir = join(packageRoot, 'dist');
  if (existsSync(distDir)) {
    rmSync(distDir, { recursive: true, force: true });
  }

  const defaultEntry = join(packageRoot, 'src', 'index.ts');
  const browserEntry = existsSync(join(packageRoot, 'src', 'entry.browser.ts'))
    ? join(packageRoot, 'src', 'entry.browser.ts')
    : defaultEntry;
  const nodeEntry = existsSync(join(packageRoot, 'src', 'entry.node.ts'))
    ? join(packageRoot, 'src', 'entry.node.ts')
    : defaultEntry;

  const external = computeExternalDeps(pkgJson, { isCorePackage });

  const common = {
    bundle: true,
    sourcemap: true,
    minify: true,
    target: ['es2022'],
    format: 'esm',
    absWorkingDir: packageRoot,
    external,
    logLevel: 'info',
  };

  if (watch) {
    const nodeCtx = await context({
      ...common,
      entryPoints: [nodeEntry],
      outfile: join(packageRoot, 'dist', 'index.node.js'),
      platform: 'node',
      conditions: ['node', 'import', 'default'],
    });
    const browserCtx = await context({
      ...common,
      entryPoints: [browserEntry],
      outfile: join(packageRoot, 'dist', 'index.browser.js'),
      platform: 'browser',
    });
    await Promise.all([nodeCtx.watch(), browserCtx.watch()]);
    console.log('Watching for changes...');
  } else {
    const nodeBuild = build({
      ...common,
      entryPoints: [nodeEntry],
      outfile: join(packageRoot, 'dist', 'index.node.js'),
      platform: 'node',
      conditions: ['node', 'import', 'default'],
    });
    const browserBuild = build({
      ...common,
      entryPoints: [browserEntry],
      outfile: join(packageRoot, 'dist', 'index.browser.js'),
      platform: 'browser',
    });
    await Promise.all([nodeBuild, browserBuild]);
  }
}

// If invoked directly from CLI
if (import.meta.url === `file://${process.argv[1]}`) {
  const watch = process.argv.includes('--watch');
  const withWasm = process.argv.includes('--with-wasm');
  runBuild({ withWasm, watch })
    .then(() => {
      if (!watch) console.log('Build completed');
    })
    .catch((err) => {
      console.error(err);
      process.exit(1);
    });
}


