# StorageHub TypeScript SDK

> Early scaffold – subject to change as development continues.

---

## Prerequisites

1. **Node.js** ≥ 23
2. **pnpm** ≥ 8 – `npm i -g pnpm`
3. **Rust toolchain** – <https://rustup.rs>
4. **WASM target & tool** (one-time):
   ```bash
   rustup target add wasm32-unknown-unknown
   cargo install wasm-pack
   ```

---

## Quick start

```bash
cd sdk
pnpm install
pnpm run build          # builds core and msp-client
# optional: if you modified the Rust WASM crate in core/
pnpm run build:wasm     # builds the WASM package in core/wasm/pkg
```

---

## Scripts

| Command | Description |
|---------|-------------|
| `pnpm run build`               | Build workspace packages (`core`, `msp-client`) |
| `pnpm run build:wasm`          | Build Rust WASM crate → `core/wasm/pkg` |
| `pnpm test`                    | Run all unit tests |
| `pnpm test:core`               | Run core unit tests |
| `pnpm test:msp-client`         | Run msp-client unit tests |
| `pnpm lint` / `pnpm format`    | Lint / format sources |
| `pnpm format:check`            | Check formatting only |
| `pnpm typecheck`               | TypeScript type-check only |
| `pnpm coverage`                | Run tests with coverage |
| `scripts/clean-install-test.sh`| Full clean build & test cycle |

---

## Folder structure

```
sdk/ – workspace root, pnpm workspace + shared tooling
├─ package.json
├─ tsconfig.json
├─ vitest.config.ts
├─ vitest.setup.ts
├─ scripts/
│  ├─ build.js
│  ├─ clean.js
│  └─ clean-install-test.sh
│
├─ core/  – “@storagehub-sdk/core”
│  ├─ package.json
│  ├─ tsconfig.json
│  ├─ src/
│  │   ├─ index.ts
│  │   ├─ wasm.ts
│  │   ├─ http/
│  │   │   ├─ errors.ts
│  │   │   └─ HttpClient.ts
│  │   ├─ wallet/
│  │   │   ├─ base.ts
│  │   │   ├─ eip1193.ts
│  │   │   ├─ errors.ts
│  │   │   ├─ local.ts
│  │   │   └─ metamask.ts
│  │   └─ types/
│  │       └─ storagehub-wasm.d.ts
│  ├─ tests/
│  │   ├─ file-manager.spec.ts
│  │   ├─ filekey.spec.ts
│  │   ├─ merkle.spec.ts
│  │   └─ wallet_local.spec.ts
│  └─ wasm/
│      ├─ Cargo.toml
│      ├─ src/
│      └─ pkg/
│
├─ msp-client/ – “@storagehub-sdk/msp-client” façade
│  ├─ package.json
│  ├─ tsconfig.json
│  ├─ src/
│  │   ├─ MspClient.ts
│  │   └─ index.ts
│  └─ tests/
│      ├─ auth.e2e.spec.ts
│      ├─ download.e2e.spec.ts
│      ├─ health.e2e.spec.ts
│      └─ upload.spec.ts
│
├─ e2e/ – Playwright E2E projects (MetaMask & MSP)
│  ├─ package.json
│  ├─ playwright.config.ts
│  ├─ README.md
│  └─ tests/
│      ├─ wallet/
│      │   └─ metamask-sdk-sign.spec.ts
│      └─ msp/
│          ├─ auth.spec.ts
│          ├─ download.spec.ts
│          ├─ health.spec.ts
│          ├─ unauthorized.spec.ts
│          └─ upload.spec.ts
│
└─ examples/
   └─ metamask-wallet/
      ├─ README.md
      ├─ index.html
      ├─ app.js
      └─ style.css
```

---
