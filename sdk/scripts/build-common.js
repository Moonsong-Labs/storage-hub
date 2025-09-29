#!/usr/bin/env node
import { build, context } from "esbuild";
import { join } from "node:path";
import {
  existsSync,
  rmSync,
  readFileSync,
  writeFileSync,
  mkdirSync,
  copyFileSync,
  readdirSync
} from "node:fs";
import { exec as _exec } from "node:child_process";
import { promisify } from "node:util";

const exec = promisify(_exec);

function getPackageJson(packageRoot) {
  const pkgPath = join(packageRoot, "package.json");
  return JSON.parse(readFileSync(pkgPath, "utf-8"));
}

function computeExternalDeps(pkgJson, { withWasm }) {
  // Compute esbuild "external" entries from package.json so our library
  // never bundles third‑party code. Consumers resolve these at runtime.
  const names = new Set();
  // Gather runtime package names (devDependencies intentionally ignored)
  for (const key of ["dependencies", "peerDependencies", "optionalDependencies"]) {
    const section = pkgJson[key];
    if (section && typeof section === "object") {
      for (const name of Object.keys(section)) names.add(name);
    }
  }

  const externals = [];
  // Externalize each package and its subpaths (e.g., name/interfaces)
  for (const name of names) {
    externals.push(name); // package root
    externals.push(`${name}/*`); // deep imports
  }

  if (withWasm) {
    // Keep local wasm outputs external; we embed bytes separately
    externals.push("../wasm/pkg/*", "./wasm/pkg/*", "wasm/pkg/*");
  }

  return externals;
}

async function runWasmBuildIfNeeded(packageRoot, { withWasm }) {
  if (!withWasm) return;
  const wasmDir = join(packageRoot, "wasm");
  const cargoToml = join(wasmDir, "Cargo.toml");
  // If the package has no embedded wasm crate, nothing to do
  if (!existsSync(cargoToml)) return;

  // Compile the wasm crate to web-target using wasm-pack (produces glue + .wasm)
  const cmd =
    "wasm-pack build ./wasm --target web --release --out-dir pkg && rm -f ./wasm/pkg/package.json ./wasm/pkg/.gitignore";
  await exec(cmd, { cwd: packageRoot });

  // Embed the produced .wasm as base64 into TypeScript so apps don’t host a file
  try {
    const wasmBinPath = join(packageRoot, "wasm", "pkg", "storagehub_wasm_bg.wasm");
    const embedPath = join(packageRoot, "src", "_wasm_embed.ts");
    if (existsSync(wasmBinPath)) {
      const buf = readFileSync(wasmBinPath);
      const b64 = buf.toString("base64");
      const content = `// Auto-generated at build time\nexport const WASM_BASE64 = ${JSON.stringify(b64)} as const;\n`;
      writeFileSync(embedPath, content, "utf8");
    } else {
      // Ensure module exists even if wasm pack output is missing
      const content = `// Auto-generated placeholder\nexport const WASM_BASE64 = '' as const;\n`;
      writeFileSync(embedPath, content, "utf8");
    }
  } catch (err) {
    console.warn("Failed to generate embedded WASM module:", err);
  }

  /*
   * Remove the URL fallback from the wasm-bindgen JS glue.
   * - Glue may set: module_or_path = new URL('storagehub_wasm_bg.wasm', import.meta.url)
   * - We always pass embedded bytes, so this URL must never be used.
   * - Leaving it causes bundlers to try to resolve/emit a .wasm file.
   * - Runs after wasm-pack; fail if the pattern isn’t found to avoid shipping unpatched glue.
   */
  try {
    const gluePath = join(packageRoot, "wasm", "pkg", "storagehub_wasm.js");
    // The glue must exist after wasm-pack; enforce it strictly
    if (!existsSync(gluePath)) {
      throw new Error("WASM glue not found at wasm/pkg/storagehub_wasm.js");
    }
    // Read current glue and apply a targeted replacement of the URL fallback
    const before = readFileSync(gluePath, "utf8");
    const after = before.replace(
      /if\s*\(\s*typeof\s+module_or_path\s*===\s*['"]undefined['"]\s*\)\s*\{\s*module_or_path\s*=\s*new\s+URL\([^)]*\);\s*\}/,
      "if (typeof module_or_path === 'undefined') { throw new Error('Embedded WASM required: URL fallback disabled'); }"
    );
    // Strict: if nothing changed, fail to avoid shipping an unpatched glue
    if (after === before) {
      throw new Error("WASM glue patch: no URL fallback pattern matched");
    }
    // Persist the patched glue
    writeFileSync(gluePath, after, "utf8");
  } catch (err) {
    throw new Error(
      `Failed to patch wasm glue to remove URL fallback: ${err instanceof Error ? err.message : String(err)}`
    );
  }
}

function copyAbiFiles(packageRoot, isCorePackage) {
  // Only copy ABI files for the core package
  if (!isCorePackage) {
    return; // Skip ABI copying for all other packages
  }

  const srcAbiDir = join(packageRoot, "src", "abi");
  const distAbiDir = join(packageRoot, "dist", "abi");

  if (existsSync(srcAbiDir)) {
    const files = readdirSync(srcAbiDir, { withFileTypes: true })
      .filter((dirent) => dirent.isFile() && dirent.name.endsWith(".abi.json"))
      .map((dirent) => dirent.name);

    if (files.length > 0) {
      // Create dist/abi directory
      mkdirSync(distAbiDir, { recursive: true });

      // Copy all .abi.json files
      for (const file of files) {
        const srcPath = join(srcAbiDir, file);
        const distPath = join(distAbiDir, file);
        copyFileSync(srcPath, distPath);
        console.log(`Copied ABI: ${file}`);
      }
    }
  }
}

export async function runBuild({ isCorePackage = false, watch = false } = {}) {
  const packageRoot = process.cwd();
  const pkgJson = getPackageJson(packageRoot);

  await runWasmBuildIfNeeded(packageRoot, { withWasm: isCorePackage });

  // Clean dist to avoid stale artifacts
  const distDir = join(packageRoot, "dist");
  if (existsSync(distDir)) {
    rmSync(distDir, { recursive: true, force: true });
  }

  const defaultEntry = join(packageRoot, "src", "index.ts");
  const browserEntry = existsSync(join(packageRoot, "src", "entry.browser.ts"))
    ? join(packageRoot, "src", "entry.browser.ts")
    : defaultEntry;
  const nodeEntry = existsSync(join(packageRoot, "src", "entry.node.ts"))
    ? join(packageRoot, "src", "entry.node.ts")
    : defaultEntry;

  const external = computeExternalDeps(pkgJson, { withWasm: isCorePackage });

  const common = {
    bundle: true,
    sourcemap: true,
    minify: true,
    target: ["es2022"],
    format: "esm",
    absWorkingDir: packageRoot,
    external,
    logLevel: "info"
  };

  if (watch) {
    const nodeCtx = await context({
      ...common,
      entryPoints: [nodeEntry],
      outfile: join(packageRoot, "dist", "index.node.js"),
      platform: "node",
      conditions: ["node", "import", "default"]
    });
    const browserCtx = await context({
      ...common,
      entryPoints: [browserEntry],
      outfile: join(packageRoot, "dist", "index.browser.js"),
      platform: "browser"
    });
    await Promise.all([nodeCtx.watch(), browserCtx.watch()]);
    console.log("Watching for changes...");
  } else {
    const nodeBuild = build({
      ...common,
      entryPoints: [nodeEntry],
      outfile: join(packageRoot, "dist", "index.node.js"),
      platform: "node",
      conditions: ["node", "import", "default"]
    });
    const browserBuild = build({
      ...common,
      entryPoints: [browserEntry],
      outfile: join(packageRoot, "dist", "index.browser.js"),
      platform: "browser"
    });
    await Promise.all([nodeBuild, browserBuild]);
  }

  // Copy ABI files to dist
  copyAbiFiles(packageRoot, isCorePackage);
}
