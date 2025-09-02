## StorageHub SDK

Developer-friendly SDK to integrate with the StorageHub network without learning internal details (nodes, pallets, EVM precompiles). The SDK is split into two packages for convenient separation of concerns: `@storagehub-sdk/core` provides backend‑agnostic primitives, while `@storagehub-sdk/msp-client` contains MSP‑specific APIs. Both package run in both browser and Node.js environments.

### Packages
- **@storagehub-sdk/core**
  - Backend‑agnostic building blocks (wallets, EIP‑1193, precompile helpers bridging Substrate↔EVM, Merkle/WASM utilities, HttpClient, shared types).
  - Read more: `sdk/core/README.md`.
  - Includes: EVM account‑typed helpers, WASM‑backed file utilities, and stable primitives usable without any backend.
  - Use for: signing, Merkle/proofs, precompile calls, low‑level HTTP, shared types.
- **@storagehub-sdk/msp-client**
  - MSP‑specific client (health, auth nonce/verify, upload/download endpoints). All MSP‑tied logic lives here.
  - Read more: `sdk/msp-client/README.md`.
  - Includes: REST contracts for MSP, token handling, streaming/multipart upload and download helpers.
  - Use for: talking to an MSP backend (auth + file transfer).

### Why this separation?
- **Abstraction boundary**: Core = stable, typed primitives. MSP client = backend contracts and token handling.
- **Portability**: Core works in browser and Node.js, independent of a backend.
- **Independent evolution**: MSP endpoints can change without affecting core primitives.

### Choosing a package
- Building features that only need chain primitives or browser wallets → start with `@storagehub-sdk/core`.
- Integrating MSP REST (auth, upload/download) → add `@storagehub-sdk/msp-client`.
- Most real apps will use both: core for signing/proofs and msp-client for data transfer.

### Examples
Hands‑on examples are available under `sdk/examples/`:
- `core-demo.mjs` – HttpClient (raw query), LocalWallet, FileManager getFingerprint
- `msp-demo.mjs` – MSP connect, auth (nonce/verify), upload/download
See `sdk/examples/README.md` for how to run them.

### Environments
- **Browser**: first‑class support (auto‑bound `fetch`) and EIP‑1193 wallets.
- **Node.js (LTS 18+)**: supported; older Node may require a `fetch` polyfill.

### Folder structure

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
