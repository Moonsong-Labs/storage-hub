import assert from "node:assert";
import { createReadStream, statSync } from "node:fs";
import { Readable } from "node:stream";
import { createPublicClient, createWalletClient, defineChain, http } from "viem";
import { privateKeyToAccount } from "viem/accounts";
import { describeBspNet, type EnrichedBspApi, ShConsts } from "../../../util";
import { SH_EVM_SOLOCHAIN_CHAIN_ID } from "../../../util/bspNet/consts";
import { ALITH_PRIVATE_KEY } from "../../../util/evmNet/keyring";
import {
  StorageHubClient,
  FileManager,
  ReplicationLevel
} from "../../../../sdk/core/dist/index.node.js";

// Helper function to compute file fingerprint using FileManager (Merkle trie root)
const computeFileFingerprint = async (filePath: string): Promise<`0x${string}`> => {
  const stats = statSync(filePath);
  const nodeStream = createReadStream(filePath);
  const webStream = Readable.toWeb(nodeStream);

  const fm = new FileManager({
    size: stats.size,
    stream: () => webStream as ReadableStream<Uint8Array>
  });

  const fingerprint = await fm.getFingerprint();
  return fingerprint.toHex() as `0x${string}`;
};

await describeBspNet(
  "Solochain EVM SDK Precompiles Integration",
  {
    initialised: false,
    networkConfig: "standard",
    runtimeType: "solochain",
    keepAlive: true,
    indexer: true /*backend: true*/
  },
  ({ before, it, createUserApi }) => {
    let userApi: EnrichedBspApi;
    let storageHubClient: InstanceType<typeof StorageHubClient>;
    let publicClient: ReturnType<typeof createPublicClient>;
    let walletClient: ReturnType<typeof createWalletClient>;
    let account: ReturnType<typeof privateKeyToAccount>;
    let bucketId: string;
    let bucketName: string;

    before(async () => {
      userApi = await createUserApi();

      // Set up StorageHub client using viem (same as the reference test)
      const rpcUrl = `http://127.0.0.1:${ShConsts.NODE_INFOS.user.port}`;

      const chain = defineChain({
        id: SH_EVM_SOLOCHAIN_CHAIN_ID,
        name: "SH-EVM_SOLO",
        nativeCurrency: { name: "StorageHub", symbol: "SH", decimals: 18 },
        rpcUrls: { default: { http: [rpcUrl] } }
      });

      account = privateKeyToAccount(ALITH_PRIVATE_KEY);
      walletClient = createWalletClient({ chain, account, transport: http(rpcUrl) });
      publicClient = createPublicClient({ chain, transport: http(rpcUrl) });

      storageHubClient = new StorageHubClient({
        rpcUrl,
        chain,
        walletClient
      });
    });

    // Create bucket
    it("should create bucket using StorageHubClient", async () => {
      bucketName = "sdk-precompiles-test-bucket";

      console.log(`[TEST] Creating bucket: ${bucketName}`);

      const valueProps = await userApi.call.storageProvidersApi.queryValuePropositionsForMsp(
        userApi.shConsts.DUMMY_MSP_ID
      );

      assert(valueProps.length > 0, "No value propositions found for MSP");
      assert(valueProps[0].id, "Value proposition ID is undefined");
      const valuePropId = valueProps[0].id.toHex();
      console.log(`[TEST] Using Value Prop ID: ${valuePropId}`);

      // Create bucket using SDK
      const txHash = await storageHubClient.createBucket(
        userApi.shConsts.DUMMY_MSP_ID as `0x${string}`,
        bucketName,
        false, // not private
        valuePropId as `0x${string}`
      );

      console.log(`[TEST] Create bucket tx sent: ${txHash}`);

      // Manual sealing is enabled; mine a block so the tx gets included
      await userApi.block.seal();

      const receipt = await publicClient.waitForTransactionReceipt({ hash: txHash });
      assert(receipt.status === "success", "Create bucket transaction failed");

      // Store bucket ID for subsequent tests
      bucketId = (await storageHubClient.deriveBucketId(account.address, bucketName)) as string;

      console.log(`[TEST] ✅ Bucket created successfully! TxHash: ${txHash}`);
      console.log(`[TEST] ✅ Bucket ID: ${bucketId}`);
    });

    // Issue storage request to upload file
    it("should issue storage request for Adolphus.jpg using StorageHubClient", async () => {
      assert(bucketId, "Bucket must be created first");

      console.log("[TEST] Computing fingerprint for Adolphus.jpg...");
      const testFilePath = new URL("../../../../docker/resource/adolphus.jpg", import.meta.url)
        .pathname;
      const fileLocation = "/test/adolphus.jpg";

      const fingerprint = await computeFileFingerprint(testFilePath);
      const fileStats = statSync(testFilePath);
      const fileSize = BigInt(fileStats.size);

      console.log("[TEST] ✅ Fingerprint computed successfully!");
      console.log(`[TEST] File: ${testFilePath}`);
      console.log(`[TEST] Fingerprint: ${fingerprint}`);
      console.log(`[TEST] File size: ${fileSize} bytes`);

      console.log("[TEST] Issuing storage request...");
      // TODO: if the owner of the file wants to perform the distribute, the peerId must be provided
      // At the moment, we rely on the MSP to distribute the file to BSPs
      const peerIds = [
        userApi.shConsts.NODE_INFOS.msp1.expectedPeerId // MSP peer ID
      ];
      const replicationLevel = ReplicationLevel.Basic;
      const replicas = 0; // Used only when ReplicationLevel = Custom

      // Issue storage request using SDK
      const txHash = await storageHubClient.issueStorageRequest(
        bucketId as `0x${string}`,
        fileLocation,
        fingerprint,
        fileSize,
        userApi.shConsts.DUMMY_MSP_ID as `0x${string}`,
        peerIds,
        replicationLevel,
        replicas
      );

      console.log(`[TEST] ✅ Storage request tx sent: ${txHash}`);

      // Manual sealing is enabled; mine a block so the tx gets included
      await userApi.block.seal();

      const receipt = await publicClient.waitForTransactionReceipt({ hash: txHash });
      assert(receipt.status === "success", "Storage request transaction failed");

      console.log(`[TEST] ✅ Storage request issued successfully! TxHash: ${txHash}`);
    });
  }
);
