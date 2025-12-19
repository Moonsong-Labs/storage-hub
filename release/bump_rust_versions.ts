#!/usr/bin/env bun

/**
 * Bump Rust crate versions for a given family so that:
 *   crate_version >= target_version (semver, x.y.z)
 *
 * - Reads configuration from release/versions.json
 * - Discovers Cargo.toml files under the configured roots
 * - Updates only the [package] version field (not dependencies)
 *
 * Usage examples:
 *   bun release/bump_rust_versions.ts
 *   bun release/bump_rust_versions.ts --family core_rust
 *   bun release/bump_rust_versions.ts --family core_rust --version 0.3.0
 *   bun release/bump_rust_versions.ts --dry-run
 */

import * as fs from "node:fs";
import * as path from "node:path";

interface CliArgs {
  family: string;
  versionOverride: string | null;
  dryRun: boolean;
  help?: boolean;
}

interface VersionFamilyConfig {
  version?: string;
  roots: string[];
}

interface VersionsManifest {
  families: Record<string, VersionFamilyConfig>;
}

interface ManifestWithPath {
  path: string;
  data: VersionsManifest;
}

interface PackageVersionInfo {
  value: string;
  indexInFile: number;
  length: number;
}

interface ParsedSemver {
  major: number;
  minor: number;
  patch: number;
  pre: string | null;
}

function printHelp(): void {
  console.log(
    [
      "Usage: bun release/bump_rust_versions.ts [options]",
      "",
      "Options:",
      "  --family <name>    Version family to use from release/versions.json (default: core_rust)",
      "  --version <x.y.z>  Override target version for the chosen family",
      "  --dry-run          Show planned changes without modifying files",
      "  --help             Show this help message",
      "",
      "Behaviour:",
      "  - Only updates [package] version fields in Cargo.toml files.",
      "  - Skips crates whose version is already greater than or equal to the target."
    ].join("\n")
  );
}

function parseArgs(argv: string[]): CliArgs {
  const args: CliArgs = {
    family: "core_rust",
    versionOverride: null,
    dryRun: false
  };

  for (let i = 2; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === "--help" || arg === "-h") {
      args.help = true;
      break;
    }
    if (arg === "--family") {
      args.family = argv[i + 1];
      i += 1;
    } else if (arg === "--version") {
      args.versionOverride = argv[i + 1];
      i += 1;
    } else if (arg === "--dry-run") {
      args.dryRun = true;
    } else {
      console.warn(`Unknown argument: ${arg}`);
    }
  }

  return args;
}

function loadManifest(rootDir: string): ManifestWithPath {
  const manifestPath = path.join(rootDir, "release", "versions.json");
  if (!fs.existsSync(manifestPath)) {
    throw new Error(`Manifest file not found at ${manifestPath}`);
  }

  const raw = fs.readFileSync(manifestPath, "utf8");
  let data: VersionsManifest;
  try {
    data = JSON.parse(raw) as VersionsManifest;
  } catch (e) {
    const err = e as Error;
    throw new Error(`Failed to parse JSON in ${manifestPath}: ${err.message}`);
  }

  if (!data.families || typeof data.families !== "object") {
    throw new Error("Manifest file must contain a 'families' object");
  }

  return { path: manifestPath, data };
}

function getFamilyConfig(manifest: ManifestWithPath, familyName: string): VersionFamilyConfig {
  const family = manifest.data.families[familyName];
  if (!family) {
    const available = Object.keys(manifest.data.families).join(", ") || "<none>";
    throw new Error(
      `Family '${familyName}' not found in manifest ${manifest.path}. Available families: ${available}`
    );
  }

  if (!family.roots || !Array.isArray(family.roots) || family.roots.length === 0) {
    throw new Error(
      `Family '${familyName}' in manifest ${manifest.path} must define a non-empty 'roots' array`
    );
  }

  return family;
}

/**
 * Very small semver parser and comparator for x.y.z[-pre] style versions.
 * We assume StorageHub uses plain x.y.z without build metadata.
 */
function parseSemver(version: string): ParsedSemver {
  if (typeof version !== "string") {
    throw new Error(`Invalid version (not a string): ${String(version)}`);
  }

  // Strip build metadata if present (x.y.z+build)
  const [mainAndPre] = version.split("+");
  const [main, pre] = mainAndPre.split("-");
  const parts = main.split(".");

  if (parts.length !== 3) {
    throw new Error(`Version "${version}" is not in x.y.z form`);
  }

  const [major, minor, patch] = parts.map((p) => {
    const n = Number(p);
    if (!Number.isInteger(n) || n < 0) {
      throw new Error(`Invalid numeric segment '${p}' in version "${version}"`);
    }
    return n;
  });

  return {
    major,
    minor,
    patch,
    pre: pre || null
  };
}

/**
 * Compare two semver strings.
 * Returns:
 *   -1 if a < b
 *    0 if a == b
 *    1 if a > b
 */
function compareSemver(a: string, b: string): number {
  const va = parseSemver(a);
  const vb = parseSemver(b);

  if (va.major !== vb.major) {
    return va.major < vb.major ? -1 : 1;
  }
  if (va.minor !== vb.minor) {
    return va.minor < vb.minor ? -1 : 1;
  }
  if (va.patch !== vb.patch) {
    return va.patch < vb.patch ? -1 : 1;
  }

  // At this point, numeric parts are equal.
  if (va.pre === vb.pre) {
    return 0;
  }

  // Pre-release is considered lower than the corresponding non-pre-release.
  if (va.pre === null && vb.pre !== null) {
    return 1;
  }
  if (va.pre !== null && vb.pre === null) {
    return -1;
  }

  // Both have some pre-release; treat them as equal for our purposes.
  return 0;
}

function findCargoTomlFiles(rootDir: string, roots: string[]): string[] {
  const results: string[] = [];

  /**
   * Recursively walk a directory and collect Cargo.toml paths.
   */
  function walk(currentDir: string): void {
    const entries = fs.readdirSync(currentDir, { withFileTypes: true });
    for (const entry of entries) {
      const entryPath = path.join(currentDir, entry.name);

      if (entry.isDirectory()) {
        const base = entry.name;
        // Skip common heavy or irrelevant directories.
        if (base === "target" || base === "node_modules" || base === ".git") {
          // eslint-disable-next-line no-continue
          continue;
        }
        walk(entryPath);
      } else if (entry.isFile() && entry.name === "Cargo.toml") {
        results.push(entryPath);
      }
    }
  }

  for (const relRoot of roots) {
    const absRoot = path.resolve(rootDir, relRoot);
    if (fs.existsSync(absRoot) && fs.statSync(absRoot).isDirectory()) {
      walk(absRoot);
    } else if (fs.existsSync(absRoot) && fs.statSync(absRoot).isFile()) {
      // Allow specifying a direct file path in roots as well.
      if (path.basename(absRoot) === "Cargo.toml") {
        results.push(absRoot);
      }
    }
  }

  // De-duplicate and sort for stable output.
  return Array.from(new Set(results)).sort();
}

/**
 * Extract the [package] version line and value. We do a minimal parse:
 *   - Find the [package] section
 *   - Find the first 'version = "..."' line after it, before the next '[' section header
 */
function extractPackageVersion(tomlContent: string): PackageVersionInfo | null {
  const packageSectionIndex = tomlContent.indexOf("[package]");
  if (packageSectionIndex === -1) {
    return null;
  }

  const afterPackage = tomlContent.slice(packageSectionIndex);
  // Stop at the next section start or end of file.
  const nextSectionMatch = afterPackage.slice("[package]".length).match(/\n\s*\[[^\]]+\]/);
  const sectionEndOffset =
    nextSectionMatch && typeof nextSectionMatch.index === "number"
      ? nextSectionMatch.index + "[package]".length
      : afterPackage.length;

  const packageBlock = afterPackage.slice(0, sectionEndOffset);
  const versionRegex = /^version\s*=\s*"([^"]+)"/m;
  const match = packageBlock.match(versionRegex);

  if (!match || match.index === undefined) {
    return null;
  }

  return {
    value: match[1],
    // We also keep the absolute index so we can replace in-place.
    indexInFile: packageSectionIndex + match.index,
    length: match[0].length
  };
}

function updateCargoTomlVersion(
  tomlContent: string,
  versionInfo: PackageVersionInfo,
  newVersion: string
): string {
  const before = tomlContent.slice(0, versionInfo.indexInFile);
  const after = tomlContent.slice(versionInfo.indexInFile + versionInfo.length);

  const newLine = `version = "${newVersion}"`;
  return `${before}${newLine}${after}`;
}

function main(): void {
  const args = parseArgs(process.argv);
  if (args.help) {
    printHelp();
    return;
  }

  const repoRoot = path.resolve(__dirname, "..");
  const manifest = loadManifest(repoRoot);
  const familyConfig = getFamilyConfig(manifest, args.family);

  const targetVersion = args.versionOverride || familyConfig.version;
  if (!targetVersion) {
    throw new Error(
      `No target version specified. Either set 'version' for family '${args.family}' in ${manifest.path} or pass --version.`
    );
  }

  // Validate target version early.
  parseSemver(targetVersion);

  console.log(
    `Bumping Rust crate versions for family '${args.family}' to at least ${targetVersion} ` +
      `(dryRun=${args.dryRun ? "true" : "false"})`
  );

  const cargoFiles = findCargoTomlFiles(repoRoot, familyConfig.roots);
  if (cargoFiles.length === 0) {
    console.warn("No Cargo.toml files found for the configured roots.");
    return;
  }

  let updatedCount = 0;
  let skippedCount = 0;

  for (const cargoPath of cargoFiles) {
    const content = fs.readFileSync(cargoPath, "utf8");
    const versionInfo = extractPackageVersion(content);
    if (!versionInfo) {
      // Probably a workspace-only Cargo.toml; skip it quietly.
      skippedCount += 1;
      // eslint-disable-next-line no-continue
      continue;
    }

    const currentVersion = versionInfo.value;
    let cmp: number;
    try {
      cmp = compareSemver(currentVersion, targetVersion);
    } catch (e) {
      const err = e as Error;
      console.error(`Error parsing version in ${cargoPath}: ${err.message}`);
      // Keep going but ensure a non-zero exit code.
      process.exitCode = 1;
      // eslint-disable-next-line no-continue
      continue;
    }

    if (cmp >= 0) {
      skippedCount += 1;
      // eslint-disable-next-line no-continue
      continue;
    }

    // Perform the bump.
    const newContent = updateCargoTomlVersion(content, versionInfo, targetVersion);
    console.log(
      `${args.dryRun ? "[DRY-RUN] " : ""}Updated ${cargoPath}: ${currentVersion} -> ${targetVersion}`
    );

    if (!args.dryRun) {
      fs.writeFileSync(cargoPath, newContent, "utf8");
    }

    updatedCount += 1;
  }

  console.log("");
  console.log(`Summary for family '${args.family}':`);
  console.log(`  Target version: ${targetVersion}`);
  console.log(`  Cargo.toml files scanned: ${cargoFiles.length}`);
  console.log(`  Updated: ${updatedCount}`);
  console.log(`  Skipped (already >= target or no [package] version): ${skippedCount}`);

  if (args.dryRun && updatedCount === 0) {
    console.log("No changes would be made with the current configuration and target version.");
  }
}

try {
  main();
} catch (e) {
  const err = e as Error;
  console.error(`Error: ${err.message}`);
  process.exit(1);
}
