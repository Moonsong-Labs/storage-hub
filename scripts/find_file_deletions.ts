/**
 * Find `fileSystem.BucketFileDeletionsCompleted` events over a block range and dump them to JSON.
 *
 * This script scans blocks from `INITIAL_BLOCK` to `FINAL_BLOCK` (inclusive), checks if the
 * `fileSystem.BucketFileDeletionsCompleted` event exists in each block, and if so, records:
 *   - blockNumber
 *   - bucketId
 *   - fileKeys[]
 *
 * The output file is a JSON array. If the file already exists, new entries are appended to the
 * existing array (the file is re-written with the combined array; it is not deleted).
 *
 * How to run (from the repository root):
 *   - Basic usage (positional args):
 *       pnpm --dir scripts find:file-deletions \
 *         <initialBlock> <finalBlock> <wsEndpoint> <outputJsonPath>
 *
 *   - Using env vars:
 *       INITIAL_BLOCK=738513 FINAL_BLOCK=738900 \
 *       WS_ENDPOINT=wss://services.datahaven-testnet.network/testnet \
 *       OUTPUT_JSON=./bucket_file_deletions.json \
 *       pnpm --dir scripts find:file-deletions
 *
 * Environment:
 *   - INITIAL_BLOCK: start block number (non-negative integer)
 *   - FINAL_BLOCK: end block number (non-negative integer)
 *   - WS_ENDPOINT / WSS_ENDPOINT: websocket endpoint (ws://... or wss://...)
 *   - OUTPUT_JSON / OUTPUT_PATH: path to output JSON file
 *   - CONCURRENCY: optional, number of parallel workers (default: 8)
 *   - FLUSH_EVERY_BLOCKS: optional, how often to flush JSON/progress (default: 250)
 *
 * Notes:
 * - `@storagehub/api-augment` MUST be the first import to properly augment types.
 * - We only fetch block hash + events (no full block body) because we only care about events.
 */
import "@storagehub/api-augment"; // must be first import

import { readFile, writeFile } from "node:fs/promises";
import { resolve as resolvePath } from "node:path";
import { ApiPromise, WsProvider } from "@polkadot/api";
import type { EventRecord } from "@polkadot/types/interfaces";
import { types as BundledTypes } from "@storagehub/types-bundle";

type CliArgs = {
  initialBlock: number;
  finalBlock: number;
  endpoint: `ws://${string}` | `wss://${string}`;
  outputPath: string;
  concurrency: number;
  flushEveryBlocks: number;
};

type DumpEntry = {
  blockNumber: number;
  bucketId: string;
  fileKeys: string[];
};

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function hasToHex(value: unknown): value is { toHex: () => string } {
  return isRecord(value) && typeof value.toHex === "function";
}

function toHexOrThrow(value: unknown, label: string): string {
  if (!hasToHex(value)) {
    throw new Error(`Expected ${label} to have a toHex() method, got: ${String(value)}`);
  }
  return value.toHex();
}

function vecToHexesOrThrow(value: unknown, label: string): string[] {
  if (Array.isArray(value)) {
    return value.map((v, i) => toHexOrThrow(v, `${label}[${i}]`));
  }

  if (isRecord(value) && typeof value.toArray === "function") {
    const arr = (value.toArray as () => unknown[])();
    return arr.map((v, i) => toHexOrThrow(v, `${label}[${i}]`));
  }

  throw new Error(
    `Expected ${label} to be an array-like (Vec) with toArray(), got: ${String(value)}`
  );
}

function parseNonNegativeInt(value: string, name: string): number {
  const n = Number.parseInt(value, 10);
  if (!Number.isFinite(n) || Number.isNaN(n) || n < 0) {
    throw new Error(`Invalid ${name}: "${value}" (expected a non-negative integer)`);
  }
  return n;
}

function parsePositiveInt(value: string, name: string): number {
  const n = Number.parseInt(value, 10);
  if (!Number.isFinite(n) || Number.isNaN(n) || n <= 0) {
    throw new Error(`Invalid ${name}: "${value}" (expected a positive integer)`);
  }
  return n;
}

function parseEndpoint(value: string): `ws://${string}` | `wss://${string}` {
  if (value.startsWith("ws://")) return value as `ws://${string}`;
  if (value.startsWith("wss://")) return value as `wss://${string}`;
  throw new Error(`Invalid endpoint: "${value}" (expected ws://... or wss://...)`);
}

function usage(): string {
  return [
    "Usage:",
    "  # From repo root",
    "  pnpm --dir scripts find:file-deletions <initialBlock> <finalBlock> <wsEndpoint> <outputJsonPath>",
    "",
    "  # From ./scripts",
    "  pnpm find:file-deletions <initialBlock> <finalBlock> <wsEndpoint> <outputJsonPath>",
    "",
    "Env alternative:",
    "  INITIAL_BLOCK=... FINAL_BLOCK=... WS_ENDPOINT=ws://... OUTPUT_JSON=./out.json pnpm --dir scripts find:file-deletions",
    "",
    "Optional env:",
    "  CONCURRENCY=8",
    "  FLUSH_EVERY_BLOCKS=250",
    ""
  ].join("\n");
}

function getArgs(): CliArgs {
  const argvRaw = process.argv.slice(2);

  if (argvRaw.includes("--help") || argvRaw.includes("-h")) {
    console.log(usage());
    process.exit(0);
  }

  const argv = argvRaw[0] === "--" ? argvRaw.slice(1) : argvRaw;

  const argvInitial = argv[0];
  const argvFinal = argv[1];
  const argvEndpoint = argv[2];
  const argvOutput = argv[3];

  const initialRaw = process.env.INITIAL_BLOCK ?? argvInitial;
  const finalRaw = process.env.FINAL_BLOCK ?? argvFinal;
  const endpointRaw = process.env.WS_ENDPOINT ?? process.env.WSS_ENDPOINT ?? argvEndpoint;
  const outputRaw = process.env.OUTPUT_JSON ?? process.env.OUTPUT_PATH ?? argvOutput;

  const concurrencyRaw = process.env.CONCURRENCY;
  const flushEveryBlocksRaw = process.env.FLUSH_EVERY_BLOCKS;

  if (!initialRaw || !finalRaw || !endpointRaw || !outputRaw) {
    throw new Error(`Missing required inputs.\n\n${usage()}`);
  }

  const initialBlock = parseNonNegativeInt(initialRaw, "initial block");
  const finalBlock = parseNonNegativeInt(finalRaw, "final block");
  if (finalBlock < initialBlock) {
    throw new Error(
      `Invalid range: finalBlock (${finalBlock}) must be >= initialBlock (${initialBlock})`
    );
  }

  const endpoint = parseEndpoint(endpointRaw);
  const outputPath = resolvePath(outputRaw);

  const concurrency = concurrencyRaw ? parsePositiveInt(concurrencyRaw, "CONCURRENCY") : 8;
  const flushEveryBlocks = flushEveryBlocksRaw
    ? parsePositiveInt(flushEveryBlocksRaw, "FLUSH_EVERY_BLOCKS")
    : 250;

  return { initialBlock, finalBlock, endpoint, outputPath, concurrency, flushEveryBlocks };
}

async function readExistingDump(outputPath: string): Promise<DumpEntry[]> {
  try {
    const raw = await readFile(outputPath, "utf8");
    const parsed: unknown = JSON.parse(raw);
    if (!Array.isArray(parsed)) {
      throw new Error(`Existing output file is not a JSON array: ${outputPath}`);
    }
    return parsed as unknown as DumpEntry[];
  } catch (err) {
    const e = err as { code?: unknown };
    if (e.code === "ENOENT") return [];
    if (e.code === "EISDIR") {
      throw new Error(
        `Output path "${outputPath}" is a directory. Please provide a file path (e.g. ./bucket_file_deletions.json).`
      );
    }
    throw err;
  }
}

async function writeDump(outputPath: string, entries: DumpEntry[]): Promise<void> {
  const json = `${JSON.stringify(entries, null, 2)}\n`;
  await writeFile(outputPath, json, "utf8");
}

function normalizeEventDataToArray(data: unknown): unknown[] | null {
  if (Array.isArray(data)) return data;
  if (isRecord(data) && typeof data.toArray === "function") {
    return (data.toArray as () => unknown[])();
  }
  return null;
}

async function scanOneBlock(
  api: ApiPromise,
  blockNumber: number
): Promise<{ blockNumber: number; matches: DumpEntry[] }> {
  const blockHash = await api.rpc.chain.getBlockHash(blockNumber);
  const eventsAt = (await (
    await api.at(blockHash)
  ).query.system.events()) as unknown as EventRecord[];

  const matches: DumpEntry[] = [];

  for (const record of eventsAt) {
    if (!api.events.fileSystem.BucketFileDeletionsCompleted.is(record.event)) continue;

    // Event signature (per metadata): (user: AccountId20, fileKeys: Vec<H256>, bucketId: H256)
    const dataArr = normalizeEventDataToArray(record.event.data);
    if (!dataArr || dataArr.length < 3) {
      throw new Error(
        `Unexpected event data for fileSystem.BucketFileDeletionsCompleted at block #${blockNumber}`
      );
    }

    const fileKeys = dataArr[1];
    const bucketId = dataArr[2];

    const bucketIdHex = toHexOrThrow(bucketId, "bucketId");
    const fileKeyHexes = vecToHexesOrThrow(fileKeys, "fileKeys");

    matches.push({ blockNumber, bucketId: bucketIdHex, fileKeys: fileKeyHexes });
  }

  return { blockNumber, matches };
}

async function main(): Promise<void> {
  const { initialBlock, finalBlock, endpoint, outputPath, concurrency, flushEveryBlocks } =
    getArgs();

  console.log("=".repeat(80));
  console.log("StorageHub Debug Block Walker (Parallel)");
  console.log("=".repeat(80));
  console.log(`Endpoint:     ${endpoint}`);
  console.log(`Range:        [${initialBlock}..${finalBlock}]`);
  console.log(`Output:       ${outputPath}`);
  console.log(`Concurrency:  ${concurrency}`);
  console.log(`Flush every:  ${flushEveryBlocks} block(s)`);
  console.log("=".repeat(80));

  const existingEntries = await readExistingDump(outputPath);
  const newEntries: DumpEntry[] = [];
  const totalBlocks = finalBlock - initialBlock + 1;

  // Create/refresh the output file early so you can "tail" it during a long run.
  await writeDump(outputPath, existingEntries);

  const flush = async (): Promise<void> => {
    // Stable ordering for output readability.
    newEntries.sort((a, b) => a.blockNumber - b.blockNumber);
    const combined = [...existingEntries, ...newEntries];
    await writeDump(outputPath, combined);
  };

  let processed = 0;
  let lastFlushedAt = 0;
  const logProgress = () => {
    const pct = totalBlocks > 0 ? ((processed / totalBlocks) * 100).toFixed(2) : "100.00";
    console.log(
      `Progress: ${processed}/${totalBlocks} blocks (${pct}%) | matches(new)=${newEntries.length}`
    );
  };

  const api = await ApiPromise.create({
    provider: new WsProvider(endpoint),
    noInitWarn: true,
    throwOnConnect: false,
    throwOnUnknown: false,
    typesBundle: BundledTypes
  });

  try {
    await api.isReady;

    let nextBlock = initialBlock;
    const lastBlock = finalBlock;
    let aborted = false;
    let firstError: unknown = null;

    const onSigInt = async () => {
      try {
        console.log("\nSIGINT received. Flushing partial results...");
        await flush();
      } finally {
        process.exit(130);
      }
    };
    process.once("SIGINT", onSigInt);

    const worker = async (): Promise<void> => {
      for (;;) {
        if (aborted) return;

        const current = nextBlock;
        if (current > lastBlock) return;
        nextBlock = current + 1;

        try {
          const { matches } = await scanOneBlock(api, current);
          for (const m of matches) {
            newEntries.push(m);
          }
        } catch (err) {
          aborted = true;
          firstError ??= err;
          return;
        } finally {
          processed += 1;

          if (processed - lastFlushedAt >= flushEveryBlocks) {
            lastFlushedAt = processed;
            logProgress();
            await flush();
          }
        }
      }
    };

    const workers = Array.from({ length: concurrency }, () => worker());
    await Promise.all(workers);

    logProgress();
    await flush();

    if (firstError) {
      throw firstError;
    }
  } finally {
    await api.disconnect();
  }

  // Console output (summary + matches)
  console.log(`Found ${newEntries.length} BucketFileDeletionsCompleted event(s) in range.`);
  for (const entry of newEntries) {
    console.log(
      `#${entry.blockNumber} bucketId=${entry.bucketId} fileKeys=${entry.fileKeys.length}`
    );
    for (const fk of entry.fileKeys) {
      console.log(`  - ${fk}`);
    }
  }
}

await main();
