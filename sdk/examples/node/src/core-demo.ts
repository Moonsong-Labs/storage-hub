import { createReadStream, statSync } from "node:fs";
import { Readable } from "node:stream";
import { HttpClient, LocalWallet, FileManager, initWasm } from "@storagehub-sdk/core";
import type { H256 } from "@polkadot/types/interfaces";

export async function runCoreDemo(): Promise<void> {
  await initWasm();
  const baseUrl = process.env.BASE_URL || "http://127.0.0.1:8080";
  const http = new HttpClient({ baseUrl });
  const health = await http.get("/health");
  console.log("health:", health);

  const TEST_PK = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";
  const wallet = LocalWallet.fromPrivateKey(TEST_PK);
  const address = await wallet.getAddress();
  const sig = await wallet.signMessage("hello from core-demo");
  console.log("wallet address:", address);
  console.log("signature:", sig);

  const filePath = new URL("../data/hello.txt", import.meta.url);
  const size = statSync(filePath).size;
  const nodeStream = createReadStream(filePath);
  const webStream = Readable.toWeb(nodeStream) as unknown as ReadableStream<Uint8Array>;

  const fm = new FileManager({ size, stream: () => webStream });
  const fingerprint: H256 = await fm.getFingerprint();
  console.log("fingerprint (H256 hex):", fingerprint.toHex());
}
