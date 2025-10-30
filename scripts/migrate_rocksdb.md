# RocksDB File Storage Migration

This one-off migration backfills new indexes and counters added to the RocksDB-backed file storage, introduced by PR "fix: 🚑 RocksDB MSP fixes when receiving files". See the PR for context: [#520](https://github.com/Moonsong-Labs/storage-hub/pull/520).

## What changed and why migration is needed

The file storage now relies on two additional data structures stored in RocksDB:

- Bucket-prefixed index (`BucketPrefix`): allows efficient deletion of all files in a bucket via a prefix scan keyed by `bucket_id || file_key`.
- Fingerprint refcount (`FingerprintRefCount`): ensures the underlying content trie (chunks/roots) shared across multiple buckets is only deleted when the last reference is removed.

Existing databases created before this change will be missing these entries for already-stored files; without backfill:

- Bucket-wide deletes won’t remove pre-existing files (they’re invisible to the new prefix scan).
- Deleting one of multiple bucket-copies of the same content could prematurely delete shared content (missing refcount).

## What the script does

The migration scans all existing file metadata and performs the following idempotent updates:

- For every file in `Metadata`, writes a `BucketPrefix` entry with key `bucket_id || file_key` and empty value, if missing.
- Counts the number of files per `fingerprint` and writes `FingerprintRefCount[fingerprint] = count` (u64 little-endian), overwriting or creating as needed.

It does not modify stored chunks, partial/final roots, or per-file chunk counts.

## Safety and rolling upgrade

- Perform a rolling migration node-by-node; no chain reset is needed.
- For each MSP/BSP node:
  1. Gracefully stop the node process/container.
  2. Back up the `--storage-layer` path (the DB folder).
  3. Run the migration against that DB path.
  4. Restart the node and let it catch up.
- The script is idempotent and safe to re-run.

## Prerequisites

- The script uses the Node RocksDB binding `@napi-rs/rocksdb`.
- You can run with Bun (recommended) or with pnpm+tsx.

Install dev dependencies (once in repo root):

```bash
# Bun
bun add -D @napi-rs/rocksdb

# Or pnpm
pnpm add -D @napi-rs/rocksdb tsx
```

## Usage

Basic usage (default column family names assumed: `Metadata=default`, `BucketPrefix=col4`, `FingerprintRefCount=col9`):

```bash
# Bun
bun run scripts/migrate_rocksdb.ts --db-path=/absolute/path/to/storage

# pnpm + tsx
pnpm tsx scripts/migrate_rocksdb.ts --db-path=/absolute/path/to/storage
```

Dry run (no writes):

```bash
bun run scripts/migrate_rocksdb.ts --db-path=/absolute/path/to/storage --dry-run
```

Custom column family names (rare; only if your DB uses different names):

```bash
bun run scripts/migrate_rocksdb.ts \
  --db-path=/absolute/path/to/storage \
  --metadata-cf=default \
  --bucketprefix-cf=col4 \
  --refcount-cf=col9
```

## Output

The script prints a concise summary, e.g.:

```
--- Migration summary ---
Scanned metadata entries: 123
BucketPrefix writes:     120
Refcount writes:         80
Skipped malformed:       0
```

If `--dry-run` is used, it will also indicate that no writes were performed.

## Notes

- Run the migration while the node is stopped to avoid concurrent access.
- Always keep a backup of the storage path until you’ve validated the node after restart.
- The operation does not alter file content or roots; it only backfills the indexes/counters required by the new logic.
