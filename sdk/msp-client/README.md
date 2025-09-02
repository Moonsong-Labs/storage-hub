# @storagehub-sdk/msp-client

High-level client for interacting with StorageHub MSP (Main Storage Provider) services.

## What is this?

The `@storagehub-sdk/msp-client` is a TypeScript client library that provides a simple, high-level interface for:

- **File Storage & Retrieval**: Upload and download files to/from StorageHub MSP services
- **Authentication**: SIWE-style (Sign-In With Ethereum) authentication with MSP providers
- **Health Monitoring**: Check MSP service availability and status
- **Bucket Management**: Interact with storage buckets and file keys

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
const client = await MspClient.connect({ 
  baseUrl: 'http://127.0.0.1:8080' // Your MSP backend URL
});

// 2. Check service health
const health = await client.getHealth();
console.log('MSP service health:', health);

// 3. Authenticate with wallet (SIWE-style)
const walletAddress = '0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266';
const chainId = 1; // Ethereum mainnet

// Get authentication message to sign
const { message } = await client.getNonce(walletAddress, chainId);
console.log('Sign this message with your wallet:', message);

// After signing with your wallet (e.g., MetaMask, WalletConnect, etc.)
const signature = '0xYourWalletSignature...'; // Replace with actual signature
const verified = await client.verify(message, signature);
client.setToken(verified.token); // Set auth token for subsequent requests

// 4. Upload a file
const bucketId = '0xYourBucketId'; // StorageHub bucket identifier  
const fileKey = '0xYourFileKey';   // Unique file identifier
const filePath = './myfile.txt';

const receipt = await client.uploadFile(bucketId, fileKey, createReadStream(filePath));
console.log('File uploaded successfully:', receipt);

// 5. Download the file
const download = await client.downloadByKey(bucketId, fileKey);
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
```

## API Reference

### Static Methods
- **`MspClient.connect(config)`** - Create and connect to MSP service
  - `config.baseUrl: string` - MSP backend URL (e.g., `http://127.0.0.1:8080`)
  - `config.timeoutMs?: number` - Request timeout in milliseconds
  - `config.defaultHeaders?: Record<string, string>` - Default HTTP headers
  - `config.fetchImpl?: typeof fetch` - Custom fetch implementation

### Instance Methods
- **`getHealth()`** - Check MSP service health and status
- **`getNonce(address, chainId)`** - Get authentication message for wallet signing
  - `address: string` - Wallet address (0x...)
  - `chainId: number` - Blockchain chain ID (1 for Ethereum mainnet)
- **`verify(message, signature)`** - Verify wallet signature and get auth token
  - `message: string` - The message that was signed
  - `signature: string` - Wallet signature (0x...)
- **`setToken(token)`** - Set authentication token for subsequent requests
- **`uploadFile(bucketId, fileKey, file)`** - Upload file to storage
  - `bucketId: string` - Storage bucket identifier
  - `fileKey: string` - Unique file key/identifier
  - `file: ReadStream | Blob | File` - File data to upload
- **`downloadByKey(bucketId, fileKey)`** - Download file by bucket and key
  - Returns: `{ stream: ReadableStream, status: string }`
- **`downloadByLocation(bucketId, filePath)`** - Download file by bucket and path
  - Returns: `{ stream: ReadableStream, status: string }`
