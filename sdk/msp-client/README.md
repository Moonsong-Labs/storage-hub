# @storagehub-sdk/msp-client

High-level client for interacting with StorageHub MSP (Main Storage Provider) services.

## What is this?

The `@storagehub-sdk/msp-client` is a TypeScript client library that provides a simple, high-level interface for:

- **File Storage & Retrieval**: Upload and download files to/from StorageHub MSP services
- **Authentication**: SIWE-style (Sign-In With Ethereum) authentication with MSP providers
- **Health Monitoring**: Check MSP service availability and status
- **Bucket Management**: Interact with storage buckets, listing the existent ones, getting their metadata and the files they contain

This package is built on top of `@storagehub-sdk/core` and provides a more convenient API for common MSP operations, abstracting away the lower-level details.

## Prerequisites

**⚠️ Important**: This client connects to a StorageHub MSP backend service. You need:

1. **A running MSP backend** - Either:
   - A production MSP service endpoint, or
   - A local MSP backend for development/testing

2. **StorageHub node** - The MSP backend requires connection to a StorageHub blockchain node

### Quick Backend Setup for Development

To run a local MSP backend with mocks for testing:

```bash
# From the StorageHub repository root
RUST_LOG=info cargo run --bin sh-msp-backend --features mocks -- --host 127.0.0.1 --port 8080
```

This starts a mock MSP backend on `http://127.0.0.1:8080` that you can use for development.

## Install

```bash
# pnpm (recommended)
pnpm add @storagehub-sdk/msp-client

# npm
npm i @storagehub-sdk/msp-client

# yarn
yarn add @storagehub-sdk/msp-client
```

## Quick Start

```ts
import { MspClient } from '@storagehub-sdk/msp-client';
import { createReadStream, createWriteStream } from 'node:fs';
import { Readable } from 'node:stream';

// 1. Connect to MSP service
let sessionRef: { token: string; user: { address: string } } | undefined;
const sessionProvider = async () => sessionRef;
const client = await MspClient.connect({ 
  baseUrl: 'http://127.0.0.1:8080'
}, sessionProvider);

// 2. Check service health
const health = await client.info.getHealth();
console.log('MSP service health:', health);

// 3. Authenticate with wallet (SIWE-style)
// Example with viem's WalletClient
import { createWalletClient, http } from 'viem';
import { privateKeyToAccount } from 'viem/accounts';
const account = privateKeyToAccount('0x<your_dev_private_key>');
const wallet = createWalletClient({ account, transport: http('http://127.0.0.1:8545') });
const session = await client.auth.SIWE(wallet);
sessionRef = session;

// 4. Upload a file
const bucketId = '0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef'; // StorageHub bucket identifier
const fileKey = '0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890';   // Unique file identifier
const filePath = './myfile.txt';
const owner = walletAddress;      // File owner
const location = 'myfile.txt';    // File location/path within the bucket

const receipt = await client.files.uploadFile(bucketId, fileKey, createReadStream(filePath), owner, location);
console.log('File uploaded successfully:', receipt);

// 5. Download the file
const download = await client.files.downloadFile(fileKey);
const outputPath = './downloaded-file.txt';

// Stream the download to a file
const writeStream = createWriteStream(outputPath);
Readable.fromWeb(download.stream).pipe(writeStream);

await new Promise((resolve, reject) => {
  writeStream.on('finish', resolve);
  writeStream.on('error', reject);
});

console.log('File downloaded successfully to:', outputPath);
console.log('Download status:', download.status);

// 6. List the buckets of the currently authenticated user
const buckets = await client.buckets.listBuckets();
console.log('Buckets:', buckets);

// 7. Get the metadata of a specific bucket
const bucket = await client.buckets.getBucket(bucketId);
console.log('Bucket:', bucket);

// 8. Get the files of the root folder of a specific bucket
const files = await client.buckets.getFiles(bucketId);
console.log('Root files:', files);

// 9. Get the files of a specific folder of a specific bucket
const folderFiles = await client.buckets.getFiles(bucketId, { path: '/path/to/folder' });
console.log('Folder files:', folderFiles);
```

## API Reference

### Static Methods
- **`MspClient.connect(config, sessionProvider)`** - Create and connect to MSP service
  - `config.baseUrl: string` - MSP backend URL (e.g., `http://127.0.0.1:8080`)
  - `config.timeoutMs?: number` - Request timeout in milliseconds
  - `config.defaultHeaders?: Record<string, string>` - Default HTTP headers
  - `config.fetchImpl?: typeof fetch` - Custom fetch implementation
  - `sessionProvider: () => Promise<Session | undefined>` - Returns the current session (or undefined)

### Modules (instance properties)
- **`auth`**: SIWE auth and session helpers
  - `SIWE(wallet, signal?)` – runs full SIWE flow and returns `Session`
  - `getProfile(signal?)` – returns the authenticated user's profile
- **`info`**: MSP info and stats
  - `getHealth(signal?)` – returns service health and status
  - `getInfo(signal?)` – returns general MSP info (id, version, owner, endpoints)
  - `getStats(signal?)` – returns capacity and usage stats
  - `getValuePropositions(signal?)` – returns available value props/pricing
  - `getPaymentStreams(signal?)` – returns the authenticated user's payment streams
- **`buckets`**: Buckets and file listings
  - `listBuckets(signal?)` – returns all buckets for the current authenticated user
  - `getBucket(bucketId, signal?)` – returns metadata for a specific bucket
  - `getFiles(bucketId, { path?, signal? })` – returns the file tree at root or at a subpath
- **`files`**: File metadata, upload and download
  - `getFileInfo(bucketId, fileKey, signal?)` – returns metadata for a specific file
  - `uploadFile(...)` – uploads a file to the MSP
  - `downloadFile(fileKey, options?)` – downloads a file by key (supports range)

### Utilities available via `files`
- `hexToBytes(hex)`
- `formFileMetadata(owner, bucketId, location, fingerprint, size)`
- `computeFileKey(metadata)`