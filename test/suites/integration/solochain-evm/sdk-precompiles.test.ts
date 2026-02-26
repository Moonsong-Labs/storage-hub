// @ts-nocheck - SDK dependencies are not available during general typecheck in CI
import assert, { strictEqual } from "node:assert";
import { createReadStream, statSync } from "node:fs";
import { Readable } from "node:stream";
import { TypeRegistry } from "@polkadot/types";
import type { AccountId20, H256 } from "@polkadot/types/interfaces";
import {
  type FileInfo,
  FileManager,
  type HttpClientConfig,
  ReplicationLevel,
  SH_FILE_SYSTEM_PRECOMPILE_ADDRESS,
  StorageHubClient
} from "@storagehub-sdk/core";
import { MspClient } from "@storagehub-sdk/msp-client";
import { createPublicClient, createWalletClient, defineChain, getAddress, http } from "viem";
import { privateKeyToAccount } from "viem/accounts";
import {
  describeMspNet,
  type EnrichedBspApi,
  ShConsts,
  type SqlClient,
  waitFor
} from "../../../util";
import type { StatsResponse } from "../../../util/backend/types";
import { SH_EVM_SOLOCHAIN_CHAIN_ID } from "../../../util/evmNet/consts";
import { ALITH_PRIVATE_KEY } from "../../../util/evmNet/keyring";
import { fileURLToPath } from "node:url";

await describeMspNet(
  "Solochain EVM SDK Precompiles Integration",
  {
    initialised: false,
    runtimeType: "solochain",
    indexer: true,
    backend: true,
    fisherman: true,
    standaloneIndexer: true
  },
  ({ before, it, createUserApi, createMsp1Api, createSqlClient, createIndexerApi }) => {
    let userApi: EnrichedBspApi;
    let msp1Api: EnrichedBspApi;
    let indexerApi: EnrichedBspApi;
    let storageHubClient: InstanceType<typeof StorageHubClient>;
    let publicClient: ReturnType<typeof createPublicClient>;
    let walletClient: ReturnType<typeof createWalletClient>;
    let account: ReturnType<typeof privateKeyToAccount>;
    let sql: SqlClient;
    let bucketId: string;
    let fileManager: FileManager;
    let fileKey: H256;
    let storageRequestBlockHash: `0x${string}`;
    let storageRequestTxHash: `0x${string}`;
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
      sql = createSqlClient();

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

      // Set up the FileManager instance for the file to manipulate
      const testFilePath = fileURLToPath(
        new URL("../../../../docker/resource/adolphus.jpg", import.meta.url)
      );
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
      // Create MspClient without session provider initially
      mspClient = await MspClient.connect(mspBackendHttpConfig);

      // Wait for the backend to be ready
      await userApi.docker.waitForLog({
        containerName: userApi.shConsts.NODE_INFOS.backend.containerName,
        searchString: "Server listening",
        timeout: 10000
      });

      // Ensure the connection works
      const healthResponse = await mspClient.info.getHealth();
      assert(healthResponse.status === "healthy", "MSP health response should be healthy");

      // Set up the authentication with the MSP backend
      const siweDomain = "localhost:3000";
      const siweUri = "http://localhost:3000";
      const siweSession = await mspClient.auth.SIWE(walletClient, siweDomain, siweUri);

      // Set the session provider after authentication
      mspClient.setSessionProvider(async () => siweSession);

      assert(createIndexerApi, "Indexer API not available");
      indexerApi = await createIndexerApi();
      await indexerApi.indexer.waitForIndexing({ producerApi: userApi, sql });
    });

    it("Postgres DB is ready", async () => {
      await userApi.docker.waitForLog({
        containerName: userApi.shConsts.NODE_INFOS.indexerDb.containerName,
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

    it("Should fetch authenticated user profile from the MSP backend", async () => {
      const profile = await mspClient.auth.getProfile();
      // Compare using EIP-55 checksum-normalized addresses
      strictEqual(
        profile.address,
        getAddress(account.address),
        "Profile address should be checksummed and match wallet address"
      );
    });

    it("Should authenticate using SIWX (CAIP-122) flow", async () => {
      // Create a new client without sessionProvider to test SIWX flow
      const mspBackendHttpConfig: HttpClientConfig = {
        baseUrl: "http://127.0.0.1:8080"
      };
      const siwxClient = await MspClient.connect(mspBackendHttpConfig);

      // Authenticate using SIWX (CAIP-122) - no domain parameter needed
      const siwxUri = "http://localhost:3000";
      const siwxSession = await siwxClient.auth.SIWX(walletClient, siwxUri);

      // Verify we got a session token
      assert(siwxSession.token, "SIWX should return a session token");
      assert(siwxSession.user, "SIWX should return user info");

      // Update the client's sessionProvider with the new session
      siwxClient.setSessionProvider(async () => siwxSession);

      // Verify authentication works by fetching profile
      const profile = await siwxClient.auth.getProfile();
      strictEqual(
        profile.address,
        getAddress(account.address),
        "SIWX profile address should match wallet address"
      );
    });

    it("Should get MSP general info via the SDK's MspClient", async () => {
      const info = await mspClient.info.getInfo();
      // TODO: Backend /info is mocked in msp.rs; assert exact fields to sanity-check wiring.
      // When backend returns dynamic values, relax these assertions.

      // client/version
      strictEqual(info.client, "storagehub-node v1.0.0", "Client should match backend mock");
      strictEqual(info.version, "StorageHub MSP v0.1.0", "Version should match backend mock");

      // mspId must match the launched DUMMY_MSP_ID
      strictEqual(
        info.mspId.toLowerCase(),
        userApi.shConsts.DUMMY_MSP_ID.toLowerCase(),
        "mspId should match expected test MSP"
      );

      // multiaddresses shape
      assert(
        Array.isArray(info.multiaddresses) && info.multiaddresses.length > 0,
        "multiaddresses should be present"
      );
      assert(
        info.multiaddresses.every(
          (ma: string) =>
            typeof ma === "string" &&
            ma.includes(`/p2p/${userApi.shConsts.NODE_INFOS.msp1.expectedPeerId}`)
        ),
        "Every multiaddress should contain the expected MSP peer ID"
      );

      // accounts (EIP-55 checksummed constants)
      strictEqual(
        info.ownerAccount,
        getAddress("0xf24FF3a9CF04c71Dbc94D0b566f7A27B94566cac"),
        "ownerAccount should match backend mock"
      );
      strictEqual(
        info.paymentAccount,
        getAddress("0xf24FF3a9CF04c71Dbc94D0b566f7A27B94566cac"),
        "paymentAccount should match backend mock"
      );

      // status/timing
      strictEqual(info.status, "active", "Status should be 'active'");
      strictEqual(info.activeSince, 123, "ActiveSince should match backend mock");
      assert(
        typeof info.uptime === "string" && info.uptime.length > 0,
        "uptime should be a non-empty string"
      );
    });

    it("Should get MSP stats via the SDK's MspClient", async () => {
      // Get MSP info from chain to compare with backend stats
      const mspId = userApi.shConsts.DUMMY_MSP_ID;
      const mspInfoOption = await userApi.query.providers.mainStorageProviders(mspId);
      assert(mspInfoOption.isSome, "MSP should exist on chain");
      const mspInfo = mspInfoOption.unwrap();

      // Get active users count via runtime API
      const activeUsersList =
        await userApi.call.paymentStreamsApi.getUsersOfPaymentStreamsOfProvider(mspId);
      const activeUsersCount = activeUsersList.length;

      // Get stats from backend via SDK
      const stats = (await mspClient.info.getStats()) as StatsResponse;

      // Verify capacity values match chain data
      strictEqual(
        stats.capacity.totalBytes,
        mspInfo.capacity.toString(),
        "MSP total capacity should match on-chain data"
      );
      strictEqual(
        stats.capacity.usedBytes,
        mspInfo.capacityUsed.toString(),
        "MSP used capacity should match on-chain data"
      );
      strictEqual(
        stats.capacity.availableBytes,
        (mspInfo.capacity.toBigInt() - mspInfo.capacityUsed.toBigInt()).toString(),
        "MSP available capacity should match calculated value (total - used)"
      );
      strictEqual(
        stats.bucketsAmount,
        mspInfo.amountOfBuckets.toString(),
        "MSP buckets amount should match on-chain data"
      );
      strictEqual(
        stats.activeUsers,
        activeUsersCount,
        "MSP active users should match runtime API data"
      );
      strictEqual(
        stats.lastCapacityChange,
        mspInfo.lastCapacityChange.toString(),
        "MSP last capacity change should match on-chain data"
      );
      strictEqual(
        stats.valuePropsAmount,
        mspInfo.amountOfValueProps.toString(),
        "MSP value props amount should match on-chain data"
      );
    });

    it("Should create a new bucket using the SDK's StorageHubClient", async () => {
      const bucketName = "sdk-precompiles-test-bucket";

      const valueProps = await userApi.call.storageProvidersApi.queryValuePropositionsForMsp(
        userApi.shConsts.DUMMY_MSP_ID
      );

      assert(valueProps.length > 0, "No value propositions found for MSP");
      assert(valueProps[0].id, "Value proposition ID is undefined");
      const valuePropId = valueProps[0].id.toHex();

      // Verify the selected on-chain value prop ID is present in the SDK response
      const sdkValueProps = await mspClient.info.getValuePropositions();
      assert(
        Array.isArray(sdkValueProps) && sdkValueProps.length > 0,
        "SDK value props should be present"
      );
      assert(
        sdkValueProps.some((vp) => vp.id === valuePropId),
        "SDK should include the selected on-chain value prop id"
      );

      // Calculate and store the bucket ID for subsequent tests
      bucketId = (await storageHubClient.deriveBucketId(account.address, bucketName)) as string;

      // Verify the bucket doesn't exist before creation
      const bucketBeforeCreation = await userApi.query.providers.buckets(bucketId);
      assert(bucketBeforeCreation.isNone, "Bucket should not exist before creation");

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
      assert(bucketAfterCreation.isSome, "Bucket should exist after creation");
      const bucketData = bucketAfterCreation.unwrap();
      assert(
        bucketData.userId.toString() === account.address,
        "Bucket userId should match account address"
      );
      assert(
        bucketData.mspId.toString() === userApi.shConsts.DUMMY_MSP_ID,
        "Bucket mspId should match expected MSP ID"
      );

      await indexerApi.indexer.waitForIndexing({ producerApi: userApi, sql });

      // Also verify through SDK / MSP backend endpoints
      const listedBuckets = await mspClient.buckets.listBuckets();
      assert(
        listedBuckets.some((b) => b.bucketId === bucketId),
        "MSP listBuckets should include the created bucket"
      );
      const sdkBucket = await mspClient.buckets.getBucket(bucketId);
      strictEqual(sdkBucket.bucketId, bucketId, "MSP getBucket should return the created bucket");
    });

    it("Should issue a storage request for Adolphus.jpg using the SDK's StorageHubClient", async () => {
      // Get the file info
      const fingerprint = await fileManager.getFingerprint();
      const fileSize = BigInt(fileManager.getFileSize());

      // Rely on the MSP to distribute the file to BSPs
      const peerIds = [
        userApi.shConsts.NODE_INFOS.msp1.expectedPeerId // MSP peer ID
      ];
      // Use Custom replication with 1 replica so the storage request gets fulfilled quickly
      // This allows us to test file deletion without having an active storage request
      const replicationLevel = ReplicationLevel.Custom;
      const replicas = 1;

      // Issue the storage request using the SDK
      storageRequestTxHash = await storageHubClient.issueStorageRequest(
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

      const receipt = await publicClient.waitForTransactionReceipt({ hash: storageRequestTxHash });
      assert(receipt.status === "success", "Storage request transaction failed");

      // Store the block hash where the transaction was included
      storageRequestBlockHash = receipt.blockHash;

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

      // Wait for indexer to process the storage request so the file record exists in DB
      await indexerApi.indexer.waitForIndexing({ producerApi: userApi, sql });
    });

    it("Should upload the file to the MSP through the backend using the SDK's StorageHubClient", async () => {
      // Ensure the MSP expects this file key before attempting upload to the backend
      await waitFor({
        lambda: async () => (await msp1Api.rpc.storagehubclient.isFileKeyExpected(fileKey)).isTrue
      });
      const fingerprint = await fileManager.getFingerprint();

      // Try to upload the file to the MSP through the SDK's MspClient that uses the MSP backend
      const uploadResponse = await mspClient.files.uploadFile(
        bucketId as `0x${string}`,
        fileKey.toHex() as `0x${string}`,
        await fileManager.getFileBlob(),
        fingerprint.toHex() as `0x${string}`,
        account.address as `0x${string}`,
        fileLocation
      );

      // Check that the upload was successful
      strictEqual(uploadResponse.status, "upload_successful", "Upload should return success");
      strictEqual(
        uploadResponse.fileKey,
        fileKey.toHex(),
        "Upload should return expected file key"
      );
      strictEqual(
        `0x${uploadResponse.bucketId}`,
        bucketId,
        "Upload should return expected bucket ID"
      );
      strictEqual(
        uploadResponse.fingerprint,
        fingerprint.toString(),
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

      // Wait for at least 1 BSP to confirm so the storage request gets fulfilled
      await userApi.wait.bspStored({ expectedExts: 1 });

      // Verify the storage request has been fulfilled and removed
      const storageRequestAfterConfirm = await userApi.query.fileSystem.storageRequests(fileKey);
      assert(
        storageRequestAfterConfirm.isNone,
        "Storage request should be fulfilled and removed after BSP confirms"
      );

      await indexerApi.indexer.waitForIndexing({ producerApi: userApi, sql });

      // Ensure file tree and file info are available via backend for this bucket
      const fileTree = (await mspClient.buckets.getFiles(bucketId)).tree;
      assert(
        Array.isArray(fileTree.children) && fileTree.children.length > 0,
        "file tree should not be empty"
      );
      const fileInfo = await mspClient.files.getFileInfo(bucketId, fileKey.toHex());
      strictEqual(fileInfo.bucketId, bucketId, "BucketId should match");
      strictEqual(fileInfo.fileKey, fileKey.toHex(), "FileKey should match");

      // Verify that the block hash is correctly stored and returned
      strictEqual(
        fileInfo.blockHash.toLowerCase(),
        storageRequestBlockHash.toLowerCase(),
        "File blockHash should match the block hash where the transaction was included"
      );

      // Verify that the EVM transaction hash is correctly stored and returned
      assert(fileInfo.txHash, "File should have a txHash since it was created via EVM transaction");
      strictEqual(
        fileInfo.txHash.toLowerCase(),
        storageRequestTxHash.toLowerCase(),
        "File txHash should match the EVM transaction hash that created it"
      );
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
      const { streams } = await mspClient.info.getPaymentStreams();
      const sdkPs = streams.find((s) => s.provider.toLowerCase() === mspId.toLowerCase());
      assert(sdkPs, "SDK did not return a payment stream for the expected MSP");

      strictEqual(sdkPs.providerType, "msp", "ProviderType should be 'msp'");
      strictEqual(
        sdkPs.costPerTick,
        onChain.rate.toString(),
        "costPerTick must match on-chain rate"
      );
    });

    it("Should download the file from the MSP through the backend using the SDK's MspClient", async () => {
      // Try to download the file from the MSP through the SDK's MspClient that uses the MSP backend
      const downloadResponse = await mspClient.files.downloadFile(fileKey.toHex());

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

    it("Should download the file using the direct download URL", async () => {
      const directUrl = await mspClient.files.getDownloadUrl(bucketId, fileKey.toHex());
      // Public file: direct download should work without auth headers.
      const res = await fetch(directUrl);
      strictEqual(res.status, 200, "Direct URL download should return success");

      const downloaded = Buffer.from(await res.arrayBuffer());
      const original = Buffer.from(await (await fileManager.getFileBlob()).arrayBuffer());
      assert(downloaded.equals(original), "Direct URL downloaded file should match original");
    });

    it("Should request deletion and verify complete cleanup", async () => {
      // Create the file info to request its deletion
      const registry = new TypeRegistry();
      const bucketIdH256 = registry.createType("H256", bucketId) as H256;
      const fingerprint = await fileManager.getFingerprint();
      const fileSize = BigInt(fileManager.getFileSize());

      const fileInfo: FileInfo = {
        fileKey: fileKey.toHex() as `0x${string}`,
        bucketId: bucketIdH256.toHex() as `0x${string}`,
        location: fileLocation,
        size: fileSize,
        fingerprint: fingerprint.toHex() as `0x${string}`
      };

      // Use the SDK's StorageHubClient to request the file deletion
      const txHash = await storageHubClient.requestDeleteFile(fileInfo);
      await userApi.wait.waitForTxInPool({
        module: "ethereum",
        method: "transact"
      });
      await userApi.block.seal();

      const receipt = await publicClient.waitForTransactionReceipt({ hash: txHash });
      assert(receipt.status === "success", "Request delete file transaction failed");

      // Verify the deletion request was enqueued on-chain
      await userApi.assert.eventPresent("fileSystem", "FileDeletionRequested");

      // Finalize the block on the indexer node and wait for the indexer to process the block
      await indexerApi.indexer.waitForIndexing({ producerApi: userApi, sql });

      // Wait for fisherman to process the file deletions
      await userApi.fisherman.retryableWaitAndVerifyBatchDeletions({
        blockProducerApi: userApi,
        deletionType: "User",
        expectExt: 2,
        userApi,
        mspApi: msp1Api,
        expectedBucketCount: 1,
        maxRetries: 3
      });

      // Wait until the MSP detects the on-chain deletion and updates its local bucket forest
      await msp1Api.wait.mspBucketFileDeletionCompleted(fileKey.toHex(), bucketId);

      // Non-producer nodes must explicitly finalize imported blocks to trigger file deletion
      // Producer node (user) has finalized blocks, but BSP and MSP must finalize locally
      const finalisedBlockHash = await userApi.rpc.chain.getFinalizedHead();

      await msp1Api.wait.blockImported(finalisedBlockHash.toString());
      await msp1Api.block.finaliseBlock(finalisedBlockHash.toString());

      // Wait until the MSP detects the now finalised deletion and correctly deletes the file from its file storage
      await msp1Api.wait.fileDeletionFromFileStorage(fileKey.toHex());

      // Attempt to download the file, it should fail with a 404 since the file was deleted
      const downloadResponse = await mspClient.files.downloadFile(fileKey.toHex());
      assert(
        downloadResponse.status === 404,
        "Download should fail after file deletion, but it succeeded"
      );
    });

    it("Should create, verify, delete, and verify deletion of a bucket", async () => {
      const testBucketName = "delete-bucket-test";

      const valueProps = await userApi.call.storageProvidersApi.queryValuePropositionsForMsp(
        userApi.shConsts.DUMMY_MSP_ID
      );

      assert(valueProps.length > 0, "No value propositions found for MSP");
      assert(valueProps[0].id, "Value proposition ID is undefined");
      const valuePropId = valueProps[0].id.toHex();

      const testBucketId = (await storageHubClient.deriveBucketId(
        account.address,
        testBucketName
      )) as string;

      const bucketBeforeCreation = await userApi.query.providers.buckets(testBucketId);
      assert(bucketBeforeCreation.isNone, "Test bucket should not exist before creation");

      const createTxHash = await storageHubClient.createBucket(
        userApi.shConsts.DUMMY_MSP_ID as `0x${string}`,
        testBucketName,
        false,
        valuePropId as `0x${string}`
      );

      await userApi.wait.waitForTxInPool({
        module: "ethereum",
        method: "transact"
      });
      await userApi.block.seal();

      const createReceipt = await publicClient.waitForTransactionReceipt({ hash: createTxHash });
      assert(createReceipt.status === "success", "Create bucket transaction failed");

      const bucketAfterCreation = await userApi.query.providers.buckets(testBucketId);
      assert(bucketAfterCreation.isSome, "Bucket should exist after creation");
      const bucketData = bucketAfterCreation.unwrap();
      assert(
        bucketData.userId.toString() === account.address,
        "Bucket userId should match account address"
      );
      assert(
        bucketData.mspId.toString() === userApi.shConsts.DUMMY_MSP_ID,
        "Bucket mspId should match expected MSP ID"
      );

      const deleteTxHash = await storageHubClient.deleteBucket(testBucketId as `0x${string}`);

      await userApi.wait.waitForTxInPool({
        module: "ethereum",
        method: "transact"
      });
      await userApi.block.seal();

      const deleteReceipt = await publicClient.waitForTransactionReceipt({ hash: deleteTxHash });
      assert(deleteReceipt.status === "success", "Delete bucket transaction failed");

      const bucketAfterDeletion = await userApi.query.providers.buckets(testBucketId);
      assert(bucketAfterDeletion.isNone, "Bucket should not exist after deletion");
    });
  }
);
