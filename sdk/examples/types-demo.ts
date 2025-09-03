import { initWasm, LocalWallet, type HttpClientConfig } from '@storagehub-sdk/core';
import type { Bucket, FileEntry, FileListResponse, UploadOptions, VerifyResponse } from '@storagehub-sdk/msp-client';
import { MspClient } from '@storagehub-sdk/msp-client';
import type { HealthStatus } from '@storagehub-sdk/msp-client';

async function main() {
  await initWasm();

  const baseUrl = process.env.BASE_URL || 'http://127.0.0.1:8080';

  // Type-only import from core
  const httpCfg: HttpClientConfig = {
    baseUrl,
    timeoutMs: 10_000,
    defaultHeaders: { 'x-demo': '1' },
  };
  void httpCfg;

  // Instantiate client and exercise typed APIs
  const client = await MspClient.connect({ baseUrl });


  const health: HealthStatus = await client.getHealth();
  // Health is typed as HealthStatus
  console.log('health status:', health.status);

  // Verify types for responses
  const verifyResponseShape = (v: VerifyResponse) => v;
  const fake: VerifyResponse = verifyResponseShape({ token: 't', user: { address: '0x0' } });
  console.log('verify shape token length:', fake.token.length);

  // Auth: SIWE-like nonce + verify using LocalWallet
  const TEST_PK =
    '0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80';
  const wallet = LocalWallet.fromPrivateKey(TEST_PK);
  const address = await wallet.getAddress();

  const { message } = await client.getNonce(address, 1);
  const signature = await wallet.signMessage(message);
  const verified = await client.verify(message, signature);
  client.setToken(verified.token);

  let fileList: FileListResponse = await client.getFiles("asd123");
  console.log(`FileListSize: ${fileList.files.length}`);
  for (const entry of fileList.files) {
    console.log("Entry information:")
    console.log(`Name: ${entry.name}`);
  }

  // Buckets list: Bucket[]
  try {
    const buckets: Bucket[] = await client.listBuckets();
    console.log('buckets count:', buckets.length);
  } catch (e) {
    console.log('listBuckets requires auth; skipping in types demo');
  }

  // Get files response and element type validation
  const bucketId = '0xBucket';
  try {
    const filesResp: FileListResponse = await client.getFiles(bucketId);
    const first: FileEntry | undefined = filesResp.files[0];
    console.log('first entry name:', first?.name ?? 'none');
  } catch (e) {
    console.log('getFiles requires auth; skipping in types demo');
  }

  // Upload options compile-time check
  const uploadOptions: UploadOptions = {
    priority: 'normal',
    mspDistribution: false,
  } as const;
  void uploadOptions;

  console.log('types import and API surface validated.');
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});


