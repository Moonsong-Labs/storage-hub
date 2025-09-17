import { createReadStream, createWriteStream } from 'node:fs';
import { Readable } from 'node:stream';
import { ReadableStream as NodeReadableStream } from 'node:stream/web';
import {
  Bucket,
  MspClient,
  type FileListResponse,
  type HealthStatus,
  type InfoResponse,
  type StatsResponse,
  type ValueProp,
  type DownloadResult,
  type FileInfo,
} from '@storagehub-sdk/msp-client';
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
  const health: HealthStatus = await client.getHealth();
  console.log('health:', health);

  // MSP info endpoints
  const info: InfoResponse = await client.getInfo();
  console.log('info:', info);
  const stats: StatsResponse = await client.getStats();
  console.log('stats:', stats);
  const valueProps: ValueProp[] = await client.getValueProps();
  console.log('valueProps count:', Array.isArray(valueProps) ? valueProps.length : 0);

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

  // Upload a file from disk (mock defaults are fine)
  const bucketId = process.env.BUCKET_ID || 'd8793e4187f5642e96016a96fb33849a7e03eda91358b311bbd426ed38b26692';
  const fileKey = process.env.FILE_KEY || 'e901c8d212325fe2f18964fd2ea6e7375e2f90835b638ddb3c08692edd7840f7';
  const filePath = new URL('../data/hello.txt', import.meta.url);
  const receipt: UploadReceipt = await client.uploadFile(bucketId, fileKey, createReadStream(filePath));
  console.log('uploaded:', receipt);

  // Download the file to disk
  const download: DownloadResult = await client.downloadByKey(bucketId, fileKey);
  const out = createWriteStream(new URL('../data/out.bin', import.meta.url));
  const nodeReadable = Readable.fromWeb(download.stream);
  nodeReadable.pipe(out);
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

  const rootFiles: FileListResponse = await client.getFiles(bucketId);
  console.log('Root files:', rootFiles);

  const thesisFiles: FileListResponse = await client.getFiles(bucketId, { path: '/Thesis/' });
  console.log('Thesis folder size:', thesisFiles.files.length);
  for (const entry of thesisFiles.files) {
    console.log(` Entry name: ${entry.name}`);
  }

  // Get file info
  const fileInfo: FileInfo = await client.getFileInfo(bucketId, fileKey);
  console.log('fileInfo:', { ...fileInfo, uploadedAt: fileInfo.uploadedAt.toISOString() });
}
