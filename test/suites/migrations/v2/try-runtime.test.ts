import { describe, it } from "node:test";
import { spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import { resolve } from "node:path";
import { strictEqual, ok } from "node:assert";

const WASM_PATH = resolve(
  process.env.WASM_PATH ??
    "../target/release/wbuild/sh-parachain-runtime/sh_parachain_runtime.compact.compressed.wasm",
);

const TESTNET_WS =
  process.env.TESTNET_WS ?? "wss://services.datahaven-testnet.network/testnet";

describe("Migration v2: try-runtime", () => {
  it("wasm artifact exists", () => {
    ok(
      existsSync(WASM_PATH),
      `Wasm not found at ${WASM_PATH}. Build with: cargo build --release -p sh-parachain-runtime --features try-runtime`,
    );
  });

  it("try-runtime CLI is installed", () => {
    const result = spawnSync("try-runtime", ["--version"], { encoding: "utf8" });
    strictEqual(result.status, 0, "try-runtime CLI not found. Install with: cargo install --git https://github.com/paritytech/try-runtime-cli --locked");
  });

  it("migration runs to completion against testnet state", { timeout: 900_000 }, () => {
    // pallet_migrations::Config::Migrations is set to () for the try-runtime feature, so
    // no MBM cursor is set after on_runtime_upgrade.
    //
    // --disable-mbm-checks: prevents try-runtime from running its separate Phase 2
    //   MBM block-production loop, which unconditionally panics on Cumulus parachains
    //   (cumulus_pallet_parachain_system::create_inherent requires relay-chain validation
    //   data unavailable in try-runtime's mock environment). The migration itself runs
    //   synchronously via TryRuntimeMigrate in the Executive's Migrations tuple.
    //
    // --checks none: prevents try_decode_entire_state from running against FileSystem
    //   storage items (e.g. StorageRequests) whose on-disk codec may differ from the
    //   testnet's older runtime. Migration correctness is instead verified by
    //   TryRuntimeMigrate::on_runtime_upgrade, which embeds pre/post checks and panics
    //   if they fail — causing try-runtime to exit non-zero (test fails).
    const result = spawnSync(
      "try-runtime",
      [
        "--runtime", WASM_PATH,
        "--disable-spec-name-check",
        "on-runtime-upgrade",
        "--blocktime", "6000",
        "--disable-spec-version-check",
        "--disable-mbm-checks",
        "--checks", "none",
        "live",
        "--uri", TESTNET_WS,
        "--pallet", "FileSystem",
      ],
      { encoding: "utf8", timeout: 890_000, maxBuffer: 100 * 1024 * 1024 },
    );

    if (result.stdout) process.stdout.write(result.stdout);
    if (result.stderr) process.stderr.write(result.stderr);

    strictEqual(
      result.status,
      0,
      `try-runtime exited with code ${result.status} (signal: ${result.signal}). Check output above for details.`,
    );
  });
});
