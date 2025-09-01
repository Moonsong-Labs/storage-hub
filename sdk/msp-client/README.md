# @storagehub-sdk/msp-client

High-level client for interacting with a StorageHub MSP service.

## Install

```bash
# pnpm (recommended)
pnpm add @storagehub-sdk/msp-client

# npm
npm i @storagehub-sdk/msp-client

# yarn
yarn add @storagehub-sdk/msp-client
```

## Quick start

```ts
import { MspClient } from '@storagehub-sdk/msp-client';
import { createReadStream, createWriteStream } from 'node:fs';
import { Readable } from 'node:stream';

// Create a client
const client = await MspClient.connect({ baseUrl: 'https://storagehub.example.com' });

// Health
const health = await client.getHealth();
console.log('health', health);

// Auth: request nonce and verify signature (SIWE-style)
const chainId = 1;
const { message } = await client.getNonce('0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266', chainId);
// sign `message` with your wallet, then:
const verified = await client.verify(message, '0xSignature');
client.setToken(verified.token);

// Upload a file
const bucketId = '0xBucketId';
const fileKey = '0xFileKey';
const filePath = './path/to/file.bin';
const receipt = await client.uploadFile(bucketId, fileKey, createReadStream(filePath));
console.log('uploaded', receipt);

// Download by key
const download = await client.downloadByKey(bucketId, fileKey);
const out = createWriteStream('./downloaded.bin');
Readable.fromWeb(download.stream).pipe(out);
await new Promise((resolve, reject) => out.on('finish', resolve).on('error', reject));
console.log('status', download.status);
```

## API surface
- `connect({ baseUrl, timeoutMs?, defaultHeaders?, fetchImpl? })`
- `getHealth()`
- `getNonce(address, chainId)`
- `verify(message, signature)`
- `setToken(token)`
- `uploadFile(bucketId, fileKey, file)`
- `downloadByKey(bucketId, fileKey)`
- `downloadByLocation(bucketId, filePath)`

## License
GPL-3.0 (see LICENSE)
