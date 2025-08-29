# StorageHub E2E Tests with dAppWright + SDK

End-to-end tests for StorageHub SDK

## 🚀 Features

- **SDK-backed flows**: Use `Eip1193Wallet.connect()`, `signMessage()`, and `sendTransaction()`
- **Automated MetaMask**: Install, import seed, handle popups via dAppWright
- **Headed and “headless” (Xvfb)**: Local headed and Docker/Xvfb for CI-like runs
- **No chain dependency**: Tx step is initiated and then rejected (no funds required)

## 📁 Structure

```
sdk/e2e/
├── package.json
├── playwright.config.ts
├── Dockerfile
├── page/
│   └── index.html          # Minimal dApp using SDK Eip1193Wallet
└── tests/
    ├── wallet/
    │   └── metamask-sdk-sign.spec.ts
    └── msp/
        ├── auth-localwallet.spec.ts
        ├── health.spec.ts
        ├── upload.spec.ts
        ├── download.spec.ts
        └── unauthorized.spec.ts
```

## 🛠️ Setup

Build SDK once so the browser import map resolves `@storagehub-sdk/core` and `@storagehub/wasm`:

```bash
pnpm -C sdk build
```

Install e2e deps and Playwright (Chromium):

```bash
cd sdk/e2e
pnpm install
pnpm exec playwright install --with-deps chromium
```

## 🧪 Running

Run tests:

```bash
# All tests
pnpm -C sdk build
cd sdk/e2e && pnpm install
pnpm exec playwright test

# Only MetaMask (headed recommended)
HEADLESS=false pnpm exec playwright test --project metamask

# Only MSP (web project)
pnpm exec playwright test --project web
```

## 🔧 How it works

- dAppWright installs MetaMask and imports the seed `"test test test test test test test test test test test junk"`.
- The page is a minimal dApp that imports the SDK via an import map:
  - `@storagehub-sdk/core` → `/core/dist/index.js`
  - `@storagehub/wasm` → `/core/wasm/pkg/storagehub_wasm.js`
  - `ethers` → CDN ESM
- Flow in test `metamask-sdk-sign.spec.ts`:
  1) Open `/e2e/page/index.html`
  2) Click “Connect” → approve in MetaMask
  3) Click “Sign Message” → approve in MetaMask; logs signature
  4) Click “Sign Transaction” → reject in MetaMask; logs a concise message

## 🐛 Troubleshooting

- `#connect` never appears: ensure the server serves the sdk root, and the test URL is `/e2e/page/index.html`.
- Module specifier errors (e.g., `ethers`): confirm the import map in `page/index.html`.
- SDK not found: run `pnpm -C sdk build` to create `core/dist` and `core/wasm/pkg`.
- Playwright binary not found: use `pnpm exec playwright ...` (pnpm resolves local binaries).

## 🧰 CI notes

- Build SDK before tests (`pnpm -C sdk build`).
- Playwright webServer auto-starts the server; reports and artifacts are written to `/tmp`.
- Use `xvfb-run` with `HEADLESS=false` for MetaMask.

## ✅ Success criteria

- Connect approved
- Message signed via SDK; signature logged
- Transaction request initiated via SDK and rejected; rejection logged
