/*
  One-off RocksDB migration for StorageHub file storage.

  Purpose:
  - Backfill BucketPrefix (bucket_id || file_key) index for efficient bucket deletes
  - Backfill FingerprintRefCount (u64 LE) so shared content across buckets is reference-counted

  Assumptions:
  - DB was created by kvdb_rocksdb with column indices matching:
      default -> Metadata (Column 0)
      col1    -> Roots
      col2    -> Chunks
      col3    -> ChunkCount
      col4    -> BucketPrefix
      col5    -> ExcludeFile
      col6    -> ExcludeUser
      col7    -> ExcludeBucket
      col8    -> ExcludeFingerprint
      col9    -> FingerprintRefCount
  - Values in Metadata are serde_json of FileMetadata
  - Keys are raw 32-byte file_key values

  Usage (recommended):
    bun run scripts/migrate_rocksdb.ts --db-path=/absolute/path/to/storage

  Or with pnpm (requires @napi-rs/rocksdb installed in this repo):
    pnpm tsx scripts/migrate_rocksdb.ts --db-path=/absolute/path/to/storage
*/

import { Buffer } from "node:buffer";
import { argv, exit } from "node:process";

type ColumnFamilyName =
  | "default"
  | "col1"
  | "col2"
  | "col3"
  | "col4"
  | "col5"
  | "col6"
  | "col7"
  | "col8"
  | "col9"
  | string;

type FileMetadataJson = {
  // owner, bucket_id, location, size, fingerprint per Rust struct
  // We only need bucket_id and fingerprint here
  bucket_id: number[];
  fingerprint: number[];
};

function parseArgs(): {
  dbPath: string;
  dryRun: boolean;
  cfMeta: ColumnFamilyName;
  cfBucket: ColumnFamilyName;
  cfRef: ColumnFamilyName;
} {
  const args = new Map<string, string | boolean>();
  for (let i = 2; i < argv.length; i++) {
    const a = argv[i];
    if (a.startsWith("--")) {
      const [k, v] = a.split("=");
      if (typeof v === "undefined") {
        // flag style
        args.set(k, true);
      } else {
        args.set(k, v);
      }
    }
  }
  const dbPath = (args.get("--db-path") as string) ?? "";
  const dryRun = Boolean(args.get("--dry-run"));
  const cfMeta = ((args.get("--metadata-cf") as string) ?? "default") as ColumnFamilyName;
  const cfBucket = ((args.get("--bucketprefix-cf") as string) ?? "col4") as ColumnFamilyName;
  const cfRef = ((args.get("--refcount-cf") as string) ?? "col9") as ColumnFamilyName;
  if (!dbPath) {
    console.error("Missing required argument: --db-path=/absolute/path/to/storage");
    exit(2);
  }
  return { dbPath, dryRun, cfMeta, cfBucket, cfRef };
}

async function openDb(dbPath: string) {
  const { RocksDB } = await import("@napi-rs/rocksdb");
  const db = new RocksDB(dbPath);
  // Open with expected CFs; create if missing to ensure we can write new ones
  await db.open({
    createIfMissing: true,
    createMissingColumnFamilies: true,
    columnFamilies: [
      { name: "default" },
      { name: "col1" },
      { name: "col2" },
      { name: "col3" },
      { name: "col4" },
      { name: "col5" },
      { name: "col6" },
      { name: "col7" },
      { name: "col8" },
      { name: "col9" }
    ]
  });
  return db;
}

function toBufferFromNumArray(arr: number[], expectedLen: number): Buffer | null {
  if (!Array.isArray(arr) || arr.some((x) => typeof x !== "number")) return null;
  if (expectedLen > 0 && arr.length !== expectedLen) return null;
  const b = Buffer.from(Uint8Array.from(arr));
  if (expectedLen > 0 && b.length !== expectedLen) return null;
  return b;
}

function u64le(n: bigint): Buffer {
  const b = Buffer.allocUnsafe(8);
  b.writeBigUInt64LE(n, 0);
  return b;
}

async function migrate({
  dbPath,
  dryRun,
  cfMeta,
  cfBucket,
  cfRef
}: {
  dbPath: string;
  dryRun: boolean;
  cfMeta: ColumnFamilyName;
  cfBucket: ColumnFamilyName;
  cfRef: ColumnFamilyName;
}) {
  console.log(`Opening RocksDB at ${dbPath}`);
  const db = await openDb(dbPath);

  const metadataCF = db.getColumnFamily(cfMeta);
  const bucketPrefixCF = db.getColumnFamily(cfBucket);
  const fingerprintRefCountCF = db.getColumnFamily(cfRef);

  if (!metadataCF || !bucketPrefixCF || !fingerprintRefCountCF) {
    console.error(
      `Failed to resolve required column families (${cfMeta}, ${cfBucket}, ${cfRef}). Aborting.`
    );
    await db.close();
    exit(1);
  }

  const fingerprintCounts = new Map<string, { buf: Buffer; count: number }>();
  let scanned = 0;
  let bucketPrefixWrites = 0;
  let refcountWrites = 0;
  let skippedMalformed = 0;

  // Scan Metadata CF
  console.log("Scanning Metadata (default) column family...");
  for await (const [key, value] of db.iterator({ columnFamily: metadataCF })) {
    scanned++;
    // Keys must be 32 bytes
    if (!(key instanceof Buffer) || key.length !== 32) {
      skippedMalformed++;
      continue;
    }
    if (!(value instanceof Buffer)) {
      skippedMalformed++;
      continue;
    }

    // Parse FileMetadata JSON
    let meta: FileMetadataJson | null = null;
    try {
      meta = JSON.parse(value.toString("utf8")) as FileMetadataJson;
    } catch {
      skippedMalformed++;
      continue;
    }
    if (!meta || !Array.isArray(meta.bucket_id) || !Array.isArray(meta.fingerprint)) {
      skippedMalformed++;
      continue;
    }

    const bucketId = toBufferFromNumArray(meta.bucket_id, 32);
    const fingerprint = toBufferFromNumArray(meta.fingerprint, 32);
    if (!bucketId || !fingerprint) {
      skippedMalformed++;
      continue;
    }

    // Backfill BucketPrefix: key = bucket_id || file_key, value = empty
    const bucketPrefKey = Buffer.concat([bucketId, key]);
    const existing = await db.get(bucketPrefKey, { columnFamily: bucketPrefixCF });
    if (!existing) {
      if (!dryRun) {
        await db.put(bucketPrefKey, Buffer.alloc(0), { columnFamily: bucketPrefixCF });
      }
      bucketPrefixWrites++;
    }

    // Count fingerprint references
    const fHex = fingerprint.toString("hex");
    const entry = fingerprintCounts.get(fHex);
    if (entry) {
      entry.count += 1;
    } else {
      fingerprintCounts.set(fHex, { buf: fingerprint, count: 1 });
    }
  }

  // Write FingerprintRefCount
  console.log(`Writing FingerprintRefCount (${cfRef})...`);
  for (const { buf, count } of fingerprintCounts.values()) {
    const existing = await db.get(buf, { columnFamily: fingerprintRefCountCF });
    const desired = u64le(BigInt(count));
    if (existing?.equals(desired)) continue;
    if (!dryRun) {
      await db.put(buf, desired, { columnFamily: fingerprintRefCountCF });
    }
    refcountWrites++;
  }

  await db.close();

  console.log("--- Migration summary ---");
  console.log(`Scanned metadata entries: ${scanned}`);
  console.log(`BucketPrefix writes:     ${bucketPrefixWrites}`);
  console.log(`Refcount writes:         ${refcountWrites}`);
  console.log(`Skipped malformed:       ${skippedMalformed}`);
  console.log(dryRun ? "(dry-run: no writes performed)" : "");
}

const { dbPath, dryRun, cfMeta, cfBucket, cfRef } = parseArgs();
// eslint-disable-next-line unicorn/prefer-top-level-await
migrate({ dbPath, dryRun, cfMeta, cfBucket, cfRef }).catch((e) => {
  console.error("Migration failed:", e);
  exit(1);
});
