import assert from "node:assert";
import type { ApiPromise } from "@polkadot/api";
import type { KeyringPair } from "@polkadot/keyring/types";
import { GenericAccountId } from "@polkadot/types";
import type { AccountId32, H256, EventRecord } from "@polkadot/types/interfaces";
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
 * Result of a failed extrinsic check.
 */
export interface FailedExtrinsicResult {
  method: string;
  error: string;
  docs: string;
  /** True if the error is MspAlreadyConfirmed - indicates the file is already accepted */
  isAlreadyConfirmed?: boolean;
}

/**
 * Checks for failed extrinsics of specific types and logs error details
 * @param api - API instance
 * @param events - Events from sealed block
 * @param targetMethods - Map of method names to check (e.g., { 'mspAcceptStorageRequest': true })
 * @param context - Context string for logging (e.g., "[Attempt 1/10]")
 * @returns Array of failure details
 */
export const logFailedTargetExtrinsics = (
  api: ApiPromise,
  events: EventRecord[],
  targetMethods: { [key: string]: boolean },
  context = ""
): FailedExtrinsicResult[] => {
  const failures: FailedExtrinsicResult[] = [];

  // Track which extrinsic index corresponds to which method
  const extrinsicMethods: string[] = [];

  // First pass: identify all extrinsics and their methods
  for (const { phase } of events) {
    if (phase.isApplyExtrinsic) {
      // Get the extrinsic method from other events in same phase
      // This is simplified - in practice you'd need to track the actual extrinsic
      extrinsicMethods[phase.asApplyExtrinsic.toNumber()] = "unknown";
    }
  }

  // Check each failed extrinsic
  for (const { event, phase } of events) {
    if (api.events.system.ExtrinsicFailed.is(event) && phase.isApplyExtrinsic) {
      const extIndex = phase.asApplyExtrinsic.toNumber();
      const errorData = event.data.dispatchError;

      let errorString = "";
      let errorDocs = "";

      if (errorData.isModule) {
        const decoded = api.registry.findMetaError(errorData.asModule);
        errorString = `${decoded.section}::${decoded.name}`;
        errorDocs = decoded.docs.join(" ");

        // Check if this is an MSP or BSP related error based on the section
        if (decoded.section === "fileSystem") {
          // Log if it's likely MSP/BSP related
          if (
            targetMethods.mspAcceptStorageRequest &&
            (decoded.name.includes("Msp") || decoded.name.includes("AlreadyConfirmed"))
          ) {
            // Check specifically for MspAlreadyConfirmed - this is expected behavior
            // when the MSP tries to accept a file that was already confirmed
            const isAlreadyConfirmed = decoded.name === "MspAlreadyConfirmed";

            if (isAlreadyConfirmed) {
              console.log(
                `${context} MspAlreadyConfirmed at extrinsic ${extIndex} - file(s) already accepted, MSP will retry`
              );
            } else {
              console.log(
                `${context} MSP accept likely failed at extrinsic ${extIndex}: ${errorString}`
              );
              if (errorDocs) console.log(`    Details: ${errorDocs}`);
            }

            failures.push({
              method: "mspAcceptStorageRequest",
              error: errorString,
              docs: errorDocs,
              isAlreadyConfirmed
            });
          }

          if (targetMethods.bspConfirmStoring && decoded.name.includes("Bsp")) {
            console.log(
              `${context} BSP confirm likely failed at extrinsic ${extIndex}: ${errorString}`
            );
            if (errorDocs) console.log(`    Details: ${errorDocs}`);
            failures.push({ method: "bspConfirmStoring", error: errorString, docs: errorDocs });
          }
        }
      } else {
        errorString = errorData.toString();
        console.log(`${context} Extrinsic ${extIndex} failed: ${errorString}`);
        failures.push({ method: "unknown", error: errorString, docs: "" });
      }
    }
  }

  return failures;
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
  /** BSP API instance for waiting for file storage (optional - if not provided, BSP checks are skipped) */
  bspApi?: EnrichedBspApi;
  /** MSP API instance for waiting for file storage and catchup (optional - if not provided, MSP checks are skipped) */
  mspApi?: EnrichedBspApi;
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
 * This function handles the flow where BSP and/or MSP respond based on which APIs are provided:
 * 1. Creates buckets if bucket names are provided (deduplicates unique bucket names)
 * 2. Prepares all storage request transactions for the provided files
 * 3. Seals all storage requests in a single block (finalized or unfinalized based on `finaliseBlock`)
 * 4. If `bspApi` is provided:
 *    - Waits for all BSP volunteers to appear in tx pool
 *    - Processes BSP confirmations in batches (handles batched extrinsics)
 *    - Verifies all files are confirmed by BSP
 *    - Waits for BSP to store all files locally
 * 5. If `mspApi` is provided:
 *    - Waits for MSP to catch up to chain tip
 *    - Processes MSP acceptances in batches (handles batched extrinsics)
 *    - Verifies all files are accepted by MSP
 *    - Waits for MSP to store all files locally
 * 6. Returns all file metadata (fileKeys, bucketIds, locations, fingerprints, fileSizes)
 *
 * **Purpose:**
 * This helper simplifies batch creating storage requests. It can handle:
 * - Both BSP and MSP responding (pass both `bspApi` and `mspApi`)
 * - BSP-only scenarios (pass only `bspApi`)
 * - MSP-only scenarios (pass only `mspApi`)
 *
 * **Parameter Requirements:**
 * - At least one of `bspApi` or `mspApi` must be provided
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
    maxAttempts = 10
  } = options;

  if (!owner) {
    throw new Error("Owner is required for batchStorageRequests");
  }

  if (!bspApi && !mspApi) {
    throw new Error("At least one of bspApi or mspApi must be provided for batchStorageRequests");
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

  // Wait for MSP to store all files (if mspApi is provided)
  if (mspApi) {
    for (const fileKey of fileKeys) {
      await waitFor({
        lambda: async () =>
          (await mspApi.rpc.storagehubclient.isFileInFileStorage(fileKey)).isFileFound
      });
    }
  }

  // Wait for MSP and BSP to catch up to the tip of the chain
  if (mspApi) {
    await api.wait.nodeCatchUpToChainTip(mspApi);
  }
  if (bspApi) {
    await api.wait.nodeCatchUpToChainTip(bspApi);
  }

  // Wait for all BSP volunteers to appear in tx pool (if bspApi is provided)
  if (bspApi) {
    await api.wait.bspVolunteerInTxPool(fileKeys.length);
  }

  // Wait for MSP acceptance and/or BSP stored confirmations (if APIs are provided)
  // MSP batches extrinsics, so we need to iteratively seal blocks and count events
  let totalAcceptance = 0;
  let totalConfirmations = 0;
  const expectMspAcceptance = mspApi !== undefined;
  const expectBspConfirmations = bspApi !== undefined;

  let attempt = 0;
  if (expectMspAcceptance || expectBspConfirmations) {
    for (
      attempt = 0;
      attempt < maxAttempts &&
      ((expectMspAcceptance && totalAcceptance < fileKeys.length) ||
        (expectBspConfirmations && totalConfirmations < fileKeys.length));
      attempt++
    ) {
      // Wait for MSP response and/or BSP stored event, depending on what's expected
      // Wait up to 5 seconds for each process, giving time for both to run.
      // Try waits sequentially, only fail if none succeed (all time out)
      let mspFound = false;
      let bspFound = false;

      if (expectMspAcceptance && totalAcceptance < fileKeys.length) {
        try {
          await api.wait.mspResponseInTxPool(1);
          mspFound = true;
        } catch (_error) {
          console.log(`[Attempt ${attempt + 1}/${maxAttempts}] MSP response not in tx pool yet`);
          console.log(
            `  - Waiting for ${fileKeys.length - totalAcceptance}/${fileKeys.length} acceptances`
          );
        }
      }

      if (expectBspConfirmations && totalConfirmations < fileKeys.length) {
        try {
          await api.wait.bspStored({
            sealBlock: false,
            timeoutMs: 3000
          });
          bspFound = true;
        } catch (_error) {
          console.log(`[Attempt ${attempt + 1}/${maxAttempts}] BSP stored not in tx pool yet`);
          console.log(
            `  - Waiting for ${fileKeys.length - totalConfirmations}/${fileKeys.length} confirmations`
          );
        }
      }

      const { events } = await api.block.seal({
        signer: localOwner,
        finaliseBlock: finaliseBlock
      });

      // Log what we're looking for
      if (mspFound || bspFound) {
        console.log(
          `[Attempt ${attempt + 1}/${maxAttempts}] Block sealed with transactions - MSP: ${mspFound}, BSP: ${bspFound}`
        );
      }

      // Check for failures of MSP/BSP extrinsics
      const targetMethods: { [key: string]: boolean } = {};
      if (mspFound && expectMspAcceptance) {
        targetMethods.mspAcceptStorageRequest = true;
      }
      if (bspFound && expectBspConfirmations) {
        targetMethods.bspConfirmStoring = true;
      }

      // Track if we saw MspAlreadyConfirmed in this attempt
      let sawAlreadyConfirmedThisAttempt = false;

      if (Object.keys(targetMethods).length > 0) {
        const failures = logFailedTargetExtrinsics(
          api,
          events || [],
          targetMethods,
          `[Attempt ${attempt + 1}/${maxAttempts}]`
        );

        if (failures.length > 0) {
          // Check if any failures are MspAlreadyConfirmed (expected behavior)
          const alreadyConfirmedFailures = failures.filter((f) => f.isAlreadyConfirmed);
          const otherFailures = failures.filter((f) => !f.isAlreadyConfirmed);

          if (alreadyConfirmedFailures.length > 0) {
            sawAlreadyConfirmedThisAttempt = true;
            console.log(
              `  - ${alreadyConfirmedFailures.length} MspAlreadyConfirmed error(s) - expected, MSP will retry with remaining files`
            );
          }

          if (otherFailures.length > 0) {
            console.log(
              `  - Found ${otherFailures.length} other failed extrinsics for MSP/BSP operations`
            );
          }
        }
      }

      if (mspFound && expectMspAcceptance) {
        // Filter for MspAcceptedStorageRequest events without asserting (may be empty if batch failed)
        const acceptEvents = (events || []).filter((e) =>
          api.events.fileSystem.MspAcceptedStorageRequest.is(e.event)
        );

        // Count total MspAcceptedStorageRequest events
        const prevTotal = totalAcceptance;
        totalAcceptance += acceptEvents.length;

        if (acceptEvents.length > 0) {
          console.log(
            `  - Found ${acceptEvents.length} MSP acceptances (${prevTotal} → ${totalAcceptance}/${fileKeys.length})`
          );
        } else if (sawAlreadyConfirmedThisAttempt) {
          // MspAlreadyConfirmed means the batch failed, so no events expected
          console.log(
            "  - No MSP acceptance events (batch failed due to MspAlreadyConfirmed, MSP will retry)"
          );
        } else {
          console.log(
            "  - WARNING: MSP was in pool but no acceptance events found (check for failures above)"
          );
        }
      }

      if (bspFound && expectBspConfirmations) {
        // Check if BSP confirmed storing events are present
        const confirmEvents = await api.assert.eventMany(
          "fileSystem",
          "BspConfirmedStoring",
          events || []
        );

        // Count total file keys confirmed in all BspConfirmedStoring events
        const prevTotal = totalConfirmations;
        for (const eventRecord of confirmEvents) {
          if (api.events.fileSystem.BspConfirmedStoring.is(eventRecord.event)) {
            totalConfirmations += eventRecord.event.data.confirmedFileKeys.length;
          }
        }

        if (confirmEvents.length > 0) {
          console.log(
            `  - Found ${confirmEvents.length} BSP confirm events (${prevTotal} → ${totalConfirmations}/${fileKeys.length})`
          );
        } else {
          console.log(
            "  - WARNING: BSP was in pool but no confirm events found (check for failures above)"
          );
        }
      }
    }

    // Log final summary
    console.log(`\n=== Batch Storage Summary After ${attempt} Attempts ===`);
    if (expectMspAcceptance) {
      console.log(
        `MSP Acceptances: ${totalAcceptance}/${fileKeys.length} ${totalAcceptance === fileKeys.length ? "✓" : "✗"}`
      );
    }
    if (expectBspConfirmations) {
      console.log(
        `BSP Confirmations: ${totalConfirmations}/${fileKeys.length} ${totalConfirmations === fileKeys.length ? "✓" : "✗"}`
      );
    }
    console.log("==========================================\n");

    if (expectMspAcceptance) {
      assert.strictEqual(
        totalAcceptance,
        fileKeys.length,
        `Expected ${fileKeys.length} MSP acceptance, but got ${totalAcceptance}. Check logs above for ExtrinsicFailed events which may indicate why MSP transactions failed.`
      );
    }

    if (expectBspConfirmations) {
      assert.strictEqual(
        totalConfirmations,
        fileKeys.length,
        `Expected ${fileKeys.length} BSP confirmations, but got ${totalConfirmations}. Check logs above for ExtrinsicFailed events which may indicate why BSP transactions failed.`
      );
    }
  }

  // Verify files are in BSP and/or MSP forests or file storages (depending on which APIs are provided)

  await waitFor({
    lambda: async () => {
      for (let index = 0; index < fileKeys.length; index++) {
        const fileKey = fileKeys[index];
        const bucketId = bucketIds[index];

        // Check file IS in BSP forest and file storage (if bspApi is provided)
        if (bspApi) {
          const bspForestResult = await bspApi.rpc.storagehubclient.isFileInForest(null, fileKey);
          if (!bspForestResult.isTrue) {
            return false;
          }

          const bspFileStorageResult =
            await bspApi.rpc.storagehubclient.isFileInFileStorage(fileKey);
          if (!bspFileStorageResult.isFileFound) {
            return false;
          }
        }

        // Check file IS in MSP forest and file storage (if mspApi is provided)
        if (mspApi) {
          const mspForestResult = await mspApi.rpc.storagehubclient.isFileInForest(
            bucketId,
            fileKey
          );
          if (!mspForestResult.isTrue) {
            return false;
          }

          const mspFileStorageResult =
            await mspApi.rpc.storagehubclient.isFileInFileStorage(fileKey);
          if (!mspFileStorageResult.isFileFound) {
            return false;
          }
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
