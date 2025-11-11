import assert from "node:assert";
import type { ApiPromise } from "@polkadot/api";
import type { KeyringPair } from "@polkadot/keyring/types";
import { GenericAccountId } from "@polkadot/types";
import type { AccountId32, H256 } from "@polkadot/types/interfaces";
import { u8aToHex } from "@polkadot/util";
import type { HexString } from "@polkadot/util/types";
import { decodeAddress } from "@polkadot/util-crypto";
import type { EnrichedBspApi } from "./test-api";
import { assertEventPresent } from "../asserts";
import { waitFor } from "./waits";
import { sealBlock } from "./block";
import * as ShConsts from "./consts";
import type { FileMetadata } from "./types";

export const sendNewStorageRequest = async (
  api: ApiPromise,
  source: string,
  location: string,
  bucketId: H256,
  owner: KeyringPair,
  mspId?: HexString,
  replicationTarget?: number | null
): Promise<FileMetadata> => {
  const ownerHexString = u8aToHex(decodeAddress(ShConsts.NODE_INFOS.user.AddressId));
  const { file_metadata: fileMetadata } = await api.rpc.storagehubclient.loadFileInStorage(
    source,
    location,
    ownerHexString.slice(2),
    bucketId
  );

  const issueOwner = owner;

  let replicationTargetToUse: { Custom: number } | { Basic: null };
  if (replicationTarget) {
    replicationTargetToUse = {
      Custom: replicationTarget
    };
  } else {
    replicationTargetToUse = {
      Basic: null
    };
  }

  const issueStorageRequestResult = await sealBlock(
    api,
    api.tx.fileSystem.issueStorageRequest(
      bucketId,
      location,
      fileMetadata.fingerprint,
      fileMetadata.file_size,
      mspId ?? ShConsts.DUMMY_MSP_ID,
      [ShConsts.NODE_INFOS.user.expectedPeerId],
      replicationTargetToUse
    ),
    issueOwner
  );

  const accountId: AccountId32 = new GenericAccountId(api.registry, issueOwner.publicKey);

  const newStorageRequestEvent = assertEventPresent(
    api,
    "fileSystem",
    "NewStorageRequest",
    issueStorageRequestResult.events
  );
  const newStorageRequestEventDataBlob =
    api.events.fileSystem.NewStorageRequest.is(newStorageRequestEvent.event) &&
    newStorageRequestEvent.event.data;

  assert(newStorageRequestEventDataBlob, "Event doesn't match Type");

  return {
    fileKey: newStorageRequestEventDataBlob.fileKey.toString(),
    bucketId: bucketId.toString(),
    location: newStorageRequestEventDataBlob.location.toString(),
    owner: accountId.toString(),
    fingerprint: fileMetadata.fingerprint.toHex(),
    fileSize: fileMetadata.file_size.toNumber()
  };
};

export const createBucketAndSendNewStorageRequest = async (
  api: ApiPromise,
  source: string,
  location: string,
  bucketName: string,
  owner: KeyringPair,
  valuePropId?: HexString | null,
  mspId?: HexString | null,
  replicationTarget?: number | null,
  finalizeBlock = true
): Promise<FileMetadata> => {
  let localValuePropId = valuePropId;
  const localOwner = owner;

  if (!localValuePropId) {
    const valueProps = await api.call.storageProvidersApi.queryValuePropositionsForMsp(
      mspId ?? ShConsts.DUMMY_MSP_ID
    );
    localValuePropId = valueProps[0].id.toHex() as HexString;

    if (!localValuePropId) {
      throw new Error("No value proposition found");
    }
  }

  const newBucketEventEvent = await createBucket(
    api,
    bucketName,
    localOwner,
    localValuePropId,
    mspId ?? ShConsts.DUMMY_MSP_ID
  );
  const newBucketEventDataBlob =
    api.events.fileSystem.NewBucket.is(newBucketEventEvent) && newBucketEventEvent.data;

  assert(newBucketEventDataBlob, "Event doesn't match Type");

  const ownerHexString = u8aToHex(decodeAddress(localOwner.address));
  const { file_metadata: fileMetadata } = await api.rpc.storagehubclient.loadFileInStorage(
    source,
    location,
    ownerHexString.slice(2),
    newBucketEventDataBlob.bucketId
  );

  let replicationTargetToUse: { Custom: number } | { Basic: null };
  if (replicationTarget) {
    replicationTargetToUse = {
      Custom: replicationTarget
    };
  } else {
    replicationTargetToUse = {
      Basic: null
    };
  }
  const issueStorageRequestResult = await sealBlock(
    api,
    api.tx.fileSystem.issueStorageRequest(
      newBucketEventDataBlob.bucketId,
      location,
      fileMetadata.fingerprint,
      fileMetadata.file_size,
      mspId ?? ShConsts.DUMMY_MSP_ID,
      [ShConsts.NODE_INFOS.user.expectedPeerId],
      replicationTargetToUse
    ),
    localOwner,
    undefined,
    undefined,
    finalizeBlock
  );

  const newStorageRequestEvent = assertEventPresent(
    api,
    "fileSystem",
    "NewStorageRequest",
    issueStorageRequestResult.events
  );
  const newStorageRequestEventDataBlob =
    api.events.fileSystem.NewStorageRequest.is(newStorageRequestEvent.event) &&
    newStorageRequestEvent.event.data;

  assert(newStorageRequestEventDataBlob, "Event doesn't match Type");

  return {
    fileKey: newStorageRequestEventDataBlob.fileKey.toString(),
    bucketId: newBucketEventDataBlob.bucketId.toString(),
    location: newStorageRequestEventDataBlob.location.toString(),
    owner: newBucketEventDataBlob.who.toString(),
    fingerprint: fileMetadata.fingerprint.toHex(),
    fileSize: fileMetadata.file_size.toNumber()
  };
};

export const createBucket = async (
  api: ApiPromise,
  bucketName: string,
  owner: KeyringPair,
  valuePropId?: HexString | null,
  mspId: HexString | null = ShConsts.DUMMY_MSP_ID
) => {
  let localValuePropId = valuePropId;

  if (localValuePropId === undefined) {
    const valueProps = await api.call.storageProvidersApi.queryValuePropositionsForMsp(
      mspId ?? ShConsts.DUMMY_MSP_ID
    );

    localValuePropId = valueProps[0].id.toHex() as HexString;
  }

  if (localValuePropId === undefined || localValuePropId === null) {
    throw new Error("No value proposition found");
  }

  const createBucketResult = await sealBlock(
    api,
    api.tx.fileSystem.createBucket(
      mspId ?? ShConsts.DUMMY_MSP_ID,
      bucketName,
      false,
      localValuePropId
    ),
    owner
  );
  const { event } = assertEventPresent(api, "fileSystem", "NewBucket", createBucketResult.events);

  return event;
};

/**
 * Configuration for a single file in a batch storage request.
 */
export interface BatchFileConfig {
  /** Source file path (local file to load) */
  source: string;
  /** Destination/location path for the file */
  destination: string;
  /** Either a bucket name (will create bucket) or bucket ID (use existing bucket) */
  bucketIdOrName: string | H256;
  /** Optional replication target. If not provided, uses Basic replication target */
  replicationTarget?: number | null;
}

/**
 * Options for batch storage requests.
 */
export interface BatchStorageRequestsOptions {
  /** Array of file configurations */
  files: BatchFileConfig[];
  /** MSP ID to use for storage requests */
  mspId?: HexString | null;
  /** Value proposition ID. If not provided, will be fetched from MSP */
  valuePropId?: HexString | null;
  /** Owner/signer for the storage requests (required) */
  owner: KeyringPair;
  /** Whether to finalize blocks */
  finaliseBlock?: boolean;
  /** BSP API instance for waiting for file storage (required) */
  bspApi: EnrichedBspApi;
  /** MSP API instance for waiting for file storage and catchup (required) */
  mspApi: EnrichedBspApi;
  /** Maximum attempts for waiting for confirmations */
  maxAttempts?: number;
}

/**
 * Result of batch storage requests operation.
 */
export interface BatchStorageRequestsResult {
  /** Array of file keys created */
  fileKeys: string[];
  /** Array of bucket IDs (one per file) */
  bucketIds: string[];
  /** Array of file locations */
  locations: string[];
  /** Array of file fingerprints */
  fingerprints: string[];
  /** Array of file sizes */
  fileSizes: number[];
}

/**
 * Batches multiple storage requests together for efficient processing.
 *
 * This function handles the complete flow where both BSP and MSP respond:
 * 1. Creates buckets if bucket names are provided (deduplicates unique bucket names)
 * 2. Prepares all storage request transactions for the provided files
 * 3. Pauses MSP1 container to deterministically control storage request flow
 * 4. Seals all storage requests in a single block (finalized or unfinalized based on `finaliseBlock`)
 * 5. Waits for all BSP volunteers to appear in tx pool
 * 6. Processes BSP confirmations in batches (handles batched extrinsics)
 * 7. Verifies all files are confirmed by BSP
 * 8. Waits for BSP to store all files locally
 * 9. Unpauses MSP1 container
 * 10. Waits for MSP to catch up to chain tip
 * 11. Processes MSP acceptances in batches (handles batched extrinsics)
 * 12. Verifies all files are accepted by MSP
 * 13. Waits for MSP to store all files locally
 * 14. Returns all file metadata (fileKeys, bucketIds, locations, fingerprints, fileSizes)
 *
 * **Purpose:**
 * This helper simplifies the common case of batch creating storage requests where both BSP and MSP
 * respond. For tests that need more granular control (e.g., BSP-only or MSP-only scenarios), write
 * custom logic instead of using this helper.
 *
 * **Parameter Requirements:**
 * - `bspApi` is required for verifying BSP file storage
 * - `mspApi` is required for MSP catchup and verifying MSP file storage
 * - `owner` is always required
 *
 * @param api - The API instance
 * @param options - Batch storage request options
 * @returns Promise resolving to batch storage request result with all file metadata
 */
export const batchStorageRequests = async (
  api: EnrichedBspApi,
  options: BatchStorageRequestsOptions
): Promise<BatchStorageRequestsResult> => {
  const {
    files,
    mspId = ShConsts.DUMMY_MSP_ID,
    valuePropId: providedValuePropId,
    owner,
    finaliseBlock = true,
    bspApi,
    mspApi,
    maxAttempts = 5
  } = options;

  if (!owner) {
    throw new Error("Owner is required for batchStorageRequests");
  }

  if (!bspApi) {
    throw new Error("bspApi is required for batchStorageRequests");
  }

  if (!mspApi) {
    throw new Error("mspApi is required for batchStorageRequests");
  }

  const localOwner = owner;
  const ownerHex = u8aToHex(decodeAddress(localOwner.address)).slice(2);

  // Get value proposition if not provided
  let valuePropId = providedValuePropId;
  if (!valuePropId) {
    const valueProps = await api.call.storageProvidersApi.queryValuePropositionsForMsp(
      mspId ?? ShConsts.DUMMY_MSP_ID
    );
    valuePropId = valueProps[0].id.toHex() as HexString;
    if (!valuePropId) {
      throw new Error("No value proposition found");
    }
  }

  const fileKeys: string[] = [];
  const bucketIds: string[] = [];
  const locations: string[] = [];
  const fingerprints: string[] = [];
  const fileSizes: number[] = [];
  const storageRequestTxs = [];

  // First pass: identify unique bucket names that need to be created
  const uniqueBucketNames = new Set<string>();
  for (const file of files) {
    if (typeof file.bucketIdOrName === "string") {
      uniqueBucketNames.add(file.bucketIdOrName);
    }
  }

  // Derive bucket IDs and check if they exist on-chain
  const bucketNameToIdMap = new Map<string, H256>();
  const bucketsToCreate = new Set<string>();

  for (const bucketName of uniqueBucketNames) {
    // Derive the bucket ID deterministically (same logic as runtime):
    // Hash(owner.encode() ++ bucket_name.encode())
    const ownerEncoded = api.createType("AccountId32", localOwner.address).toU8a();
    const nameEncoded = api.createType("Bytes", bucketName).toU8a();
    const concat = new Uint8Array([...ownerEncoded, ...nameEncoded]);
    const bucketId = api.createType("H256", api.registry.hash(concat));

    // Check if bucket already exists on-chain
    const bucketOption = await api.query.providers.buckets(bucketId);

    if (bucketOption.isSome) {
      // Bucket already exists, reuse it
      bucketNameToIdMap.set(bucketName, bucketId);
    } else {
      // Bucket doesn't exist, mark it for creation
      bucketsToCreate.add(bucketName);
      bucketNameToIdMap.set(bucketName, bucketId);
    }
  }

  // Create only the buckets that don't exist yet
  for (const bucketName of bucketsToCreate) {
    const newBucketEvent = await createBucket(
      api,
      bucketName,
      localOwner,
      valuePropId,
      mspId ?? ShConsts.DUMMY_MSP_ID
    );
    const newBucketEventData =
      api.events.fileSystem.NewBucket.is(newBucketEvent) && newBucketEvent.data;
    assert(newBucketEventData, "NewBucket event data not found");

    // Verify the derived bucket ID matches the one from the event
    const derivedBucketId = bucketNameToIdMap.get(bucketName);
    assert(
      derivedBucketId?.eq(newBucketEventData.bucketId),
      `Derived bucket ID ${derivedBucketId?.toHex()} doesn't match event bucket ID ${newBucketEventData.bucketId.toHex()}`
    );
  }

  // Second pass: prepare storage request transactions
  for (let i = 0; i < files.length; i++) {
    const file = files[i];
    let bucketId: H256;

    if (typeof file.bucketIdOrName === "string") {
      // Use the bucket we just created (look up by name)
      const createdBucketId = bucketNameToIdMap.get(file.bucketIdOrName);
      assert(createdBucketId, `Bucket ID not found for bucket name: ${file.bucketIdOrName}`);
      bucketId = createdBucketId;
    } else {
      // Use the provided bucket ID
      bucketId = file.bucketIdOrName;
    }

    const {
      file_key,
      file_metadata: { location, fingerprint, file_size }
    } = await api.rpc.storagehubclient.loadFileInStorage(
      file.source,
      file.destination,
      ownerHex,
      bucketId.toString()
    );

    fileKeys.push(file_key.toString());
    bucketIds.push(bucketId.toString());
    locations.push(location.toHex());
    fingerprints.push(fingerprint.toHex());
    fileSizes.push(file_size.toNumber());

    let replicationTargetToUse: { Custom: number } | { Basic: null };
    if (file.replicationTarget !== undefined && file.replicationTarget !== null) {
      replicationTargetToUse = { Custom: file.replicationTarget };
    } else {
      replicationTargetToUse = { Basic: null };
    }

    storageRequestTxs.push(
      api.tx.fileSystem.issueStorageRequest(
        bucketId,
        location,
        fingerprint,
        file_size,
        mspId ?? ShConsts.DUMMY_MSP_ID,
        [ShConsts.NODE_INFOS.user.expectedPeerId],
        replicationTargetToUse
      )
    );
  }

  // Seal all storage requests in a single block
  await sealBlock(api, storageRequestTxs, localOwner, undefined, undefined, finaliseBlock);

  // Wait for MSP to store all files
  for (const fileKey of fileKeys) {
    await waitFor({
      lambda: async () =>
        (await mspApi.rpc.storagehubclient.isFileInFileStorage(fileKey)).isFileFound
    });
  }

  // Wait for MSP and BSP to catch up to the tip of the chain
  await api.wait.nodeCatchUpToChainTip(mspApi);
  await api.wait.nodeCatchUpToChainTip(bspApi);

  // Wait for all BSP volunteers, MSP acceptances and BSP stored confirmations
  // Process them incrementally by sealing blocks as extrinsics accumulate
  let totalVolunteers = 0;
  let totalAcceptance = 0;
  let totalConfirmations = 0;
  for (
    let attempt = 0;
    attempt < maxAttempts &&
    (totalVolunteers < fileKeys.length ||
      totalAcceptance < fileKeys.length ||
      totalConfirmations < fileKeys.length);
    attempt++
  ) {
    let volunteerFound = false;
    let mspFound = false;
    let bspFound = false;

    // Check for BSP volunteer extrinsics (only if we haven't received all volunteers yet)
    if (totalVolunteers < fileKeys.length) {
      try {
        await api.wait.bspVolunteerInTxPool(1);
        volunteerFound = true;
      } catch {}
    }

    // Check for MSP response extrinsics (only if we haven't received all acceptances yet)
    if (totalAcceptance < fileKeys.length) {
      try {
        await api.wait.mspResponseInTxPool(1, 3000);
        mspFound = true;
      } catch {}
    }

    // Check for BSP stored events (only if we haven't received all confirmations yet)
    if (totalConfirmations < fileKeys.length) {
      try {
        await api.wait.bspStored({
          sealBlock: false,
          timeoutMs: 3000
        });
        bspFound = true;
      } catch {}
    }

    const { events } = await api.block.seal({
      signer: localOwner,
      finaliseBlock: finaliseBlock
    });

    if (volunteerFound) {
      // Count BSP volunteer events
      const volunteerEvents = await api.assert.eventMany(
        "fileSystem",
        "AcceptedBspVolunteer",
        events
      );
      totalVolunteers += volunteerEvents.length;
    }

    if (mspFound) {
      const acceptEvents = await api.assert.eventMany(
        "fileSystem",
        "MspAcceptedStorageRequest",
        events
      );

      // Count total MspAcceptedStorageRequest events
      totalAcceptance += acceptEvents.length;
    }

    if (bspFound) {
      // Check if BSP confirmed storing events are present
      const confirmEvents = await api.assert.eventMany("fileSystem", "BspConfirmedStoring", events);

      // Count total file keys confirmed in all BspConfirmedStoring events
      for (const eventRecord of confirmEvents) {
        if (api.events.fileSystem.BspConfirmedStoring.is(eventRecord.event)) {
          totalConfirmations += eventRecord.event.data.confirmedFileKeys.length;
        }
      }
    }
  }

  assert.strictEqual(
    totalVolunteers,
    fileKeys.length,
    `Expected ${fileKeys.length} BSP volunteers, but got ${totalVolunteers}`
  );

  assert.strictEqual(
    totalAcceptance,
    fileKeys.length,
    `Expected ${fileKeys.length} MSP acceptance, but got ${totalAcceptance}`
  );

  assert.strictEqual(
    totalConfirmations,
    fileKeys.length,
    `Expected ${fileKeys.length} BSP confirmations, but got ${totalConfirmations}`
  );
  // Verify files are in BSP or MSP forests or file storages

  await waitFor({
    lambda: async () => {
      for (let index = 0; index < fileKeys.length; index++) {
        const fileKey = fileKeys[index];
        const bucketId = bucketIds[index];
        // Check file IS in BSP forest
        const bspForestResult = await bspApi.rpc.storagehubclient.isFileInForest(null, fileKey);
        if (!bspForestResult.isTrue) {
          return false;
        }

        // Check file IS in BSP file storage
        const bspFileStorageResult = await bspApi.rpc.storagehubclient.isFileInFileStorage(fileKey);
        if (!bspFileStorageResult.isFileFound) {
          return false;
        }

        // Check file IS in MSP forest
        const mspForestResult = await mspApi.rpc.storagehubclient.isFileInForest(bucketId, fileKey);
        if (!mspForestResult.isTrue) {
          return false;
        }

        // Check file IS in MSP file storage
        const mspFileStorageResult = await mspApi.rpc.storagehubclient.isFileInFileStorage(fileKey);
        if (!mspFileStorageResult.isFileFound) {
          return false;
        }
      }
      return true;
    }
  });

  return {
    fileKeys,
    bucketIds,
    locations,
    fingerprints,
    fileSizes
  };
};
