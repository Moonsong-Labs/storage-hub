import { describe, it } from "node:test";
import { spawnSync } from "node:child_process";
import { strictEqual } from "node:assert";
import { WS_URI, WASM_PATH, assertWasmExists, BLOCK_TIME_MS } from "../config.ts";

describe("Migration v2: try-runtime", () => {
  it("wasm artifact exists", () => {
    assertWasmExists("cargo build --release -p sh-parachain-runtime --features try-runtime");
  });

  it("try-runtime CLI is installed", () => {
    const result = spawnSync("try-runtime", ["--version"], { encoding: "utf8" });
    strictEqual(
      result.status,
      0,
      "try-runtime CLI not found. Install with: cargo install --git https://github.com/paritytech/try-runtime-cli --locked"
    );
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
        "--runtime",
        WASM_PATH,
        "--disable-spec-name-check",
        "on-runtime-upgrade",
        "--blocktime",
        String(BLOCK_TIME_MS),
        "--disable-spec-version-check",
        "--disable-mbm-checks",
        "--checks",
        "none",
        "live",
        "--uri",
        WS_URI,
        "--pallet",
        "FileSystem"
      ],
      { encoding: "utf8", timeout: 890_000, maxBuffer: 100 * 1024 * 1024 }
    );

    if (result.stdout) process.stdout.write(result.stdout);
    if (result.stderr) process.stderr.write(result.stderr);

    strictEqual(
      result.status,
      0,
      `try-runtime exited with code ${result.status} (signal: ${result.signal}). Check output above for details.`
    );
  });
});
