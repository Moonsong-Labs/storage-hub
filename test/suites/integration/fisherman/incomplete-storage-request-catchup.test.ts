import assert from "node:assert";
import { u8aToHex } from "@polkadot/util";
import { decodeAddress } from "@polkadot/util-crypto";
import {
  describeMspNet,
  type EnrichedBspApi,
  shUser,
  bspKey,
  waitFor,
  ShConsts
} from "../../../util";

/**
 * FISHERMAN INCOMPLETE STORAGE REQUESTS WITH CATCHUP
 *
 * Purpose: Tests the fisherman's ability to build forest proofs with unfinalized files
 *          when processing incomplete storage request events.
 *
 * What makes this test unique:
 * - Pauses fisherman to accumulate events
 * - Creates finalized incomplete storage request events (expired)
 * - Adds NEW files in unfinalized blocks to update forest state
 * - Verifies fisherman builds proofs with unfinalized forest state
 * - Tests BSP-only scenario (MSP paused to prevent acceptance)
 *
 * Test Scenario:
 * 1. Pauses MSP to ensure only BSP accepts storage request
 * 2. Creates storage request that will expire (BSP volunteers and stores)
 * 3. Pauses fisherman before expiration
 * 4. Finalizes expiration to create IncompleteStorageRequest event
 * 5. Adds 2 NEW files in unfinalized blocks (updates BSP forest)
 * 6. Resumes fisherman - it should build proofs including unfinalized files
 */
await describeMspNet(
  "Fisherman Incomplete Storage Requests with Catchup",
  {
    initialised: false,
    indexer: true,
    fisherman: true,
    indexerMode: "fishing",
    standaloneIndexer: true
  },
  ({
    before,
    it,
    createUserApi,
    createBspApi,
    createMsp1Api,
    createFishermanApi,
    createIndexerApi
  }) => {
    let userApi: EnrichedBspApi;
    let bspApi: EnrichedBspApi;
    let msp1Api: EnrichedBspApi;
    let indexerApi: EnrichedBspApi;
    let fishermanApi: EnrichedBspApi;
    before(async () => {
      userApi = await createUserApi();
      bspApi = await createBspApi();
      const maybeMsp1Api = await createMsp1Api();

      assert(maybeMsp1Api, "MSP API not available");
      msp1Api = maybeMsp1Api;

      // Wait for user node to be ready
      await userApi.docker.waitForLog({
        searchString: "💤 Idle",
        containerName: userApi.shConsts.NODE_INFOS.user.containerName,
        timeout: 10000
      });

      // Ensure fisherman node is ready
      assert(createFishermanApi, "Fisherman API not available for fisherman test");
      fishermanApi = await createFishermanApi();

      // Connect to standalone indexer node
      assert(
        createIndexerApi,
        "Indexer API not available. Ensure `standaloneIndexer` is set to `true` in the network configuration."
      );
      indexerApi = await createIndexerApi();

      // Wait for indexer to process the finalized block (producerApi will seal a finalized block by default)
      await indexerApi.indexer.waitForIndexing({ producerApi: userApi });
    });

    it("processes expired request (BSP only) with catchup", async () => {
      const bucketName = "test-expired-bsp-catchup";
      const source = "res/whatsup.jpg";
      const destination = "test/expired-bsp.txt";

      // Pause MSP container to ensure only BSP accepts the initial request
      await userApi.docker.pauseContainer(userApi.shConsts.NODE_INFOS.msp1.containerName);

      // Step 1: Create storage request that will expire, with BSP storing file (finalized)
      const { fileKey, bucketId } = await userApi.file.createBucketAndSendNewStorageRequest(
        source,
        destination,
        bucketName,
        null,
        ShConsts.DUMMY_MSP_ID,
        shUser,
        1
      );

      const storageRequest = await userApi.query.fileSystem.storageRequests(fileKey);
      assert(storageRequest.isSome);
      const expiresAt = storageRequest.unwrap().expiresAt.toNumber();

      await userApi.wait.bspVolunteer();

      await waitFor({
        lambda: async () =>
          (await bspApi.rpc.storagehubclient.isFileInFileStorage(fileKey)).isFileFound
      });

      const bspAddress = userApi.createType("Address", bspKey.address);
      await userApi.wait.bspStored({
        expectedExts: 1,
        bspAccount: bspAddress,
        sealBlock: true
      });

      // Step 2: Pause fisherman before expiration
      await userApi.docker.pauseContainer(userApi.shConsts.NODE_INFOS.fisherman.containerName);

      // Step 3: Finalize expiration to create incomplete storage request event
      const incompleteStorageRequestResult = await userApi.block.skipTo(expiresAt, {
        finalised: true
      });

      await userApi.assert.eventPresent(
        "fileSystem",
        "IncompleteStorageRequest",
        incompleteStorageRequestResult.events
      );

      // Wait for indexer to process the finalized incomplete storage request event
      await indexerApi.indexer.waitForIndexing({ producerApi: userApi });

      // Step 4: Add NEW files in unfinalized blocks (using same bucket to update BSP forest)
      // BSP-only scenario - MSP is paused, so we use custom logic instead of helper
      const fileKeys: string[] = [];
      const storageRequestTxs = [];
      const ownerHex = u8aToHex(decodeAddress(userApi.shConsts.NODE_INFOS.user.AddressId)).slice(2);
      const bucketIdH256 = userApi.createType("H256", bucketId);

      // Prepare storage request transactions for 2 files
      for (let i = 0; i < 2; i++) {
        const newDest = `test/catchup-new-${i}.txt`;

        const {
          file_key,
          file_metadata: { location, fingerprint, file_size }
        } = await userApi.rpc.storagehubclient.loadFileInStorage(
          source,
          newDest,
          ownerHex,
          bucketId.toString()
        );

        fileKeys.push(file_key.toString());

        storageRequestTxs.push(
          userApi.tx.fileSystem.issueStorageRequest(
            bucketIdH256,
            location,
            fingerprint,
            file_size,
            ShConsts.DUMMY_MSP_ID,
            [userApi.shConsts.NODE_INFOS.user.expectedPeerId],
            { Custom: 1 }
          )
        );
      }

      // Seal all storage requests in a single block (unfinalized)
      await userApi.block.seal({ calls: storageRequestTxs, signer: shUser, finaliseBlock: false });

      // Wait for all BSP volunteers to appear in tx pool
      await userApi.wait.bspVolunteer(fileKeys.length);
      await userApi.block.seal({ finaliseBlock: false });

      // Wait for all BSP stored confirmations
      // BSP batches extrinsics, so we need to iteratively seal blocks and count events
      let totalConfirmations = 0;
      const maxAttempts = 3;
      for (
        let attempt = 0;
        attempt < maxAttempts && totalConfirmations < fileKeys.length;
        attempt++
      ) {
        // Wait for at least one bspConfirmStoring extrinsic in tx pool (don't check exact count)
        await userApi.wait.bspStored({
          sealBlock: false,
          timeoutMs: 5000,
          bspAccount: bspAddress
        });

        // Seal the block and count BspConfirmedStoring events (unfinalized)
        const { events } = await userApi.block.seal({ finaliseBlock: false });
        const confirmEvents = await userApi.assert.eventMany(
          "fileSystem",
          "BspConfirmedStoring",
          events
        );

        // Count total file keys in all BspConfirmedStoring events
        for (const eventRecord of confirmEvents) {
          if (userApi.events.fileSystem.BspConfirmedStoring.is(eventRecord.event)) {
            totalConfirmations += eventRecord.event.data.confirmedFileKeys.length;
          }
        }
      }

      assert.strictEqual(
        totalConfirmations,
        fileKeys.length,
        `Expected ${fileKeys.length} BSP confirmations, but got ${totalConfirmations}`
      );

      // Wait for BSP to store all files locally
      for (const fileKey of fileKeys) {
        await waitFor({
          lambda: async () =>
            (await bspApi.rpc.storagehubclient.isFileInFileStorage(fileKey)).isFileFound
        });
      }

      // Verify gap between finalized and current head
      const finalizedHead = await userApi.rpc.chain.getFinalizedHead();
      const currentHead = await userApi.rpc.chain.getHeader();
      assert(
        currentHead.number.toNumber() >
          (await userApi.rpc.chain.getHeader(finalizedHead)).number.toNumber(),
        "Current head should be ahead of finalized head"
      );

      await userApi.block.seal({ finaliseBlock: false });

      // Step 5: Resume fisherman - it should build proofs with updated forest
      await userApi.docker.resumeContainer({
        containerName: userApi.shConsts.NODE_INFOS.fisherman.containerName
      });

      // Wait for fisherman to catch up to the chain tip to get be able to see new fulfilled storage requests from the BSP in unfinalized blocks
      await fishermanApi.wait.nodeCatchUpToChainTip(userApi);

      // Wait for fisherman to process incomplete storage deletions with retry if we encounter ForestProofVerificationFailed errors (stale proofs)
      // When resuming the fisherman node, the deletion task can trigger and submit a deletion extrinsic before the blockchain service imports the latest block containing the unfinalized BSP
      // confirmations. In this scenario, it is guaranteed that the deletion extrinsic will fail with a ForestProofVerificationFailed error and we must let the fisherman retry.
      await userApi.fisherman.retryableWaitAndVerifyBatchDeletions({
        blockProducerApi: userApi,
        deletionType: "Incomplete",
        expectExt: 1, // 1 BSP only (MSP did not accept)
        userApi,
        bspApi,
        expectedBspCount: 1,
        maxRetries: 3
      });

      // Always resume MSP container even if test fails
      await userApi.docker.resumeContainer({
        containerName: userApi.shConsts.NODE_INFOS.msp1.containerName
      });

      // Seal block so MSP gets out of sync mode
      await userApi.block.seal();

      // Wait for MSP to catch up to the chain tip
      await msp1Api.wait.nodeCatchUpToChainTip(userApi);
    });
  }
);
