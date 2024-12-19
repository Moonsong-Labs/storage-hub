import "@polkadot/api-base/types/consts";
import type { ApiTypes, AugmentedConst } from "@polkadot/api-base/types";
import type { Option, u128, u16, u32, u64, u8 } from "@polkadot/types-codec";
import type { Codec } from "@polkadot/types-codec/types";
import type { AccountId32, H256, Perbill } from "@polkadot/types/interfaces/runtime";
import type {
  FrameSystemLimitsBlockLength,
  FrameSystemLimitsBlockWeights,
  SpVersionRuntimeVersion,
  SpWeightsRuntimeDbWeight,
  SpWeightsWeightV2Weight
} from "@polkadot/types/lookup";
export type __AugmentedConst<ApiType extends ApiTypes> = AugmentedConst<ApiType>;
declare module "@polkadot/api-base/types/consts" {
  interface AugmentedConsts<ApiType extends ApiTypes> {
    aura: {
      /**
       * The slot duration Aura should run with, expressed in milliseconds.
       * The effective value of this type should not change while the chain is running.
       *
       * For backwards compatibility either use [`MinimumPeriodTimesTwo`] or a const.
       **/
      slotDuration: u64 & AugmentedConst<ApiType>;
      /**
       * Generic const
       **/
      [key: string]: Codec;
    };
    balances: {
      /**
       * The minimum amount required to keep an account open. MUST BE GREATER THAN ZERO!
       *
       * If you *really* need it to be zero, you can enable the feature `insecure_zero_ed` for
       * this pallet. However, you do so at your own risk: this will open up a major DoS vector.
       * In case you have multiple sources of provider references, you may also get unexpected
       * behaviour if you set this to zero.
       *
       * Bottom line: Do yourself a favour and make it at least one!
       **/
      existentialDeposit: u128 & AugmentedConst<ApiType>;
      /**
       * The maximum number of individual freeze locks that can exist on an account at any time.
       **/
      maxFreezes: u32 & AugmentedConst<ApiType>;
      /**
       * The maximum number of locks that should exist on an account.
       * Not strictly enforced, but used for weight estimation.
       *
       * Use of locks is deprecated in favour of freezes. See `https://github.com/paritytech/substrate/pull/12951/`
       **/
      maxLocks: u32 & AugmentedConst<ApiType>;
      /**
       * The maximum number of named reserves that can exist on an account.
       *
       * Use of reserves is deprecated in favour of holds. See `https://github.com/paritytech/substrate/pull/12951/`
       **/
      maxReserves: u32 & AugmentedConst<ApiType>;
      /**
       * Generic const
       **/
      [key: string]: Codec;
    };
    fileSystem: {
      /**
       * Penalty payed by a BSP when they forcefully stop storing a file.
       **/
      bspStopStoringFilePenalty: u128 & AugmentedConst<ApiType>;
      /**
       * Default replication target
       **/
      defaultReplicationTarget: u32 & AugmentedConst<ApiType>;
      /**
       * Maximum batch of storage requests that can be confirmed at once when calling `bsp_confirm_storing`.
       **/
      maxBatchConfirmStorageRequests: u32 & AugmentedConst<ApiType>;
      /**
       * Maximum batch of storage requests that can be responded to at once when calling `msp_respond_storage_requests_multiple_buckets`.
       **/
      maxBatchMspRespondStorageRequests: u32 & AugmentedConst<ApiType>;
      /**
       * Maximum number of multiaddresses for a storage request.
       **/
      maxDataServerMultiAddresses: u32 & AugmentedConst<ApiType>;
      /**
       * Maximum number of expired items (per type) to clean up in a single block.
       **/
      maxExpiredItemsInBlock: u32 & AugmentedConst<ApiType>;
      /**
       * Maximum byte size of a file path.
       **/
      maxFilePathSize: u32 & AugmentedConst<ApiType>;
      /**
       * Maximum number of peer ids for a storage request.
       **/
      maxNumberOfPeerIds: u32 & AugmentedConst<ApiType>;
      /**
       * Maximum byte size of a peer id.
       **/
      maxPeerIdSize: u32 & AugmentedConst<ApiType>;
      /**
       * Maximum number of file deletion requests a user can have pending.
       **/
      maxUserPendingDeletionRequests: u32 & AugmentedConst<ApiType>;
      /**
       * Maximum number of move bucket requests a user can have pending.
       **/
      maxUserPendingMoveBucketRequests: u32 & AugmentedConst<ApiType>;
      /**
       * Number of blocks required to pass between a BSP requesting to stop storing a file and it being able to confirm to stop storing it.
       **/
      minWaitForStopStoring: u32 & AugmentedConst<ApiType>;
      /**
       * Time-to-live for a move bucket request, after which the request is considered expired.
       **/
      moveBucketRequestTtl: u32 & AugmentedConst<ApiType>;
      /**
       * Time-to-live for a pending file deletion request, after which a priority challenge is sent out to enforce the deletion.
       **/
      pendingFileDeletionRequestTtl: u32 & AugmentedConst<ApiType>;
      /**
       * Deposit held from the User when creating a new storage request
       **/
      storageRequestCreationDeposit: u128 & AugmentedConst<ApiType>;
      /**
       * Time-to-live for a storage request.
       **/
      storageRequestTtl: u32 & AugmentedConst<ApiType>;
      /**
       * The treasury account of the runtime, where a fraction of each payment goes.
       **/
      treasuryAccount: AccountId32 & AugmentedConst<ApiType>;
      /**
       * Generic const
       **/
      [key: string]: Codec;
    };
    messageQueue: {
      /**
       * The size of the page; this implies the maximum message size which can be sent.
       *
       * A good value depends on the expected message sizes, their weights, the weight that is
       * available for processing them and the maximal needed message size. The maximal message
       * size is slightly lower than this as defined by [`MaxMessageLenOf`].
       **/
      heapSize: u32 & AugmentedConst<ApiType>;
      /**
       * The maximum amount of weight (if any) to be used from remaining weight `on_idle` which
       * should be provided to the message queue for servicing enqueued items `on_idle`.
       * Useful for parachains to process messages at the same block they are received.
       *
       * If `None`, it will not call `ServiceQueues::service_queues` in `on_idle`.
       **/
      idleMaxServiceWeight: Option<SpWeightsWeightV2Weight> & AugmentedConst<ApiType>;
      /**
       * The maximum number of stale pages (i.e. of overweight messages) allowed before culling
       * can happen. Once there are more stale pages than this, then historical pages may be
       * dropped, even if they contain unprocessed overweight messages.
       **/
      maxStale: u32 & AugmentedConst<ApiType>;
      /**
       * The amount of weight (if any) which should be provided to the message queue for
       * servicing enqueued items `on_initialize`.
       *
       * This may be legitimately `None` in the case that you will call
       * `ServiceQueues::service_queues` manually or set [`Self::IdleMaxServiceWeight`] to have
       * it run in `on_idle`.
       **/
      serviceWeight: Option<SpWeightsWeightV2Weight> & AugmentedConst<ApiType>;
      /**
       * Generic const
       **/
      [key: string]: Codec;
    };
    nfts: {
      /**
       * The maximum approvals an item could have.
       **/
      approvalsLimit: u32 & AugmentedConst<ApiType>;
      /**
       * The basic amount of funds that must be reserved when adding an attribute to an item.
       **/
      attributeDepositBase: u128 & AugmentedConst<ApiType>;
      /**
       * The basic amount of funds that must be reserved for collection.
       **/
      collectionDeposit: u128 & AugmentedConst<ApiType>;
      /**
       * The additional funds that must be reserved for the number of bytes store in metadata,
       * either "normal" metadata or attribute metadata.
       **/
      depositPerByte: u128 & AugmentedConst<ApiType>;
      /**
       * Disables some of pallet's features.
       **/
      features: u64 & AugmentedConst<ApiType>;
      /**
       * The maximum attributes approvals an item could have.
       **/
      itemAttributesApprovalsLimit: u32 & AugmentedConst<ApiType>;
      /**
       * The basic amount of funds that must be reserved for an item.
       **/
      itemDeposit: u128 & AugmentedConst<ApiType>;
      /**
       * The maximum length of an attribute key.
       **/
      keyLimit: u32 & AugmentedConst<ApiType>;
      /**
       * The max number of attributes a user could set per call.
       **/
      maxAttributesPerCall: u32 & AugmentedConst<ApiType>;
      /**
       * The max duration in blocks for deadlines.
       **/
      maxDeadlineDuration: u32 & AugmentedConst<ApiType>;
      /**
       * The max number of tips a user could send.
       **/
      maxTips: u32 & AugmentedConst<ApiType>;
      /**
       * The basic amount of funds that must be reserved when adding metadata to your item.
       **/
      metadataDepositBase: u128 & AugmentedConst<ApiType>;
      /**
       * The maximum length of data stored on-chain.
       **/
      stringLimit: u32 & AugmentedConst<ApiType>;
      /**
       * The maximum length of an attribute value.
       **/
      valueLimit: u32 & AugmentedConst<ApiType>;
      /**
       * Generic const
       **/
      [key: string]: Codec;
    };
    parachainSystem: {
      /**
       * Returns the parachain ID we are running with.
       **/
      selfParaId: u32 & AugmentedConst<ApiType>;
      /**
       * Generic const
       **/
      [key: string]: Codec;
    };
    paymentStreams: {
      /**
       * The base deposit for a new payment stream. The actual deposit will be this constant + the deposit calculated using the `NewStreamDeposit` constant.
       **/
      baseDeposit: u128 & AugmentedConst<ApiType>;
      /**
       * The maximum amount of Users that a Provider can charge in a single extrinsic execution.
       * This is used to prevent a Provider from charging too many Users in a single block, which could lead to a DoS attack.
       **/
      maxUsersToCharge: u32 & AugmentedConst<ApiType>;
      /**
       * The number of ticks that correspond to the deposit that a User has to pay to open a payment stream.
       * This means that, from the balance of the User for which the payment stream is being created, the amount
       * `NewStreamDeposit * rate + BaseDeposit` will be held as a deposit.
       * In the case of dynamic-rate payment streams, `rate` will be `amount_provided_in_giga_units * price_per_giga_unit_per_tick`, where `price_per_giga_unit_per_tick` is
       * obtained from the `CurrentPricePerGigaUnitPerTick` storage.
       **/
      newStreamDeposit: u32 & AugmentedConst<ApiType>;
      /**
       * The treasury account of the runtime, where a fraction of each payment goes.
       **/
      treasuryAccount: AccountId32 & AugmentedConst<ApiType>;
      /**
       * The number of ticks that a user will have to wait after it has been flagged as without funds to be able to clear that flag
       * and be able to pay for services again. If there's any outstanding debt when the flag is cleared, it will be paid.
       **/
      userWithoutFundsCooldown: u32 & AugmentedConst<ApiType>;
      /**
       * Generic const
       **/
      [key: string]: Codec;
    };
    proofsDealer: {
      /**
       * The minimum unused weight that a block must have to be considered _not_ full.
       *
       * This is used as part of the criteria for checking if the network is presumably under a spam attack.
       * For example, this can be set to the benchmarked weight of a `submit_proof` extrinsic, which would
       * mean that a block is not considered full if a `submit_proof` extrinsic could have still fit in it.
       **/
      blockFullnessHeadroom: SpWeightsWeightV2Weight & AugmentedConst<ApiType>;
      /**
       * The period of blocks for which the block fullness is checked.
       *
       * This is the amount of blocks from the past, for which the block fullness has been checked
       * and is stored. Blocks older than `current_block` - [`Config::BlockFullnessPeriod`] are
       * cleared from storage.
       *
       * This constant should be equal or smaller than the [`Config::ChallengeTicksTolerance`] constant,
       * if the goal is to prevent spamming attacks that would prevent honest Providers from submitting
       * their proofs in time.
       **/
      blockFullnessPeriod: u32 & AugmentedConst<ApiType>;
      /**
       * The number of ticks that challenges history is kept for.
       * After this many ticks, challenges are removed from [`TickToChallengesSeed`] StorageMap.
       * A "tick" is usually one block, but some blocks may be skipped due to migrations.
       **/
      challengeHistoryLength: u32 & AugmentedConst<ApiType>;
      /**
       * The fee charged for submitting a challenge.
       * This fee goes to the Treasury, and is used to prevent spam. Registered Providers are
       * exempt from this fee.
       **/
      challengesFee: u128 & AugmentedConst<ApiType>;
      /**
       * The length of the `ChallengesQueue` StorageValue.
       * This is to limit the size of the queue, and therefore the number of
       * manual challenges that can be made.
       **/
      challengesQueueLength: u32 & AugmentedConst<ApiType>;
      /**
       * The tolerance in number of ticks (almost equivalent to blocks, but skipping MBM) that
       * a Provider has to submit a proof, counting from the tick the challenge is emitted for
       * that Provider.
       *
       * For example, if a Provider is supposed to submit a proof for tick `n`, and the tolerance
       * is set to `t`, then the Provider has to submit a proof for challenges in tick `n`, before
       * `n + t`.
       **/
      challengeTicksTolerance: u32 & AugmentedConst<ApiType>;
      /**
       * The number of blocks in between a checkpoint challenges round (i.e. with custom challenges).
       * This is used to determine when to include the challenges from the `ChallengesQueue` and
       * `PriorityChallengesQueue` in the `BlockToChallenges` StorageMap. These checkpoint challenge
       * rounds have to be answered by ALL Providers, and this is enforced by the `submit_proof`
       * extrinsic.
       *
       * WARNING: This period needs to be equal or larger than the challenge period of the smallest
       * Provider in the network. If the smallest Provider has a challenge period of 10 ticks (blocks),
       * then the checkpoint challenge period needs to be at least 10 ticks.
       **/
      checkpointChallengePeriod: u32 & AugmentedConst<ApiType>;
      /**
       * The maximum number of custom challenges that can be made in a single checkpoint block.
       **/
      maxCustomChallengesPerBlock: u32 & AugmentedConst<ApiType>;
      /**
       * The maximum number of Providers that can be slashed per tick.
       *
       * Providers are marked as slashable if they are found in the [`TickToProvidersDeadlines`] StorageMap
       * for the current challenges tick. It is expected that most of the times, there will be little to
       * no Providers in the [`TickToProvidersDeadlines`] StorageMap for the current challenges tick. That
       * is because Providers are expected to submit proofs in time. However, in the extreme scenario where
       * a large number of Providers are missing the proof submissions, this configuration is used to keep
       * the execution of the `on_poll` hook bounded.
       **/
      maxSlashableProvidersPerTick: u32 & AugmentedConst<ApiType>;
      /**
       * The maximum amount of Providers that can submit a proof in a single block.
       * Although this can be seen as an arbitrary limit, if set to the already existing
       * implicit limit that is "how many `submit_proof` extrinsics fit in the weight of
       * a block, this wouldn't add any additional artificial limit.
       **/
      maxSubmittersPerTick: u32 & AugmentedConst<ApiType>;
      /**
       * The minimum period in which a Provider can be challenged, regardless of their stake.
       **/
      minChallengePeriod: u32 & AugmentedConst<ApiType>;
      /**
       * The minimum ratio (or percentage if you will) of blocks that must be considered _not_ full,
       * from the total number of [`Config::BlockFullnessPeriod`] blocks taken into account.
       *
       * If less than this percentage of blocks are not full, the networks is considered to be presumably
       * under a spam attack.
       * This can also be thought of as the maximum ratio of misbehaving collators tolerated. For example,
       * if this is set to `Perbill::from_percent(50)`, then if more than half of the last [`Config::BlockFullnessPeriod`]
       * blocks are not full, then one of those blocks surely was produced by an honest collator, meaning
       * that there was at least one truly _not_ full block in the last [`Config::BlockFullnessPeriod`] blocks.
       **/
      minNotFullBlocksRatio: Perbill & AugmentedConst<ApiType>;
      /**
       * The number of random challenges that are generated per block, using the random seed
       * generated for that block.
       **/
      randomChallengesPerBlock: u32 & AugmentedConst<ApiType>;
      /**
       * The ratio to convert staked balance to block period.
       * This is used to determine the period in which a Provider should submit a proof, based on
       * their stake. The period is calculated as `StakeToChallengePeriod / stake`, saturating at [`Config::MinChallengePeriod`].
       **/
      stakeToChallengePeriod: u128 & AugmentedConst<ApiType>;
      /**
       * The target number of ticks for which to store the submitters that submitted valid proofs in them,
       * stored in the `ValidProofSubmittersLastTicks` StorageMap. That storage will be trimmed down to this number
       * of ticks in the `on_idle` hook of this pallet, to avoid bloating the state.
       **/
      targetTicksStorageOfSubmitters: u32 & AugmentedConst<ApiType>;
      /**
       * The Treasury AccountId.
       * The account to which:
       * - The fees for submitting a challenge are transferred.
       * - The slashed funds are transferred.
       **/
      treasury: AccountId32 & AugmentedConst<ApiType>;
      /**
       * Generic const
       **/
      [key: string]: Codec;
    };
    providers: {
      /**
       * The amount of blocks that a BSP must wait before being able to sign off, after being signed up.
       *
       * This is to prevent BSPs from signing up and off too quickly, thus making it harder for an attacker
       * to suddenly have a large portion of the total number of BSPs. The reason for this, is that the
       * attacker would have to lock up a large amount of funds for this period of time.
       **/
      bspSignUpLockPeriod: u32 & AugmentedConst<ApiType>;
      /**
       * The amount that an account has to deposit to create a bucket.
       **/
      bucketDeposit: u128 & AugmentedConst<ApiType>;
      /**
       * Type that represents the byte limit of a bucket name.
       **/
      bucketNameLimit: u32 & AugmentedConst<ApiType>;
      /**
       * The default value of the root of the Merkle Patricia Trie of the runtime
       **/
      defaultMerkleRoot: H256 & AugmentedConst<ApiType>;
      /**
       * The slope of the collateral vs storage capacity curve. In other terms, how many tokens a Storage Provider should add as collateral to increase its storage capacity in one unit of StorageDataUnit.
       **/
      depositPerData: u128 & AugmentedConst<ApiType>;
      /**
       * The maximum amount of blocks after which a sign up request expires so the randomness cannot be chosen
       **/
      maxBlocksForRandomness: u32 & AugmentedConst<ApiType>;
      maxCommitmentSize: u32 & AugmentedConst<ApiType>;
      /**
       * Maximum number of expired items (per type) to clean up in a single block.
       **/
      maxExpiredItemsInBlock: u32 & AugmentedConst<ApiType>;
      /**
       * The estimated maximum size of an unknown file.
       *
       * Used primarily to slash a Storage Provider when it fails to provide a chunk of data for an unknown file size.
       **/
      maxFileSize: u64 & AugmentedConst<ApiType>;
      /**
       * The maximum amount of multiaddresses that a Storage Provider can have.
       **/
      maxMultiAddressAmount: u32 & AugmentedConst<ApiType>;
      /**
       * The maximum size of a multiaddress.
       **/
      maxMultiAddressSize: u32 & AugmentedConst<ApiType>;
      /**
       * The maximum number of protocols the MSP can support (at least within the runtime).
       **/
      maxProtocols: u32 & AugmentedConst<ApiType>;
      /**
       * The minimum amount of blocks between capacity changes for a SP
       **/
      minBlocksBetweenCapacityChanges: u32 & AugmentedConst<ApiType>;
      /**
       * Time-to-live for a provider to top up their deposit to cover a capacity deficit.
       *
       * This TTL is used to determine at what point to insert the expiration item in the
       * [`ProviderTopUpExpirations`] storage which is processed in the `on_idle` hook at
       * the time when the tick has been reached.
       **/
      providerTopUpTtl: u32 & AugmentedConst<ApiType>;
      /**
       * The slash factor deducted from a Storage Provider's deposit for every single storage proof they fail to provide.
       **/
      slashAmountPerMaxFileSize: u128 & AugmentedConst<ApiType>;
      /**
       * The amount that a BSP receives as allocation of storage capacity when it deposits SpMinDeposit.
       **/
      spMinCapacity: u64 & AugmentedConst<ApiType>;
      /**
       * The minimum amount that an account has to deposit to become a storage provider.
       **/
      spMinDeposit: u128 & AugmentedConst<ApiType>;
      /**
       * Starting reputation weight for a newly registered BSP.
       **/
      startingReputationWeight: u32 & AugmentedConst<ApiType>;
      /**
       * The Treasury AccountId.
       * The account to which:
       * - The fees for submitting a challenge are transferred.
       * - The slashed funds are transferred.
       **/
      treasury: AccountId32 & AugmentedConst<ApiType>;
      /**
       * 0-size bucket fixed rate payment stream (i.e. the amount charged as a base
       * fee for a bucket that doesn't have any files yet)
       **/
      zeroSizeBucketFixedRate: u128 & AugmentedConst<ApiType>;
      /**
       * Generic const
       **/
      [key: string]: Codec;
    };
    system: {
      /**
       * Maximum number of block number to block hash mappings to keep (oldest pruned first).
       **/
      blockHashCount: u32 & AugmentedConst<ApiType>;
      /**
       * The maximum length of a block (in bytes).
       **/
      blockLength: FrameSystemLimitsBlockLength & AugmentedConst<ApiType>;
      /**
       * Block & extrinsics weights: base values and limits.
       **/
      blockWeights: FrameSystemLimitsBlockWeights & AugmentedConst<ApiType>;
      /**
       * The weight of runtime database operations the runtime can invoke.
       **/
      dbWeight: SpWeightsRuntimeDbWeight & AugmentedConst<ApiType>;
      /**
       * The designated SS58 prefix of this chain.
       *
       * This replaces the "ss58Format" property declared in the chain spec. Reason is
       * that the runtime should know about the prefix in order to make use of it as
       * an identifier of the chain.
       **/
      ss58Prefix: u16 & AugmentedConst<ApiType>;
      /**
       * Get the chain's in-code version.
       **/
      version: SpVersionRuntimeVersion & AugmentedConst<ApiType>;
      /**
       * Generic const
       **/
      [key: string]: Codec;
    };
    timestamp: {
      /**
       * The minimum period between blocks.
       *
       * Be aware that this is different to the *expected* period that the block production
       * apparatus provides. Your chosen consensus system will generally work with this to
       * determine a sensible block time. For example, in the Aura pallet it will be double this
       * period on default settings.
       **/
      minimumPeriod: u64 & AugmentedConst<ApiType>;
      /**
       * Generic const
       **/
      [key: string]: Codec;
    };
    transactionPayment: {
      /**
       * A fee multiplier for `Operational` extrinsics to compute "virtual tip" to boost their
       * `priority`
       *
       * This value is multiplied by the `final_fee` to obtain a "virtual tip" that is later
       * added to a tip component in regular `priority` calculations.
       * It means that a `Normal` transaction can front-run a similarly-sized `Operational`
       * extrinsic (with no tip), by including a tip value greater than the virtual tip.
       *
       * ```rust,ignore
       * // For `Normal`
       * let priority = priority_calc(tip);
       *
       * // For `Operational`
       * let virtual_tip = (inclusion_fee + tip) * OperationalFeeMultiplier;
       * let priority = priority_calc(tip + virtual_tip);
       * ```
       *
       * Note that since we use `final_fee` the multiplier applies also to the regular `tip`
       * sent with the transaction. So, not only does the transaction get a priority bump based
       * on the `inclusion_fee`, but we also amplify the impact of tips applied to `Operational`
       * transactions.
       **/
      operationalFeeMultiplier: u8 & AugmentedConst<ApiType>;
      /**
       * Generic const
       **/
      [key: string]: Codec;
    };
    xcmpQueue: {
      /**
       * Maximal number of outbound XCMP channels that can have messages queued at the same time.
       *
       * If this is reached, then no further messages can be sent to channels that do not yet
       * have a message queued. This should be set to the expected maximum of outbound channels
       * which is determined by [`Self::ChannelInfo`]. It is important to set this large enough,
       * since otherwise the congestion control protocol will not work as intended and messages
       * may be dropped. This value increases the PoV and should therefore not be picked too
       * high. Governance needs to pay attention to not open more channels than this value.
       **/
      maxActiveOutboundChannels: u32 & AugmentedConst<ApiType>;
      /**
       * The maximum number of inbound XCMP channels that can be suspended simultaneously.
       *
       * Any further channel suspensions will fail and messages may get dropped without further
       * notice. Choosing a high value (1000) is okay; the trade-off that is described in
       * [`InboundXcmpSuspended`] still applies at that scale.
       **/
      maxInboundSuspended: u32 & AugmentedConst<ApiType>;
      /**
       * The maximal page size for HRMP message pages.
       *
       * A lower limit can be set dynamically, but this is the hard-limit for the PoV worst case
       * benchmarking. The limit for the size of a message is slightly below this, since some
       * overhead is incurred for encoding the format.
       **/
      maxPageSize: u32 & AugmentedConst<ApiType>;
      /**
       * Generic const
       **/
      [key: string]: Codec;
    };
  }
}
