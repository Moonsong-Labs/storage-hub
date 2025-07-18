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
   cargo install wasm-pack
   ```

---

## Quick start

```bash
cd sdk
pnpm install           # builds the WASM crate automatically
pnpm run build
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
sdk/ – workspace root, pnpm workspace + shared tooling
├─ package.json
├─ tsconfig.json
├─ vitest.config.ts
├─ scripts/
│  ├─ build.js
│  └─ clean.js
├─ .gitignore
│
├─ core/  – “@storagehub-sdk/core”
│  ├─ package.json
│  ├─ tsconfig.json
│  ├─ src/
│  │   ├─ index.ts
│  │   ├─ wasm.ts
│  │   └─ types/
│  │       └─ storagehub-wasm.d.ts
│  ├─ tests/
│  │   └─ wasm.spec.ts
│  └─ wasm/
│      ├─ Cargo.toml
│      ├─ src/
│      └─ pkg/
│
└─ msp-client/ – “@storagehub-sdk/msp-client” façade
   ├─ package.json
   ├─ tsconfig.json
   ├─ src/
   │   ├─ MspClient.ts
   │   └─ index.ts
   └─ tests/
```

---
