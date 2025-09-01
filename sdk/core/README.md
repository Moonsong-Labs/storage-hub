# @storagehub-sdk/core

Core primitives for the StorageHub SDK

## Install

```bash
# pnpm
pnpm add @storagehub-sdk/core

# npm
npm i @storagehub-sdk/core

# yarn
yarn add @storagehub-sdk/core
```

## Quick start

### HTTP client
```ts
import { HttpClient } from '@storagehub-sdk/core';

const http = new HttpClient({ baseUrl: 'https://storagehub.example.com' });
const health = await http.get('/health');
console.log(health);
```

### Local wallet (development/testing)
```ts
import { LocalWallet } from '@storagehub-sdk/core';

const wallet = LocalWallet.fromPrivateKey(
  '0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80',
);
const address = await wallet.getAddress();
const sig = await wallet.signMessage('hello');
console.log({ address, sig });
```

### EIP-1193 wallet (browser)
```ts
import { Eip1193Wallet } from '@storagehub-sdk/core';

const wallet = await Eip1193Wallet.connect(); // prompts the injected wallet
const address = await wallet.getAddress();
console.log(address);
```

### Compute file fingerprint
```ts
import { FileManager } from '@storagehub-sdk/core';
import { createReadStream, statSync } from 'node:fs';
import { Readable } from 'node:stream';

const filePath = './path/to/file.bin';
const size = statSync(filePath).size;

// Convert Node Readable to a Web ReadableStream<Uint8Array>
const nodeStream = createReadStream(filePath);
const webStream = Readable.toWeb(nodeStream) as unknown as ReadableStream<Uint8Array>;

const fm = new FileManager({ size, stream: () => webStream });
const fingerprint = await fm.getFingerprint();
console.log('H256 fingerprint', fingerprint.toHex());
```

## License
GPL-3.0 (see LICENSE)
