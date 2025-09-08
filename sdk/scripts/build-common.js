#!/usr/bin/env node
import { build, context } from 'esbuild';
import { join } from 'node:path';
import { existsSync, rmSync, readFileSync, writeFileSync } from 'node:fs';
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
  // If the package has no embedded wasm crate, nothing to do
  if (!existsSync(cargoToml)) return;

  // Compile the wasm crate to web-target using wasm-pack (produces glue + .wasm)
  const cmd = 'wasm-pack build ./wasm --target web --release --out-dir pkg && rm -f ./wasm/pkg/package.json ./wasm/pkg/.gitignore';
  await exec(cmd, { cwd: packageRoot });

  // Embed the produced .wasm as base64 into TypeScript so apps don’t host a file
  try {
    const wasmBinPath = join(packageRoot, 'wasm', 'pkg', 'storagehub_wasm_bg.wasm');
    const embedPath = join(packageRoot, 'src', '_wasm_embed.ts');
    if (existsSync(wasmBinPath)) {
      const buf = readFileSync(wasmBinPath);
      const b64 = buf.toString('base64');
      const content = `// Auto-generated at build time\nexport const WASM_BASE64 = ${JSON.stringify(b64)} as const;\n`;
      writeFileSync(embedPath, content, 'utf8');
    } else {
      // Ensure module exists even if wasm pack output is missing
      const content = `// Auto-generated placeholder\nexport const WASM_BASE64 = '' as const;\n`;
      writeFileSync(embedPath, content, 'utf8');
    }
  } catch (err) {
    console.warn('Failed to generate embedded WASM module:', err);
  }

  /*
   * Remove the URL fallback from the wasm-bindgen JS glue.
   * - Glue may set: module_or_path = new URL('storagehub_wasm_bg.wasm', import.meta.url)
   * - We always pass embedded bytes, so this URL must never be used.
   * - Leaving it causes bundlers to try to resolve/emit a .wasm file.
   * - Runs after wasm-pack; fail if the pattern isn’t found to avoid shipping unpatched glue.
   */
  try {
    const gluePath = join(packageRoot, 'wasm', 'pkg', 'storagehub_wasm.js');
    // The glue must exist after wasm-pack; enforce it strictly
    if (!existsSync(gluePath)) {
      throw new Error('WASM glue not found at wasm/pkg/storagehub_wasm.js');
    }
    // Read current glue and apply a targeted replacement of the URL fallback
    const before = readFileSync(gluePath, 'utf8');
    const after = before.replace(
      /if\s*\(\s*typeof\s+module_or_path\s*===\s*['\"]undefined['\"]\s*\)\s*\{\s*module_or_path\s*=\s*new\s+URL\([^)]*\);\s*\}/,
      "if (typeof module_or_path === 'undefined') { throw new Error('Embedded WASM required: URL fallback disabled'); }",
    );
    // Strict: if nothing changed, fail to avoid shipping an unpatched glue
    if (after === before) {
      throw new Error('WASM glue patch: no URL fallback pattern matched');
    }
    // Persist the patched glue
    writeFileSync(gluePath, after, 'utf8');
  } catch (err) {
    throw new Error(`Failed to patch wasm glue to remove URL fallback: ${err instanceof Error ? err.message : String(err)}`);
  }
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


