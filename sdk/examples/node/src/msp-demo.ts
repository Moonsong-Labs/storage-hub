import { createReadStream, createWriteStream } from 'node:fs';
import { Readable } from 'node:stream';
import { Bucket, MspClient, type FileListResponse } from '@storagehub-sdk/msp-client';
import { LocalWallet, initWasm } from '@storagehub-sdk/core';
import type { VerifyResponse, UploadReceipt } from '@storagehub-sdk/msp-client';

export async function runMspDemo(): Promise<void> {
  // Initialize embedded WASM once
  await initWasm();
  // Backend endpoint and chain id
  const baseUrl = process.env.BASE_URL || 'http://127.0.0.1:8080';
  const chainId = Number(process.env.CHAIN_ID || '1');

  // Connect MSP client
  const client = await MspClient.connect({ baseUrl });

  // Health check
  const health = await client.getHealth();
  console.log('health:', health);

  // Prepare wallet (test key) and derive address
  const TEST_PK =
    '0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80';
  const wallet = LocalWallet.fromPrivateKey(TEST_PK);
  const address = await wallet.getAddress();

  // SIWE-like: request message, sign, and verify to obtain JWT
  const { message } = await client.getNonce(address, chainId);
  const signature = await wallet.signMessage(message);
  const verified: VerifyResponse = await client.verify(message, signature);
  client.setToken(verified.token);
  console.log('verified user:', verified.user);

  // Upload a file from disk
  const bucketId = '0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef';
  const fileKey = '0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890';
  const filePath = new URL('../data/hello.txt', import.meta.url);
  const owner = address;
  const location = 'hello.txt';
  const receipt: UploadReceipt = await client.uploadFile(bucketId, fileKey, createReadStream(filePath), owner, location);
  console.log('uploaded:', receipt);

  // Download the file to disk
  const download = await client.downloadByKey(fileKey);
  const out = createWriteStream(new URL('../data/out.bin', import.meta.url));
  Readable.fromWeb(download.stream).pipe(out);
  await new Promise((resolve, reject) => out.on('finish', resolve).on('error', reject));
  console.log('download status:', download.status);

  // List buckets
  const buckets: Bucket[] = await client.listBuckets();
  console.log(`Buckets size: ${buckets.length}`);
  for (const bucket of buckets) {
    console.log(` Bucket name: ${bucket.name}`);
  }

  // Get a bucket and list root + a subpath
  const bucket: Bucket = await client.getBucket(bucketId);
  console.log('Bucket metadata:', bucket);

  const rootFiles = await client.getFiles(bucketId);
  console.log('Root files:', rootFiles);

  const thesisFiles: FileListResponse = await client.getFiles(bucketId, { path: '/Thesis/' });
  console.log('Thesis folder size:', thesisFiles.files.length);
  for (const entry of thesisFiles.files) {
    console.log(` Entry name: ${entry.name}`);
  }
}
