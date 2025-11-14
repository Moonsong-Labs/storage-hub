// @ts-nocheck - SDK dependencies are not available during general typecheck in CI
import assert, { strictEqual } from "node:assert";
import { createReadStream } from "node:fs";
import { writeFile } from "node:fs/promises";
import { Readable } from "node:stream";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { TypeRegistry } from "@polkadot/types";
import type { AccountId20, H256 } from "@polkadot/types/interfaces";
import {
  FileManager,
  type HttpClientConfig,
  ReplicationLevel,
  SH_FILE_SYSTEM_PRECOMPILE_ADDRESS,
  StorageHubClient
} from "@storagehub-sdk/core";
import { MspClient } from "@storagehub-sdk/msp-client";
import { createPublicClient, createWalletClient, defineChain, http } from "viem";
import { privateKeyToAccount } from "viem/accounts";
import { describeMspNet, type EnrichedBspApi, ShConsts } from "../../util";
import { SH_EVM_SOLOCHAIN_CHAIN_ID } from "../../util/evmNet/consts";
import { ALITH_PRIVATE_KEY } from "../../util/evmNet/keyring";

await describeMspNet(
  "SDK Big Files Performance",
  {
    initialised: false,
    runtimeType: "solochain",
    indexer: true,
    backend: true,
    fisherman: true,
    networkConfig: "standard"
  },
  ({ before, it, createUserApi, createMsp1Api }) => {
    const perfRows: Array<{
      file: string;
      sizeMB: number;
      fingerprintMs: number;
      uploadMs: number;
      downloadMs: number;
    }> = [];
    let userApi: EnrichedBspApi;
    let msp1Api: EnrichedBspApi;
    let storageHubClient: InstanceType<typeof StorageHubClient>;
    let publicClient: ReturnType<typeof createPublicClient>;
    let walletClient: ReturnType<typeof createWalletClient>;
    let account: ReturnType<typeof privateKeyToAccount>;
    let bucketId: string;

    let mspClient: MspClient;
    let sessionToken: string | undefined;
    const sessionProvider = async () =>
      sessionToken ? ({ token: sessionToken, user: { address: "" } } as const) : undefined;

    before(async () => {
      userApi = await createUserApi();
      const maybeMsp1Api = await createMsp1Api();
      if (maybeMsp1Api) {
        msp1Api = maybeMsp1Api;
      } else {
        throw new Error("MSP API for first MSP not available");
      }

      // Set up the StorageHub SDK client using viem
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
        walletClient,
        filesystemContractAddress: SH_FILE_SYSTEM_PRECOMPILE_ADDRESS
      });

      const mspBackendHttpConfig: HttpClientConfig = {
        baseUrl: "http://127.0.0.1:8080",
        // Large files need longer than the HttpClient default (30s)
        timeoutMs: 900_000
      };
      mspClient = await MspClient.connect(mspBackendHttpConfig, sessionProvider);

      // Wait for the backend to be ready
      await userApi.docker.waitForLog({
        containerName: "storage-hub-sh-backend-1",
        searchString: "Server listening",
        timeout: 10000
      });

      // Ensure the connection works
      const healthResponse = await mspClient.info.getHealth();
      assert(healthResponse.status === "healthy", "MSP health response should be healthy");

      // Set up the authentication with the MSP backend
      const siweSession = await mspClient.auth.SIWE(walletClient);
      sessionToken = siweSession.token;
    });

    it("Postgres DB is ready", async () => {
      await userApi.docker.waitForLog({
        containerName: "storage-hub-sh-postgres-1",
        searchString: "database system is ready to accept connections",
        timeout: 10000
      });
    });

    it("Network launches and can be queried", async () => {
      const userNodePeerId = await userApi.rpc.system.localPeerId();
      strictEqual(userNodePeerId.toString(), userApi.shConsts.NODE_INFOS.user.expectedPeerId);

      const mspNodePeerId = await msp1Api.rpc.system.localPeerId();
      strictEqual(mspNodePeerId.toString(), userApi.shConsts.NODE_INFOS.msp1.expectedPeerId);
    });

    // Create a dedicated bucket for perf tests
    before(async () => {
      const bucketName = "perf-bucket";
      const valueProps = await userApi.call.storageProvidersApi.queryValuePropositionsForMsp(
        userApi.shConsts.DUMMY_MSP_ID
      );
      assert(valueProps.length > 0, "No value propositions found for MSP");
      const valuePropId = valueProps[0].id.toHex();

      bucketId = (await storageHubClient.deriveBucketId(account.address, bucketName)) as string;
      const beforeBucket = await userApi.query.providers.buckets(bucketId);
      if (beforeBucket.isNone) {
        const txHash = await storageHubClient.createBucket(
          userApi.shConsts.DUMMY_MSP_ID as `0x${string}`,
          bucketName,
          false,
          valuePropId as `0x${string}`
        );
        await userApi.wait.waitForTxInPool({ module: "ethereum", method: "transact" });
        await userApi.block.seal();
        const receipt = await publicClient.waitForTransactionReceipt({ hash: txHash });
        assert(receipt.status === "success", "Create bucket transaction failed");
      }
      const afterBucket = await userApi.query.providers.buckets(bucketId);
      assert(afterBucket.isSome, "Perf bucket should exist");
    });

    function makeInMemoryWebStream(
      totalBytes: number,
      chunkBytes: number
    ): ReadableStream<Uint8Array> {
      const size = Math.max(1, Math.min(chunkBytes, totalBytes));
      const chunk = new Uint8Array(size);
      chunk.fill(0xaa);
      let remaining = totalBytes;
      return new ReadableStream<Uint8Array>({
        pull(controller) {
          if (remaining <= 0) {
            controller.close();
            return;
          }
          const take = Math.min(chunk.length, remaining);
          controller.enqueue(take === chunk.length ? chunk : chunk.subarray(0, take));
          remaining -= take;
        }
      });
    }

    // Encapsulated (disabled) steps: upload and download
    async function _issueStorageRequestAndUpload(
      filePath: string,
      size: number,
      mgr: FileManager,
      fingerprint: H256,
      fileLocationPerf: string
    ): Promise<{ uploadMs: number; fileKeyHex: string }> {
      const READ_HIGH_WATERMARK_BYTES = 128 * 1024 * 1024;
      const readOpts = { highWaterMark: READ_HIGH_WATERMARK_BYTES };
      const registry = new TypeRegistry();
      const owner = registry.createType("AccountId20", account.address) as AccountId20;
      const bucketIdH256 = registry.createType("H256", bucketId) as H256;
      const fileKeyPerf = await mgr.computeFileKey(owner, bucketIdH256, fileLocationPerf);
      const fileSizeBig = BigInt(size);
      const peerIds = [userApi.shConsts.NODE_INFOS.msp1.expectedPeerId];

      const t2 =
        typeof performance !== "undefined" && typeof performance.now === "function"
          ? performance.now()
          : Date.now();
      const txHash = await storageHubClient.issueStorageRequest(
        bucketId as `0x${string}`,
        fileLocationPerf,
        fingerprint.toHex() as `0x${string}`,
        fileSizeBig,
        userApi.shConsts.DUMMY_MSP_ID as `0x${string}`,
        peerIds,
        ReplicationLevel.Basic,
        0
      );
      await userApi.wait.waitForTxInPool({ module: "ethereum", method: "transact" });
      await userApi.block.seal();
      const receipt = await publicClient.waitForTransactionReceipt({ hash: txHash });
      assert(receipt.status === "success", "Storage request transaction failed");

      // Wait until MSP is expecting this file key (RPC guard in backend relies on this)
      {
        const hexKey = fileKeyPerf.toHex();
        const maxWaitMs = 30_000;
        const stepMs = 500;
        let ok = false;
        for (let waited = 0; waited <= maxWaitMs; waited += stepMs) {
          try {
            // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment
            const expected = await (msp1Api as any).rpc.storagehubclient.isFileKeyExpected(hexKey);
            if (expected === true || (expected && expected.isTrue === true)) {
              ok = true;
              break;
            }
          } catch {}
          await new Promise((r) => setTimeout(r, stepMs));
        }
        assert(ok, "MSP did not register expected file key in time");
      }

      const uploadResp = await mspClient.files.uploadFileFastStream(
        bucketId,
        fileKeyPerf.toHex(),
        Readable.toWeb(createReadStream(filePath, readOpts)) as ReadableStream<Uint8Array>,
        account.address,
        fileLocationPerf,
        { precomputed: { fingerprint: fingerprint.toU8a(), fileSize: size } }
      );
      assert.equal(uploadResp.status, "upload_successful");

      const hexKey = fileKeyPerf.toHex();
      await msp1Api.wait.fileStorageComplete(hexKey);
      await userApi.wait.mspResponseInTxPool(1);
      await userApi.block.seal();
      const t3 =
        typeof performance !== "undefined" && typeof performance.now === "function"
          ? performance.now()
          : Date.now();
      const uploadMs = Math.round(t3 - t2);
      return { uploadMs, fileKeyHex: hexKey };
    }

    async function _downloadAndMeasure(
      fileKeyHex: string
    ): Promise<{ downloadMs: number; downloadBlob: Blob }> {
      const t4 =
        typeof performance !== "undefined" && typeof performance.now === "function"
          ? performance.now()
          : Date.now();
      const downloadResponse = await mspClient.files.downloadFile(fileKeyHex);
      assert.equal(downloadResponse.status, 200);
      const downloadBlob = await new Response(downloadResponse.stream).blob();
      const t5 =
        typeof performance !== "undefined" && typeof performance.now === "function"
          ? performance.now()
          : Date.now();
      const downloadMs = Math.round(t5 - t4);
      return { downloadMs, downloadBlob };
    }

    // Generic perf test: fingerprint, upload (issue+upload), download timings
    it("Perf timings across file sizes: fingerprint only", async () => {
      const PERF_SIZES_MB = [10, 50, 100, 500, 1024];
      for (const sizeMB of PERF_SIZES_MB) {
        const filename = `${sizeMB}MB.bin`;
        const sizeBytes = sizeMB * 1024 * 1024;

        const READ_HIGH_WATERMARK_BYTES = 128 * 1024 * 1024;
        const mgr = new FileManager({
          size: sizeBytes,
          stream: () => makeInMemoryWebStream(sizeBytes, READ_HIGH_WATERMARK_BYTES)
        });

        // 1) Fingerprint timing
        const t0 =
          typeof performance !== "undefined" && typeof performance.now === "function"
            ? performance.now()
            : Date.now();
        const _fingerprint = await mgr.getFingerprint();
        const t1 =
          typeof performance !== "undefined" && typeof performance.now === "function"
            ? performance.now()
            : Date.now();
        const fingerprintMs = Math.round(t1 - t0);

        // Upload and download disabled; use placeholders
        const uploadMs = 0;
        const downloadMs = 0;

        // Collect row for summary table
        perfRows.push({
          file: filename,
          sizeMB: Math.round((sizeBytes / (1024 * 1024)) * 100) / 100,
          fingerprintMs,
          uploadMs,
          downloadMs
        });
      }

      const outPath = join(tmpdir(), `sdk-big-files-perf-${Date.now()}.json`);
      await writeFile(outPath, JSON.stringify(perfRows, null, 2), "utf8");
      // eslint-disable-next-line no-console
      console.log(`Perf results written to: ${outPath}`);
    });
  }
);
