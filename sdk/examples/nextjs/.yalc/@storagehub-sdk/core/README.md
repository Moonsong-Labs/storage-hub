## StorageHub SDK — Core

Foundational, backend-agnostic building blocks for StorageHub. This package contains StorageHub-specific logic that does not depend on the MSP backend implementation: wallet utilities, blockchain interaction helpers, precompile bindings, Merkle tooling, cryptography/WASM helpers, low-level HTTP utilities, and shared types. Its goal is to let developers integrate StorageHub without needing to understand network implementation details (nodes, pallets, EVM precompiles), by exposing stable, typed primitives and adapters.

### What this package is
- **Wallet and signing**: Local wallet utilities and EIP-1193 adapter for injected wallets.
- **Blockchain interaction**: Helpers for StorageHub chain types/encodings with EVM account types.
- **Precompile helpers**: Bridge Substrate and EVM via precompiles to enable smart‑contract flows (issue storage requests, create buckets, etc.).
- **Merkle utilities**: Build/verify trees and proofs used by StorageHub.
- **Crypto/WASM helpers**: Performance-critical primitives via WASM (e.g., FileManager's Merkle/trie and hashing).
- **HTTP utilities**: Minimal `HttpClient` and types for consistent networking.
- **Shared types/constants**: Common shapes used across SDK packages.

### What this package is not
- **Not an MSP client**: No MSP auth, upload/download, or REST contracts. See `@storagehub-sdk/msp-client`.
- **Not app-specific**: No UI/product code; only reusable SDK primitives.

### Install
```bash
pnpm add @storagehub-sdk/core
# or
npm i @storagehub-sdk/core
# or
yarn add @storagehub-sdk/core
```

### Quick start
```ts
import { HttpClient, type HttpClientConfig } from '@storagehub-sdk/core';

const http = new HttpClient({
  baseUrl: 'https://example.invalid',
} satisfies HttpClientConfig);

```

### Local wallet
```ts
import { LocalWallet } from '@storagehub-sdk/core';

const wallet = LocalWallet.fromPrivateKey('0xYourPrivateKey');
const address = await wallet.getAddress();
const signature = await wallet.signMessage('hello');
```

### EIP-1193 wallet (browser)
```ts
import { Eip1193Wallet } from '@storagehub-sdk/core';

const wallet = await Eip1193Wallet.connect(); // prompts injected wallet (e.g., MetaMask)
const address = await wallet.getAddress();
const signature = await wallet.signMessage('hello');
```

### FileManager (WASM-backed)
```ts
import { FileManager } from '@storagehub-sdk/core';
import { createReadStream, statSync } from 'node:fs';
import { Readable } from 'node:stream';

const path = './path/to/file.bin';
const size = statSync(path).size;
const webStream = Readable.toWeb(createReadStream(path)) as unknown as ReadableStream<Uint8Array>;

const fm = new FileManager({ size, stream: () => webStream });
const fingerprint = await fm.getFingerprint(); // H256
```

### Design principles
- **Backend-agnostic**: No MSP-specific behavior here.
- **Composable and typed**: Small utilities, strong types, clear errors.
- **Runtime-friendly**: Browser and Node support; `fetch` is properly bound in browsers.
- **Separation of concerns**: Core primitives live here; MSP flows live in `msp-client`.

### Roadmap
- **Typed precompile interfaces**: Strongly-typed bindings and ABI-like helpers.
- **Proof tooling**: End-to-end generation/verification for storage proofs.
- **Chunking/commitment utilities**: Streaming chunker and deterministic trees.

### When to use `core` vs `msp-client`
- Use **`@storagehub-sdk/core`** for primitives that must work without any backend (wallets, Merkle, precompiles, chain helpers, low-level HTTP).
- Use **`@storagehub-sdk/msp-client`** for MSP endpoints, auth/nonce/sign/verify flows, and file transfer APIs.

### Environments
Supported environments: web browsers and Node.js.
