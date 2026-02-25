import assert from "node:assert";
import type { Option } from "@polkadot/types";
import type { H256 } from "@polkadot/types/interfaces";
import {
  describeMspNet,
  type EnrichedBspApi,
  type FileMetadata,
  shUser,
  type SqlClient,
  hexToBuffer,
  bspTwoKey,
  ShConsts
} from "../../../util";

/**
 * Test that verifies the `is_in_bucket` field is correctly inherited when creating
 * new file records for subsequent storage requests of the same file key.
 *
 * This test covers the fix from PR #598:
 * - When a user issues a subsequent storage request for a file key that the MSP is already storing,
 *   the MSP accepts with an inclusion proof (since the file already exists in the bucket).
 * - The runtime doesn't emit a `MutationsApplied` event (bucket root unchanged).
 * - Previously: The new file record was created with `is_in_bucket = false` (incorrect).
 * - Fix: New file records now inherit `is_in_bucket` from sibling records with the same file key.
 */
await describeMspNet(
  "Indexer Service - is_in_bucket Consistency for Repeated Storage Requests",
  {
    initialised: false,
    indexer: true,
    indexerMode: "full",
    standaloneIndexer: true
  },
  ({
    before,
    it,
    createUserApi,
    createBspApi,
    createMsp1Api,
    createSqlClient,
    createIndexerApi
  }) => {
    let userApi: EnrichedBspApi;
    let mspApi: EnrichedBspApi;
    let indexerApi: EnrichedBspApi;
    let sql: SqlClient;

    before(async () => {
      userApi = await createUserApi();
      await createBspApi();
      const maybeMspApi = await createMsp1Api();

      assert(maybeMspApi, "MSP API not available");
      mspApi = maybeMspApi;

      sql = createSqlClient();

      // Connect to standalone indexer node
      assert(
        createIndexerApi,
        "Indexer API not available. Ensure `standaloneIndexer` is set to `true` in the network configuration."
      );
      indexerApi = await createIndexerApi();

      // Wait for indexer to be ready and process initial blocks
      await indexerApi.indexer.waitForIndexing({ producerApi: userApi, sql });
    });

    it("subsequent storage request for same file key inherits is_in_bucket from sibling record", async () => {
      const source = "res/whatsup.jpg";
      const destination = "test/is-in-bucket-test.jpg";
      const bucketName = "is-in-bucket-test-bucket";

      const mspId = userApi.shConsts.DUMMY_MSP_ID;
      const valueProps = await userApi.call.storageProvidersApi.queryValuePropositionsForMsp(mspId);
      const valuePropId = valueProps[0].id;

      // Step 1: Create bucket and issue first storage request with replication target of 1
      const firstFile: FileMetadata = await userApi.file.createBucketAndSendNewStorageRequest(
        source,
        destination,
        bucketName,
        valuePropId,
        mspId,
        shUser,
        1,
        true
      );

      // Step 2: Wait for MSP to accept the storage request (triggers MutationsApplied event)
      await mspApi.wait.fileStorageComplete(firstFile.fileKey);
      await userApi.wait.mspResponseInTxPool();

      // Step 3: Wait for BSP to volunteer and store
      await userApi.wait.bspVolunteer(1);

      // Verify MspAcceptedStorageRequest event
      const { event: storageRequestAccepted } = await userApi.assert.eventPresent(
        "fileSystem",
        "MspAcceptedStorageRequest"
      );

      const storageRequestAcceptedDataBlob =
        userApi.events.fileSystem.MspAcceptedStorageRequest.is(storageRequestAccepted) &&
        storageRequestAccepted.data;

      assert(storageRequestAcceptedDataBlob, "MspAcceptedStorageRequest event data not found");
      assert.strictEqual(
        storageRequestAcceptedDataBlob.fileKey.toString(),
        firstFile.fileKey,
        "File key should match the first storage request"
      );

      // Wait for BSP to confirm storage
      const bspAccount = userApi.createType("Address", userApi.accounts.bspKey.address);
      await userApi.wait.bspStored({ expectedExts: 1, bspAccount });

      // Wait for indexer to process the block with MutationsApplied event
      await indexerApi.indexer.waitForIndexing({ producerApi: userApi, sql });

      // Step 4: Verify the first file record has is_in_bucket = true
      await indexerApi.indexer.waitForFileIndexed({
        sql,
        fileKey: firstFile.fileKey
      });
      await indexerApi.indexer.waitForMspFileAssociation({
        sql,
        fileKey: firstFile.fileKey
      });

      const firstFileRecords = await sql`
        SELECT id, file_key, is_in_bucket, created_at 
        FROM file 
        WHERE file_key = ${hexToBuffer(firstFile.fileKey)}
        ORDER BY created_at ASC
      `;

      assert.strictEqual(
        firstFileRecords.length,
        1,
        "Should have exactly one file record after first storage request"
      );
      assert.strictEqual(
        firstFileRecords[0].is_in_bucket,
        true,
        "First file record should have is_in_bucket = true (set by MutationsApplied event)"
      );

      // Step 5: Issue a second storage request for the SAME file in the SAME bucket
      // This simulates a user adding redundancy by requesting more BSPs
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
            { Custom: 2 } // Higher replication target
          )
        ],
        signer: shUser
      });

      // Verify NewStorageRequest event for second request
      const { event: newStorageRequestEvent } = await userApi.assert.eventPresent(
        "fileSystem",
        "NewStorageRequestV2"
      );

      const newStorageRequestDataBlob =
        userApi.events.fileSystem.NewStorageRequestV2.is(newStorageRequestEvent) &&
        newStorageRequestEvent.data;

      assert(newStorageRequestDataBlob, "NewStorageRequestV2 event data not found");

      // The file key should be the same as the first storage request since it's the same file
      const secondFileKey = newStorageRequestDataBlob.fileKey.toString();
      assert.strictEqual(
        secondFileKey,
        firstFile.fileKey,
        "Second storage request should have the same file key as the first"
      );

      // Step 6: Wait for MSP to accept the second request
      // Note: No MutationsApplied event will be emitted since the file is already in the bucket
      await mspApi.wait.fileStorageComplete(secondFileKey);
      await userApi.wait.mspResponseInTxPool();
      await userApi.block.seal();

      // Verify MspAcceptedStorageRequest event for second request
      const { event: secondStorageRequestAccepted } = await userApi.assert.eventPresent(
        "fileSystem",
        "MspAcceptedStorageRequest"
      );

      const secondStorageRequestAcceptedDataBlob =
        userApi.events.fileSystem.MspAcceptedStorageRequest.is(secondStorageRequestAccepted) &&
        secondStorageRequestAccepted.data;

      assert(
        secondStorageRequestAcceptedDataBlob,
        "Second MspAcceptedStorageRequest event data not found"
      );

      // Wait for indexer to process the block
      await indexerApi.indexer.waitForIndexing({ producerApi: userApi, sql });

      // Step 8 & 9: Query all file records with the same file key and verify is_in_bucket consistency
      const allFileRecords = await sql`
        SELECT id, file_key, is_in_bucket, created_at 
        FROM file 
        WHERE file_key = ${hexToBuffer(firstFile.fileKey)}
        ORDER BY created_at ASC
      `;

      // Should have exactly 2 file records for the same file key
      assert.strictEqual(
        allFileRecords.length,
        2,
        "Should have exactly two file records after both storage requests"
      );

      // Both file records should have is_in_bucket = true
      assert.strictEqual(
        allFileRecords[0].is_in_bucket,
        true,
        "First file record should have is_in_bucket = true"
      );
      assert.strictEqual(
        allFileRecords[1].is_in_bucket,
        true,
        "Second file record should have is_in_bucket = true (inherited from first record)"
      );

      // Additional verification: The second record is newer
      assert(
        allFileRecords[1].created_at >= allFileRecords[0].created_at,
        "Second file record should be created after or at the same time as the first"
      );

      // Verify both records have the same file key
      const firstRecordFileKey = `0x${allFileRecords[0].file_key.toString("hex")}`;
      const secondRecordFileKey = `0x${allFileRecords[1].file_key.toString("hex")}`;
      assert.strictEqual(
        firstRecordFileKey,
        secondRecordFileKey,
        "Both file records should have the same file key"
      );
      assert.strictEqual(
        firstRecordFileKey,
        firstFile.fileKey,
        "File keys should match the original storage request"
      );
    });

    it("bucket deletion sets is_in_bucket to false for all file records with same file key", async () => {
      // This test verifies that when a bucket deletion (deleteFiles with bspId = null) occurs,
      // ALL file records with the same file key have their is_in_bucket field set to false.
      // We manually submit the deleteFiles extrinsic instead of relying on the fisherman.

      const source = "res/cloud.jpg";
      const destination = "test/is-in-bucket-deletion-test.jpg";
      const bucketName = "is-in-bucket-deletion-bucket";

      const mspId = userApi.shConsts.DUMMY_MSP_ID;
      const valueProps = await userApi.call.storageProvidersApi.queryValuePropositionsForMsp(mspId);
      const valuePropId = valueProps[0].id;

      // Step 1: Create bucket and issue first storage request
      const firstFile: FileMetadata = await userApi.file.createBucketAndSendNewStorageRequest(
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
      await mspApi.wait.fileStorageComplete(firstFile.fileKey);
      await userApi.wait.mspResponseInTxPool();

      // Wait for BSP to volunteer and store
      await userApi.wait.bspVolunteer(1);
      await userApi.assert.eventPresent("fileSystem", "MspAcceptedStorageRequest");

      const bspAccount = userApi.createType("Address", userApi.accounts.bspKey.address);
      await userApi.wait.bspStored({ expectedExts: 1, bspAccount });

      await indexerApi.indexer.waitForIndexing({ producerApi: userApi, sql });

      // Wait for first file to be indexed
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
      assert.strictEqual(firstFileRecords.length, 1, "Should have one file record");
      assert.strictEqual(
        firstFileRecords[0].is_in_bucket,
        true,
        "First file record should have is_in_bucket = true"
      );

      // Onboard BSP2 after first storage request completes so BSP1 handles first request
      // and BSP2 is available for the second request's additional slot
      await userApi.docker.onboardBsp({
        bspSigner: bspTwoKey,
        name: "sh-bsp-two",
        bspId: ShConsts.BSP_TWO_ID,
        additionalArgs: ["--keystore-path=/keystore/bsp-two"],
        waitForIdle: true
      });

      // Step 3: Issue second storage request for same file (creates second file record)
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

      // Wait for MSP to accept second request
      await mspApi.wait.fileStorageComplete(firstFile.fileKey);
      await userApi.wait.mspResponseInTxPool();

      // Wait for BSP2 to volunteer for the additional slot (replication target increased to 2)
      await userApi.wait.bspVolunteer(1);
      await userApi.assert.eventPresent("fileSystem", "MspAcceptedStorageRequest");

      // Wait for BSP2 to confirm storage
      const bspTwoAccount = userApi.createType("Address", bspTwoKey.address);
      await userApi.wait.bspStored({
        expectedExts: 1,
        bspAccount: bspTwoAccount
      });

      await indexerApi.indexer.waitForIndexing({ producerApi: userApi, sql });

      // Wait for second file record to have BSP association
      await indexerApi.indexer.waitForBspFileAssociation({
        sql,
        fileKey: firstFile.fileKey
      });

      // Verify we now have two file records, both with is_in_bucket = true
      const twoFileRecords = await sql`
        SELECT id, file_key, is_in_bucket FROM file 
        WHERE file_key = ${hexToBuffer(firstFile.fileKey)}
        ORDER BY created_at ASC
      `;
      assert.strictEqual(twoFileRecords.length, 2, "Should have two file records");
      assert.strictEqual(
        twoFileRecords[0].is_in_bucket,
        true,
        "First record should have is_in_bucket = true"
      );
      assert.strictEqual(
        twoFileRecords[1].is_in_bucket,
        true,
        "Second record should have is_in_bucket = true"
      );

      // Step 4: Create file deletion request with signed intention
      const fileOperationIntention = {
        fileKey: firstFile.fileKey,
        operation: { Delete: null }
      };
      const intentionCodec = userApi.createType(
        "PalletFileSystemFileOperationIntention",
        fileOperationIntention
      );
      const rawSignature = shUser.sign(intentionCodec.toU8a());
      const userSignature = userApi.createType("MultiSignature", {
        Sr25519: rawSignature
      });

      const fileDeletionRequest = {
        fileOwner: shUser.address,
        signedIntention: fileOperationIntention,
        signature: userSignature,
        bucketId: firstFile.bucketId,
        location: firstFile.location,
        size_: firstFile.fileSize,
        fingerprint: firstFile.fingerprint
      };

      // Step 5: Generate forest proof from MSP for the bucket
      const bucketIdOption: Option<H256> = userApi.createType("Option<H256>", firstFile.bucketId);
      const forestProof = await mspApi.rpc.storagehubclient.generateForestProof(bucketIdOption, [
        firstFile.fileKey
      ]);

      // Step 6: Call deleteFiles with bspId = null (bucket deletion)
      await userApi.block.seal({
        calls: [userApi.tx.fileSystem.deleteFiles([fileDeletionRequest], null, forestProof)],
        signer: shUser
      });

      // Wait for indexer to process MutationsApplied event
      await indexerApi.indexer.waitForIndexing({ producerApi: userApi, sql });

      // Step 7: Verify ALL file records have is_in_bucket = false
      const allFileRecordsAfterDeletion = await sql`
        SELECT id, file_key, is_in_bucket FROM file 
        WHERE file_key = ${hexToBuffer(firstFile.fileKey)}
      `;

      // Both file records should still exist (BSP association remains)
      assert.strictEqual(
        allFileRecordsAfterDeletion.length,
        2,
        "Both file records should still exist after bucket deletion"
      );

      // All records should have is_in_bucket = false
      for (const record of allFileRecordsAfterDeletion) {
        assert.strictEqual(
          record.is_in_bucket,
          false,
          "All file records should have is_in_bucket = false after bucket deletion"
        );
      }

      // Verify BSP associations still exist (both BSP1 and BSP2)
      const bspAssociations = await sql`
        SELECT bf.* FROM bsp_file bf
        INNER JOIN file f ON bf.file_id = f.id
        WHERE f.file_key = ${hexToBuffer(firstFile.fileKey)}
      `;
      assert.strictEqual(
        bspAssociations.length,
        2,
        "Both BSP associations should still exist after bucket deletion"
      );
    });
  }
);
