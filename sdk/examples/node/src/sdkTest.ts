import process from 'node:process';
import { MspClient, type Session } from '@storagehub-sdk/msp-client';
import { createWalletClient, http } from 'viem';
import { privateKeyToAccount } from 'viem/accounts';
import { createWriteStream, mkdirSync } from 'node:fs';
import path from 'node:path';
import { Readable } from 'node:stream';

function readSeed(): string {
  const mnemonic = process.env.TEST_SEED;
  if (!mnemonic) throw new Error('TEST_SEED env var (mnemonic) is required');
  return mnemonic;
}

function createViemChain(chainId: number, rpcUrl: string) {
  return {
    id: chainId,
    name: `chain-${chainId}`,
    nativeCurrency: { name: 'Native', symbol: 'NATIVE', decimals: 18 },
    rpcUrls: { default: { http: [rpcUrl] } },
  } as const;
}

async function authenticateMspClient(
  mspBaseUrl: string,
  rpcUrl: string,
  chainId: number,
) {
  const chain = createViemChain(chainId, rpcUrl);
  const TEST_PK = "";
  const account = privateKeyToAccount(TEST_PK as `0x${string}`);
  const address = account.address as string;
  const walletClient = createWalletClient({
    account,
    chain,
    transport: http(rpcUrl),
  });

  const authSessionProvider = async (): Promise<Session> => ({
    token: '',
    user: { address },
  });
  const authClient = await MspClient.connect(
    { baseUrl: mspBaseUrl },
    authSessionProvider,
  );

  const session = await authClient.auth.SIWE(walletClient);
  const sessionProvider = async (): Promise<Session> => session;

  const mspClient = await MspClient.connect(
    { baseUrl: mspBaseUrl },
    sessionProvider,
  );

  return { mspClient, address };
}

async function main() {
  const chainId = 181222;
  const rpcUrl = `http://127.0.0.1:9888`;
  const baseUrl = "http://127.0.0.1:8080"
  const fileKey = '0x73d2fb3630d30b775a2e2ae17ca45b172eaaad0fd0cb0ab29d1f383717e523b0'
  const { mspClient, address } = await authenticateMspClient(
    baseUrl,
    rpcUrl,
    chainId,
  );
  if (address) console.log('Authenticated as:', address);

  const file = await mspClient.files.downloadFile(fileKey);
  console.log('downloadFile response object:', file);

  if (!file || !file.stream) {
    throw new Error('Failed to get file or file.stream from downloadFile');
  }

  // Prepare output directory and file path
  const OUTPUT_DIR = process.env.OUTPUT_DIR || './downloads';
  mkdirSync(OUTPUT_DIR, { recursive: true });
  const baseName = process.env.OUTPUT_FILE_BASENAME || 'downloaded_file';
  const shortKey = (fileKey.startsWith('0x') ? fileKey.slice(2) : fileKey).slice(0, 8);
  const outPath = path.join(OUTPUT_DIR, `${baseName}_${shortKey}_${Date.now()}.jpg`);

  // Convert Web ReadableStream to Node.js Readable and pipe to disk
  const nodeReadable = Readable.fromWeb(file.stream as unknown as ReadableStream);
  let totalBytes = 0;
  await new Promise<void>((resolve, reject) => {
    const ws = createWriteStream(outPath);
    nodeReadable.on('data', (chunk: Buffer) => {
      totalBytes += chunk.length;
      if (totalBytes % (1024 * 1024) < chunk.length) {
        console.log(`... written ${totalBytes} bytes ...`);
      }
    });
    nodeReadable.on('error', reject);
    ws.on('error', reject);
    ws.on('finish', () => resolve());
    nodeReadable.pipe(ws);
  });

  console.log('\n--- DOWNLOAD COMPLETE ---');
  console.log(`Saved to: ${outPath}`);
  console.log(`Total bytes written: ${totalBytes}`);
}

main()
  .then(() => process.exit(0))
  .catch((err) => {
    console.error('Download script failed:', err);
    process.exit(1);
  });
