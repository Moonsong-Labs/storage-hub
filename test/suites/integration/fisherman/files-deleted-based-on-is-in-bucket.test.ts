import assert from "node:assert";
import {
  describeMspNet,
  type EnrichedBspApi,
  type SqlClient,
  shUser,
  mspKey,
  hexToBuffer,
  bspTwoKey,
  ShConsts
} from "../../../util";

/**
 * Test that verifies fisherman batch processing for file deletions when files are in bucket forests or not.
 *
 * We check that the fisherman only submits bucket file deletions when they are actually in bucket forests.
 * The fisherman knows this because in the indexer database, we set the `is_in_bucket` field to false for files that are not in bucket forests.
 * The test scenarios below are:
 * - User requests file deletions for files that are in bucket forests (i.e. the MSP accepted the storage request). We expect
 *   the fisherman to submit bucket file deletions for the files that are in the bucket forest.
 * - Storage requests are revoked by the user before the MSP accepted the storage request (i.e. the files are not in the bucket forest).
 *   We expect the fisherman to submit BSP file deletions for the files that are not in the bucket forest but not bucket file deletions.
 * - A subsequent storage request is revoked after the MSP already accepted the first request
 *   (file is already in the bucket forest). We expect the fisherman to only submit BSP deletion for the revoked request,
 *   NOT a bucket deletion.
 */
await describeMspNet(
  "Fisherman Batch File Deletion - MSP Stop Storing Bucket",
  {
    initialised: false,
    indexer: true,
    fisherman: true,
    indexerMode: "fishing",
    standaloneIndexer: true,
    logLevel: "debug"
  },
  ({
    before,
    after,
    it,
    createUserApi,
    createBspApi,
    createMsp1Api,
    createSqlClient,
    createFishermanApi,
    createIndexerApi,
    createApi
  }) => {
    let userApi: EnrichedBspApi;
    let bspApi: EnrichedBspApi;
    let bspTwoApi: EnrichedBspApi;
    let msp1Api: EnrichedBspApi;
    let indexerApi: EnrichedBspApi;
    let sql: SqlClient;

    before(async () => {
      userApi = await createUserApi();
      bspApi = await createBspApi();
      const maybeMsp1Api = await createMsp1Api();

      assert(maybeMsp1Api, "MSP API not available");
      msp1Api = maybeMsp1Api;
      sql = createSqlClient();

      await userApi.docker.waitForLog({
        searchString: "💤 Idle",
        containerName: userApi.shConsts.NODE_INFOS.user.containerName,
        timeout: 10000
      });

      // Ensure fisherman node is ready (created but not used directly - helper functions handle interaction)
      assert(
        createFishermanApi,
        "Fisherman API not available. Ensure `fisherman` is set to `true` in the network configuration."
      );
      await createFishermanApi();

      // Connect to standalone indexer node
      assert(
        createIndexerApi,
        "Indexer API not available. Ensure `standaloneIndexer` is set to `true` in the network configuration."
      );
      indexerApi = await createIndexerApi();

      // Wait for indexer to process the finalized block (producerApi will seal a finalized block by default)
      await indexerApi.indexer.waitForIndexing({ producerApi: userApi, sql });
    });

    after(async () => {
      if (bspTwoApi) {
        await bspTwoApi.disconnect();
      }
    });

    it("batches user-requested file deletions after MSP stops storing bucket", async () => {
      const mspId = userApi.shConsts.DUMMY_MSP_ID;

      const valueProps = await userApi.call.storageProvidersApi.queryValuePropositionsForMsp(mspId);
      const valuePropId = valueProps[0].id;

      // Use batchStorageRequests helper to create 3 buckets with 2 files each (6 files total)
      const batchResult = await userApi.file.batchStorageRequests({
        files: [
          {
            source: "res/smile.jpg",
            destination: "test/msp-stop-b0-f0.txt",
            bucketIdOrName: "test-msp-stop-bucket-0",
            replicationTarget: 1
          },
          {
            source: "res/smile.jpg",
            destination: "test/msp-stop-b0-f1.txt",
            bucketIdOrName: "test-msp-stop-bucket-0",
            replicationTarget: 1
          },
          {
            source: "res/smile.jpg",
            destination: "test/msp-stop-b1-f0.txt",
            bucketIdOrName: "test-msp-stop-bucket-1",
            replicationTarget: 1
          },
          {
            source: "res/smile.jpg",
            destination: "test/msp-stop-b1-f1.txt",
            bucketIdOrName: "test-msp-stop-bucket-1",
            replicationTarget: 1
          },
          {
            source: "res/smile.jpg",
            destination: "test/msp-stop-b2-f0.txt",
            bucketIdOrName: "test-msp-stop-bucket-2",
            replicationTarget: 1
          },
          {
            source: "res/smile.jpg",
            destination: "test/msp-stop-b2-f1.txt",
            bucketIdOrName: "test-msp-stop-bucket-2",
            replicationTarget: 1
          }
        ],
        mspId,
        valuePropId,
        owner: shUser,
        bspApi,
        mspApi: msp1Api
      });

      const { fileKeys, bucketIds, locations, fingerprints, fileSizes } = batchResult;

      await indexerApi.indexer.waitForIndexing({ producerApi: userApi, sql });

      // Wait for all files to be indexed
      for (const fileKey of fileKeys) {
        await indexerApi.indexer.waitForFileIndexed({ sql, fileKey });
        await indexerApi.indexer.waitForMspFileAssociation({ sql, fileKey });
        await indexerApi.indexer.waitForBspFileAssociation({ sql, fileKey });
      }

      // Get unique bucket IDs (3 buckets)
      const uniqueBucketIds = Array.from(new Set(bucketIds));

      // MSP stops storing all buckets
      const stopStoringCalls = uniqueBucketIds.map((bucketId) =>
        userApi.tx.fileSystem.mspStopStoringBucket(bucketId)
      );

      // Seal a single block with all MSP stop storing calls
      const stopStoringResult = await userApi.block.seal({
        calls: stopStoringCalls,
        signer: mspKey
      });

      // Verify all MspStoppedStoringBucket events are present (one per bucket)
      const stoppedStoringEvents = (stopStoringResult.events || []).filter((record) =>
        userApi.events.fileSystem.MspStoppedStoringBucket.is(record.event)
      );

      assert.equal(
        stoppedStoringEvents.length,
        uniqueBucketIds.length,
        `Should have ${uniqueBucketIds.length} MspStoppedStoringBucket events`
      );

      await indexerApi.indexer.waitForIndexing({ producerApi: userApi, sql });

      // Verify MSP file associations are removed for all files
      for (const fileKey of fileKeys) {
        const mspFileAssociations = await sql`
          SELECT mf.* FROM msp_file mf
          INNER JOIN file f ON mf.file_id = f.id
          WHERE f.file_key = ${hexToBuffer(fileKey)}
        `;
        assert.equal(
          mspFileAssociations.length,
          0,
          `MSP file association should be removed for file ${fileKey} after MSP stops storing bucket`
        );
      }

      // Verify is_in_bucket is still true for all files
      for (const fileKey of fileKeys) {
        const fileRecord = await sql`
          SELECT file_key, is_in_bucket FROM file WHERE file_key = ${hexToBuffer(fileKey)}
        `;

        assert.equal(fileRecord.length, 1, `Should have file record for ${fileKey} in database`);

        assert.equal(
          fileRecord[0].is_in_bucket,
          true,
          `File ${fileKey} should have is_in_bucket = true even after MSP stops storing`
        );
      }

      // Build all deletion request calls
      const deletionCalls = [];
      for (let i = 0; i < fileKeys.length; i++) {
        const fileOperationIntention = {
          fileKey: fileKeys[i],
          operation: { Delete: null }
        };

        const intentionCodec = userApi.createType(
          "PalletFileSystemFileOperationIntention",
          fileOperationIntention
        );
        const intentionPayload = intentionCodec.toU8a();
        const rawSignature = shUser.sign(intentionPayload);
        const userSignature = userApi.createType("MultiSignature", {
          Sr25519: rawSignature
        });

        deletionCalls.push(
          userApi.tx.fileSystem.requestDeleteFile(
            fileOperationIntention,
            userSignature,
            bucketIds[i],
            locations[i],
            fileSizes[i],
            fingerprints[i]
          )
        );
      }

      // Seal a single block with all deletion requests
      const deletionRequestResult = await userApi.block.seal({
        calls: deletionCalls,
        signer: shUser
      });

      // Verify all FileDeletionRequested events are present (one per file)
      const deletionRequestedEvents = (deletionRequestResult.events || []).filter((record) =>
        userApi.events.fileSystem.FileDeletionRequested.is(record.event)
      );

      assert.equal(
        deletionRequestedEvents.length,
        fileKeys.length,
        `Should have ${fileKeys.length} FileDeletionRequested events`
      );

      await indexerApi.indexer.waitForIndexing({ producerApi: userApi, sql });

      // Verify deletion signatures are stored in database for the User deletion type
      await indexerApi.indexer.verifyDeletionSignaturesStored({
        sql,
        fileKeys
      });

      // Use fisherman helper to verify batch deletions are processed correctly
      // Skip forest root verification for buckets that MSP stopped storing since the MSP is no longer managing them
      await userApi.fisherman.retryableWaitAndVerifyBatchDeletions({
        blockProducerApi: userApi,
        deletionType: "User",
        expectExt: 4,
        userApi,
        bspApi,
        expectedBspCount: 1,
        mspApi: msp1Api,
        expectedBucketCount: 3,
        skipBucketIds: uniqueBucketIds,
        maxRetries: 3
      });
    });

    it("batches incomplete storage request deletions when MSP never accepted (BSP only)", async () => {
      // Pause MSP before creating storage requests so it never accepts them
      await userApi.docker.pauseContainer(userApi.shConsts.NODE_INFOS.msp1.containerName);

      try {
        const mspId = userApi.shConsts.DUMMY_MSP_ID;
        const valueProps =
          await userApi.call.storageProvidersApi.queryValuePropositionsForMsp(mspId);
        const valuePropId = valueProps[0].id;

        // Use batchStorageRequests helper with only bspApi (no mspApi) to create 3 buckets with 2 files each (6 files total)
        // Since mspApi is not provided, MSP checks are skipped
        const batchResult = await userApi.file.batchStorageRequests({
          files: [
            {
              source: "res/smile.jpg",
              destination: "test/msp-never-accept-b0-f0.txt",
              bucketIdOrName: "test-msp-never-accept-bucket-0",
              replicationTarget: 1
            },
            {
              source: "res/smile.jpg",
              destination: "test/msp-never-accept-b0-f1.txt",
              bucketIdOrName: "test-msp-never-accept-bucket-0",
              replicationTarget: 1
            },
            {
              source: "res/smile.jpg",
              destination: "test/msp-never-accept-b1-f0.txt",
              bucketIdOrName: "test-msp-never-accept-bucket-1",
              replicationTarget: 1
            },
            {
              source: "res/smile.jpg",
              destination: "test/msp-never-accept-b1-f1.txt",
              bucketIdOrName: "test-msp-never-accept-bucket-1",
              replicationTarget: 1
            },
            {
              source: "res/smile.jpg",
              destination: "test/msp-never-accept-b2-f0.txt",
              bucketIdOrName: "test-msp-never-accept-bucket-2",
              replicationTarget: 1
            },
            {
              source: "res/smile.jpg",
              destination: "test/msp-never-accept-b2-f1.txt",
              bucketIdOrName: "test-msp-never-accept-bucket-2",
              replicationTarget: 1
            }
          ],
          mspId,
          valuePropId,
          owner: shUser,
          bspApi,
          // mspApi is intentionally not provided - MSP checks will be skipped
          maxAttempts: 5
        });

        const { fileKeys } = batchResult;

        await indexerApi.indexer.waitForIndexing({ producerApi: userApi, sql });

        // Wait for all files to be indexed and verify BSP associations exist
        for (const fileKey of fileKeys) {
          await indexerApi.indexer.waitForFileIndexed({ sql, fileKey });
          await indexerApi.indexer.waitForBspFileAssociation({ sql, fileKey });
        }

        // Verify MSP file associations do NOT exist (MSP never accepted)
        for (const fileKey of fileKeys) {
          const mspFileAssociations = await sql`
            SELECT mf.* FROM msp_file mf
            INNER JOIN file f ON mf.file_id = f.id
            WHERE f.file_key = ${hexToBuffer(fileKey)}
          `;
          assert.equal(
            mspFileAssociations.length,
            0,
            `MSP file association should not exist for file ${fileKey} since MSP never accepted`
          );
        }

        // Verify is_in_bucket is false for all files (MSP never added them to bucket forests)
        for (const fileKey of fileKeys) {
          const fileRecord = await sql`
            SELECT file_key, is_in_bucket FROM file WHERE file_key = ${hexToBuffer(fileKey)}
          `;

          assert.equal(fileRecord.length, 1, `Should have file record for ${fileKey} in database`);

          assert.equal(
            fileRecord[0].is_in_bucket,
            false,
            `File ${fileKey} should have is_in_bucket = false since MSP never accepted and added to bucket forest`
          );
        }

        // Build all revocation calls
        const revocationCalls = fileKeys.map((fileKey) =>
          userApi.tx.fileSystem.revokeStorageRequest(fileKey)
        );

        // Seal a single block with all revocation requests
        const revokeResult = await userApi.block.seal({
          calls: revocationCalls,
          signer: shUser
        });

        // Verify all StorageRequestRevoked events are present (one per file)
        const revokedEvents = (revokeResult.events || []).filter((record) =>
          userApi.events.fileSystem.StorageRequestRevoked.is(record.event)
        );

        assert.equal(
          revokedEvents.length,
          fileKeys.length,
          `Should have ${fileKeys.length} StorageRequestRevoked events`
        );

        // Verify all IncompleteStorageRequest events are present (one per file)
        const incompleteEvents = (revokeResult.events || []).filter((record) =>
          userApi.events.fileSystem.IncompleteStorageRequest.is(record.event)
        );

        assert.equal(
          incompleteEvents.length,
          fileKeys.length,
          `Should have ${fileKeys.length} IncompleteStorageRequest events`
        );

        // Verify incomplete storage request state
        const incompleteStorageRequests =
          await userApi.query.fileSystem.incompleteStorageRequests.entries();
        assert(incompleteStorageRequests.length > 0, "Should have incomplete storage requests");

        await indexerApi.indexer.waitForIndexing({ producerApi: userApi, sql });

        // Use fisherman helper to verify batch deletions are processed correctly
        // Since MSP never accepted, fisherman should only delete from BSP (no bucket deletions)
        await userApi.fisherman.retryableWaitAndVerifyBatchDeletions({
          blockProducerApi: userApi,
          deletionType: "Incomplete",
          expectExt: 1, // Only BSP deletion, no bucket deletions
          userApi,
          bspApi,
          expectedBspCount: 1,
          mspApi: msp1Api,
          expectedBucketCount: 0, // No bucket deletions since MSP never accepted
          maxRetries: 3
        });

        // Wait for indexer to process the finalized block
        await indexerApi.indexer.waitForIndexing({ producerApi: userApi, sql });

        // Verify all files are deleted from the database
        const deletedFiles = await sql`
          SELECT file_key FROM file
        `;
        assert(deletedFiles.length === 0, "Should have no files in database");
      } finally {
        // Always resume MSP container even if the test fails
        await userApi.docker.resumeContainer({
          containerName: userApi.shConsts.NODE_INFOS.msp1.containerName
        });
      }
    });

    it("revoked subsequent storage request only triggers BSP deletion (not bucket deletion) when file already in bucket", async () => {
      // This test verifies that when a subsequent storage request is revoked after the MSP
      // has already accepted the first request (file is in bucket), the fisherman only submits
      // BSP deletion for the revoked request, NOT a bucket deletion (since is_in_bucket = true).

      const source = "res/adolphus.jpg";
      const destination = "test/revoke-subsequent-fisherman.jpg";
      const bucketName = "revoke-subsequent-fisherman-bucket";

      const mspId = userApi.shConsts.DUMMY_MSP_ID;
      const valueProps = await userApi.call.storageProvidersApi.queryValuePropositionsForMsp(mspId);
      const valuePropId = valueProps[0].id;

      // Step 1: Create bucket and issue first storage request - MSP will accept
      const firstFile = await userApi.file.createBucketAndSendNewStorageRequest(
        source,
        destination,
        bucketName,
        valuePropId,
        mspId,
        shUser,
        1,
        true
      );

      // Step 2: Wait for MSP to accept (triggers MutationsApplied, sets is_in_bucket = true)
      await msp1Api.wait.fileStorageComplete(firstFile.fileKey);
      await userApi.wait.mspResponseInTxPool();

      // Wait for BSP to volunteer and store
      await userApi.wait.bspVolunteer(1);
      await userApi.assert.eventPresent("fileSystem", "MspAcceptedStorageRequest");

      const bspAccount = userApi.createType("Address", userApi.accounts.bspKey.address);
      await userApi.wait.bspStored({ expectedExts: 1, bspAccount });

      await indexerApi.indexer.waitForIndexing({ producerApi: userApi, sql });

      // Wait for first file to be indexed with MSP and BSP associations
      await indexerApi.indexer.waitForFileIndexed({
        sql,
        fileKey: firstFile.fileKey
      });
      await indexerApi.indexer.waitForMspFileAssociation({
        sql,
        fileKey: firstFile.fileKey
      });
      await indexerApi.indexer.waitForBspFileAssociation({
        sql,
        fileKey: firstFile.fileKey
      });

      // Verify first file record has is_in_bucket = true
      const firstFileRecords = await sql`
        SELECT id, file_key, is_in_bucket FROM file 
        WHERE file_key = ${hexToBuffer(firstFile.fileKey)}
      `;
      assert.equal(firstFileRecords.length, 1, "Should have one file record");
      assert.equal(
        firstFileRecords[0].is_in_bucket,
        true,
        "First file record should have is_in_bucket = true after MSP acceptance"
      );

      // Step 3: Onboard BSP2 after first storage request completes
      // This ensures BSP2 is available to volunteer for the second request
      const { rpcPort: bspTwoRpcPort } = await userApi.docker.onboardBsp({
        bspSigner: bspTwoKey,
        name: "sh-bsp-two",
        bspId: ShConsts.BSP_TWO_ID,
        additionalArgs: ["--keystore-path=/keystore/bsp-two"],
        waitForIdle: true
      });

      // Create API connection to BSP two for forest root verification
      bspTwoApi = await createApi(`ws://127.0.0.1:${bspTwoRpcPort}`);

      // Step 5: Issue second storage request for same file
      const fingerprint = userApi.shConsts.TEST_ARTEFACTS[source].fingerprint;
      const fileSize = userApi.shConsts.TEST_ARTEFACTS[source].size;

      await userApi.block.seal({
        calls: [
          userApi.tx.fileSystem.issueStorageRequest(
            firstFile.bucketId,
            destination,
            fingerprint,
            fileSize,
            mspId,
            [userApi.shConsts.NODE_INFOS.user.expectedPeerId],
            { Custom: 2 }
          )
        ],
        signer: shUser
      });

      const { event: newStorageRequestEvent } = await userApi.assert.eventPresent(
        "fileSystem",
        "NewStorageRequest"
      );

      const newStorageRequestDataBlob =
        userApi.events.fileSystem.NewStorageRequest.is(newStorageRequestEvent) &&
        newStorageRequestEvent.data;

      assert(newStorageRequestDataBlob, "NewStorageRequest event data not found");
      const secondFileKey = newStorageRequestDataBlob.fileKey.toString();

      // Wait for BSP2 to volunteer for second request
      // BSP1 already stored the file, so only BSP2 will volunteer
      await userApi.wait.bspVolunteer(1);
      const bspTwoAccount = userApi.createType("Address", bspTwoKey.address);
      await userApi.wait.bspStored({
        expectedExts: 1,
        bspAccount: bspTwoAccount
      });

      await indexerApi.indexer.waitForIndexing({ producerApi: userApi, sql });

      // Wait for second file record to be indexed
      await indexerApi.indexer.waitForFileIndexed({
        sql,
        fileKey: secondFileKey
      });

      // Step 6: Verify second file record has is_in_bucket = true (inherited from first)
      const allRecordsBeforeRevoke = await sql`
          SELECT id, file_key, is_in_bucket, created_at FROM file 
          WHERE file_key = ${hexToBuffer(secondFileKey)}
          ORDER BY created_at ASC
        `;

      assert.equal(
        allRecordsBeforeRevoke.length,
        2,
        "Should have two file records (first and second storage request)"
      );
      assert.equal(
        allRecordsBeforeRevoke[0].is_in_bucket,
        true,
        "First file record should have is_in_bucket = true"
      );
      assert.equal(
        allRecordsBeforeRevoke[1].is_in_bucket,
        true,
        "Second file record should have is_in_bucket = true (inherited from first)"
      );

      // Step 7: Revoke the second storage request
      await userApi.block.seal({
        calls: [userApi.tx.fileSystem.revokeStorageRequest(secondFileKey)],
        signer: shUser
      });

      await userApi.assert.eventPresent("fileSystem", "StorageRequestRevoked");
      await userApi.assert.eventPresent("fileSystem", "IncompleteStorageRequest");

      await indexerApi.indexer.waitForIndexing({ producerApi: userApi, sql });

      // Verify is_in_bucket is still true for the first record (file is still in bucket)
      const recordsAfterRevoke = await sql`
          SELECT id, is_in_bucket, created_at FROM file 
          WHERE file_key = ${hexToBuffer(secondFileKey)}
          ORDER BY created_at ASC
        `;

      assert(recordsAfterRevoke.length >= 1, "At least first file record should exist");
      assert.equal(
        recordsAfterRevoke[0].is_in_bucket,
        true,
        "First file record should still have is_in_bucket = true (file still in bucket)"
      );

      // Step 8: Use fisherman to verify batch deletions
      // The fisherman should ONLY submit BSP deletion (not bucket deletion)
      // because is_in_bucket = true means the file is still in the bucket forest
      await userApi.fisherman.retryableWaitAndVerifyBatchDeletions({
        blockProducerApi: userApi,
        deletionType: "Incomplete",
        expectExt: 1, // Only BSP deletion, NO bucket deletion
        userApi,
        bspApi: bspTwoApi,
        expectedBspCount: 1, // BSP deletion for the revoked request
        mspApi: msp1Api,
        expectedBucketCount: 0, // NO bucket deletion since file is still in bucket (is_in_bucket = true)
        maxRetries: 3
      });

      // Verify MSP association still exists (file is still in bucket)
      const mspAssociations = await sql`
          SELECT mf.* FROM msp_file mf
          INNER JOIN file f ON mf.file_id = f.id
          WHERE f.file_key = ${hexToBuffer(secondFileKey)}
        `;
      assert.equal(
        mspAssociations.length,
        2,
        "MSP association from first request should still exist"
      );

      // Verify at least the first file record still exists with is_in_bucket = true
      const finalRecords = await sql`
          SELECT id, is_in_bucket FROM file 
          WHERE file_key = ${hexToBuffer(secondFileKey)}
        `;
      assert(finalRecords.length >= 1, "At least first file record should still exist");
      for (const rec of finalRecords) {
        assert.equal(
          rec.is_in_bucket,
          true,
          "All file records should have is_in_bucket = true after fisherman processing"
        );
      }
    });
  }
);
