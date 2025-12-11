import '@polkadot/api-base/types/submittable';
import type { ApiTypes, AugmentedSubmittable, SubmittableExtrinsic, SubmittableExtrinsicFunction } from '@polkadot/api-base/types';
import type { Bytes, Compact, Option, Vec, bool, u128, u32, u64 } from '@polkadot/types-codec';
import type { AnyNumber, IMethod, ITuple } from '@polkadot/types-codec/types';
import type { AccountId32, Call, H256, MultiAddress } from '@polkadot/types/interfaces/runtime';
import type { CumulusPrimitivesCoreAggregateMessageOrigin, CumulusPrimitivesParachainInherentParachainInherentData, PalletBalancesAdjustmentDirection, PalletFileSystemBucketMoveRequestResponse, PalletFileSystemFileDeletionRequest, PalletFileSystemFileKeyWithProof, PalletFileSystemFileOperationIntention, PalletFileSystemReplicationTarget, PalletFileSystemStorageRequestMspBucketResponse, PalletNftsAttributeNamespace, PalletNftsCancelAttributesApprovalWitness, PalletNftsCollectionConfig, PalletNftsDestroyWitness, PalletNftsItemConfig, PalletNftsItemTip, PalletNftsMintSettings, PalletNftsMintWitness, PalletNftsPreSignedAttributes, PalletNftsPreSignedMint, PalletNftsPriceWithDirection, PalletProofsDealerProof, ShParachainRuntimeConfigsRuntimeParamsRuntimeParameters, ShParachainRuntimeSessionKeys, SpRuntimeMultiSignature, SpTrieStorageProofCompactProof, SpWeightsWeightV2Weight, StagingXcmExecutorAssetTransferTransferType, StagingXcmV5Location, XcmV3WeightLimit, XcmVersionedAssetId, XcmVersionedAssets, XcmVersionedLocation, XcmVersionedXcm } from '@polkadot/types/lookup';
export type __AugmentedSubmittable = AugmentedSubmittable<() => unknown>;
export type __SubmittableExtrinsic<ApiType extends ApiTypes> = SubmittableExtrinsic<ApiType>;
export type __SubmittableExtrinsicFunction<ApiType extends ApiTypes> = SubmittableExtrinsicFunction<ApiType>;
declare module '@polkadot/api-base/types/submittable' {
    interface AugmentedSubmittables<ApiType extends ApiTypes> {
        balances: {
            /**
             * Burn the specified liquid free balance from the origin account.
             *
             * If the origin's account ends up below the existential deposit as a result
             * of the burn and `keep_alive` is false, the account will be reaped.
             *
             * Unlike sending funds to a _burn_ address, which merely makes the funds inaccessible,
             * this `burn` operation will reduce total issuance by the amount _burned_.
             **/
            burn: AugmentedSubmittable<(value: Compact<u128> | AnyNumber | Uint8Array, keepAlive: bool | boolean | Uint8Array) => SubmittableExtrinsic<ApiType>, [Compact<u128>, bool]>;
            /**
             * Adjust the total issuance in a saturating way.
             *
             * Can only be called by root and always needs a positive `delta`.
             *
             * # Example
             **/
            forceAdjustTotalIssuance: AugmentedSubmittable<(direction: PalletBalancesAdjustmentDirection | 'Increase' | 'Decrease' | number | Uint8Array, delta: Compact<u128> | AnyNumber | Uint8Array) => SubmittableExtrinsic<ApiType>, [PalletBalancesAdjustmentDirection, Compact<u128>]>;
            /**
             * Set the regular balance of a given account.
             *
             * The dispatch origin for this call is `root`.
             **/
            forceSetBalance: AugmentedSubmittable<(who: MultiAddress | {
                Id: any;
            } | {
                Index: any;
            } | {
                Raw: any;
            } | {
                Address32: any;
            } | {
                Address20: any;
            } | string | Uint8Array, newFree: Compact<u128> | AnyNumber | Uint8Array) => SubmittableExtrinsic<ApiType>, [MultiAddress, Compact<u128>]>;
            /**
             * Exactly as `transfer_allow_death`, except the origin must be root and the source account
             * may be specified.
             **/
            forceTransfer: AugmentedSubmittable<(source: MultiAddress | {
                Id: any;
            } | {
                Index: any;
            } | {
                Raw: any;
            } | {
                Address32: any;
            } | {
                Address20: any;
            } | string | Uint8Array, dest: MultiAddress | {
                Id: any;
            } | {
                Index: any;
            } | {
                Raw: any;
            } | {
                Address32: any;
            } | {
                Address20: any;
            } | string | Uint8Array, value: Compact<u128> | AnyNumber | Uint8Array) => SubmittableExtrinsic<ApiType>, [MultiAddress, MultiAddress, Compact<u128>]>;
            /**
             * Unreserve some balance from a user by force.
             *
             * Can only be called by ROOT.
             **/
            forceUnreserve: AugmentedSubmittable<(who: MultiAddress | {
                Id: any;
            } | {
                Index: any;
            } | {
                Raw: any;
            } | {
                Address32: any;
            } | {
                Address20: any;
            } | string | Uint8Array, amount: u128 | AnyNumber | Uint8Array) => SubmittableExtrinsic<ApiType>, [MultiAddress, u128]>;
            /**
             * Transfer the entire transferable balance from the caller account.
             *
             * NOTE: This function only attempts to transfer _transferable_ balances. This means that
             * any locked, reserved, or existential deposits (when `keep_alive` is `true`), will not be
             * transferred by this function. To ensure that this function results in a killed account,
             * you might need to prepare the account by removing any reference counters, storage
             * deposits, etc...
             *
             * The dispatch origin of this call must be Signed.
             *
             * - `dest`: The recipient of the transfer.
             * - `keep_alive`: A boolean to determine if the `transfer_all` operation should send all
             * of the funds the account has, causing the sender account to be killed (false), or
             * transfer everything except at least the existential deposit, which will guarantee to
             * keep the sender account alive (true).
             **/
            transferAll: AugmentedSubmittable<(dest: MultiAddress | {
                Id: any;
            } | {
                Index: any;
            } | {
                Raw: any;
            } | {
                Address32: any;
            } | {
                Address20: any;
            } | string | Uint8Array, keepAlive: bool | boolean | Uint8Array) => SubmittableExtrinsic<ApiType>, [MultiAddress, bool]>;
            /**
             * Transfer some liquid free balance to another account.
             *
             * `transfer_allow_death` will set the `FreeBalance` of the sender and receiver.
             * If the sender's account is below the existential deposit as a result
             * of the transfer, the account will be reaped.
             *
             * The dispatch origin for this call must be `Signed` by the transactor.
             **/
            transferAllowDeath: AugmentedSubmittable<(dest: MultiAddress | {
                Id: any;
            } | {
                Index: any;
            } | {
                Raw: any;
            } | {
                Address32: any;
            } | {
                Address20: any;
            } | string | Uint8Array, value: Compact<u128> | AnyNumber | Uint8Array) => SubmittableExtrinsic<ApiType>, [MultiAddress, Compact<u128>]>;
            /**
             * Same as the [`transfer_allow_death`] call, but with a check that the transfer will not
             * kill the origin account.
             *
             * 99% of the time you want [`transfer_allow_death`] instead.
             *
             * [`transfer_allow_death`]: struct.Pallet.html#method.transfer
             **/
            transferKeepAlive: AugmentedSubmittable<(dest: MultiAddress | {
                Id: any;
            } | {
                Index: any;
            } | {
                Raw: any;
            } | {
                Address32: any;
            } | {
                Address20: any;
            } | string | Uint8Array, value: Compact<u128> | AnyNumber | Uint8Array) => SubmittableExtrinsic<ApiType>, [MultiAddress, Compact<u128>]>;
            /**
             * Upgrade a specified account.
             *
             * - `origin`: Must be `Signed`.
             * - `who`: The account to be upgraded.
             *
             * This will waive the transaction fee if at least all but 10% of the accounts needed to
             * be upgraded. (We let some not have to be upgraded just in order to allow for the
             * possibility of churn).
             **/
            upgradeAccounts: AugmentedSubmittable<(who: Vec<AccountId32> | (AccountId32 | string | Uint8Array)[]) => SubmittableExtrinsic<ApiType>, [Vec<AccountId32>]>;
            /**
             * Generic tx
             **/
            [key: string]: SubmittableExtrinsicFunction<ApiType>;
        };
        bucketNfts: {
            /**
             * Share access to files within a bucket with another account.
             *
             * The `read_access_regex` parameter is optional and when set to `None` it means that the recipient will be denied access for any read request within the bucket.
             **/
            shareAccess: AugmentedSubmittable<(recipient: MultiAddress | {
                Id: any;
            } | {
                Index: any;
            } | {
                Raw: any;
            } | {
                Address32: any;
            } | {
                Address20: any;
            } | string | Uint8Array, bucket: H256 | string | Uint8Array, itemId: u32 | AnyNumber | Uint8Array, readAccessRegex: Option<Bytes> | null | Uint8Array | Bytes | string) => SubmittableExtrinsic<ApiType>, [MultiAddress, H256, u32, Option<Bytes>]>;
            /**
             * Update read access for an item.
             **/
            updateReadAccess: AugmentedSubmittable<(bucket: H256 | string | Uint8Array, itemId: u32 | AnyNumber | Uint8Array, readAccessRegex: Option<Bytes> | null | Uint8Array | Bytes | string) => SubmittableExtrinsic<ApiType>, [H256, u32, Option<Bytes>]>;
            /**
             * Generic tx
             **/
            [key: string]: SubmittableExtrinsicFunction<ApiType>;
        };
        collatorSelection: {
            /**
             * Add a new account `who` to the list of `Invulnerables` collators. `who` must have
             * registered session keys. If `who` is a candidate, they will be removed.
             *
             * The origin for this call must be the `UpdateOrigin`.
             **/
            addInvulnerable: AugmentedSubmittable<(who: AccountId32 | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [AccountId32]>;
            /**
             * Deregister `origin` as a collator candidate. Note that the collator can only leave on
             * session change. The `CandidacyBond` will be unreserved immediately.
             *
             * This call will fail if the total number of candidates would drop below
             * `MinEligibleCollators`.
             **/
            leaveIntent: AugmentedSubmittable<() => SubmittableExtrinsic<ApiType>, []>;
            /**
             * Register this account as a collator candidate. The account must (a) already have
             * registered session keys and (b) be able to reserve the `CandidacyBond`.
             *
             * This call is not available to `Invulnerable` collators.
             **/
            registerAsCandidate: AugmentedSubmittable<() => SubmittableExtrinsic<ApiType>, []>;
            /**
             * Remove an account `who` from the list of `Invulnerables` collators. `Invulnerables` must
             * be sorted.
             *
             * The origin for this call must be the `UpdateOrigin`.
             **/
            removeInvulnerable: AugmentedSubmittable<(who: AccountId32 | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [AccountId32]>;
            /**
             * Set the candidacy bond amount.
             *
             * If the candidacy bond is increased by this call, all current candidates which have a
             * deposit lower than the new bond will be kicked from the list and get their deposits
             * back.
             *
             * The origin for this call must be the `UpdateOrigin`.
             **/
            setCandidacyBond: AugmentedSubmittable<(bond: u128 | AnyNumber | Uint8Array) => SubmittableExtrinsic<ApiType>, [u128]>;
            /**
             * Set the ideal number of non-invulnerable collators. If lowering this number, then the
             * number of running collators could be higher than this figure. Aside from that edge case,
             * there should be no other way to have more candidates than the desired number.
             *
             * The origin for this call must be the `UpdateOrigin`.
             **/
            setDesiredCandidates: AugmentedSubmittable<(max: u32 | AnyNumber | Uint8Array) => SubmittableExtrinsic<ApiType>, [u32]>;
            /**
             * Set the list of invulnerable (fixed) collators. These collators must do some
             * preparation, namely to have registered session keys.
             *
             * The call will remove any accounts that have not registered keys from the set. That is,
             * it is non-atomic; the caller accepts all `AccountId`s passed in `new` _individually_ as
             * acceptable Invulnerables, and is not proposing a _set_ of new Invulnerables.
             *
             * This call does not maintain mutual exclusivity of `Invulnerables` and `Candidates`. It
             * is recommended to use a batch of `add_invulnerable` and `remove_invulnerable` instead. A
             * `batch_all` can also be used to enforce atomicity. If any candidates are included in
             * `new`, they should be removed with `remove_invulnerable_candidate` after execution.
             *
             * Must be called by the `UpdateOrigin`.
             **/
            setInvulnerables: AugmentedSubmittable<(updated: Vec<AccountId32> | (AccountId32 | string | Uint8Array)[]) => SubmittableExtrinsic<ApiType>, [Vec<AccountId32>]>;
            /**
             * The caller `origin` replaces a candidate `target` in the collator candidate list by
             * reserving `deposit`. The amount `deposit` reserved by the caller must be greater than
             * the existing bond of the target it is trying to replace.
             *
             * This call will fail if the caller is already a collator candidate or invulnerable, the
             * caller does not have registered session keys, the target is not a collator candidate,
             * and/or the `deposit` amount cannot be reserved.
             **/
            takeCandidateSlot: AugmentedSubmittable<(deposit: u128 | AnyNumber | Uint8Array, target: AccountId32 | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [u128, AccountId32]>;
            /**
             * Update the candidacy bond of collator candidate `origin` to a new amount `new_deposit`.
             *
             * Setting a `new_deposit` that is lower than the current deposit while `origin` is
             * occupying a top-`DesiredCandidates` slot is not allowed.
             *
             * This call will fail if `origin` is not a collator candidate, the updated bond is lower
             * than the minimum candidacy bond, and/or the amount cannot be reserved.
             **/
            updateBond: AugmentedSubmittable<(newDeposit: u128 | AnyNumber | Uint8Array) => SubmittableExtrinsic<ApiType>, [u128]>;
            /**
             * Generic tx
             **/
            [key: string]: SubmittableExtrinsicFunction<ApiType>;
        };
        cumulusXcm: {
            /**
             * Generic tx
             **/
            [key: string]: SubmittableExtrinsicFunction<ApiType>;
        };
        fileSystem: {
            /**
             * Executed by a BSP to confirm stopping storage of a file and remove it from their forest.
             *
             * This is the second step of the two-phase stop storing process. The BSP must have previously
             * called [`bsp_request_stop_storing`] to open a pending stop storing request.
             *
             * A minimum waiting period ([`MinWaitForStopStoring`]) must pass between the request and this
             * confirmation. This prevents a BSP from immediately dropping a file when challenged for it,
             * ensuring they can't avoid slashing by quickly calling stop storing upon receiving a challenge.
             *
             * ## What this extrinsic does
             *
             * 1. Verifies the pending stop storing request exists and the minimum wait time has passed
             * 2. Verifies the file is still in the BSP's forest via the inclusion proof
             * 3. **Removes the file from the BSP's forest and updates their root**
             * 4. Decreases the BSP's used capacity
             * 5. Stops challenge/randomness cycles if the BSP has no more files
             *
             * Note: The payment stream was already updated in [`bsp_request_stop_storing`].
             *
             * ## Errors
             *
             * - [`PendingStopStoringRequestNotFound`]: No pending request exists for this BSP and file
             * - [`MinWaitForStopStoringNotReached`]: The minimum waiting period hasn't passed yet
             * - [`OperationNotAllowedWithInsolventUser`]: The file owner is insolvent (the BSP should use
             * [`stop_storing_for_insolvent_user`] instead)
             **/
            bspConfirmStopStoring: AugmentedSubmittable<(fileKey: H256 | string | Uint8Array, inclusionForestProof: SpTrieStorageProofCompactProof | {
                encodedNodes?: any;
            } | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [H256, SpTrieStorageProofCompactProof]>;
            /**
             * Used by a BSP to confirm they are storing data of a storage request.
             **/
            bspConfirmStoring: AugmentedSubmittable<(nonInclusionForestProof: SpTrieStorageProofCompactProof | {
                encodedNodes?: any;
            } | string | Uint8Array, fileKeysAndProofs: Vec<PalletFileSystemFileKeyWithProof> | (PalletFileSystemFileKeyWithProof | {
                fileKey?: any;
                proof?: any;
            } | string | Uint8Array)[]) => SubmittableExtrinsic<ApiType>, [SpTrieStorageProofCompactProof, Vec<PalletFileSystemFileKeyWithProof>]>;
            /**
             * Executed by a BSP to request to stop storing a file.
             *
             * This is the first step of a two-phase process for a BSP to voluntarily stop storing a file.
             * The BSP must later call [`bsp_confirm_stop_storing`] after a minimum waiting period to
             * complete the process and actually remove the file from their forest.
             *
             * **Important**: This extrinsic does NOT modify the BSP's forest root. The file remains in the
             * BSP's forest until [`bsp_confirm_stop_storing`] is called.
             *
             * The BSP is required to provide the file metadata (bucket_id, location, owner, fingerprint, size)
             * to reconstruct and verify the file key. The BSP can get this metadata from its file storage, but
             * it providing it is not a proof that the BSP actually has the file, since this metadata can be obtained
             * from the original storage request or from the assigned MSP if the storage request no longer exists.
             *
             * ## Behavior based on storage request state
             *
             * 1. **Storage request exists and BSP has confirmed storing it**: The BSP is removed from the
             * storage request's confirmed and volunteered lists and the confirmed/volunteered counts are decremented.
             * The BSP is also removed from the storage request as a data server.
             *
             * 2. **Storage request exists but BSP is not a volunteer**: The `bsps_required` count is
             * incremented to compensate for the BSP leaving.
             *
             * 3. **No storage request exists**: A new storage request is created with `bsps_required = 1`
             * so another BSP can pick up the file and maintain its replication target. If `can_serve` is true,
             * the requesting BSP is added as a data server to help the new volunteer download the file.
             *
             * ## Fees
             *
             * The BSP is charged a penalty fee ([`BspStopStoringFilePenalty`]) which is transferred to the treasury.
             *
             * ## Payment Stream
             *
             * The payment stream with the file owner is **updated immediately** in this extrinsic (not in
             * [`bsp_confirm_stop_storing`]). This removes any financial incentive for the BSP to delay or
             * skip the confirmation, as they stop getting paid as soon as they announce their intent to stop storing.
             *
             * ## Restrictions
             *
             * This extrinsic will fail with [`FileHasIncompleteStorageRequest`] if an `IncompleteStorageRequest`
             * exists for the file key. The BSP must wait until fisherman nodes clean up the incomplete request.
             **/
            bspRequestStopStoring: AugmentedSubmittable<(fileKey: H256 | string | Uint8Array, bucketId: H256 | string | Uint8Array, location: Bytes | string | Uint8Array, owner: AccountId32 | string | Uint8Array, fingerprint: H256 | string | Uint8Array, size: u64 | AnyNumber | Uint8Array, canServe: bool | boolean | Uint8Array, inclusionForestProof: SpTrieStorageProofCompactProof | {
                encodedNodes?: any;
            } | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [H256, H256, Bytes, AccountId32, H256, u64, bool, SpTrieStorageProofCompactProof]>;
            /**
             * Used by a BSP to volunteer for storing a file.
             *
             * The transaction will fail if the XOR between the file ID and the BSP ID is not below the threshold,
             * so a BSP is strongly advised to check beforehand. Another reason for failure is
             * if the maximum number of BSPs has been reached. A successful assignment as BSP means
             * that some of the collateral tokens of that MSP are frozen.
             **/
            bspVolunteer: AugmentedSubmittable<(fileKey: H256 | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [H256]>;
            /**
             * Create and associate a collection with a bucket.
             **/
            createAndAssociateCollectionWithBucket: AugmentedSubmittable<(bucketId: H256 | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [H256]>;
            createBucket: AugmentedSubmittable<(mspId: H256 | string | Uint8Array, name: Bytes | string | Uint8Array, private: bool | boolean | Uint8Array, valuePropId: H256 | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [H256, Bytes, bool, H256]>;
            /**
             * Dispatchable extrinsic that allows a User to delete any of their buckets if it is currently empty.
             * This way, the User is allowed to remove now unused buckets to recover their deposit for them.
             *
             * The User must provide the BucketId of the bucket they want to delete, which should correspond to a
             * bucket that is both theirs and currently empty.
             *
             * To check if a bucket is empty, we compare its current root with the one of an empty trie.
             **/
            deleteBucket: AugmentedSubmittable<(bucketId: H256 | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [H256]>;
            /**
             * Deletes files from a provider's forest, changing its root
             *
             * This extrinsic allows any actor to execute file deletion based on signed intentions
             * from the `FileDeletionRequested` event. It requires a valid forest proof showing that
             * all files exist in the specified provider's forest before allowing deletion.
             *
             * Multiple files can be deleted in a single call using one forest proof bounded by [`MaxFileDeletionsPerExtrinsic`](Config::MaxFileDeletionsPerExtrinsic).
             *
             * If `bsp_id` is `None`, files will be deleted from the bucket forest.
             * If `bsp_id` is `Some(id)`, files will be deleted from the specified BSP's forest.
             **/
            deleteFiles: AugmentedSubmittable<(fileDeletions: Vec<PalletFileSystemFileDeletionRequest> | (PalletFileSystemFileDeletionRequest | {
                fileOwner?: any;
                signedIntention?: any;
                signature?: any;
                bucketId?: any;
                location?: any;
                size_?: any;
                fingerprint?: any;
            } | string | Uint8Array)[], bspId: Option<H256> | null | Uint8Array | H256 | string, forestProof: SpTrieStorageProofCompactProof | {
                encodedNodes?: any;
            } | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [Vec<PalletFileSystemFileDeletionRequest>, Option<H256>, SpTrieStorageProofCompactProof]>;
            /**
             * Delete files from an incomplete (rejected, expired or revoked) storage request.
             *
             * This extrinsic allows fisherman nodes to delete files from providers when IncompleteStorageRequestMetadata
             * for the given file keys exist in the IncompleteStorageRequests mapping. It validates that the metadata exists
             * for each file, that the provider has the files in its Merkle Patricia Forest, and verifies the file keys match
             * the metadata.
             *
             * Multiple files can be deleted in a single call using one forest proof bounded by [`MaxFileDeletionsPerExtrinsic`](Config::MaxFileDeletionsPerExtrinsic).
             **/
            deleteFilesForIncompleteStorageRequest: AugmentedSubmittable<(fileKeys: Vec<H256> | (H256 | string | Uint8Array)[], bspId: Option<H256> | null | Uint8Array | H256 | string, forestProof: SpTrieStorageProofCompactProof | {
                encodedNodes?: any;
            } | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [Vec<H256>, Option<H256>, SpTrieStorageProofCompactProof]>;
            /**
             * Issue a new storage request for a file
             **/
            issueStorageRequest: AugmentedSubmittable<(bucketId: H256 | string | Uint8Array, location: Bytes | string | Uint8Array, fingerprint: H256 | string | Uint8Array, size: u64 | AnyNumber | Uint8Array, mspId: H256 | string | Uint8Array, peerIds: Vec<Bytes> | (Bytes | string | Uint8Array)[], replicationTarget: PalletFileSystemReplicationTarget | {
                Basic: any;
            } | {
                Standard: any;
            } | {
                HighSecurity: any;
            } | {
                SuperHighSecurity: any;
            } | {
                UltraHighSecurity: any;
            } | {
                Custom: any;
            } | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [H256, Bytes, H256, u64, H256, Vec<Bytes>, PalletFileSystemReplicationTarget]>;
            mspRespondMoveBucketRequest: AugmentedSubmittable<(bucketId: H256 | string | Uint8Array, response: PalletFileSystemBucketMoveRequestResponse | 'Accepted' | 'Rejected' | number | Uint8Array) => SubmittableExtrinsic<ApiType>, [H256, PalletFileSystemBucketMoveRequestResponse]>;
            /**
             * Used by a MSP to accept or decline storage requests in batches, grouped by bucket.
             *
             * This follows a best-effort strategy, meaning that all file keys will be processed and declared to have successfully be
             * accepted, rejected or have failed to be processed in the results of the event emitted.
             *
             * The MSP has to provide a file proof for all the file keys that are being accepted and a non-inclusion proof for the file keys
             * in the bucket's Merkle Patricia Forest. The file proofs for the file keys is necessary to verify that
             * the MSP actually has the files, while the non-inclusion proof is necessary to verify that the MSP
             * wasn't storing it before.
             **/
            mspRespondStorageRequestsMultipleBuckets: AugmentedSubmittable<(storageRequestMspResponse: Vec<PalletFileSystemStorageRequestMspBucketResponse> | (PalletFileSystemStorageRequestMspBucketResponse | {
                bucketId?: any;
                accept?: any;
                reject?: any;
            } | string | Uint8Array)[]) => SubmittableExtrinsic<ApiType>, [Vec<PalletFileSystemStorageRequestMspBucketResponse>]>;
            mspStopStoringBucket: AugmentedSubmittable<(bucketId: H256 | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [H256]>;
            /**
             * Executed by a MSP to stop storing a bucket from an insolvent user.
             *
             * This is used when a user has become insolvent and the MSP needs to stop storing the buckets of that user, since
             * it won't be getting paid for them anymore.
             * It validates that:
             * - The sender is the MSP that's currently storing the bucket, and the bucket exists.
             * - That the user is currently insolvent OR
             * - That the payment stream between the MSP and user doesn't exist (which would occur as a consequence of the MSP previously
             * having deleted another bucket it was storing for this user through this extrinsic).
             * And then completely removes the bucket from the system.
             *
             * If there was a storage request pending for the bucket, it will eventually expire without being fulfilled (because the MSP can't
             * accept storage requests for insolvent users and BSPs can't volunteer nor confirm them either) and afterwards any BSPs that
             * had confirmed the file can just call `sp_stop_storing_for_insolvent_user` to get rid of it.
             **/
            mspStopStoringBucketForInsolventUser: AugmentedSubmittable<(bucketId: H256 | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [H256]>;
            /**
             * Request deletion of a file using a signed delete intention.
             *
             * The origin must be signed and the signature must be valid for the given delete intention.
             * The delete intention must contain the file key and the delete operation.
             * File metadata is provided separately for ownership verification.
             **/
            requestDeleteFile: AugmentedSubmittable<(signedIntention: PalletFileSystemFileOperationIntention | {
                fileKey?: any;
                operation?: any;
            } | string | Uint8Array, signature: SpRuntimeMultiSignature | {
                Ed25519: any;
            } | {
                Sr25519: any;
            } | {
                Ecdsa: any;
            } | string | Uint8Array, bucketId: H256 | string | Uint8Array, location: Bytes | string | Uint8Array, size: u64 | AnyNumber | Uint8Array, fingerprint: H256 | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [PalletFileSystemFileOperationIntention, SpRuntimeMultiSignature, H256, Bytes, u64, H256]>;
            requestMoveBucket: AugmentedSubmittable<(bucketId: H256 | string | Uint8Array, newMspId: H256 | string | Uint8Array, newValuePropId: H256 | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [H256, H256, H256]>;
            /**
             * Revoke storage request
             **/
            revokeStorageRequest: AugmentedSubmittable<(fileKey: H256 | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [H256]>;
            /**
             * Executed by a SP to stop storing a file from an insolvent user.
             *
             * This is used when a user has become insolvent and the SP needs to stop storing the files of that user, since
             * it won't be getting paid for it anymore.
             * The validations are similar to the ones in the `bsp_request_stop_storing` and `bsp_confirm_stop_storing` extrinsics, but the SP doesn't need to
             * wait for a minimum amount of blocks to confirm to stop storing the file nor it has to be a BSP.
             **/
            stopStoringForInsolventUser: AugmentedSubmittable<(fileKey: H256 | string | Uint8Array, bucketId: H256 | string | Uint8Array, location: Bytes | string | Uint8Array, owner: AccountId32 | string | Uint8Array, fingerprint: H256 | string | Uint8Array, size: u64 | AnyNumber | Uint8Array, inclusionForestProof: SpTrieStorageProofCompactProof | {
                encodedNodes?: any;
            } | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [H256, H256, Bytes, AccountId32, H256, u64, SpTrieStorageProofCompactProof]>;
            updateBucketPrivacy: AugmentedSubmittable<(bucketId: H256 | string | Uint8Array, private: bool | boolean | Uint8Array) => SubmittableExtrinsic<ApiType>, [H256, bool]>;
            /**
             * Generic tx
             **/
            [key: string]: SubmittableExtrinsicFunction<ApiType>;
        };
        messageQueue: {
            /**
             * Execute an overweight message.
             *
             * Temporary processing errors will be propagated whereas permanent errors are treated
             * as success condition.
             *
             * - `origin`: Must be `Signed`.
             * - `message_origin`: The origin from which the message to be executed arrived.
             * - `page`: The page in the queue in which the message to be executed is sitting.
             * - `index`: The index into the queue of the message to be executed.
             * - `weight_limit`: The maximum amount of weight allowed to be consumed in the execution
             * of the message.
             *
             * Benchmark complexity considerations: O(index + weight_limit).
             **/
            executeOverweight: AugmentedSubmittable<(messageOrigin: CumulusPrimitivesCoreAggregateMessageOrigin | {
                Here: any;
            } | {
                Parent: any;
            } | {
                Sibling: any;
            } | string | Uint8Array, page: u32 | AnyNumber | Uint8Array, index: u32 | AnyNumber | Uint8Array, weightLimit: SpWeightsWeightV2Weight | {
                refTime?: any;
                proofSize?: any;
            } | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [CumulusPrimitivesCoreAggregateMessageOrigin, u32, u32, SpWeightsWeightV2Weight]>;
            /**
             * Remove a page which has no more messages remaining to be processed or is stale.
             **/
            reapPage: AugmentedSubmittable<(messageOrigin: CumulusPrimitivesCoreAggregateMessageOrigin | {
                Here: any;
            } | {
                Parent: any;
            } | {
                Sibling: any;
            } | string | Uint8Array, pageIndex: u32 | AnyNumber | Uint8Array) => SubmittableExtrinsic<ApiType>, [CumulusPrimitivesCoreAggregateMessageOrigin, u32]>;
            /**
             * Generic tx
             **/
            [key: string]: SubmittableExtrinsicFunction<ApiType>;
        };
        nfts: {
            /**
             * Approve item's attributes to be changed by a delegated third-party account.
             *
             * Origin must be Signed and must be an owner of the `item`.
             *
             * - `collection`: A collection of the item.
             * - `item`: The item that holds attributes.
             * - `delegate`: The account to delegate permission to change attributes of the item.
             *
             * Emits `ItemAttributesApprovalAdded` on success.
             **/
            approveItemAttributes: AugmentedSubmittable<(collection: u32 | AnyNumber | Uint8Array, item: u32 | AnyNumber | Uint8Array, delegate: MultiAddress | {
                Id: any;
            } | {
                Index: any;
            } | {
                Raw: any;
            } | {
                Address32: any;
            } | {
                Address20: any;
            } | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [u32, u32, MultiAddress]>;
            /**
             * Approve an item to be transferred by a delegated third-party account.
             *
             * Origin must be either `ForceOrigin` or Signed and the sender should be the Owner of the
             * `item`.
             *
             * - `collection`: The collection of the item to be approved for delegated transfer.
             * - `item`: The item to be approved for delegated transfer.
             * - `delegate`: The account to delegate permission to transfer the item.
             * - `maybe_deadline`: Optional deadline for the approval. Specified by providing the
             * number of blocks after which the approval will expire
             *
             * Emits `TransferApproved` on success.
             *
             * Weight: `O(1)`
             **/
            approveTransfer: AugmentedSubmittable<(collection: u32 | AnyNumber | Uint8Array, item: u32 | AnyNumber | Uint8Array, delegate: MultiAddress | {
                Id: any;
            } | {
                Index: any;
            } | {
                Raw: any;
            } | {
                Address32: any;
            } | {
                Address20: any;
            } | string | Uint8Array, maybeDeadline: Option<u32> | null | Uint8Array | u32 | AnyNumber) => SubmittableExtrinsic<ApiType>, [u32, u32, MultiAddress, Option<u32>]>;
            /**
             * Destroy a single item.
             *
             * The origin must conform to `ForceOrigin` or must be Signed and the signing account must
             * be the owner of the `item`.
             *
             * - `collection`: The collection of the item to be burned.
             * - `item`: The item to be burned.
             *
             * Emits `Burned`.
             *
             * Weight: `O(1)`
             **/
            burn: AugmentedSubmittable<(collection: u32 | AnyNumber | Uint8Array, item: u32 | AnyNumber | Uint8Array) => SubmittableExtrinsic<ApiType>, [u32, u32]>;
            /**
             * Allows to buy an item if it's up for sale.
             *
             * Origin must be Signed and must not be the owner of the `item`.
             *
             * - `collection`: The collection of the item.
             * - `item`: The item the sender wants to buy.
             * - `bid_price`: The price the sender is willing to pay.
             *
             * Emits `ItemBought` on success.
             **/
            buyItem: AugmentedSubmittable<(collection: u32 | AnyNumber | Uint8Array, item: u32 | AnyNumber | Uint8Array, bidPrice: u128 | AnyNumber | Uint8Array) => SubmittableExtrinsic<ApiType>, [u32, u32, u128]>;
            /**
             * Cancel one of the transfer approvals for a specific item.
             *
             * Origin must be either:
             * - the `Force` origin;
             * - `Signed` with the signer being the Owner of the `item`;
             *
             * Arguments:
             * - `collection`: The collection of the item of whose approval will be cancelled.
             * - `item`: The item of the collection of whose approval will be cancelled.
             * - `delegate`: The account that is going to loose their approval.
             *
             * Emits `ApprovalCancelled` on success.
             *
             * Weight: `O(1)`
             **/
            cancelApproval: AugmentedSubmittable<(collection: u32 | AnyNumber | Uint8Array, item: u32 | AnyNumber | Uint8Array, delegate: MultiAddress | {
                Id: any;
            } | {
                Index: any;
            } | {
                Raw: any;
            } | {
                Address32: any;
            } | {
                Address20: any;
            } | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [u32, u32, MultiAddress]>;
            /**
             * Cancel the previously provided approval to change item's attributes.
             * All the previously set attributes by the `delegate` will be removed.
             *
             * Origin must be Signed and must be an owner of the `item`.
             *
             * - `collection`: Collection that the item is contained within.
             * - `item`: The item that holds attributes.
             * - `delegate`: The previously approved account to remove.
             *
             * Emits `ItemAttributesApprovalRemoved` on success.
             **/
            cancelItemAttributesApproval: AugmentedSubmittable<(collection: u32 | AnyNumber | Uint8Array, item: u32 | AnyNumber | Uint8Array, delegate: MultiAddress | {
                Id: any;
            } | {
                Index: any;
            } | {
                Raw: any;
            } | {
                Address32: any;
            } | {
                Address20: any;
            } | string | Uint8Array, witness: PalletNftsCancelAttributesApprovalWitness | {
                accountAttributes?: any;
            } | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [u32, u32, MultiAddress, PalletNftsCancelAttributesApprovalWitness]>;
            /**
             * Cancel an atomic swap.
             *
             * Origin must be Signed.
             * Origin must be an owner of the `item` if the deadline hasn't expired.
             *
             * - `collection`: The collection of the item.
             * - `item`: The item an owner wants to give.
             *
             * Emits `SwapCancelled` on success.
             **/
            cancelSwap: AugmentedSubmittable<(offeredCollection: u32 | AnyNumber | Uint8Array, offeredItem: u32 | AnyNumber | Uint8Array) => SubmittableExtrinsic<ApiType>, [u32, u32]>;
            /**
             * Claim an atomic swap.
             * This method executes a pending swap, that was created by a counterpart before.
             *
             * Origin must be Signed and must be an owner of the `item`.
             *
             * - `send_collection`: The collection of the item to be sent.
             * - `send_item`: The item to be sent.
             * - `receive_collection`: The collection of the item to be received.
             * - `receive_item`: The item to be received.
             * - `witness_price`: A price that was previously agreed on.
             *
             * Emits `SwapClaimed` on success.
             **/
            claimSwap: AugmentedSubmittable<(sendCollection: u32 | AnyNumber | Uint8Array, sendItem: u32 | AnyNumber | Uint8Array, receiveCollection: u32 | AnyNumber | Uint8Array, receiveItem: u32 | AnyNumber | Uint8Array, witnessPrice: Option<PalletNftsPriceWithDirection> | null | Uint8Array | PalletNftsPriceWithDirection | {
                amount?: any;
                direction?: any;
            } | string) => SubmittableExtrinsic<ApiType>, [u32, u32, u32, u32, Option<PalletNftsPriceWithDirection>]>;
            /**
             * Cancel all the approvals of a specific item.
             *
             * Origin must be either:
             * - the `Force` origin;
             * - `Signed` with the signer being the Owner of the `item`;
             *
             * Arguments:
             * - `collection`: The collection of the item of whose approvals will be cleared.
             * - `item`: The item of the collection of whose approvals will be cleared.
             *
             * Emits `AllApprovalsCancelled` on success.
             *
             * Weight: `O(1)`
             **/
            clearAllTransferApprovals: AugmentedSubmittable<(collection: u32 | AnyNumber | Uint8Array, item: u32 | AnyNumber | Uint8Array) => SubmittableExtrinsic<ApiType>, [u32, u32]>;
            /**
             * Clear an attribute for a collection or item.
             *
             * Origin must be either `ForceOrigin` or Signed and the sender should be the Owner of the
             * attribute.
             *
             * Any deposit is freed for the collection's owner.
             *
             * - `collection`: The identifier of the collection whose item's metadata to clear.
             * - `maybe_item`: The identifier of the item whose metadata to clear.
             * - `namespace`: Attribute's namespace.
             * - `key`: The key of the attribute.
             *
             * Emits `AttributeCleared`.
             *
             * Weight: `O(1)`
             **/
            clearAttribute: AugmentedSubmittable<(collection: u32 | AnyNumber | Uint8Array, maybeItem: Option<u32> | null | Uint8Array | u32 | AnyNumber, namespace: PalletNftsAttributeNamespace | {
                Pallet: any;
            } | {
                CollectionOwner: any;
            } | {
                ItemOwner: any;
            } | {
                Account: any;
            } | string | Uint8Array, key: Bytes | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [u32, Option<u32>, PalletNftsAttributeNamespace, Bytes]>;
            /**
             * Clear the metadata for a collection.
             *
             * Origin must be either `ForceOrigin` or `Signed` and the sender should be the Admin of
             * the `collection`.
             *
             * Any deposit is freed for the collection's owner.
             *
             * - `collection`: The identifier of the collection whose metadata to clear.
             *
             * Emits `CollectionMetadataCleared`.
             *
             * Weight: `O(1)`
             **/
            clearCollectionMetadata: AugmentedSubmittable<(collection: u32 | AnyNumber | Uint8Array) => SubmittableExtrinsic<ApiType>, [u32]>;
            /**
             * Clear the metadata for an item.
             *
             * Origin must be either `ForceOrigin` or Signed and the sender should be the Admin of the
             * `collection`.
             *
             * Any deposit is freed for the collection's owner.
             *
             * - `collection`: The identifier of the collection whose item's metadata to clear.
             * - `item`: The identifier of the item whose metadata to clear.
             *
             * Emits `ItemMetadataCleared`.
             *
             * Weight: `O(1)`
             **/
            clearMetadata: AugmentedSubmittable<(collection: u32 | AnyNumber | Uint8Array, item: u32 | AnyNumber | Uint8Array) => SubmittableExtrinsic<ApiType>, [u32, u32]>;
            /**
             * Issue a new collection of non-fungible items from a public origin.
             *
             * This new collection has no items initially and its owner is the origin.
             *
             * The origin must be Signed and the sender must have sufficient funds free.
             *
             * `CollectionDeposit` funds of sender are reserved.
             *
             * Parameters:
             * - `admin`: The admin of this collection. The admin is the initial address of each
             * member of the collection's admin team.
             *
             * Emits `Created` event when successful.
             *
             * Weight: `O(1)`
             **/
            create: AugmentedSubmittable<(admin: MultiAddress | {
                Id: any;
            } | {
                Index: any;
            } | {
                Raw: any;
            } | {
                Address32: any;
            } | {
                Address20: any;
            } | string | Uint8Array, config: PalletNftsCollectionConfig | {
                settings?: any;
                maxSupply?: any;
                mintSettings?: any;
            } | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [MultiAddress, PalletNftsCollectionConfig]>;
            /**
             * Register a new atomic swap, declaring an intention to send an `item` in exchange for
             * `desired_item` from origin to target on the current blockchain.
             * The target can execute the swap during the specified `duration` of blocks (if set).
             * Additionally, the price could be set for the desired `item`.
             *
             * Origin must be Signed and must be an owner of the `item`.
             *
             * - `collection`: The collection of the item.
             * - `item`: The item an owner wants to give.
             * - `desired_collection`: The collection of the desired item.
             * - `desired_item`: The desired item an owner wants to receive.
             * - `maybe_price`: The price an owner is willing to pay or receive for the desired `item`.
             * - `duration`: A deadline for the swap. Specified by providing the number of blocks
             * after which the swap will expire.
             *
             * Emits `SwapCreated` on success.
             **/
            createSwap: AugmentedSubmittable<(offeredCollection: u32 | AnyNumber | Uint8Array, offeredItem: u32 | AnyNumber | Uint8Array, desiredCollection: u32 | AnyNumber | Uint8Array, maybeDesiredItem: Option<u32> | null | Uint8Array | u32 | AnyNumber, maybePrice: Option<PalletNftsPriceWithDirection> | null | Uint8Array | PalletNftsPriceWithDirection | {
                amount?: any;
                direction?: any;
            } | string, duration: u32 | AnyNumber | Uint8Array) => SubmittableExtrinsic<ApiType>, [u32, u32, u32, Option<u32>, Option<PalletNftsPriceWithDirection>, u32]>;
            /**
             * Destroy a collection of fungible items.
             *
             * The origin must conform to `ForceOrigin` or must be `Signed` and the sender must be the
             * owner of the `collection`.
             *
             * NOTE: The collection must have 0 items to be destroyed.
             *
             * - `collection`: The identifier of the collection to be destroyed.
             * - `witness`: Information on the items minted in the collection. This must be
             * correct.
             *
             * Emits `Destroyed` event when successful.
             *
             * Weight: `O(m + c + a)` where:
             * - `m = witness.item_metadatas`
             * - `c = witness.item_configs`
             * - `a = witness.attributes`
             **/
            destroy: AugmentedSubmittable<(collection: u32 | AnyNumber | Uint8Array, witness: PalletNftsDestroyWitness | {
                itemMetadatas?: any;
                itemConfigs?: any;
                attributes?: any;
            } | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [u32, PalletNftsDestroyWitness]>;
            /**
             * Change the config of a collection.
             *
             * Origin must be `ForceOrigin`.
             *
             * - `collection`: The identifier of the collection.
             * - `config`: The new config of this collection.
             *
             * Emits `CollectionConfigChanged`.
             *
             * Weight: `O(1)`
             **/
            forceCollectionConfig: AugmentedSubmittable<(collection: u32 | AnyNumber | Uint8Array, config: PalletNftsCollectionConfig | {
                settings?: any;
                maxSupply?: any;
                mintSettings?: any;
            } | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [u32, PalletNftsCollectionConfig]>;
            /**
             * Change the Owner of a collection.
             *
             * Origin must be `ForceOrigin`.
             *
             * - `collection`: The identifier of the collection.
             * - `owner`: The new Owner of this collection.
             *
             * Emits `OwnerChanged`.
             *
             * Weight: `O(1)`
             **/
            forceCollectionOwner: AugmentedSubmittable<(collection: u32 | AnyNumber | Uint8Array, owner: MultiAddress | {
                Id: any;
            } | {
                Index: any;
            } | {
                Raw: any;
            } | {
                Address32: any;
            } | {
                Address20: any;
            } | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [u32, MultiAddress]>;
            /**
             * Issue a new collection of non-fungible items from a privileged origin.
             *
             * This new collection has no items initially.
             *
             * The origin must conform to `ForceOrigin`.
             *
             * Unlike `create`, no funds are reserved.
             *
             * - `owner`: The owner of this collection of items. The owner has full superuser
             * permissions over this item, but may later change and configure the permissions using
             * `transfer_ownership` and `set_team`.
             *
             * Emits `ForceCreated` event when successful.
             *
             * Weight: `O(1)`
             **/
            forceCreate: AugmentedSubmittable<(owner: MultiAddress | {
                Id: any;
            } | {
                Index: any;
            } | {
                Raw: any;
            } | {
                Address32: any;
            } | {
                Address20: any;
            } | string | Uint8Array, config: PalletNftsCollectionConfig | {
                settings?: any;
                maxSupply?: any;
                mintSettings?: any;
            } | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [MultiAddress, PalletNftsCollectionConfig]>;
            /**
             * Mint an item of a particular collection from a privileged origin.
             *
             * The origin must conform to `ForceOrigin` or must be `Signed` and the sender must be the
             * Issuer of the `collection`.
             *
             * - `collection`: The collection of the item to be minted.
             * - `item`: An identifier of the new item.
             * - `mint_to`: Account into which the item will be minted.
             * - `item_config`: A config of the new item.
             *
             * Emits `Issued` event when successful.
             *
             * Weight: `O(1)`
             **/
            forceMint: AugmentedSubmittable<(collection: u32 | AnyNumber | Uint8Array, item: u32 | AnyNumber | Uint8Array, mintTo: MultiAddress | {
                Id: any;
            } | {
                Index: any;
            } | {
                Raw: any;
            } | {
                Address32: any;
            } | {
                Address20: any;
            } | string | Uint8Array, itemConfig: PalletNftsItemConfig | {
                settings?: any;
            } | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [u32, u32, MultiAddress, PalletNftsItemConfig]>;
            /**
             * Force-set an attribute for a collection or item.
             *
             * Origin must be `ForceOrigin`.
             *
             * If the attribute already exists and it was set by another account, the deposit
             * will be returned to the previous owner.
             *
             * - `set_as`: An optional owner of the attribute.
             * - `collection`: The identifier of the collection whose item's metadata to set.
             * - `maybe_item`: The identifier of the item whose metadata to set.
             * - `namespace`: Attribute's namespace.
             * - `key`: The key of the attribute.
             * - `value`: The value to which to set the attribute.
             *
             * Emits `AttributeSet`.
             *
             * Weight: `O(1)`
             **/
            forceSetAttribute: AugmentedSubmittable<(setAs: Option<AccountId32> | null | Uint8Array | AccountId32 | string, collection: u32 | AnyNumber | Uint8Array, maybeItem: Option<u32> | null | Uint8Array | u32 | AnyNumber, namespace: PalletNftsAttributeNamespace | {
                Pallet: any;
            } | {
                CollectionOwner: any;
            } | {
                ItemOwner: any;
            } | {
                Account: any;
            } | string | Uint8Array, key: Bytes | string | Uint8Array, value: Bytes | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [Option<AccountId32>, u32, Option<u32>, PalletNftsAttributeNamespace, Bytes, Bytes]>;
            /**
             * Disallows specified settings for the whole collection.
             *
             * Origin must be Signed and the sender should be the Owner of the `collection`.
             *
             * - `collection`: The collection to be locked.
             * - `lock_settings`: The settings to be locked.
             *
             * Note: it's possible to only lock(set) the setting, but not to unset it.
             *
             * Emits `CollectionLocked`.
             *
             * Weight: `O(1)`
             **/
            lockCollection: AugmentedSubmittable<(collection: u32 | AnyNumber | Uint8Array, lockSettings: u64 | AnyNumber | Uint8Array) => SubmittableExtrinsic<ApiType>, [u32, u64]>;
            /**
             * Disallows changing the metadata or attributes of the item.
             *
             * Origin must be either `ForceOrigin` or Signed and the sender should be the Admin
             * of the `collection`.
             *
             * - `collection`: The collection if the `item`.
             * - `item`: An item to be locked.
             * - `lock_metadata`: Specifies whether the metadata should be locked.
             * - `lock_attributes`: Specifies whether the attributes in the `CollectionOwner` namespace
             * should be locked.
             *
             * Note: `lock_attributes` affects the attributes in the `CollectionOwner` namespace only.
             * When the metadata or attributes are locked, it won't be possible the unlock them.
             *
             * Emits `ItemPropertiesLocked`.
             *
             * Weight: `O(1)`
             **/
            lockItemProperties: AugmentedSubmittable<(collection: u32 | AnyNumber | Uint8Array, item: u32 | AnyNumber | Uint8Array, lockMetadata: bool | boolean | Uint8Array, lockAttributes: bool | boolean | Uint8Array) => SubmittableExtrinsic<ApiType>, [u32, u32, bool, bool]>;
            /**
             * Disallow further unprivileged transfer of an item.
             *
             * Origin must be Signed and the sender should be the Freezer of the `collection`.
             *
             * - `collection`: The collection of the item to be changed.
             * - `item`: The item to become non-transferable.
             *
             * Emits `ItemTransferLocked`.
             *
             * Weight: `O(1)`
             **/
            lockItemTransfer: AugmentedSubmittable<(collection: u32 | AnyNumber | Uint8Array, item: u32 | AnyNumber | Uint8Array) => SubmittableExtrinsic<ApiType>, [u32, u32]>;
            /**
             * Mint an item of a particular collection.
             *
             * The origin must be Signed and the sender must comply with the `mint_settings` rules.
             *
             * - `collection`: The collection of the item to be minted.
             * - `item`: An identifier of the new item.
             * - `mint_to`: Account into which the item will be minted.
             * - `witness_data`: When the mint type is `HolderOf(collection_id)`, then the owned
             * item_id from that collection needs to be provided within the witness data object. If
             * the mint price is set, then it should be additionally confirmed in the `witness_data`.
             *
             * Note: the deposit will be taken from the `origin` and not the `owner` of the `item`.
             *
             * Emits `Issued` event when successful.
             *
             * Weight: `O(1)`
             **/
            mint: AugmentedSubmittable<(collection: u32 | AnyNumber | Uint8Array, item: u32 | AnyNumber | Uint8Array, mintTo: MultiAddress | {
                Id: any;
            } | {
                Index: any;
            } | {
                Raw: any;
            } | {
                Address32: any;
            } | {
                Address20: any;
            } | string | Uint8Array, witnessData: Option<PalletNftsMintWitness> | null | Uint8Array | PalletNftsMintWitness | {
                ownedItem?: any;
                mintPrice?: any;
            } | string) => SubmittableExtrinsic<ApiType>, [u32, u32, MultiAddress, Option<PalletNftsMintWitness>]>;
            /**
             * Mint an item by providing the pre-signed approval.
             *
             * Origin must be Signed.
             *
             * - `mint_data`: The pre-signed approval that consists of the information about the item,
             * its metadata, attributes, who can mint it (`None` for anyone) and until what block
             * number.
             * - `signature`: The signature of the `data` object.
             * - `signer`: The `data` object's signer. Should be an Issuer of the collection.
             *
             * Emits `Issued` on success.
             * Emits `AttributeSet` if the attributes were provided.
             * Emits `ItemMetadataSet` if the metadata was not empty.
             **/
            mintPreSigned: AugmentedSubmittable<(mintData: PalletNftsPreSignedMint | {
                collection?: any;
                item?: any;
                attributes?: any;
                metadata?: any;
                onlyAccount?: any;
                deadline?: any;
                mintPrice?: any;
            } | string | Uint8Array, signature: SpRuntimeMultiSignature | {
                Ed25519: any;
            } | {
                Sr25519: any;
            } | {
                Ecdsa: any;
            } | string | Uint8Array, signer: AccountId32 | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [PalletNftsPreSignedMint, SpRuntimeMultiSignature, AccountId32]>;
            /**
             * Allows to pay the tips.
             *
             * Origin must be Signed.
             *
             * - `tips`: Tips array.
             *
             * Emits `TipSent` on every tip transfer.
             **/
            payTips: AugmentedSubmittable<(tips: Vec<PalletNftsItemTip> | (PalletNftsItemTip | {
                collection?: any;
                item?: any;
                receiver?: any;
                amount?: any;
            } | string | Uint8Array)[]) => SubmittableExtrinsic<ApiType>, [Vec<PalletNftsItemTip>]>;
            /**
             * Re-evaluate the deposits on some items.
             *
             * Origin must be Signed and the sender should be the Owner of the `collection`.
             *
             * - `collection`: The collection of the items to be reevaluated.
             * - `items`: The items of the collection whose deposits will be reevaluated.
             *
             * NOTE: This exists as a best-effort function. Any items which are unknown or
             * in the case that the owner account does not have reservable funds to pay for a
             * deposit increase are ignored. Generally the owner isn't going to call this on items
             * whose existing deposit is less than the refreshed deposit as it would only cost them,
             * so it's of little consequence.
             *
             * It will still return an error in the case that the collection is unknown or the signer
             * is not permitted to call it.
             *
             * Weight: `O(items.len())`
             **/
            redeposit: AugmentedSubmittable<(collection: u32 | AnyNumber | Uint8Array, items: Vec<u32> | (u32 | AnyNumber | Uint8Array)[]) => SubmittableExtrinsic<ApiType>, [u32, Vec<u32>]>;
            /**
             * Set (or reset) the acceptance of ownership for a particular account.
             *
             * Origin must be `Signed` and if `maybe_collection` is `Some`, then the signer must have a
             * provider reference.
             *
             * - `maybe_collection`: The identifier of the collection whose ownership the signer is
             * willing to accept, or if `None`, an indication that the signer is willing to accept no
             * ownership transferal.
             *
             * Emits `OwnershipAcceptanceChanged`.
             **/
            setAcceptOwnership: AugmentedSubmittable<(maybeCollection: Option<u32> | null | Uint8Array | u32 | AnyNumber) => SubmittableExtrinsic<ApiType>, [Option<u32>]>;
            /**
             * Set an attribute for a collection or item.
             *
             * Origin must be Signed and must conform to the namespace ruleset:
             * - `CollectionOwner` namespace could be modified by the `collection` Admin only;
             * - `ItemOwner` namespace could be modified by the `maybe_item` owner only. `maybe_item`
             * should be set in that case;
             * - `Account(AccountId)` namespace could be modified only when the `origin` was given a
             * permission to do so;
             *
             * The funds of `origin` are reserved according to the formula:
             * `AttributeDepositBase + DepositPerByte * (key.len + value.len)` taking into
             * account any already reserved funds.
             *
             * - `collection`: The identifier of the collection whose item's metadata to set.
             * - `maybe_item`: The identifier of the item whose metadata to set.
             * - `namespace`: Attribute's namespace.
             * - `key`: The key of the attribute.
             * - `value`: The value to which to set the attribute.
             *
             * Emits `AttributeSet`.
             *
             * Weight: `O(1)`
             **/
            setAttribute: AugmentedSubmittable<(collection: u32 | AnyNumber | Uint8Array, maybeItem: Option<u32> | null | Uint8Array | u32 | AnyNumber, namespace: PalletNftsAttributeNamespace | {
                Pallet: any;
            } | {
                CollectionOwner: any;
            } | {
                ItemOwner: any;
            } | {
                Account: any;
            } | string | Uint8Array, key: Bytes | string | Uint8Array, value: Bytes | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [u32, Option<u32>, PalletNftsAttributeNamespace, Bytes, Bytes]>;
            /**
             * Set attributes for an item by providing the pre-signed approval.
             *
             * Origin must be Signed and must be an owner of the `data.item`.
             *
             * - `data`: The pre-signed approval that consists of the information about the item,
             * attributes to update and until what block number.
             * - `signature`: The signature of the `data` object.
             * - `signer`: The `data` object's signer. Should be an Admin of the collection for the
             * `CollectionOwner` namespace.
             *
             * Emits `AttributeSet` for each provided attribute.
             * Emits `ItemAttributesApprovalAdded` if the approval wasn't set before.
             * Emits `PreSignedAttributesSet` on success.
             **/
            setAttributesPreSigned: AugmentedSubmittable<(data: PalletNftsPreSignedAttributes | {
                collection?: any;
                item?: any;
                attributes?: any;
                namespace?: any;
                deadline?: any;
            } | string | Uint8Array, signature: SpRuntimeMultiSignature | {
                Ed25519: any;
            } | {
                Sr25519: any;
            } | {
                Ecdsa: any;
            } | string | Uint8Array, signer: AccountId32 | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [PalletNftsPreSignedAttributes, SpRuntimeMultiSignature, AccountId32]>;
            /**
             * Set the maximum number of items a collection could have.
             *
             * Origin must be either `ForceOrigin` or `Signed` and the sender should be the Owner of
             * the `collection`.
             *
             * - `collection`: The identifier of the collection to change.
             * - `max_supply`: The maximum number of items a collection could have.
             *
             * Emits `CollectionMaxSupplySet` event when successful.
             **/
            setCollectionMaxSupply: AugmentedSubmittable<(collection: u32 | AnyNumber | Uint8Array, maxSupply: u32 | AnyNumber | Uint8Array) => SubmittableExtrinsic<ApiType>, [u32, u32]>;
            /**
             * Set the metadata for a collection.
             *
             * Origin must be either `ForceOrigin` or `Signed` and the sender should be the Admin of
             * the `collection`.
             *
             * If the origin is `Signed`, then funds of signer are reserved according to the formula:
             * `MetadataDepositBase + DepositPerByte * data.len` taking into
             * account any already reserved funds.
             *
             * - `collection`: The identifier of the item whose metadata to update.
             * - `data`: The general information of this item. Limited in length by `StringLimit`.
             *
             * Emits `CollectionMetadataSet`.
             *
             * Weight: `O(1)`
             **/
            setCollectionMetadata: AugmentedSubmittable<(collection: u32 | AnyNumber | Uint8Array, data: Bytes | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [u32, Bytes]>;
            /**
             * Set the metadata for an item.
             *
             * Origin must be either `ForceOrigin` or Signed and the sender should be the Admin of the
             * `collection`.
             *
             * If the origin is Signed, then funds of signer are reserved according to the formula:
             * `MetadataDepositBase + DepositPerByte * data.len` taking into
             * account any already reserved funds.
             *
             * - `collection`: The identifier of the collection whose item's metadata to set.
             * - `item`: The identifier of the item whose metadata to set.
             * - `data`: The general information of this item. Limited in length by `StringLimit`.
             *
             * Emits `ItemMetadataSet`.
             *
             * Weight: `O(1)`
             **/
            setMetadata: AugmentedSubmittable<(collection: u32 | AnyNumber | Uint8Array, item: u32 | AnyNumber | Uint8Array, data: Bytes | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [u32, u32, Bytes]>;
            /**
             * Set (or reset) the price for an item.
             *
             * Origin must be Signed and must be the owner of the `item`.
             *
             * - `collection`: The collection of the item.
             * - `item`: The item to set the price for.
             * - `price`: The price for the item. Pass `None`, to reset the price.
             * - `buyer`: Restricts the buy operation to a specific account.
             *
             * Emits `ItemPriceSet` on success if the price is not `None`.
             * Emits `ItemPriceRemoved` on success if the price is `None`.
             **/
            setPrice: AugmentedSubmittable<(collection: u32 | AnyNumber | Uint8Array, item: u32 | AnyNumber | Uint8Array, price: Option<u128> | null | Uint8Array | u128 | AnyNumber, whitelistedBuyer: Option<MultiAddress> | null | Uint8Array | MultiAddress | {
                Id: any;
            } | {
                Index: any;
            } | {
                Raw: any;
            } | {
                Address32: any;
            } | {
                Address20: any;
            } | string) => SubmittableExtrinsic<ApiType>, [u32, u32, Option<u128>, Option<MultiAddress>]>;
            /**
             * Change the Issuer, Admin and Freezer of a collection.
             *
             * Origin must be either `ForceOrigin` or Signed and the sender should be the Owner of the
             * `collection`.
             *
             * Note: by setting the role to `None` only the `ForceOrigin` will be able to change it
             * after to `Some(account)`.
             *
             * - `collection`: The collection whose team should be changed.
             * - `issuer`: The new Issuer of this collection.
             * - `admin`: The new Admin of this collection.
             * - `freezer`: The new Freezer of this collection.
             *
             * Emits `TeamChanged`.
             *
             * Weight: `O(1)`
             **/
            setTeam: AugmentedSubmittable<(collection: u32 | AnyNumber | Uint8Array, issuer: Option<MultiAddress> | null | Uint8Array | MultiAddress | {
                Id: any;
            } | {
                Index: any;
            } | {
                Raw: any;
            } | {
                Address32: any;
            } | {
                Address20: any;
            } | string, admin: Option<MultiAddress> | null | Uint8Array | MultiAddress | {
                Id: any;
            } | {
                Index: any;
            } | {
                Raw: any;
            } | {
                Address32: any;
            } | {
                Address20: any;
            } | string, freezer: Option<MultiAddress> | null | Uint8Array | MultiAddress | {
                Id: any;
            } | {
                Index: any;
            } | {
                Raw: any;
            } | {
                Address32: any;
            } | {
                Address20: any;
            } | string) => SubmittableExtrinsic<ApiType>, [u32, Option<MultiAddress>, Option<MultiAddress>, Option<MultiAddress>]>;
            /**
             * Move an item from the sender account to another.
             *
             * Origin must be Signed and the signing account must be either:
             * - the Owner of the `item`;
             * - the approved delegate for the `item` (in this case, the approval is reset).
             *
             * Arguments:
             * - `collection`: The collection of the item to be transferred.
             * - `item`: The item to be transferred.
             * - `dest`: The account to receive ownership of the item.
             *
             * Emits `Transferred`.
             *
             * Weight: `O(1)`
             **/
            transfer: AugmentedSubmittable<(collection: u32 | AnyNumber | Uint8Array, item: u32 | AnyNumber | Uint8Array, dest: MultiAddress | {
                Id: any;
            } | {
                Index: any;
            } | {
                Raw: any;
            } | {
                Address32: any;
            } | {
                Address20: any;
            } | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [u32, u32, MultiAddress]>;
            /**
             * Change the Owner of a collection.
             *
             * Origin must be Signed and the sender should be the Owner of the `collection`.
             *
             * - `collection`: The collection whose owner should be changed.
             * - `owner`: The new Owner of this collection. They must have called
             * `set_accept_ownership` with `collection` in order for this operation to succeed.
             *
             * Emits `OwnerChanged`.
             *
             * Weight: `O(1)`
             **/
            transferOwnership: AugmentedSubmittable<(collection: u32 | AnyNumber | Uint8Array, newOwner: MultiAddress | {
                Id: any;
            } | {
                Index: any;
            } | {
                Raw: any;
            } | {
                Address32: any;
            } | {
                Address20: any;
            } | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [u32, MultiAddress]>;
            /**
             * Re-allow unprivileged transfer of an item.
             *
             * Origin must be Signed and the sender should be the Freezer of the `collection`.
             *
             * - `collection`: The collection of the item to be changed.
             * - `item`: The item to become transferable.
             *
             * Emits `ItemTransferUnlocked`.
             *
             * Weight: `O(1)`
             **/
            unlockItemTransfer: AugmentedSubmittable<(collection: u32 | AnyNumber | Uint8Array, item: u32 | AnyNumber | Uint8Array) => SubmittableExtrinsic<ApiType>, [u32, u32]>;
            /**
             * Update mint settings.
             *
             * Origin must be either `ForceOrigin` or `Signed` and the sender should be the Issuer
             * of the `collection`.
             *
             * - `collection`: The identifier of the collection to change.
             * - `mint_settings`: The new mint settings.
             *
             * Emits `CollectionMintSettingsUpdated` event when successful.
             **/
            updateMintSettings: AugmentedSubmittable<(collection: u32 | AnyNumber | Uint8Array, mintSettings: PalletNftsMintSettings | {
                mintType?: any;
                price?: any;
                startBlock?: any;
                endBlock?: any;
                defaultItemSettings?: any;
            } | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [u32, PalletNftsMintSettings]>;
            /**
             * Generic tx
             **/
            [key: string]: SubmittableExtrinsicFunction<ApiType>;
        };
        parachainInfo: {
            /**
             * Generic tx
             **/
            [key: string]: SubmittableExtrinsicFunction<ApiType>;
        };
        parachainSystem: {
            /**
             * Set the current validation data.
             *
             * This should be invoked exactly once per block. It will panic at the finalization
             * phase if the call was not invoked.
             *
             * The dispatch origin for this call must be `Inherent`
             *
             * As a side effect, this function upgrades the current validation function
             * if the appropriate time has come.
             **/
            setValidationData: AugmentedSubmittable<(data: CumulusPrimitivesParachainInherentParachainInherentData | {
                validationData?: any;
                relayChainState?: any;
                downwardMessages?: any;
                horizontalMessages?: any;
            } | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [CumulusPrimitivesParachainInherentParachainInherentData]>;
            sudoSendUpwardMessage: AugmentedSubmittable<(message: Bytes | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [Bytes]>;
            /**
             * Generic tx
             **/
            [key: string]: SubmittableExtrinsicFunction<ApiType>;
        };
        parameters: {
            /**
             * Set the value of a parameter.
             *
             * The dispatch origin of this call must be `AdminOrigin` for the given `key`. Values be
             * deleted by setting them to `None`.
             **/
            setParameter: AugmentedSubmittable<(keyValue: ShParachainRuntimeConfigsRuntimeParamsRuntimeParameters | {
                RuntimeConfig: any;
            } | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [ShParachainRuntimeConfigsRuntimeParamsRuntimeParameters]>;
            /**
             * Generic tx
             **/
            [key: string]: SubmittableExtrinsicFunction<ApiType>;
        };
        paymentStreams: {
            /**
             * Dispatchable extrinsic that allows Providers to charge multiple User's payment streams.
             *
             * The dispatch origin for this call must be Signed.
             * The origin must be the Provider that has at least one type of payment stream with each of the Users.
             *
             * Parameters:
             * - `user_accounts`: The array of User Account IDs that have payment streams with the Provider.
             *
             * This extrinsic will perform the following checks and logic:
             * 1. Check that the extrinsic was signed and get the signer.
             * 2. Check that the array of Users is not bigger than the maximum allowed.
             * 3. Execute a for loop for each User in the array of User Account IDs, in which it:
             * a. Checks that a payment stream between the signer (Provider) and the User exists
             * b. If there is a fixed-rate payment stream:
             * 1. Get the rate of the payment stream
             * 2. Get the difference between the last charged tick number and the last chargeable tick number of the stream
             * 3. Calculate the amount to charge doing `rate * difference`
             * 4. Charge the user (if the user does not have enough funds, it gets flagged and a `UserWithoutFunds` event is emitted)
             * 5. Update the last charged tick number of the payment stream
             * c. If there is a dynamic-rate payment stream:
             * 1. Get the amount provided by the Provider
             * 2. Get the difference between price index when the stream was last charged and the price index at the last chargeable tick
             * 3. Calculate the amount to charge doing `amount_provided * difference`
             * 4. Charge the user (if the user does not have enough funds, it gets flagged and a `UserWithoutFunds` event is emitted)
             * 5. Update the price index when the stream was last charged of the payment stream
             *
             * Emits a `PaymentStreamCharged` per User that had to pay and a `UsersCharged` event when successful.
             *
             * Notes: a Provider could have both a fixed-rate and a dynamic-rate payment stream with a User. If that's the case, this extrinsic
             * will try to charge both and the amount charged will be the sum of the amounts charged for each payment stream.
             **/
            chargeMultipleUsersPaymentStreams: AugmentedSubmittable<(userAccounts: Vec<AccountId32> | (AccountId32 | string | Uint8Array)[]) => SubmittableExtrinsic<ApiType>, [Vec<AccountId32>]>;
            /**
             * Dispatchable extrinsic that allows Providers to charge a payment stream from a User.
             *
             * The dispatch origin for this call must be Signed.
             * The origin must be the Provider that has at least one type of payment stream with the User.
             *
             * Parameters:
             * - `user_account`: The User Account ID that the payment stream is for.
             *
             * This extrinsic will perform the following checks and logic:
             * 1. Check that the extrinsic was signed and get the signer.
             * 2. Check that a payment stream between the signer (Provider) and the User exists
             * 3. If there is a fixed-rate payment stream:
             * 1. Get the rate of the payment stream
             * 2. Get the difference between the last charged tick number and the last chargeable tick number of the stream
             * 3. Calculate the amount to charge doing `rate * difference`
             * 4. Charge the user (if the user does not have enough funds, it gets flagged and a `UserWithoutFunds` event is emitted)
             * 5. Update the last charged tick number of the payment stream
             * 4. If there is a dynamic-rate payment stream:
             * 1. Get the amount provided by the Provider
             * 2. Get the difference between price index when the stream was last charged and the price index at the last chargeable tick
             * 3. Calculate the amount to charge doing `amount_provided * difference`
             * 4. Charge the user (if the user does not have enough funds, it gets flagged and a `UserWithoutFunds` event is emitted)
             * 5. Update the price index when the stream was last charged of the payment stream
             *
             * Emits a `PaymentStreamCharged` event when successful.
             *
             * Notes: a Provider could have both a fixed-rate and a dynamic-rate payment stream with a User. If that's the case, this extrinsic
             * will try to charge both and the amount charged will be the sum of the amounts charged for each payment stream.
             **/
            chargePaymentStreams: AugmentedSubmittable<(userAccount: AccountId32 | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [AccountId32]>;
            /**
             * Dispatchable extrinsic that allows a user flagged as without funds long ago enough to clear this flag from its account,
             * allowing it to begin contracting and paying for services again. It should have previously paid all its outstanding debt.
             *
             * The dispatch origin for this call must be Signed.
             * The origin must be the User that has been flagged as without funds.
             *
             * This extrinsic will perform the following checks and logic:
             * 1. Check that the extrinsic was signed and get the signer.
             * 2. Check that the user has been flagged as without funds.
             * 3. Check that the cooldown period has passed since the user was flagged as without funds.
             * 4. Check that there's no remaining outstanding debt.
             * 5. Unflag the user as without funds.
             *
             * Emits a 'UserSolvent' event when successful.
             **/
            clearInsolventFlag: AugmentedSubmittable<() => SubmittableExtrinsic<ApiType>, []>;
            /**
             * Dispatchable extrinsic that allows root to add a dynamic-rate payment stream from a User to a Provider.
             *
             * The dispatch origin for this call must be Root (Payment streams should only be added by traits in other pallets,
             * this extrinsic is for manual testing).
             *
             * Parameters:
             * - `provider_id`: The Provider ID that the payment stream is for.
             * - `user_account`: The User Account ID that the payment stream is for.
             * - `amount_provided`: The initial amount provided by the Provider.
             *
             * This extrinsic will perform the following checks and logic:
             * 1. Check that the extrinsic was executed by the root origin
             * 2. Check that the payment stream does not already exist
             * 3. Check that the User has enough funds to pay the deposit
             * 4. Hold the deposit from the User
             * 5. Update the Payment Streams storage to add the new payment stream
             *
             * Emits `DynamicRatePaymentStreamCreated` event when successful.
             **/
            createDynamicRatePaymentStream: AugmentedSubmittable<(providerId: H256 | string | Uint8Array, userAccount: AccountId32 | string | Uint8Array, amountProvided: u64 | AnyNumber | Uint8Array) => SubmittableExtrinsic<ApiType>, [H256, AccountId32, u64]>;
            /**
             * Dispatchable extrinsic that allows root to add a fixed-rate payment stream from a User to a Provider.
             *
             * The dispatch origin for this call must be Root (Payment streams should only be added by traits in other pallets,
             * this extrinsic is for manual testing).
             *
             * Parameters:
             * - `provider_id`: The Provider ID that the payment stream is for.
             * - `user_account`: The User Account ID that the payment stream is for.
             * - `rate`: The initial rate of the payment stream.
             *
             * This extrinsic will perform the following checks and logic:
             * 1. Check that the extrinsic was executed by the root origin
             * 2. Check that the payment stream does not already exist
             * 3. Check that the User has enough funds to pay the deposit
             * 4. Hold the deposit from the User
             * 5. Update the Payment Streams storage to add the new payment stream
             *
             * Emits `FixedRatePaymentStreamCreated` event when successful.
             **/
            createFixedRatePaymentStream: AugmentedSubmittable<(providerId: H256 | string | Uint8Array, userAccount: AccountId32 | string | Uint8Array, rate: u128 | AnyNumber | Uint8Array) => SubmittableExtrinsic<ApiType>, [H256, AccountId32, u128]>;
            /**
             * Dispatchable extrinsic that allows root to delete an existing dynamic-rate payment stream between a User and a Provider.
             *
             * The dispatch origin for this call must be Root (Payment streams should only be added by traits in other pallets,
             * this extrinsic is for manual testing).
             *
             * Parameters:
             * - `provider_id`: The Provider ID that the payment stream is for.
             * - `user_account`: The User Account ID that the payment stream is for.
             *
             * This extrinsic will perform the following checks and logic:
             * 1. Check that the extrinsic was executed by the root origin
             * 2. Check that the payment stream exists
             * 3. Update the Payment Streams storage to remove the payment stream
             *
             * Emits `DynamicRatePaymentStreamDeleted` event when successful.
             **/
            deleteDynamicRatePaymentStream: AugmentedSubmittable<(providerId: H256 | string | Uint8Array, userAccount: AccountId32 | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [H256, AccountId32]>;
            /**
             * Dispatchable extrinsic that allows root to delete an existing fixed-rate payment stream between a User and a Provider.
             *
             * The dispatch origin for this call must be Root (Payment streams should only be added by traits in other pallets,
             * this extrinsic is for manual testing).
             *
             * Parameters:
             * - `provider_id`: The Provider ID that the payment stream is for.
             * - `user_account`: The User Account ID that the payment stream is for.
             *
             * This extrinsic will perform the following checks and logic:
             * 1. Check that the extrinsic was executed by the root origin
             * 2. Check that the payment stream exists
             * 3. Update the Payment Streams storage to remove the payment stream
             *
             * Emits `FixedRatePaymentStreamDeleted` event when successful.
             **/
            deleteFixedRatePaymentStream: AugmentedSubmittable<(providerId: H256 | string | Uint8Array, userAccount: AccountId32 | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [H256, AccountId32]>;
            /**
             * Dispatchable extrinsic that allows a user flagged as without funds to pay the Providers that still have payment streams
             * with it, in order to recover as much of its deposits as possible.
             *
             * The dispatch origin for this call must be Signed.
             * The origin must be the User that has been flagged as without funds.
             *
             * This extrinsic will perform the following checks and logic:
             * 1. Check that the extrinsic was signed and get the signer.
             * 2. Check that the user has been flagged as without funds.
             * 3. Release the user's funds that were held as a deposit for each payment stream to be paid.
             * 4. Get the payment streams that the user has with the provided list of Providers, and pay them for the services.
             * 5. Delete the charged payment streams of the user.
             *
             * Emits a 'UserPaidSomeDebts' event when successful if the user has remaining debts. If the user has successfully paid all its debts,
             * it emits a 'UserPaidAllDebts' event.
             *
             * Notes: this extrinsic iterates over the provided list of Providers, getting the payment streams they have with the user and charging
             * them, so the execution could get expensive. It's recommended to provide a list of Providers that the user actually has payment streams with,
             * which can be obtained by calling the `get_providers_with_payment_streams_with_user` runtime API.
             * There was an idea to limit the amount of Providers that can be received by this extrinsic using a constant in the configuration of this pallet,
             * but the correct benchmarking of this extrinsic should be enough to avoid any potential abuse.
             **/
            payOutstandingDebt: AugmentedSubmittable<(providers: Vec<H256> | (H256 | string | Uint8Array)[]) => SubmittableExtrinsic<ApiType>, [Vec<H256>]>;
            /**
             * Dispatchable extrinsic that allows root to update an existing dynamic-rate payment stream between a User and a Provider.
             *
             * The dispatch origin for this call must be Root (Payment streams should only be added by traits in other pallets,
             * this extrinsic is for manual testing).
             *
             * Parameters:
             * - `provider_id`: The Provider ID that the payment stream is for.
             * - `user_account`: The User Account ID that the payment stream is for.
             * - `new_amount_provided`: The new amount provided by the Provider.
             *
             * This extrinsic will perform the following checks and logic:
             * 1. Check that the extrinsic was executed by the root origin
             * 2. Check that the payment stream exists
             * 3. Update the Payment Streams storage to update the payment stream
             *
             * Emits `DynamicRatePaymentStreamUpdated` event when successful.
             **/
            updateDynamicRatePaymentStream: AugmentedSubmittable<(providerId: H256 | string | Uint8Array, userAccount: AccountId32 | string | Uint8Array, newAmountProvided: u64 | AnyNumber | Uint8Array) => SubmittableExtrinsic<ApiType>, [H256, AccountId32, u64]>;
            /**
             * Dispatchable extrinsic that allows root to update an existing fixed-rate payment stream between a User and a Provider.
             *
             * The dispatch origin for this call must be Root (Payment streams should only be added by traits in other pallets,
             * this extrinsic is for manual testing).
             *
             * Parameters:
             * - `provider_id`: The Provider ID that the payment stream is for.
             * - `user_account`: The User Account ID that the payment stream is for.
             * - `new_rate`: The new rate of the payment stream.
             *
             * This extrinsic will perform the following checks and logic:
             * 1. Check that the extrinsic was executed by the root origin
             * 2. Check that the payment stream exists
             * 3. Update the Payment Streams storage to update the payment stream
             *
             * Emits `FixedRatePaymentStreamUpdated` event when successful.
             **/
            updateFixedRatePaymentStream: AugmentedSubmittable<(providerId: H256 | string | Uint8Array, userAccount: AccountId32 | string | Uint8Array, newRate: u128 | AnyNumber | Uint8Array) => SubmittableExtrinsic<ApiType>, [H256, AccountId32, u128]>;
            /**
             * Generic tx
             **/
            [key: string]: SubmittableExtrinsicFunction<ApiType>;
        };
        polkadotXcm: {
            /**
             * Claims assets trapped on this pallet because of leftover assets during XCM execution.
             *
             * - `origin`: Anyone can call this extrinsic.
             * - `assets`: The exact assets that were trapped. Use the version to specify what version
             * was the latest when they were trapped.
             * - `beneficiary`: The location/account where the claimed assets will be deposited.
             **/
            claimAssets: AugmentedSubmittable<(assets: XcmVersionedAssets | {
                V3: any;
            } | {
                V4: any;
            } | {
                V5: any;
            } | string | Uint8Array, beneficiary: XcmVersionedLocation | {
                V3: any;
            } | {
                V4: any;
            } | {
                V5: any;
            } | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [XcmVersionedAssets, XcmVersionedLocation]>;
            /**
             * Execute an XCM message from a local, signed, origin.
             *
             * An event is deposited indicating whether `msg` could be executed completely or only
             * partially.
             *
             * No more than `max_weight` will be used in its attempted execution. If this is less than
             * the maximum amount of weight that the message could take to be executed, then no
             * execution attempt will be made.
             **/
            execute: AugmentedSubmittable<(message: XcmVersionedXcm | {
                V3: any;
            } | {
                V4: any;
            } | {
                V5: any;
            } | string | Uint8Array, maxWeight: SpWeightsWeightV2Weight | {
                refTime?: any;
                proofSize?: any;
            } | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [XcmVersionedXcm, SpWeightsWeightV2Weight]>;
            /**
             * Set a safe XCM version (the version that XCM should be encoded with if the most recent
             * version a destination can accept is unknown).
             *
             * - `origin`: Must be an origin specified by AdminOrigin.
             * - `maybe_xcm_version`: The default XCM encoding version, or `None` to disable.
             **/
            forceDefaultXcmVersion: AugmentedSubmittable<(maybeXcmVersion: Option<u32> | null | Uint8Array | u32 | AnyNumber) => SubmittableExtrinsic<ApiType>, [Option<u32>]>;
            /**
             * Ask a location to notify us regarding their XCM version and any changes to it.
             *
             * - `origin`: Must be an origin specified by AdminOrigin.
             * - `location`: The location to which we should subscribe for XCM version notifications.
             **/
            forceSubscribeVersionNotify: AugmentedSubmittable<(location: XcmVersionedLocation | {
                V3: any;
            } | {
                V4: any;
            } | {
                V5: any;
            } | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [XcmVersionedLocation]>;
            /**
             * Set or unset the global suspension state of the XCM executor.
             *
             * - `origin`: Must be an origin specified by AdminOrigin.
             * - `suspended`: `true` to suspend, `false` to resume.
             **/
            forceSuspension: AugmentedSubmittable<(suspended: bool | boolean | Uint8Array) => SubmittableExtrinsic<ApiType>, [bool]>;
            /**
             * Require that a particular destination should no longer notify us regarding any XCM
             * version changes.
             *
             * - `origin`: Must be an origin specified by AdminOrigin.
             * - `location`: The location to which we are currently subscribed for XCM version
             * notifications which we no longer desire.
             **/
            forceUnsubscribeVersionNotify: AugmentedSubmittable<(location: XcmVersionedLocation | {
                V3: any;
            } | {
                V4: any;
            } | {
                V5: any;
            } | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [XcmVersionedLocation]>;
            /**
             * Extoll that a particular destination can be communicated with through a particular
             * version of XCM.
             *
             * - `origin`: Must be an origin specified by AdminOrigin.
             * - `location`: The destination that is being described.
             * - `xcm_version`: The latest version of XCM that `location` supports.
             **/
            forceXcmVersion: AugmentedSubmittable<(location: StagingXcmV5Location | {
                parents?: any;
                interior?: any;
            } | string | Uint8Array, version: u32 | AnyNumber | Uint8Array) => SubmittableExtrinsic<ApiType>, [StagingXcmV5Location, u32]>;
            /**
             * Transfer some assets from the local chain to the destination chain through their local,
             * destination or remote reserve.
             *
             * `assets` must have same reserve location and may not be teleportable to `dest`.
             * - `assets` have local reserve: transfer assets to sovereign account of destination
             * chain and forward a notification XCM to `dest` to mint and deposit reserve-based
             * assets to `beneficiary`.
             * - `assets` have destination reserve: burn local assets and forward a notification to
             * `dest` chain to withdraw the reserve assets from this chain's sovereign account and
             * deposit them to `beneficiary`.
             * - `assets` have remote reserve: burn local assets, forward XCM to reserve chain to move
             * reserves from this chain's SA to `dest` chain's SA, and forward another XCM to `dest`
             * to mint and deposit reserve-based assets to `beneficiary`.
             *
             * Fee payment on the destination side is made from the asset in the `assets` vector of
             * index `fee_asset_item`, up to enough to pay for `weight_limit` of weight. If more weight
             * is needed than `weight_limit`, then the operation will fail and the sent assets may be
             * at risk.
             *
             * - `origin`: Must be capable of withdrawing the `assets` and executing XCM.
             * - `dest`: Destination context for the assets. Will typically be `[Parent,
             * Parachain(..)]` to send from parachain to parachain, or `[Parachain(..)]` to send from
             * relay to parachain.
             * - `beneficiary`: A beneficiary location for the assets in the context of `dest`. Will
             * generally be an `AccountId32` value.
             * - `assets`: The assets to be withdrawn. This should include the assets used to pay the
             * fee on the `dest` (and possibly reserve) chains.
             * - `fee_asset_item`: The index into `assets` of the item which should be used to pay
             * fees.
             * - `weight_limit`: The remote-side weight limit, if any, for the XCM fee purchase.
             **/
            limitedReserveTransferAssets: AugmentedSubmittable<(dest: XcmVersionedLocation | {
                V3: any;
            } | {
                V4: any;
            } | {
                V5: any;
            } | string | Uint8Array, beneficiary: XcmVersionedLocation | {
                V3: any;
            } | {
                V4: any;
            } | {
                V5: any;
            } | string | Uint8Array, assets: XcmVersionedAssets | {
                V3: any;
            } | {
                V4: any;
            } | {
                V5: any;
            } | string | Uint8Array, feeAssetItem: u32 | AnyNumber | Uint8Array, weightLimit: XcmV3WeightLimit | {
                Unlimited: any;
            } | {
                Limited: any;
            } | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [XcmVersionedLocation, XcmVersionedLocation, XcmVersionedAssets, u32, XcmV3WeightLimit]>;
            /**
             * Teleport some assets from the local chain to some destination chain.
             *
             * Fee payment on the destination side is made from the asset in the `assets` vector of
             * index `fee_asset_item`, up to enough to pay for `weight_limit` of weight. If more weight
             * is needed than `weight_limit`, then the operation will fail and the sent assets may be
             * at risk.
             *
             * - `origin`: Must be capable of withdrawing the `assets` and executing XCM.
             * - `dest`: Destination context for the assets. Will typically be `[Parent,
             * Parachain(..)]` to send from parachain to parachain, or `[Parachain(..)]` to send from
             * relay to parachain.
             * - `beneficiary`: A beneficiary location for the assets in the context of `dest`. Will
             * generally be an `AccountId32` value.
             * - `assets`: The assets to be withdrawn. This should include the assets used to pay the
             * fee on the `dest` chain.
             * - `fee_asset_item`: The index into `assets` of the item which should be used to pay
             * fees.
             * - `weight_limit`: The remote-side weight limit, if any, for the XCM fee purchase.
             **/
            limitedTeleportAssets: AugmentedSubmittable<(dest: XcmVersionedLocation | {
                V3: any;
            } | {
                V4: any;
            } | {
                V5: any;
            } | string | Uint8Array, beneficiary: XcmVersionedLocation | {
                V3: any;
            } | {
                V4: any;
            } | {
                V5: any;
            } | string | Uint8Array, assets: XcmVersionedAssets | {
                V3: any;
            } | {
                V4: any;
            } | {
                V5: any;
            } | string | Uint8Array, feeAssetItem: u32 | AnyNumber | Uint8Array, weightLimit: XcmV3WeightLimit | {
                Unlimited: any;
            } | {
                Limited: any;
            } | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [XcmVersionedLocation, XcmVersionedLocation, XcmVersionedAssets, u32, XcmV3WeightLimit]>;
            /**
             * Transfer some assets from the local chain to the destination chain through their local,
             * destination or remote reserve.
             *
             * `assets` must have same reserve location and may not be teleportable to `dest`.
             * - `assets` have local reserve: transfer assets to sovereign account of destination
             * chain and forward a notification XCM to `dest` to mint and deposit reserve-based
             * assets to `beneficiary`.
             * - `assets` have destination reserve: burn local assets and forward a notification to
             * `dest` chain to withdraw the reserve assets from this chain's sovereign account and
             * deposit them to `beneficiary`.
             * - `assets` have remote reserve: burn local assets, forward XCM to reserve chain to move
             * reserves from this chain's SA to `dest` chain's SA, and forward another XCM to `dest`
             * to mint and deposit reserve-based assets to `beneficiary`.
             *
             * **This function is deprecated: Use `limited_reserve_transfer_assets` instead.**
             *
             * Fee payment on the destination side is made from the asset in the `assets` vector of
             * index `fee_asset_item`. The weight limit for fees is not provided and thus is unlimited,
             * with all fees taken as needed from the asset.
             *
             * - `origin`: Must be capable of withdrawing the `assets` and executing XCM.
             * - `dest`: Destination context for the assets. Will typically be `[Parent,
             * Parachain(..)]` to send from parachain to parachain, or `[Parachain(..)]` to send from
             * relay to parachain.
             * - `beneficiary`: A beneficiary location for the assets in the context of `dest`. Will
             * generally be an `AccountId32` value.
             * - `assets`: The assets to be withdrawn. This should include the assets used to pay the
             * fee on the `dest` (and possibly reserve) chains.
             * - `fee_asset_item`: The index into `assets` of the item which should be used to pay
             * fees.
             **/
            reserveTransferAssets: AugmentedSubmittable<(dest: XcmVersionedLocation | {
                V3: any;
            } | {
                V4: any;
            } | {
                V5: any;
            } | string | Uint8Array, beneficiary: XcmVersionedLocation | {
                V3: any;
            } | {
                V4: any;
            } | {
                V5: any;
            } | string | Uint8Array, assets: XcmVersionedAssets | {
                V3: any;
            } | {
                V4: any;
            } | {
                V5: any;
            } | string | Uint8Array, feeAssetItem: u32 | AnyNumber | Uint8Array) => SubmittableExtrinsic<ApiType>, [XcmVersionedLocation, XcmVersionedLocation, XcmVersionedAssets, u32]>;
            send: AugmentedSubmittable<(dest: XcmVersionedLocation | {
                V3: any;
            } | {
                V4: any;
            } | {
                V5: any;
            } | string | Uint8Array, message: XcmVersionedXcm | {
                V3: any;
            } | {
                V4: any;
            } | {
                V5: any;
            } | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [XcmVersionedLocation, XcmVersionedXcm]>;
            /**
             * Teleport some assets from the local chain to some destination chain.
             *
             * **This function is deprecated: Use `limited_teleport_assets` instead.**
             *
             * Fee payment on the destination side is made from the asset in the `assets` vector of
             * index `fee_asset_item`. The weight limit for fees is not provided and thus is unlimited,
             * with all fees taken as needed from the asset.
             *
             * - `origin`: Must be capable of withdrawing the `assets` and executing XCM.
             * - `dest`: Destination context for the assets. Will typically be `[Parent,
             * Parachain(..)]` to send from parachain to parachain, or `[Parachain(..)]` to send from
             * relay to parachain.
             * - `beneficiary`: A beneficiary location for the assets in the context of `dest`. Will
             * generally be an `AccountId32` value.
             * - `assets`: The assets to be withdrawn. This should include the assets used to pay the
             * fee on the `dest` chain.
             * - `fee_asset_item`: The index into `assets` of the item which should be used to pay
             * fees.
             **/
            teleportAssets: AugmentedSubmittable<(dest: XcmVersionedLocation | {
                V3: any;
            } | {
                V4: any;
            } | {
                V5: any;
            } | string | Uint8Array, beneficiary: XcmVersionedLocation | {
                V3: any;
            } | {
                V4: any;
            } | {
                V5: any;
            } | string | Uint8Array, assets: XcmVersionedAssets | {
                V3: any;
            } | {
                V4: any;
            } | {
                V5: any;
            } | string | Uint8Array, feeAssetItem: u32 | AnyNumber | Uint8Array) => SubmittableExtrinsic<ApiType>, [XcmVersionedLocation, XcmVersionedLocation, XcmVersionedAssets, u32]>;
            /**
             * Transfer some assets from the local chain to the destination chain through their local,
             * destination or remote reserve, or through teleports.
             *
             * Fee payment on the destination side is made from the asset in the `assets` vector of
             * index `fee_asset_item` (hence referred to as `fees`), up to enough to pay for
             * `weight_limit` of weight. If more weight is needed than `weight_limit`, then the
             * operation will fail and the sent assets may be at risk.
             *
             * `assets` (excluding `fees`) must have same reserve location or otherwise be teleportable
             * to `dest`, no limitations imposed on `fees`.
             * - for local reserve: transfer assets to sovereign account of destination chain and
             * forward a notification XCM to `dest` to mint and deposit reserve-based assets to
             * `beneficiary`.
             * - for destination reserve: burn local assets and forward a notification to `dest` chain
             * to withdraw the reserve assets from this chain's sovereign account and deposit them
             * to `beneficiary`.
             * - for remote reserve: burn local assets, forward XCM to reserve chain to move reserves
             * from this chain's SA to `dest` chain's SA, and forward another XCM to `dest` to mint
             * and deposit reserve-based assets to `beneficiary`.
             * - for teleports: burn local assets and forward XCM to `dest` chain to mint/teleport
             * assets and deposit them to `beneficiary`.
             *
             * - `origin`: Must be capable of withdrawing the `assets` and executing XCM.
             * - `dest`: Destination context for the assets. Will typically be `X2(Parent,
             * Parachain(..))` to send from parachain to parachain, or `X1(Parachain(..))` to send
             * from relay to parachain.
             * - `beneficiary`: A beneficiary location for the assets in the context of `dest`. Will
             * generally be an `AccountId32` value.
             * - `assets`: The assets to be withdrawn. This should include the assets used to pay the
             * fee on the `dest` (and possibly reserve) chains.
             * - `fee_asset_item`: The index into `assets` of the item which should be used to pay
             * fees.
             * - `weight_limit`: The remote-side weight limit, if any, for the XCM fee purchase.
             **/
            transferAssets: AugmentedSubmittable<(dest: XcmVersionedLocation | {
                V3: any;
            } | {
                V4: any;
            } | {
                V5: any;
            } | string | Uint8Array, beneficiary: XcmVersionedLocation | {
                V3: any;
            } | {
                V4: any;
            } | {
                V5: any;
            } | string | Uint8Array, assets: XcmVersionedAssets | {
                V3: any;
            } | {
                V4: any;
            } | {
                V5: any;
            } | string | Uint8Array, feeAssetItem: u32 | AnyNumber | Uint8Array, weightLimit: XcmV3WeightLimit | {
                Unlimited: any;
            } | {
                Limited: any;
            } | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [XcmVersionedLocation, XcmVersionedLocation, XcmVersionedAssets, u32, XcmV3WeightLimit]>;
            /**
             * Transfer assets from the local chain to the destination chain using explicit transfer
             * types for assets and fees.
             *
             * `assets` must have same reserve location or may be teleportable to `dest`. Caller must
             * provide the `assets_transfer_type` to be used for `assets`:
             * - `TransferType::LocalReserve`: transfer assets to sovereign account of destination
             * chain and forward a notification XCM to `dest` to mint and deposit reserve-based
             * assets to `beneficiary`.
             * - `TransferType::DestinationReserve`: burn local assets and forward a notification to
             * `dest` chain to withdraw the reserve assets from this chain's sovereign account and
             * deposit them to `beneficiary`.
             * - `TransferType::RemoteReserve(reserve)`: burn local assets, forward XCM to `reserve`
             * chain to move reserves from this chain's SA to `dest` chain's SA, and forward another
             * XCM to `dest` to mint and deposit reserve-based assets to `beneficiary`. Typically
             * the remote `reserve` is Asset Hub.
             * - `TransferType::Teleport`: burn local assets and forward XCM to `dest` chain to
             * mint/teleport assets and deposit them to `beneficiary`.
             *
             * On the destination chain, as well as any intermediary hops, `BuyExecution` is used to
             * buy execution using transferred `assets` identified by `remote_fees_id`.
             * Make sure enough of the specified `remote_fees_id` asset is included in the given list
             * of `assets`. `remote_fees_id` should be enough to pay for `weight_limit`. If more weight
             * is needed than `weight_limit`, then the operation will fail and the sent assets may be
             * at risk.
             *
             * `remote_fees_id` may use different transfer type than rest of `assets` and can be
             * specified through `fees_transfer_type`.
             *
             * The caller needs to specify what should happen to the transferred assets once they reach
             * the `dest` chain. This is done through the `custom_xcm_on_dest` parameter, which
             * contains the instructions to execute on `dest` as a final step.
             * This is usually as simple as:
             * `Xcm(vec![DepositAsset { assets: Wild(AllCounted(assets.len())), beneficiary }])`,
             * but could be something more exotic like sending the `assets` even further.
             *
             * - `origin`: Must be capable of withdrawing the `assets` and executing XCM.
             * - `dest`: Destination context for the assets. Will typically be `[Parent,
             * Parachain(..)]` to send from parachain to parachain, or `[Parachain(..)]` to send from
             * relay to parachain, or `(parents: 2, (GlobalConsensus(..), ..))` to send from
             * parachain across a bridge to another ecosystem destination.
             * - `assets`: The assets to be withdrawn. This should include the assets used to pay the
             * fee on the `dest` (and possibly reserve) chains.
             * - `assets_transfer_type`: The XCM `TransferType` used to transfer the `assets`.
             * - `remote_fees_id`: One of the included `assets` to be used to pay fees.
             * - `fees_transfer_type`: The XCM `TransferType` used to transfer the `fees` assets.
             * - `custom_xcm_on_dest`: The XCM to be executed on `dest` chain as the last step of the
             * transfer, which also determines what happens to the assets on the destination chain.
             * - `weight_limit`: The remote-side weight limit, if any, for the XCM fee purchase.
             **/
            transferAssetsUsingTypeAndThen: AugmentedSubmittable<(dest: XcmVersionedLocation | {
                V3: any;
            } | {
                V4: any;
            } | {
                V5: any;
            } | string | Uint8Array, assets: XcmVersionedAssets | {
                V3: any;
            } | {
                V4: any;
            } | {
                V5: any;
            } | string | Uint8Array, assetsTransferType: StagingXcmExecutorAssetTransferTransferType | {
                Teleport: any;
            } | {
                LocalReserve: any;
            } | {
                DestinationReserve: any;
            } | {
                RemoteReserve: any;
            } | string | Uint8Array, remoteFeesId: XcmVersionedAssetId | {
                V3: any;
            } | {
                V4: any;
            } | {
                V5: any;
            } | string | Uint8Array, feesTransferType: StagingXcmExecutorAssetTransferTransferType | {
                Teleport: any;
            } | {
                LocalReserve: any;
            } | {
                DestinationReserve: any;
            } | {
                RemoteReserve: any;
            } | string | Uint8Array, customXcmOnDest: XcmVersionedXcm | {
                V3: any;
            } | {
                V4: any;
            } | {
                V5: any;
            } | string | Uint8Array, weightLimit: XcmV3WeightLimit | {
                Unlimited: any;
            } | {
                Limited: any;
            } | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [XcmVersionedLocation, XcmVersionedAssets, StagingXcmExecutorAssetTransferTransferType, XcmVersionedAssetId, StagingXcmExecutorAssetTransferTransferType, XcmVersionedXcm, XcmV3WeightLimit]>;
            /**
             * Generic tx
             **/
            [key: string]: SubmittableExtrinsicFunction<ApiType>;
        };
        proofsDealer: {
            /**
             * Introduce a new challenge.
             *
             * This function allows authorized origins to add a new challenge to the `ChallengesQueue`.
             * The challenge will be dispatched in the coming blocks.
             * Users are charged a small fee for submitting a challenge, which
             * goes to the Treasury.
             **/
            challenge: AugmentedSubmittable<(key: H256 | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [H256]>;
            /**
             * Initialise a Provider's challenge cycle.
             *
             * Only callable by sudo.
             *
             * Sets the last tick the Provider submitted a proof for to the current tick, and sets the
             * deadline for submitting a proof to the current tick + the Provider's period + the tolerance.
             **/
            forceInitialiseChallengeCycle: AugmentedSubmittable<(provider: H256 | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [H256]>;
            priorityChallenge: AugmentedSubmittable<(key: H256 | string | Uint8Array, shouldRemoveKey: bool | boolean | Uint8Array) => SubmittableExtrinsic<ApiType>, [H256, bool]>;
            /**
             * Set the [`ChallengesTickerPaused`] to `true` or `false`.
             *
             * Only callable by sudo.
             **/
            setPaused: AugmentedSubmittable<(paused: bool | boolean | Uint8Array) => SubmittableExtrinsic<ApiType>, [bool]>;
            /**
             * For a Provider to submit a proof.
             *
             * Checks that `provider` is a registered Provider. If none
             * is provided, the proof submitter is considered to be the Provider.
             * Relies on a Providers pallet to get the root for the Provider.
             * Validates that the proof corresponds to a challenge that was made in the past,
             * by checking the [`TickToChallengesSeed`] StorageMap. The challenge tick that the
             * Provider should be submitting a proof for is retrieved from [`ProviderToProofSubmissionRecord`],
             * and it was calculated based on the last tick they submitted a proof for, and the challenge
             * period for that Provider, at the time of the previous proof submission or when it was
             * marked as slashable.
             *
             * This extrinsic also checks that there hasn't been a checkpoint challenge round
             * in between the last time the Provider submitted a proof for and the tick
             * for which the proof is being submitted. If there has been, the Provider is
             * expected to include responses to the checkpoint challenges in the proof.
             *
             * If valid:
             * - Pushes forward the Provider in the [`TickToProvidersDeadlines`] StorageMap a number
             * of ticks corresponding to the stake of the Provider.
             * - Registers the last tick for which the Provider submitted a proof for in
             * [`ProviderToProofSubmissionRecord`], as well as the next tick for which the Provider
             * should submit a proof for.
             *
             * Execution of this extrinsic should be refunded if the proof is valid.
             **/
            submitProof: AugmentedSubmittable<(proof: PalletProofsDealerProof | {
                forestProof?: any;
                keyProofs?: any;
            } | string | Uint8Array, provider: Option<H256> | null | Uint8Array | H256 | string) => SubmittableExtrinsic<ApiType>, [PalletProofsDealerProof, Option<H256>]>;
            /**
             * Generic tx
             **/
            [key: string]: SubmittableExtrinsicFunction<ApiType>;
        };
        providers: {
            /**
             * Dispatchable extrinsic that allows BSPs and MSPs to add a new multiaddress to their account.
             *
             * The dispatch origin for this call must be Signed.
             * The origin must be the account that wants to add a new multiaddress.
             *
             * Parameters:
             * - `new_multiaddress`: The new multiaddress that the signer wants to add to its account.
             *
             * This extrinsic will perform the following checks and logic:
             * 1. Check that the extrinsic was signed and get the signer.
             * 2. Check that the signer is registered as a MSP or BSP.
             * 3. Check that the Provider has not reached the maximum amount of multiaddresses.
             * 4. Check that the multiaddress is valid (size and any other relevant checks). TODO: Implement this.
             * 5. Update the Provider's storage to add the multiaddress.
             *
             * Emits `MultiAddressAdded` event when successful.
             **/
            addMultiaddress: AugmentedSubmittable<(newMultiaddress: Bytes | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [Bytes]>;
            /**
             * Dispatchable extrinsic only callable by an MSP that allows it to add a value proposition to its service
             *
             * The dispatch origin for this call must be Signed.
             * The origin must be the account that wants to add a value proposition.
             *
             * Emits `ValuePropAdded` event when successful.
             **/
            addValueProp: AugmentedSubmittable<(pricePerGigaUnitOfDataPerBlock: u128 | AnyNumber | Uint8Array, commitment: Bytes | string | Uint8Array, bucketDataLimit: u64 | AnyNumber | Uint8Array) => SubmittableExtrinsic<ApiType>, [u128, Bytes, u64]>;
            /**
             * Dispatchable extrinsic that allows users to sign off as a Backup Storage Provider.
             *
             * The dispatch origin for this call must be Signed.
             * The origin must be the account that wants to sign off as a Backup Storage Provider.
             *
             * This extrinsic will perform the following checks and logic:
             * 1. Check that the extrinsic was signed and get the signer.
             * 2. Check that the signer is registered as a BSP
             * 3. Check that the BSP has no storage assigned to it
             * 4. Update the BSPs storage, removing the signer as an BSP
             * 5. Update the total capacity of all BSPs, removing the capacity of the signer
             * 6. Return the deposit to the signer
             * 7. Decrement the storage that holds total amount of BSPs currently in the system
             *
             * Emits `BspSignOffSuccess` event when successful.
             **/
            bspSignOff: AugmentedSubmittable<() => SubmittableExtrinsic<ApiType>, []>;
            /**
             * Dispatchable extrinsic that allows a user with a pending Sign Up Request to cancel it, getting the deposit back.
             *
             * The dispatch origin for this call must be Signed.
             * The origin must be the account that requested to sign up as a Storage Provider.
             *
             * This extrinsic will perform the following checks and logic:
             * 1. Check that the extrinsic was signed and get the signer.
             * 2. Check that the signer has requested to sign up as a SP
             * 3. Delete the request from the Sign Up Requests storage
             * 4. Return the deposit to the signer
             *
             * Emits `SignUpRequestCanceled` event when successful.
             **/
            cancelSignUp: AugmentedSubmittable<() => SubmittableExtrinsic<ApiType>, []>;
            /**
             * Dispatchable extrinsic that allows users to change their amount of stored data
             *
             * The dispatch origin for this call must be Signed.
             * The origin must be the account that wants to change its capacity.
             *
             * Parameters:
             * - `new_capacity`: The new total amount of data that the Storage Provider wants to be able to store.
             *
             * This extrinsic will perform the following checks and logic:
             * 1. Check that the extrinsic was signed and get the signer.
             * 2. Check that the signer is registered as a SP
             * 3. Check that enough time has passed since the last time the SP changed its capacity
             * 4. Check that the new capacity is greater than the minimum required by the runtime
             * 5. Check that the new capacity is greater than the data used by this SP
             * 6. Calculate the new deposit needed for this new capacity
             * 7. Check to see if the new deposit needed is greater or less than the current deposit
             * a. If the new deposit is greater than the current deposit:
             * i. Check that the signer has enough funds to pay this extra deposit
             * ii. Hold the extra deposit from the signer
             * b. If the new deposit is less than the current deposit, return the held difference to the signer
             * 7. Update the SPs storage to change the total data
             * 8. If user is a BSP, update the total capacity of the network (sum of all capacities of BSPs)
             *
             * Emits `CapacityChanged` event when successful.
             **/
            changeCapacity: AugmentedSubmittable<(newCapacity: u64 | AnyNumber | Uint8Array) => SubmittableExtrinsic<ApiType>, [u64]>;
            /**
             * Dispatchable extrinsic that allows users to confirm their sign up as a Storage Provider, either MSP or BSP.
             *
             * The dispatch origin for this call must be Signed.
             * The origin must be the account that requested to sign up as a Storage Provider, except when providing a
             * `provider_account` parameter, in which case the origin can be any account.
             *
             * Parameters:
             * - `provider_account`: The account that requested to sign up as a Storage Provider. If not provided, the signer
             * will be considered the account that requested to sign up.
             *
             * This extrinsic will perform the following checks and logic:
             * 1. Check that the extrinsic was signed
             * 2. Check that the account received has requested to register as a SP
             * 3. Check that the current randomness is sufficiently fresh to be used as a salt for that request
             * 4. Check that the request has not expired
             * 5. Register the signer as a MSP or BSP with the data provided in the request
             *
             * Emits `MspSignUpSuccess` or `BspSignUpSuccess` event when successful, depending on the type of sign up.
             *
             * Notes:
             * - This extrinsic could be called by the user itself or by a third party
             * - The deposit that the user has to pay to register as a SP is held when the user requests to register as a SP
             * - If this extrinsic is successful, it will be free for the caller, to incentive state de-bloating
             **/
            confirmSignUp: AugmentedSubmittable<(providerAccount: Option<AccountId32> | null | Uint8Array | AccountId32 | string) => SubmittableExtrinsic<ApiType>, [Option<AccountId32>]>;
            /**
             * Delete a provider from the system.
             *
             * This can only be done if the following conditions are met:
             * - The provider is insolvent.
             * - The provider has no active payment streams.
             *
             * This is a free operation and can be called by anyone with a signed transaction.
             *
             * You can utilize the runtime API `can_delete_provider` to check if a provider can be deleted
             * to automate the process.
             *
             * Emits `MspDeleted` or `BspDeleted` event when successful.
             *
             * This operation is free if successful to encourage the community to delete insolvent providers,
             * debloating the state.
             **/
            deleteProvider: AugmentedSubmittable<(providerId: H256 | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [H256]>;
            /**
             * Dispatchable extrinsic that allows to forcefully and automatically sing up a Backup Storage Provider.
             *
             * The dispatch origin for this call must be Root.
             * The `who` parameter is the account that wants to sign up as a Backup Storage Provider.
             *
             * Funds proportional to the capacity requested are reserved (held) from the account passed as the `who` parameter.
             *
             * Parameters:
             * - `who`: The account that wants to sign up as a Backup Storage Provider.
             * - `bsp_id`: The Backup Storage Provider ID that the account passed as the `who` parameter is requesting to sign up as.
             * - `capacity`: The total amount of data that the Backup Storage Provider will be able to store.
             * - `multiaddresses`: The vector of multiaddresses that the signer wants to register (according to the
             * [Multiaddr spec](https://github.com/multiformats/multiaddr))
             *
             * This extrinsic will perform the steps of:
             * 1. [request_bsp_sign_up](crate::dispatchables::request_bsp_sign_up)
             * 2. [confirm_sign_up](crate::dispatchables::confirm_sign_up)
             *
             * Emits `BspRequestSignUpSuccess` and `BspSignUpSuccess` events when successful.
             **/
            forceBspSignUp: AugmentedSubmittable<(who: AccountId32 | string | Uint8Array, bspId: H256 | string | Uint8Array, capacity: u64 | AnyNumber | Uint8Array, multiaddresses: Vec<Bytes> | (Bytes | string | Uint8Array)[], paymentAccount: AccountId32 | string | Uint8Array, weight: Option<u32> | null | Uint8Array | u32 | AnyNumber) => SubmittableExtrinsic<ApiType>, [AccountId32, H256, u64, Vec<Bytes>, AccountId32, Option<u32>]>;
            /**
             * Dispatchable extrinsic that allows to forcefully and automatically sign up a Main Storage Provider.
             *
             * The dispatch origin for this call must be Root.
             * The `who` parameter is the account that wants to sign up as a Main Storage Provider.
             *
             * Funds proportional to the capacity requested are reserved (held) from the account passed as the `who` parameter.
             *
             * Parameters:
             * - `who`: The account that wants to sign up as a Main Storage Provider.
             * - `msp_id`: The Main Storage Provider ID that the account passed as the `who` parameter is requesting to sign up as.
             * - `capacity`: The total amount of data that the Main Storage Provider will be able to store.
             * - `multiaddresses`: The vector of multiaddresses that the signer wants to register (according to the
             * [Multiaddr spec](https://github.com/multiformats/multiaddr))
             * - `value_prop`: The value proposition that the signer will provide as a Main Storage Provider to
             * users and wants to register on-chain. It could be data limits, communication protocols to access the user's
             * data, and more.
             *
             * This extrinsic will perform the steps of:
             * 1. [request_msp_sign_up](crate::dispatchables::request_msp_sign_up)
             * 2. [confirm_sign_up](crate::dispatchables::confirm_sign_up)
             *
             * Emits `MspRequestSignUpSuccess` and `MspSignUpSuccess` events when successful.
             **/
            forceMspSignUp: AugmentedSubmittable<(who: AccountId32 | string | Uint8Array, mspId: H256 | string | Uint8Array, capacity: u64 | AnyNumber | Uint8Array, multiaddresses: Vec<Bytes> | (Bytes | string | Uint8Array)[], valuePropPricePerGigaUnitOfDataPerBlock: u128 | AnyNumber | Uint8Array, commitment: Bytes | string | Uint8Array, valuePropMaxDataLimit: u64 | AnyNumber | Uint8Array, paymentAccount: AccountId32 | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [AccountId32, H256, u64, Vec<Bytes>, u128, Bytes, u64, AccountId32]>;
            /**
             * Dispatchable extrinsic only callable by an MSP that allows it to make a value proposition unavailable.
             *
             * This operation cannot be reversed. You can only add new value propositions.
             * This will not affect existing buckets which are using this value proposition.
             **/
            makeValuePropUnavailable: AugmentedSubmittable<(valuePropId: H256 | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [H256]>;
            /**
             * Dispatchable extrinsic that allows users to sign off as a Main Storage Provider.
             *
             * The dispatch origin for this call must be Signed.
             * The origin must be the account that wants to sign off as a Main Storage Provider.
             *
             * This extrinsic will perform the following checks and logic:
             * 1. Check that the extrinsic was signed and get the signer.
             * 2. Check that the signer is registered as a MSP
             * 3. Check that the MSP has no storage assigned to it (no buckets or data used by it)
             * 4. Update the MSPs storage, removing the signer as an MSP
             * 5. Return the deposit to the signer
             * 6. Decrement the storage that holds total amount of MSPs currently in the system
             *
             * Emits `MspSignOffSuccess` event when successful.
             **/
            mspSignOff: AugmentedSubmittable<(mspId: H256 | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [H256]>;
            /**
             * Dispatchable extrinsic that allows BSPs and MSPs to remove an existing multiaddress from their account.
             *
             * The dispatch origin for this call must be Signed.
             * The origin must be the account that wants to remove a multiaddress.
             *
             * Parameters:
             * - `multiaddress`: The multiaddress that the signer wants to remove from its account.
             *
             * This extrinsic will perform the following checks and logic:
             * 1. Check that the extrinsic was signed and get the signer.
             * 2. Check that the signer is registered as a MSP or BSP.
             * 3. Check that the multiaddress exists in the Provider's account.
             * 4. Update the Provider's storage to remove the multiaddress.
             *
             * Emits `MultiAddressRemoved` event when successful.
             **/
            removeMultiaddress: AugmentedSubmittable<(multiaddress: Bytes | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [Bytes]>;
            /**
             * Dispatchable extrinsic that allows users to sign up as a Backup Storage Provider.
             *
             * The dispatch origin for this call must be Signed.
             * The origin must be the account that wants to sign up as a Backup Storage Provider.
             *
             * Funds proportional to the capacity requested are reserved (held) from the account.
             *
             * Parameters:
             * - `capacity`: The total amount of data that the Backup Storage Provider will be able to store.
             * - `multiaddresses`: The vector of multiaddresses that the signer wants to register (according to the
             * [Multiaddr spec](https://github.com/multiformats/multiaddr))
             *
             * This extrinsic will perform the following checks and logic:
             * 1. Check that the extrinsic was signed and get the signer.
             * 2. Check that the signer is not already registered as either a MSP or BSP
             * 3. Check that the multiaddress is valid
             * 4. Check that the data to be stored is greater than the minimum required by the runtime
             * 5. Calculate how much deposit will the signer have to pay using the amount of data it wants to store
             * 6. Check that the signer has enough funds to pay the deposit
             * 7. Hold the deposit from the signer
             * 8. Update the Sign Up Requests storage to add the signer as requesting to sign up as a BSP
             *
             * Emits `BspRequestSignUpSuccess` event when successful.
             **/
            requestBspSignUp: AugmentedSubmittable<(capacity: u64 | AnyNumber | Uint8Array, multiaddresses: Vec<Bytes> | (Bytes | string | Uint8Array)[], paymentAccount: AccountId32 | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [u64, Vec<Bytes>, AccountId32]>;
            /**
             * Dispatchable extrinsic that allows users to request to sign up as a Main Storage Provider.
             *
             * The dispatch origin for this call must be Signed.
             * The origin must be the account that wants to sign up as a Main Storage Provider.
             *
             * Funds proportional to the capacity requested are reserved (held) from the account.
             *
             * Parameters:
             * - `capacity`: The total amount of data that the Main Storage Provider will be able to store.
             * - `multiaddresses`: The vector of multiaddresses that the signer wants to register (according to the
             * [Multiaddr spec](https://github.com/multiformats/multiaddr))
             * - `value_prop`: The value proposition that the signer will provide as a Main Storage Provider to
             * users and wants to register on-chain. It could be data limits, communication protocols to access the user's
             * data, and more.
             *
             * This extrinsic will perform the following checks and logic:
             * 1. Check that the extrinsic was signed and get the signer.
             * 2. Check that the signer is not already registered as either a MSP or BSP
             * 3. Check that the multiaddress is valid
             * 4. Check that the data to be stored is greater than the minimum required by the runtime.
             * 5. Calculate how much deposit will the signer have to pay using the amount of data it wants to store
             * 6. Check that the signer has enough funds to pay the deposit
             * 7. Hold the deposit from the signer
             * 8. Update the Sign Up Requests storage to add the signer as requesting to sign up as a MSP
             *
             * Emits `MspRequestSignUpSuccess` event when successful.
             **/
            requestMspSignUp: AugmentedSubmittable<(capacity: u64 | AnyNumber | Uint8Array, multiaddresses: Vec<Bytes> | (Bytes | string | Uint8Array)[], valuePropPricePerGigaUnitOfDataPerBlock: u128 | AnyNumber | Uint8Array, commitment: Bytes | string | Uint8Array, valuePropMaxDataLimit: u64 | AnyNumber | Uint8Array, paymentAccount: AccountId32 | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [u64, Vec<Bytes>, u128, Bytes, u64, AccountId32]>;
            /**
             * Dispatchable extrinsic to slash a _slashable_ Storage Provider.
             *
             * A Storage Provider is _slashable_ iff it has failed to respond to challenges for providing proofs of storage.
             * In the context of the StorageHub protocol, the proofs-dealer pallet marks a Storage Provider as _slashable_ when it fails to respond to challenges.
             *
             * This is a free operation to incentivise the community to slash misbehaving providers.
             **/
            slash: AugmentedSubmittable<(providerId: H256 | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [H256]>;
            /**
             * BSP operation to stop all of your automatic cycles.
             *
             * This includes:
             *
             * - Commit reveal randomness cycle
             * - Proof challenge cycle
             *
             * If you are an BSP, the only requirement that must be met is that your root is the default one (an empty root).
             **/
            stopAllCycles: AugmentedSubmittable<() => SubmittableExtrinsic<ApiType>, []>;
            /**
             * Dispatchable extrinsic to top-up the deposit of a Storage Provider.
             *
             * The dispatch origin for this call must be signed.
             **/
            topUpDeposit: AugmentedSubmittable<() => SubmittableExtrinsic<ApiType>, []>;
            /**
             * Generic tx
             **/
            [key: string]: SubmittableExtrinsicFunction<ApiType>;
        };
        randomness: {
            /**
             * This inherent that must be included (DispatchClass::Mandatory) at each block saves the latest randomness available from the
             * relay chain into a variable that can then be used as a seed for commitments that happened during
             * the previous relay chain epoch
             **/
            setBabeRandomness: AugmentedSubmittable<() => SubmittableExtrinsic<ApiType>, []>;
            /**
             * Generic tx
             **/
            [key: string]: SubmittableExtrinsicFunction<ApiType>;
        };
        session: {
            /**
             * Removes any session key(s) of the function caller.
             *
             * This doesn't take effect until the next session.
             *
             * The dispatch origin of this function must be Signed and the account must be either be
             * convertible to a validator ID using the chain's typical addressing system (this usually
             * means being a controller account) or directly convertible into a validator ID (which
             * usually means being a stash account).
             *
             * ## Complexity
             * - `O(1)` in number of key types. Actual cost depends on the number of length of
             * `T::Keys::key_ids()` which is fixed.
             **/
            purgeKeys: AugmentedSubmittable<() => SubmittableExtrinsic<ApiType>, []>;
            /**
             * Sets the session key(s) of the function caller to `keys`.
             * Allows an account to set its session key prior to becoming a validator.
             * This doesn't take effect until the next session.
             *
             * The dispatch origin of this function must be signed.
             *
             * ## Complexity
             * - `O(1)`. Actual cost depends on the number of length of `T::Keys::key_ids()` which is
             * fixed.
             **/
            setKeys: AugmentedSubmittable<(keys: ShParachainRuntimeSessionKeys | {
                aura?: any;
            } | string | Uint8Array, proof: Bytes | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [ShParachainRuntimeSessionKeys, Bytes]>;
            /**
             * Generic tx
             **/
            [key: string]: SubmittableExtrinsicFunction<ApiType>;
        };
        sudo: {
            /**
             * Permanently removes the sudo key.
             *
             * **This cannot be un-done.**
             **/
            removeKey: AugmentedSubmittable<() => SubmittableExtrinsic<ApiType>, []>;
            /**
             * Authenticates the current sudo key and sets the given AccountId (`new`) as the new sudo
             * key.
             **/
            setKey: AugmentedSubmittable<(updated: MultiAddress | {
                Id: any;
            } | {
                Index: any;
            } | {
                Raw: any;
            } | {
                Address32: any;
            } | {
                Address20: any;
            } | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [MultiAddress]>;
            /**
             * Authenticates the sudo key and dispatches a function call with `Root` origin.
             **/
            sudo: AugmentedSubmittable<(call: Call | IMethod | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [Call]>;
            /**
             * Authenticates the sudo key and dispatches a function call with `Signed` origin from
             * a given account.
             *
             * The dispatch origin for this call must be _Signed_.
             **/
            sudoAs: AugmentedSubmittable<(who: MultiAddress | {
                Id: any;
            } | {
                Index: any;
            } | {
                Raw: any;
            } | {
                Address32: any;
            } | {
                Address20: any;
            } | string | Uint8Array, call: Call | IMethod | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [MultiAddress, Call]>;
            /**
             * Authenticates the sudo key and dispatches a function call with `Root` origin.
             * This function does not check the weight of the call, and instead allows the
             * Sudo user to specify the weight of the call.
             *
             * The dispatch origin for this call must be _Signed_.
             **/
            sudoUncheckedWeight: AugmentedSubmittable<(call: Call | IMethod | string | Uint8Array, weight: SpWeightsWeightV2Weight | {
                refTime?: any;
                proofSize?: any;
            } | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [Call, SpWeightsWeightV2Weight]>;
            /**
             * Generic tx
             **/
            [key: string]: SubmittableExtrinsicFunction<ApiType>;
        };
        system: {
            /**
             * Provide the preimage (runtime binary) `code` for an upgrade that has been authorized.
             *
             * If the authorization required a version check, this call will ensure the spec name
             * remains unchanged and that the spec version has increased.
             *
             * Depending on the runtime's `OnSetCode` configuration, this function may directly apply
             * the new `code` in the same block or attempt to schedule the upgrade.
             *
             * All origins are allowed.
             **/
            applyAuthorizedUpgrade: AugmentedSubmittable<(code: Bytes | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [Bytes]>;
            /**
             * Authorize an upgrade to a given `code_hash` for the runtime. The runtime can be supplied
             * later.
             *
             * This call requires Root origin.
             **/
            authorizeUpgrade: AugmentedSubmittable<(codeHash: H256 | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [H256]>;
            /**
             * Authorize an upgrade to a given `code_hash` for the runtime. The runtime can be supplied
             * later.
             *
             * WARNING: This authorizes an upgrade that will take place without any safety checks, for
             * example that the spec name remains the same and that the version number increases. Not
             * recommended for normal use. Use `authorize_upgrade` instead.
             *
             * This call requires Root origin.
             **/
            authorizeUpgradeWithoutChecks: AugmentedSubmittable<(codeHash: H256 | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [H256]>;
            /**
             * Kill all storage items with a key that starts with the given prefix.
             *
             * **NOTE:** We rely on the Root origin to provide us the number of subkeys under
             * the prefix we are removing to accurately calculate the weight of this function.
             **/
            killPrefix: AugmentedSubmittable<(prefix: Bytes | string | Uint8Array, subkeys: u32 | AnyNumber | Uint8Array) => SubmittableExtrinsic<ApiType>, [Bytes, u32]>;
            /**
             * Kill some items from storage.
             **/
            killStorage: AugmentedSubmittable<(keys: Vec<Bytes> | (Bytes | string | Uint8Array)[]) => SubmittableExtrinsic<ApiType>, [Vec<Bytes>]>;
            /**
             * Make some on-chain remark.
             *
             * Can be executed by every `origin`.
             **/
            remark: AugmentedSubmittable<(remark: Bytes | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [Bytes]>;
            /**
             * Make some on-chain remark and emit event.
             **/
            remarkWithEvent: AugmentedSubmittable<(remark: Bytes | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [Bytes]>;
            /**
             * Set the new runtime code.
             **/
            setCode: AugmentedSubmittable<(code: Bytes | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [Bytes]>;
            /**
             * Set the new runtime code without doing any checks of the given `code`.
             *
             * Note that runtime upgrades will not run if this is called with a not-increasing spec
             * version!
             **/
            setCodeWithoutChecks: AugmentedSubmittable<(code: Bytes | string | Uint8Array) => SubmittableExtrinsic<ApiType>, [Bytes]>;
            /**
             * Set the number of pages in the WebAssembly environment's heap.
             **/
            setHeapPages: AugmentedSubmittable<(pages: u64 | AnyNumber | Uint8Array) => SubmittableExtrinsic<ApiType>, [u64]>;
            /**
             * Set some items of storage.
             **/
            setStorage: AugmentedSubmittable<(items: Vec<ITuple<[Bytes, Bytes]>> | ([Bytes | string | Uint8Array, Bytes | string | Uint8Array])[]) => SubmittableExtrinsic<ApiType>, [Vec<ITuple<[Bytes, Bytes]>>]>;
            /**
             * Generic tx
             **/
            [key: string]: SubmittableExtrinsicFunction<ApiType>;
        };
        timestamp: {
            /**
             * Set the current time.
             *
             * This call should be invoked exactly once per block. It will panic at the finalization
             * phase, if this call hasn't been invoked by that time.
             *
             * The timestamp should be greater than the previous one by the amount specified by
             * [`Config::MinimumPeriod`].
             *
             * The dispatch origin for this call must be _None_.
             *
             * This dispatch class is _Mandatory_ to ensure it gets executed in the block. Be aware
             * that changing the complexity of this call could result exhausting the resources in a
             * block to execute any other calls.
             *
             * ## Complexity
             * - `O(1)` (Note that implementations of `OnTimestampSet` must also be `O(1)`)
             * - 1 storage read and 1 storage mutation (codec `O(1)` because of `DidUpdate::take` in
             * `on_finalize`)
             * - 1 event handler `on_timestamp_set`. Must be `O(1)`.
             **/
            set: AugmentedSubmittable<(now: Compact<u64> | AnyNumber | Uint8Array) => SubmittableExtrinsic<ApiType>, [Compact<u64>]>;
            /**
             * Generic tx
             **/
            [key: string]: SubmittableExtrinsicFunction<ApiType>;
        };
        xcmpQueue: {
            /**
             * Resumes all XCM executions for the XCMP queue.
             *
             * Note that this function doesn't change the status of the in/out bound channels.
             *
             * - `origin`: Must pass `ControllerOrigin`.
             **/
            resumeXcmExecution: AugmentedSubmittable<() => SubmittableExtrinsic<ApiType>, []>;
            /**
             * Suspends all XCM executions for the XCMP queue, regardless of the sender's origin.
             *
             * - `origin`: Must pass `ControllerOrigin`.
             **/
            suspendXcmExecution: AugmentedSubmittable<() => SubmittableExtrinsic<ApiType>, []>;
            /**
             * Overwrites the number of pages which must be in the queue after which we drop any
             * further messages from the channel.
             *
             * - `origin`: Must pass `Root`.
             * - `new`: Desired value for `QueueConfigData.drop_threshold`
             **/
            updateDropThreshold: AugmentedSubmittable<(updated: u32 | AnyNumber | Uint8Array) => SubmittableExtrinsic<ApiType>, [u32]>;
            /**
             * Overwrites the number of pages which the queue must be reduced to before it signals
             * that message sending may recommence after it has been suspended.
             *
             * - `origin`: Must pass `Root`.
             * - `new`: Desired value for `QueueConfigData.resume_threshold`
             **/
            updateResumeThreshold: AugmentedSubmittable<(updated: u32 | AnyNumber | Uint8Array) => SubmittableExtrinsic<ApiType>, [u32]>;
            /**
             * Overwrites the number of pages which must be in the queue for the other side to be
             * told to suspend their sending.
             *
             * - `origin`: Must pass `Root`.
             * - `new`: Desired value for `QueueConfigData.suspend_value`
             **/
            updateSuspendThreshold: AugmentedSubmittable<(updated: u32 | AnyNumber | Uint8Array) => SubmittableExtrinsic<ApiType>, [u32]>;
            /**
             * Generic tx
             **/
            [key: string]: SubmittableExtrinsicFunction<ApiType>;
        };
    }
}
