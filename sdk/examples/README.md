# StorageHub SDK – Examples

## Prerequisites
- Node.js 22+
- Rust toolchain (to run the backend)

## Start the backend with mocks
From repo root:
```bash
RUST_LOG=info cargo run --bin sh-msp-backend --features mocks -- --host 127.0.0.1 --port 8080
```
Backend listens on `http://127.0.0.1:8080` by default.

## Run examples
From repo root:
```bash
cd sdk/examples
pnpm install

# Health + wallet + fingerprint demo
pnpm run core

# MSP client demo (auth + upload + download)
pnpm run msp
```

## Files
- `core-demo.mjs` – HttpClient, LocalWallet, FileManager fingerprint
- `msp-demo.mjs` – MspClient connect, auth (nonce/verify), upload/download
- `data/hello.txt` – sample file for demos
