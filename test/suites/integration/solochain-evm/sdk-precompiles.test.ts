// @ts-nocheck - SDK dependencies are not available during general typecheck in CI
import assert, { strictEqual } from "node:assert";
import { createReadStream, statSync } from "node:fs";
import { Readable } from "node:stream";
import { TypeRegistry } from "@polkadot/types";
import type { AccountId20, H256 } from "@polkadot/types/interfaces";
import {
  FileManager,
  type HttpClientConfig,
  ReplicationLevel,
  StorageHubClient
} from "@storagehub-sdk/core";
import { MspClient } from "@storagehub-sdk/msp-client";
import { createPublicClient, createWalletClient, defineChain, http } from "viem";
import { privateKeyToAccount } from "viem/accounts";
import { describeMspNet, type EnrichedBspApi, ShConsts } from "../../../util";
import { SH_EVM_SOLOCHAIN_CHAIN_ID } from "../../../util/evmNet/consts";
import { ALITH_PRIVATE_KEY } from "../../../util/evmNet/keyring";

await describeMspNet(
  "Solochain EVM SDK Precompiles Integration",
  {
    initialised: false,
    runtimeType: "solochain",
    indexer: true,
    backend: true
  },
  ({ before, it, createUserApi, createMsp1Api }) => {
    let userApi: EnrichedBspApi;
    let msp1Api: EnrichedBspApi;
    let storageHubClient: InstanceType<typeof StorageHubClient>;
    let publicClient: ReturnType<typeof createPublicClient>;
    let walletClient: ReturnType<typeof createWalletClient>;
    let account: ReturnType<typeof privateKeyToAccount>;
    let bucketId: string;
    let fileManager: FileManager;
    let fileKey: H256;
    let fileLocation: string;
    let mspClient: MspClient;

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
        walletClient
      });

      // Set up the FileManager instance for the file to manipulate
      const testFilePath = new URL("../../../../docker/resource/adolphus.jpg", import.meta.url)
        .pathname;
      const testFileSize = statSync(testFilePath).size;
      fileManager = new FileManager({
        size: testFileSize,
        stream: () => Readable.toWeb(createReadStream(testFilePath)) as ReadableStream<Uint8Array>
      });
      fileLocation = "/test/adolphus.jpg";

      // Set up the MspClient instance to connect to the MSP's backend
      // TODO: We should have the backend info somewhere in the consts
      const mspBackendHttpConfig: HttpClientConfig = {
        baseUrl: "http://127.0.0.1:8080"
      };
      mspClient = await MspClient.connect(mspBackendHttpConfig);

      // Wait for the backend to be ready
      await userApi.docker.waitForLog({
        containerName: "storage-hub-sh-backend-1",
        searchString: "Server listening on",
        timeout: 10000
      });

      // Ensure the connection works
      const healthResponse = await mspClient.getHealth();
      assert(healthResponse.status === "healthy", "MSP health response should be healthy");

      // Set up the authentication with the MSP backend
      const chainId = SH_EVM_SOLOCHAIN_CHAIN_ID;
      const { message } = await mspClient.getNonce(account.address, chainId);
      const signature = await walletClient.signMessage({ account, message });
      const verified = await mspClient.verify(message, signature);
      mspClient.setToken(verified.token);
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

    it("Should create a new bucket using the SDK's StorageHubClient", async () => {
      const bucketName = "sdk-precompiles-test-bucket";

      const valueProps = await userApi.call.storageProvidersApi.queryValuePropositionsForMsp(
        userApi.shConsts.DUMMY_MSP_ID
      );

      assert(valueProps.length > 0, "No value propositions found for MSP");
      assert(valueProps[0].id, "Value proposition ID is undefined");
      const valuePropId = valueProps[0].id.toHex();

      // Calculate and store the bucket ID for subsequent tests
      bucketId = (await storageHubClient.deriveBucketId(account.address, bucketName)) as string;

      // Verify the bucket doesn't exist before creation
      const bucketBeforeCreation = await userApi.query.providers.buckets(bucketId);
      assert(bucketBeforeCreation.isEmpty, "Bucket should not exist before creation");

      // Create the bucket using the SDK
      const txHash = await storageHubClient.createBucket(
        userApi.shConsts.DUMMY_MSP_ID as `0x${string}`,
        bucketName,
        false,
        valuePropId as `0x${string}`
      );

      // Check that the tx is in the mempool
      await userApi.wait.waitForTxInPool({
        module: "ethereum",
        method: "transact"
      });

      // Seal the block so the tx gets included
      await userApi.block.seal();

      const receipt = await publicClient.waitForTransactionReceipt({ hash: txHash });
      assert(receipt.status === "success", "Create bucket transaction failed");

      // Verify bucket exists after creation
      const bucketAfterCreation = await userApi.query.providers.buckets(bucketId);
      assert(!bucketAfterCreation.isEmpty, "Bucket should exist after creation");
      const bucketData = bucketAfterCreation.unwrap();
      assert(
        bucketData.userId.toString() === account.address,
        "Bucket userId should match account address"
      );
      assert(
        bucketData.mspId.toString() === userApi.shConsts.DUMMY_MSP_ID,
        "Bucket mspId should match expected MSP ID"
      );
    });

    it("Should issue a storage request for Adolphus.jpg using the SDK's StorageHubClient", async () => {
      // Get the file info
      const fingerprint = await fileManager.getFingerprint();
      const fileSize = BigInt(fileManager.getFileSize());

      // Rely on the MSP to distribute the file to BSPs
      const peerIds = [
        userApi.shConsts.NODE_INFOS.msp1.expectedPeerId // MSP peer ID
      ];
      const replicationLevel = ReplicationLevel.Basic;
      const replicas = 0; // Used only when ReplicationLevel = Custom

      // Issue the storage request using the SDK
      const txHash = await storageHubClient.issueStorageRequest(
        bucketId as `0x${string}`,
        fileLocation,
        fingerprint.toHex() as `0x${string}`,
        fileSize,
        userApi.shConsts.DUMMY_MSP_ID as `0x${string}`,
        peerIds,
        replicationLevel,
        replicas
      );

      // Check that the tx is in the mempool
      await userApi.wait.waitForTxInPool({
        module: "ethereum",
        method: "transact"
      });

      // Seal the block so the tx gets included
      await userApi.block.seal();

      const receipt = await publicClient.waitForTransactionReceipt({ hash: txHash });
      assert(receipt.status === "success", "Storage request transaction failed");

      // Compute the file key
      const registry = new TypeRegistry();
      const owner = registry.createType("AccountId20", account.address) as AccountId20;
      const bucketIdH256 = registry.createType("H256", bucketId) as H256;
      fileKey = await fileManager.computeFileKey(owner, bucketIdH256, fileLocation);

      // Check that the storage request exists on chain
      const storageRequest = await userApi.query.fileSystem.storageRequests(fileKey);
      assert(storageRequest.isSome, "Storage request not found on chain");
      const storageRequestData = storageRequest.unwrap();
      strictEqual(
        storageRequestData.bucketId.toString(),
        bucketId,
        "Storage request bucketId should match expected bucketId"
      );
      strictEqual(
        storageRequestData.location.toUtf8(),
        fileLocation,
        "Storage request location should match expected location"
      );
      strictEqual(
        storageRequestData.fingerprint.toString(),
        fingerprint.toString(),
        "Storage request fingerprint should match expected fingerprint"
      );
      strictEqual(
        storageRequestData.size_.toString(),
        fileSize.toString(),
        "Storage request fileSize should match expected fileSize"
      );
    });

    it("Should upload the file to the MSP through the backend using the SDK's StorageHubClient", async () => {
      // Try to upload the file to the MSP through the SDK's MspClient that uses the MSP backend
      const uploadResponse = await mspClient.uploadFile(
        bucketId,
        fileKey.toHex(),
        await fileManager.getFileBlob(),
        account.address,
        fileLocation
      );

      // Check that the upload was successful
      strictEqual(uploadResponse.status, "upload_successful", "Upload should return success");
      strictEqual(
        `0x${uploadResponse.fileKey}`,
        fileKey.toHex(),
        "Upload should return expected file key"
      );
      strictEqual(
        `0x${uploadResponse.bucketId}`,
        bucketId,
        "Upload should return expected bucket ID"
      );
      strictEqual(
        `0x${uploadResponse.fingerprint}`,
        (await fileManager.getFingerprint()).toString(),
        "Upload should return expected fingerprint"
      );
      strictEqual(uploadResponse.location, fileLocation, "Upload should return expected location");

      // Wait until the MSP has received and stored the file
      const hexFileKey = fileKey.toHex();
      await msp1Api.wait.fileStorageComplete(hexFileKey);

      // Make sure the accept transaction from the MSP is in the tx pool
      await userApi.wait.mspResponseInTxPool(1);

      // Seal the block containing the MSP's acceptance
      await userApi.block.seal();

      // Check that there's a `MspAcceptedStorageRequest` event
      const mspAcceptedStorageRequestEvent = await userApi.assert.eventPresent(
        "fileSystem",
        "MspAcceptedStorageRequest"
      );

      // Get its file key
      let mspAcceptedStorageRequestDataBlob: any;
      if (mspAcceptedStorageRequestEvent) {
        mspAcceptedStorageRequestDataBlob =
          userApi.events.fileSystem.MspAcceptedStorageRequest.is(
            mspAcceptedStorageRequestEvent.event
          ) && mspAcceptedStorageRequestEvent.event.data;
      }
      const acceptedFileKey = mspAcceptedStorageRequestDataBlob.fileKey.toString();
      assert(acceptedFileKey, "MspAcceptedStorageRequest event were found");

      // The file key accepted by the MSP should be the same as the one uploaded
      assert(
        hexFileKey === acceptedFileKey,
        "File key accepted by the MSP should be the same as the one uploaded"
      );

      // Ensure the file is now stored in the MSP's file storage
      await msp1Api.wait.fileStorageComplete(hexFileKey);
    });

    it("Should fetch payment streams using the SDK's MspClient", async () => {
      // Get on-chain information for payment streams
      const mspId = userApi.shConsts.DUMMY_MSP_ID;
      const maybeOnChain = await userApi.query.paymentStreams.fixedRatePaymentStreams(
        mspId,
        account.address
      );
      assert(maybeOnChain.isSome, "On-chain fixed-rate payment stream not found");
      const onChain = maybeOnChain.unwrap();

      // Retrieve payment streams for the authenticated using the SDK
      const { streams } = await mspClient.getPaymentStreams();
      const sdkPs = streams.find((s) => s.provider.toLowerCase() === mspId.toLowerCase());
      assert(sdkPs, "SDK did not return a payment stream for the expected MSP");

      strictEqual(sdkPs.providerType, "msp", "providerType should be 'msp'");
      strictEqual(
        sdkPs.costPerTick,
        onChain.rate.toString(),
        "costPerTick must match on-chain rate"
      );
    });

    it("Should download the file from the MSP through the backend using the SDK's MspClient", async () => {
      // Try to download the file from the MSP through the SDK's MspClient that uses the MSP backend
      const downloadResponse = await mspClient.downloadByKey(fileKey.toHex());

      // Check that the download was successful
      strictEqual(downloadResponse.status, 200, "Download should return success");

      // Get the download file and load it into memory as a blob
      const downloadFileBlob = await new Response(downloadResponse.stream).blob();

      // Check that the file is the same as the one uploaded, converting both blobs to a comparable format
      assert(
        Buffer.from(await downloadFileBlob.arrayBuffer()).equals(
          Buffer.from(await (await fileManager.getFileBlob()).arrayBuffer())
        ),
        "File should be the same as the one uploaded"
      );
    });
  }
);
