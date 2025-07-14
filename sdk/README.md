# StorageHub TypeScript SDK

> Early scaffold – subject to change as development continues.

---

## Prerequisites

1. **Node.js** ≥ 18 (recommended 20+)
2. **pnpm** ≥ 8 – `npm i -g pnpm`
3. **Rust toolchain** – <https://rustup.rs>
4. **WASM target & tool** (one-time):
   ```bash
   rustup target add wasm32-unknown-unknown
   cargo install wasm-pack # if not already installed
   ```

---

## Quick start

```bash
cd sdk
pnpm install           # builds the WASM crate automatically
pnpm run build         # bundles the TypeScript SDK
```

### Smoke-test the WASM helper

```bash
node -e "import('@storagehub/sdk').then(m => console.log('2+3 =', m.add(2,3)))"
# → 2+3 = 5
```

---

## Scripts

| Command | Description |
|---------|-------------|
| `pnpm run build:wasm` | Compile the Rust crate → `wasm/pkg` (runs automatically on install) |
| `pnpm run build`      | Bundle TypeScript → `dist/` |
| `pnpm test`           | Run Vitest unit tests |
| `scripts/clean-install-test.sh` | Full clean build & test cycle |

---

## Folder structure

```
sdk/
  ts/          – TypeScript source
  wasm/        – Rust crate
    pkg/       – Generated JS/WASM package (git-ignored)
  dist/        – Bundled JS output (git-ignored)
  scripts/     – Helper scripts
```

---

## Troubleshooting

* **WASM build fails** – Ensure `wasm-pack` is on your `$PATH` and the `wasm32-unknown-unknown` target is installed.
* **Engine warnings** – Other workspace packages may target newer Node versions; they can be ignored when working only in `sdk/`. 