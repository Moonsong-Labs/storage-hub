### Forest RocksDB migration script

This one-off script migrates Forest Storage RocksDB directories from the old layout to the new layout introduced in PR #518.

It does NOT change File Storage, which remains under `{storage_path}/storagehub/file_storage/`.

What changes

- Old Forest layout (before PR #518):
  - Directories next to your `--storage-path`, named like `{storage_path}_[58, 99, ...]` (a bracketed decimal byte array suffix).
  - Forest DB located at `{storage_path}_[...]/storagehub/forest_storage/`.
- New Forest layout (after PR #518):
  - Per-key folders under `{storage_path}/storagehub/forest_storage/0x<hex>`.

This script moves/copies each old forest DB to the new per-key directory. The decimal byte array is decoded to bytes, then converted to a `0x`-prefixed hex dir name.

Safety requirements

- Stop the node process(es) before running the migration to avoid open RocksDB files.
- You can run a dry run first to preview actions.

Script location

- `scripts/migrate_forests.ts`

Usage

- Dry run:

  - bun: `bun run scripts/migrate_forests.ts --storage-path /path/to/storage --mode dry-run`
  - pnpm: `pnpm dlx tsx scripts/migrate_forests.ts --storage-path /path/to/storage --mode dry-run`

- Execute (fast move on same filesystem):

  - bun: `bun run scripts/migrate_forests.ts --storage-path /path/to/storage --mode rename`
  - pnpm: `pnpm dlx tsx scripts/migrate_forests.ts --storage-path /path/to/storage --mode rename`

- Execute (keep backup; slower):
  - bun: `bun run scripts/migrate_forests.ts --storage-path /path/to/storage --mode copy`
  - pnpm: `pnpm dlx tsx scripts/migrate_forests.ts --storage-path /path/to/storage --mode copy`

Arguments

- `--storage-path <path>`: the value you pass to your node via `--storage-path`.
- `--mode <dry-run|rename|copy>`: default `rename`.
  - `dry-run`: no changes, only logs planned moves.
  - `rename`: move directories; if cross-device, falls back to copy+delete.
  - `copy`: copy directories and keep the originals.

What it migrates

- From: `{parent_of_storage_path}/{basename(storage_path)}_[<decimal byte array>]/storagehub/forest_storage/`
- To: `{storage_path}/storagehub/forest_storage/0x<hex>`

Notes

- If the destination already exists, that key is skipped.
- After a successful move/copy, the script tries to remove the empty old parent directory.
- This migration covers both BSP (e.g., key for `":current_forest_key"` stored as `[58, 99, ...]`) and MSP bucket IDs (also stored as decimal arrays).

Reference

- Change rationale and new layout: PR #518
