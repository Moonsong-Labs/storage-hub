/**
 * Benchmark test: MSP distributes file to N BSPs
 *
 * This test verifies that an MSP can distribute a file to multiple BSPs in parallel.
 * It uses the dynamic network infrastructure to launch a network with N BSPs and 1 MSP.
 *
 * Test flow:
 * 1. Set replication target to BSP_COUNT
 * 2. Create bucket and load file into user storage
 * 3. Issue storage request
 * 4. Wait for MSP to download file and accept storage request
 * 5. Delete file from user node (so BSPs must get it from MSP)
 * 6. Verify BSPs volunteered and wait for them to confirm storing
 * 7. Verify all BSPs have the file in their storage
 */

import assert, { strictEqual } from "node:assert";
import { u8aToHex } from "@polkadot/util";
import { decodeAddress } from "@polkadot/util-crypto";
import type { EnrichedBspApi } from "../../../util";
import { describeNetwork } from "../../../util/netLaunch/dynamic/testrunner";

/**
 * Number of BSPs to run in this benchmark test.
 * All checks and assertions will use this value.
 */
const BSP_COUNT = 10;

await describeNetwork(
  `MSP distributes files to ${BSP_COUNT} BSPs`,
  {
    bsps: BSP_COUNT,
    msps: 1,
    fishermen: 0
  },
  {
    timeout: 600000 // 10 minutes for network startup + test
  },
  (ctx) => {
    let userApi: EnrichedBspApi;
    let mspApi: EnrichedBspApi;
    let blockProducerApi: EnrichedBspApi;

    ctx.before(async () => {
      userApi = await ctx.network.getUserApi(0);
      mspApi = await ctx.network.getMspApi(0);
      blockProducerApi = await ctx.network.getBlockProducerApi();
    });

    ctx.it("Network launches and can be queried", async () => {
      // Verify we have the expected number of BSPs
      strictEqual(ctx.network.bspCount, BSP_COUNT, `Expected ${BSP_COUNT} BSP nodes`);

      // Verify all BSPs have unique peer IDs
      const peerIds = new Set<string>();
      await ctx.network.forEachBsp(async (api, index) => {
        const peerId = await api.rpc.system.localPeerId();
        const peerIdStr = peerId.toString();
        assert.ok(!peerIds.has(peerIdStr), `BSP ${index} has duplicate peer ID: ${peerIdStr}`);
        peerIds.add(peerIdStr);
      });
      strictEqual(peerIds.size, BSP_COUNT, `Expected ${BSP_COUNT} unique peer IDs`);

      // Verify MSP is available
      const mspPeerId = await mspApi.rpc.system.localPeerId();
      assert.ok(mspPeerId.toString().length > 0, "MSP should have a valid peer ID");
    });

    ctx.it(`MSP distributes file to ${BSP_COUNT} BSPs correctly`, async () => {
      const source = "res/whatsup.jpg";
      const destination = "test/whatsup.jpg";
      const bucketName = "distribution-benchmark-bucket";
      let bspsPaused = false;

      // Match the stable MSP distribution flow:
      // keep BSPs paused until user file is deleted, so BSPs can only receive via MSP.
      for (let i = 0; i < BSP_COUNT; i++) {
        await userApi.docker.pauseContainer(`storage-hub-sh-bsp-${i}`);
      }
      bspsPaused = true;

      try {
        // Step 1: Set replication target to BSP_COUNT (all BSPs will volunteer once resumed)
        await blockProducerApi.block.seal({
          calls: [
            blockProducerApi.tx.sudo.sudo(
              blockProducerApi.tx.parameters.setParameter({
                RuntimeConfig: { MaxReplicationTarget: [null, BSP_COUNT] }
              })
            )
          ]
        });

        // Get identities for later use
        const userIdentity = ctx.network.getUserIdentity(0);
        const mspProviderId = ctx.network.getMspProviderId(0);

        // Step 2: Create bucket using the file helper API
        // Note: file.newBucket works with dynamic networks since it uses the provided keyring
        const newBucketEvent = await blockProducerApi.file.newBucket(
          bucketName,
          userIdentity.identity.keyring,
          undefined, // valuePropId - will be fetched automatically (must be undefined, not null)
          mspProviderId
        );
        const newBucketEventDataBlob =
          blockProducerApi.events.fileSystem.NewBucket.is(newBucketEvent) && newBucketEvent.data;
        assert(newBucketEventDataBlob, "NewBucket event doesn't match expected type");

        // Get user's address for loading file (required for dynamic network context)
        const ownerHex = u8aToHex(decodeAddress(userIdentity.identity.keyring.address)).slice(2);

        // Load file into storage
        const {
          file_metadata: { location, fingerprint, file_size }
        } = await userApi.rpc.storagehubclient.loadFileInStorage(
          source,
          destination,
          ownerHex,
          newBucketEventDataBlob.bucketId
        );
        strictEqual(location.toHuman(), destination);

        // Get peer IDs for storage request (both user and MSP so file can be fetched from either)
        const userPeerId = (await userApi.rpc.system.localPeerId()).toString();
        const mspPeerId = (await mspApi.rpc.system.localPeerId()).toString();

        // Step 3: Issue storage request with custom replication target matching BSP_COUNT
        await blockProducerApi.block.seal({
          calls: [
            blockProducerApi.tx.fileSystem.issueStorageRequest(
              newBucketEventDataBlob.bucketId,
              destination,
              fingerprint.toHex(),
              file_size.toBigInt(),
              mspProviderId,
              [userPeerId, mspPeerId],
              { Custom: BSP_COUNT }
            )
          ],
          signer: userIdentity.identity.keyring
        });

        // Get the file key from the NewStorageRequest event
        const { event: newStorageRequestEvent } = await blockProducerApi.assert.eventPresent(
          "fileSystem",
          "NewStorageRequest"
        );
        const newStorageRequestDataBlob =
          blockProducerApi.events.fileSystem.NewStorageRequest.is(newStorageRequestEvent) &&
          newStorageRequestEvent.data;
        assert(
          newStorageRequestDataBlob,
          "NewStorageRequest event data does not match expected type"
        );
        const fileKey = newStorageRequestDataBlob.fileKey.toString();

        // Wait for MSP to sync with the chain tip before expecting it to process the storage request
        await blockProducerApi.wait.nodeCatchUpToChainTip(mspApi);

        // Step 4: Wait for MSP to download and accept before any BSP can volunteer.
        await mspApi.wait.fileStorageComplete(fileKey);
        await blockProducerApi.wait.mspResponseInTxPool(1, 60000);
        await blockProducerApi.block.seal();

        // Verify MSP accepted - use blockProducerApi since it produced the block
        const { event: mspAcceptedEvent } = await blockProducerApi.assert.eventPresent(
          "fileSystem",
          "MspAcceptedStorageRequest"
        );
        const mspAcceptedDataBlob =
          blockProducerApi.events.fileSystem.MspAcceptedStorageRequest.is(mspAcceptedEvent) &&
          mspAcceptedEvent.data;
        assert(mspAcceptedDataBlob, "MspAcceptedStorageRequest event data does not match type");
        strictEqual(mspAcceptedDataBlob.fileKey.toString(), fileKey);

        // Step 5: Delete file from user node before BSPs can volunteer.
        // This ensures BSPs must receive the file from MSP only.
        await userApi.rpc.storagehubclient.removeFilesFromFileStorage([fileKey]);
        await userApi.wait.fileDeletionFromFileStorage(fileKey);

        // Step 6: Resume BSPs now that user no longer has the file.
        for (let i = 0; i < BSP_COUNT; i++) {
          await userApi.docker.resumeContainer({ containerName: `storage-hub-sh-bsp-${i}` });
        }
        bspsPaused = false;

        // Ensure all BSPs catch up before expecting volunteer/confirm flows.
        await ctx.network.forEachBsp(async (bspApi) => {
          await blockProducerApi.wait.nodeCatchUpToChainTip(bspApi);
        });

        // Step 7: Verify BSPs volunteered and wait for them to confirm storing
        await blockProducerApi.wait.bspVolunteerInTxPool(BSP_COUNT, 60000);
        await blockProducerApi.block.seal();

        const volunteerEvents = await blockProducerApi.assert.eventMany(
          "fileSystem",
          "AcceptedBspVolunteer"
        );
        strictEqual(
          volunteerEvents.length,
          BSP_COUNT,
          `Expected ${BSP_COUNT} AcceptedBspVolunteer events`
        );

        // Wait for all BSPs to confirm storing.
        // Seal progressively to avoid deadlock when only a subset of confirmations
        // is initially in the tx pool.
        {
          const startedAt = Date.now();
          let totalConfirmations = 0;
          const normalizeHex = (value: string) =>
            (value.startsWith("0x") ? value : `0x${value}`).toLowerCase();
          const targetFileKey = normalizeHex(fileKey);

          while (Date.now() - startedAt < 120_000 && totalConfirmations < BSP_COUNT) {
            try {
              await blockProducerApi.wait.bspStored({ timeoutMs: 3000, sealBlock: false });
            } catch {
              // No confirm-storing extrinsics pending yet.
            }

            const { events } = await blockProducerApi.block.seal();

            for (const eventRecord of events ?? []) {
              if (!blockProducerApi.events.fileSystem.BspConfirmedStoring.is(eventRecord.event)) {
                continue;
              }
              const confirmedForTargetFile = eventRecord.event.data.confirmedFileKeys.filter(
                (entry) => {
                  const confirmedFileKey = Array.isArray(entry) ? entry[0] : entry;
                  return normalizeHex(confirmedFileKey.toString()) === targetFileKey;
                }
              ).length;
              totalConfirmations += confirmedForTargetFile;
            }
          }

          strictEqual(
            totalConfirmations,
            BSP_COUNT,
            `Expected ${BSP_COUNT} BspConfirmedStoring confirmations, got ${totalConfirmations}`
          );
        }

        // Step 8: Verify all BSPs have the file in their storage
        for (let i = 0; i < BSP_COUNT; i++) {
          const bspApi = await ctx.network.getBspApi(i);

          // Verify file is in BSP's file storage
          await bspApi.wait.fileStorageComplete(fileKey);

          // Verify file is in BSP's forest (direct check since there's no "waitForFileInForest" helper)
          const isInForest = await bspApi.rpc.storagehubclient.isFileInForest(null, fileKey);
          assert(isInForest.isTrue, `File not found in BSP ${i} forest`);
        }
      } finally {
        if (bspsPaused) {
          try {
            for (let i = 0; i < BSP_COUNT; i++) {
              await userApi.docker.resumeContainer({ containerName: `storage-hub-sh-bsp-${i}` });
            }
          } catch {
            // best-effort cleanup; network teardown will still remove containers
          }
        }
      }
    });
  }
);
