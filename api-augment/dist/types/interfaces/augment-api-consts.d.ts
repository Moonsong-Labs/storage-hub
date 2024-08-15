import "@polkadot/api-base/types/consts";
import type { ApiTypes, AugmentedConst } from "@polkadot/api-base/types";
import type { Option, u128, u16, u32, u64, u8 } from "@polkadot/types-codec";
import type { Codec } from "@polkadot/types-codec/types";
import type { AccountId32, H256 } from "@polkadot/types/interfaces/runtime";
import type { FrameSystemLimitsBlockLength, FrameSystemLimitsBlockWeights, SpVersionRuntimeVersion, SpWeightsRuntimeDbWeight, SpWeightsWeightV2Weight } from "@polkadot/types/lookup";
export type __AugmentedConst<ApiType extends ApiTypes> = AugmentedConst<ApiType>;
declare module "@polkadot/api-base/types/consts" {
    interface AugmentedConsts<ApiType extends ApiTypes> {
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
             **/
            maxLocks: u32 & AugmentedConst<ApiType>;
            /**
             * The maximum number of named reserves that can exist on an account.
             **/
            maxReserves: u32 & AugmentedConst<ApiType>;
            /**
             * Generic const
             **/
            [key: string]: Codec;
        };
        fileSystem: {
            /**
             * Horizontal asymptote which the volunteering threshold approaches as more BSPs are registered in the system.
             **/
            assignmentThresholdAsymptote: u128 & AugmentedConst<ApiType>;
            /**
             * Asymptotic decay function for the assignment threshold.
             **/
            assignmentThresholdDecayFactor: u128 & AugmentedConst<ApiType>;
            /**
             * The multiplier increases the threshold over time (blocks) which increases the
             * likelihood of a BSP successfully volunteering to store a file.
             **/
            assignmentThresholdMultiplier: u128 & AugmentedConst<ApiType>;
            /**
             * Maximum batch of storage requests that can be confirmed at once when calling `bsp_confirm_storing`.
             **/
            maxBatchConfirmStorageRequests: u32 & AugmentedConst<ApiType>;
            /**
             * Maximum number of BSPs that can store a file.
             *
             * This is used to limit the number of BSPs storing a file and claiming rewards for it.
             * If this number is too high, then the reward for storing a file might be to diluted and pointless to store.
             **/
            maxBspsPerStorageRequest: u32 & AugmentedConst<ApiType>;
            /**
             * Maximum number of multiaddresses for a storage request.
             **/
            maxDataServerMultiAddresses: u32 & AugmentedConst<ApiType>;
            /**
             * Maximum number of expired storage requests to clean up in a single block.
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
             * Time-to-live for a storage request.
             **/
            storageRequestTtl: u32 & AugmentedConst<ApiType>;
            /**
             * Minimum number of BSPs required to store a file.
             *
             * This is also used as a default value if the BSPs required are not specified when creating a storage request.
             **/
            targetBspsRequired: u32 & AugmentedConst<ApiType>;
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
             * The maximum number of stale pages (i.e. of overweight messages) allowed before culling
             * can happen. Once there are more stale pages than this, then historical pages may be
             * dropped, even if they contain unprocessed overweight messages.
             **/
            maxStale: u32 & AugmentedConst<ApiType>;
            /**
             * The amount of weight (if any) which should be provided to the message queue for
             * servicing enqueued items.
             *
             * This may be legitimately `None` in the case that you will call
             * `ServiceQueues::service_queues` manually.
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
             * The number of ticks that correspond to the deposit that a User has to pay to open a payment stream.
             * This means that, from the balance of the User for which the payment stream is being created, the amount
             * `NewStreamDeposit * rate` will be held as a deposit.
             * In the case of dynamic-rate payment streams, `rate` will be `amount_provided * current_service_price`, where `current_service_price` has
             * to be provided by the pallet using the `PaymentStreamsInterface` interface.
             **/
            newStreamDeposit: u32 & AugmentedConst<ApiType>;
            /**
             * Generic const
             **/
            [key: string]: Codec;
        };
        proofsDealer: {
            /**
             * The number of ticks that challenges history is kept for.
             * After this many ticks, challenges are removed from `TickToChallengesSeed` StorageMap.
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
             * The maximum amount of Providers that can submit a proof in a single block.
             * Although this can be seen as an arbitrary limit, if set to the already existing
             * implicit limit that is "how many `submit_proof` extrinsics fit in the weight of
             * a block, this wouldn't add any additional artificial limit.
             **/
            maxSubmittersPerTick: u32 & AugmentedConst<ApiType>;
            /**
             * The number of random challenges that are generated per block, using the random seed
             * generated for that block.
             **/
            randomChallengesPerBlock: u32 & AugmentedConst<ApiType>;
            /**
             * The ratio to convert staked balance to block period.
             * This is used to determine the period in which a Provider should submit a proof, based on
             * their stake. The period is calculated as `stake / StakeToBlockPeriod`, saturating at 1.
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
             * The slope of the collateral vs storage capacity curve. In other terms, how many tokens a Storage Provider should add as collateral to increase its storage capacity in one unit of StorageData.
             **/
            depositPerData: u128 & AugmentedConst<ApiType>;
            /**
             * The maximum amount of blocks after which a sign up request expires so the randomness cannot be chosen
             **/
            maxBlocksForRandomness: u32 & AugmentedConst<ApiType>;
            /**
             * The maximum amount of Buckets that a MSP can have.
             **/
            maxBuckets: u32 & AugmentedConst<ApiType>;
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
             * The slash factor deducted from a Storage Provider's deposit for every single storage proof they fail to provide.
             **/
            slashFactor: u128 & AugmentedConst<ApiType>;
            /**
             * The amount that a BSP receives as allocation of storage capacity when it deposits SpMinDeposit.
             **/
            spMinCapacity: u32 & AugmentedConst<ApiType>;
            /**
             * The minimum amount that an account has to deposit to become a storage provider.
             **/
            spMinDeposit: u128 & AugmentedConst<ApiType>;
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
             * The maximum number of inbound XCMP channels that can be suspended simultaneously.
             *
             * Any further channel suspensions will fail and messages may get dropped without further
             * notice. Choosing a high value (1000) is okay; the trade-off that is described in
             * [`InboundXcmpSuspended`] still applies at that scale.
             **/
            maxInboundSuspended: u32 & AugmentedConst<ApiType>;
            /**
             * Generic const
             **/
            [key: string]: Codec;
        };
    }
}
