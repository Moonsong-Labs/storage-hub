import { createReadStream, statSync } from 'node:fs';
import { Readable } from 'node:stream';
import { HttpClient, LocalWallet, FileManager, initWasm } from '@storagehub-sdk/core';

async function main() {
  await initWasm();
  const baseUrl = process.env.BASE_URL || 'http://127.0.0.1:8080';
  const http = new HttpClient({ baseUrl });
  const health = await http.get('/health');
  console.log('health:', health);

  // Local wallet (testing key)
  const TEST_PK =
    '0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80';
  const wallet = LocalWallet.fromPrivateKey(TEST_PK);
  const address = await wallet.getAddress();
  const sig = await wallet.signMessage('hello from core-demo');
  console.log('wallet address:', address);
  console.log('signature:', sig);

  // 3) FileManager fingerprint (convert Node stream to Web stream)
  const filePath = new URL('./data/hello.txt', import.meta.url);
  const size = statSync(filePath).size;
  const nodeStream = createReadStream(filePath);
  const webStream = Readable.toWeb(nodeStream);

  const fm = new FileManager({ size, stream: () => webStream });
  const fingerprint = await fm.getFingerprint();
  console.log('fingerprint (H256 hex):', fingerprint.toHex());
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});


