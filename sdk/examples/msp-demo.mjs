import { createReadStream, createWriteStream } from 'node:fs';
import { Readable } from 'node:stream';
import { MspClient } from '@storagehub-sdk/msp-client';
import { LocalWallet, initWasm } from '@storagehub-sdk/core';

async function main() {
  await initWasm();
  const baseUrl = process.env.BASE_URL || 'http://127.0.0.1:8080';
  const chainId = Number(process.env.CHAIN_ID || '1');

  const client = await MspClient.connect({ baseUrl });

  // Health
  const health = await client.getHealth();
  console.log('health:', health);

  // Auth: SIWE-like nonce + verify using LocalWallet
  const TEST_PK =
    '0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80';
  const wallet = LocalWallet.fromPrivateKey(TEST_PK);
  const address = await wallet.getAddress();

  const { message } = await client.getNonce(address, chainId);
  const signature = await wallet.signMessage(message);
  const verified = await client.verify(message, signature);
  client.setToken(verified.token);
  console.log('verified user:', verified.user);

  // Upload
  const bucketId = '0xBucketId';
  const fileKey = '0xFileKey';
  const filePath = new URL('./data/hello.txt', import.meta.url);
  const receipt = await client.uploadFile(bucketId, fileKey, createReadStream(filePath));
  console.log('uploaded:', receipt);

  // Download
  const download = await client.downloadByKey(bucketId, fileKey);
  const out = createWriteStream(new URL('./data/out.bin', import.meta.url));
  Readable.fromWeb(download.stream).pipe(out);
  await new Promise((resolve, reject) => out.on('finish', resolve).on('error', reject));
  console.log('download status:', download.status);
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});


