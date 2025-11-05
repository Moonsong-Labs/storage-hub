import path from 'node:path';
import { statSync, createReadStream } from 'node:fs';
import process from 'node:process';
import assert from 'node:assert';
import { Readable } from 'node:stream';
import { createPublicClient, createWalletClient, defineChain, http } from 'viem';
import { privateKeyToAccount } from 'viem/accounts';
import { TypeRegistry } from '@polkadot/types';
import type { AccountId20, H256 } from '@polkadot/types/interfaces';
import {
  FileManager,
  StorageHubClient,
  SH_FILE_SYSTEM_PRECOMPILE_ADDRESS,
  ReplicationLevel,
} from '@storagehub-sdk/core';
import { MspClient, type Session } from '@storagehub-sdk/msp-client';

function getEnv(name: string, fallback?: string): string {
  const v = process.env[name] ?? fallback;
  if (v === undefined) throw new Error(`${name} is required`);
  return v;
}

function extractPeerIdFromMultiaddresses(multiaddresses: string[]): string | undefined {
  for (const ma of multiaddresses) {
    const idx = ma.lastIndexOf('/p2p/');
    if (idx !== -1) {
      const tail = ma.slice(idx + 5);
      const peerId = tail.split('/')[0];
      if (peerId) return peerId;
    }
  }
  return undefined;
}

async function connectMspClients(rpcUrl: string, baseUrl: string, chainId: number) {
  const chain = defineChain({
    id: chainId,
    name: `SH-${chainId}`,
    nativeCurrency: { name: 'SH', symbol: 'SH', decimals: 18 },
    rpcUrls: { default: { http: [rpcUrl] } },
  });

  const TEST_PK = (process.env.TEST_PK ??
    '0x5fb92d6e98884f76de468fa3f6278f8807c48bebc13595d45af5bdc4da702133') as `0x${string}`;
  const account = privateKeyToAccount(TEST_PK);

  const walletClient = createWalletClient({ chain, account, transport: http(rpcUrl) });
  const publicClient = createPublicClient({ chain, transport: http(rpcUrl) });

  const authSessionProvider = async (): Promise<Session> => ({ token: '', user: { address: account.address } });
  const authClient = await MspClient.connect({ baseUrl }, authSessionProvider);
  const session = await authClient.auth.SIWE(walletClient);
  const sessionProvider = async (): Promise<Session> => session;
  const mspClient = await MspClient.connect({ baseUrl }, sessionProvider);

  const storageHubClient = new StorageHubClient({ rpcUrl, chain, walletClient, filesystemContractAddress: SH_FILE_SYSTEM_PRECOMPILE_ADDRESS });

  return { walletClient, publicClient, storageHubClient, mspClient, account } as const;
}

async function main() {
  const baseDir = path.resolve(process.cwd());
  const rpcUrl = getEnv('RPC_URL', 'http://127.0.0.1:9888');
  const baseUrl = getEnv('MSP_BASE_URL', 'http://127.0.0.1:8080');
  const chainId = Number(getEnv('CHAIN_ID', '181222'));

  // File to upload (override with FILE_PATH env)
  const defaultFilePath = path.join(baseDir, '../../../docker/resource/adolphus.jpg');
  const filePath = getEnv('FILE_PATH', defaultFilePath);
  const fileLocation = getEnv('FILE_LOCATION', '/test/adolphus.jpg');
  assert(statSync(filePath).isFile(), `File not found: ${filePath}`);

  const { publicClient, storageHubClient, mspClient, account } = await connectMspClients(rpcUrl, baseUrl, chainId);

  // Resolve MSP info and value proposition to create a bucket
  const info = await mspClient.info.getInfo();
  const mspId = getEnv('MSP_ID', info.mspId);
  const valueProps = await mspClient.info.getValuePropositions();
  assert(valueProps.length > 0, 'No value propositions found for MSP');
  const valuePropId = valueProps[0].id as `0x${string}`;

  // Ensure a bucket exists
  const bucketName = getEnv('BUCKET_NAME', 'sdk-example-bucket');
  const bucketId = (await storageHubClient.deriveBucketId(account.address, bucketName)) as string;

  // Try to create bucket (idempotent: if it exists, tx may fail; we ignore if already exists)
  try {
    const txHash = await storageHubClient.createBucket(mspId as `0x${string}`, bucketName, false, valuePropId);
    await publicClient.waitForTransactionReceipt({ hash: txHash });
  } catch (_) {
    // swallow errors if bucket already exists
  }

  // Prepare FileManager
  const fileSize = statSync(filePath).size;
  const fileManager = new FileManager({
    size: fileSize,
    stream: () => Readable.toWeb(createReadStream(filePath)) as unknown as ReadableStream<Uint8Array>,
  });

  // Issue storage request (include MSP peerId so MSP registers expectation)
  const fingerprint = await fileManager.getFingerprint();
  const peerId = extractPeerIdFromMultiaddresses(info.multiaddresses);
  const peerIds: string[] = peerId ? [peerId] : [];
  const txHash = await storageHubClient.issueStorageRequest(
    bucketId as `0x${string}`,
    fileLocation,
    fingerprint.toHex() as `0x${string}`,
    BigInt(fileSize),
    mspId as `0x${string}`,
    peerIds,
    ReplicationLevel.Basic,
    0,
  );
  await publicClient.waitForTransactionReceipt({ hash: txHash });

  // Small wait to allow MSP to process NewStorageRequest and register expectation
  await new Promise((r) => setTimeout(r, 1500));

  // Compute file key so caller can reuse it
  const registry = new TypeRegistry();
  const owner = registry.createType('AccountId20', account.address) as AccountId20;
  const bucketIdH256 = registry.createType('H256', bucketId) as H256;
  const fileKey = await fileManager.computeFileKey(owner, bucketIdH256, fileLocation);

  // Upload the file to MSP backend
  const blob = await fileManager.getFileBlob();
  // Retry upload until MSP expects the key
  const maxRetries = Number(process.env.UPLOAD_RETRIES ?? 10);
  const retryDelayMs = Number(process.env.UPLOAD_RETRY_DELAY_MS ?? 1000);
  let uploadResp: any;
  for (let attempt = 0; attempt <= maxRetries; attempt++) {
    try {
      uploadResp = await mspClient.files.uploadFile(bucketId, fileKey.toHex(), blob, account.address, fileLocation);
      break;
    } catch (e: any) {
      const msg = String(e?.body?.error || e?.message || e);
      if (msg.includes('not expecting')) {
        if (attempt === maxRetries) throw e;
        await new Promise((r) => setTimeout(r, retryDelayMs));
        continue;
      }
      throw e;
    }
  }
  assert(uploadResp.status === 'upload_successful', 'Upload should return success');

  // Print the fileKey so it can be used by the download script
  // eslint-disable-next-line no-console
  console.log('\nUploaded file successfully. Use this fileKey in sdkTest.ts:');
  // eslint-disable-next-line no-console
  console.log(fileKey.toHex());
}

main().catch((e) => {
  // eslint-disable-next-line no-console
  console.error('uploadAndPrintFileKey failed:', e);
  process.exit(1);
});


