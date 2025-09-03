# StorageHub SDK – Examples

## Overview
These examples are a “hello world” for the StorageHub SDK. They show how to:
- Use backend‑agnostic primitives in `@storagehub-sdk/core` (wallet, WASM file utilities).
- Call an MSP backend with `@storagehub-sdk/msp-client` for authenticated operations like upload/download.

## Why is a backend needed for some operations?
Some operations (e.g., getting a nonce, verifying a signature, uploading/downloading files) must go through an MSP REST service. You can target any running MSP, or run a local mocked backend for development.

### Start a mocked backend locally
From repo root:
```bash
RUST_LOG=info cargo run --bin sh-msp-backend --features mocks -- --host 127.0.0.1 --port 8080
```
Backend listens on `http://127.0.0.1:8080` by default.

## Run the examples
From the repository root:
```bash
cd sdk/examples
pnpm install

# Core demo: health + wallet + fingerprint
pnpm run core

# MSP client demo: auth + upload + download
pnpm run msp
```

## Files
- `core-demo.mjs` – HttpClient (raw query), LocalWallet, FileManager getFingerprint
- `msp-demo.mjs` – MspClient connect, auth (nonce/verify), upload/download
- `data/hello.txt` – sample file for demos

## Troubleshooting
- Ensure `BASE_URL` points to your MSP (default: `http://127.0.0.1:8080`).
- If uploads return 401, verify `client.setToken(token)` was called after verify.
