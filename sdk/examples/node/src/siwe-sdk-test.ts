/**
 * SIWE authentication test using SDK methods (`auth.getNonce` + `auth.verify`)
 * with `enableCookies` for sticky session support.
 *
 * Uses the SDK's low-level auth methods so we can track per-account retry
 * counts. The goal is to confirm that `enableCookies: true` eliminates
 * (or drastically reduces) the retries needed to verify a signature.
 *
 * Usage:
 *   MNEMONIC="..." pnpm siwe-sdk-test
 *   MNEMONIC="..." NUM_ACCOUNTS=10 COOKIE_MODE=enabled pnpm siwe-sdk-test
 */

import { MspClient } from "@storagehub-sdk/msp-client";
import type { Session, SessionProvider } from "@storagehub-sdk/msp-client";
import { createWalletClient, http, defineChain } from "viem";
import { mnemonicToAccount } from "viem/accounts";

// ── Stagenet defaults (from storagehub.stagenet.json) ───────────────
const STAGENET_BASE_URL = "https://deo-dh-backend.stagenet.datahaven-infra.network";
const STAGENET_CHAIN_ID = 55932;
const STAGENET_RPC = "https://services.datahaven-dev.network/stagenet";
const STAGENET_SIWE_DOMAIN = "localhost:3001";
const STAGENET_SIWE_URI = "http://localhost:3001";

// ── Config from env ─────────────────────────────────────────────────
const MNEMONIC = process.env.MNEMONIC;
if (!MNEMONIC) {
  console.error("ERROR: MNEMONIC env var is required");
  process.exit(1);
}

const NUM_ACCOUNTS = Number(process.env.NUM_ACCOUNTS || "5");
const MAX_RETRIES = Number(process.env.MAX_RETRIES || "10");
const BACKOFF_MS = Number(process.env.BACKOFF_MS || "100");
const BASE_URL = process.env.BASE_URL || STAGENET_BASE_URL;
const CHAIN_ID = Number(process.env.CHAIN_ID || String(STAGENET_CHAIN_ID));
const RPC_URL = process.env.RPC_URL || STAGENET_RPC;
const SIWE_DOMAIN = process.env.SIWE_DOMAIN || STAGENET_SIWE_DOMAIN;
const SIWE_URI = process.env.SIWE_URI || STAGENET_SIWE_URI;
const COOKIE_MODE = (process.env.COOKIE_MODE || "disabled").toLowerCase() === "enabled";

// ── Types ───────────────────────────────────────────────────────────
interface AccountResult {
  index: number;
  address: string;
  status: "success" | "failure";
  attempts: number;
  elapsedMs: number;
  error?: string;
}

// ── Helpers ─────────────────────────────────────────────────────────
const shortAddr = (addr: string) => `${addr.slice(0, 6)}...${addr.slice(-4)}`;
const delay = (ms: number) => new Promise((r) => setTimeout(r, ms));

const chain = defineChain({
  id: CHAIN_ID,
  name: "DataHaven Stagenet",
  nativeCurrency: { name: "Stage", symbol: "STAGE", decimals: 18 },
  rpcUrls: { default: { http: [RPC_URL] } },
});

async function authenticateAccount(index: number): Promise<AccountResult> {
  const tag = `[Account #${index}]`;
  const t0 = performance.now();

  const account = mnemonicToAccount(MNEMONIC!, { addressIndex: index });
  const address = account.address;
  console.log(`${tag} ${shortAddr(address)} - starting SIWE flow`);

  const wallet = createWalletClient({ chain, account, transport: http(RPC_URL) });

  const sessionProvider: SessionProvider = async () => undefined;
  const client = await MspClient.connect(
    { baseUrl: BASE_URL, timeoutMs: 30_000, enableCookies: COOKIE_MODE },
    sessionProvider
  );

  // Step 1: get nonce
  let message: string;
  try {
    const nonceResp = await client.auth.getNonce(address, CHAIN_ID, SIWE_DOMAIN, SIWE_URI);
    message = nonceResp.message;
    console.log(`${tag} ${shortAddr(address)} - nonce obtained`);
  } catch (err) {
    const elapsed = Math.round(performance.now() - t0);
    const errMsg = err instanceof Error ? err.message : String(err);
    console.error(`${tag} ${shortAddr(address)} - getNonce FAILED: ${errMsg}`);
    return { index, address, status: "failure", attempts: 0, elapsedMs: elapsed, error: `getNonce: ${errMsg}` };
  }

  // Step 2: sign message
  let signature: string;
  try {
    signature = await wallet.signMessage({ account, message });
    console.log(`${tag} ${shortAddr(address)} - message signed`);
  } catch (err) {
    const elapsed = Math.round(performance.now() - t0);
    const errMsg = err instanceof Error ? err.message : String(err);
    console.error(`${tag} ${shortAddr(address)} - signMessage FAILED: ${errMsg}`);
    return { index, address, status: "failure", attempts: 0, elapsedMs: elapsed, error: `sign: ${errMsg}` };
  }

  // Step 3: verify with retries (this is where sticky session issues surface)
  let lastError: unknown;
  for (let attempt = 1; attempt <= MAX_RETRIES; attempt++) {
    try {
      const session: Session = await client.auth.verify(message, signature);
      const elapsed = Math.round(performance.now() - t0);
      console.log(
        `${tag} ${shortAddr(address)} - ✅ verified on attempt ${attempt}/${MAX_RETRIES} (${elapsed}ms) user=${session.user.address}`
      );
      return { index, address, status: "success", attempts: attempt, elapsedMs: elapsed };
    } catch (err) {
      lastError = err;
      const errMsg = err instanceof Error ? err.message : String(err);
      console.warn(`${tag} ${shortAddr(address)} - attempt ${attempt}/${MAX_RETRIES} FAILED: ${errMsg}`);
      if (attempt < MAX_RETRIES) {
        await delay(BACKOFF_MS);
      }
    }
  }

  const elapsed = Math.round(performance.now() - t0);
  const errMsg = lastError instanceof Error ? lastError.message : String(lastError);
  console.error(`${tag} ${shortAddr(address)} - ❌ all ${MAX_RETRIES} attempts exhausted`);
  return { index, address, status: "failure", attempts: MAX_RETRIES, elapsedMs: elapsed, error: errMsg };
}

// ── Main ────────────────────────────────────────────────────────────
async function main() {
  console.log("=".repeat(70));
  console.log("SIWE SDK Authentication Test");
  console.log("=".repeat(70));
  console.log(`Backend:      ${BASE_URL}`);
  console.log(`Chain ID:     ${CHAIN_ID}`);
  console.log(`SIWE domain:  ${SIWE_DOMAIN}`);
  console.log(`SIWE URI:     ${SIWE_URI}`);
  console.log(`Accounts:     ${NUM_ACCOUNTS}`);
  console.log(`Max retries:  ${MAX_RETRIES}`);
  console.log(`Backoff:      ${BACKOFF_MS}ms`);
  console.log(`Cookie mode:  ${COOKIE_MODE ? "enabled" : "disabled"}`);
  console.log("=".repeat(70));
  console.log();

  const tasks = Array.from({ length: NUM_ACCOUNTS }, (_, i) => authenticateAccount(i));
  const results = await Promise.allSettled(tasks);

  const accountResults: AccountResult[] = results.map((r, i) => {
    if (r.status === "fulfilled") return r.value;
    return {
      index: i,
      address: "unknown",
      status: "failure" as const,
      attempts: 0,
      elapsedMs: 0,
      error: r.reason instanceof Error ? r.reason.message : String(r.reason),
    };
  });

  // ── Summary table ───────────────────────────────────────────────
  console.log();
  console.log("=".repeat(70));
  console.log("RESULTS SUMMARY");
  console.log("=".repeat(70));
  console.log(
    "#".padEnd(5) +
    "Address".padEnd(16) +
    "Status".padEnd(10) +
    "Attempts".padEnd(10) +
    "Time".padEnd(10) +
    "Error"
  );
  console.log("-".repeat(70));

  for (const r of accountResults) {
    console.log(
      String(r.index).padEnd(5) +
      shortAddr(r.address).padEnd(16) +
      r.status.padEnd(10) +
      String(r.attempts).padEnd(10) +
      `${r.elapsedMs}ms`.padEnd(10) +
      (r.error || "")
    );
  }

  const successes = accountResults.filter((r) => r.status === "success");
  const failures = accountResults.filter((r) => r.status === "failure");
  const avgAttempts = successes.length
    ? (successes.reduce((s, r) => s + r.attempts, 0) / successes.length).toFixed(1)
    : "N/A";

  console.log("-".repeat(70));
  console.log(`Total: ${accountResults.length} | Success: ${successes.length} | Failed: ${failures.length}`);
  console.log(`Average attempts (successful): ${avgAttempts}`);

  if (failures.length > 0) {
    console.log();
    console.log("⚠️  Some accounts failed authentication — sticky sessions may not be working correctly.");
  }

  if (successes.length > 0 && successes.every((r) => r.attempts === 1)) {
    console.log();
    console.log("✅ All successful accounts verified on first attempt — sticky sessions appear to be working.");
  } else if (successes.length > 0) {
    console.log();
    console.log(
      `⚠️  Successful accounts needed ${avgAttempts} attempts on average — sticky sessions may be flaky.`
    );
  }

  console.log("=".repeat(70));

  if (failures.length > 0) process.exit(1);
}

await main();
