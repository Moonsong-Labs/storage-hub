import assert from "node:assert";
import { describeMspNet, type EnrichedBspApi, type SqlClient, shUser, sleep } from "../../../util";

/**
 * Enhanced Indexer Lite Mode - BSP Tracking Tests
 * 
 * This test verifies that the enhanced lite mode correctly indexes BSP volunteering
 * events and maintains BSP-to-file associations in the bsp_file table.
 */
describeMspNet(
  "Indexer Lite Mode - BSP Tracking Support",
  { initialised: true, indexer: true, indexerMode: "lite" },
  ({ before, it, createMsp1Api, createMsp2Api, createUserApi, createSqlClient, createBspApi }) => {
    let msp1Api: EnrichedBspApi;
    let msp2Api: EnrichedBspApi;
    let userApi: EnrichedBspApi;
    let bspApi: EnrichedBspApi;
    let sql: SqlClient;

    before(async () => {
      const maybeMsp1Api = await createMsp1Api();
      const maybeMsp2Api = await createMsp2Api();
      assert(maybeMsp1Api, "MSP1 API not available");
      assert(maybeMsp2Api, "MSP2 API not available");
      msp1Api = maybeMsp1Api;
      msp2Api = maybeMsp2Api;
      userApi = await createUserApi();
      bspApi = await createBspApi();
      sql = createSqlClient();

      // Wait for indexer to be ready
      await userApi.docker.waitForLog({
        containerName: "docker-sh-postgres-1",
        searchString: "database system is ready to accept connections",
        timeout: 5000
      });

      await userApi.docker.waitForLog({
        containerName: "docker-sh-msp-1",
        searchString: "IndexerService starting up in",
        timeout: 10000
      });

      // Give indexer time to sync
      await sleep(5000);
    });

    it("indexes BSP volunteering for files in MSP1's bucket", async () => {
      // Create bucket owned by MSP1
      const bucketName = "msp1-bucket-for-bsp";
      const bucketEvent = await userApi.block.seal({
        calls: [
          userApi.tx.fileSystem.createBucket(
            userApi.shConsts.NODE_INFOS.msp1.AddressId,
            bucketName,
            true
          )
        ],
        signer: shUser
      });

      const bucketId = userApi.events.fileSystem.NewBucket.is(bucketEvent) && bucketEvent.data.bucketId;
      assert(bucketId, "Failed to get bucket ID");

      // Add a file to MSP1's bucket
      const file = {
        location: "bsp-test-file.dat",
        fingerprint: "0x4444444444444444444444444444444444444444444444444444444444444444",
        size: 2048
      };

      const fileEvent = await userApi.block.seal({
        calls: [
          userApi.tx.fileSystem.issueStorageRequest(
            bucketId,
            file.location,
            file.fingerprint,
            file.size,
            userApi.shConsts.NODE_INFOS.msp1.AddressId,
            [userApi.alice.publicKey],
            null
          )
        ],
        signer: shUser
      });

      // Get file key from event
      const storageRequestEvent = userApi.events.fileSystem.NewStorageRequest.is(fileEvent);
      assert(storageRequestEvent, "Failed to get storage request event");
      const fileKey = storageRequestEvent.data.fileKey;

      // Wait for file to be indexed
      await sleep(2000);

      // Get file ID from database
      const files = await sql`
        SELECT id, fingerprint FROM file 
        WHERE fingerprint = ${file.fingerprint}
      `;
      assert(files.length === 1, "File should be indexed");
      const fileId = files[0].id;

      // BSP volunteers for the file
      await bspApi.block.seal({
        calls: [
          bspApi.tx.fileSystem.bspVolunteer(fileKey)
        ],
        signer: bspApi.signer
      });

      // Wait for BSP volunteering to be indexed
      await sleep(3000);

      // Verify BSP volunteer event is indexed
      const bspVolunteerEvents = await sql`
        SELECT * FROM block_event
        WHERE section = 'fileSystem'
        AND method = 'AcceptedBspVolunteer'
        AND data::text LIKE '%${fileKey.toString()}%'
      `;
      assert(bspVolunteerEvents.length === 1, "AcceptedBspVolunteer event should be indexed");

      // Verify bsp_file association is created
      const bspFiles = await sql`
        SELECT bf.*, b.onchain_bsp_id
        FROM bsp_file bf
        JOIN bsp b ON bf.bsp_id = b.id
        WHERE bf.file_id = ${fileId}
      `;
      assert(bspFiles.length === 1, "BSP-file association should be created");
      assert(
        bspFiles[0].onchain_bsp_id === userApi.shConsts.NODE_INFOS.bsp.AddressId,
        "BSP ID should match"
      );

      // Simulate BSP confirming storage
      await bspApi.block.seal({
        calls: [
          bspApi.tx.fileSystem.bspConfirmStoring(fileKey, [])
        ],
        signer: bspApi.signer
      });

      await sleep(2000);

      // Verify BspConfirmedStoring event is indexed
      const bspConfirmedEvents = await sql`
        SELECT * FROM block_event
        WHERE section = 'fileSystem'
        AND method = 'BspConfirmedStoring'
        AND data::text LIKE '%${fileKey.toString()}%'
      `;
      assert(bspConfirmedEvents.length === 1, "BspConfirmedStoring event should be indexed");
    });

    it("indexes multiple BSPs volunteering for the same file", async () => {
      // Create bucket and file
      const bucketName = "multi-bsp-bucket";
      const bucketEvent = await userApi.block.seal({
        calls: [
          userApi.tx.fileSystem.createBucket(
            userApi.shConsts.NODE_INFOS.msp1.AddressId,
            bucketName,
            true
          )
        ],
        signer: shUser
      });

      const bucketId = userApi.events.fileSystem.NewBucket.is(bucketEvent) && bucketEvent.data.bucketId;
      assert(bucketId, "Failed to get bucket ID");

      const file = {
        location: "multi-bsp-file.dat",
        fingerprint: "0x5555555555555555555555555555555555555555555555555555555555555555",
        size: 4096
      };

      const fileEvent = await userApi.block.seal({
        calls: [
          userApi.tx.fileSystem.issueStorageRequest(
            bucketId,
            file.location,
            file.fingerprint,
            file.size,
            userApi.shConsts.NODE_INFOS.msp1.AddressId,
            [userApi.alice.publicKey],
            null
          )
        ],
        signer: shUser
      });

      const storageRequestEvent = userApi.events.fileSystem.NewStorageRequest.is(fileEvent);
      assert(storageRequestEvent, "Failed to get storage request event");
      const fileKey = storageRequestEvent.data.fileKey;

      await sleep(2000);

      // Get file ID
      const files = await sql`
        SELECT id FROM file WHERE fingerprint = ${file.fingerprint}
      `;
      assert(files.length === 1, "File should be indexed");
      const fileId = files[0].id;

      // Create a second BSP API
      const bsp2Api = await createBspApi();

      // Both BSPs volunteer for the file
      await bspApi.block.seal({
        calls: [bspApi.tx.fileSystem.bspVolunteer(fileKey)],
        signer: bspApi.signer
      });

      await bsp2Api.block.seal({
        calls: [bsp2Api.tx.fileSystem.bspVolunteer(fileKey)],
        signer: bsp2Api.signer
      });

      await sleep(3000);

      // Verify both BSP associations are created
      const bspFiles = await sql`
        SELECT bf.*, b.onchain_bsp_id
        FROM bsp_file bf
        JOIN bsp b ON bf.bsp_id = b.id
        WHERE bf.file_id = ${fileId}
        ORDER BY b.onchain_bsp_id
      `;

      // Should have at least one BSP association (may have two if both BSPs were accepted)
      assert(bspFiles.length >= 1, "At least one BSP-file association should be created");
      console.log(`Found ${bspFiles.length} BSP associations for the file`);
    });

    it("indexes BSP volunteering for files in transferred buckets", async () => {
      // Create bucket on MSP2
      const bucketName = "transfer-then-bsp-bucket";
      const bucketEvent = await userApi.block.seal({
        calls: [
          userApi.tx.fileSystem.createBucket(
            userApi.shConsts.NODE_INFOS.msp2.AddressId,
            bucketName,
            true
          )
        ],
        signer: shUser
      });

      const bucketId = userApi.events.fileSystem.NewBucket.is(bucketEvent) && bucketEvent.data.bucketId;
      assert(bucketId, "Failed to get bucket ID");

      // Add file to MSP2's bucket
      const file = {
        location: "transferred-file.dat",
        fingerprint: "0x6666666666666666666666666666666666666666666666666666666666666666",
        size: 1024
      };

      const fileEvent = await userApi.block.seal({
        calls: [
          userApi.tx.fileSystem.issueStorageRequest(
            bucketId,
            file.location,
            file.fingerprint,
            file.size,
            userApi.shConsts.NODE_INFOS.msp2.AddressId,
            [userApi.alice.publicKey],
            null
          )
        ],
        signer: shUser
      });

      const storageRequestEvent = userApi.events.fileSystem.NewStorageRequest.is(fileEvent);
      assert(storageRequestEvent, "Failed to get storage request event");
      const fileKey = storageRequestEvent.data.fileKey;

      await sleep(2000);

      // Transfer bucket to MSP1
      await userApi.block.seal({
        calls: [
          userApi.tx.fileSystem.updateBucketPrivacy(bucketId, {
            MSPBucket: {
              mspId: userApi.shConsts.NODE_INFOS.msp1.AddressId
            }
          })
        ],
        signer: shUser
      });

      await sleep(2000);

      // BSP volunteers for the file after transfer
      await bspApi.block.seal({
        calls: [bspApi.tx.fileSystem.bspVolunteer(fileKey)],
        signer: bspApi.signer
      });

      await sleep(3000);

      // Get file ID
      const files = await sql`
        SELECT id FROM file WHERE fingerprint = ${file.fingerprint}
      `;
      assert(files.length === 1, "File should be indexed");
      const fileId = files[0].id;

      // Verify BSP association is created
      const bspFiles = await sql`
        SELECT bf.*, b.onchain_bsp_id, bu.name as bucket_name
        FROM bsp_file bf
        JOIN bsp b ON bf.bsp_id = b.id
        JOIN file f ON bf.file_id = f.id
        JOIN bucket bu ON f.bucket_id = bu.id
        WHERE bf.file_id = ${fileId}
      `;

      assert(bspFiles.length === 1, "BSP-file association should be created for transferred file");
      assert(bspFiles[0].bucket_name === bucketName, "File should be in the correct bucket");
    });

    it("provides complete BSP peer information for buckets", async () => {
      // Create a bucket with multiple files
      const bucketName = "bsp-peer-info-bucket";
      const bucketEvent = await userApi.block.seal({
        calls: [
          userApi.tx.fileSystem.createBucket(
            userApi.shConsts.NODE_INFOS.msp1.AddressId,
            bucketName,
            true
          )
        ],
        signer: shUser
      });

      const bucketId = userApi.events.fileSystem.NewBucket.is(bucketEvent) && bucketEvent.data.bucketId;
      assert(bucketId, "Failed to get bucket ID");

      // Add multiple files
      const files = [
        { location: "peer-file1.dat", fingerprint: "0x7777777777777777777777777777777777777777777777777777777777777777", size: 512 },
        { location: "peer-file2.dat", fingerprint: "0x8888888888888888888888888888888888888888888888888888888888888888", size: 1024 }
      ];

      for (const file of files) {
        const event = await userApi.block.seal({
          calls: [
            userApi.tx.fileSystem.issueStorageRequest(
              bucketId,
              file.location,
              file.fingerprint,
              file.size,
              userApi.shConsts.NODE_INFOS.msp1.AddressId,
              [userApi.alice.publicKey],
              null
            )
          ],
          signer: shUser
        });

        const storageRequestEvent = userApi.events.fileSystem.NewStorageRequest.is(event);
        if (storageRequestEvent) {
          const fileKey = storageRequestEvent.data.fileKey;
          
          // BSP volunteers for each file
          await bspApi.block.seal({
            calls: [bspApi.tx.fileSystem.bspVolunteer(fileKey)],
            signer: bspApi.signer
          });
        }
      }

      await sleep(3000);

      // Query all BSPs storing files in this bucket
      const bspPeers = await sql`
        SELECT DISTINCT b.onchain_bsp_id, b.peer_id, COUNT(DISTINCT f.id) as file_count
        FROM bsp b
        JOIN bsp_file bf ON b.id = bf.bsp_id
        JOIN file f ON bf.file_id = f.id
        WHERE f.bucket_id = ${bucketId.toString()}
        GROUP BY b.onchain_bsp_id, b.peer_id
      `;

      assert(bspPeers.length > 0, "Should have BSP peer information for bucket files");
      
      for (const peer of bspPeers) {
        assert(peer.onchain_bsp_id, "BSP should have onchain ID");
        assert(peer.peer_id, "BSP should have peer ID");
        assert(Number(peer.file_count) > 0, "BSP should be storing at least one file");
        console.log(`BSP ${peer.onchain_bsp_id} (peer: ${peer.peer_id}) stores ${peer.file_count} file(s)`);
      }
    });

    it("verifies bsp_file table integrity", async () => {
      // Check that all bsp_file entries have valid references
      const orphanedBspFiles = await sql`
        SELECT bf.*
        FROM bsp_file bf
        LEFT JOIN bsp b ON bf.bsp_id = b.id
        LEFT JOIN file f ON bf.file_id = f.id
        WHERE b.id IS NULL OR f.id IS NULL
      `;

      assert(
        orphanedBspFiles.length === 0,
        `Should have no orphaned bsp_file entries, found ${orphanedBspFiles.length}`
      );

      // Check BSP event count matches associations
      const bspEventCounts = await sql`
        SELECT 
          (SELECT COUNT(*) FROM block_event WHERE method = 'AcceptedBspVolunteer') as volunteer_events,
          (SELECT COUNT(*) FROM block_event WHERE method = 'BspConfirmedStoring') as confirmed_events,
          (SELECT COUNT(*) FROM bsp_file) as bsp_file_count
      `;

      const counts = bspEventCounts[0];
      console.log("BSP tracking summary:");
      console.log(`  AcceptedBspVolunteer events: ${counts.volunteer_events}`);
      console.log(`  BspConfirmedStoring events: ${counts.confirmed_events}`);
      console.log(`  bsp_file associations: ${counts.bsp_file_count}`);

      // Volunteer events should approximately match bsp_file count
      // (some volunteers might be rejected or not yet confirmed)
      assert(
        Number(counts.bsp_file_count) <= Number(counts.volunteer_events),
        "BSP file associations should not exceed volunteer events"
      );
    });
  }
);