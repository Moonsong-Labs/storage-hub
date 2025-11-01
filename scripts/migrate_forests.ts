/* eslint-disable no-console */
import { promises as fs } from "node:fs";
import path from "node:path";

type Mode = "dry-run" | "rename" | "copy";

function parseArgs(argv: string[]) {
  const args: Record<string, string> = {};
  for (let i = 2; i < argv.length; i++) {
    const a = argv[i];
    if (a.startsWith("--")) {
      const [k, v] = a.split("=", 2);
      if (v !== undefined) args[k.slice(2)] = v;
      else if (i + 1 < argv.length && !argv[i + 1].startsWith("--")) {
        args[k.slice(2)] = argv[++i];
      } else {
        args[k.slice(2)] = "true";
      }
    }
  }
  return args;
}

function parseBracketedDecimalArray(s: string): Uint8Array | null {
  // Expect: [58, 99, 117, ...]
  const trimmed = s.trim();
  if (!trimmed.startsWith("[") || !trimmed.endsWith("]")) return null;
  const inner = trimmed.slice(1, -1).trim();
  if (inner.length === 0) return new Uint8Array();
  const parts = inner.split(",").map((p) => p.trim());
  const bytes = new Uint8Array(parts.length);
  for (let i = 0; i < parts.length; i++) {
    const n = Number(parts[i]);
    if (!Number.isInteger(n) || n < 0 || n > 255) {
      throw new Error(`Invalid byte value "${parts[i]}" in ${s}`);
    }
    bytes[i] = n;
  }
  return bytes;
}

function to0xHex(bytes: Uint8Array): string {
  return `0x${Buffer.from(bytes).toString("hex")}`;
}

async function exists(p: string): Promise<boolean> {
  try {
    await fs.stat(p);
    return true;
  } catch {
    return false;
  }
}

async function isRocksDbDir(dir: string): Promise<boolean> {
  // Heuristic: RocksDB dirs typically have "CURRENT" and/or "MANIFEST-*"
  const hasCurrent = await exists(path.join(dir, "CURRENT"));
  if (hasCurrent) return true;
  try {
    const files = await fs.readdir(dir);
    return files.some((f) => f.startsWith("MANIFEST-"));
  } catch {
    return false;
  }
}

async function copyDir(src: string, dest: string) {
  if (typeof fs.cp === "function") {
    await fs.cp(src, dest, { recursive: true, force: true, errorOnExist: false });
    return;
  }
  await fs.mkdir(dest, { recursive: true });
  const entries = await fs.readdir(src, { withFileTypes: true });
  for (const e of entries) {
    const s = path.join(src, e.name);
    const d = path.join(dest, e.name);
    if (e.isDirectory()) await copyDir(s, d);
    else if (e.isSymbolicLink()) {
      const link = await fs.readlink(s);
      await fs.symlink(link, d);
    } else {
      await fs.copyFile(s, d);
    }
  }
}

async function migrate(storagePath: string, mode: Mode) {
  const absStoragePath = path.resolve(storagePath);
  const parent = path.dirname(absStoragePath);
  const base = path.basename(absStoragePath);

  const siblings = await fs.readdir(parent, { withFileTypes: true });
  for (const entry of siblings) {
    if (!entry.isDirectory()) continue;
    if (!entry.name.startsWith(`${base}_`)) continue;

    const suffix = entry.name.slice(base.length + 1); // after underscore
    let keyBytes: Uint8Array | null = null;

    try {
      keyBytes = parseBracketedDecimalArray(suffix);
      if (!keyBytes) continue; // not an old forest dir we care about
    } catch (e) {
      console.warn(`WARN: Skipping "${entry.name}" — cannot parse suffix: ${(e as Error).message}`);
      continue;
    }

    const srcDir = path.join(parent, entry.name, "storagehub", "forest_storage");
    if (!(await exists(srcDir)) || !(await isRocksDbDir(srcDir))) {
      continue;
    }

    const keyDir = to0xHex(keyBytes);
    const dstDir = path.join(absStoragePath, "storagehub", "forest_storage", keyDir);

    await fs.mkdir(path.dirname(dstDir), { recursive: true });

    if (await exists(dstDir)) {
      console.log(`SKIP (exists): ${dstDir}`);
      continue;
    }

    console.log(`MIGRATE: ${srcDir} -> ${dstDir} (mode=${mode})`);

    if (mode === "dry-run") continue;

    if (mode === "rename") {
      try {
        await fs.rename(srcDir, dstDir);
      } catch (e: any) {
        if (e?.code === "EXDEV") {
          // Cross-device: fallback to copy+remove
          await copyDir(srcDir, dstDir);
          await fs.rm(srcDir, { recursive: true, force: true });
        } else {
          throw e;
        }
      }
    } else if (mode === "copy") {
      await copyDir(srcDir, dstDir);
      // keep old as backup (no delete)
    }

    // Optional: remove empty old parent dir
    const oldParent = path.join(parent, entry.name);
    try {
      const rem = await fs.readdir(oldParent);
      if (rem.length === 0) await fs.rmdir(oldParent);
    } catch {
      /* ignore */
    }
  }

  console.log("Done.");
}

async function main() {
  const args = parseArgs(process.argv);
  const storagePath = args["storage-path"];
  const mode = ((args.mode as Mode) || "rename") as Mode;

  if (!storagePath) {
    console.error(
      "Usage: bun run scripts/migrate_forests.ts --storage-path <path> [--mode dry-run|rename|copy]"
    );
    console.error(
      "   or: pnpm dlx tsx scripts/migrate_forests.ts --storage-path <path> [--mode dry-run|rename|copy]"
    );
    process.exit(1);
  }
  if (!["dry-run", "rename", "copy"].includes(mode)) {
    console.error(`Invalid --mode: ${mode}`);
    process.exit(1);
  }

  console.log("IMPORTANT: Ensure your node process is stopped before migrating.");

  await migrate(storagePath, mode);
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
