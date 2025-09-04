# StorageHub SDK – Examples

Both consume published `@storagehub-sdk/core` and `@storagehub-sdk/msp-client`.

## Why is a backend needed for some operations?
Some operations (e.g., getting a nonce, verifying a signature, uploading/downloading files) must go through an MSP REST service. You can target any running MSP, or run a local mocked backend for development.

### Start a mocked backend locally
From repo root:
```bash
RUST_LOG=info cargo run --bin sh-msp-backend --features mocks -- --host 127.0.0.1 --port 8080
```
Backend listens on `http://127.0.0.1:8080` by default.

## Quick start
```bash
# Node example
cd sdk/examples/node
pnpm install
pnpm start

# Next.js example
cd sdk/examples/nextjs
pnpm install
pnpm dev
```

