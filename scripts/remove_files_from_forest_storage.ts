/**
 * Remove files from forest storage for a list of buckets, using the
 * `storagehubclient_removeFilesFromForestStorage` RPC exposed by the node.
 *
 * How to run (from the repository root):
 *   - Basic usage (JSON file is mandatory):
 *       pnpm --dir scripts remove:files-from-forest-storage \
 *         --file=/path/to/bucket_file_deletions.json
 *
 *   - With custom RPC URL and higher concurrency:
 *       pnpm --dir scripts remove:files-from-forest-storage \
 *         --file=/path/to/bucket_file_deletions.json \
 *         --rpc-url=http://127.0.0.1:9933 \
 *         --concurrency=16
 *
 *   - Dry run (no RPC calls, just logs):
 *       pnpm --dir scripts remove:files-from-forest-storage \
 *         --file=/path/to/bucket_file_deletions.json \
 *         --dry-run
 *
 * Environment:
 *   - NODE_RPC_URL: optional, overrides the default RPC URL.
 *
 * Options:
 *   - --file=PATH        Path to the JSON file with bucket/file deletions.
 *   - --rpc-url=URL      Node HTTP JSON-RPC endpoint (e.g. http://127.0.0.1:9933).
 *   - --concurrency=N    Number of buckets to process in parallel (default: 8).
 *   - --dry-run          Log planned operations without calling the RPC.
 */

import { readFile } from "node:fs/promises";
import { resolve } from "node:path";

type BucketDeletionEntry = {
  blockNumber: number;
  bucketId: string;
  fileKeys: string[];
};

type RemoveFilesFromForestStorageResult = "ForestNotFound" | "Success" | string;

const DEFAULT_RPC_URL = process.env.NODE_RPC_URL ?? "http://127.0.0.1:9933";
const DEFAULT_CONCURRENCY = 8;

type CliOptions = {
  jsonPath: string;
  rpcUrl: string;
  concurrency: number;
  dryRun: boolean;
};

function parseArgs(argv: string[]): CliOptions {
  if (argv.includes("--help") || argv.includes("-h")) {
    console.log(
      "Usage: pnpm --dir scripts remove:files-from-forest-storage " +
        "--file=/path/to/bucket_file_deletions.json " +
        "[--rpc-url=URL] [--concurrency=N] [--dry-run]"
    );
    process.exit(0);
  }

  let jsonPath: string | undefined;
  let rpcUrl = DEFAULT_RPC_URL;
  let concurrency = DEFAULT_CONCURRENCY;
  let dryRun = false;

  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];

    if (arg.startsWith("--file=")) {
      jsonPath = arg.slice("--file=".length);
    } else if (arg === "--file") {
      const next = argv[i + 1];
      if (!next || next.startsWith("--")) {
        throw new Error(
          "Missing value for --file. Usage: --file=/path/to/file.json or --file /path/to/file.json"
        );
      }
      jsonPath = next;
      i += 1;
    } else if (arg.startsWith("--rpc-url=")) {
      rpcUrl = arg.slice("--rpc-url=".length);
    } else if (arg.startsWith("--concurrency=")) {
      const value = Number.parseInt(arg.slice("--concurrency=".length), 10);
      if (!Number.isNaN(value) && value > 0) {
        concurrency = value;
      }
    } else if (arg === "--dry-run") {
      dryRun = true;
    }
  }

  if (!jsonPath) {
    throw new Error(
      "Missing required --file option.\n" +
        "Usage: pnpm --dir scripts remove:files-from-forest-storage " +
        "--file=/path/to/bucket_file_deletions.json " +
        "[--rpc-url=URL] [--concurrency=N] [--dry-run]"
    );
  }

  return {
    jsonPath,
    rpcUrl,
    concurrency,
    dryRun
  };
}

async function loadDeletions(jsonPath: string): Promise<BucketDeletionEntry[]> {
  const absolutePath = resolve(jsonPath);
  const raw = await readFile(absolutePath, "utf8");
  const parsed = JSON.parse(raw) as BucketDeletionEntry[];

  return parsed;
}

function groupByBucket(entries: BucketDeletionEntry[]): Map<string, Set<string>> {
  const buckets = new Map<string, Set<string>>();

  for (const entry of entries) {
    const { bucketId, fileKeys } = entry;
    let set = buckets.get(bucketId);
    if (!set) {
      set = new Set<string>();
      buckets.set(bucketId, set);
    }

    for (const key of fileKeys) {
      set.add(key);
    }
  }

  return buckets;
}

async function callRemoveFilesFromForestStorage(
  rpcUrl: string,
  forestKey: string,
  fileKeys: string[],
  requestId: number
): Promise<RemoveFilesFromForestStorageResult> {
  const body = {
    jsonrpc: "2.0",
    id: requestId,
    method: "storagehubclient_removeFilesFromForestStorage",
    // forest_key is an Option<Hash> on the Rust side; passing the hex string
    // here corresponds to `Some(forest_key)`. The node RPC uses the same
    // JSON-RPC endpoint as the chain.
    params: [forestKey, fileKeys]
  };

  const response = await fetch(rpcUrl, {
    method: "POST",
    headers: {
      "content-type": "application/json"
    },
    body: JSON.stringify(body)
  });

  if (!response.ok) {
    throw new Error(`HTTP error from RPC endpoint: ${response.status} ${response.statusText}`);
  }

  const json = (await response.json()) as {
    result?: RemoveFilesFromForestStorageResult;
    error?: { code: number; message: string; data?: unknown };
  };

  if (json.error) {
    throw new Error(
      `JSON-RPC error: code=${json.error.code}, message=${json.error.message}, data=${JSON.stringify(json.error.data)}`
    );
  }

  return json.result ?? "UnknownResult";
}

async function main() {
  const options = parseArgs(process.argv.slice(2));
  const { jsonPath, rpcUrl, concurrency, dryRun } = options;

  console.log(
    `Using JSON file: ${jsonPath}\nRPC endpoint: ${rpcUrl}\nConcurrency: ${concurrency}\nDry run: ${dryRun}`
  );

  const entries = await loadDeletions(jsonPath);
  console.log(`Loaded ${entries.length} deletion entries from JSON file.`);

  const bucketMap = groupByBucket(entries);
  const bucketItems = Array.from(bucketMap.entries()).map(([bucketId, keys]) => ({
    bucketId,
    fileKeys: Array.from(keys)
  }));

  console.log(
    `Grouped into ${bucketItems.length} unique buckets. Total unique file keys: ${bucketItems.reduce((acc, { fileKeys }) => acc + fileKeys.length, 0)}`
  );

  if (dryRun) {
    console.log("Dry run enabled; not sending any RPC calls.");
    for (const { bucketId, fileKeys } of bucketItems) {
      console.log(
        `Would call storagehubclient_removeFilesFromForestStorage for bucket ${bucketId} with ${fileKeys.length} file(s).`
      );
    }
    return;
  }

  let index = 0;
  let successCount = 0;
  let forestNotFoundCount = 0;
  let errorCount = 0;

  async function worker(workerId: number) {
    while (true) {
      const currentIndex = index;
      if (currentIndex >= bucketItems.length) {
        return;
      }
      index += 1;

      const { bucketId, fileKeys } = bucketItems[currentIndex];
      try {
        console.log(
          `[worker ${workerId}] Calling removeFilesFromForestStorage for bucket ${bucketId} with ${fileKeys.length} file(s)...`
        );
        const result = await callRemoveFilesFromForestStorage(
          rpcUrl,
          bucketId,
          fileKeys,
          currentIndex
        );

        if (result === "Success") {
          successCount += 1;
          console.log(
            `[worker ${workerId}] Bucket ${bucketId}: Success (removed ${fileKeys.length} file(s)).`
          );
        } else if (result === "ForestNotFound") {
          forestNotFoundCount += 1;
          console.warn(
            `[worker ${workerId}] Bucket ${bucketId}: ForestNotFound (nothing removed).`
          );
        } else {
          // Unknown / future variant.
          successCount += 1;
          console.log(
            `[worker ${workerId}] Bucket ${bucketId}: Received result "${result}" (treated as success).`
          );
        }
      } catch (error) {
        errorCount += 1;
        console.error(`[worker ${workerId}] Error processing bucket ${bucketId}:`, error);
      }
    }
  }

  const workers: Promise<void>[] = [];
  const workerCount = Math.min(concurrency, bucketItems.length || 1);
  for (let i = 0; i < workerCount; i += 1) {
    workers.push(worker(i + 1));
  }

  await Promise.all(workers);

  console.log(
    `Finished processing buckets. Success: ${successCount}, ForestNotFound: ${forestNotFoundCount}, Errors: ${errorCount}.`
  );
}

// Run the script.
void main().catch((error) => {
  console.error("Fatal error while running bucket deletions script:", error);
  process.exit(1);
});
