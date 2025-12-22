#!/usr/bin/env tsx
/**
 * Block-walking debug script.
 *
 * Usage:
 *   pnpm --dir test debug:block-walk -- <initialBlock> <finalBlock> <wsEndpoint>
 *
 * Or via env:
 *   INITIAL_BLOCK=1 FINAL_BLOCK=10 WS_ENDPOINT=ws://127.0.0.1:9944 pnpm --dir test debug:block-walk
 *
 * Notes:
 * - `@storagehub/api-augment` MUST be the first import to properly augment types.
 * - This script currently only prints basic block info. We’ll add extrinsic/RPC inspection next.
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

  throw new Error(`Expected ${label} to be an array-like (Vec) with toArray(), got: ${String(value)}`);
}

function parsePositiveInt(value: string, name: string): number {
  const n = Number.parseInt(value, 10);
  if (!Number.isFinite(n) || Number.isNaN(n) || n < 0) {
    throw new Error(`Invalid ${name}: "${value}" (expected a non-negative integer)`);
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
    "  pnpm --dir test debug:block-walk -- <initialBlock> <finalBlock> <wsEndpoint> <outputJsonPath>",
    "",
    "  # From ./test",
    "  pnpm debug:block-walk -- <initialBlock> <finalBlock> <wsEndpoint> <outputJsonPath>",
    "",
    "Env alternative:",
    "  INITIAL_BLOCK=... FINAL_BLOCK=... WS_ENDPOINT=ws://... OUTPUT_JSON=./out.json pnpm --dir test debug:block-walk",
    ""
  ].join("\n");
}

function getArgs(): CliArgs {
  const argvInitial = process.argv[2];
  const argvFinal = process.argv[3];
  const argvEndpoint = process.argv[4];
  const argvOutput = process.argv[5];

  const initialRaw = process.env.INITIAL_BLOCK ?? argvInitial;
  const finalRaw = process.env.FINAL_BLOCK ?? argvFinal;
  const endpointRaw = process.env.WS_ENDPOINT ?? process.env.WSS_ENDPOINT ?? argvEndpoint;
  const outputRaw = process.env.OUTPUT_JSON ?? process.env.OUTPUT_PATH ?? argvOutput;

  if (!initialRaw || !finalRaw || !endpointRaw || !outputRaw) {
    throw new Error(`Missing required inputs.\n\n${usage()}`);
  }

  const initialBlock = parsePositiveInt(initialRaw, "initial block");
  const finalBlock = parsePositiveInt(finalRaw, "final block");
  if (finalBlock < initialBlock) {
    throw new Error(
      `Invalid range: finalBlock (${finalBlock}) must be >= initialBlock (${initialBlock})`
    );
  }

  const endpoint = parseEndpoint(endpointRaw);
  const outputPath = resolvePath(outputRaw);

  return { initialBlock, finalBlock, endpoint, outputPath };
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
    throw err;
  }
}

async function writeDump(outputPath: string, entries: DumpEntry[]): Promise<void> {
  const json = `${JSON.stringify(entries, null, 2)}\n`;
  await writeFile(outputPath, json, "utf8");
}

async function main(): Promise<void> {
  const { initialBlock, finalBlock, endpoint, outputPath } = getArgs();

  console.log("=".repeat(80));
  console.log("StorageHub Debug Block Walker");
  console.log("=".repeat(80));
  console.log(`Endpoint: ${endpoint}`);
  console.log(`Range:    [${initialBlock}..${finalBlock}]`);
  console.log(`Output:   ${outputPath}`);
  console.log("=".repeat(80));

  const dumpEntries = await readExistingDump(outputPath);

  const api = await ApiPromise.create({
    provider: new WsProvider(endpoint),
    noInitWarn: true,
    throwOnConnect: false,
    throwOnUnknown: false,
    typesBundle: BundledTypes
  });

  try {
    await api.isReady;

    for (let blockNumber = initialBlock; blockNumber <= finalBlock; blockNumber += 1) {
      const blockHash = await api.rpc.chain.getBlockHash(blockNumber);
      const header = await api.rpc.chain.getHeader(blockHash);
      const signedBlock = await api.rpc.chain.getBlock(blockHash);
      const eventsAt = (await (await api.at(blockHash)).query.system.events()) as unknown as EventRecord[];

      const extrinsicsCount = signedBlock.block.extrinsics.length;

      console.log(
        `#${blockNumber} ${blockHash.toHex()} | parent=${header.parentHash.toHex()} | exts=${extrinsicsCount}`
      );

      for (const record of eventsAt) {
        if (!api.events.fileSystem.BucketFileDeletionsCompleted.is(record.event)) continue;

        // Event signature (per metadata): (user: AccountId20, fileKeys: Vec<H256>, bucketId: H256)
        // We intentionally avoid relying on generated tuple typings here and instead convert via runtime methods.
        const data = record.event.data;

        const dataArr =
          isRecord(data) && typeof data.toArray === "function"
            ? (data.toArray as () => unknown[])()
            : (Array.isArray(data) ? data : null);

        if (!dataArr || dataArr.length < 3) {
          throw new Error(
            `Unexpected event data for fileSystem.BucketFileDeletionsCompleted at block #${blockNumber}`
          );
        }

        const fileKeys = dataArr[1];
        const bucketId = dataArr[2];

        const bucketIdHex = toHexOrThrow(bucketId, "bucketId");
        const fileKeyHexes = vecToHexesOrThrow(fileKeys, "fileKeys");

        console.log(
          `  -> fileSystem.BucketFileDeletionsCompleted bucketId=${bucketIdHex} fileKeys=${fileKeyHexes.length}`
        );
        for (const fk of fileKeyHexes) {
          console.log(`     - ${fk}`);
        }

        dumpEntries.push({ blockNumber, bucketId: bucketIdHex, fileKeys: fileKeyHexes });
        await writeDump(outputPath, dumpEntries);
      }
    }
  } finally {
    await api.disconnect();
  }
}

await main();


