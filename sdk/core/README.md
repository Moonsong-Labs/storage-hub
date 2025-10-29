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

### EVM Integration with StorageHub

The Core SDK provides seamless integration with StorageHub's EVM precompiles, offering automatic gas estimation, type safety, and a clean developer experience.

#### ABI Generation
- Solidity contracts are automatically compiled to ABI during `prebuild` using `solc-js`
- Generated ABIs are strongly typed using `viem` and `abitype`
- No code generation required - pure TypeScript type inference

#### StorageHubClient - Unified EVM Interface

```ts
import { StorageHubClient, SH_FILE_SYSTEM_PRECOMPILE_ADDRESS } from '@storagehub-sdk/core';
import { createWalletClient, defineChain, http } from 'viem';
import { privateKeyToAccount } from 'viem/accounts';

// Define your StorageHub chain
const storageHubChain = defineChain({
  id: 181222,
  name: 'StorageHub',
  nativeCurrency: { name: 'StorageHub', symbol: 'SH', decimals: 18 },
  rpcUrls: { default: { http: ['http://localhost:9944'] } },
});

// Create wallet client
const account = privateKeyToAccount('0x...');
const walletClient = createWalletClient({
  chain: storageHubChain,
  account,
  transport: http('http://localhost:9944')
});

// Create StorageHub client - handles everything automatically
const hub = new StorageHubClient({
  rpcUrl: 'http://localhost:9944',
  chain: storageHubChain,
  walletClient,
  filesystemContractAddress: SH_FILE_SYSTEM_PRECOMPILE_ADDRESS
});
```

#### Basic Usage

```ts
// Read operations (no gas required)
const name = new TextEncoder().encode('my-bucket');
const bucketId = await hub.deriveBucketId('0xOwnerAddress', name);
const pendingRequests = await hub.getPendingFileDeletionRequestsCount('0xUserAddress');

// Write operations (automatic gas estimation)
const txHash = await hub.createBucket(
  '0xMspId',           // MSP ID
  name,               // Bucket name (max 100 bytes)
  false,              // isPrivate
  '0xValuePropId'     // Value proposition ID
);

// Write with custom gas options
const txHash2 = await hub.updateBucketPrivacy('0xBucketId', true, {
  gasMultiplier: 8,        // Higher safety margin
  gasPrice: parseGwei('2') // Custom gas price
});
```

#### Available Methods

**Bucket Management:**
- `createBucket(mspId, name, isPrivate, valuePropId, options?)`
- `deleteBucket(bucketId, options?)`
- `updateBucketPrivacy(bucketId, isPrivate, options?)`
- `requestMoveBucket(bucketId, newMspId, newValuePropId, options?)`

**Storage Operations:**
- `issueStorageRequest(bucketId, location, fingerprint, size, mspId, peerIds, replicationTarget, customReplicationTarget, options?)`
- `revokeStorageRequest(fileKey, options?)`
- `requestDeleteFile(fileInfo, options?)`

**Collections:**
- `createAndAssociateCollectionWithBucket(bucketId, options?)`

**Read Operations:**
- `deriveBucketId(owner, name)`
- `getPendingFileDeletionRequestsCount(user)`

#### Gas Handling

The SDK provides intelligent gas estimation with Frontier chain optimizations:

```ts
// Automatic gas estimation (recommended)
await hub.createBucket(mspId, name, false, valuePropId);

// Custom gas options
await hub.createBucket(mspId, name, false, valuePropId, {
  gasMultiplier: 6,           // Safety multiplier (default: 5)
  gasPrice: parseGwei('1.5'), // Legacy gas pricing
  // OR EIP-1559 fees:
  maxFeePerGas: parseGwei('2'),
  maxPriorityFeePerGas: parseGwei('0.5')
});

// Explicit gas limit
await hub.createBucket(mspId, name, false, valuePropId, {
  gas: 500_000n  // Skip estimation, use exact amount
});
```

#### Error Handling

```ts
try {
  await hub.createBucket(mspId, name, false, valuePropId);
} catch (error) {
  if (error.message.includes('exceeds maximum length')) {
    // Handle validation errors
  } else if (error.message.includes('OutOfGas')) {
    // Handle gas estimation issues
  }
}
```

#### Type Safety

All methods are fully typed with parameter validation:

```ts
// ✅ Type-safe parameters
const name = new TextEncoder().encode('bucket-name'); // Uint8Array
const mspId = '0x...' as `0x${string}`;              // Hex string type

// ❌ Compile-time errors for invalid types
await hub.createBucket('invalid', 'string', false, mspId); // TypeScript error
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
