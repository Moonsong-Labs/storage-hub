#!/usr/bin/env bun

/**
 * Check that release versions are consistent across:
 *  - Release markdown description
 *  - release/versions.json manifest
 *  - Cargo.toml and package.json files for each family
 *
 * Behaviour:
 *  - Fails if any family version in the release doc disagrees with release/versions.json.
 *  - Fails if any crate/package is BEHIND its family's version.
 *  - WARNs (does not fail) when:
 *      - A file has no version field.
 *      - A file is AHEAD of the family's version.
 *
 * Usage examples:
 *   bun release/check_versions.ts --release-doc release/StorageHub-release0.3.0.md
 *   bun release/check_versions.ts --release-doc path/to/doc.md --families core_rust,sdk
 */

import * as fs from "node:fs";
import * as path from "node:path";

interface CliArgs {
  releaseDocPath: string;
  families: string[] | null;
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

interface ParsedSemver {
  major: number;
  minor: number;
  patch: number;
  pre: string | null;
}

type FileStatusKind = "ok_exact" | "ok_ahead" | "behind" | "missing_version";

interface FileStatus {
  kind: FileStatusKind;
  file: string;
  current?: string;
  target: string;
}

function printHelp(): void {
  console.log(
    [
      "Usage: bun release/check_versions.ts --release-doc <path> [options]",
      "",
      "Options:",
      "  --release-doc <path>  Path to the release markdown file.",
      "  --families <names>    Comma-separated list of families to check (default: all in manifest).",
      "  --help                Show this help message.",
      "",
      "Behaviour:",
      "  - Validates that the release doc versions match release/versions.json.",
      "  - Validates that all Cargo.toml/package.json versions are >= their family's version.",
      "  - Missing or ahead versions WARN; behind versions ERROR."
    ].join("\n")
  );
}

function parseArgs(argv: string[]): CliArgs {
  let releaseDocPath = "";
  let families: string[] | null = null;
  let help = false;

  for (let i = 2; i < argv.length; i += 1) {
    const arg = argv[i];
    if (arg === "--help" || arg === "-h") {
      help = true;
      break;
    }
    if (arg === "--release-doc") {
      releaseDocPath = argv[i + 1];
      i += 1;
    } else if (arg === "--families") {
      families = argv[i + 1]
        .split(",")
        .map((s) => s.trim())
        .filter((s) => s.length > 0);
      i += 1;
    } else {
      console.warn(`Unknown argument: ${arg}`);
    }
  }

  return {
    releaseDocPath,
    families,
    help
  };
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

  function walk(currentDir: string): void {
    const entries = fs.readdirSync(currentDir, { withFileTypes: true });
    for (const entry of entries) {
      const entryPath = path.join(currentDir, entry.name);

      if (entry.isDirectory()) {
        const base = entry.name;
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
      if (path.basename(absRoot) === "Cargo.toml") {
        results.push(absRoot);
      }
    }
  }

  return Array.from(new Set(results)).sort();
}

function findPackageJsonFiles(rootDir: string, roots: string[]): string[] {
  const results: string[] = [];

  function walk(currentDir: string): void {
    const entries = fs.readdirSync(currentDir, { withFileTypes: true });
    for (const entry of entries) {
      const entryPath = path.join(currentDir, entry.name);

      if (entry.isDirectory()) {
        const base = entry.name;
        if (base === "node_modules" || base === ".git" || base === "dist") {
          // eslint-disable-next-line no-continue
          continue;
        }
        walk(entryPath);
      } else if (entry.isFile() && entry.name === "package.json") {
        results.push(entryPath);
      }
    }
  }

  for (const relRoot of roots) {
    const absRoot = path.resolve(rootDir, relRoot);
    if (fs.existsSync(absRoot) && fs.statSync(absRoot).isDirectory()) {
      walk(absRoot);
    } else if (fs.existsSync(absRoot) && fs.statSync(absRoot).isFile()) {
      if (path.basename(absRoot) === "package.json") {
        results.push(absRoot);
      }
    }
  }

  return Array.from(new Set(results)).sort();
}

function extractCargoPackageVersion(tomlContent: string): string | null {
  const packageSectionIndex = tomlContent.indexOf("[package]");
  if (packageSectionIndex === -1) {
    return null;
  }

  const afterPackage = tomlContent.slice(packageSectionIndex);
  const nextSectionMatch = afterPackage.slice("[package]".length).match(/\n\s*\[[^\]]+\]/);
  const sectionEndOffset =
    nextSectionMatch && typeof nextSectionMatch.index === "number"
      ? nextSectionMatch.index + "[package]".length
      : afterPackage.length;

  const packageBlock = afterPackage.slice(0, sectionEndOffset);
  const versionRegex = /^version\s*=\s*"([^"]+)"/m;
  const match = packageBlock.match(versionRegex);

  if (!match) {
    return null;
  }

  return match[1];
}

/**
 * Parse the release markdown and return the versions it declares for each family we care about.
 *
 * Assumes a release doc similar to StorageHub-release0.2.0.md, with lines like:
 *   - Client code: v0.2.0
 *   - SH SDK (npm): v0.3.3 ...
 *   - types-bundle/api-augment (npm): `@storagehub/types-bundle` v0.2.7, `@storagehub/api-augment` v0.2.10
 */
function parseReleaseDocVersions(
  releaseDocPath: string,
  manifest: ManifestWithPath
): Record<string, string> {
  if (!fs.existsSync(releaseDocPath)) {
    throw new Error(`Release doc not found at ${releaseDocPath}`);
  }

  const raw = fs.readFileSync(releaseDocPath, "utf8");

  const familyVersionsFromDoc: Record<string, string> = {};

  // Helper to assign and detect inconsistencies inside the doc itself.
  function setDocVersion(family: string, version: string): void {
    const existing = familyVersionsFromDoc[family];
    if (existing && existing !== version) {
      console.warn(
        `[WARN] Release doc contains multiple versions for family '${family}': '${existing}' vs '${version}'`
      );
    }
    familyVersionsFromDoc[family] = version;
  }

  // core_rust families: Client / Pallets / Runtime / Backend image
  const corePatterns = [
    /^-\s*Client code:\s*v?(\d+\.\d+\.\d+)/m,
    /^-\s*Pallets code:\s*v?(\d+\.\d+\.\d+)/m,
    /^-\s*Runtime code:\s*v?(\d+\.\d+\.\d+)/m,
    /^-\s*SH Backend Docker image:\s*v?(\d+\.\d+\.\d+)/m
  ];
  for (const re of corePatterns) {
    const match = raw.match(re);
    if (match) {
      setDocVersion("core_rust", match[1]);
    }
  }

  // sdk family: SH SDK (npm): vX.Y.Z
  const sdkMatch = raw.match(/^-+\s*SH SDK \(npm\):\s*v?(\d+\.\d+\.\d+)/m);
  if (sdkMatch) {
    setDocVersion("sdk", sdkMatch[1]);
  }

  // types_bundle and api_augment from the combined line.
  // Example:
  // - types-bundle/api-augment (npm): `@storagehub/types-bundle` v0.2.7, `@storagehub/api-augment` v0.2.10
  const taLineMatch = raw.match(/^-\s*types-bundle\/api-augment \(npm\):(?<rest>.*)$/m);
  if (taLineMatch?.groups?.rest) {
    const rest = taLineMatch.groups.rest;

    const typesBundleMatch = rest.match(/`@storagehub\/types-bundle`\s*v?(\d+\.\d+\.\d+)/);
    if (typesBundleMatch) {
      setDocVersion("types_bundle", typesBundleMatch[1]);
    }

    const apiAugmentMatch = rest.match(/`@storagehub\/api-augment`\s*v?(\d+\.\d+\.\d+)/);
    if (apiAugmentMatch) {
      setDocVersion("api_augment", apiAugmentMatch[1]);
    }
  }

  // Warn if any manifest families have no version in the doc.
  for (const family of Object.keys(manifest.data.families)) {
    if (!familyVersionsFromDoc[family]) {
      console.warn(`[WARN] Release doc does not contain a version entry for family '${family}'`);
    }
  }

  return familyVersionsFromDoc;
}

function checkDocAgainstManifest(
  docVersions: Record<string, string>,
  manifest: ManifestWithPath,
  familiesToCheck: string[]
): boolean {
  let ok = true;

  console.log("");
  console.log("=== Release doc vs manifest ===");

  for (const family of familiesToCheck) {
    const manifestFamily = manifest.data.families[family];
    if (!manifestFamily) {
      console.warn(`[WARN] Family '${family}' present in selection but missing from manifest.`);
      continue;
    }

    const manifestVersion = manifestFamily.version;
    const docVersion = docVersions[family];

    if (!manifestVersion) {
      console.warn(`[WARN] Family '${family}' has no 'version' in manifest.`);
      continue;
    }

    if (!docVersion) {
      console.warn(
        `[WARN] No version found in release doc for family '${family}' (manifest: ${manifestVersion}).`
      );
      // Don't fail purely for missing doc version, but call it out.
      continue;
    }

    try {
      const cmp = compareSemver(docVersion, manifestVersion);
      if (cmp < 0) {
        console.error(
          `[ERROR] Version mismatch for family '${family}': doc=${docVersion} (behind), manifest=${manifestVersion}`
        );
        ok = false;
      } else if (cmp > 0) {
        console.warn(
          `[WARN]  Version mismatch for family '${family}': doc=${docVersion} (ahead), manifest=${manifestVersion}`
        );
      } else {
        console.log(`[OK]    Family '${family}': ${docVersion}`);
      }
    } catch (e) {
      const err = e as Error;
      console.error(
        `[ERROR] Failed to compare versions for family '${family}': doc='${docVersion}', manifest='${manifestVersion}': ${err.message}`
      );
      ok = false;
    }
  }

  return ok;
}

function checkFamilyInCode(
  repoRoot: string,
  manifest: ManifestWithPath,
  familyName: string
): FileStatus[] {
  const familyConfig = getFamilyConfig(manifest, familyName);
  const targetVersion = familyConfig.version;

  if (!targetVersion) {
    console.warn(
      `[WARN] Family '${familyName}' has no 'version' in manifest; skipping code checks for this family.`
    );
    return [];
  }

  const results: FileStatus[] = [];

  if (familyName === "core_rust") {
    const cargoFiles = findCargoTomlFiles(repoRoot, familyConfig.roots);
    for (const cargoPath of cargoFiles) {
      const content = fs.readFileSync(cargoPath, "utf8");
      const currentVersion = extractCargoPackageVersion(content);
      if (!currentVersion) {
        results.push({
          kind: "missing_version",
          file: cargoPath,
          target: targetVersion
        });
        continue;
      }

      let cmp: number;
      try {
        cmp = compareSemver(currentVersion, targetVersion);
      } catch (e) {
        const err = e as Error;
        console.error(`Error parsing version in ${cargoPath}: ${err.message}`);
        results.push({
          kind: "behind",
          file: cargoPath,
          current: currentVersion,
          target: targetVersion
        });
        continue;
      }

      if (cmp < 0) {
        results.push({
          kind: "behind",
          file: cargoPath,
          current: currentVersion,
          target: targetVersion
        });
      } else if (cmp === 0) {
        results.push({
          kind: "ok_exact",
          file: cargoPath,
          current: currentVersion,
          target: targetVersion
        });
      } else {
        results.push({
          kind: "ok_ahead",
          file: cargoPath,
          current: currentVersion,
          target: targetVersion
        });
      }
    }
  } else {
    const packageFiles = findPackageJsonFiles(repoRoot, familyConfig.roots);

    for (const pkgPath of packageFiles) {
      const raw = fs.readFileSync(pkgPath, "utf8");
      let pkg: any;
      try {
        pkg = JSON.parse(raw);
      } catch (e) {
        const err = e as Error;
        console.error(`Failed to parse JSON in ${pkgPath}: ${err.message}`);
        results.push({
          kind: "behind",
          file: pkgPath,
          target: targetVersion
        });
        continue;
      }

      const currentVersion: unknown = pkg.version;
      if (typeof currentVersion !== "string") {
        results.push({
          kind: "missing_version",
          file: pkgPath,
          target: targetVersion
        });
        continue;
      }

      let cmp: number;
      try {
        cmp = compareSemver(currentVersion, targetVersion);
      } catch (e) {
        const err = e as Error;
        console.error(`Error parsing version in ${pkgPath}: ${err.message}`);
        results.push({
          kind: "behind",
          file: pkgPath,
          current: currentVersion,
          target: targetVersion
        });
        continue;
      }

      if (cmp < 0) {
        results.push({
          kind: "behind",
          file: pkgPath,
          current: currentVersion,
          target: targetVersion
        });
      } else if (cmp === 0) {
        results.push({
          kind: "ok_exact",
          file: pkgPath,
          current: currentVersion,
          target: targetVersion
        });
      } else {
        results.push({
          kind: "ok_ahead",
          file: pkgPath,
          current: currentVersion,
          target: targetVersion
        });
      }
    }
  }

  return results;
}

function summariseFileStatuses(familyName: string, statuses: FileStatus[]): { hasBehind: boolean } {
  const behind = statuses.filter((s) => s.kind === "behind");
  const missing = statuses.filter((s) => s.kind === "missing_version");
  const ahead = statuses.filter((s) => s.kind === "ok_ahead");
  const exact = statuses.filter((s) => s.kind === "ok_exact");

  console.log("");
  console.log(`=== Family '${familyName}' in code ===`);
  console.log(`  Exact match: ${exact.length}`);
  console.log(`  Ahead:       ${ahead.length}`);
  console.log(`  Behind:      ${behind.length}`);
  console.log(`  Missing:     ${missing.length}`);

  for (const s of behind) {
    console.error(`[ERROR] [BEHIND] ${s.file}: ${s.current ?? "unknown"} -> target ${s.target}`);
  }

  for (const s of missing) {
    console.warn(`[WARN]  [MISSING VERSION] ${s.file}: target ${s.target}`);
  }

  for (const s of ahead) {
    console.warn(`[WARN]  [AHEAD] ${s.file}: ${s.current ?? "unknown"} (target ${s.target})`);
  }

  return { hasBehind: behind.length > 0 };
}

function main(): void {
  const args = parseArgs(process.argv);
  if (args.help) {
    printHelp();
    return;
  }

  if (!args.releaseDocPath) {
    throw new Error("Missing required --release-doc <path> argument.");
  }

  const repoRoot = path.resolve(__dirname, "..");
  const manifest = loadManifest(repoRoot);
  const allFamilies = Object.keys(manifest.data.families);
  const familiesToCheck = args.families && args.families.length > 0 ? args.families : allFamilies;

  console.log("==============================================");
  console.log(" version check");
  console.log(` Release doc: ${args.releaseDocPath}`);
  console.log(` Families:    ${familiesToCheck.join(", ")}`);
  console.log("==============================================");

  const docVersions = parseReleaseDocVersions(args.releaseDocPath, manifest);

  const docOk = checkDocAgainstManifest(docVersions, manifest, familiesToCheck);

  let hasBehindAny = false;

  for (const family of familiesToCheck) {
    const statuses = checkFamilyInCode(repoRoot, manifest, family);
    const { hasBehind } = summariseFileStatuses(family, statuses);
    if (hasBehind) {
      hasBehindAny = true;
    }
  }

  if (!docOk || hasBehindAny) {
    process.exitCode = 1;
  } else {
    console.log("");
    console.log("All versions are consistent across release doc, manifest, and code.");
  }
}

try {
  main();
} catch (e) {
  const err = e as Error;
  console.error(`Error: ${err.message}`);
  process.exit(1);
}
