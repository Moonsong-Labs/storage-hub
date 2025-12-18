import '@polkadot/types/lookup';
import type { BTreeMap, BTreeSet, Bytes, Compact, Enum, Null, Option, Result, Struct, Text, U8aFixed, Vec, bool, u128, u16, u32, u64, u8 } from '@polkadot/types-codec';
import type { ITuple } from '@polkadot/types-codec/types';
import type { AccountId32, Call, H256, MultiAddress, Perbill } from '@polkadot/types/interfaces/runtime';
import type { Event } from '@polkadot/types/interfaces/system';
declare module '@polkadot/types/lookup' {
    /** @name FrameSystemAccountInfo (3) */
    interface FrameSystemAccountInfo extends Struct {
        readonly nonce: u32;
        readonly consumers: u32;
        readonly providers: u32;
        readonly sufficients: u32;
        readonly data: PalletBalancesAccountData;
    }
    /** @name PalletBalancesAccountData (5) */
    interface PalletBalancesAccountData extends Struct {
        readonly free: u128;
        readonly reserved: u128;
        readonly frozen: u128;
        readonly flags: u128;
    }
    /** @name FrameSupportDispatchPerDispatchClassWeight (9) */
    interface FrameSupportDispatchPerDispatchClassWeight extends Struct {
        readonly normal: SpWeightsWeightV2Weight;
        readonly operational: SpWeightsWeightV2Weight;
        readonly mandatory: SpWeightsWeightV2Weight;
    }
    /** @name SpWeightsWeightV2Weight (10) */
    interface SpWeightsWeightV2Weight extends Struct {
        readonly refTime: Compact<u64>;
        readonly proofSize: Compact<u64>;
    }
    /** @name SpRuntimeDigest (15) */
    interface SpRuntimeDigest extends Struct {
        readonly logs: Vec<SpRuntimeDigestDigestItem>;
    }
    /** @name SpRuntimeDigestDigestItem (17) */
    interface SpRuntimeDigestDigestItem extends Enum {
        readonly isOther: boolean;
        readonly asOther: Bytes;
        readonly isConsensus: boolean;
        readonly asConsensus: ITuple<[U8aFixed, Bytes]>;
        readonly isSeal: boolean;
        readonly asSeal: ITuple<[U8aFixed, Bytes]>;
        readonly isPreRuntime: boolean;
        readonly asPreRuntime: ITuple<[U8aFixed, Bytes]>;
        readonly isRuntimeEnvironmentUpdated: boolean;
        readonly type: 'Other' | 'Consensus' | 'Seal' | 'PreRuntime' | 'RuntimeEnvironmentUpdated';
    }
    /** @name FrameSystemEventRecord (20) */
    interface FrameSystemEventRecord extends Struct {
        readonly phase: FrameSystemPhase;
        readonly event: Event;
        readonly topics: Vec<H256>;
    }
    /** @name FrameSystemEvent (22) */
    interface FrameSystemEvent extends Enum {
        readonly isExtrinsicSuccess: boolean;
        readonly asExtrinsicSuccess: {
            readonly dispatchInfo: FrameSystemDispatchEventInfo;
        } & Struct;
        readonly isExtrinsicFailed: boolean;
        readonly asExtrinsicFailed: {
            readonly dispatchError: SpRuntimeDispatchError;
            readonly dispatchInfo: FrameSystemDispatchEventInfo;
        } & Struct;
        readonly isCodeUpdated: boolean;
        readonly isNewAccount: boolean;
        readonly asNewAccount: {
            readonly account: AccountId32;
        } & Struct;
        readonly isKilledAccount: boolean;
        readonly asKilledAccount: {
            readonly account: AccountId32;
        } & Struct;
        readonly isRemarked: boolean;
        readonly asRemarked: {
            readonly sender: AccountId32;
            readonly hash_: H256;
        } & Struct;
        readonly isUpgradeAuthorized: boolean;
        readonly asUpgradeAuthorized: {
            readonly codeHash: H256;
            readonly checkVersion: bool;
        } & Struct;
        readonly type: 'ExtrinsicSuccess' | 'ExtrinsicFailed' | 'CodeUpdated' | 'NewAccount' | 'KilledAccount' | 'Remarked' | 'UpgradeAuthorized';
    }
    /** @name FrameSystemDispatchEventInfo (23) */
    interface FrameSystemDispatchEventInfo extends Struct {
        readonly weight: SpWeightsWeightV2Weight;
        readonly class: FrameSupportDispatchDispatchClass;
        readonly paysFee: FrameSupportDispatchPays;
    }
    /** @name FrameSupportDispatchDispatchClass (24) */
    interface FrameSupportDispatchDispatchClass extends Enum {
        readonly isNormal: boolean;
        readonly isOperational: boolean;
        readonly isMandatory: boolean;
        readonly type: 'Normal' | 'Operational' | 'Mandatory';
    }
    /** @name FrameSupportDispatchPays (25) */
    interface FrameSupportDispatchPays extends Enum {
        readonly isYes: boolean;
        readonly isNo: boolean;
        readonly type: 'Yes' | 'No';
    }
    /** @name SpRuntimeDispatchError (26) */
    interface SpRuntimeDispatchError extends Enum {
        readonly isOther: boolean;
        readonly isCannotLookup: boolean;
        readonly isBadOrigin: boolean;
        readonly isModule: boolean;
        readonly asModule: SpRuntimeModuleError;
        readonly isConsumerRemaining: boolean;
        readonly isNoProviders: boolean;
        readonly isTooManyConsumers: boolean;
        readonly isToken: boolean;
        readonly asToken: SpRuntimeTokenError;
        readonly isArithmetic: boolean;
        readonly asArithmetic: SpArithmeticArithmeticError;
        readonly isTransactional: boolean;
        readonly asTransactional: SpRuntimeTransactionalError;
        readonly isExhausted: boolean;
        readonly isCorruption: boolean;
        readonly isUnavailable: boolean;
        readonly isRootNotAllowed: boolean;
        readonly isTrie: boolean;
        readonly asTrie: SpRuntimeProvingTrieTrieError;
        readonly type: 'Other' | 'CannotLookup' | 'BadOrigin' | 'Module' | 'ConsumerRemaining' | 'NoProviders' | 'TooManyConsumers' | 'Token' | 'Arithmetic' | 'Transactional' | 'Exhausted' | 'Corruption' | 'Unavailable' | 'RootNotAllowed' | 'Trie';
    }
    /** @name SpRuntimeModuleError (27) */
    interface SpRuntimeModuleError extends Struct {
        readonly index: u8;
        readonly error: U8aFixed;
    }
    /** @name SpRuntimeTokenError (28) */
    interface SpRuntimeTokenError extends Enum {
        readonly isFundsUnavailable: boolean;
        readonly isOnlyProvider: boolean;
        readonly isBelowMinimum: boolean;
        readonly isCannotCreate: boolean;
        readonly isUnknownAsset: boolean;
        readonly isFrozen: boolean;
        readonly isUnsupported: boolean;
        readonly isCannotCreateHold: boolean;
        readonly isNotExpendable: boolean;
        readonly isBlocked: boolean;
        readonly type: 'FundsUnavailable' | 'OnlyProvider' | 'BelowMinimum' | 'CannotCreate' | 'UnknownAsset' | 'Frozen' | 'Unsupported' | 'CannotCreateHold' | 'NotExpendable' | 'Blocked';
    }
    /** @name SpArithmeticArithmeticError (29) */
    interface SpArithmeticArithmeticError extends Enum {
        readonly isUnderflow: boolean;
        readonly isOverflow: boolean;
        readonly isDivisionByZero: boolean;
        readonly type: 'Underflow' | 'Overflow' | 'DivisionByZero';
    }
    /** @name SpRuntimeTransactionalError (30) */
    interface SpRuntimeTransactionalError extends Enum {
        readonly isLimitReached: boolean;
        readonly isNoLayer: boolean;
        readonly type: 'LimitReached' | 'NoLayer';
    }
    /** @name SpRuntimeProvingTrieTrieError (31) */
    interface SpRuntimeProvingTrieTrieError extends Enum {
        readonly isInvalidStateRoot: boolean;
        readonly isIncompleteDatabase: boolean;
        readonly isValueAtIncompleteKey: boolean;
        readonly isDecoderError: boolean;
        readonly isInvalidHash: boolean;
        readonly isDuplicateKey: boolean;
        readonly isExtraneousNode: boolean;
        readonly isExtraneousValue: boolean;
        readonly isExtraneousHashReference: boolean;
        readonly isInvalidChildReference: boolean;
        readonly isValueMismatch: boolean;
        readonly isIncompleteProof: boolean;
        readonly isRootMismatch: boolean;
        readonly isDecodeError: boolean;
        readonly type: 'InvalidStateRoot' | 'IncompleteDatabase' | 'ValueAtIncompleteKey' | 'DecoderError' | 'InvalidHash' | 'DuplicateKey' | 'ExtraneousNode' | 'ExtraneousValue' | 'ExtraneousHashReference' | 'InvalidChildReference' | 'ValueMismatch' | 'IncompleteProof' | 'RootMismatch' | 'DecodeError';
    }
    /** @name CumulusPalletParachainSystemEvent (32) */
    interface CumulusPalletParachainSystemEvent extends Enum {
        readonly isValidationFunctionStored: boolean;
        readonly isValidationFunctionApplied: boolean;
        readonly asValidationFunctionApplied: {
            readonly relayChainBlockNum: u32;
        } & Struct;
        readonly isValidationFunctionDiscarded: boolean;
        readonly isDownwardMessagesReceived: boolean;
        readonly asDownwardMessagesReceived: {
            readonly count: u32;
        } & Struct;
        readonly isDownwardMessagesProcessed: boolean;
        readonly asDownwardMessagesProcessed: {
            readonly weightUsed: SpWeightsWeightV2Weight;
            readonly dmqHead: H256;
        } & Struct;
        readonly isUpwardMessageSent: boolean;
        readonly asUpwardMessageSent: {
            readonly messageHash: Option<U8aFixed>;
        } & Struct;
        readonly type: 'ValidationFunctionStored' | 'ValidationFunctionApplied' | 'ValidationFunctionDiscarded' | 'DownwardMessagesReceived' | 'DownwardMessagesProcessed' | 'UpwardMessageSent';
    }
    /** @name PalletBalancesEvent (34) */
    interface PalletBalancesEvent extends Enum {
        readonly isEndowed: boolean;
        readonly asEndowed: {
            readonly account: AccountId32;
            readonly freeBalance: u128;
        } & Struct;
        readonly isDustLost: boolean;
        readonly asDustLost: {
            readonly account: AccountId32;
            readonly amount: u128;
        } & Struct;
        readonly isTransfer: boolean;
        readonly asTransfer: {
            readonly from: AccountId32;
            readonly to: AccountId32;
            readonly amount: u128;
        } & Struct;
        readonly isBalanceSet: boolean;
        readonly asBalanceSet: {
            readonly who: AccountId32;
            readonly free: u128;
        } & Struct;
        readonly isReserved: boolean;
        readonly asReserved: {
            readonly who: AccountId32;
            readonly amount: u128;
        } & Struct;
        readonly isUnreserved: boolean;
        readonly asUnreserved: {
            readonly who: AccountId32;
            readonly amount: u128;
        } & Struct;
        readonly isReserveRepatriated: boolean;
        readonly asReserveRepatriated: {
            readonly from: AccountId32;
            readonly to: AccountId32;
            readonly amount: u128;
            readonly destinationStatus: FrameSupportTokensMiscBalanceStatus;
        } & Struct;
        readonly isDeposit: boolean;
        readonly asDeposit: {
            readonly who: AccountId32;
            readonly amount: u128;
        } & Struct;
        readonly isWithdraw: boolean;
        readonly asWithdraw: {
            readonly who: AccountId32;
            readonly amount: u128;
        } & Struct;
        readonly isSlashed: boolean;
        readonly asSlashed: {
            readonly who: AccountId32;
            readonly amount: u128;
        } & Struct;
        readonly isMinted: boolean;
        readonly asMinted: {
            readonly who: AccountId32;
            readonly amount: u128;
        } & Struct;
        readonly isBurned: boolean;
        readonly asBurned: {
            readonly who: AccountId32;
            readonly amount: u128;
        } & Struct;
        readonly isSuspended: boolean;
        readonly asSuspended: {
            readonly who: AccountId32;
            readonly amount: u128;
        } & Struct;
        readonly isRestored: boolean;
        readonly asRestored: {
            readonly who: AccountId32;
            readonly amount: u128;
        } & Struct;
        readonly isUpgraded: boolean;
        readonly asUpgraded: {
            readonly who: AccountId32;
        } & Struct;
        readonly isIssued: boolean;
        readonly asIssued: {
            readonly amount: u128;
        } & Struct;
        readonly isRescinded: boolean;
        readonly asRescinded: {
            readonly amount: u128;
        } & Struct;
        readonly isLocked: boolean;
        readonly asLocked: {
            readonly who: AccountId32;
            readonly amount: u128;
        } & Struct;
        readonly isUnlocked: boolean;
        readonly asUnlocked: {
            readonly who: AccountId32;
            readonly amount: u128;
        } & Struct;
        readonly isFrozen: boolean;
        readonly asFrozen: {
            readonly who: AccountId32;
            readonly amount: u128;
        } & Struct;
        readonly isThawed: boolean;
        readonly asThawed: {
            readonly who: AccountId32;
            readonly amount: u128;
        } & Struct;
        readonly isTotalIssuanceForced: boolean;
        readonly asTotalIssuanceForced: {
            readonly old: u128;
            readonly new_: u128;
        } & Struct;
        readonly type: 'Endowed' | 'DustLost' | 'Transfer' | 'BalanceSet' | 'Reserved' | 'Unreserved' | 'ReserveRepatriated' | 'Deposit' | 'Withdraw' | 'Slashed' | 'Minted' | 'Burned' | 'Suspended' | 'Restored' | 'Upgraded' | 'Issued' | 'Rescinded' | 'Locked' | 'Unlocked' | 'Frozen' | 'Thawed' | 'TotalIssuanceForced';
    }
    /** @name FrameSupportTokensMiscBalanceStatus (35) */
    interface FrameSupportTokensMiscBalanceStatus extends Enum {
        readonly isFree: boolean;
        readonly isReserved: boolean;
        readonly type: 'Free' | 'Reserved';
    }
    /** @name PalletTransactionPaymentEvent (36) */
    interface PalletTransactionPaymentEvent extends Enum {
        readonly isTransactionFeePaid: boolean;
        readonly asTransactionFeePaid: {
            readonly who: AccountId32;
            readonly actualFee: u128;
            readonly tip: u128;
        } & Struct;
        readonly type: 'TransactionFeePaid';
    }
    /** @name PalletSudoEvent (37) */
    interface PalletSudoEvent extends Enum {
        readonly isSudid: boolean;
        readonly asSudid: {
            readonly sudoResult: Result<Null, SpRuntimeDispatchError>;
        } & Struct;
        readonly isKeyChanged: boolean;
        readonly asKeyChanged: {
            readonly old: Option<AccountId32>;
            readonly new_: AccountId32;
        } & Struct;
        readonly isKeyRemoved: boolean;
        readonly isSudoAsDone: boolean;
        readonly asSudoAsDone: {
            readonly sudoResult: Result<Null, SpRuntimeDispatchError>;
        } & Struct;
        readonly type: 'Sudid' | 'KeyChanged' | 'KeyRemoved' | 'SudoAsDone';
    }
    /** @name PalletCollatorSelectionEvent (41) */
    interface PalletCollatorSelectionEvent extends Enum {
        readonly isNewInvulnerables: boolean;
        readonly asNewInvulnerables: {
            readonly invulnerables: Vec<AccountId32>;
        } & Struct;
        readonly isInvulnerableAdded: boolean;
        readonly asInvulnerableAdded: {
            readonly accountId: AccountId32;
        } & Struct;
        readonly isInvulnerableRemoved: boolean;
        readonly asInvulnerableRemoved: {
            readonly accountId: AccountId32;
        } & Struct;
        readonly isNewDesiredCandidates: boolean;
        readonly asNewDesiredCandidates: {
            readonly desiredCandidates: u32;
        } & Struct;
        readonly isNewCandidacyBond: boolean;
        readonly asNewCandidacyBond: {
            readonly bondAmount: u128;
        } & Struct;
        readonly isCandidateAdded: boolean;
        readonly asCandidateAdded: {
            readonly accountId: AccountId32;
            readonly deposit: u128;
        } & Struct;
        readonly isCandidateBondUpdated: boolean;
        readonly asCandidateBondUpdated: {
            readonly accountId: AccountId32;
            readonly deposit: u128;
        } & Struct;
        readonly isCandidateRemoved: boolean;
        readonly asCandidateRemoved: {
            readonly accountId: AccountId32;
        } & Struct;
        readonly isCandidateReplaced: boolean;
        readonly asCandidateReplaced: {
            readonly old: AccountId32;
            readonly new_: AccountId32;
            readonly deposit: u128;
        } & Struct;
        readonly isInvalidInvulnerableSkipped: boolean;
        readonly asInvalidInvulnerableSkipped: {
            readonly accountId: AccountId32;
        } & Struct;
        readonly type: 'NewInvulnerables' | 'InvulnerableAdded' | 'InvulnerableRemoved' | 'NewDesiredCandidates' | 'NewCandidacyBond' | 'CandidateAdded' | 'CandidateBondUpdated' | 'CandidateRemoved' | 'CandidateReplaced' | 'InvalidInvulnerableSkipped';
    }
    /** @name PalletSessionEvent (43) */
    interface PalletSessionEvent extends Enum {
        readonly isNewSession: boolean;
        readonly asNewSession: {
            readonly sessionIndex: u32;
        } & Struct;
        readonly type: 'NewSession';
    }
    /** @name CumulusPalletXcmpQueueEvent (44) */
    interface CumulusPalletXcmpQueueEvent extends Enum {
        readonly isXcmpMessageSent: boolean;
        readonly asXcmpMessageSent: {
            readonly messageHash: U8aFixed;
        } & Struct;
        readonly type: 'XcmpMessageSent';
    }
    /** @name PalletXcmEvent (45) */
    interface PalletXcmEvent extends Enum {
        readonly isAttempted: boolean;
        readonly asAttempted: {
            readonly outcome: StagingXcmV5TraitsOutcome;
        } & Struct;
        readonly isSent: boolean;
        readonly asSent: {
            readonly origin: StagingXcmV5Location;
            readonly destination: StagingXcmV5Location;
            readonly message: StagingXcmV5Xcm;
            readonly messageId: U8aFixed;
        } & Struct;
        readonly isUnexpectedResponse: boolean;
        readonly asUnexpectedResponse: {
            readonly origin: StagingXcmV5Location;
            readonly queryId: u64;
        } & Struct;
        readonly isResponseReady: boolean;
        readonly asResponseReady: {
            readonly queryId: u64;
            readonly response: StagingXcmV5Response;
        } & Struct;
        readonly isNotified: boolean;
        readonly asNotified: {
            readonly queryId: u64;
            readonly palletIndex: u8;
            readonly callIndex: u8;
        } & Struct;
        readonly isNotifyOverweight: boolean;
        readonly asNotifyOverweight: {
            readonly queryId: u64;
            readonly palletIndex: u8;
            readonly callIndex: u8;
            readonly actualWeight: SpWeightsWeightV2Weight;
            readonly maxBudgetedWeight: SpWeightsWeightV2Weight;
        } & Struct;
        readonly isNotifyDispatchError: boolean;
        readonly asNotifyDispatchError: {
            readonly queryId: u64;
            readonly palletIndex: u8;
            readonly callIndex: u8;
        } & Struct;
        readonly isNotifyDecodeFailed: boolean;
        readonly asNotifyDecodeFailed: {
            readonly queryId: u64;
            readonly palletIndex: u8;
            readonly callIndex: u8;
        } & Struct;
        readonly isInvalidResponder: boolean;
        readonly asInvalidResponder: {
            readonly origin: StagingXcmV5Location;
            readonly queryId: u64;
            readonly expectedLocation: Option<StagingXcmV5Location>;
        } & Struct;
        readonly isInvalidResponderVersion: boolean;
        readonly asInvalidResponderVersion: {
            readonly origin: StagingXcmV5Location;
            readonly queryId: u64;
        } & Struct;
        readonly isResponseTaken: boolean;
        readonly asResponseTaken: {
            readonly queryId: u64;
        } & Struct;
        readonly isAssetsTrapped: boolean;
        readonly asAssetsTrapped: {
            readonly hash_: H256;
            readonly origin: StagingXcmV5Location;
            readonly assets: XcmVersionedAssets;
        } & Struct;
        readonly isVersionChangeNotified: boolean;
        readonly asVersionChangeNotified: {
            readonly destination: StagingXcmV5Location;
            readonly result: u32;
            readonly cost: StagingXcmV5AssetAssets;
            readonly messageId: U8aFixed;
        } & Struct;
        readonly isSupportedVersionChanged: boolean;
        readonly asSupportedVersionChanged: {
            readonly location: StagingXcmV5Location;
            readonly version: u32;
        } & Struct;
        readonly isNotifyTargetSendFail: boolean;
        readonly asNotifyTargetSendFail: {
            readonly location: StagingXcmV5Location;
            readonly queryId: u64;
            readonly error: XcmV5TraitsError;
        } & Struct;
        readonly isNotifyTargetMigrationFail: boolean;
        readonly asNotifyTargetMigrationFail: {
            readonly location: XcmVersionedLocation;
            readonly queryId: u64;
        } & Struct;
        readonly isInvalidQuerierVersion: boolean;
        readonly asInvalidQuerierVersion: {
            readonly origin: StagingXcmV5Location;
            readonly queryId: u64;
        } & Struct;
        readonly isInvalidQuerier: boolean;
        readonly asInvalidQuerier: {
            readonly origin: StagingXcmV5Location;
            readonly queryId: u64;
            readonly expectedQuerier: StagingXcmV5Location;
            readonly maybeActualQuerier: Option<StagingXcmV5Location>;
        } & Struct;
        readonly isVersionNotifyStarted: boolean;
        readonly asVersionNotifyStarted: {
            readonly destination: StagingXcmV5Location;
            readonly cost: StagingXcmV5AssetAssets;
            readonly messageId: U8aFixed;
        } & Struct;
        readonly isVersionNotifyRequested: boolean;
        readonly asVersionNotifyRequested: {
            readonly destination: StagingXcmV5Location;
            readonly cost: StagingXcmV5AssetAssets;
            readonly messageId: U8aFixed;
        } & Struct;
        readonly isVersionNotifyUnrequested: boolean;
        readonly asVersionNotifyUnrequested: {
            readonly destination: StagingXcmV5Location;
            readonly cost: StagingXcmV5AssetAssets;
            readonly messageId: U8aFixed;
        } & Struct;
        readonly isFeesPaid: boolean;
        readonly asFeesPaid: {
            readonly paying: StagingXcmV5Location;
            readonly fees: StagingXcmV5AssetAssets;
        } & Struct;
        readonly isAssetsClaimed: boolean;
        readonly asAssetsClaimed: {
            readonly hash_: H256;
            readonly origin: StagingXcmV5Location;
            readonly assets: XcmVersionedAssets;
        } & Struct;
        readonly isVersionMigrationFinished: boolean;
        readonly asVersionMigrationFinished: {
            readonly version: u32;
        } & Struct;
        readonly type: 'Attempted' | 'Sent' | 'UnexpectedResponse' | 'ResponseReady' | 'Notified' | 'NotifyOverweight' | 'NotifyDispatchError' | 'NotifyDecodeFailed' | 'InvalidResponder' | 'InvalidResponderVersion' | 'ResponseTaken' | 'AssetsTrapped' | 'VersionChangeNotified' | 'SupportedVersionChanged' | 'NotifyTargetSendFail' | 'NotifyTargetMigrationFail' | 'InvalidQuerierVersion' | 'InvalidQuerier' | 'VersionNotifyStarted' | 'VersionNotifyRequested' | 'VersionNotifyUnrequested' | 'FeesPaid' | 'AssetsClaimed' | 'VersionMigrationFinished';
    }
    /** @name StagingXcmV5TraitsOutcome (46) */
    interface StagingXcmV5TraitsOutcome extends Enum {
        readonly isComplete: boolean;
        readonly asComplete: {
            readonly used: SpWeightsWeightV2Weight;
        } & Struct;
        readonly isIncomplete: boolean;
        readonly asIncomplete: {
            readonly used: SpWeightsWeightV2Weight;
            readonly error: XcmV5TraitsError;
        } & Struct;
        readonly isError: boolean;
        readonly asError: {
            readonly error: XcmV5TraitsError;
        } & Struct;
        readonly type: 'Complete' | 'Incomplete' | 'Error';
    }
    /** @name XcmV5TraitsError (47) */
    interface XcmV5TraitsError extends Enum {
        readonly isOverflow: boolean;
        readonly isUnimplemented: boolean;
        readonly isUntrustedReserveLocation: boolean;
        readonly isUntrustedTeleportLocation: boolean;
        readonly isLocationFull: boolean;
        readonly isLocationNotInvertible: boolean;
        readonly isBadOrigin: boolean;
        readonly isInvalidLocation: boolean;
        readonly isAssetNotFound: boolean;
        readonly isFailedToTransactAsset: boolean;
        readonly isNotWithdrawable: boolean;
        readonly isLocationCannotHold: boolean;
        readonly isExceedsMaxMessageSize: boolean;
        readonly isDestinationUnsupported: boolean;
        readonly isTransport: boolean;
        readonly isUnroutable: boolean;
        readonly isUnknownClaim: boolean;
        readonly isFailedToDecode: boolean;
        readonly isMaxWeightInvalid: boolean;
        readonly isNotHoldingFees: boolean;
        readonly isTooExpensive: boolean;
        readonly isTrap: boolean;
        readonly asTrap: u64;
        readonly isExpectationFalse: boolean;
        readonly isPalletNotFound: boolean;
        readonly isNameMismatch: boolean;
        readonly isVersionIncompatible: boolean;
        readonly isHoldingWouldOverflow: boolean;
        readonly isExportError: boolean;
        readonly isReanchorFailed: boolean;
        readonly isNoDeal: boolean;
        readonly isFeesNotMet: boolean;
        readonly isLockError: boolean;
        readonly isNoPermission: boolean;
        readonly isUnanchored: boolean;
        readonly isNotDepositable: boolean;
        readonly isTooManyAssets: boolean;
        readonly isUnhandledXcmVersion: boolean;
        readonly isWeightLimitReached: boolean;
        readonly asWeightLimitReached: SpWeightsWeightV2Weight;
        readonly isBarrier: boolean;
        readonly isWeightNotComputable: boolean;
        readonly isExceedsStackLimit: boolean;
        readonly type: 'Overflow' | 'Unimplemented' | 'UntrustedReserveLocation' | 'UntrustedTeleportLocation' | 'LocationFull' | 'LocationNotInvertible' | 'BadOrigin' | 'InvalidLocation' | 'AssetNotFound' | 'FailedToTransactAsset' | 'NotWithdrawable' | 'LocationCannotHold' | 'ExceedsMaxMessageSize' | 'DestinationUnsupported' | 'Transport' | 'Unroutable' | 'UnknownClaim' | 'FailedToDecode' | 'MaxWeightInvalid' | 'NotHoldingFees' | 'TooExpensive' | 'Trap' | 'ExpectationFalse' | 'PalletNotFound' | 'NameMismatch' | 'VersionIncompatible' | 'HoldingWouldOverflow' | 'ExportError' | 'ReanchorFailed' | 'NoDeal' | 'FeesNotMet' | 'LockError' | 'NoPermission' | 'Unanchored' | 'NotDepositable' | 'TooManyAssets' | 'UnhandledXcmVersion' | 'WeightLimitReached' | 'Barrier' | 'WeightNotComputable' | 'ExceedsStackLimit';
    }
    /** @name StagingXcmV5Location (48) */
    interface StagingXcmV5Location extends Struct {
        readonly parents: u8;
        readonly interior: StagingXcmV5Junctions;
    }
    /** @name StagingXcmV5Junctions (49) */
    interface StagingXcmV5Junctions extends Enum {
        readonly isHere: boolean;
        readonly isX1: boolean;
        readonly asX1: Vec<StagingXcmV5Junction>;
        readonly isX2: boolean;
        readonly asX2: Vec<StagingXcmV5Junction>;
        readonly isX3: boolean;
        readonly asX3: Vec<StagingXcmV5Junction>;
        readonly isX4: boolean;
        readonly asX4: Vec<StagingXcmV5Junction>;
        readonly isX5: boolean;
        readonly asX5: Vec<StagingXcmV5Junction>;
        readonly isX6: boolean;
        readonly asX6: Vec<StagingXcmV5Junction>;
        readonly isX7: boolean;
        readonly asX7: Vec<StagingXcmV5Junction>;
        readonly isX8: boolean;
        readonly asX8: Vec<StagingXcmV5Junction>;
        readonly type: 'Here' | 'X1' | 'X2' | 'X3' | 'X4' | 'X5' | 'X6' | 'X7' | 'X8';
    }
    /** @name StagingXcmV5Junction (51) */
    interface StagingXcmV5Junction extends Enum {
        readonly isParachain: boolean;
        readonly asParachain: Compact<u32>;
        readonly isAccountId32: boolean;
        readonly asAccountId32: {
            readonly network: Option<StagingXcmV5JunctionNetworkId>;
            readonly id: U8aFixed;
        } & Struct;
        readonly isAccountIndex64: boolean;
        readonly asAccountIndex64: {
            readonly network: Option<StagingXcmV5JunctionNetworkId>;
            readonly index: Compact<u64>;
        } & Struct;
        readonly isAccountKey20: boolean;
        readonly asAccountKey20: {
            readonly network: Option<StagingXcmV5JunctionNetworkId>;
            readonly key: U8aFixed;
        } & Struct;
        readonly isPalletInstance: boolean;
        readonly asPalletInstance: u8;
        readonly isGeneralIndex: boolean;
        readonly asGeneralIndex: Compact<u128>;
        readonly isGeneralKey: boolean;
        readonly asGeneralKey: {
            readonly length: u8;
            readonly data: U8aFixed;
        } & Struct;
        readonly isOnlyChild: boolean;
        readonly isPlurality: boolean;
        readonly asPlurality: {
            readonly id: XcmV3JunctionBodyId;
            readonly part: XcmV3JunctionBodyPart;
        } & Struct;
        readonly isGlobalConsensus: boolean;
        readonly asGlobalConsensus: StagingXcmV5JunctionNetworkId;
        readonly type: 'Parachain' | 'AccountId32' | 'AccountIndex64' | 'AccountKey20' | 'PalletInstance' | 'GeneralIndex' | 'GeneralKey' | 'OnlyChild' | 'Plurality' | 'GlobalConsensus';
    }
    /** @name StagingXcmV5JunctionNetworkId (54) */
    interface StagingXcmV5JunctionNetworkId extends Enum {
        readonly isByGenesis: boolean;
        readonly asByGenesis: U8aFixed;
        readonly isByFork: boolean;
        readonly asByFork: {
            readonly blockNumber: u64;
            readonly blockHash: U8aFixed;
        } & Struct;
        readonly isPolkadot: boolean;
        readonly isKusama: boolean;
        readonly isEthereum: boolean;
        readonly asEthereum: {
            readonly chainId: Compact<u64>;
        } & Struct;
        readonly isBitcoinCore: boolean;
        readonly isBitcoinCash: boolean;
        readonly isPolkadotBulletin: boolean;
        readonly type: 'ByGenesis' | 'ByFork' | 'Polkadot' | 'Kusama' | 'Ethereum' | 'BitcoinCore' | 'BitcoinCash' | 'PolkadotBulletin';
    }
    /** @name XcmV3JunctionBodyId (57) */
    interface XcmV3JunctionBodyId extends Enum {
        readonly isUnit: boolean;
        readonly isMoniker: boolean;
        readonly asMoniker: U8aFixed;
        readonly isIndex: boolean;
        readonly asIndex: Compact<u32>;
        readonly isExecutive: boolean;
        readonly isTechnical: boolean;
        readonly isLegislative: boolean;
        readonly isJudicial: boolean;
        readonly isDefense: boolean;
        readonly isAdministration: boolean;
        readonly isTreasury: boolean;
        readonly type: 'Unit' | 'Moniker' | 'Index' | 'Executive' | 'Technical' | 'Legislative' | 'Judicial' | 'Defense' | 'Administration' | 'Treasury';
    }
    /** @name XcmV3JunctionBodyPart (58) */
    interface XcmV3JunctionBodyPart extends Enum {
        readonly isVoice: boolean;
        readonly isMembers: boolean;
        readonly asMembers: {
            readonly count: Compact<u32>;
        } & Struct;
        readonly isFraction: boolean;
        readonly asFraction: {
            readonly nom: Compact<u32>;
            readonly denom: Compact<u32>;
        } & Struct;
        readonly isAtLeastProportion: boolean;
        readonly asAtLeastProportion: {
            readonly nom: Compact<u32>;
            readonly denom: Compact<u32>;
        } & Struct;
        readonly isMoreThanProportion: boolean;
        readonly asMoreThanProportion: {
            readonly nom: Compact<u32>;
            readonly denom: Compact<u32>;
        } & Struct;
        readonly type: 'Voice' | 'Members' | 'Fraction' | 'AtLeastProportion' | 'MoreThanProportion';
    }
    /** @name StagingXcmV5Xcm (66) */
    interface StagingXcmV5Xcm extends Vec<StagingXcmV5Instruction> {
    }
    /** @name StagingXcmV5Instruction (68) */
    interface StagingXcmV5Instruction extends Enum {
        readonly isWithdrawAsset: boolean;
        readonly asWithdrawAsset: StagingXcmV5AssetAssets;
        readonly isReserveAssetDeposited: boolean;
        readonly asReserveAssetDeposited: StagingXcmV5AssetAssets;
        readonly isReceiveTeleportedAsset: boolean;
        readonly asReceiveTeleportedAsset: StagingXcmV5AssetAssets;
        readonly isQueryResponse: boolean;
        readonly asQueryResponse: {
            readonly queryId: Compact<u64>;
            readonly response: StagingXcmV5Response;
            readonly maxWeight: SpWeightsWeightV2Weight;
            readonly querier: Option<StagingXcmV5Location>;
        } & Struct;
        readonly isTransferAsset: boolean;
        readonly asTransferAsset: {
            readonly assets: StagingXcmV5AssetAssets;
            readonly beneficiary: StagingXcmV5Location;
        } & Struct;
        readonly isTransferReserveAsset: boolean;
        readonly asTransferReserveAsset: {
            readonly assets: StagingXcmV5AssetAssets;
            readonly dest: StagingXcmV5Location;
            readonly xcm: StagingXcmV5Xcm;
        } & Struct;
        readonly isTransact: boolean;
        readonly asTransact: {
            readonly originKind: XcmV3OriginKind;
            readonly fallbackMaxWeight: Option<SpWeightsWeightV2Weight>;
            readonly call: XcmDoubleEncoded;
        } & Struct;
        readonly isHrmpNewChannelOpenRequest: boolean;
        readonly asHrmpNewChannelOpenRequest: {
            readonly sender: Compact<u32>;
            readonly maxMessageSize: Compact<u32>;
            readonly maxCapacity: Compact<u32>;
        } & Struct;
        readonly isHrmpChannelAccepted: boolean;
        readonly asHrmpChannelAccepted: {
            readonly recipient: Compact<u32>;
        } & Struct;
        readonly isHrmpChannelClosing: boolean;
        readonly asHrmpChannelClosing: {
            readonly initiator: Compact<u32>;
            readonly sender: Compact<u32>;
            readonly recipient: Compact<u32>;
        } & Struct;
        readonly isClearOrigin: boolean;
        readonly isDescendOrigin: boolean;
        readonly asDescendOrigin: StagingXcmV5Junctions;
        readonly isReportError: boolean;
        readonly asReportError: StagingXcmV5QueryResponseInfo;
        readonly isDepositAsset: boolean;
        readonly asDepositAsset: {
            readonly assets: StagingXcmV5AssetAssetFilter;
            readonly beneficiary: StagingXcmV5Location;
        } & Struct;
        readonly isDepositReserveAsset: boolean;
        readonly asDepositReserveAsset: {
            readonly assets: StagingXcmV5AssetAssetFilter;
            readonly dest: StagingXcmV5Location;
            readonly xcm: StagingXcmV5Xcm;
        } & Struct;
        readonly isExchangeAsset: boolean;
        readonly asExchangeAsset: {
            readonly give: StagingXcmV5AssetAssetFilter;
            readonly want: StagingXcmV5AssetAssets;
            readonly maximal: bool;
        } & Struct;
        readonly isInitiateReserveWithdraw: boolean;
        readonly asInitiateReserveWithdraw: {
            readonly assets: StagingXcmV5AssetAssetFilter;
            readonly reserve: StagingXcmV5Location;
            readonly xcm: StagingXcmV5Xcm;
        } & Struct;
        readonly isInitiateTeleport: boolean;
        readonly asInitiateTeleport: {
            readonly assets: StagingXcmV5AssetAssetFilter;
            readonly dest: StagingXcmV5Location;
            readonly xcm: StagingXcmV5Xcm;
        } & Struct;
        readonly isReportHolding: boolean;
        readonly asReportHolding: {
            readonly responseInfo: StagingXcmV5QueryResponseInfo;
            readonly assets: StagingXcmV5AssetAssetFilter;
        } & Struct;
        readonly isBuyExecution: boolean;
        readonly asBuyExecution: {
            readonly fees: StagingXcmV5Asset;
            readonly weightLimit: XcmV3WeightLimit;
        } & Struct;
        readonly isRefundSurplus: boolean;
        readonly isSetErrorHandler: boolean;
        readonly asSetErrorHandler: StagingXcmV5Xcm;
        readonly isSetAppendix: boolean;
        readonly asSetAppendix: StagingXcmV5Xcm;
        readonly isClearError: boolean;
        readonly isClaimAsset: boolean;
        readonly asClaimAsset: {
            readonly assets: StagingXcmV5AssetAssets;
            readonly ticket: StagingXcmV5Location;
        } & Struct;
        readonly isTrap: boolean;
        readonly asTrap: Compact<u64>;
        readonly isSubscribeVersion: boolean;
        readonly asSubscribeVersion: {
            readonly queryId: Compact<u64>;
            readonly maxResponseWeight: SpWeightsWeightV2Weight;
        } & Struct;
        readonly isUnsubscribeVersion: boolean;
        readonly isBurnAsset: boolean;
        readonly asBurnAsset: StagingXcmV5AssetAssets;
        readonly isExpectAsset: boolean;
        readonly asExpectAsset: StagingXcmV5AssetAssets;
        readonly isExpectOrigin: boolean;
        readonly asExpectOrigin: Option<StagingXcmV5Location>;
        readonly isExpectError: boolean;
        readonly asExpectError: Option<ITuple<[u32, XcmV5TraitsError]>>;
        readonly isExpectTransactStatus: boolean;
        readonly asExpectTransactStatus: XcmV3MaybeErrorCode;
        readonly isQueryPallet: boolean;
        readonly asQueryPallet: {
            readonly moduleName: Bytes;
            readonly responseInfo: StagingXcmV5QueryResponseInfo;
        } & Struct;
        readonly isExpectPallet: boolean;
        readonly asExpectPallet: {
            readonly index: Compact<u32>;
            readonly name: Bytes;
            readonly moduleName: Bytes;
            readonly crateMajor: Compact<u32>;
            readonly minCrateMinor: Compact<u32>;
        } & Struct;
        readonly isReportTransactStatus: boolean;
        readonly asReportTransactStatus: StagingXcmV5QueryResponseInfo;
        readonly isClearTransactStatus: boolean;
        readonly isUniversalOrigin: boolean;
        readonly asUniversalOrigin: StagingXcmV5Junction;
        readonly isExportMessage: boolean;
        readonly asExportMessage: {
            readonly network: StagingXcmV5JunctionNetworkId;
            readonly destination: StagingXcmV5Junctions;
            readonly xcm: StagingXcmV5Xcm;
        } & Struct;
        readonly isLockAsset: boolean;
        readonly asLockAsset: {
            readonly asset: StagingXcmV5Asset;
            readonly unlocker: StagingXcmV5Location;
        } & Struct;
        readonly isUnlockAsset: boolean;
        readonly asUnlockAsset: {
            readonly asset: StagingXcmV5Asset;
            readonly target: StagingXcmV5Location;
        } & Struct;
        readonly isNoteUnlockable: boolean;
        readonly asNoteUnlockable: {
            readonly asset: StagingXcmV5Asset;
            readonly owner: StagingXcmV5Location;
        } & Struct;
        readonly isRequestUnlock: boolean;
        readonly asRequestUnlock: {
            readonly asset: StagingXcmV5Asset;
            readonly locker: StagingXcmV5Location;
        } & Struct;
        readonly isSetFeesMode: boolean;
        readonly asSetFeesMode: {
            readonly jitWithdraw: bool;
        } & Struct;
        readonly isSetTopic: boolean;
        readonly asSetTopic: U8aFixed;
        readonly isClearTopic: boolean;
        readonly isAliasOrigin: boolean;
        readonly asAliasOrigin: StagingXcmV5Location;
        readonly isUnpaidExecution: boolean;
        readonly asUnpaidExecution: {
            readonly weightLimit: XcmV3WeightLimit;
            readonly checkOrigin: Option<StagingXcmV5Location>;
        } & Struct;
        readonly isPayFees: boolean;
        readonly asPayFees: {
            readonly asset: StagingXcmV5Asset;
        } & Struct;
        readonly isInitiateTransfer: boolean;
        readonly asInitiateTransfer: {
            readonly destination: StagingXcmV5Location;
            readonly remoteFees: Option<StagingXcmV5AssetAssetTransferFilter>;
            readonly preserveOrigin: bool;
            readonly assets: Vec<StagingXcmV5AssetAssetTransferFilter>;
            readonly remoteXcm: StagingXcmV5Xcm;
        } & Struct;
        readonly isExecuteWithOrigin: boolean;
        readonly asExecuteWithOrigin: {
            readonly descendantOrigin: Option<StagingXcmV5Junctions>;
            readonly xcm: StagingXcmV5Xcm;
        } & Struct;
        readonly isSetHints: boolean;
        readonly asSetHints: {
            readonly hints: Vec<StagingXcmV5Hint>;
        } & Struct;
        readonly type: 'WithdrawAsset' | 'ReserveAssetDeposited' | 'ReceiveTeleportedAsset' | 'QueryResponse' | 'TransferAsset' | 'TransferReserveAsset' | 'Transact' | 'HrmpNewChannelOpenRequest' | 'HrmpChannelAccepted' | 'HrmpChannelClosing' | 'ClearOrigin' | 'DescendOrigin' | 'ReportError' | 'DepositAsset' | 'DepositReserveAsset' | 'ExchangeAsset' | 'InitiateReserveWithdraw' | 'InitiateTeleport' | 'ReportHolding' | 'BuyExecution' | 'RefundSurplus' | 'SetErrorHandler' | 'SetAppendix' | 'ClearError' | 'ClaimAsset' | 'Trap' | 'SubscribeVersion' | 'UnsubscribeVersion' | 'BurnAsset' | 'ExpectAsset' | 'ExpectOrigin' | 'ExpectError' | 'ExpectTransactStatus' | 'QueryPallet' | 'ExpectPallet' | 'ReportTransactStatus' | 'ClearTransactStatus' | 'UniversalOrigin' | 'ExportMessage' | 'LockAsset' | 'UnlockAsset' | 'NoteUnlockable' | 'RequestUnlock' | 'SetFeesMode' | 'SetTopic' | 'ClearTopic' | 'AliasOrigin' | 'UnpaidExecution' | 'PayFees' | 'InitiateTransfer' | 'ExecuteWithOrigin' | 'SetHints';
    }
    /** @name StagingXcmV5AssetAssets (69) */
    interface StagingXcmV5AssetAssets extends Vec<StagingXcmV5Asset> {
    }
    /** @name StagingXcmV5Asset (71) */
    interface StagingXcmV5Asset extends Struct {
        readonly id: StagingXcmV5AssetAssetId;
        readonly fun: StagingXcmV5AssetFungibility;
    }
    /** @name StagingXcmV5AssetAssetId (72) */
    interface StagingXcmV5AssetAssetId extends StagingXcmV5Location {
    }
    /** @name StagingXcmV5AssetFungibility (73) */
    interface StagingXcmV5AssetFungibility extends Enum {
        readonly isFungible: boolean;
        readonly asFungible: Compact<u128>;
        readonly isNonFungible: boolean;
        readonly asNonFungible: StagingXcmV5AssetAssetInstance;
        readonly type: 'Fungible' | 'NonFungible';
    }
    /** @name StagingXcmV5AssetAssetInstance (74) */
    interface StagingXcmV5AssetAssetInstance extends Enum {
        readonly isUndefined: boolean;
        readonly isIndex: boolean;
        readonly asIndex: Compact<u128>;
        readonly isArray4: boolean;
        readonly asArray4: U8aFixed;
        readonly isArray8: boolean;
        readonly asArray8: U8aFixed;
        readonly isArray16: boolean;
        readonly asArray16: U8aFixed;
        readonly isArray32: boolean;
        readonly asArray32: U8aFixed;
        readonly type: 'Undefined' | 'Index' | 'Array4' | 'Array8' | 'Array16' | 'Array32';
    }
    /** @name StagingXcmV5Response (77) */
    interface StagingXcmV5Response extends Enum {
        readonly isNull: boolean;
        readonly isAssets: boolean;
        readonly asAssets: StagingXcmV5AssetAssets;
        readonly isExecutionResult: boolean;
        readonly asExecutionResult: Option<ITuple<[u32, XcmV5TraitsError]>>;
        readonly isVersion: boolean;
        readonly asVersion: u32;
        readonly isPalletsInfo: boolean;
        readonly asPalletsInfo: Vec<StagingXcmV5PalletInfo>;
        readonly isDispatchResult: boolean;
        readonly asDispatchResult: XcmV3MaybeErrorCode;
        readonly type: 'Null' | 'Assets' | 'ExecutionResult' | 'Version' | 'PalletsInfo' | 'DispatchResult';
    }
    /** @name StagingXcmV5PalletInfo (81) */
    interface StagingXcmV5PalletInfo extends Struct {
        readonly index: Compact<u32>;
        readonly name: Bytes;
        readonly moduleName: Bytes;
        readonly major: Compact<u32>;
        readonly minor: Compact<u32>;
        readonly patch: Compact<u32>;
    }
    /** @name XcmV3MaybeErrorCode (84) */
    interface XcmV3MaybeErrorCode extends Enum {
        readonly isSuccess: boolean;
        readonly isError: boolean;
        readonly asError: Bytes;
        readonly isTruncatedError: boolean;
        readonly asTruncatedError: Bytes;
        readonly type: 'Success' | 'Error' | 'TruncatedError';
    }
    /** @name XcmV3OriginKind (87) */
    interface XcmV3OriginKind extends Enum {
        readonly isNative: boolean;
        readonly isSovereignAccount: boolean;
        readonly isSuperuser: boolean;
        readonly isXcm: boolean;
        readonly type: 'Native' | 'SovereignAccount' | 'Superuser' | 'Xcm';
    }
    /** @name XcmDoubleEncoded (89) */
    interface XcmDoubleEncoded extends Struct {
        readonly encoded: Bytes;
    }
    /** @name StagingXcmV5QueryResponseInfo (90) */
    interface StagingXcmV5QueryResponseInfo extends Struct {
        readonly destination: StagingXcmV5Location;
        readonly queryId: Compact<u64>;
        readonly maxWeight: SpWeightsWeightV2Weight;
    }
    /** @name StagingXcmV5AssetAssetFilter (91) */
    interface StagingXcmV5AssetAssetFilter extends Enum {
        readonly isDefinite: boolean;
        readonly asDefinite: StagingXcmV5AssetAssets;
        readonly isWild: boolean;
        readonly asWild: StagingXcmV5AssetWildAsset;
        readonly type: 'Definite' | 'Wild';
    }
    /** @name StagingXcmV5AssetWildAsset (92) */
    interface StagingXcmV5AssetWildAsset extends Enum {
        readonly isAll: boolean;
        readonly isAllOf: boolean;
        readonly asAllOf: {
            readonly id: StagingXcmV5AssetAssetId;
            readonly fun: StagingXcmV5AssetWildFungibility;
        } & Struct;
        readonly isAllCounted: boolean;
        readonly asAllCounted: Compact<u32>;
        readonly isAllOfCounted: boolean;
        readonly asAllOfCounted: {
            readonly id: StagingXcmV5AssetAssetId;
            readonly fun: StagingXcmV5AssetWildFungibility;
            readonly count: Compact<u32>;
        } & Struct;
        readonly type: 'All' | 'AllOf' | 'AllCounted' | 'AllOfCounted';
    }
    /** @name StagingXcmV5AssetWildFungibility (93) */
    interface StagingXcmV5AssetWildFungibility extends Enum {
        readonly isFungible: boolean;
        readonly isNonFungible: boolean;
        readonly type: 'Fungible' | 'NonFungible';
    }
    /** @name XcmV3WeightLimit (94) */
    interface XcmV3WeightLimit extends Enum {
        readonly isUnlimited: boolean;
        readonly isLimited: boolean;
        readonly asLimited: SpWeightsWeightV2Weight;
        readonly type: 'Unlimited' | 'Limited';
    }
    /** @name StagingXcmV5AssetAssetTransferFilter (96) */
    interface StagingXcmV5AssetAssetTransferFilter extends Enum {
        readonly isTeleport: boolean;
        readonly asTeleport: StagingXcmV5AssetAssetFilter;
        readonly isReserveDeposit: boolean;
        readonly asReserveDeposit: StagingXcmV5AssetAssetFilter;
        readonly isReserveWithdraw: boolean;
        readonly asReserveWithdraw: StagingXcmV5AssetAssetFilter;
        readonly type: 'Teleport' | 'ReserveDeposit' | 'ReserveWithdraw';
    }
    /** @name StagingXcmV5Hint (101) */
    interface StagingXcmV5Hint extends Enum {
        readonly isAssetClaimer: boolean;
        readonly asAssetClaimer: {
            readonly location: StagingXcmV5Location;
        } & Struct;
        readonly type: 'AssetClaimer';
    }
    /** @name XcmVersionedAssets (103) */
    interface XcmVersionedAssets extends Enum {
        readonly isV3: boolean;
        readonly asV3: XcmV3MultiassetMultiAssets;
        readonly isV4: boolean;
        readonly asV4: StagingXcmV4AssetAssets;
        readonly isV5: boolean;
        readonly asV5: StagingXcmV5AssetAssets;
        readonly type: 'V3' | 'V4' | 'V5';
    }
    /** @name XcmV3MultiassetMultiAssets (104) */
    interface XcmV3MultiassetMultiAssets extends Vec<XcmV3MultiAsset> {
    }
    /** @name XcmV3MultiAsset (106) */
    interface XcmV3MultiAsset extends Struct {
        readonly id: XcmV3MultiassetAssetId;
        readonly fun: XcmV3MultiassetFungibility;
    }
    /** @name XcmV3MultiassetAssetId (107) */
    interface XcmV3MultiassetAssetId extends Enum {
        readonly isConcrete: boolean;
        readonly asConcrete: StagingXcmV3MultiLocation;
        readonly isAbstract: boolean;
        readonly asAbstract: U8aFixed;
        readonly type: 'Concrete' | 'Abstract';
    }
    /** @name StagingXcmV3MultiLocation (108) */
    interface StagingXcmV3MultiLocation extends Struct {
        readonly parents: u8;
        readonly interior: XcmV3Junctions;
    }
    /** @name XcmV3Junctions (109) */
    interface XcmV3Junctions extends Enum {
        readonly isHere: boolean;
        readonly isX1: boolean;
        readonly asX1: XcmV3Junction;
        readonly isX2: boolean;
        readonly asX2: ITuple<[XcmV3Junction, XcmV3Junction]>;
        readonly isX3: boolean;
        readonly asX3: ITuple<[XcmV3Junction, XcmV3Junction, XcmV3Junction]>;
        readonly isX4: boolean;
        readonly asX4: ITuple<[XcmV3Junction, XcmV3Junction, XcmV3Junction, XcmV3Junction]>;
        readonly isX5: boolean;
        readonly asX5: ITuple<[XcmV3Junction, XcmV3Junction, XcmV3Junction, XcmV3Junction, XcmV3Junction]>;
        readonly isX6: boolean;
        readonly asX6: ITuple<[XcmV3Junction, XcmV3Junction, XcmV3Junction, XcmV3Junction, XcmV3Junction, XcmV3Junction]>;
        readonly isX7: boolean;
        readonly asX7: ITuple<[XcmV3Junction, XcmV3Junction, XcmV3Junction, XcmV3Junction, XcmV3Junction, XcmV3Junction, XcmV3Junction]>;
        readonly isX8: boolean;
        readonly asX8: ITuple<[XcmV3Junction, XcmV3Junction, XcmV3Junction, XcmV3Junction, XcmV3Junction, XcmV3Junction, XcmV3Junction, XcmV3Junction]>;
        readonly type: 'Here' | 'X1' | 'X2' | 'X3' | 'X4' | 'X5' | 'X6' | 'X7' | 'X8';
    }
    /** @name XcmV3Junction (110) */
    interface XcmV3Junction extends Enum {
        readonly isParachain: boolean;
        readonly asParachain: Compact<u32>;
        readonly isAccountId32: boolean;
        readonly asAccountId32: {
            readonly network: Option<XcmV3JunctionNetworkId>;
            readonly id: U8aFixed;
        } & Struct;
        readonly isAccountIndex64: boolean;
        readonly asAccountIndex64: {
            readonly network: Option<XcmV3JunctionNetworkId>;
            readonly index: Compact<u64>;
        } & Struct;
        readonly isAccountKey20: boolean;
        readonly asAccountKey20: {
            readonly network: Option<XcmV3JunctionNetworkId>;
            readonly key: U8aFixed;
        } & Struct;
        readonly isPalletInstance: boolean;
        readonly asPalletInstance: u8;
        readonly isGeneralIndex: boolean;
        readonly asGeneralIndex: Compact<u128>;
        readonly isGeneralKey: boolean;
        readonly asGeneralKey: {
            readonly length: u8;
            readonly data: U8aFixed;
        } & Struct;
        readonly isOnlyChild: boolean;
        readonly isPlurality: boolean;
        readonly asPlurality: {
            readonly id: XcmV3JunctionBodyId;
            readonly part: XcmV3JunctionBodyPart;
        } & Struct;
        readonly isGlobalConsensus: boolean;
        readonly asGlobalConsensus: XcmV3JunctionNetworkId;
        readonly type: 'Parachain' | 'AccountId32' | 'AccountIndex64' | 'AccountKey20' | 'PalletInstance' | 'GeneralIndex' | 'GeneralKey' | 'OnlyChild' | 'Plurality' | 'GlobalConsensus';
    }
    /** @name XcmV3JunctionNetworkId (112) */
    interface XcmV3JunctionNetworkId extends Enum {
        readonly isByGenesis: boolean;
        readonly asByGenesis: U8aFixed;
        readonly isByFork: boolean;
        readonly asByFork: {
            readonly blockNumber: u64;
            readonly blockHash: U8aFixed;
        } & Struct;
        readonly isPolkadot: boolean;
        readonly isKusama: boolean;
        readonly isWestend: boolean;
        readonly isRococo: boolean;
        readonly isWococo: boolean;
        readonly isEthereum: boolean;
        readonly asEthereum: {
            readonly chainId: Compact<u64>;
        } & Struct;
        readonly isBitcoinCore: boolean;
        readonly isBitcoinCash: boolean;
        readonly isPolkadotBulletin: boolean;
        readonly type: 'ByGenesis' | 'ByFork' | 'Polkadot' | 'Kusama' | 'Westend' | 'Rococo' | 'Wococo' | 'Ethereum' | 'BitcoinCore' | 'BitcoinCash' | 'PolkadotBulletin';
    }
    /** @name XcmV3MultiassetFungibility (113) */
    interface XcmV3MultiassetFungibility extends Enum {
        readonly isFungible: boolean;
        readonly asFungible: Compact<u128>;
        readonly isNonFungible: boolean;
        readonly asNonFungible: XcmV3MultiassetAssetInstance;
        readonly type: 'Fungible' | 'NonFungible';
    }
    /** @name XcmV3MultiassetAssetInstance (114) */
    interface XcmV3MultiassetAssetInstance extends Enum {
        readonly isUndefined: boolean;
        readonly isIndex: boolean;
        readonly asIndex: Compact<u128>;
        readonly isArray4: boolean;
        readonly asArray4: U8aFixed;
        readonly isArray8: boolean;
        readonly asArray8: U8aFixed;
        readonly isArray16: boolean;
        readonly asArray16: U8aFixed;
        readonly isArray32: boolean;
        readonly asArray32: U8aFixed;
        readonly type: 'Undefined' | 'Index' | 'Array4' | 'Array8' | 'Array16' | 'Array32';
    }
    /** @name StagingXcmV4AssetAssets (115) */
    interface StagingXcmV4AssetAssets extends Vec<StagingXcmV4Asset> {
    }
    /** @name StagingXcmV4Asset (117) */
    interface StagingXcmV4Asset extends Struct {
        readonly id: StagingXcmV4AssetAssetId;
        readonly fun: StagingXcmV4AssetFungibility;
    }
    /** @name StagingXcmV4AssetAssetId (118) */
    interface StagingXcmV4AssetAssetId extends StagingXcmV4Location {
    }
    /** @name StagingXcmV4Location (119) */
    interface StagingXcmV4Location extends Struct {
        readonly parents: u8;
        readonly interior: StagingXcmV4Junctions;
    }
    /** @name StagingXcmV4Junctions (120) */
    interface StagingXcmV4Junctions extends Enum {
        readonly isHere: boolean;
        readonly isX1: boolean;
        readonly asX1: StagingXcmV4Junction;
        readonly isX2: boolean;
        readonly asX2: StagingXcmV4Junction;
        readonly isX3: boolean;
        readonly asX3: StagingXcmV4Junction;
        readonly isX4: boolean;
        readonly asX4: StagingXcmV4Junction;
        readonly isX5: boolean;
        readonly asX5: StagingXcmV4Junction;
        readonly isX6: boolean;
        readonly asX6: StagingXcmV4Junction;
        readonly isX7: boolean;
        readonly asX7: StagingXcmV4Junction;
        readonly isX8: boolean;
        readonly asX8: StagingXcmV4Junction;
        readonly type: 'Here' | 'X1' | 'X2' | 'X3' | 'X4' | 'X5' | 'X6' | 'X7' | 'X8';
    }
    /** @name StagingXcmV4Junction (122) */
    interface StagingXcmV4Junction extends Enum {
        readonly isParachain: boolean;
        readonly asParachain: Compact<u32>;
        readonly isAccountId32: boolean;
        readonly asAccountId32: {
            readonly network: Option<StagingXcmV4JunctionNetworkId>;
            readonly id: U8aFixed;
        } & Struct;
        readonly isAccountIndex64: boolean;
        readonly asAccountIndex64: {
            readonly network: Option<StagingXcmV4JunctionNetworkId>;
            readonly index: Compact<u64>;
        } & Struct;
        readonly isAccountKey20: boolean;
        readonly asAccountKey20: {
            readonly network: Option<StagingXcmV4JunctionNetworkId>;
            readonly key: U8aFixed;
        } & Struct;
        readonly isPalletInstance: boolean;
        readonly asPalletInstance: u8;
        readonly isGeneralIndex: boolean;
        readonly asGeneralIndex: Compact<u128>;
        readonly isGeneralKey: boolean;
        readonly asGeneralKey: {
            readonly length: u8;
            readonly data: U8aFixed;
        } & Struct;
        readonly isOnlyChild: boolean;
        readonly isPlurality: boolean;
        readonly asPlurality: {
            readonly id: XcmV3JunctionBodyId;
            readonly part: XcmV3JunctionBodyPart;
        } & Struct;
        readonly isGlobalConsensus: boolean;
        readonly asGlobalConsensus: StagingXcmV4JunctionNetworkId;
        readonly type: 'Parachain' | 'AccountId32' | 'AccountIndex64' | 'AccountKey20' | 'PalletInstance' | 'GeneralIndex' | 'GeneralKey' | 'OnlyChild' | 'Plurality' | 'GlobalConsensus';
    }
    /** @name StagingXcmV4JunctionNetworkId (124) */
    interface StagingXcmV4JunctionNetworkId extends Enum {
        readonly isByGenesis: boolean;
        readonly asByGenesis: U8aFixed;
        readonly isByFork: boolean;
        readonly asByFork: {
            readonly blockNumber: u64;
            readonly blockHash: U8aFixed;
        } & Struct;
        readonly isPolkadot: boolean;
        readonly isKusama: boolean;
        readonly isWestend: boolean;
        readonly isRococo: boolean;
        readonly isWococo: boolean;
        readonly isEthereum: boolean;
        readonly asEthereum: {
            readonly chainId: Compact<u64>;
        } & Struct;
        readonly isBitcoinCore: boolean;
        readonly isBitcoinCash: boolean;
        readonly isPolkadotBulletin: boolean;
        readonly type: 'ByGenesis' | 'ByFork' | 'Polkadot' | 'Kusama' | 'Westend' | 'Rococo' | 'Wococo' | 'Ethereum' | 'BitcoinCore' | 'BitcoinCash' | 'PolkadotBulletin';
    }
    /** @name StagingXcmV4AssetFungibility (132) */
    interface StagingXcmV4AssetFungibility extends Enum {
        readonly isFungible: boolean;
        readonly asFungible: Compact<u128>;
        readonly isNonFungible: boolean;
        readonly asNonFungible: StagingXcmV4AssetAssetInstance;
        readonly type: 'Fungible' | 'NonFungible';
    }
    /** @name StagingXcmV4AssetAssetInstance (133) */
    interface StagingXcmV4AssetAssetInstance extends Enum {
        readonly isUndefined: boolean;
        readonly isIndex: boolean;
        readonly asIndex: Compact<u128>;
        readonly isArray4: boolean;
        readonly asArray4: U8aFixed;
        readonly isArray8: boolean;
        readonly asArray8: U8aFixed;
        readonly isArray16: boolean;
        readonly asArray16: U8aFixed;
        readonly isArray32: boolean;
        readonly asArray32: U8aFixed;
        readonly type: 'Undefined' | 'Index' | 'Array4' | 'Array8' | 'Array16' | 'Array32';
    }
    /** @name XcmVersionedLocation (134) */
    interface XcmVersionedLocation extends Enum {
        readonly isV3: boolean;
        readonly asV3: StagingXcmV3MultiLocation;
        readonly isV4: boolean;
        readonly asV4: StagingXcmV4Location;
        readonly isV5: boolean;
        readonly asV5: StagingXcmV5Location;
        readonly type: 'V3' | 'V4' | 'V5';
    }
    /** @name CumulusPalletXcmEvent (135) */
    interface CumulusPalletXcmEvent extends Enum {
        readonly isInvalidFormat: boolean;
        readonly asInvalidFormat: U8aFixed;
        readonly isUnsupportedVersion: boolean;
        readonly asUnsupportedVersion: U8aFixed;
        readonly isExecutedDownward: boolean;
        readonly asExecutedDownward: ITuple<[U8aFixed, StagingXcmV5TraitsOutcome]>;
        readonly type: 'InvalidFormat' | 'UnsupportedVersion' | 'ExecutedDownward';
    }
    /** @name PalletMessageQueueEvent (136) */
    interface PalletMessageQueueEvent extends Enum {
        readonly isProcessingFailed: boolean;
        readonly asProcessingFailed: {
            readonly id: H256;
            readonly origin: CumulusPrimitivesCoreAggregateMessageOrigin;
            readonly error: FrameSupportMessagesProcessMessageError;
        } & Struct;
        readonly isProcessed: boolean;
        readonly asProcessed: {
            readonly id: H256;
            readonly origin: CumulusPrimitivesCoreAggregateMessageOrigin;
            readonly weightUsed: SpWeightsWeightV2Weight;
            readonly success: bool;
        } & Struct;
        readonly isOverweightEnqueued: boolean;
        readonly asOverweightEnqueued: {
            readonly id: U8aFixed;
            readonly origin: CumulusPrimitivesCoreAggregateMessageOrigin;
            readonly pageIndex: u32;
            readonly messageIndex: u32;
        } & Struct;
        readonly isPageReaped: boolean;
        readonly asPageReaped: {
            readonly origin: CumulusPrimitivesCoreAggregateMessageOrigin;
            readonly index: u32;
        } & Struct;
        readonly type: 'ProcessingFailed' | 'Processed' | 'OverweightEnqueued' | 'PageReaped';
    }
    /** @name CumulusPrimitivesCoreAggregateMessageOrigin (137) */
    interface CumulusPrimitivesCoreAggregateMessageOrigin extends Enum {
        readonly isHere: boolean;
        readonly isParent: boolean;
        readonly isSibling: boolean;
        readonly asSibling: u32;
        readonly type: 'Here' | 'Parent' | 'Sibling';
    }
    /** @name FrameSupportMessagesProcessMessageError (139) */
    interface FrameSupportMessagesProcessMessageError extends Enum {
        readonly isBadFormat: boolean;
        readonly isCorrupt: boolean;
        readonly isUnsupported: boolean;
        readonly isOverweight: boolean;
        readonly asOverweight: SpWeightsWeightV2Weight;
        readonly isYield: boolean;
        readonly isStackLimitReached: boolean;
        readonly type: 'BadFormat' | 'Corrupt' | 'Unsupported' | 'Overweight' | 'Yield' | 'StackLimitReached';
    }
    /** @name PalletStorageProvidersEvent (140) */
    interface PalletStorageProvidersEvent extends Enum {
        readonly isMspRequestSignUpSuccess: boolean;
        readonly asMspRequestSignUpSuccess: {
            readonly who: AccountId32;
            readonly multiaddresses: Vec<Bytes>;
            readonly capacity: u64;
        } & Struct;
        readonly isMspSignUpSuccess: boolean;
        readonly asMspSignUpSuccess: {
            readonly who: AccountId32;
            readonly mspId: H256;
            readonly multiaddresses: Vec<Bytes>;
            readonly capacity: u64;
            readonly valueProp: PalletStorageProvidersValuePropositionWithId;
        } & Struct;
        readonly isBspRequestSignUpSuccess: boolean;
        readonly asBspRequestSignUpSuccess: {
            readonly who: AccountId32;
            readonly multiaddresses: Vec<Bytes>;
            readonly capacity: u64;
        } & Struct;
        readonly isBspSignUpSuccess: boolean;
        readonly asBspSignUpSuccess: {
            readonly who: AccountId32;
            readonly bspId: H256;
            readonly root: H256;
            readonly multiaddresses: Vec<Bytes>;
            readonly capacity: u64;
        } & Struct;
        readonly isSignUpRequestCanceled: boolean;
        readonly asSignUpRequestCanceled: {
            readonly who: AccountId32;
        } & Struct;
        readonly isMspSignOffSuccess: boolean;
        readonly asMspSignOffSuccess: {
            readonly who: AccountId32;
            readonly mspId: H256;
        } & Struct;
        readonly isBspSignOffSuccess: boolean;
        readonly asBspSignOffSuccess: {
            readonly who: AccountId32;
            readonly bspId: H256;
        } & Struct;
        readonly isCapacityChanged: boolean;
        readonly asCapacityChanged: {
            readonly who: AccountId32;
            readonly providerId: PalletStorageProvidersStorageProviderId;
            readonly oldCapacity: u64;
            readonly newCapacity: u64;
            readonly nextBlockWhenChangeAllowed: u32;
        } & Struct;
        readonly isSlashed: boolean;
        readonly asSlashed: {
            readonly providerId: H256;
            readonly amount: u128;
        } & Struct;
        readonly isAwaitingTopUp: boolean;
        readonly asAwaitingTopUp: {
            readonly providerId: H256;
            readonly topUpMetadata: PalletStorageProvidersTopUpMetadata;
        } & Struct;
        readonly isTopUpFulfilled: boolean;
        readonly asTopUpFulfilled: {
            readonly providerId: H256;
            readonly amount: u128;
        } & Struct;
        readonly isFailedToGetOwnerAccountOfInsolventProvider: boolean;
        readonly asFailedToGetOwnerAccountOfInsolventProvider: {
            readonly providerId: H256;
        } & Struct;
        readonly isFailedToSlashInsolventProvider: boolean;
        readonly asFailedToSlashInsolventProvider: {
            readonly providerId: H256;
            readonly amountToSlash: u128;
            readonly error: SpRuntimeDispatchError;
        } & Struct;
        readonly isFailedToStopAllCyclesForInsolventBsp: boolean;
        readonly asFailedToStopAllCyclesForInsolventBsp: {
            readonly providerId: H256;
            readonly error: SpRuntimeDispatchError;
        } & Struct;
        readonly isFailedToInsertProviderTopUpExpiration: boolean;
        readonly asFailedToInsertProviderTopUpExpiration: {
            readonly providerId: H256;
            readonly expirationTick: u32;
        } & Struct;
        readonly isProviderInsolvent: boolean;
        readonly asProviderInsolvent: {
            readonly providerId: H256;
        } & Struct;
        readonly isBucketsOfInsolventMsp: boolean;
        readonly asBucketsOfInsolventMsp: {
            readonly mspId: H256;
            readonly buckets: Vec<H256>;
        } & Struct;
        readonly isBucketRootChanged: boolean;
        readonly asBucketRootChanged: {
            readonly bucketId: H256;
            readonly oldRoot: H256;
            readonly newRoot: H256;
        } & Struct;
        readonly isMultiAddressAdded: boolean;
        readonly asMultiAddressAdded: {
            readonly providerId: H256;
            readonly newMultiaddress: Bytes;
        } & Struct;
        readonly isMultiAddressRemoved: boolean;
        readonly asMultiAddressRemoved: {
            readonly providerId: H256;
            readonly removedMultiaddress: Bytes;
        } & Struct;
        readonly isValuePropAdded: boolean;
        readonly asValuePropAdded: {
            readonly mspId: H256;
            readonly valuePropId: H256;
            readonly valueProp: PalletStorageProvidersValueProposition;
        } & Struct;
        readonly isValuePropUnavailable: boolean;
        readonly asValuePropUnavailable: {
            readonly mspId: H256;
            readonly valuePropId: H256;
        } & Struct;
        readonly isMspDeleted: boolean;
        readonly asMspDeleted: {
            readonly providerId: H256;
        } & Struct;
        readonly isBspDeleted: boolean;
        readonly asBspDeleted: {
            readonly providerId: H256;
        } & Struct;
        readonly type: 'MspRequestSignUpSuccess' | 'MspSignUpSuccess' | 'BspRequestSignUpSuccess' | 'BspSignUpSuccess' | 'SignUpRequestCanceled' | 'MspSignOffSuccess' | 'BspSignOffSuccess' | 'CapacityChanged' | 'Slashed' | 'AwaitingTopUp' | 'TopUpFulfilled' | 'FailedToGetOwnerAccountOfInsolventProvider' | 'FailedToSlashInsolventProvider' | 'FailedToStopAllCyclesForInsolventBsp' | 'FailedToInsertProviderTopUpExpiration' | 'ProviderInsolvent' | 'BucketsOfInsolventMsp' | 'BucketRootChanged' | 'MultiAddressAdded' | 'MultiAddressRemoved' | 'ValuePropAdded' | 'ValuePropUnavailable' | 'MspDeleted' | 'BspDeleted';
    }
    /** @name PalletStorageProvidersValuePropositionWithId (144) */
    interface PalletStorageProvidersValuePropositionWithId extends Struct {
        readonly id: H256;
        readonly valueProp: PalletStorageProvidersValueProposition;
    }
    /** @name PalletStorageProvidersValueProposition (145) */
    interface PalletStorageProvidersValueProposition extends Struct {
        readonly pricePerGigaUnitOfDataPerBlock: u128;
        readonly commitment: Bytes;
        readonly bucketDataLimit: u64;
        readonly available: bool;
    }
    /** @name PalletStorageProvidersStorageProviderId (147) */
    interface PalletStorageProvidersStorageProviderId extends Enum {
        readonly isBackupStorageProvider: boolean;
        readonly asBackupStorageProvider: H256;
        readonly isMainStorageProvider: boolean;
        readonly asMainStorageProvider: H256;
        readonly type: 'BackupStorageProvider' | 'MainStorageProvider';
    }
    /** @name PalletStorageProvidersTopUpMetadata (148) */
    interface PalletStorageProvidersTopUpMetadata extends Struct {
        readonly startedAt: u32;
        readonly endTickGracePeriod: u32;
    }
    /** @name PalletFileSystemEvent (150) */
    interface PalletFileSystemEvent extends Enum {
        readonly isNewBucket: boolean;
        readonly asNewBucket: {
            readonly who: AccountId32;
            readonly mspId: H256;
            readonly bucketId: H256;
            readonly name: Bytes;
            readonly root: H256;
            readonly collectionId: Option<u32>;
            readonly private: bool;
            readonly valuePropId: H256;
        } & Struct;
        readonly isBucketDeleted: boolean;
        readonly asBucketDeleted: {
            readonly who: AccountId32;
            readonly bucketId: H256;
            readonly maybeCollectionId: Option<u32>;
        } & Struct;
        readonly isBucketPrivacyUpdated: boolean;
        readonly asBucketPrivacyUpdated: {
            readonly who: AccountId32;
            readonly bucketId: H256;
            readonly collectionId: Option<u32>;
            readonly private: bool;
        } & Struct;
        readonly isNewCollectionAndAssociation: boolean;
        readonly asNewCollectionAndAssociation: {
            readonly who: AccountId32;
            readonly bucketId: H256;
            readonly collectionId: u32;
        } & Struct;
        readonly isMoveBucketRequested: boolean;
        readonly asMoveBucketRequested: {
            readonly who: AccountId32;
            readonly bucketId: H256;
            readonly newMspId: H256;
            readonly newValuePropId: H256;
        } & Struct;
        readonly isMoveBucketRequestExpired: boolean;
        readonly asMoveBucketRequestExpired: {
            readonly bucketId: H256;
        } & Struct;
        readonly isMoveBucketAccepted: boolean;
        readonly asMoveBucketAccepted: {
            readonly bucketId: H256;
            readonly oldMspId: Option<H256>;
            readonly newMspId: H256;
            readonly valuePropId: H256;
        } & Struct;
        readonly isMoveBucketRejected: boolean;
        readonly asMoveBucketRejected: {
            readonly bucketId: H256;
            readonly oldMspId: Option<H256>;
            readonly newMspId: H256;
        } & Struct;
        readonly isNewStorageRequest: boolean;
        readonly asNewStorageRequest: {
            readonly who: AccountId32;
            readonly fileKey: H256;
            readonly bucketId: H256;
            readonly location: Bytes;
            readonly fingerprint: H256;
            readonly size_: u64;
            readonly peerIds: Vec<Bytes>;
            readonly expiresAt: u32;
        } & Struct;
        readonly isMspAcceptedStorageRequest: boolean;
        readonly asMspAcceptedStorageRequest: {
            readonly fileKey: H256;
            readonly fileMetadata: ShpFileMetadataFileMetadata;
        } & Struct;
        readonly isStorageRequestFulfilled: boolean;
        readonly asStorageRequestFulfilled: {
            readonly fileKey: H256;
        } & Struct;
        readonly isStorageRequestExpired: boolean;
        readonly asStorageRequestExpired: {
            readonly fileKey: H256;
        } & Struct;
        readonly isStorageRequestRevoked: boolean;
        readonly asStorageRequestRevoked: {
            readonly fileKey: H256;
        } & Struct;
        readonly isStorageRequestRejected: boolean;
        readonly asStorageRequestRejected: {
            readonly fileKey: H256;
            readonly mspId: H256;
            readonly bucketId: H256;
            readonly reason: PalletFileSystemRejectedStorageRequestReason;
        } & Struct;
        readonly isIncompleteStorageRequest: boolean;
        readonly asIncompleteStorageRequest: {
            readonly fileKey: H256;
        } & Struct;
        readonly isIncompleteStorageRequestCleanedUp: boolean;
        readonly asIncompleteStorageRequestCleanedUp: {
            readonly fileKey: H256;
        } & Struct;
        readonly isAcceptedBspVolunteer: boolean;
        readonly asAcceptedBspVolunteer: {
            readonly bspId: H256;
            readonly bucketId: H256;
            readonly location: Bytes;
            readonly fingerprint: H256;
            readonly multiaddresses: Vec<Bytes>;
            readonly owner: AccountId32;
            readonly size_: u64;
        } & Struct;
        readonly isBspConfirmedStoring: boolean;
        readonly asBspConfirmedStoring: {
            readonly who: AccountId32;
            readonly bspId: H256;
            readonly confirmedFileKeys: Vec<ITuple<[H256, ShpFileMetadataFileMetadata]>>;
            readonly skippedFileKeys: Vec<H256>;
            readonly newRoot: H256;
        } & Struct;
        readonly isBspChallengeCycleInitialised: boolean;
        readonly asBspChallengeCycleInitialised: {
            readonly who: AccountId32;
            readonly bspId: H256;
        } & Struct;
        readonly isBspRequestedToStopStoring: boolean;
        readonly asBspRequestedToStopStoring: {
            readonly bspId: H256;
            readonly fileKey: H256;
            readonly owner: AccountId32;
            readonly location: Bytes;
        } & Struct;
        readonly isBspConfirmStoppedStoring: boolean;
        readonly asBspConfirmStoppedStoring: {
            readonly bspId: H256;
            readonly fileKey: H256;
            readonly newRoot: H256;
        } & Struct;
        readonly isMspStoppedStoringBucket: boolean;
        readonly asMspStoppedStoringBucket: {
            readonly mspId: H256;
            readonly owner: AccountId32;
            readonly bucketId: H256;
        } & Struct;
        readonly isSpStopStoringInsolventUser: boolean;
        readonly asSpStopStoringInsolventUser: {
            readonly spId: H256;
            readonly fileKey: H256;
            readonly owner: AccountId32;
            readonly location: Bytes;
            readonly newRoot: H256;
        } & Struct;
        readonly isMspStopStoringBucketInsolventUser: boolean;
        readonly asMspStopStoringBucketInsolventUser: {
            readonly mspId: H256;
            readonly owner: AccountId32;
            readonly bucketId: H256;
        } & Struct;
        readonly isFileDeletionRequested: boolean;
        readonly asFileDeletionRequested: {
            readonly signedDeleteIntention: PalletFileSystemFileOperationIntention;
            readonly signature: SpRuntimeMultiSignature;
        } & Struct;
        readonly isBucketFileDeletionsCompleted: boolean;
        readonly asBucketFileDeletionsCompleted: {
            readonly user: AccountId32;
            readonly fileKeys: Vec<H256>;
            readonly bucketId: H256;
            readonly mspId: Option<H256>;
            readonly oldRoot: H256;
            readonly newRoot: H256;
        } & Struct;
        readonly isBspFileDeletionsCompleted: boolean;
        readonly asBspFileDeletionsCompleted: {
            readonly users: Vec<AccountId32>;
            readonly fileKeys: Vec<H256>;
            readonly bspId: H256;
            readonly oldRoot: H256;
            readonly newRoot: H256;
        } & Struct;
        readonly isUsedCapacityShouldBeZero: boolean;
        readonly asUsedCapacityShouldBeZero: {
            readonly actualUsedCapacity: u64;
        } & Struct;
        readonly isFailedToReleaseStorageRequestCreationDeposit: boolean;
        readonly asFailedToReleaseStorageRequestCreationDeposit: {
            readonly fileKey: H256;
            readonly owner: AccountId32;
            readonly amountToReturn: u128;
            readonly error: SpRuntimeDispatchError;
        } & Struct;
        readonly type: 'NewBucket' | 'BucketDeleted' | 'BucketPrivacyUpdated' | 'NewCollectionAndAssociation' | 'MoveBucketRequested' | 'MoveBucketRequestExpired' | 'MoveBucketAccepted' | 'MoveBucketRejected' | 'NewStorageRequest' | 'MspAcceptedStorageRequest' | 'StorageRequestFulfilled' | 'StorageRequestExpired' | 'StorageRequestRevoked' | 'StorageRequestRejected' | 'IncompleteStorageRequest' | 'IncompleteStorageRequestCleanedUp' | 'AcceptedBspVolunteer' | 'BspConfirmedStoring' | 'BspChallengeCycleInitialised' | 'BspRequestedToStopStoring' | 'BspConfirmStoppedStoring' | 'MspStoppedStoringBucket' | 'SpStopStoringInsolventUser' | 'MspStopStoringBucketInsolventUser' | 'FileDeletionRequested' | 'BucketFileDeletionsCompleted' | 'BspFileDeletionsCompleted' | 'UsedCapacityShouldBeZero' | 'FailedToReleaseStorageRequestCreationDeposit';
    }
    /** @name ShpFileMetadataFileMetadata (154) */
    interface ShpFileMetadataFileMetadata extends Struct {
        readonly owner: Bytes;
        readonly bucketId: Bytes;
        readonly location: Bytes;
        readonly fileSize: Compact<u64>;
        readonly fingerprint: ShpFileMetadataFingerprint;
    }
    /** @name ShpFileMetadataFingerprint (155) */
    interface ShpFileMetadataFingerprint extends U8aFixed {
    }
    /** @name PalletFileSystemRejectedStorageRequestReason (156) */
    interface PalletFileSystemRejectedStorageRequestReason extends Enum {
        readonly isReachedMaximumCapacity: boolean;
        readonly isReceivedInvalidProof: boolean;
        readonly isFileKeyAlreadyStored: boolean;
        readonly isRequestExpired: boolean;
        readonly isInternalError: boolean;
        readonly type: 'ReachedMaximumCapacity' | 'ReceivedInvalidProof' | 'FileKeyAlreadyStored' | 'RequestExpired' | 'InternalError';
    }
    /** @name PalletFileSystemFileOperationIntention (161) */
    interface PalletFileSystemFileOperationIntention extends Struct {
        readonly fileKey: H256;
        readonly operation: PalletFileSystemFileOperation;
    }
    /** @name PalletFileSystemFileOperation (162) */
    interface PalletFileSystemFileOperation extends Enum {
        readonly isDelete: boolean;
        readonly type: 'Delete';
    }
    /** @name SpRuntimeMultiSignature (163) */
    interface SpRuntimeMultiSignature extends Enum {
        readonly isEd25519: boolean;
        readonly asEd25519: U8aFixed;
        readonly isSr25519: boolean;
        readonly asSr25519: U8aFixed;
        readonly isEcdsa: boolean;
        readonly asEcdsa: U8aFixed;
        readonly type: 'Ed25519' | 'Sr25519' | 'Ecdsa';
    }
    /** @name PalletProofsDealerEvent (168) */
    interface PalletProofsDealerEvent extends Enum {
        readonly isNewChallenge: boolean;
        readonly asNewChallenge: {
            readonly who: Option<AccountId32>;
            readonly keyChallenged: H256;
        } & Struct;
        readonly isNewPriorityChallenge: boolean;
        readonly asNewPriorityChallenge: {
            readonly who: Option<AccountId32>;
            readonly keyChallenged: H256;
            readonly shouldRemoveKey: bool;
        } & Struct;
        readonly isProofAccepted: boolean;
        readonly asProofAccepted: {
            readonly providerId: H256;
            readonly proof: PalletProofsDealerProof;
            readonly lastTickProven: u32;
        } & Struct;
        readonly isNewChallengeSeed: boolean;
        readonly asNewChallengeSeed: {
            readonly challengesTicker: u32;
            readonly seed: H256;
        } & Struct;
        readonly isNewCheckpointChallenge: boolean;
        readonly asNewCheckpointChallenge: {
            readonly challengesTicker: u32;
            readonly challenges: Vec<PalletProofsDealerCustomChallenge>;
        } & Struct;
        readonly isSlashableProvider: boolean;
        readonly asSlashableProvider: {
            readonly provider: H256;
            readonly nextChallengeDeadline: u32;
        } & Struct;
        readonly isNoRecordOfLastSubmittedProof: boolean;
        readonly asNoRecordOfLastSubmittedProof: {
            readonly provider: H256;
        } & Struct;
        readonly isNewChallengeCycleInitialised: boolean;
        readonly asNewChallengeCycleInitialised: {
            readonly currentTick: u32;
            readonly nextChallengeDeadline: u32;
            readonly provider: H256;
            readonly maybeProviderAccount: Option<AccountId32>;
        } & Struct;
        readonly isMutationsAppliedForProvider: boolean;
        readonly asMutationsAppliedForProvider: {
            readonly providerId: H256;
            readonly mutations: Vec<ITuple<[H256, ShpTraitsTrieMutation]>>;
            readonly oldRoot: H256;
            readonly newRoot: H256;
        } & Struct;
        readonly isMutationsApplied: boolean;
        readonly asMutationsApplied: {
            readonly mutations: Vec<ITuple<[H256, ShpTraitsTrieMutation]>>;
            readonly oldRoot: H256;
            readonly newRoot: H256;
            readonly eventInfo: Option<Bytes>;
        } & Struct;
        readonly isChallengesTickerSet: boolean;
        readonly asChallengesTickerSet: {
            readonly paused: bool;
        } & Struct;
        readonly type: 'NewChallenge' | 'NewPriorityChallenge' | 'ProofAccepted' | 'NewChallengeSeed' | 'NewCheckpointChallenge' | 'SlashableProvider' | 'NoRecordOfLastSubmittedProof' | 'NewChallengeCycleInitialised' | 'MutationsAppliedForProvider' | 'MutationsApplied' | 'ChallengesTickerSet';
    }
    /** @name PalletProofsDealerProof (169) */
    interface PalletProofsDealerProof extends Struct {
        readonly forestProof: SpTrieStorageProofCompactProof;
        readonly keyProofs: BTreeMap<H256, PalletProofsDealerKeyProof>;
    }
    /** @name SpTrieStorageProofCompactProof (170) */
    interface SpTrieStorageProofCompactProof extends Struct {
        readonly encodedNodes: Vec<Bytes>;
    }
    /** @name PalletProofsDealerKeyProof (173) */
    interface PalletProofsDealerKeyProof extends Struct {
        readonly proof: ShpFileKeyVerifierFileKeyProof;
        readonly challengeCount: u32;
    }
    /** @name ShpFileKeyVerifierFileKeyProof (174) */
    interface ShpFileKeyVerifierFileKeyProof extends Struct {
        readonly fileMetadata: ShpFileMetadataFileMetadata;
        readonly proof: SpTrieStorageProofCompactProof;
    }
    /** @name PalletProofsDealerCustomChallenge (178) */
    interface PalletProofsDealerCustomChallenge extends Struct {
        readonly key: H256;
        readonly shouldRemoveKey: bool;
    }
    /** @name ShpTraitsTrieMutation (182) */
    interface ShpTraitsTrieMutation extends Enum {
        readonly isAdd: boolean;
        readonly asAdd: ShpTraitsTrieAddMutation;
        readonly isRemove: boolean;
        readonly asRemove: ShpTraitsTrieRemoveMutation;
        readonly type: 'Add' | 'Remove';
    }
    /** @name ShpTraitsTrieAddMutation (183) */
    interface ShpTraitsTrieAddMutation extends Struct {
        readonly value: Bytes;
    }
    /** @name ShpTraitsTrieRemoveMutation (184) */
    interface ShpTraitsTrieRemoveMutation extends Struct {
        readonly maybeValue: Option<Bytes>;
    }
    /** @name PalletRandomnessEvent (186) */
    interface PalletRandomnessEvent extends Enum {
        readonly isNewOneEpochAgoRandomnessAvailable: boolean;
        readonly asNewOneEpochAgoRandomnessAvailable: {
            readonly randomnessSeed: H256;
            readonly fromEpoch: u64;
            readonly validUntilBlock: u32;
        } & Struct;
        readonly type: 'NewOneEpochAgoRandomnessAvailable';
    }
    /** @name PalletPaymentStreamsEvent (187) */
    interface PalletPaymentStreamsEvent extends Enum {
        readonly isFixedRatePaymentStreamCreated: boolean;
        readonly asFixedRatePaymentStreamCreated: {
            readonly userAccount: AccountId32;
            readonly providerId: H256;
            readonly rate: u128;
        } & Struct;
        readonly isFixedRatePaymentStreamUpdated: boolean;
        readonly asFixedRatePaymentStreamUpdated: {
            readonly userAccount: AccountId32;
            readonly providerId: H256;
            readonly newRate: u128;
        } & Struct;
        readonly isFixedRatePaymentStreamDeleted: boolean;
        readonly asFixedRatePaymentStreamDeleted: {
            readonly userAccount: AccountId32;
            readonly providerId: H256;
        } & Struct;
        readonly isDynamicRatePaymentStreamCreated: boolean;
        readonly asDynamicRatePaymentStreamCreated: {
            readonly userAccount: AccountId32;
            readonly providerId: H256;
            readonly amountProvided: u64;
        } & Struct;
        readonly isDynamicRatePaymentStreamUpdated: boolean;
        readonly asDynamicRatePaymentStreamUpdated: {
            readonly userAccount: AccountId32;
            readonly providerId: H256;
            readonly newAmountProvided: u64;
        } & Struct;
        readonly isDynamicRatePaymentStreamDeleted: boolean;
        readonly asDynamicRatePaymentStreamDeleted: {
            readonly userAccount: AccountId32;
            readonly providerId: H256;
        } & Struct;
        readonly isPaymentStreamCharged: boolean;
        readonly asPaymentStreamCharged: {
            readonly userAccount: AccountId32;
            readonly providerId: H256;
            readonly amount: u128;
            readonly lastTickCharged: u32;
            readonly chargedAtTick: u32;
        } & Struct;
        readonly isUsersCharged: boolean;
        readonly asUsersCharged: {
            readonly userAccounts: Vec<AccountId32>;
            readonly providerId: H256;
            readonly chargedAtTick: u32;
        } & Struct;
        readonly isLastChargeableInfoUpdated: boolean;
        readonly asLastChargeableInfoUpdated: {
            readonly providerId: H256;
            readonly lastChargeableTick: u32;
            readonly lastChargeablePriceIndex: u128;
        } & Struct;
        readonly isUserWithoutFunds: boolean;
        readonly asUserWithoutFunds: {
            readonly who: AccountId32;
        } & Struct;
        readonly isUserPaidAllDebts: boolean;
        readonly asUserPaidAllDebts: {
            readonly who: AccountId32;
        } & Struct;
        readonly isUserPaidSomeDebts: boolean;
        readonly asUserPaidSomeDebts: {
            readonly who: AccountId32;
        } & Struct;
        readonly isUserSolvent: boolean;
        readonly asUserSolvent: {
            readonly who: AccountId32;
        } & Struct;
        readonly isInconsistentTickProcessing: boolean;
        readonly asInconsistentTickProcessing: {
            readonly lastProcessedTick: u32;
            readonly tickToProcess: u32;
        } & Struct;
        readonly type: 'FixedRatePaymentStreamCreated' | 'FixedRatePaymentStreamUpdated' | 'FixedRatePaymentStreamDeleted' | 'DynamicRatePaymentStreamCreated' | 'DynamicRatePaymentStreamUpdated' | 'DynamicRatePaymentStreamDeleted' | 'PaymentStreamCharged' | 'UsersCharged' | 'LastChargeableInfoUpdated' | 'UserWithoutFunds' | 'UserPaidAllDebts' | 'UserPaidSomeDebts' | 'UserSolvent' | 'InconsistentTickProcessing';
    }
    /** @name PalletBucketNftsEvent (189) */
    interface PalletBucketNftsEvent extends Enum {
        readonly isAccessShared: boolean;
        readonly asAccessShared: {
            readonly issuer: AccountId32;
            readonly recipient: AccountId32;
        } & Struct;
        readonly isItemReadAccessUpdated: boolean;
        readonly asItemReadAccessUpdated: {
            readonly admin: AccountId32;
            readonly bucket: H256;
            readonly itemId: u32;
        } & Struct;
        readonly isItemBurned: boolean;
        readonly asItemBurned: {
            readonly account: AccountId32;
            readonly bucket: H256;
            readonly itemId: u32;
        } & Struct;
        readonly type: 'AccessShared' | 'ItemReadAccessUpdated' | 'ItemBurned';
    }
    /** @name PalletNftsEvent (190) */
    interface PalletNftsEvent extends Enum {
        readonly isCreated: boolean;
        readonly asCreated: {
            readonly collection: u32;
            readonly creator: AccountId32;
            readonly owner: AccountId32;
        } & Struct;
        readonly isForceCreated: boolean;
        readonly asForceCreated: {
            readonly collection: u32;
            readonly owner: AccountId32;
        } & Struct;
        readonly isDestroyed: boolean;
        readonly asDestroyed: {
            readonly collection: u32;
        } & Struct;
        readonly isIssued: boolean;
        readonly asIssued: {
            readonly collection: u32;
            readonly item: u32;
            readonly owner: AccountId32;
        } & Struct;
        readonly isTransferred: boolean;
        readonly asTransferred: {
            readonly collection: u32;
            readonly item: u32;
            readonly from: AccountId32;
            readonly to: AccountId32;
        } & Struct;
        readonly isBurned: boolean;
        readonly asBurned: {
            readonly collection: u32;
            readonly item: u32;
            readonly owner: AccountId32;
        } & Struct;
        readonly isItemTransferLocked: boolean;
        readonly asItemTransferLocked: {
            readonly collection: u32;
            readonly item: u32;
        } & Struct;
        readonly isItemTransferUnlocked: boolean;
        readonly asItemTransferUnlocked: {
            readonly collection: u32;
            readonly item: u32;
        } & Struct;
        readonly isItemPropertiesLocked: boolean;
        readonly asItemPropertiesLocked: {
            readonly collection: u32;
            readonly item: u32;
            readonly lockMetadata: bool;
            readonly lockAttributes: bool;
        } & Struct;
        readonly isCollectionLocked: boolean;
        readonly asCollectionLocked: {
            readonly collection: u32;
        } & Struct;
        readonly isOwnerChanged: boolean;
        readonly asOwnerChanged: {
            readonly collection: u32;
            readonly newOwner: AccountId32;
        } & Struct;
        readonly isTeamChanged: boolean;
        readonly asTeamChanged: {
            readonly collection: u32;
            readonly issuer: Option<AccountId32>;
            readonly admin: Option<AccountId32>;
            readonly freezer: Option<AccountId32>;
        } & Struct;
        readonly isTransferApproved: boolean;
        readonly asTransferApproved: {
            readonly collection: u32;
            readonly item: u32;
            readonly owner: AccountId32;
            readonly delegate: AccountId32;
            readonly deadline: Option<u32>;
        } & Struct;
        readonly isApprovalCancelled: boolean;
        readonly asApprovalCancelled: {
            readonly collection: u32;
            readonly item: u32;
            readonly owner: AccountId32;
            readonly delegate: AccountId32;
        } & Struct;
        readonly isAllApprovalsCancelled: boolean;
        readonly asAllApprovalsCancelled: {
            readonly collection: u32;
            readonly item: u32;
            readonly owner: AccountId32;
        } & Struct;
        readonly isCollectionConfigChanged: boolean;
        readonly asCollectionConfigChanged: {
            readonly collection: u32;
        } & Struct;
        readonly isCollectionMetadataSet: boolean;
        readonly asCollectionMetadataSet: {
            readonly collection: u32;
            readonly data: Bytes;
        } & Struct;
        readonly isCollectionMetadataCleared: boolean;
        readonly asCollectionMetadataCleared: {
            readonly collection: u32;
        } & Struct;
        readonly isItemMetadataSet: boolean;
        readonly asItemMetadataSet: {
            readonly collection: u32;
            readonly item: u32;
            readonly data: Bytes;
        } & Struct;
        readonly isItemMetadataCleared: boolean;
        readonly asItemMetadataCleared: {
            readonly collection: u32;
            readonly item: u32;
        } & Struct;
        readonly isRedeposited: boolean;
        readonly asRedeposited: {
            readonly collection: u32;
            readonly successfulItems: Vec<u32>;
        } & Struct;
        readonly isAttributeSet: boolean;
        readonly asAttributeSet: {
            readonly collection: u32;
            readonly maybeItem: Option<u32>;
            readonly key: Bytes;
            readonly value: Bytes;
            readonly namespace: PalletNftsAttributeNamespace;
        } & Struct;
        readonly isAttributeCleared: boolean;
        readonly asAttributeCleared: {
            readonly collection: u32;
            readonly maybeItem: Option<u32>;
            readonly key: Bytes;
            readonly namespace: PalletNftsAttributeNamespace;
        } & Struct;
        readonly isItemAttributesApprovalAdded: boolean;
        readonly asItemAttributesApprovalAdded: {
            readonly collection: u32;
            readonly item: u32;
            readonly delegate: AccountId32;
        } & Struct;
        readonly isItemAttributesApprovalRemoved: boolean;
        readonly asItemAttributesApprovalRemoved: {
            readonly collection: u32;
            readonly item: u32;
            readonly delegate: AccountId32;
        } & Struct;
        readonly isOwnershipAcceptanceChanged: boolean;
        readonly asOwnershipAcceptanceChanged: {
            readonly who: AccountId32;
            readonly maybeCollection: Option<u32>;
        } & Struct;
        readonly isCollectionMaxSupplySet: boolean;
        readonly asCollectionMaxSupplySet: {
            readonly collection: u32;
            readonly maxSupply: u32;
        } & Struct;
        readonly isCollectionMintSettingsUpdated: boolean;
        readonly asCollectionMintSettingsUpdated: {
            readonly collection: u32;
        } & Struct;
        readonly isNextCollectionIdIncremented: boolean;
        readonly asNextCollectionIdIncremented: {
            readonly nextId: Option<u32>;
        } & Struct;
        readonly isItemPriceSet: boolean;
        readonly asItemPriceSet: {
            readonly collection: u32;
            readonly item: u32;
            readonly price: u128;
            readonly whitelistedBuyer: Option<AccountId32>;
        } & Struct;
        readonly isItemPriceRemoved: boolean;
        readonly asItemPriceRemoved: {
            readonly collection: u32;
            readonly item: u32;
        } & Struct;
        readonly isItemBought: boolean;
        readonly asItemBought: {
            readonly collection: u32;
            readonly item: u32;
            readonly price: u128;
            readonly seller: AccountId32;
            readonly buyer: AccountId32;
        } & Struct;
        readonly isTipSent: boolean;
        readonly asTipSent: {
            readonly collection: u32;
            readonly item: u32;
            readonly sender: AccountId32;
            readonly receiver: AccountId32;
            readonly amount: u128;
        } & Struct;
        readonly isSwapCreated: boolean;
        readonly asSwapCreated: {
            readonly offeredCollection: u32;
            readonly offeredItem: u32;
            readonly desiredCollection: u32;
            readonly desiredItem: Option<u32>;
            readonly price: Option<PalletNftsPriceWithDirection>;
            readonly deadline: u32;
        } & Struct;
        readonly isSwapCancelled: boolean;
        readonly asSwapCancelled: {
            readonly offeredCollection: u32;
            readonly offeredItem: u32;
            readonly desiredCollection: u32;
            readonly desiredItem: Option<u32>;
            readonly price: Option<PalletNftsPriceWithDirection>;
            readonly deadline: u32;
        } & Struct;
        readonly isSwapClaimed: boolean;
        readonly asSwapClaimed: {
            readonly sentCollection: u32;
            readonly sentItem: u32;
            readonly sentItemOwner: AccountId32;
            readonly receivedCollection: u32;
            readonly receivedItem: u32;
            readonly receivedItemOwner: AccountId32;
            readonly price: Option<PalletNftsPriceWithDirection>;
            readonly deadline: u32;
        } & Struct;
        readonly isPreSignedAttributesSet: boolean;
        readonly asPreSignedAttributesSet: {
            readonly collection: u32;
            readonly item: u32;
            readonly namespace: PalletNftsAttributeNamespace;
        } & Struct;
        readonly isPalletAttributeSet: boolean;
        readonly asPalletAttributeSet: {
            readonly collection: u32;
            readonly item: Option<u32>;
            readonly attribute: PalletNftsPalletAttributes;
            readonly value: Bytes;
        } & Struct;
        readonly type: 'Created' | 'ForceCreated' | 'Destroyed' | 'Issued' | 'Transferred' | 'Burned' | 'ItemTransferLocked' | 'ItemTransferUnlocked' | 'ItemPropertiesLocked' | 'CollectionLocked' | 'OwnerChanged' | 'TeamChanged' | 'TransferApproved' | 'ApprovalCancelled' | 'AllApprovalsCancelled' | 'CollectionConfigChanged' | 'CollectionMetadataSet' | 'CollectionMetadataCleared' | 'ItemMetadataSet' | 'ItemMetadataCleared' | 'Redeposited' | 'AttributeSet' | 'AttributeCleared' | 'ItemAttributesApprovalAdded' | 'ItemAttributesApprovalRemoved' | 'OwnershipAcceptanceChanged' | 'CollectionMaxSupplySet' | 'CollectionMintSettingsUpdated' | 'NextCollectionIdIncremented' | 'ItemPriceSet' | 'ItemPriceRemoved' | 'ItemBought' | 'TipSent' | 'SwapCreated' | 'SwapCancelled' | 'SwapClaimed' | 'PreSignedAttributesSet' | 'PalletAttributeSet';
    }
    /** @name PalletNftsAttributeNamespace (194) */
    interface PalletNftsAttributeNamespace extends Enum {
        readonly isPallet: boolean;
        readonly isCollectionOwner: boolean;
        readonly isItemOwner: boolean;
        readonly isAccount: boolean;
        readonly asAccount: AccountId32;
        readonly type: 'Pallet' | 'CollectionOwner' | 'ItemOwner' | 'Account';
    }
    /** @name PalletNftsPriceWithDirection (196) */
    interface PalletNftsPriceWithDirection extends Struct {
        readonly amount: u128;
        readonly direction: PalletNftsPriceDirection;
    }
    /** @name PalletNftsPriceDirection (197) */
    interface PalletNftsPriceDirection extends Enum {
        readonly isSend: boolean;
        readonly isReceive: boolean;
        readonly type: 'Send' | 'Receive';
    }
    /** @name PalletNftsPalletAttributes (198) */
    interface PalletNftsPalletAttributes extends Enum {
        readonly isUsedToClaim: boolean;
        readonly asUsedToClaim: u32;
        readonly isTransferDisabled: boolean;
        readonly type: 'UsedToClaim' | 'TransferDisabled';
    }
    /** @name PalletParametersEvent (199) */
    interface PalletParametersEvent extends Enum {
        readonly isUpdated: boolean;
        readonly asUpdated: {
            readonly key: ShParachainRuntimeConfigsRuntimeParamsRuntimeParametersKey;
            readonly oldValue: Option<ShParachainRuntimeConfigsRuntimeParamsRuntimeParametersValue>;
            readonly newValue: Option<ShParachainRuntimeConfigsRuntimeParamsRuntimeParametersValue>;
        } & Struct;
        readonly type: 'Updated';
    }
    /** @name ShParachainRuntimeConfigsRuntimeParamsRuntimeParametersKey (200) */
    interface ShParachainRuntimeConfigsRuntimeParamsRuntimeParametersKey extends Enum {
        readonly isRuntimeConfig: boolean;
        readonly asRuntimeConfig: ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigParametersKey;
        readonly type: 'RuntimeConfig';
    }
    /** @name ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigParametersKey (201) */
    interface ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigParametersKey extends Enum {
        readonly isSlashAmountPerMaxFileSize: boolean;
        readonly isStakeToChallengePeriod: boolean;
        readonly isCheckpointChallengePeriod: boolean;
        readonly isMinChallengePeriod: boolean;
        readonly isSystemUtilisationLowerThresholdPercentage: boolean;
        readonly isSystemUtilisationUpperThresholdPercentage: boolean;
        readonly isMostlyStablePrice: boolean;
        readonly isMaxPrice: boolean;
        readonly isMinPrice: boolean;
        readonly isUpperExponentFactor: boolean;
        readonly isLowerExponentFactor: boolean;
        readonly isZeroSizeBucketFixedRate: boolean;
        readonly isIdealUtilisationRate: boolean;
        readonly isDecayRate: boolean;
        readonly isMinimumTreasuryCut: boolean;
        readonly isMaximumTreasuryCut: boolean;
        readonly isBspStopStoringFilePenalty: boolean;
        readonly isProviderTopUpTtl: boolean;
        readonly isBasicReplicationTarget: boolean;
        readonly isStandardReplicationTarget: boolean;
        readonly isHighSecurityReplicationTarget: boolean;
        readonly isSuperHighSecurityReplicationTarget: boolean;
        readonly isUltraHighSecurityReplicationTarget: boolean;
        readonly isMaxReplicationTarget: boolean;
        readonly isTickRangeToMaximumThreshold: boolean;
        readonly isStorageRequestTtl: boolean;
        readonly isMinWaitForStopStoring: boolean;
        readonly isMinSeedPeriod: boolean;
        readonly isStakeToSeedPeriod: boolean;
        readonly isUpfrontTicksToPay: boolean;
        readonly type: 'SlashAmountPerMaxFileSize' | 'StakeToChallengePeriod' | 'CheckpointChallengePeriod' | 'MinChallengePeriod' | 'SystemUtilisationLowerThresholdPercentage' | 'SystemUtilisationUpperThresholdPercentage' | 'MostlyStablePrice' | 'MaxPrice' | 'MinPrice' | 'UpperExponentFactor' | 'LowerExponentFactor' | 'ZeroSizeBucketFixedRate' | 'IdealUtilisationRate' | 'DecayRate' | 'MinimumTreasuryCut' | 'MaximumTreasuryCut' | 'BspStopStoringFilePenalty' | 'ProviderTopUpTtl' | 'BasicReplicationTarget' | 'StandardReplicationTarget' | 'HighSecurityReplicationTarget' | 'SuperHighSecurityReplicationTarget' | 'UltraHighSecurityReplicationTarget' | 'MaxReplicationTarget' | 'TickRangeToMaximumThreshold' | 'StorageRequestTtl' | 'MinWaitForStopStoring' | 'MinSeedPeriod' | 'StakeToSeedPeriod' | 'UpfrontTicksToPay';
    }
    /** @name ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigSlashAmountPerMaxFileSize (202) */
    type ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigSlashAmountPerMaxFileSize = Null;
    /** @name ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigStakeToChallengePeriod (203) */
    type ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigStakeToChallengePeriod = Null;
    /** @name ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigCheckpointChallengePeriod (204) */
    type ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigCheckpointChallengePeriod = Null;
    /** @name ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMinChallengePeriod (205) */
    type ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMinChallengePeriod = Null;
    /** @name ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigSystemUtilisationLowerThresholdPercentage (206) */
    type ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigSystemUtilisationLowerThresholdPercentage = Null;
    /** @name ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigSystemUtilisationUpperThresholdPercentage (207) */
    type ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigSystemUtilisationUpperThresholdPercentage = Null;
    /** @name ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMostlyStablePrice (208) */
    type ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMostlyStablePrice = Null;
    /** @name ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMaxPrice (209) */
    type ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMaxPrice = Null;
    /** @name ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMinPrice (210) */
    type ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMinPrice = Null;
    /** @name ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigUpperExponentFactor (211) */
    type ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigUpperExponentFactor = Null;
    /** @name ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigLowerExponentFactor (212) */
    type ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigLowerExponentFactor = Null;
    /** @name ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigZeroSizeBucketFixedRate (213) */
    type ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigZeroSizeBucketFixedRate = Null;
    /** @name ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigIdealUtilisationRate (214) */
    type ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigIdealUtilisationRate = Null;
    /** @name ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigDecayRate (215) */
    type ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigDecayRate = Null;
    /** @name ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMinimumTreasuryCut (216) */
    type ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMinimumTreasuryCut = Null;
    /** @name ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMaximumTreasuryCut (217) */
    type ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMaximumTreasuryCut = Null;
    /** @name ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigBspStopStoringFilePenalty (218) */
    type ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigBspStopStoringFilePenalty = Null;
    /** @name ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigProviderTopUpTtl (219) */
    type ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigProviderTopUpTtl = Null;
    /** @name ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigBasicReplicationTarget (220) */
    type ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigBasicReplicationTarget = Null;
    /** @name ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigStandardReplicationTarget (221) */
    type ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigStandardReplicationTarget = Null;
    /** @name ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigHighSecurityReplicationTarget (222) */
    type ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigHighSecurityReplicationTarget = Null;
    /** @name ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigSuperHighSecurityReplicationTarget (223) */
    type ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigSuperHighSecurityReplicationTarget = Null;
    /** @name ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigUltraHighSecurityReplicationTarget (224) */
    type ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigUltraHighSecurityReplicationTarget = Null;
    /** @name ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMaxReplicationTarget (225) */
    type ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMaxReplicationTarget = Null;
    /** @name ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigTickRangeToMaximumThreshold (226) */
    type ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigTickRangeToMaximumThreshold = Null;
    /** @name ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigStorageRequestTtl (227) */
    type ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigStorageRequestTtl = Null;
    /** @name ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMinWaitForStopStoring (228) */
    type ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMinWaitForStopStoring = Null;
    /** @name ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMinSeedPeriod (229) */
    type ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMinSeedPeriod = Null;
    /** @name ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigStakeToSeedPeriod (230) */
    type ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigStakeToSeedPeriod = Null;
    /** @name ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigUpfrontTicksToPay (231) */
    type ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigUpfrontTicksToPay = Null;
    /** @name ShParachainRuntimeConfigsRuntimeParamsRuntimeParametersValue (233) */
    interface ShParachainRuntimeConfigsRuntimeParamsRuntimeParametersValue extends Enum {
        readonly isRuntimeConfig: boolean;
        readonly asRuntimeConfig: ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigParametersValue;
        readonly type: 'RuntimeConfig';
    }
    /** @name ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigParametersValue (234) */
    interface ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigParametersValue extends Enum {
        readonly isSlashAmountPerMaxFileSize: boolean;
        readonly asSlashAmountPerMaxFileSize: u128;
        readonly isStakeToChallengePeriod: boolean;
        readonly asStakeToChallengePeriod: u128;
        readonly isCheckpointChallengePeriod: boolean;
        readonly asCheckpointChallengePeriod: u32;
        readonly isMinChallengePeriod: boolean;
        readonly asMinChallengePeriod: u32;
        readonly isSystemUtilisationLowerThresholdPercentage: boolean;
        readonly asSystemUtilisationLowerThresholdPercentage: Perbill;
        readonly isSystemUtilisationUpperThresholdPercentage: boolean;
        readonly asSystemUtilisationUpperThresholdPercentage: Perbill;
        readonly isMostlyStablePrice: boolean;
        readonly asMostlyStablePrice: u128;
        readonly isMaxPrice: boolean;
        readonly asMaxPrice: u128;
        readonly isMinPrice: boolean;
        readonly asMinPrice: u128;
        readonly isUpperExponentFactor: boolean;
        readonly asUpperExponentFactor: u32;
        readonly isLowerExponentFactor: boolean;
        readonly asLowerExponentFactor: u32;
        readonly isZeroSizeBucketFixedRate: boolean;
        readonly asZeroSizeBucketFixedRate: u128;
        readonly isIdealUtilisationRate: boolean;
        readonly asIdealUtilisationRate: Perbill;
        readonly isDecayRate: boolean;
        readonly asDecayRate: Perbill;
        readonly isMinimumTreasuryCut: boolean;
        readonly asMinimumTreasuryCut: Perbill;
        readonly isMaximumTreasuryCut: boolean;
        readonly asMaximumTreasuryCut: Perbill;
        readonly isBspStopStoringFilePenalty: boolean;
        readonly asBspStopStoringFilePenalty: u128;
        readonly isProviderTopUpTtl: boolean;
        readonly asProviderTopUpTtl: u32;
        readonly isBasicReplicationTarget: boolean;
        readonly asBasicReplicationTarget: u32;
        readonly isStandardReplicationTarget: boolean;
        readonly asStandardReplicationTarget: u32;
        readonly isHighSecurityReplicationTarget: boolean;
        readonly asHighSecurityReplicationTarget: u32;
        readonly isSuperHighSecurityReplicationTarget: boolean;
        readonly asSuperHighSecurityReplicationTarget: u32;
        readonly isUltraHighSecurityReplicationTarget: boolean;
        readonly asUltraHighSecurityReplicationTarget: u32;
        readonly isMaxReplicationTarget: boolean;
        readonly asMaxReplicationTarget: u32;
        readonly isTickRangeToMaximumThreshold: boolean;
        readonly asTickRangeToMaximumThreshold: u32;
        readonly isStorageRequestTtl: boolean;
        readonly asStorageRequestTtl: u32;
        readonly isMinWaitForStopStoring: boolean;
        readonly asMinWaitForStopStoring: u32;
        readonly isMinSeedPeriod: boolean;
        readonly asMinSeedPeriod: u32;
        readonly isStakeToSeedPeriod: boolean;
        readonly asStakeToSeedPeriod: u128;
        readonly isUpfrontTicksToPay: boolean;
        readonly asUpfrontTicksToPay: u32;
        readonly type: 'SlashAmountPerMaxFileSize' | 'StakeToChallengePeriod' | 'CheckpointChallengePeriod' | 'MinChallengePeriod' | 'SystemUtilisationLowerThresholdPercentage' | 'SystemUtilisationUpperThresholdPercentage' | 'MostlyStablePrice' | 'MaxPrice' | 'MinPrice' | 'UpperExponentFactor' | 'LowerExponentFactor' | 'ZeroSizeBucketFixedRate' | 'IdealUtilisationRate' | 'DecayRate' | 'MinimumTreasuryCut' | 'MaximumTreasuryCut' | 'BspStopStoringFilePenalty' | 'ProviderTopUpTtl' | 'BasicReplicationTarget' | 'StandardReplicationTarget' | 'HighSecurityReplicationTarget' | 'SuperHighSecurityReplicationTarget' | 'UltraHighSecurityReplicationTarget' | 'MaxReplicationTarget' | 'TickRangeToMaximumThreshold' | 'StorageRequestTtl' | 'MinWaitForStopStoring' | 'MinSeedPeriod' | 'StakeToSeedPeriod' | 'UpfrontTicksToPay';
    }
    /** @name FrameSystemPhase (236) */
    interface FrameSystemPhase extends Enum {
        readonly isApplyExtrinsic: boolean;
        readonly asApplyExtrinsic: u32;
        readonly isFinalization: boolean;
        readonly isInitialization: boolean;
        readonly type: 'ApplyExtrinsic' | 'Finalization' | 'Initialization';
    }
    /** @name FrameSystemLastRuntimeUpgradeInfo (239) */
    interface FrameSystemLastRuntimeUpgradeInfo extends Struct {
        readonly specVersion: Compact<u32>;
        readonly specName: Text;
    }
    /** @name FrameSystemCodeUpgradeAuthorization (242) */
    interface FrameSystemCodeUpgradeAuthorization extends Struct {
        readonly codeHash: H256;
        readonly checkVersion: bool;
    }
    /** @name FrameSystemCall (243) */
    interface FrameSystemCall extends Enum {
        readonly isRemark: boolean;
        readonly asRemark: {
            readonly remark: Bytes;
        } & Struct;
        readonly isSetHeapPages: boolean;
        readonly asSetHeapPages: {
            readonly pages: u64;
        } & Struct;
        readonly isSetCode: boolean;
        readonly asSetCode: {
            readonly code: Bytes;
        } & Struct;
        readonly isSetCodeWithoutChecks: boolean;
        readonly asSetCodeWithoutChecks: {
            readonly code: Bytes;
        } & Struct;
        readonly isSetStorage: boolean;
        readonly asSetStorage: {
            readonly items: Vec<ITuple<[Bytes, Bytes]>>;
        } & Struct;
        readonly isKillStorage: boolean;
        readonly asKillStorage: {
            readonly keys_: Vec<Bytes>;
        } & Struct;
        readonly isKillPrefix: boolean;
        readonly asKillPrefix: {
            readonly prefix: Bytes;
            readonly subkeys: u32;
        } & Struct;
        readonly isRemarkWithEvent: boolean;
        readonly asRemarkWithEvent: {
            readonly remark: Bytes;
        } & Struct;
        readonly isAuthorizeUpgrade: boolean;
        readonly asAuthorizeUpgrade: {
            readonly codeHash: H256;
        } & Struct;
        readonly isAuthorizeUpgradeWithoutChecks: boolean;
        readonly asAuthorizeUpgradeWithoutChecks: {
            readonly codeHash: H256;
        } & Struct;
        readonly isApplyAuthorizedUpgrade: boolean;
        readonly asApplyAuthorizedUpgrade: {
            readonly code: Bytes;
        } & Struct;
        readonly type: 'Remark' | 'SetHeapPages' | 'SetCode' | 'SetCodeWithoutChecks' | 'SetStorage' | 'KillStorage' | 'KillPrefix' | 'RemarkWithEvent' | 'AuthorizeUpgrade' | 'AuthorizeUpgradeWithoutChecks' | 'ApplyAuthorizedUpgrade';
    }
    /** @name FrameSystemLimitsBlockWeights (246) */
    interface FrameSystemLimitsBlockWeights extends Struct {
        readonly baseBlock: SpWeightsWeightV2Weight;
        readonly maxBlock: SpWeightsWeightV2Weight;
        readonly perClass: FrameSupportDispatchPerDispatchClassWeightsPerClass;
    }
    /** @name FrameSupportDispatchPerDispatchClassWeightsPerClass (247) */
    interface FrameSupportDispatchPerDispatchClassWeightsPerClass extends Struct {
        readonly normal: FrameSystemLimitsWeightsPerClass;
        readonly operational: FrameSystemLimitsWeightsPerClass;
        readonly mandatory: FrameSystemLimitsWeightsPerClass;
    }
    /** @name FrameSystemLimitsWeightsPerClass (248) */
    interface FrameSystemLimitsWeightsPerClass extends Struct {
        readonly baseExtrinsic: SpWeightsWeightV2Weight;
        readonly maxExtrinsic: Option<SpWeightsWeightV2Weight>;
        readonly maxTotal: Option<SpWeightsWeightV2Weight>;
        readonly reserved: Option<SpWeightsWeightV2Weight>;
    }
    /** @name FrameSystemLimitsBlockLength (249) */
    interface FrameSystemLimitsBlockLength extends Struct {
        readonly max: FrameSupportDispatchPerDispatchClassU32;
    }
    /** @name FrameSupportDispatchPerDispatchClassU32 (250) */
    interface FrameSupportDispatchPerDispatchClassU32 extends Struct {
        readonly normal: u32;
        readonly operational: u32;
        readonly mandatory: u32;
    }
    /** @name SpWeightsRuntimeDbWeight (251) */
    interface SpWeightsRuntimeDbWeight extends Struct {
        readonly read: u64;
        readonly write: u64;
    }
    /** @name SpVersionRuntimeVersion (252) */
    interface SpVersionRuntimeVersion extends Struct {
        readonly specName: Text;
        readonly implName: Text;
        readonly authoringVersion: u32;
        readonly specVersion: u32;
        readonly implVersion: u32;
        readonly apis: Vec<ITuple<[U8aFixed, u32]>>;
        readonly transactionVersion: u32;
        readonly systemVersion: u8;
    }
    /** @name FrameSystemError (257) */
    interface FrameSystemError extends Enum {
        readonly isInvalidSpecName: boolean;
        readonly isSpecVersionNeedsToIncrease: boolean;
        readonly isFailedToExtractRuntimeVersion: boolean;
        readonly isNonDefaultComposite: boolean;
        readonly isNonZeroRefCount: boolean;
        readonly isCallFiltered: boolean;
        readonly isMultiBlockMigrationsOngoing: boolean;
        readonly isNothingAuthorized: boolean;
        readonly isUnauthorized: boolean;
        readonly type: 'InvalidSpecName' | 'SpecVersionNeedsToIncrease' | 'FailedToExtractRuntimeVersion' | 'NonDefaultComposite' | 'NonZeroRefCount' | 'CallFiltered' | 'MultiBlockMigrationsOngoing' | 'NothingAuthorized' | 'Unauthorized';
    }
    /** @name CumulusPalletParachainSystemUnincludedSegmentAncestor (259) */
    interface CumulusPalletParachainSystemUnincludedSegmentAncestor extends Struct {
        readonly usedBandwidth: CumulusPalletParachainSystemUnincludedSegmentUsedBandwidth;
        readonly paraHeadHash: Option<H256>;
        readonly consumedGoAheadSignal: Option<PolkadotPrimitivesV8UpgradeGoAhead>;
    }
    /** @name CumulusPalletParachainSystemUnincludedSegmentUsedBandwidth (260) */
    interface CumulusPalletParachainSystemUnincludedSegmentUsedBandwidth extends Struct {
        readonly umpMsgCount: u32;
        readonly umpTotalBytes: u32;
        readonly hrmpOutgoing: BTreeMap<u32, CumulusPalletParachainSystemUnincludedSegmentHrmpChannelUpdate>;
    }
    /** @name CumulusPalletParachainSystemUnincludedSegmentHrmpChannelUpdate (262) */
    interface CumulusPalletParachainSystemUnincludedSegmentHrmpChannelUpdate extends Struct {
        readonly msgCount: u32;
        readonly totalBytes: u32;
    }
    /** @name PolkadotPrimitivesV8UpgradeGoAhead (266) */
    interface PolkadotPrimitivesV8UpgradeGoAhead extends Enum {
        readonly isAbort: boolean;
        readonly isGoAhead: boolean;
        readonly type: 'Abort' | 'GoAhead';
    }
    /** @name CumulusPalletParachainSystemUnincludedSegmentSegmentTracker (267) */
    interface CumulusPalletParachainSystemUnincludedSegmentSegmentTracker extends Struct {
        readonly usedBandwidth: CumulusPalletParachainSystemUnincludedSegmentUsedBandwidth;
        readonly hrmpWatermark: Option<u32>;
        readonly consumedGoAheadSignal: Option<PolkadotPrimitivesV8UpgradeGoAhead>;
    }
    /** @name PolkadotPrimitivesV8PersistedValidationData (268) */
    interface PolkadotPrimitivesV8PersistedValidationData extends Struct {
        readonly parentHead: Bytes;
        readonly relayParentNumber: u32;
        readonly relayParentStorageRoot: H256;
        readonly maxPovSize: u32;
    }
    /** @name PolkadotPrimitivesV8UpgradeRestriction (271) */
    interface PolkadotPrimitivesV8UpgradeRestriction extends Enum {
        readonly isPresent: boolean;
        readonly type: 'Present';
    }
    /** @name SpTrieStorageProof (272) */
    interface SpTrieStorageProof extends Struct {
        readonly trieNodes: BTreeSet<Bytes>;
    }
    /** @name CumulusPalletParachainSystemRelayStateSnapshotMessagingStateSnapshot (274) */
    interface CumulusPalletParachainSystemRelayStateSnapshotMessagingStateSnapshot extends Struct {
        readonly dmqMqcHead: H256;
        readonly relayDispatchQueueRemainingCapacity: CumulusPalletParachainSystemRelayStateSnapshotRelayDispatchQueueRemainingCapacity;
        readonly ingressChannels: Vec<ITuple<[u32, PolkadotPrimitivesV8AbridgedHrmpChannel]>>;
        readonly egressChannels: Vec<ITuple<[u32, PolkadotPrimitivesV8AbridgedHrmpChannel]>>;
    }
    /** @name CumulusPalletParachainSystemRelayStateSnapshotRelayDispatchQueueRemainingCapacity (275) */
    interface CumulusPalletParachainSystemRelayStateSnapshotRelayDispatchQueueRemainingCapacity extends Struct {
        readonly remainingCount: u32;
        readonly remainingSize: u32;
    }
    /** @name PolkadotPrimitivesV8AbridgedHrmpChannel (278) */
    interface PolkadotPrimitivesV8AbridgedHrmpChannel extends Struct {
        readonly maxCapacity: u32;
        readonly maxTotalSize: u32;
        readonly maxMessageSize: u32;
        readonly msgCount: u32;
        readonly totalSize: u32;
        readonly mqcHead: Option<H256>;
    }
    /** @name PolkadotPrimitivesV8AbridgedHostConfiguration (279) */
    interface PolkadotPrimitivesV8AbridgedHostConfiguration extends Struct {
        readonly maxCodeSize: u32;
        readonly maxHeadDataSize: u32;
        readonly maxUpwardQueueCount: u32;
        readonly maxUpwardQueueSize: u32;
        readonly maxUpwardMessageSize: u32;
        readonly maxUpwardMessageNumPerCandidate: u32;
        readonly hrmpMaxMessageNumPerCandidate: u32;
        readonly validationUpgradeCooldown: u32;
        readonly validationUpgradeDelay: u32;
        readonly asyncBackingParams: PolkadotPrimitivesV8AsyncBackingAsyncBackingParams;
    }
    /** @name PolkadotPrimitivesV8AsyncBackingAsyncBackingParams (280) */
    interface PolkadotPrimitivesV8AsyncBackingAsyncBackingParams extends Struct {
        readonly maxCandidateDepth: u32;
        readonly allowedAncestryLen: u32;
    }
    /** @name PolkadotCorePrimitivesOutboundHrmpMessage (286) */
    interface PolkadotCorePrimitivesOutboundHrmpMessage extends Struct {
        readonly recipient: u32;
        readonly data: Bytes;
    }
    /** @name CumulusPalletParachainSystemCall (288) */
    interface CumulusPalletParachainSystemCall extends Enum {
        readonly isSetValidationData: boolean;
        readonly asSetValidationData: {
            readonly data: CumulusPrimitivesParachainInherentParachainInherentData;
        } & Struct;
        readonly isSudoSendUpwardMessage: boolean;
        readonly asSudoSendUpwardMessage: {
            readonly message: Bytes;
        } & Struct;
        readonly type: 'SetValidationData' | 'SudoSendUpwardMessage';
    }
    /** @name CumulusPrimitivesParachainInherentParachainInherentData (289) */
    interface CumulusPrimitivesParachainInherentParachainInherentData extends Struct {
        readonly validationData: PolkadotPrimitivesV8PersistedValidationData;
        readonly relayChainState: SpTrieStorageProof;
        readonly downwardMessages: Vec<PolkadotCorePrimitivesInboundDownwardMessage>;
        readonly horizontalMessages: BTreeMap<u32, Vec<PolkadotCorePrimitivesInboundHrmpMessage>>;
    }
    /** @name PolkadotCorePrimitivesInboundDownwardMessage (291) */
    interface PolkadotCorePrimitivesInboundDownwardMessage extends Struct {
        readonly sentAt: u32;
        readonly msg: Bytes;
    }
    /** @name PolkadotCorePrimitivesInboundHrmpMessage (294) */
    interface PolkadotCorePrimitivesInboundHrmpMessage extends Struct {
        readonly sentAt: u32;
        readonly data: Bytes;
    }
    /** @name CumulusPalletParachainSystemError (297) */
    interface CumulusPalletParachainSystemError extends Enum {
        readonly isOverlappingUpgrades: boolean;
        readonly isProhibitedByPolkadot: boolean;
        readonly isTooBig: boolean;
        readonly isValidationDataNotAvailable: boolean;
        readonly isHostConfigurationNotAvailable: boolean;
        readonly isNotScheduled: boolean;
        readonly isNothingAuthorized: boolean;
        readonly isUnauthorized: boolean;
        readonly type: 'OverlappingUpgrades' | 'ProhibitedByPolkadot' | 'TooBig' | 'ValidationDataNotAvailable' | 'HostConfigurationNotAvailable' | 'NotScheduled' | 'NothingAuthorized' | 'Unauthorized';
    }
    /** @name PalletTimestampCall (298) */
    interface PalletTimestampCall extends Enum {
        readonly isSet: boolean;
        readonly asSet: {
            readonly now: Compact<u64>;
        } & Struct;
        readonly type: 'Set';
    }
    /** @name StagingParachainInfoCall (299) */
    type StagingParachainInfoCall = Null;
    /** @name PalletBalancesBalanceLock (301) */
    interface PalletBalancesBalanceLock extends Struct {
        readonly id: U8aFixed;
        readonly amount: u128;
        readonly reasons: PalletBalancesReasons;
    }
    /** @name PalletBalancesReasons (302) */
    interface PalletBalancesReasons extends Enum {
        readonly isFee: boolean;
        readonly isMisc: boolean;
        readonly isAll: boolean;
        readonly type: 'Fee' | 'Misc' | 'All';
    }
    /** @name PalletBalancesReserveData (305) */
    interface PalletBalancesReserveData extends Struct {
        readonly id: U8aFixed;
        readonly amount: u128;
    }
    /** @name ShParachainRuntimeRuntimeHoldReason (309) */
    interface ShParachainRuntimeRuntimeHoldReason extends Enum {
        readonly isProviders: boolean;
        readonly asProviders: PalletStorageProvidersHoldReason;
        readonly isFileSystem: boolean;
        readonly asFileSystem: PalletFileSystemHoldReason;
        readonly isPaymentStreams: boolean;
        readonly asPaymentStreams: PalletPaymentStreamsHoldReason;
        readonly type: 'Providers' | 'FileSystem' | 'PaymentStreams';
    }
    /** @name PalletStorageProvidersHoldReason (310) */
    interface PalletStorageProvidersHoldReason extends Enum {
        readonly isStorageProviderDeposit: boolean;
        readonly isBucketDeposit: boolean;
        readonly type: 'StorageProviderDeposit' | 'BucketDeposit';
    }
    /** @name PalletFileSystemHoldReason (311) */
    interface PalletFileSystemHoldReason extends Enum {
        readonly isStorageRequestCreationHold: boolean;
        readonly isFileDeletionRequestHold: boolean;
        readonly type: 'StorageRequestCreationHold' | 'FileDeletionRequestHold';
    }
    /** @name PalletPaymentStreamsHoldReason (312) */
    interface PalletPaymentStreamsHoldReason extends Enum {
        readonly isPaymentStreamDeposit: boolean;
        readonly type: 'PaymentStreamDeposit';
    }
    /** @name FrameSupportTokensMiscIdAmount (315) */
    interface FrameSupportTokensMiscIdAmount extends Struct {
        readonly id: Null;
        readonly amount: u128;
    }
    /** @name PalletBalancesCall (317) */
    interface PalletBalancesCall extends Enum {
        readonly isTransferAllowDeath: boolean;
        readonly asTransferAllowDeath: {
            readonly dest: MultiAddress;
            readonly value: Compact<u128>;
        } & Struct;
        readonly isForceTransfer: boolean;
        readonly asForceTransfer: {
            readonly source: MultiAddress;
            readonly dest: MultiAddress;
            readonly value: Compact<u128>;
        } & Struct;
        readonly isTransferKeepAlive: boolean;
        readonly asTransferKeepAlive: {
            readonly dest: MultiAddress;
            readonly value: Compact<u128>;
        } & Struct;
        readonly isTransferAll: boolean;
        readonly asTransferAll: {
            readonly dest: MultiAddress;
            readonly keepAlive: bool;
        } & Struct;
        readonly isForceUnreserve: boolean;
        readonly asForceUnreserve: {
            readonly who: MultiAddress;
            readonly amount: u128;
        } & Struct;
        readonly isUpgradeAccounts: boolean;
        readonly asUpgradeAccounts: {
            readonly who: Vec<AccountId32>;
        } & Struct;
        readonly isForceSetBalance: boolean;
        readonly asForceSetBalance: {
            readonly who: MultiAddress;
            readonly newFree: Compact<u128>;
        } & Struct;
        readonly isForceAdjustTotalIssuance: boolean;
        readonly asForceAdjustTotalIssuance: {
            readonly direction: PalletBalancesAdjustmentDirection;
            readonly delta: Compact<u128>;
        } & Struct;
        readonly isBurn: boolean;
        readonly asBurn: {
            readonly value: Compact<u128>;
            readonly keepAlive: bool;
        } & Struct;
        readonly type: 'TransferAllowDeath' | 'ForceTransfer' | 'TransferKeepAlive' | 'TransferAll' | 'ForceUnreserve' | 'UpgradeAccounts' | 'ForceSetBalance' | 'ForceAdjustTotalIssuance' | 'Burn';
    }
    /** @name PalletBalancesAdjustmentDirection (320) */
    interface PalletBalancesAdjustmentDirection extends Enum {
        readonly isIncrease: boolean;
        readonly isDecrease: boolean;
        readonly type: 'Increase' | 'Decrease';
    }
    /** @name PalletBalancesError (321) */
    interface PalletBalancesError extends Enum {
        readonly isVestingBalance: boolean;
        readonly isLiquidityRestrictions: boolean;
        readonly isInsufficientBalance: boolean;
        readonly isExistentialDeposit: boolean;
        readonly isExpendability: boolean;
        readonly isExistingVestingSchedule: boolean;
        readonly isDeadAccount: boolean;
        readonly isTooManyReserves: boolean;
        readonly isTooManyHolds: boolean;
        readonly isTooManyFreezes: boolean;
        readonly isIssuanceDeactivated: boolean;
        readonly isDeltaZero: boolean;
        readonly type: 'VestingBalance' | 'LiquidityRestrictions' | 'InsufficientBalance' | 'ExistentialDeposit' | 'Expendability' | 'ExistingVestingSchedule' | 'DeadAccount' | 'TooManyReserves' | 'TooManyHolds' | 'TooManyFreezes' | 'IssuanceDeactivated' | 'DeltaZero';
    }
    /** @name PalletTransactionPaymentReleases (322) */
    interface PalletTransactionPaymentReleases extends Enum {
        readonly isV1Ancient: boolean;
        readonly isV2: boolean;
        readonly type: 'V1Ancient' | 'V2';
    }
    /** @name PalletSudoCall (323) */
    interface PalletSudoCall extends Enum {
        readonly isSudo: boolean;
        readonly asSudo: {
            readonly call: Call;
        } & Struct;
        readonly isSudoUncheckedWeight: boolean;
        readonly asSudoUncheckedWeight: {
            readonly call: Call;
            readonly weight: SpWeightsWeightV2Weight;
        } & Struct;
        readonly isSetKey: boolean;
        readonly asSetKey: {
            readonly new_: MultiAddress;
        } & Struct;
        readonly isSudoAs: boolean;
        readonly asSudoAs: {
            readonly who: MultiAddress;
            readonly call: Call;
        } & Struct;
        readonly isRemoveKey: boolean;
        readonly type: 'Sudo' | 'SudoUncheckedWeight' | 'SetKey' | 'SudoAs' | 'RemoveKey';
    }
    /** @name PalletCollatorSelectionCall (325) */
    interface PalletCollatorSelectionCall extends Enum {
        readonly isSetInvulnerables: boolean;
        readonly asSetInvulnerables: {
            readonly new_: Vec<AccountId32>;
        } & Struct;
        readonly isSetDesiredCandidates: boolean;
        readonly asSetDesiredCandidates: {
            readonly max: u32;
        } & Struct;
        readonly isSetCandidacyBond: boolean;
        readonly asSetCandidacyBond: {
            readonly bond: u128;
        } & Struct;
        readonly isRegisterAsCandidate: boolean;
        readonly isLeaveIntent: boolean;
        readonly isAddInvulnerable: boolean;
        readonly asAddInvulnerable: {
            readonly who: AccountId32;
        } & Struct;
        readonly isRemoveInvulnerable: boolean;
        readonly asRemoveInvulnerable: {
            readonly who: AccountId32;
        } & Struct;
        readonly isUpdateBond: boolean;
        readonly asUpdateBond: {
            readonly newDeposit: u128;
        } & Struct;
        readonly isTakeCandidateSlot: boolean;
        readonly asTakeCandidateSlot: {
            readonly deposit: u128;
            readonly target: AccountId32;
        } & Struct;
        readonly type: 'SetInvulnerables' | 'SetDesiredCandidates' | 'SetCandidacyBond' | 'RegisterAsCandidate' | 'LeaveIntent' | 'AddInvulnerable' | 'RemoveInvulnerable' | 'UpdateBond' | 'TakeCandidateSlot';
    }
    /** @name PalletSessionCall (326) */
    interface PalletSessionCall extends Enum {
        readonly isSetKeys: boolean;
        readonly asSetKeys: {
            readonly keys_: ShParachainRuntimeSessionKeys;
            readonly proof: Bytes;
        } & Struct;
        readonly isPurgeKeys: boolean;
        readonly type: 'SetKeys' | 'PurgeKeys';
    }
    /** @name ShParachainRuntimeSessionKeys (327) */
    interface ShParachainRuntimeSessionKeys extends Struct {
        readonly aura: SpConsensusAuraSr25519AppSr25519Public;
    }
    /** @name SpConsensusAuraSr25519AppSr25519Public (328) */
    interface SpConsensusAuraSr25519AppSr25519Public extends U8aFixed {
    }
    /** @name CumulusPalletXcmpQueueCall (329) */
    interface CumulusPalletXcmpQueueCall extends Enum {
        readonly isSuspendXcmExecution: boolean;
        readonly isResumeXcmExecution: boolean;
        readonly isUpdateSuspendThreshold: boolean;
        readonly asUpdateSuspendThreshold: {
            readonly new_: u32;
        } & Struct;
        readonly isUpdateDropThreshold: boolean;
        readonly asUpdateDropThreshold: {
            readonly new_: u32;
        } & Struct;
        readonly isUpdateResumeThreshold: boolean;
        readonly asUpdateResumeThreshold: {
            readonly new_: u32;
        } & Struct;
        readonly type: 'SuspendXcmExecution' | 'ResumeXcmExecution' | 'UpdateSuspendThreshold' | 'UpdateDropThreshold' | 'UpdateResumeThreshold';
    }
    /** @name PalletXcmCall (330) */
    interface PalletXcmCall extends Enum {
        readonly isSend: boolean;
        readonly asSend: {
            readonly dest: XcmVersionedLocation;
            readonly message: XcmVersionedXcm;
        } & Struct;
        readonly isTeleportAssets: boolean;
        readonly asTeleportAssets: {
            readonly dest: XcmVersionedLocation;
            readonly beneficiary: XcmVersionedLocation;
            readonly assets: XcmVersionedAssets;
            readonly feeAssetItem: u32;
        } & Struct;
        readonly isReserveTransferAssets: boolean;
        readonly asReserveTransferAssets: {
            readonly dest: XcmVersionedLocation;
            readonly beneficiary: XcmVersionedLocation;
            readonly assets: XcmVersionedAssets;
            readonly feeAssetItem: u32;
        } & Struct;
        readonly isExecute: boolean;
        readonly asExecute: {
            readonly message: XcmVersionedXcm;
            readonly maxWeight: SpWeightsWeightV2Weight;
        } & Struct;
        readonly isForceXcmVersion: boolean;
        readonly asForceXcmVersion: {
            readonly location: StagingXcmV5Location;
            readonly version: u32;
        } & Struct;
        readonly isForceDefaultXcmVersion: boolean;
        readonly asForceDefaultXcmVersion: {
            readonly maybeXcmVersion: Option<u32>;
        } & Struct;
        readonly isForceSubscribeVersionNotify: boolean;
        readonly asForceSubscribeVersionNotify: {
            readonly location: XcmVersionedLocation;
        } & Struct;
        readonly isForceUnsubscribeVersionNotify: boolean;
        readonly asForceUnsubscribeVersionNotify: {
            readonly location: XcmVersionedLocation;
        } & Struct;
        readonly isLimitedReserveTransferAssets: boolean;
        readonly asLimitedReserveTransferAssets: {
            readonly dest: XcmVersionedLocation;
            readonly beneficiary: XcmVersionedLocation;
            readonly assets: XcmVersionedAssets;
            readonly feeAssetItem: u32;
            readonly weightLimit: XcmV3WeightLimit;
        } & Struct;
        readonly isLimitedTeleportAssets: boolean;
        readonly asLimitedTeleportAssets: {
            readonly dest: XcmVersionedLocation;
            readonly beneficiary: XcmVersionedLocation;
            readonly assets: XcmVersionedAssets;
            readonly feeAssetItem: u32;
            readonly weightLimit: XcmV3WeightLimit;
        } & Struct;
        readonly isForceSuspension: boolean;
        readonly asForceSuspension: {
            readonly suspended: bool;
        } & Struct;
        readonly isTransferAssets: boolean;
        readonly asTransferAssets: {
            readonly dest: XcmVersionedLocation;
            readonly beneficiary: XcmVersionedLocation;
            readonly assets: XcmVersionedAssets;
            readonly feeAssetItem: u32;
            readonly weightLimit: XcmV3WeightLimit;
        } & Struct;
        readonly isClaimAssets: boolean;
        readonly asClaimAssets: {
            readonly assets: XcmVersionedAssets;
            readonly beneficiary: XcmVersionedLocation;
        } & Struct;
        readonly isTransferAssetsUsingTypeAndThen: boolean;
        readonly asTransferAssetsUsingTypeAndThen: {
            readonly dest: XcmVersionedLocation;
            readonly assets: XcmVersionedAssets;
            readonly assetsTransferType: StagingXcmExecutorAssetTransferTransferType;
            readonly remoteFeesId: XcmVersionedAssetId;
            readonly feesTransferType: StagingXcmExecutorAssetTransferTransferType;
            readonly customXcmOnDest: XcmVersionedXcm;
            readonly weightLimit: XcmV3WeightLimit;
        } & Struct;
        readonly type: 'Send' | 'TeleportAssets' | 'ReserveTransferAssets' | 'Execute' | 'ForceXcmVersion' | 'ForceDefaultXcmVersion' | 'ForceSubscribeVersionNotify' | 'ForceUnsubscribeVersionNotify' | 'LimitedReserveTransferAssets' | 'LimitedTeleportAssets' | 'ForceSuspension' | 'TransferAssets' | 'ClaimAssets' | 'TransferAssetsUsingTypeAndThen';
    }
    /** @name XcmVersionedXcm (331) */
    interface XcmVersionedXcm extends Enum {
        readonly isV3: boolean;
        readonly asV3: XcmV3Xcm;
        readonly isV4: boolean;
        readonly asV4: StagingXcmV4Xcm;
        readonly isV5: boolean;
        readonly asV5: StagingXcmV5Xcm;
        readonly type: 'V3' | 'V4' | 'V5';
    }
    /** @name XcmV3Xcm (332) */
    interface XcmV3Xcm extends Vec<XcmV3Instruction> {
    }
    /** @name XcmV3Instruction (334) */
    interface XcmV3Instruction extends Enum {
        readonly isWithdrawAsset: boolean;
        readonly asWithdrawAsset: XcmV3MultiassetMultiAssets;
        readonly isReserveAssetDeposited: boolean;
        readonly asReserveAssetDeposited: XcmV3MultiassetMultiAssets;
        readonly isReceiveTeleportedAsset: boolean;
        readonly asReceiveTeleportedAsset: XcmV3MultiassetMultiAssets;
        readonly isQueryResponse: boolean;
        readonly asQueryResponse: {
            readonly queryId: Compact<u64>;
            readonly response: XcmV3Response;
            readonly maxWeight: SpWeightsWeightV2Weight;
            readonly querier: Option<StagingXcmV3MultiLocation>;
        } & Struct;
        readonly isTransferAsset: boolean;
        readonly asTransferAsset: {
            readonly assets: XcmV3MultiassetMultiAssets;
            readonly beneficiary: StagingXcmV3MultiLocation;
        } & Struct;
        readonly isTransferReserveAsset: boolean;
        readonly asTransferReserveAsset: {
            readonly assets: XcmV3MultiassetMultiAssets;
            readonly dest: StagingXcmV3MultiLocation;
            readonly xcm: XcmV3Xcm;
        } & Struct;
        readonly isTransact: boolean;
        readonly asTransact: {
            readonly originKind: XcmV3OriginKind;
            readonly requireWeightAtMost: SpWeightsWeightV2Weight;
            readonly call: XcmDoubleEncoded;
        } & Struct;
        readonly isHrmpNewChannelOpenRequest: boolean;
        readonly asHrmpNewChannelOpenRequest: {
            readonly sender: Compact<u32>;
            readonly maxMessageSize: Compact<u32>;
            readonly maxCapacity: Compact<u32>;
        } & Struct;
        readonly isHrmpChannelAccepted: boolean;
        readonly asHrmpChannelAccepted: {
            readonly recipient: Compact<u32>;
        } & Struct;
        readonly isHrmpChannelClosing: boolean;
        readonly asHrmpChannelClosing: {
            readonly initiator: Compact<u32>;
            readonly sender: Compact<u32>;
            readonly recipient: Compact<u32>;
        } & Struct;
        readonly isClearOrigin: boolean;
        readonly isDescendOrigin: boolean;
        readonly asDescendOrigin: XcmV3Junctions;
        readonly isReportError: boolean;
        readonly asReportError: XcmV3QueryResponseInfo;
        readonly isDepositAsset: boolean;
        readonly asDepositAsset: {
            readonly assets: XcmV3MultiassetMultiAssetFilter;
            readonly beneficiary: StagingXcmV3MultiLocation;
        } & Struct;
        readonly isDepositReserveAsset: boolean;
        readonly asDepositReserveAsset: {
            readonly assets: XcmV3MultiassetMultiAssetFilter;
            readonly dest: StagingXcmV3MultiLocation;
            readonly xcm: XcmV3Xcm;
        } & Struct;
        readonly isExchangeAsset: boolean;
        readonly asExchangeAsset: {
            readonly give: XcmV3MultiassetMultiAssetFilter;
            readonly want: XcmV3MultiassetMultiAssets;
            readonly maximal: bool;
        } & Struct;
        readonly isInitiateReserveWithdraw: boolean;
        readonly asInitiateReserveWithdraw: {
            readonly assets: XcmV3MultiassetMultiAssetFilter;
            readonly reserve: StagingXcmV3MultiLocation;
            readonly xcm: XcmV3Xcm;
        } & Struct;
        readonly isInitiateTeleport: boolean;
        readonly asInitiateTeleport: {
            readonly assets: XcmV3MultiassetMultiAssetFilter;
            readonly dest: StagingXcmV3MultiLocation;
            readonly xcm: XcmV3Xcm;
        } & Struct;
        readonly isReportHolding: boolean;
        readonly asReportHolding: {
            readonly responseInfo: XcmV3QueryResponseInfo;
            readonly assets: XcmV3MultiassetMultiAssetFilter;
        } & Struct;
        readonly isBuyExecution: boolean;
        readonly asBuyExecution: {
            readonly fees: XcmV3MultiAsset;
            readonly weightLimit: XcmV3WeightLimit;
        } & Struct;
        readonly isRefundSurplus: boolean;
        readonly isSetErrorHandler: boolean;
        readonly asSetErrorHandler: XcmV3Xcm;
        readonly isSetAppendix: boolean;
        readonly asSetAppendix: XcmV3Xcm;
        readonly isClearError: boolean;
        readonly isClaimAsset: boolean;
        readonly asClaimAsset: {
            readonly assets: XcmV3MultiassetMultiAssets;
            readonly ticket: StagingXcmV3MultiLocation;
        } & Struct;
        readonly isTrap: boolean;
        readonly asTrap: Compact<u64>;
        readonly isSubscribeVersion: boolean;
        readonly asSubscribeVersion: {
            readonly queryId: Compact<u64>;
            readonly maxResponseWeight: SpWeightsWeightV2Weight;
        } & Struct;
        readonly isUnsubscribeVersion: boolean;
        readonly isBurnAsset: boolean;
        readonly asBurnAsset: XcmV3MultiassetMultiAssets;
        readonly isExpectAsset: boolean;
        readonly asExpectAsset: XcmV3MultiassetMultiAssets;
        readonly isExpectOrigin: boolean;
        readonly asExpectOrigin: Option<StagingXcmV3MultiLocation>;
        readonly isExpectError: boolean;
        readonly asExpectError: Option<ITuple<[u32, XcmV3TraitsError]>>;
        readonly isExpectTransactStatus: boolean;
        readonly asExpectTransactStatus: XcmV3MaybeErrorCode;
        readonly isQueryPallet: boolean;
        readonly asQueryPallet: {
            readonly moduleName: Bytes;
            readonly responseInfo: XcmV3QueryResponseInfo;
        } & Struct;
        readonly isExpectPallet: boolean;
        readonly asExpectPallet: {
            readonly index: Compact<u32>;
            readonly name: Bytes;
            readonly moduleName: Bytes;
            readonly crateMajor: Compact<u32>;
            readonly minCrateMinor: Compact<u32>;
        } & Struct;
        readonly isReportTransactStatus: boolean;
        readonly asReportTransactStatus: XcmV3QueryResponseInfo;
        readonly isClearTransactStatus: boolean;
        readonly isUniversalOrigin: boolean;
        readonly asUniversalOrigin: XcmV3Junction;
        readonly isExportMessage: boolean;
        readonly asExportMessage: {
            readonly network: XcmV3JunctionNetworkId;
            readonly destination: XcmV3Junctions;
            readonly xcm: XcmV3Xcm;
        } & Struct;
        readonly isLockAsset: boolean;
        readonly asLockAsset: {
            readonly asset: XcmV3MultiAsset;
            readonly unlocker: StagingXcmV3MultiLocation;
        } & Struct;
        readonly isUnlockAsset: boolean;
        readonly asUnlockAsset: {
            readonly asset: XcmV3MultiAsset;
            readonly target: StagingXcmV3MultiLocation;
        } & Struct;
        readonly isNoteUnlockable: boolean;
        readonly asNoteUnlockable: {
            readonly asset: XcmV3MultiAsset;
            readonly owner: StagingXcmV3MultiLocation;
        } & Struct;
        readonly isRequestUnlock: boolean;
        readonly asRequestUnlock: {
            readonly asset: XcmV3MultiAsset;
            readonly locker: StagingXcmV3MultiLocation;
        } & Struct;
        readonly isSetFeesMode: boolean;
        readonly asSetFeesMode: {
            readonly jitWithdraw: bool;
        } & Struct;
        readonly isSetTopic: boolean;
        readonly asSetTopic: U8aFixed;
        readonly isClearTopic: boolean;
        readonly isAliasOrigin: boolean;
        readonly asAliasOrigin: StagingXcmV3MultiLocation;
        readonly isUnpaidExecution: boolean;
        readonly asUnpaidExecution: {
            readonly weightLimit: XcmV3WeightLimit;
            readonly checkOrigin: Option<StagingXcmV3MultiLocation>;
        } & Struct;
        readonly type: 'WithdrawAsset' | 'ReserveAssetDeposited' | 'ReceiveTeleportedAsset' | 'QueryResponse' | 'TransferAsset' | 'TransferReserveAsset' | 'Transact' | 'HrmpNewChannelOpenRequest' | 'HrmpChannelAccepted' | 'HrmpChannelClosing' | 'ClearOrigin' | 'DescendOrigin' | 'ReportError' | 'DepositAsset' | 'DepositReserveAsset' | 'ExchangeAsset' | 'InitiateReserveWithdraw' | 'InitiateTeleport' | 'ReportHolding' | 'BuyExecution' | 'RefundSurplus' | 'SetErrorHandler' | 'SetAppendix' | 'ClearError' | 'ClaimAsset' | 'Trap' | 'SubscribeVersion' | 'UnsubscribeVersion' | 'BurnAsset' | 'ExpectAsset' | 'ExpectOrigin' | 'ExpectError' | 'ExpectTransactStatus' | 'QueryPallet' | 'ExpectPallet' | 'ReportTransactStatus' | 'ClearTransactStatus' | 'UniversalOrigin' | 'ExportMessage' | 'LockAsset' | 'UnlockAsset' | 'NoteUnlockable' | 'RequestUnlock' | 'SetFeesMode' | 'SetTopic' | 'ClearTopic' | 'AliasOrigin' | 'UnpaidExecution';
    }
    /** @name XcmV3Response (335) */
    interface XcmV3Response extends Enum {
        readonly isNull: boolean;
        readonly isAssets: boolean;
        readonly asAssets: XcmV3MultiassetMultiAssets;
        readonly isExecutionResult: boolean;
        readonly asExecutionResult: Option<ITuple<[u32, XcmV3TraitsError]>>;
        readonly isVersion: boolean;
        readonly asVersion: u32;
        readonly isPalletsInfo: boolean;
        readonly asPalletsInfo: Vec<XcmV3PalletInfo>;
        readonly isDispatchResult: boolean;
        readonly asDispatchResult: XcmV3MaybeErrorCode;
        readonly type: 'Null' | 'Assets' | 'ExecutionResult' | 'Version' | 'PalletsInfo' | 'DispatchResult';
    }
    /** @name XcmV3TraitsError (338) */
    interface XcmV3TraitsError extends Enum {
        readonly isOverflow: boolean;
        readonly isUnimplemented: boolean;
        readonly isUntrustedReserveLocation: boolean;
        readonly isUntrustedTeleportLocation: boolean;
        readonly isLocationFull: boolean;
        readonly isLocationNotInvertible: boolean;
        readonly isBadOrigin: boolean;
        readonly isInvalidLocation: boolean;
        readonly isAssetNotFound: boolean;
        readonly isFailedToTransactAsset: boolean;
        readonly isNotWithdrawable: boolean;
        readonly isLocationCannotHold: boolean;
        readonly isExceedsMaxMessageSize: boolean;
        readonly isDestinationUnsupported: boolean;
        readonly isTransport: boolean;
        readonly isUnroutable: boolean;
        readonly isUnknownClaim: boolean;
        readonly isFailedToDecode: boolean;
        readonly isMaxWeightInvalid: boolean;
        readonly isNotHoldingFees: boolean;
        readonly isTooExpensive: boolean;
        readonly isTrap: boolean;
        readonly asTrap: u64;
        readonly isExpectationFalse: boolean;
        readonly isPalletNotFound: boolean;
        readonly isNameMismatch: boolean;
        readonly isVersionIncompatible: boolean;
        readonly isHoldingWouldOverflow: boolean;
        readonly isExportError: boolean;
        readonly isReanchorFailed: boolean;
        readonly isNoDeal: boolean;
        readonly isFeesNotMet: boolean;
        readonly isLockError: boolean;
        readonly isNoPermission: boolean;
        readonly isUnanchored: boolean;
        readonly isNotDepositable: boolean;
        readonly isUnhandledXcmVersion: boolean;
        readonly isWeightLimitReached: boolean;
        readonly asWeightLimitReached: SpWeightsWeightV2Weight;
        readonly isBarrier: boolean;
        readonly isWeightNotComputable: boolean;
        readonly isExceedsStackLimit: boolean;
        readonly type: 'Overflow' | 'Unimplemented' | 'UntrustedReserveLocation' | 'UntrustedTeleportLocation' | 'LocationFull' | 'LocationNotInvertible' | 'BadOrigin' | 'InvalidLocation' | 'AssetNotFound' | 'FailedToTransactAsset' | 'NotWithdrawable' | 'LocationCannotHold' | 'ExceedsMaxMessageSize' | 'DestinationUnsupported' | 'Transport' | 'Unroutable' | 'UnknownClaim' | 'FailedToDecode' | 'MaxWeightInvalid' | 'NotHoldingFees' | 'TooExpensive' | 'Trap' | 'ExpectationFalse' | 'PalletNotFound' | 'NameMismatch' | 'VersionIncompatible' | 'HoldingWouldOverflow' | 'ExportError' | 'ReanchorFailed' | 'NoDeal' | 'FeesNotMet' | 'LockError' | 'NoPermission' | 'Unanchored' | 'NotDepositable' | 'UnhandledXcmVersion' | 'WeightLimitReached' | 'Barrier' | 'WeightNotComputable' | 'ExceedsStackLimit';
    }
    /** @name XcmV3PalletInfo (340) */
    interface XcmV3PalletInfo extends Struct {
        readonly index: Compact<u32>;
        readonly name: Bytes;
        readonly moduleName: Bytes;
        readonly major: Compact<u32>;
        readonly minor: Compact<u32>;
        readonly patch: Compact<u32>;
    }
    /** @name XcmV3QueryResponseInfo (344) */
    interface XcmV3QueryResponseInfo extends Struct {
        readonly destination: StagingXcmV3MultiLocation;
        readonly queryId: Compact<u64>;
        readonly maxWeight: SpWeightsWeightV2Weight;
    }
    /** @name XcmV3MultiassetMultiAssetFilter (345) */
    interface XcmV3MultiassetMultiAssetFilter extends Enum {
        readonly isDefinite: boolean;
        readonly asDefinite: XcmV3MultiassetMultiAssets;
        readonly isWild: boolean;
        readonly asWild: XcmV3MultiassetWildMultiAsset;
        readonly type: 'Definite' | 'Wild';
    }
    /** @name XcmV3MultiassetWildMultiAsset (346) */
    interface XcmV3MultiassetWildMultiAsset extends Enum {
        readonly isAll: boolean;
        readonly isAllOf: boolean;
        readonly asAllOf: {
            readonly id: XcmV3MultiassetAssetId;
            readonly fun: XcmV3MultiassetWildFungibility;
        } & Struct;
        readonly isAllCounted: boolean;
        readonly asAllCounted: Compact<u32>;
        readonly isAllOfCounted: boolean;
        readonly asAllOfCounted: {
            readonly id: XcmV3MultiassetAssetId;
            readonly fun: XcmV3MultiassetWildFungibility;
            readonly count: Compact<u32>;
        } & Struct;
        readonly type: 'All' | 'AllOf' | 'AllCounted' | 'AllOfCounted';
    }
    /** @name XcmV3MultiassetWildFungibility (347) */
    interface XcmV3MultiassetWildFungibility extends Enum {
        readonly isFungible: boolean;
        readonly isNonFungible: boolean;
        readonly type: 'Fungible' | 'NonFungible';
    }
    /** @name StagingXcmV4Xcm (348) */
    interface StagingXcmV4Xcm extends Vec<StagingXcmV4Instruction> {
    }
    /** @name StagingXcmV4Instruction (350) */
    interface StagingXcmV4Instruction extends Enum {
        readonly isWithdrawAsset: boolean;
        readonly asWithdrawAsset: StagingXcmV4AssetAssets;
        readonly isReserveAssetDeposited: boolean;
        readonly asReserveAssetDeposited: StagingXcmV4AssetAssets;
        readonly isReceiveTeleportedAsset: boolean;
        readonly asReceiveTeleportedAsset: StagingXcmV4AssetAssets;
        readonly isQueryResponse: boolean;
        readonly asQueryResponse: {
            readonly queryId: Compact<u64>;
            readonly response: StagingXcmV4Response;
            readonly maxWeight: SpWeightsWeightV2Weight;
            readonly querier: Option<StagingXcmV4Location>;
        } & Struct;
        readonly isTransferAsset: boolean;
        readonly asTransferAsset: {
            readonly assets: StagingXcmV4AssetAssets;
            readonly beneficiary: StagingXcmV4Location;
        } & Struct;
        readonly isTransferReserveAsset: boolean;
        readonly asTransferReserveAsset: {
            readonly assets: StagingXcmV4AssetAssets;
            readonly dest: StagingXcmV4Location;
            readonly xcm: StagingXcmV4Xcm;
        } & Struct;
        readonly isTransact: boolean;
        readonly asTransact: {
            readonly originKind: XcmV3OriginKind;
            readonly requireWeightAtMost: SpWeightsWeightV2Weight;
            readonly call: XcmDoubleEncoded;
        } & Struct;
        readonly isHrmpNewChannelOpenRequest: boolean;
        readonly asHrmpNewChannelOpenRequest: {
            readonly sender: Compact<u32>;
            readonly maxMessageSize: Compact<u32>;
            readonly maxCapacity: Compact<u32>;
        } & Struct;
        readonly isHrmpChannelAccepted: boolean;
        readonly asHrmpChannelAccepted: {
            readonly recipient: Compact<u32>;
        } & Struct;
        readonly isHrmpChannelClosing: boolean;
        readonly asHrmpChannelClosing: {
            readonly initiator: Compact<u32>;
            readonly sender: Compact<u32>;
            readonly recipient: Compact<u32>;
        } & Struct;
        readonly isClearOrigin: boolean;
        readonly isDescendOrigin: boolean;
        readonly asDescendOrigin: StagingXcmV4Junctions;
        readonly isReportError: boolean;
        readonly asReportError: StagingXcmV4QueryResponseInfo;
        readonly isDepositAsset: boolean;
        readonly asDepositAsset: {
            readonly assets: StagingXcmV4AssetAssetFilter;
            readonly beneficiary: StagingXcmV4Location;
        } & Struct;
        readonly isDepositReserveAsset: boolean;
        readonly asDepositReserveAsset: {
            readonly assets: StagingXcmV4AssetAssetFilter;
            readonly dest: StagingXcmV4Location;
            readonly xcm: StagingXcmV4Xcm;
        } & Struct;
        readonly isExchangeAsset: boolean;
        readonly asExchangeAsset: {
            readonly give: StagingXcmV4AssetAssetFilter;
            readonly want: StagingXcmV4AssetAssets;
            readonly maximal: bool;
        } & Struct;
        readonly isInitiateReserveWithdraw: boolean;
        readonly asInitiateReserveWithdraw: {
            readonly assets: StagingXcmV4AssetAssetFilter;
            readonly reserve: StagingXcmV4Location;
            readonly xcm: StagingXcmV4Xcm;
        } & Struct;
        readonly isInitiateTeleport: boolean;
        readonly asInitiateTeleport: {
            readonly assets: StagingXcmV4AssetAssetFilter;
            readonly dest: StagingXcmV4Location;
            readonly xcm: StagingXcmV4Xcm;
        } & Struct;
        readonly isReportHolding: boolean;
        readonly asReportHolding: {
            readonly responseInfo: StagingXcmV4QueryResponseInfo;
            readonly assets: StagingXcmV4AssetAssetFilter;
        } & Struct;
        readonly isBuyExecution: boolean;
        readonly asBuyExecution: {
            readonly fees: StagingXcmV4Asset;
            readonly weightLimit: XcmV3WeightLimit;
        } & Struct;
        readonly isRefundSurplus: boolean;
        readonly isSetErrorHandler: boolean;
        readonly asSetErrorHandler: StagingXcmV4Xcm;
        readonly isSetAppendix: boolean;
        readonly asSetAppendix: StagingXcmV4Xcm;
        readonly isClearError: boolean;
        readonly isClaimAsset: boolean;
        readonly asClaimAsset: {
            readonly assets: StagingXcmV4AssetAssets;
            readonly ticket: StagingXcmV4Location;
        } & Struct;
        readonly isTrap: boolean;
        readonly asTrap: Compact<u64>;
        readonly isSubscribeVersion: boolean;
        readonly asSubscribeVersion: {
            readonly queryId: Compact<u64>;
            readonly maxResponseWeight: SpWeightsWeightV2Weight;
        } & Struct;
        readonly isUnsubscribeVersion: boolean;
        readonly isBurnAsset: boolean;
        readonly asBurnAsset: StagingXcmV4AssetAssets;
        readonly isExpectAsset: boolean;
        readonly asExpectAsset: StagingXcmV4AssetAssets;
        readonly isExpectOrigin: boolean;
        readonly asExpectOrigin: Option<StagingXcmV4Location>;
        readonly isExpectError: boolean;
        readonly asExpectError: Option<ITuple<[u32, XcmV3TraitsError]>>;
        readonly isExpectTransactStatus: boolean;
        readonly asExpectTransactStatus: XcmV3MaybeErrorCode;
        readonly isQueryPallet: boolean;
        readonly asQueryPallet: {
            readonly moduleName: Bytes;
            readonly responseInfo: StagingXcmV4QueryResponseInfo;
        } & Struct;
        readonly isExpectPallet: boolean;
        readonly asExpectPallet: {
            readonly index: Compact<u32>;
            readonly name: Bytes;
            readonly moduleName: Bytes;
            readonly crateMajor: Compact<u32>;
            readonly minCrateMinor: Compact<u32>;
        } & Struct;
        readonly isReportTransactStatus: boolean;
        readonly asReportTransactStatus: StagingXcmV4QueryResponseInfo;
        readonly isClearTransactStatus: boolean;
        readonly isUniversalOrigin: boolean;
        readonly asUniversalOrigin: StagingXcmV4Junction;
        readonly isExportMessage: boolean;
        readonly asExportMessage: {
            readonly network: StagingXcmV4JunctionNetworkId;
            readonly destination: StagingXcmV4Junctions;
            readonly xcm: StagingXcmV4Xcm;
        } & Struct;
        readonly isLockAsset: boolean;
        readonly asLockAsset: {
            readonly asset: StagingXcmV4Asset;
            readonly unlocker: StagingXcmV4Location;
        } & Struct;
        readonly isUnlockAsset: boolean;
        readonly asUnlockAsset: {
            readonly asset: StagingXcmV4Asset;
            readonly target: StagingXcmV4Location;
        } & Struct;
        readonly isNoteUnlockable: boolean;
        readonly asNoteUnlockable: {
            readonly asset: StagingXcmV4Asset;
            readonly owner: StagingXcmV4Location;
        } & Struct;
        readonly isRequestUnlock: boolean;
        readonly asRequestUnlock: {
            readonly asset: StagingXcmV4Asset;
            readonly locker: StagingXcmV4Location;
        } & Struct;
        readonly isSetFeesMode: boolean;
        readonly asSetFeesMode: {
            readonly jitWithdraw: bool;
        } & Struct;
        readonly isSetTopic: boolean;
        readonly asSetTopic: U8aFixed;
        readonly isClearTopic: boolean;
        readonly isAliasOrigin: boolean;
        readonly asAliasOrigin: StagingXcmV4Location;
        readonly isUnpaidExecution: boolean;
        readonly asUnpaidExecution: {
            readonly weightLimit: XcmV3WeightLimit;
            readonly checkOrigin: Option<StagingXcmV4Location>;
        } & Struct;
        readonly type: 'WithdrawAsset' | 'ReserveAssetDeposited' | 'ReceiveTeleportedAsset' | 'QueryResponse' | 'TransferAsset' | 'TransferReserveAsset' | 'Transact' | 'HrmpNewChannelOpenRequest' | 'HrmpChannelAccepted' | 'HrmpChannelClosing' | 'ClearOrigin' | 'DescendOrigin' | 'ReportError' | 'DepositAsset' | 'DepositReserveAsset' | 'ExchangeAsset' | 'InitiateReserveWithdraw' | 'InitiateTeleport' | 'ReportHolding' | 'BuyExecution' | 'RefundSurplus' | 'SetErrorHandler' | 'SetAppendix' | 'ClearError' | 'ClaimAsset' | 'Trap' | 'SubscribeVersion' | 'UnsubscribeVersion' | 'BurnAsset' | 'ExpectAsset' | 'ExpectOrigin' | 'ExpectError' | 'ExpectTransactStatus' | 'QueryPallet' | 'ExpectPallet' | 'ReportTransactStatus' | 'ClearTransactStatus' | 'UniversalOrigin' | 'ExportMessage' | 'LockAsset' | 'UnlockAsset' | 'NoteUnlockable' | 'RequestUnlock' | 'SetFeesMode' | 'SetTopic' | 'ClearTopic' | 'AliasOrigin' | 'UnpaidExecution';
    }
    /** @name StagingXcmV4Response (351) */
    interface StagingXcmV4Response extends Enum {
        readonly isNull: boolean;
        readonly isAssets: boolean;
        readonly asAssets: StagingXcmV4AssetAssets;
        readonly isExecutionResult: boolean;
        readonly asExecutionResult: Option<ITuple<[u32, XcmV3TraitsError]>>;
        readonly isVersion: boolean;
        readonly asVersion: u32;
        readonly isPalletsInfo: boolean;
        readonly asPalletsInfo: Vec<StagingXcmV4PalletInfo>;
        readonly isDispatchResult: boolean;
        readonly asDispatchResult: XcmV3MaybeErrorCode;
        readonly type: 'Null' | 'Assets' | 'ExecutionResult' | 'Version' | 'PalletsInfo' | 'DispatchResult';
    }
    /** @name StagingXcmV4PalletInfo (353) */
    interface StagingXcmV4PalletInfo extends Struct {
        readonly index: Compact<u32>;
        readonly name: Bytes;
        readonly moduleName: Bytes;
        readonly major: Compact<u32>;
        readonly minor: Compact<u32>;
        readonly patch: Compact<u32>;
    }
    /** @name StagingXcmV4QueryResponseInfo (357) */
    interface StagingXcmV4QueryResponseInfo extends Struct {
        readonly destination: StagingXcmV4Location;
        readonly queryId: Compact<u64>;
        readonly maxWeight: SpWeightsWeightV2Weight;
    }
    /** @name StagingXcmV4AssetAssetFilter (358) */
    interface StagingXcmV4AssetAssetFilter extends Enum {
        readonly isDefinite: boolean;
        readonly asDefinite: StagingXcmV4AssetAssets;
        readonly isWild: boolean;
        readonly asWild: StagingXcmV4AssetWildAsset;
        readonly type: 'Definite' | 'Wild';
    }
    /** @name StagingXcmV4AssetWildAsset (359) */
    interface StagingXcmV4AssetWildAsset extends Enum {
        readonly isAll: boolean;
        readonly isAllOf: boolean;
        readonly asAllOf: {
            readonly id: StagingXcmV4AssetAssetId;
            readonly fun: StagingXcmV4AssetWildFungibility;
        } & Struct;
        readonly isAllCounted: boolean;
        readonly asAllCounted: Compact<u32>;
        readonly isAllOfCounted: boolean;
        readonly asAllOfCounted: {
            readonly id: StagingXcmV4AssetAssetId;
            readonly fun: StagingXcmV4AssetWildFungibility;
            readonly count: Compact<u32>;
        } & Struct;
        readonly type: 'All' | 'AllOf' | 'AllCounted' | 'AllOfCounted';
    }
    /** @name StagingXcmV4AssetWildFungibility (360) */
    interface StagingXcmV4AssetWildFungibility extends Enum {
        readonly isFungible: boolean;
        readonly isNonFungible: boolean;
        readonly type: 'Fungible' | 'NonFungible';
    }
    /** @name StagingXcmExecutorAssetTransferTransferType (372) */
    interface StagingXcmExecutorAssetTransferTransferType extends Enum {
        readonly isTeleport: boolean;
        readonly isLocalReserve: boolean;
        readonly isDestinationReserve: boolean;
        readonly isRemoteReserve: boolean;
        readonly asRemoteReserve: XcmVersionedLocation;
        readonly type: 'Teleport' | 'LocalReserve' | 'DestinationReserve' | 'RemoteReserve';
    }
    /** @name XcmVersionedAssetId (373) */
    interface XcmVersionedAssetId extends Enum {
        readonly isV3: boolean;
        readonly asV3: XcmV3MultiassetAssetId;
        readonly isV4: boolean;
        readonly asV4: StagingXcmV4AssetAssetId;
        readonly isV5: boolean;
        readonly asV5: StagingXcmV5AssetAssetId;
        readonly type: 'V3' | 'V4' | 'V5';
    }
    /** @name CumulusPalletXcmCall (374) */
    type CumulusPalletXcmCall = Null;
    /** @name PalletMessageQueueCall (375) */
    interface PalletMessageQueueCall extends Enum {
        readonly isReapPage: boolean;
        readonly asReapPage: {
            readonly messageOrigin: CumulusPrimitivesCoreAggregateMessageOrigin;
            readonly pageIndex: u32;
        } & Struct;
        readonly isExecuteOverweight: boolean;
        readonly asExecuteOverweight: {
            readonly messageOrigin: CumulusPrimitivesCoreAggregateMessageOrigin;
            readonly page: u32;
            readonly index: u32;
            readonly weightLimit: SpWeightsWeightV2Weight;
        } & Struct;
        readonly type: 'ReapPage' | 'ExecuteOverweight';
    }
    /** @name PalletStorageProvidersCall (376) */
    interface PalletStorageProvidersCall extends Enum {
        readonly isRequestMspSignUp: boolean;
        readonly asRequestMspSignUp: {
            readonly capacity: u64;
            readonly multiaddresses: Vec<Bytes>;
            readonly valuePropPricePerGigaUnitOfDataPerBlock: u128;
            readonly commitment: Bytes;
            readonly valuePropMaxDataLimit: u64;
            readonly paymentAccount: AccountId32;
        } & Struct;
        readonly isRequestBspSignUp: boolean;
        readonly asRequestBspSignUp: {
            readonly capacity: u64;
            readonly multiaddresses: Vec<Bytes>;
            readonly paymentAccount: AccountId32;
        } & Struct;
        readonly isConfirmSignUp: boolean;
        readonly asConfirmSignUp: {
            readonly providerAccount: Option<AccountId32>;
        } & Struct;
        readonly isCancelSignUp: boolean;
        readonly isMspSignOff: boolean;
        readonly asMspSignOff: {
            readonly mspId: H256;
        } & Struct;
        readonly isBspSignOff: boolean;
        readonly isChangeCapacity: boolean;
        readonly asChangeCapacity: {
            readonly newCapacity: u64;
        } & Struct;
        readonly isAddValueProp: boolean;
        readonly asAddValueProp: {
            readonly pricePerGigaUnitOfDataPerBlock: u128;
            readonly commitment: Bytes;
            readonly bucketDataLimit: u64;
        } & Struct;
        readonly isMakeValuePropUnavailable: boolean;
        readonly asMakeValuePropUnavailable: {
            readonly valuePropId: H256;
        } & Struct;
        readonly isAddMultiaddress: boolean;
        readonly asAddMultiaddress: {
            readonly newMultiaddress: Bytes;
        } & Struct;
        readonly isRemoveMultiaddress: boolean;
        readonly asRemoveMultiaddress: {
            readonly multiaddress: Bytes;
        } & Struct;
        readonly isForceMspSignUp: boolean;
        readonly asForceMspSignUp: {
            readonly who: AccountId32;
            readonly mspId: H256;
            readonly capacity: u64;
            readonly multiaddresses: Vec<Bytes>;
            readonly valuePropPricePerGigaUnitOfDataPerBlock: u128;
            readonly commitment: Bytes;
            readonly valuePropMaxDataLimit: u64;
            readonly paymentAccount: AccountId32;
        } & Struct;
        readonly isForceBspSignUp: boolean;
        readonly asForceBspSignUp: {
            readonly who: AccountId32;
            readonly bspId: H256;
            readonly capacity: u64;
            readonly multiaddresses: Vec<Bytes>;
            readonly paymentAccount: AccountId32;
            readonly weight: Option<u32>;
        } & Struct;
        readonly isSlash: boolean;
        readonly asSlash: {
            readonly providerId: H256;
        } & Struct;
        readonly isTopUpDeposit: boolean;
        readonly isDeleteProvider: boolean;
        readonly asDeleteProvider: {
            readonly providerId: H256;
        } & Struct;
        readonly isStopAllCycles: boolean;
        readonly type: 'RequestMspSignUp' | 'RequestBspSignUp' | 'ConfirmSignUp' | 'CancelSignUp' | 'MspSignOff' | 'BspSignOff' | 'ChangeCapacity' | 'AddValueProp' | 'MakeValuePropUnavailable' | 'AddMultiaddress' | 'RemoveMultiaddress' | 'ForceMspSignUp' | 'ForceBspSignUp' | 'Slash' | 'TopUpDeposit' | 'DeleteProvider' | 'StopAllCycles';
    }
    /** @name PalletFileSystemCall (377) */
    interface PalletFileSystemCall extends Enum {
        readonly isCreateBucket: boolean;
        readonly asCreateBucket: {
            readonly mspId: H256;
            readonly name: Bytes;
            readonly private: bool;
            readonly valuePropId: H256;
        } & Struct;
        readonly isRequestMoveBucket: boolean;
        readonly asRequestMoveBucket: {
            readonly bucketId: H256;
            readonly newMspId: H256;
            readonly newValuePropId: H256;
        } & Struct;
        readonly isMspRespondMoveBucketRequest: boolean;
        readonly asMspRespondMoveBucketRequest: {
            readonly bucketId: H256;
            readonly response: PalletFileSystemBucketMoveRequestResponse;
        } & Struct;
        readonly isUpdateBucketPrivacy: boolean;
        readonly asUpdateBucketPrivacy: {
            readonly bucketId: H256;
            readonly private: bool;
        } & Struct;
        readonly isCreateAndAssociateCollectionWithBucket: boolean;
        readonly asCreateAndAssociateCollectionWithBucket: {
            readonly bucketId: H256;
        } & Struct;
        readonly isDeleteBucket: boolean;
        readonly asDeleteBucket: {
            readonly bucketId: H256;
        } & Struct;
        readonly isIssueStorageRequest: boolean;
        readonly asIssueStorageRequest: {
            readonly bucketId: H256;
            readonly location: Bytes;
            readonly fingerprint: H256;
            readonly size_: u64;
            readonly mspId: H256;
            readonly peerIds: Vec<Bytes>;
            readonly replicationTarget: PalletFileSystemReplicationTarget;
        } & Struct;
        readonly isRevokeStorageRequest: boolean;
        readonly asRevokeStorageRequest: {
            readonly fileKey: H256;
        } & Struct;
        readonly isMspRespondStorageRequestsMultipleBuckets: boolean;
        readonly asMspRespondStorageRequestsMultipleBuckets: {
            readonly storageRequestMspResponse: Vec<PalletFileSystemStorageRequestMspBucketResponse>;
        } & Struct;
        readonly isMspStopStoringBucket: boolean;
        readonly asMspStopStoringBucket: {
            readonly bucketId: H256;
        } & Struct;
        readonly isBspVolunteer: boolean;
        readonly asBspVolunteer: {
            readonly fileKey: H256;
        } & Struct;
        readonly isBspConfirmStoring: boolean;
        readonly asBspConfirmStoring: {
            readonly nonInclusionForestProof: SpTrieStorageProofCompactProof;
            readonly fileKeysAndProofs: Vec<PalletFileSystemFileKeyWithProof>;
        } & Struct;
        readonly isBspRequestStopStoring: boolean;
        readonly asBspRequestStopStoring: {
            readonly fileKey: H256;
            readonly bucketId: H256;
            readonly location: Bytes;
            readonly owner: AccountId32;
            readonly fingerprint: H256;
            readonly size_: u64;
            readonly canServe: bool;
            readonly inclusionForestProof: SpTrieStorageProofCompactProof;
        } & Struct;
        readonly isBspConfirmStopStoring: boolean;
        readonly asBspConfirmStopStoring: {
            readonly fileKey: H256;
            readonly inclusionForestProof: SpTrieStorageProofCompactProof;
        } & Struct;
        readonly isStopStoringForInsolventUser: boolean;
        readonly asStopStoringForInsolventUser: {
            readonly fileKey: H256;
            readonly bucketId: H256;
            readonly location: Bytes;
            readonly owner: AccountId32;
            readonly fingerprint: H256;
            readonly size_: u64;
            readonly inclusionForestProof: SpTrieStorageProofCompactProof;
        } & Struct;
        readonly isMspStopStoringBucketForInsolventUser: boolean;
        readonly asMspStopStoringBucketForInsolventUser: {
            readonly bucketId: H256;
        } & Struct;
        readonly isRequestDeleteFile: boolean;
        readonly asRequestDeleteFile: {
            readonly signedIntention: PalletFileSystemFileOperationIntention;
            readonly signature: SpRuntimeMultiSignature;
            readonly bucketId: H256;
            readonly location: Bytes;
            readonly size_: u64;
            readonly fingerprint: H256;
        } & Struct;
        readonly isDeleteFiles: boolean;
        readonly asDeleteFiles: {
            readonly fileDeletions: Vec<PalletFileSystemFileDeletionRequest>;
            readonly bspId: Option<H256>;
            readonly forestProof: SpTrieStorageProofCompactProof;
        } & Struct;
        readonly isDeleteFilesForIncompleteStorageRequest: boolean;
        readonly asDeleteFilesForIncompleteStorageRequest: {
            readonly fileKeys: Vec<H256>;
            readonly bspId: Option<H256>;
            readonly forestProof: SpTrieStorageProofCompactProof;
        } & Struct;
        readonly type: 'CreateBucket' | 'RequestMoveBucket' | 'MspRespondMoveBucketRequest' | 'UpdateBucketPrivacy' | 'CreateAndAssociateCollectionWithBucket' | 'DeleteBucket' | 'IssueStorageRequest' | 'RevokeStorageRequest' | 'MspRespondStorageRequestsMultipleBuckets' | 'MspStopStoringBucket' | 'BspVolunteer' | 'BspConfirmStoring' | 'BspRequestStopStoring' | 'BspConfirmStopStoring' | 'StopStoringForInsolventUser' | 'MspStopStoringBucketForInsolventUser' | 'RequestDeleteFile' | 'DeleteFiles' | 'DeleteFilesForIncompleteStorageRequest';
    }
    /** @name PalletFileSystemBucketMoveRequestResponse (378) */
    interface PalletFileSystemBucketMoveRequestResponse extends Enum {
        readonly isAccepted: boolean;
        readonly isRejected: boolean;
        readonly type: 'Accepted' | 'Rejected';
    }
    /** @name PalletFileSystemReplicationTarget (379) */
    interface PalletFileSystemReplicationTarget extends Enum {
        readonly isBasic: boolean;
        readonly isStandard: boolean;
        readonly isHighSecurity: boolean;
        readonly isSuperHighSecurity: boolean;
        readonly isUltraHighSecurity: boolean;
        readonly isCustom: boolean;
        readonly asCustom: u32;
        readonly type: 'Basic' | 'Standard' | 'HighSecurity' | 'SuperHighSecurity' | 'UltraHighSecurity' | 'Custom';
    }
    /** @name PalletFileSystemStorageRequestMspBucketResponse (381) */
    interface PalletFileSystemStorageRequestMspBucketResponse extends Struct {
        readonly bucketId: H256;
        readonly accept: Option<PalletFileSystemStorageRequestMspAcceptedFileKeys>;
        readonly reject: Vec<PalletFileSystemRejectedStorageRequest>;
    }
    /** @name PalletFileSystemStorageRequestMspAcceptedFileKeys (383) */
    interface PalletFileSystemStorageRequestMspAcceptedFileKeys extends Struct {
        readonly fileKeysAndProofs: Vec<PalletFileSystemFileKeyWithProof>;
        readonly forestProof: SpTrieStorageProofCompactProof;
    }
    /** @name PalletFileSystemFileKeyWithProof (385) */
    interface PalletFileSystemFileKeyWithProof extends Struct {
        readonly fileKey: H256;
        readonly proof: ShpFileKeyVerifierFileKeyProof;
    }
    /** @name PalletFileSystemRejectedStorageRequest (387) */
    interface PalletFileSystemRejectedStorageRequest extends Struct {
        readonly fileKey: H256;
        readonly reason: PalletFileSystemRejectedStorageRequestReason;
    }
    /** @name PalletFileSystemFileDeletionRequest (390) */
    interface PalletFileSystemFileDeletionRequest extends Struct {
        readonly fileOwner: AccountId32;
        readonly signedIntention: PalletFileSystemFileOperationIntention;
        readonly signature: SpRuntimeMultiSignature;
        readonly bucketId: H256;
        readonly location: Bytes;
        readonly size_: u64;
        readonly fingerprint: H256;
    }
    /** @name PalletProofsDealerCall (392) */
    interface PalletProofsDealerCall extends Enum {
        readonly isChallenge: boolean;
        readonly asChallenge: {
            readonly key: H256;
        } & Struct;
        readonly isSubmitProof: boolean;
        readonly asSubmitProof: {
            readonly proof: PalletProofsDealerProof;
            readonly provider: Option<H256>;
        } & Struct;
        readonly isForceInitialiseChallengeCycle: boolean;
        readonly asForceInitialiseChallengeCycle: {
            readonly provider: H256;
        } & Struct;
        readonly isSetPaused: boolean;
        readonly asSetPaused: {
            readonly paused: bool;
        } & Struct;
        readonly isPriorityChallenge: boolean;
        readonly asPriorityChallenge: {
            readonly key: H256;
            readonly shouldRemoveKey: bool;
        } & Struct;
        readonly type: 'Challenge' | 'SubmitProof' | 'ForceInitialiseChallengeCycle' | 'SetPaused' | 'PriorityChallenge';
    }
    /** @name PalletRandomnessCall (393) */
    interface PalletRandomnessCall extends Enum {
        readonly isSetBabeRandomness: boolean;
        readonly type: 'SetBabeRandomness';
    }
    /** @name PalletPaymentStreamsCall (394) */
    interface PalletPaymentStreamsCall extends Enum {
        readonly isCreateFixedRatePaymentStream: boolean;
        readonly asCreateFixedRatePaymentStream: {
            readonly providerId: H256;
            readonly userAccount: AccountId32;
            readonly rate: u128;
        } & Struct;
        readonly isUpdateFixedRatePaymentStream: boolean;
        readonly asUpdateFixedRatePaymentStream: {
            readonly providerId: H256;
            readonly userAccount: AccountId32;
            readonly newRate: u128;
        } & Struct;
        readonly isDeleteFixedRatePaymentStream: boolean;
        readonly asDeleteFixedRatePaymentStream: {
            readonly providerId: H256;
            readonly userAccount: AccountId32;
        } & Struct;
        readonly isCreateDynamicRatePaymentStream: boolean;
        readonly asCreateDynamicRatePaymentStream: {
            readonly providerId: H256;
            readonly userAccount: AccountId32;
            readonly amountProvided: u64;
        } & Struct;
        readonly isUpdateDynamicRatePaymentStream: boolean;
        readonly asUpdateDynamicRatePaymentStream: {
            readonly providerId: H256;
            readonly userAccount: AccountId32;
            readonly newAmountProvided: u64;
        } & Struct;
        readonly isDeleteDynamicRatePaymentStream: boolean;
        readonly asDeleteDynamicRatePaymentStream: {
            readonly providerId: H256;
            readonly userAccount: AccountId32;
        } & Struct;
        readonly isChargePaymentStreams: boolean;
        readonly asChargePaymentStreams: {
            readonly userAccount: AccountId32;
        } & Struct;
        readonly isChargeMultipleUsersPaymentStreams: boolean;
        readonly asChargeMultipleUsersPaymentStreams: {
            readonly userAccounts: Vec<AccountId32>;
        } & Struct;
        readonly isPayOutstandingDebt: boolean;
        readonly asPayOutstandingDebt: {
            readonly providers: Vec<H256>;
        } & Struct;
        readonly isClearInsolventFlag: boolean;
        readonly type: 'CreateFixedRatePaymentStream' | 'UpdateFixedRatePaymentStream' | 'DeleteFixedRatePaymentStream' | 'CreateDynamicRatePaymentStream' | 'UpdateDynamicRatePaymentStream' | 'DeleteDynamicRatePaymentStream' | 'ChargePaymentStreams' | 'ChargeMultipleUsersPaymentStreams' | 'PayOutstandingDebt' | 'ClearInsolventFlag';
    }
    /** @name PalletBucketNftsCall (395) */
    interface PalletBucketNftsCall extends Enum {
        readonly isShareAccess: boolean;
        readonly asShareAccess: {
            readonly recipient: MultiAddress;
            readonly bucket: H256;
            readonly itemId: u32;
            readonly readAccessRegex: Option<Bytes>;
        } & Struct;
        readonly isUpdateReadAccess: boolean;
        readonly asUpdateReadAccess: {
            readonly bucket: H256;
            readonly itemId: u32;
            readonly readAccessRegex: Option<Bytes>;
        } & Struct;
        readonly type: 'ShareAccess' | 'UpdateReadAccess';
    }
    /** @name PalletNftsCall (397) */
    interface PalletNftsCall extends Enum {
        readonly isCreate: boolean;
        readonly asCreate: {
            readonly admin: MultiAddress;
            readonly config: PalletNftsCollectionConfig;
        } & Struct;
        readonly isForceCreate: boolean;
        readonly asForceCreate: {
            readonly owner: MultiAddress;
            readonly config: PalletNftsCollectionConfig;
        } & Struct;
        readonly isDestroy: boolean;
        readonly asDestroy: {
            readonly collection: u32;
            readonly witness: PalletNftsDestroyWitness;
        } & Struct;
        readonly isMint: boolean;
        readonly asMint: {
            readonly collection: u32;
            readonly item: u32;
            readonly mintTo: MultiAddress;
            readonly witnessData: Option<PalletNftsMintWitness>;
        } & Struct;
        readonly isForceMint: boolean;
        readonly asForceMint: {
            readonly collection: u32;
            readonly item: u32;
            readonly mintTo: MultiAddress;
            readonly itemConfig: PalletNftsItemConfig;
        } & Struct;
        readonly isBurn: boolean;
        readonly asBurn: {
            readonly collection: u32;
            readonly item: u32;
        } & Struct;
        readonly isTransfer: boolean;
        readonly asTransfer: {
            readonly collection: u32;
            readonly item: u32;
            readonly dest: MultiAddress;
        } & Struct;
        readonly isRedeposit: boolean;
        readonly asRedeposit: {
            readonly collection: u32;
            readonly items: Vec<u32>;
        } & Struct;
        readonly isLockItemTransfer: boolean;
        readonly asLockItemTransfer: {
            readonly collection: u32;
            readonly item: u32;
        } & Struct;
        readonly isUnlockItemTransfer: boolean;
        readonly asUnlockItemTransfer: {
            readonly collection: u32;
            readonly item: u32;
        } & Struct;
        readonly isLockCollection: boolean;
        readonly asLockCollection: {
            readonly collection: u32;
            readonly lockSettings: u64;
        } & Struct;
        readonly isTransferOwnership: boolean;
        readonly asTransferOwnership: {
            readonly collection: u32;
            readonly newOwner: MultiAddress;
        } & Struct;
        readonly isSetTeam: boolean;
        readonly asSetTeam: {
            readonly collection: u32;
            readonly issuer: Option<MultiAddress>;
            readonly admin: Option<MultiAddress>;
            readonly freezer: Option<MultiAddress>;
        } & Struct;
        readonly isForceCollectionOwner: boolean;
        readonly asForceCollectionOwner: {
            readonly collection: u32;
            readonly owner: MultiAddress;
        } & Struct;
        readonly isForceCollectionConfig: boolean;
        readonly asForceCollectionConfig: {
            readonly collection: u32;
            readonly config: PalletNftsCollectionConfig;
        } & Struct;
        readonly isApproveTransfer: boolean;
        readonly asApproveTransfer: {
            readonly collection: u32;
            readonly item: u32;
            readonly delegate: MultiAddress;
            readonly maybeDeadline: Option<u32>;
        } & Struct;
        readonly isCancelApproval: boolean;
        readonly asCancelApproval: {
            readonly collection: u32;
            readonly item: u32;
            readonly delegate: MultiAddress;
        } & Struct;
        readonly isClearAllTransferApprovals: boolean;
        readonly asClearAllTransferApprovals: {
            readonly collection: u32;
            readonly item: u32;
        } & Struct;
        readonly isLockItemProperties: boolean;
        readonly asLockItemProperties: {
            readonly collection: u32;
            readonly item: u32;
            readonly lockMetadata: bool;
            readonly lockAttributes: bool;
        } & Struct;
        readonly isSetAttribute: boolean;
        readonly asSetAttribute: {
            readonly collection: u32;
            readonly maybeItem: Option<u32>;
            readonly namespace: PalletNftsAttributeNamespace;
            readonly key: Bytes;
            readonly value: Bytes;
        } & Struct;
        readonly isForceSetAttribute: boolean;
        readonly asForceSetAttribute: {
            readonly setAs: Option<AccountId32>;
            readonly collection: u32;
            readonly maybeItem: Option<u32>;
            readonly namespace: PalletNftsAttributeNamespace;
            readonly key: Bytes;
            readonly value: Bytes;
        } & Struct;
        readonly isClearAttribute: boolean;
        readonly asClearAttribute: {
            readonly collection: u32;
            readonly maybeItem: Option<u32>;
            readonly namespace: PalletNftsAttributeNamespace;
            readonly key: Bytes;
        } & Struct;
        readonly isApproveItemAttributes: boolean;
        readonly asApproveItemAttributes: {
            readonly collection: u32;
            readonly item: u32;
            readonly delegate: MultiAddress;
        } & Struct;
        readonly isCancelItemAttributesApproval: boolean;
        readonly asCancelItemAttributesApproval: {
            readonly collection: u32;
            readonly item: u32;
            readonly delegate: MultiAddress;
            readonly witness: PalletNftsCancelAttributesApprovalWitness;
        } & Struct;
        readonly isSetMetadata: boolean;
        readonly asSetMetadata: {
            readonly collection: u32;
            readonly item: u32;
            readonly data: Bytes;
        } & Struct;
        readonly isClearMetadata: boolean;
        readonly asClearMetadata: {
            readonly collection: u32;
            readonly item: u32;
        } & Struct;
        readonly isSetCollectionMetadata: boolean;
        readonly asSetCollectionMetadata: {
            readonly collection: u32;
            readonly data: Bytes;
        } & Struct;
        readonly isClearCollectionMetadata: boolean;
        readonly asClearCollectionMetadata: {
            readonly collection: u32;
        } & Struct;
        readonly isSetAcceptOwnership: boolean;
        readonly asSetAcceptOwnership: {
            readonly maybeCollection: Option<u32>;
        } & Struct;
        readonly isSetCollectionMaxSupply: boolean;
        readonly asSetCollectionMaxSupply: {
            readonly collection: u32;
            readonly maxSupply: u32;
        } & Struct;
        readonly isUpdateMintSettings: boolean;
        readonly asUpdateMintSettings: {
            readonly collection: u32;
            readonly mintSettings: PalletNftsMintSettings;
        } & Struct;
        readonly isSetPrice: boolean;
        readonly asSetPrice: {
            readonly collection: u32;
            readonly item: u32;
            readonly price: Option<u128>;
            readonly whitelistedBuyer: Option<MultiAddress>;
        } & Struct;
        readonly isBuyItem: boolean;
        readonly asBuyItem: {
            readonly collection: u32;
            readonly item: u32;
            readonly bidPrice: u128;
        } & Struct;
        readonly isPayTips: boolean;
        readonly asPayTips: {
            readonly tips: Vec<PalletNftsItemTip>;
        } & Struct;
        readonly isCreateSwap: boolean;
        readonly asCreateSwap: {
            readonly offeredCollection: u32;
            readonly offeredItem: u32;
            readonly desiredCollection: u32;
            readonly maybeDesiredItem: Option<u32>;
            readonly maybePrice: Option<PalletNftsPriceWithDirection>;
            readonly duration: u32;
        } & Struct;
        readonly isCancelSwap: boolean;
        readonly asCancelSwap: {
            readonly offeredCollection: u32;
            readonly offeredItem: u32;
        } & Struct;
        readonly isClaimSwap: boolean;
        readonly asClaimSwap: {
            readonly sendCollection: u32;
            readonly sendItem: u32;
            readonly receiveCollection: u32;
            readonly receiveItem: u32;
            readonly witnessPrice: Option<PalletNftsPriceWithDirection>;
        } & Struct;
        readonly isMintPreSigned: boolean;
        readonly asMintPreSigned: {
            readonly mintData: PalletNftsPreSignedMint;
            readonly signature: SpRuntimeMultiSignature;
            readonly signer: AccountId32;
        } & Struct;
        readonly isSetAttributesPreSigned: boolean;
        readonly asSetAttributesPreSigned: {
            readonly data: PalletNftsPreSignedAttributes;
            readonly signature: SpRuntimeMultiSignature;
            readonly signer: AccountId32;
        } & Struct;
        readonly type: 'Create' | 'ForceCreate' | 'Destroy' | 'Mint' | 'ForceMint' | 'Burn' | 'Transfer' | 'Redeposit' | 'LockItemTransfer' | 'UnlockItemTransfer' | 'LockCollection' | 'TransferOwnership' | 'SetTeam' | 'ForceCollectionOwner' | 'ForceCollectionConfig' | 'ApproveTransfer' | 'CancelApproval' | 'ClearAllTransferApprovals' | 'LockItemProperties' | 'SetAttribute' | 'ForceSetAttribute' | 'ClearAttribute' | 'ApproveItemAttributes' | 'CancelItemAttributesApproval' | 'SetMetadata' | 'ClearMetadata' | 'SetCollectionMetadata' | 'ClearCollectionMetadata' | 'SetAcceptOwnership' | 'SetCollectionMaxSupply' | 'UpdateMintSettings' | 'SetPrice' | 'BuyItem' | 'PayTips' | 'CreateSwap' | 'CancelSwap' | 'ClaimSwap' | 'MintPreSigned' | 'SetAttributesPreSigned';
    }
    /** @name PalletNftsCollectionConfig (398) */
    interface PalletNftsCollectionConfig extends Struct {
        readonly settings: u64;
        readonly maxSupply: Option<u32>;
        readonly mintSettings: PalletNftsMintSettings;
    }
    /** @name PalletNftsCollectionSetting (400) */
    interface PalletNftsCollectionSetting extends Enum {
        readonly isTransferableItems: boolean;
        readonly isUnlockedMetadata: boolean;
        readonly isUnlockedAttributes: boolean;
        readonly isUnlockedMaxSupply: boolean;
        readonly isDepositRequired: boolean;
        readonly type: 'TransferableItems' | 'UnlockedMetadata' | 'UnlockedAttributes' | 'UnlockedMaxSupply' | 'DepositRequired';
    }
    /** @name PalletNftsMintSettings (401) */
    interface PalletNftsMintSettings extends Struct {
        readonly mintType: PalletNftsMintType;
        readonly price: Option<u128>;
        readonly startBlock: Option<u32>;
        readonly endBlock: Option<u32>;
        readonly defaultItemSettings: u64;
    }
    /** @name PalletNftsMintType (402) */
    interface PalletNftsMintType extends Enum {
        readonly isIssuer: boolean;
        readonly isPublic: boolean;
        readonly isHolderOf: boolean;
        readonly asHolderOf: u32;
        readonly type: 'Issuer' | 'Public' | 'HolderOf';
    }
    /** @name PalletNftsItemSetting (405) */
    interface PalletNftsItemSetting extends Enum {
        readonly isTransferable: boolean;
        readonly isUnlockedMetadata: boolean;
        readonly isUnlockedAttributes: boolean;
        readonly type: 'Transferable' | 'UnlockedMetadata' | 'UnlockedAttributes';
    }
    /** @name PalletNftsDestroyWitness (406) */
    interface PalletNftsDestroyWitness extends Struct {
        readonly itemMetadatas: Compact<u32>;
        readonly itemConfigs: Compact<u32>;
        readonly attributes: Compact<u32>;
    }
    /** @name PalletNftsMintWitness (408) */
    interface PalletNftsMintWitness extends Struct {
        readonly ownedItem: Option<u32>;
        readonly mintPrice: Option<u128>;
    }
    /** @name PalletNftsItemConfig (409) */
    interface PalletNftsItemConfig extends Struct {
        readonly settings: u64;
    }
    /** @name PalletNftsCancelAttributesApprovalWitness (411) */
    interface PalletNftsCancelAttributesApprovalWitness extends Struct {
        readonly accountAttributes: u32;
    }
    /** @name PalletNftsItemTip (413) */
    interface PalletNftsItemTip extends Struct {
        readonly collection: u32;
        readonly item: u32;
        readonly receiver: AccountId32;
        readonly amount: u128;
    }
    /** @name PalletNftsPreSignedMint (415) */
    interface PalletNftsPreSignedMint extends Struct {
        readonly collection: u32;
        readonly item: u32;
        readonly attributes: Vec<ITuple<[Bytes, Bytes]>>;
        readonly metadata: Bytes;
        readonly onlyAccount: Option<AccountId32>;
        readonly deadline: u32;
        readonly mintPrice: Option<u128>;
    }
    /** @name PalletNftsPreSignedAttributes (416) */
    interface PalletNftsPreSignedAttributes extends Struct {
        readonly collection: u32;
        readonly item: u32;
        readonly attributes: Vec<ITuple<[Bytes, Bytes]>>;
        readonly namespace: PalletNftsAttributeNamespace;
        readonly deadline: u32;
    }
    /** @name PalletParametersCall (417) */
    interface PalletParametersCall extends Enum {
        readonly isSetParameter: boolean;
        readonly asSetParameter: {
            readonly keyValue: ShParachainRuntimeConfigsRuntimeParamsRuntimeParameters;
        } & Struct;
        readonly type: 'SetParameter';
    }
    /** @name ShParachainRuntimeConfigsRuntimeParamsRuntimeParameters (418) */
    interface ShParachainRuntimeConfigsRuntimeParamsRuntimeParameters extends Enum {
        readonly isRuntimeConfig: boolean;
        readonly asRuntimeConfig: ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigParameters;
        readonly type: 'RuntimeConfig';
    }
    /** @name ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigParameters (419) */
    interface ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigParameters extends Enum {
        readonly isSlashAmountPerMaxFileSize: boolean;
        readonly asSlashAmountPerMaxFileSize: ITuple<[ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigSlashAmountPerMaxFileSize, Option<u128>]>;
        readonly isStakeToChallengePeriod: boolean;
        readonly asStakeToChallengePeriod: ITuple<[ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigStakeToChallengePeriod, Option<u128>]>;
        readonly isCheckpointChallengePeriod: boolean;
        readonly asCheckpointChallengePeriod: ITuple<[ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigCheckpointChallengePeriod, Option<u32>]>;
        readonly isMinChallengePeriod: boolean;
        readonly asMinChallengePeriod: ITuple<[ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMinChallengePeriod, Option<u32>]>;
        readonly isSystemUtilisationLowerThresholdPercentage: boolean;
        readonly asSystemUtilisationLowerThresholdPercentage: ITuple<[ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigSystemUtilisationLowerThresholdPercentage, Option<Perbill>]>;
        readonly isSystemUtilisationUpperThresholdPercentage: boolean;
        readonly asSystemUtilisationUpperThresholdPercentage: ITuple<[ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigSystemUtilisationUpperThresholdPercentage, Option<Perbill>]>;
        readonly isMostlyStablePrice: boolean;
        readonly asMostlyStablePrice: ITuple<[ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMostlyStablePrice, Option<u128>]>;
        readonly isMaxPrice: boolean;
        readonly asMaxPrice: ITuple<[ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMaxPrice, Option<u128>]>;
        readonly isMinPrice: boolean;
        readonly asMinPrice: ITuple<[ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMinPrice, Option<u128>]>;
        readonly isUpperExponentFactor: boolean;
        readonly asUpperExponentFactor: ITuple<[ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigUpperExponentFactor, Option<u32>]>;
        readonly isLowerExponentFactor: boolean;
        readonly asLowerExponentFactor: ITuple<[ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigLowerExponentFactor, Option<u32>]>;
        readonly isZeroSizeBucketFixedRate: boolean;
        readonly asZeroSizeBucketFixedRate: ITuple<[ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigZeroSizeBucketFixedRate, Option<u128>]>;
        readonly isIdealUtilisationRate: boolean;
        readonly asIdealUtilisationRate: ITuple<[ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigIdealUtilisationRate, Option<Perbill>]>;
        readonly isDecayRate: boolean;
        readonly asDecayRate: ITuple<[ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigDecayRate, Option<Perbill>]>;
        readonly isMinimumTreasuryCut: boolean;
        readonly asMinimumTreasuryCut: ITuple<[ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMinimumTreasuryCut, Option<Perbill>]>;
        readonly isMaximumTreasuryCut: boolean;
        readonly asMaximumTreasuryCut: ITuple<[ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMaximumTreasuryCut, Option<Perbill>]>;
        readonly isBspStopStoringFilePenalty: boolean;
        readonly asBspStopStoringFilePenalty: ITuple<[ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigBspStopStoringFilePenalty, Option<u128>]>;
        readonly isProviderTopUpTtl: boolean;
        readonly asProviderTopUpTtl: ITuple<[ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigProviderTopUpTtl, Option<u32>]>;
        readonly isBasicReplicationTarget: boolean;
        readonly asBasicReplicationTarget: ITuple<[ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigBasicReplicationTarget, Option<u32>]>;
        readonly isStandardReplicationTarget: boolean;
        readonly asStandardReplicationTarget: ITuple<[ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigStandardReplicationTarget, Option<u32>]>;
        readonly isHighSecurityReplicationTarget: boolean;
        readonly asHighSecurityReplicationTarget: ITuple<[ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigHighSecurityReplicationTarget, Option<u32>]>;
        readonly isSuperHighSecurityReplicationTarget: boolean;
        readonly asSuperHighSecurityReplicationTarget: ITuple<[ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigSuperHighSecurityReplicationTarget, Option<u32>]>;
        readonly isUltraHighSecurityReplicationTarget: boolean;
        readonly asUltraHighSecurityReplicationTarget: ITuple<[ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigUltraHighSecurityReplicationTarget, Option<u32>]>;
        readonly isMaxReplicationTarget: boolean;
        readonly asMaxReplicationTarget: ITuple<[ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMaxReplicationTarget, Option<u32>]>;
        readonly isTickRangeToMaximumThreshold: boolean;
        readonly asTickRangeToMaximumThreshold: ITuple<[ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigTickRangeToMaximumThreshold, Option<u32>]>;
        readonly isStorageRequestTtl: boolean;
        readonly asStorageRequestTtl: ITuple<[ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigStorageRequestTtl, Option<u32>]>;
        readonly isMinWaitForStopStoring: boolean;
        readonly asMinWaitForStopStoring: ITuple<[ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMinWaitForStopStoring, Option<u32>]>;
        readonly isMinSeedPeriod: boolean;
        readonly asMinSeedPeriod: ITuple<[ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMinSeedPeriod, Option<u32>]>;
        readonly isStakeToSeedPeriod: boolean;
        readonly asStakeToSeedPeriod: ITuple<[ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigStakeToSeedPeriod, Option<u128>]>;
        readonly isUpfrontTicksToPay: boolean;
        readonly asUpfrontTicksToPay: ITuple<[ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigUpfrontTicksToPay, Option<u32>]>;
        readonly type: 'SlashAmountPerMaxFileSize' | 'StakeToChallengePeriod' | 'CheckpointChallengePeriod' | 'MinChallengePeriod' | 'SystemUtilisationLowerThresholdPercentage' | 'SystemUtilisationUpperThresholdPercentage' | 'MostlyStablePrice' | 'MaxPrice' | 'MinPrice' | 'UpperExponentFactor' | 'LowerExponentFactor' | 'ZeroSizeBucketFixedRate' | 'IdealUtilisationRate' | 'DecayRate' | 'MinimumTreasuryCut' | 'MaximumTreasuryCut' | 'BspStopStoringFilePenalty' | 'ProviderTopUpTtl' | 'BasicReplicationTarget' | 'StandardReplicationTarget' | 'HighSecurityReplicationTarget' | 'SuperHighSecurityReplicationTarget' | 'UltraHighSecurityReplicationTarget' | 'MaxReplicationTarget' | 'TickRangeToMaximumThreshold' | 'StorageRequestTtl' | 'MinWaitForStopStoring' | 'MinSeedPeriod' | 'StakeToSeedPeriod' | 'UpfrontTicksToPay';
    }
    /** @name PalletSudoError (421) */
    interface PalletSudoError extends Enum {
        readonly isRequireSudo: boolean;
        readonly type: 'RequireSudo';
    }
    /** @name PalletCollatorSelectionCandidateInfo (424) */
    interface PalletCollatorSelectionCandidateInfo extends Struct {
        readonly who: AccountId32;
        readonly deposit: u128;
    }
    /** @name PalletCollatorSelectionError (426) */
    interface PalletCollatorSelectionError extends Enum {
        readonly isTooManyCandidates: boolean;
        readonly isTooFewEligibleCollators: boolean;
        readonly isAlreadyCandidate: boolean;
        readonly isNotCandidate: boolean;
        readonly isTooManyInvulnerables: boolean;
        readonly isAlreadyInvulnerable: boolean;
        readonly isNotInvulnerable: boolean;
        readonly isNoAssociatedValidatorId: boolean;
        readonly isValidatorNotRegistered: boolean;
        readonly isInsertToCandidateListFailed: boolean;
        readonly isRemoveFromCandidateListFailed: boolean;
        readonly isDepositTooLow: boolean;
        readonly isUpdateCandidateListFailed: boolean;
        readonly isInsufficientBond: boolean;
        readonly isTargetIsNotCandidate: boolean;
        readonly isIdenticalDeposit: boolean;
        readonly isInvalidUnreserve: boolean;
        readonly type: 'TooManyCandidates' | 'TooFewEligibleCollators' | 'AlreadyCandidate' | 'NotCandidate' | 'TooManyInvulnerables' | 'AlreadyInvulnerable' | 'NotInvulnerable' | 'NoAssociatedValidatorId' | 'ValidatorNotRegistered' | 'InsertToCandidateListFailed' | 'RemoveFromCandidateListFailed' | 'DepositTooLow' | 'UpdateCandidateListFailed' | 'InsufficientBond' | 'TargetIsNotCandidate' | 'IdenticalDeposit' | 'InvalidUnreserve';
    }
    /** @name SpCoreCryptoKeyTypeId (430) */
    interface SpCoreCryptoKeyTypeId extends U8aFixed {
    }
    /** @name PalletSessionError (431) */
    interface PalletSessionError extends Enum {
        readonly isInvalidProof: boolean;
        readonly isNoAssociatedValidatorId: boolean;
        readonly isDuplicatedKey: boolean;
        readonly isNoKeys: boolean;
        readonly isNoAccount: boolean;
        readonly type: 'InvalidProof' | 'NoAssociatedValidatorId' | 'DuplicatedKey' | 'NoKeys' | 'NoAccount';
    }
    /** @name CumulusPalletXcmpQueueOutboundChannelDetails (440) */
    interface CumulusPalletXcmpQueueOutboundChannelDetails extends Struct {
        readonly recipient: u32;
        readonly state: CumulusPalletXcmpQueueOutboundState;
        readonly signalsExist: bool;
        readonly firstIndex: u16;
        readonly lastIndex: u16;
    }
    /** @name CumulusPalletXcmpQueueOutboundState (441) */
    interface CumulusPalletXcmpQueueOutboundState extends Enum {
        readonly isOk: boolean;
        readonly isSuspended: boolean;
        readonly type: 'Ok' | 'Suspended';
    }
    /** @name CumulusPalletXcmpQueueQueueConfigData (445) */
    interface CumulusPalletXcmpQueueQueueConfigData extends Struct {
        readonly suspendThreshold: u32;
        readonly dropThreshold: u32;
        readonly resumeThreshold: u32;
    }
    /** @name CumulusPalletXcmpQueueError (446) */
    interface CumulusPalletXcmpQueueError extends Enum {
        readonly isBadQueueConfig: boolean;
        readonly isAlreadySuspended: boolean;
        readonly isAlreadyResumed: boolean;
        readonly isTooManyActiveOutboundChannels: boolean;
        readonly isTooBig: boolean;
        readonly type: 'BadQueueConfig' | 'AlreadySuspended' | 'AlreadyResumed' | 'TooManyActiveOutboundChannels' | 'TooBig';
    }
    /** @name PalletXcmQueryStatus (447) */
    interface PalletXcmQueryStatus extends Enum {
        readonly isPending: boolean;
        readonly asPending: {
            readonly responder: XcmVersionedLocation;
            readonly maybeMatchQuerier: Option<XcmVersionedLocation>;
            readonly maybeNotify: Option<ITuple<[u8, u8]>>;
            readonly timeout: u32;
        } & Struct;
        readonly isVersionNotifier: boolean;
        readonly asVersionNotifier: {
            readonly origin: XcmVersionedLocation;
            readonly isActive: bool;
        } & Struct;
        readonly isReady: boolean;
        readonly asReady: {
            readonly response: XcmVersionedResponse;
            readonly at: u32;
        } & Struct;
        readonly type: 'Pending' | 'VersionNotifier' | 'Ready';
    }
    /** @name XcmVersionedResponse (451) */
    interface XcmVersionedResponse extends Enum {
        readonly isV3: boolean;
        readonly asV3: XcmV3Response;
        readonly isV4: boolean;
        readonly asV4: StagingXcmV4Response;
        readonly isV5: boolean;
        readonly asV5: StagingXcmV5Response;
        readonly type: 'V3' | 'V4' | 'V5';
    }
    /** @name PalletXcmVersionMigrationStage (457) */
    interface PalletXcmVersionMigrationStage extends Enum {
        readonly isMigrateSupportedVersion: boolean;
        readonly isMigrateVersionNotifiers: boolean;
        readonly isNotifyCurrentTargets: boolean;
        readonly asNotifyCurrentTargets: Option<Bytes>;
        readonly isMigrateAndNotifyOldTargets: boolean;
        readonly type: 'MigrateSupportedVersion' | 'MigrateVersionNotifiers' | 'NotifyCurrentTargets' | 'MigrateAndNotifyOldTargets';
    }
    /** @name PalletXcmRemoteLockedFungibleRecord (459) */
    interface PalletXcmRemoteLockedFungibleRecord extends Struct {
        readonly amount: u128;
        readonly owner: XcmVersionedLocation;
        readonly locker: XcmVersionedLocation;
        readonly consumers: Vec<ITuple<[Null, u128]>>;
    }
    /** @name PalletXcmError (466) */
    interface PalletXcmError extends Enum {
        readonly isUnreachable: boolean;
        readonly isSendFailure: boolean;
        readonly isFiltered: boolean;
        readonly isUnweighableMessage: boolean;
        readonly isDestinationNotInvertible: boolean;
        readonly isEmpty: boolean;
        readonly isCannotReanchor: boolean;
        readonly isTooManyAssets: boolean;
        readonly isInvalidOrigin: boolean;
        readonly isBadVersion: boolean;
        readonly isBadLocation: boolean;
        readonly isNoSubscription: boolean;
        readonly isAlreadySubscribed: boolean;
        readonly isCannotCheckOutTeleport: boolean;
        readonly isLowBalance: boolean;
        readonly isTooManyLocks: boolean;
        readonly isAccountNotSovereign: boolean;
        readonly isFeesNotMet: boolean;
        readonly isLockNotFound: boolean;
        readonly isInUse: boolean;
        readonly isInvalidAssetUnknownReserve: boolean;
        readonly isInvalidAssetUnsupportedReserve: boolean;
        readonly isTooManyReserves: boolean;
        readonly isLocalExecutionIncomplete: boolean;
        readonly type: 'Unreachable' | 'SendFailure' | 'Filtered' | 'UnweighableMessage' | 'DestinationNotInvertible' | 'Empty' | 'CannotReanchor' | 'TooManyAssets' | 'InvalidOrigin' | 'BadVersion' | 'BadLocation' | 'NoSubscription' | 'AlreadySubscribed' | 'CannotCheckOutTeleport' | 'LowBalance' | 'TooManyLocks' | 'AccountNotSovereign' | 'FeesNotMet' | 'LockNotFound' | 'InUse' | 'InvalidAssetUnknownReserve' | 'InvalidAssetUnsupportedReserve' | 'TooManyReserves' | 'LocalExecutionIncomplete';
    }
    /** @name PalletMessageQueueBookState (467) */
    interface PalletMessageQueueBookState extends Struct {
        readonly begin: u32;
        readonly end: u32;
        readonly count: u32;
        readonly readyNeighbours: Option<PalletMessageQueueNeighbours>;
        readonly messageCount: u64;
        readonly size_: u64;
    }
    /** @name PalletMessageQueueNeighbours (469) */
    interface PalletMessageQueueNeighbours extends Struct {
        readonly prev: CumulusPrimitivesCoreAggregateMessageOrigin;
        readonly next: CumulusPrimitivesCoreAggregateMessageOrigin;
    }
    /** @name PalletMessageQueuePage (471) */
    interface PalletMessageQueuePage extends Struct {
        readonly remaining: u32;
        readonly remainingSize: u32;
        readonly firstIndex: u32;
        readonly first: u32;
        readonly last: u32;
        readonly heap: Bytes;
    }
    /** @name PalletMessageQueueError (473) */
    interface PalletMessageQueueError extends Enum {
        readonly isNotReapable: boolean;
        readonly isNoPage: boolean;
        readonly isNoMessage: boolean;
        readonly isAlreadyProcessed: boolean;
        readonly isQueued: boolean;
        readonly isInsufficientWeight: boolean;
        readonly isTemporarilyUnprocessable: boolean;
        readonly isQueuePaused: boolean;
        readonly isRecursiveDisallowed: boolean;
        readonly type: 'NotReapable' | 'NoPage' | 'NoMessage' | 'AlreadyProcessed' | 'Queued' | 'InsufficientWeight' | 'TemporarilyUnprocessable' | 'QueuePaused' | 'RecursiveDisallowed';
    }
    /** @name PalletStorageProvidersSignUpRequest (474) */
    interface PalletStorageProvidersSignUpRequest extends Struct {
        readonly spSignUpRequest: PalletStorageProvidersSignUpRequestSpParams;
        readonly at: u32;
    }
    /** @name PalletStorageProvidersSignUpRequestSpParams (475) */
    interface PalletStorageProvidersSignUpRequestSpParams extends Enum {
        readonly isBackupStorageProvider: boolean;
        readonly asBackupStorageProvider: PalletStorageProvidersBackupStorageProvider;
        readonly isMainStorageProvider: boolean;
        readonly asMainStorageProvider: PalletStorageProvidersMainStorageProviderSignUpRequest;
        readonly type: 'BackupStorageProvider' | 'MainStorageProvider';
    }
    /** @name PalletStorageProvidersBackupStorageProvider (476) */
    interface PalletStorageProvidersBackupStorageProvider extends Struct {
        readonly capacity: u64;
        readonly capacityUsed: u64;
        readonly multiaddresses: Vec<Bytes>;
        readonly root: H256;
        readonly lastCapacityChange: u32;
        readonly ownerAccount: AccountId32;
        readonly paymentAccount: AccountId32;
        readonly reputationWeight: u32;
        readonly signUpBlock: u32;
    }
    /** @name PalletStorageProvidersMainStorageProviderSignUpRequest (477) */
    interface PalletStorageProvidersMainStorageProviderSignUpRequest extends Struct {
        readonly mspInfo: PalletStorageProvidersMainStorageProvider;
        readonly valueProp: PalletStorageProvidersValueProposition;
    }
    /** @name PalletStorageProvidersMainStorageProvider (478) */
    interface PalletStorageProvidersMainStorageProvider extends Struct {
        readonly capacity: u64;
        readonly capacityUsed: u64;
        readonly multiaddresses: Vec<Bytes>;
        readonly amountOfBuckets: u128;
        readonly amountOfValueProps: u32;
        readonly lastCapacityChange: u32;
        readonly ownerAccount: AccountId32;
        readonly paymentAccount: AccountId32;
        readonly signUpBlock: u32;
    }
    /** @name PalletStorageProvidersBucket (479) */
    interface PalletStorageProvidersBucket extends Struct {
        readonly root: H256;
        readonly userId: AccountId32;
        readonly mspId: Option<H256>;
        readonly private: bool;
        readonly readAccessGroupId: Option<u32>;
        readonly size_: u64;
        readonly valuePropId: H256;
    }
    /** @name PalletStorageProvidersError (483) */
    interface PalletStorageProvidersError extends Enum {
        readonly isAlreadyRegistered: boolean;
        readonly isSignUpNotRequested: boolean;
        readonly isSignUpRequestPending: boolean;
        readonly isNoMultiAddress: boolean;
        readonly isInvalidMultiAddress: boolean;
        readonly isStorageTooLow: boolean;
        readonly isNotEnoughBalance: boolean;
        readonly isCannotHoldDeposit: boolean;
        readonly isStorageStillInUse: boolean;
        readonly isSignOffPeriodNotPassed: boolean;
        readonly isRandomnessNotValidYet: boolean;
        readonly isSignUpRequestExpired: boolean;
        readonly isNewCapacityLessThanUsedStorage: boolean;
        readonly isNewCapacityEqualsCurrentCapacity: boolean;
        readonly isNewCapacityCantBeZero: boolean;
        readonly isNotEnoughTimePassed: boolean;
        readonly isNewUsedCapacityExceedsStorageCapacity: boolean;
        readonly isDepositTooLow: boolean;
        readonly isNotRegistered: boolean;
        readonly isNoUserId: boolean;
        readonly isNoBucketId: boolean;
        readonly isSpRegisteredButDataNotFound: boolean;
        readonly isBucketNotFound: boolean;
        readonly isBucketAlreadyExists: boolean;
        readonly isBucketNotEmpty: boolean;
        readonly isBucketsMovedAmountMismatch: boolean;
        readonly isAppendBucketToMspFailed: boolean;
        readonly isProviderNotSlashable: boolean;
        readonly isTopUpNotRequired: boolean;
        readonly isBucketMustHaveMspForOperation: boolean;
        readonly isMultiAddressesMaxAmountReached: boolean;
        readonly isMultiAddressNotFound: boolean;
        readonly isMultiAddressAlreadyExists: boolean;
        readonly isLastMultiAddressCantBeRemoved: boolean;
        readonly isValuePropositionNotFound: boolean;
        readonly isValuePropositionAlreadyExists: boolean;
        readonly isValuePropositionNotAvailable: boolean;
        readonly isCantDeactivateLastValueProp: boolean;
        readonly isValuePropositionsDeletedAmountMismatch: boolean;
        readonly isFixedRatePaymentStreamNotFound: boolean;
        readonly isMspAlreadyAssignedToBucket: boolean;
        readonly isBucketSizeExceedsLimit: boolean;
        readonly isBucketHasNoValueProposition: boolean;
        readonly isMaxBlockNumberReached: boolean;
        readonly isOperationNotAllowedForInsolventProvider: boolean;
        readonly isDeleteProviderConditionsNotMet: boolean;
        readonly isCannotStopCycleWithNonDefaultRoot: boolean;
        readonly isBspOnlyOperation: boolean;
        readonly isMspOnlyOperation: boolean;
        readonly isInvalidEncodedFileMetadata: boolean;
        readonly isInvalidEncodedAccountId: boolean;
        readonly isPaymentStreamNotFound: boolean;
        readonly type: 'AlreadyRegistered' | 'SignUpNotRequested' | 'SignUpRequestPending' | 'NoMultiAddress' | 'InvalidMultiAddress' | 'StorageTooLow' | 'NotEnoughBalance' | 'CannotHoldDeposit' | 'StorageStillInUse' | 'SignOffPeriodNotPassed' | 'RandomnessNotValidYet' | 'SignUpRequestExpired' | 'NewCapacityLessThanUsedStorage' | 'NewCapacityEqualsCurrentCapacity' | 'NewCapacityCantBeZero' | 'NotEnoughTimePassed' | 'NewUsedCapacityExceedsStorageCapacity' | 'DepositTooLow' | 'NotRegistered' | 'NoUserId' | 'NoBucketId' | 'SpRegisteredButDataNotFound' | 'BucketNotFound' | 'BucketAlreadyExists' | 'BucketNotEmpty' | 'BucketsMovedAmountMismatch' | 'AppendBucketToMspFailed' | 'ProviderNotSlashable' | 'TopUpNotRequired' | 'BucketMustHaveMspForOperation' | 'MultiAddressesMaxAmountReached' | 'MultiAddressNotFound' | 'MultiAddressAlreadyExists' | 'LastMultiAddressCantBeRemoved' | 'ValuePropositionNotFound' | 'ValuePropositionAlreadyExists' | 'ValuePropositionNotAvailable' | 'CantDeactivateLastValueProp' | 'ValuePropositionsDeletedAmountMismatch' | 'FixedRatePaymentStreamNotFound' | 'MspAlreadyAssignedToBucket' | 'BucketSizeExceedsLimit' | 'BucketHasNoValueProposition' | 'MaxBlockNumberReached' | 'OperationNotAllowedForInsolventProvider' | 'DeleteProviderConditionsNotMet' | 'CannotStopCycleWithNonDefaultRoot' | 'BspOnlyOperation' | 'MspOnlyOperation' | 'InvalidEncodedFileMetadata' | 'InvalidEncodedAccountId' | 'PaymentStreamNotFound';
    }
    /** @name PalletFileSystemStorageRequestMetadata (484) */
    interface PalletFileSystemStorageRequestMetadata extends Struct {
        readonly requestedAt: u32;
        readonly expiresAt: u32;
        readonly owner: AccountId32;
        readonly bucketId: H256;
        readonly location: Bytes;
        readonly fingerprint: H256;
        readonly size_: u64;
        readonly mspStatus: PalletFileSystemMspStorageRequestStatus;
        readonly userPeerIds: Vec<Bytes>;
        readonly bspsRequired: u32;
        readonly bspsConfirmed: u32;
        readonly bspsVolunteered: u32;
        readonly depositPaid: u128;
    }
    /** @name PalletFileSystemMspStorageRequestStatus (485) */
    interface PalletFileSystemMspStorageRequestStatus extends Enum {
        readonly isNone: boolean;
        readonly isPending: boolean;
        readonly asPending: H256;
        readonly isAcceptedNewFile: boolean;
        readonly asAcceptedNewFile: H256;
        readonly isAcceptedExistingFile: boolean;
        readonly asAcceptedExistingFile: H256;
        readonly type: 'None' | 'Pending' | 'AcceptedNewFile' | 'AcceptedExistingFile';
    }
    /** @name PalletFileSystemStorageRequestBspsMetadata (486) */
    interface PalletFileSystemStorageRequestBspsMetadata extends Struct {
        readonly confirmed: bool;
    }
    /** @name PalletFileSystemPendingFileDeletionRequest (488) */
    interface PalletFileSystemPendingFileDeletionRequest extends Struct {
        readonly user: AccountId32;
        readonly fileKey: H256;
        readonly bucketId: H256;
        readonly fileSize: u64;
        readonly depositPaidForCreation: u128;
        readonly queuePriorityChallenge: bool;
    }
    /** @name PalletFileSystemPendingStopStoringRequest (490) */
    interface PalletFileSystemPendingStopStoringRequest extends Struct {
        readonly tickWhenRequested: u32;
        readonly fileOwner: AccountId32;
        readonly fileSize: u64;
    }
    /** @name PalletFileSystemMoveBucketRequestMetadata (491) */
    interface PalletFileSystemMoveBucketRequestMetadata extends Struct {
        readonly requester: AccountId32;
        readonly newMspId: H256;
        readonly newValuePropId: H256;
    }
    /** @name PalletFileSystemIncompleteStorageRequestMetadata (492) */
    interface PalletFileSystemIncompleteStorageRequestMetadata extends Struct {
        readonly owner: AccountId32;
        readonly bucketId: H256;
        readonly location: Bytes;
        readonly fileSize: u64;
        readonly fingerprint: H256;
        readonly pendingBspRemovals: Vec<H256>;
        readonly pendingBucketRemoval: bool;
    }
    /** @name PalletFileSystemError (494) */
    interface PalletFileSystemError extends Enum {
        readonly isNotABsp: boolean;
        readonly isNotAMsp: boolean;
        readonly isNotASp: boolean;
        readonly isStorageRequestAlreadyRegistered: boolean;
        readonly isStorageRequestNotFound: boolean;
        readonly isStorageRequestExists: boolean;
        readonly isStorageRequestNotAuthorized: boolean;
        readonly isStorageRequestBspsRequiredFulfilled: boolean;
        readonly isTooManyStorageRequestResponses: boolean;
        readonly isIncompleteStorageRequestNotFound: boolean;
        readonly isReplicationTargetCannotBeZero: boolean;
        readonly isReplicationTargetExceedsMaximum: boolean;
        readonly isBspNotVolunteered: boolean;
        readonly isBspNotConfirmed: boolean;
        readonly isBspAlreadyConfirmed: boolean;
        readonly isBspAlreadyVolunteered: boolean;
        readonly isBspNotEligibleToVolunteer: boolean;
        readonly isInsufficientAvailableCapacity: boolean;
        readonly isNoFileKeysToConfirm: boolean;
        readonly isMspNotStoringBucket: boolean;
        readonly isNotSelectedMsp: boolean;
        readonly isMspAlreadyConfirmed: boolean;
        readonly isRequestWithoutMsp: boolean;
        readonly isMspAlreadyStoringBucket: boolean;
        readonly isBucketNotFound: boolean;
        readonly isBucketNotEmpty: boolean;
        readonly isNotBucketOwner: boolean;
        readonly isBucketIsBeingMoved: boolean;
        readonly isInvalidBucketIdFileKeyPair: boolean;
        readonly isValuePropositionNotAvailable: boolean;
        readonly isCollectionNotFound: boolean;
        readonly isMoveBucketRequestNotFound: boolean;
        readonly isInvalidFileKeyMetadata: boolean;
        readonly isFileSizeCannotBeZero: boolean;
        readonly isProviderNotStoringFile: boolean;
        readonly isFileHasActiveStorageRequest: boolean;
        readonly isFileHasIncompleteStorageRequest: boolean;
        readonly isBatchFileDeletionMustContainSingleBucket: boolean;
        readonly isDuplicateFileKeyInBatchFileDeletion: boolean;
        readonly isNoFileKeysToDelete: boolean;
        readonly isFailedToPushFileKeyToBucketDeletionVector: boolean;
        readonly isFailedToPushUserToBspDeletionVector: boolean;
        readonly isFailedToPushFileKeyToBspDeletionVector: boolean;
        readonly isPendingStopStoringRequestNotFound: boolean;
        readonly isMinWaitForStopStoringNotReached: boolean;
        readonly isPendingStopStoringRequestAlreadyExists: boolean;
        readonly isExpectedNonInclusionProof: boolean;
        readonly isExpectedInclusionProof: boolean;
        readonly isFixedRatePaymentStreamNotFound: boolean;
        readonly isDynamicRatePaymentStreamNotFound: boolean;
        readonly isOperationNotAllowedWithInsolventUser: boolean;
        readonly isUserNotInsolvent: boolean;
        readonly isOperationNotAllowedForInsolventProvider: boolean;
        readonly isInvalidSignature: boolean;
        readonly isInvalidProviderID: boolean;
        readonly isInvalidSignedOperation: boolean;
        readonly isNoGlobalReputationWeightSet: boolean;
        readonly isNoBspReputationWeightSet: boolean;
        readonly isCannotHoldDeposit: boolean;
        readonly isMaxTickNumberReached: boolean;
        readonly isThresholdArithmeticError: boolean;
        readonly isRootNotUpdated: boolean;
        readonly isImpossibleFailedToGetValue: boolean;
        readonly isFailedToQueryEarliestFileVolunteerTick: boolean;
        readonly isFailedToGetOwnerAccount: boolean;
        readonly isFailedToGetPaymentAccount: boolean;
        readonly isFailedToComputeFileKey: boolean;
        readonly isFailedToCreateFileMetadata: boolean;
        readonly isFileMetadataProcessingQueueFull: boolean;
        readonly type: 'NotABsp' | 'NotAMsp' | 'NotASp' | 'StorageRequestAlreadyRegistered' | 'StorageRequestNotFound' | 'StorageRequestExists' | 'StorageRequestNotAuthorized' | 'StorageRequestBspsRequiredFulfilled' | 'TooManyStorageRequestResponses' | 'IncompleteStorageRequestNotFound' | 'ReplicationTargetCannotBeZero' | 'ReplicationTargetExceedsMaximum' | 'BspNotVolunteered' | 'BspNotConfirmed' | 'BspAlreadyConfirmed' | 'BspAlreadyVolunteered' | 'BspNotEligibleToVolunteer' | 'InsufficientAvailableCapacity' | 'NoFileKeysToConfirm' | 'MspNotStoringBucket' | 'NotSelectedMsp' | 'MspAlreadyConfirmed' | 'RequestWithoutMsp' | 'MspAlreadyStoringBucket' | 'BucketNotFound' | 'BucketNotEmpty' | 'NotBucketOwner' | 'BucketIsBeingMoved' | 'InvalidBucketIdFileKeyPair' | 'ValuePropositionNotAvailable' | 'CollectionNotFound' | 'MoveBucketRequestNotFound' | 'InvalidFileKeyMetadata' | 'FileSizeCannotBeZero' | 'ProviderNotStoringFile' | 'FileHasActiveStorageRequest' | 'FileHasIncompleteStorageRequest' | 'BatchFileDeletionMustContainSingleBucket' | 'DuplicateFileKeyInBatchFileDeletion' | 'NoFileKeysToDelete' | 'FailedToPushFileKeyToBucketDeletionVector' | 'FailedToPushUserToBspDeletionVector' | 'FailedToPushFileKeyToBspDeletionVector' | 'PendingStopStoringRequestNotFound' | 'MinWaitForStopStoringNotReached' | 'PendingStopStoringRequestAlreadyExists' | 'ExpectedNonInclusionProof' | 'ExpectedInclusionProof' | 'FixedRatePaymentStreamNotFound' | 'DynamicRatePaymentStreamNotFound' | 'OperationNotAllowedWithInsolventUser' | 'UserNotInsolvent' | 'OperationNotAllowedForInsolventProvider' | 'InvalidSignature' | 'InvalidProviderID' | 'InvalidSignedOperation' | 'NoGlobalReputationWeightSet' | 'NoBspReputationWeightSet' | 'CannotHoldDeposit' | 'MaxTickNumberReached' | 'ThresholdArithmeticError' | 'RootNotUpdated' | 'ImpossibleFailedToGetValue' | 'FailedToQueryEarliestFileVolunteerTick' | 'FailedToGetOwnerAccount' | 'FailedToGetPaymentAccount' | 'FailedToComputeFileKey' | 'FailedToCreateFileMetadata' | 'FileMetadataProcessingQueueFull';
    }
    /** @name PalletProofsDealerProofSubmissionRecord (496) */
    interface PalletProofsDealerProofSubmissionRecord extends Struct {
        readonly lastTickProven: u32;
        readonly nextTickToSubmitProofFor: u32;
    }
    /** @name PalletProofsDealerError (503) */
    interface PalletProofsDealerError extends Enum {
        readonly isNotProvider: boolean;
        readonly isChallengesQueueOverflow: boolean;
        readonly isPriorityChallengesQueueOverflow: boolean;
        readonly isFeeChargeFailed: boolean;
        readonly isEmptyKeyProofs: boolean;
        readonly isProviderRootNotFound: boolean;
        readonly isZeroRoot: boolean;
        readonly isNoRecordOfLastSubmittedProof: boolean;
        readonly isProviderStakeNotFound: boolean;
        readonly isZeroStake: boolean;
        readonly isStakeCouldNotBeConverted: boolean;
        readonly isChallengesTickNotReached: boolean;
        readonly isChallengesTickTooOld: boolean;
        readonly isChallengesTickTooLate: boolean;
        readonly isSeedNotFound: boolean;
        readonly isCheckpointChallengesNotFound: boolean;
        readonly isForestProofVerificationFailed: boolean;
        readonly isIncorrectNumberOfKeyProofs: boolean;
        readonly isKeyProofNotFound: boolean;
        readonly isKeyProofVerificationFailed: boolean;
        readonly isFailedToApplyDelta: boolean;
        readonly isUnexpectedNumberOfRemoveMutations: boolean;
        readonly isFailedToUpdateProviderAfterKeyRemoval: boolean;
        readonly isTooManyValidProofSubmitters: boolean;
        readonly type: 'NotProvider' | 'ChallengesQueueOverflow' | 'PriorityChallengesQueueOverflow' | 'FeeChargeFailed' | 'EmptyKeyProofs' | 'ProviderRootNotFound' | 'ZeroRoot' | 'NoRecordOfLastSubmittedProof' | 'ProviderStakeNotFound' | 'ZeroStake' | 'StakeCouldNotBeConverted' | 'ChallengesTickNotReached' | 'ChallengesTickTooOld' | 'ChallengesTickTooLate' | 'SeedNotFound' | 'CheckpointChallengesNotFound' | 'ForestProofVerificationFailed' | 'IncorrectNumberOfKeyProofs' | 'KeyProofNotFound' | 'KeyProofVerificationFailed' | 'FailedToApplyDelta' | 'UnexpectedNumberOfRemoveMutations' | 'FailedToUpdateProviderAfterKeyRemoval' | 'TooManyValidProofSubmitters';
    }
    /** @name PalletPaymentStreamsFixedRatePaymentStream (506) */
    interface PalletPaymentStreamsFixedRatePaymentStream extends Struct {
        readonly rate: u128;
        readonly lastChargedTick: u32;
        readonly userDeposit: u128;
        readonly outOfFundsTick: Option<u32>;
    }
    /** @name PalletPaymentStreamsDynamicRatePaymentStream (507) */
    interface PalletPaymentStreamsDynamicRatePaymentStream extends Struct {
        readonly amountProvided: u64;
        readonly priceIndexWhenLastCharged: u128;
        readonly userDeposit: u128;
        readonly outOfFundsTick: Option<u32>;
    }
    /** @name PalletPaymentStreamsProviderLastChargeableInfo (508) */
    interface PalletPaymentStreamsProviderLastChargeableInfo extends Struct {
        readonly lastChargeableTick: u32;
        readonly priceIndex: u128;
    }
    /** @name PalletPaymentStreamsError (509) */
    interface PalletPaymentStreamsError extends Enum {
        readonly isPaymentStreamAlreadyExists: boolean;
        readonly isPaymentStreamNotFound: boolean;
        readonly isNotAProvider: boolean;
        readonly isProviderInconsistencyError: boolean;
        readonly isCannotHoldDeposit: boolean;
        readonly isUpdateRateToSameRate: boolean;
        readonly isUpdateAmountToSameAmount: boolean;
        readonly isRateCantBeZero: boolean;
        readonly isAmountProvidedCantBeZero: boolean;
        readonly isLastChargedGreaterThanLastChargeable: boolean;
        readonly isInvalidLastChargeableBlockNumber: boolean;
        readonly isInvalidLastChargeablePriceIndex: boolean;
        readonly isChargeOverflow: boolean;
        readonly isUserWithoutFunds: boolean;
        readonly isUserNotFlaggedAsWithoutFunds: boolean;
        readonly isCooldownPeriodNotPassed: boolean;
        readonly isUserHasRemainingDebt: boolean;
        readonly isProviderInsolvent: boolean;
        readonly type: 'PaymentStreamAlreadyExists' | 'PaymentStreamNotFound' | 'NotAProvider' | 'ProviderInconsistencyError' | 'CannotHoldDeposit' | 'UpdateRateToSameRate' | 'UpdateAmountToSameAmount' | 'RateCantBeZero' | 'AmountProvidedCantBeZero' | 'LastChargedGreaterThanLastChargeable' | 'InvalidLastChargeableBlockNumber' | 'InvalidLastChargeablePriceIndex' | 'ChargeOverflow' | 'UserWithoutFunds' | 'UserNotFlaggedAsWithoutFunds' | 'CooldownPeriodNotPassed' | 'UserHasRemainingDebt' | 'ProviderInsolvent';
    }
    /** @name PalletBucketNftsError (510) */
    interface PalletBucketNftsError extends Enum {
        readonly isBucketIsNotPrivate: boolean;
        readonly isNotBucketOwner: boolean;
        readonly isNoCorrespondingCollection: boolean;
        readonly isConvertBytesToBoundedVec: boolean;
        readonly type: 'BucketIsNotPrivate' | 'NotBucketOwner' | 'NoCorrespondingCollection' | 'ConvertBytesToBoundedVec';
    }
    /** @name PalletNftsCollectionDetails (511) */
    interface PalletNftsCollectionDetails extends Struct {
        readonly owner: AccountId32;
        readonly ownerDeposit: u128;
        readonly items: u32;
        readonly itemMetadatas: u32;
        readonly itemConfigs: u32;
        readonly attributes: u32;
    }
    /** @name PalletNftsCollectionRole (516) */
    interface PalletNftsCollectionRole extends Enum {
        readonly isIssuer: boolean;
        readonly isFreezer: boolean;
        readonly isAdmin: boolean;
        readonly type: 'Issuer' | 'Freezer' | 'Admin';
    }
    /** @name PalletNftsItemDetails (517) */
    interface PalletNftsItemDetails extends Struct {
        readonly owner: AccountId32;
        readonly approvals: BTreeMap<AccountId32, Option<u32>>;
        readonly deposit: PalletNftsItemDeposit;
    }
    /** @name PalletNftsItemDeposit (518) */
    interface PalletNftsItemDeposit extends Struct {
        readonly account: AccountId32;
        readonly amount: u128;
    }
    /** @name PalletNftsCollectionMetadata (523) */
    interface PalletNftsCollectionMetadata extends Struct {
        readonly deposit: u128;
        readonly data: Bytes;
    }
    /** @name PalletNftsItemMetadata (524) */
    interface PalletNftsItemMetadata extends Struct {
        readonly deposit: PalletNftsItemMetadataDeposit;
        readonly data: Bytes;
    }
    /** @name PalletNftsItemMetadataDeposit (525) */
    interface PalletNftsItemMetadataDeposit extends Struct {
        readonly account: Option<AccountId32>;
        readonly amount: u128;
    }
    /** @name PalletNftsAttributeDeposit (528) */
    interface PalletNftsAttributeDeposit extends Struct {
        readonly account: Option<AccountId32>;
        readonly amount: u128;
    }
    /** @name PalletNftsPendingSwap (532) */
    interface PalletNftsPendingSwap extends Struct {
        readonly desiredCollection: u32;
        readonly desiredItem: Option<u32>;
        readonly price: Option<PalletNftsPriceWithDirection>;
        readonly deadline: u32;
    }
    /** @name PalletNftsPalletFeature (534) */
    interface PalletNftsPalletFeature extends Enum {
        readonly isTrading: boolean;
        readonly isAttributes: boolean;
        readonly isApprovals: boolean;
        readonly isSwaps: boolean;
        readonly type: 'Trading' | 'Attributes' | 'Approvals' | 'Swaps';
    }
    /** @name PalletNftsError (535) */
    interface PalletNftsError extends Enum {
        readonly isNoPermission: boolean;
        readonly isUnknownCollection: boolean;
        readonly isAlreadyExists: boolean;
        readonly isApprovalExpired: boolean;
        readonly isWrongOwner: boolean;
        readonly isBadWitness: boolean;
        readonly isCollectionIdInUse: boolean;
        readonly isItemsNonTransferable: boolean;
        readonly isNotDelegate: boolean;
        readonly isWrongDelegate: boolean;
        readonly isUnapproved: boolean;
        readonly isUnaccepted: boolean;
        readonly isItemLocked: boolean;
        readonly isLockedItemAttributes: boolean;
        readonly isLockedCollectionAttributes: boolean;
        readonly isLockedItemMetadata: boolean;
        readonly isLockedCollectionMetadata: boolean;
        readonly isMaxSupplyReached: boolean;
        readonly isMaxSupplyLocked: boolean;
        readonly isMaxSupplyTooSmall: boolean;
        readonly isUnknownItem: boolean;
        readonly isUnknownSwap: boolean;
        readonly isMetadataNotFound: boolean;
        readonly isAttributeNotFound: boolean;
        readonly isNotForSale: boolean;
        readonly isBidTooLow: boolean;
        readonly isReachedApprovalLimit: boolean;
        readonly isDeadlineExpired: boolean;
        readonly isWrongDuration: boolean;
        readonly isMethodDisabled: boolean;
        readonly isWrongSetting: boolean;
        readonly isInconsistentItemConfig: boolean;
        readonly isNoConfig: boolean;
        readonly isRolesNotCleared: boolean;
        readonly isMintNotStarted: boolean;
        readonly isMintEnded: boolean;
        readonly isAlreadyClaimed: boolean;
        readonly isIncorrectData: boolean;
        readonly isWrongOrigin: boolean;
        readonly isWrongSignature: boolean;
        readonly isIncorrectMetadata: boolean;
        readonly isMaxAttributesLimitReached: boolean;
        readonly isWrongNamespace: boolean;
        readonly isCollectionNotEmpty: boolean;
        readonly isWitnessRequired: boolean;
        readonly type: 'NoPermission' | 'UnknownCollection' | 'AlreadyExists' | 'ApprovalExpired' | 'WrongOwner' | 'BadWitness' | 'CollectionIdInUse' | 'ItemsNonTransferable' | 'NotDelegate' | 'WrongDelegate' | 'Unapproved' | 'Unaccepted' | 'ItemLocked' | 'LockedItemAttributes' | 'LockedCollectionAttributes' | 'LockedItemMetadata' | 'LockedCollectionMetadata' | 'MaxSupplyReached' | 'MaxSupplyLocked' | 'MaxSupplyTooSmall' | 'UnknownItem' | 'UnknownSwap' | 'MetadataNotFound' | 'AttributeNotFound' | 'NotForSale' | 'BidTooLow' | 'ReachedApprovalLimit' | 'DeadlineExpired' | 'WrongDuration' | 'MethodDisabled' | 'WrongSetting' | 'InconsistentItemConfig' | 'NoConfig' | 'RolesNotCleared' | 'MintNotStarted' | 'MintEnded' | 'AlreadyClaimed' | 'IncorrectData' | 'WrongOrigin' | 'WrongSignature' | 'IncorrectMetadata' | 'MaxAttributesLimitReached' | 'WrongNamespace' | 'CollectionNotEmpty' | 'WitnessRequired';
    }
    /** @name FrameSystemExtensionsCheckNonZeroSender (538) */
    type FrameSystemExtensionsCheckNonZeroSender = Null;
    /** @name FrameSystemExtensionsCheckSpecVersion (539) */
    type FrameSystemExtensionsCheckSpecVersion = Null;
    /** @name FrameSystemExtensionsCheckTxVersion (540) */
    type FrameSystemExtensionsCheckTxVersion = Null;
    /** @name FrameSystemExtensionsCheckGenesis (541) */
    type FrameSystemExtensionsCheckGenesis = Null;
    /** @name FrameSystemExtensionsCheckNonce (544) */
    interface FrameSystemExtensionsCheckNonce extends Compact<u32> {
    }
    /** @name FrameSystemExtensionsCheckWeight (545) */
    type FrameSystemExtensionsCheckWeight = Null;
    /** @name PalletTransactionPaymentChargeTransactionPayment (546) */
    interface PalletTransactionPaymentChargeTransactionPayment extends Compact<u128> {
    }
    /** @name CumulusPrimitivesStorageWeightReclaimStorageWeightReclaim (547) */
    type CumulusPrimitivesStorageWeightReclaimStorageWeightReclaim = Null;
    /** @name FrameMetadataHashExtensionCheckMetadataHash (548) */
    interface FrameMetadataHashExtensionCheckMetadataHash extends Struct {
        readonly mode: FrameMetadataHashExtensionMode;
    }
    /** @name FrameMetadataHashExtensionMode (549) */
    interface FrameMetadataHashExtensionMode extends Enum {
        readonly isDisabled: boolean;
        readonly isEnabled: boolean;
        readonly type: 'Disabled' | 'Enabled';
    }
    /** @name ShParachainRuntimeRuntime (550) */
    type ShParachainRuntimeRuntime = Null;
}
