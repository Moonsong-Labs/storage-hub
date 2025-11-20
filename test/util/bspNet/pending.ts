import assert from "node:assert";
import { decodeAddress } from "@polkadot/util-crypto";
import { waitFor } from "../index";
import type { SqlClient } from "./types";

export const ACTIVE_STATES = ["future", "ready", "broadcast", "in_block", "retracted"] as const;

export const accountIdFromAddress = (address: string): Buffer => {
  const u8 = decodeAddress(address);
  return Buffer.from(u8);
};

export const getByNonce = async (options: { sql: SqlClient; accountId: Buffer; nonce: bigint }) => {
  const { sql, accountId, nonce } = options;
  const nonceNum = Number(nonce);
  const rows = await sql`
    SELECT account_id, nonce, hash, call_scale, state, creator_id, created_at, updated_at, watched
    FROM pending_transactions
    WHERE account_id = ${accountId} AND nonce = ${nonceNum}
    LIMIT 1
  `;
  return rows[0] ?? null;
};

export const getAllByAccount = async (options: { sql: SqlClient; accountId: Buffer }) => {
  const { sql, accountId } = options;
  const rows = await sql`
    SELECT account_id, nonce, hash, call_scale, state, creator_id, created_at, updated_at, watched
    FROM pending_transactions
    WHERE account_id = ${accountId}
    ORDER BY nonce
  `;
  return rows;
};

export const countActive = async (options: { sql: SqlClient; accountId: Buffer }) => {
  const { sql, accountId } = options;
  const rows = await sql`
    SELECT COUNT(*)::BIGINT AS cnt
    FROM pending_transactions
    WHERE account_id = ${accountId}
      AND state IN ('future','ready','broadcast','in_block','retracted')
  `;
  return BigInt(rows[0]?.cnt ?? 0n);
};

export const waitForState = async (options: {
  sql: SqlClient;
  accountId: Buffer;
  nonce: bigint;
  state: string;
  timeoutMs?: number;
  pollMs?: number;
}) => {
  const { sql, accountId, nonce, state, timeoutMs = 10000, pollMs = 200 } = options;
  const iterations = Math.max(1, Math.ceil(timeoutMs / pollMs));
  await waitFor({
    iterations,
    delay: pollMs,
    lambda: async () => {
      const row = await getByNonce({ sql, accountId, nonce });
      return row?.state === state;
    }
  });
};

export const expectClearedBelow = async (options: {
  sql: SqlClient;
  accountId: Buffer;
  onChainNonce: bigint;
}) => {
  const { sql, accountId, onChainNonce } = options;
  const nonceNum = Number(onChainNonce);
  const rows = await sql`
    SELECT COUNT(*)::BIGINT AS cnt
    FROM pending_transactions
    WHERE account_id = ${accountId}
      AND nonce < ${nonceNum}
      AND state IN ('future','ready','broadcast','in_block','retracted')
  `;
  const cnt = BigInt(rows[0]?.cnt ?? 0n);
  assert.equal(cnt, 0n, `Expected no active pending tx below nonce ${onChainNonce}, found ${cnt}`);
};
