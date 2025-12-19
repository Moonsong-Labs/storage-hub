// Auto-generated via `yarn polkadot-types-from-defs`, do not edit
/* eslint-disable */

// import type lookup before we augment - in some environments
// this is required to allow for ambient/previous definitions
import "@polkadot/types/lookup";

import type {
  BTreeMap,
  Bytes,
  Compact,
  Enum,
  Null,
  Option,
  Result,
  Struct,
  Text,
  U256,
  U8aFixed,
  Vec,
  bool,
  u128,
  u32,
  u64,
  u8
} from "@polkadot/types-codec";
import type { ITuple } from "@polkadot/types-codec/types";
import type { AccountId20, Call, H160, H256, Perbill } from "@polkadot/types/interfaces/runtime";
import type { Event } from "@polkadot/types/interfaces/system";

declare module "@polkadot/types/lookup" {
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

  /** @name SpRuntimeDigest (16) */
  interface SpRuntimeDigest extends Struct {
    readonly logs: Vec<SpRuntimeDigestDigestItem>;
  }

  /** @name SpRuntimeDigestDigestItem (18) */
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
    readonly type: "Other" | "Consensus" | "Seal" | "PreRuntime" | "RuntimeEnvironmentUpdated";
  }

  /** @name FrameSystemEventRecord (21) */
  interface FrameSystemEventRecord extends Struct {
    readonly phase: FrameSystemPhase;
    readonly event: Event;
    readonly topics: Vec<H256>;
  }

  /** @name FrameSystemEvent (23) */
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
      readonly account: AccountId20;
    } & Struct;
    readonly isKilledAccount: boolean;
    readonly asKilledAccount: {
      readonly account: AccountId20;
    } & Struct;
    readonly isRemarked: boolean;
    readonly asRemarked: {
      readonly sender: AccountId20;
      readonly hash_: H256;
    } & Struct;
    readonly isUpgradeAuthorized: boolean;
    readonly asUpgradeAuthorized: {
      readonly codeHash: H256;
      readonly checkVersion: bool;
    } & Struct;
    readonly type:
      | "ExtrinsicSuccess"
      | "ExtrinsicFailed"
      | "CodeUpdated"
      | "NewAccount"
      | "KilledAccount"
      | "Remarked"
      | "UpgradeAuthorized";
  }

  /** @name FrameSystemDispatchEventInfo (24) */
  interface FrameSystemDispatchEventInfo extends Struct {
    readonly weight: SpWeightsWeightV2Weight;
    readonly class: FrameSupportDispatchDispatchClass;
    readonly paysFee: FrameSupportDispatchPays;
  }

  /** @name FrameSupportDispatchDispatchClass (25) */
  interface FrameSupportDispatchDispatchClass extends Enum {
    readonly isNormal: boolean;
    readonly isOperational: boolean;
    readonly isMandatory: boolean;
    readonly type: "Normal" | "Operational" | "Mandatory";
  }

  /** @name FrameSupportDispatchPays (26) */
  interface FrameSupportDispatchPays extends Enum {
    readonly isYes: boolean;
    readonly isNo: boolean;
    readonly type: "Yes" | "No";
  }

  /** @name SpRuntimeDispatchError (27) */
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
    readonly type:
      | "Other"
      | "CannotLookup"
      | "BadOrigin"
      | "Module"
      | "ConsumerRemaining"
      | "NoProviders"
      | "TooManyConsumers"
      | "Token"
      | "Arithmetic"
      | "Transactional"
      | "Exhausted"
      | "Corruption"
      | "Unavailable"
      | "RootNotAllowed"
      | "Trie";
  }

  /** @name SpRuntimeModuleError (28) */
  interface SpRuntimeModuleError extends Struct {
    readonly index: u8;
    readonly error: U8aFixed;
  }

  /** @name SpRuntimeTokenError (29) */
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
    readonly type:
      | "FundsUnavailable"
      | "OnlyProvider"
      | "BelowMinimum"
      | "CannotCreate"
      | "UnknownAsset"
      | "Frozen"
      | "Unsupported"
      | "CannotCreateHold"
      | "NotExpendable"
      | "Blocked";
  }

  /** @name SpArithmeticArithmeticError (30) */
  interface SpArithmeticArithmeticError extends Enum {
    readonly isUnderflow: boolean;
    readonly isOverflow: boolean;
    readonly isDivisionByZero: boolean;
    readonly type: "Underflow" | "Overflow" | "DivisionByZero";
  }

  /** @name SpRuntimeTransactionalError (31) */
  interface SpRuntimeTransactionalError extends Enum {
    readonly isLimitReached: boolean;
    readonly isNoLayer: boolean;
    readonly type: "LimitReached" | "NoLayer";
  }

  /** @name SpRuntimeProvingTrieTrieError (32) */
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
    readonly type:
      | "InvalidStateRoot"
      | "IncompleteDatabase"
      | "ValueAtIncompleteKey"
      | "DecoderError"
      | "InvalidHash"
      | "DuplicateKey"
      | "ExtraneousNode"
      | "ExtraneousValue"
      | "ExtraneousHashReference"
      | "InvalidChildReference"
      | "ValueMismatch"
      | "IncompleteProof"
      | "RootMismatch"
      | "DecodeError";
  }

  /** @name PalletBalancesEvent (33) */
  interface PalletBalancesEvent extends Enum {
    readonly isEndowed: boolean;
    readonly asEndowed: {
      readonly account: AccountId20;
      readonly freeBalance: u128;
    } & Struct;
    readonly isDustLost: boolean;
    readonly asDustLost: {
      readonly account: AccountId20;
      readonly amount: u128;
    } & Struct;
    readonly isTransfer: boolean;
    readonly asTransfer: {
      readonly from: AccountId20;
      readonly to: AccountId20;
      readonly amount: u128;
    } & Struct;
    readonly isBalanceSet: boolean;
    readonly asBalanceSet: {
      readonly who: AccountId20;
      readonly free: u128;
    } & Struct;
    readonly isReserved: boolean;
    readonly asReserved: {
      readonly who: AccountId20;
      readonly amount: u128;
    } & Struct;
    readonly isUnreserved: boolean;
    readonly asUnreserved: {
      readonly who: AccountId20;
      readonly amount: u128;
    } & Struct;
    readonly isReserveRepatriated: boolean;
    readonly asReserveRepatriated: {
      readonly from: AccountId20;
      readonly to: AccountId20;
      readonly amount: u128;
      readonly destinationStatus: FrameSupportTokensMiscBalanceStatus;
    } & Struct;
    readonly isDeposit: boolean;
    readonly asDeposit: {
      readonly who: AccountId20;
      readonly amount: u128;
    } & Struct;
    readonly isWithdraw: boolean;
    readonly asWithdraw: {
      readonly who: AccountId20;
      readonly amount: u128;
    } & Struct;
    readonly isSlashed: boolean;
    readonly asSlashed: {
      readonly who: AccountId20;
      readonly amount: u128;
    } & Struct;
    readonly isMinted: boolean;
    readonly asMinted: {
      readonly who: AccountId20;
      readonly amount: u128;
    } & Struct;
    readonly isBurned: boolean;
    readonly asBurned: {
      readonly who: AccountId20;
      readonly amount: u128;
    } & Struct;
    readonly isSuspended: boolean;
    readonly asSuspended: {
      readonly who: AccountId20;
      readonly amount: u128;
    } & Struct;
    readonly isRestored: boolean;
    readonly asRestored: {
      readonly who: AccountId20;
      readonly amount: u128;
    } & Struct;
    readonly isUpgraded: boolean;
    readonly asUpgraded: {
      readonly who: AccountId20;
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
      readonly who: AccountId20;
      readonly amount: u128;
    } & Struct;
    readonly isUnlocked: boolean;
    readonly asUnlocked: {
      readonly who: AccountId20;
      readonly amount: u128;
    } & Struct;
    readonly isFrozen: boolean;
    readonly asFrozen: {
      readonly who: AccountId20;
      readonly amount: u128;
    } & Struct;
    readonly isThawed: boolean;
    readonly asThawed: {
      readonly who: AccountId20;
      readonly amount: u128;
    } & Struct;
    readonly isTotalIssuanceForced: boolean;
    readonly asTotalIssuanceForced: {
      readonly old: u128;
      readonly new_: u128;
    } & Struct;
    readonly type:
      | "Endowed"
      | "DustLost"
      | "Transfer"
      | "BalanceSet"
      | "Reserved"
      | "Unreserved"
      | "ReserveRepatriated"
      | "Deposit"
      | "Withdraw"
      | "Slashed"
      | "Minted"
      | "Burned"
      | "Suspended"
      | "Restored"
      | "Upgraded"
      | "Issued"
      | "Rescinded"
      | "Locked"
      | "Unlocked"
      | "Frozen"
      | "Thawed"
      | "TotalIssuanceForced";
  }

  /** @name FrameSupportTokensMiscBalanceStatus (34) */
  interface FrameSupportTokensMiscBalanceStatus extends Enum {
    readonly isFree: boolean;
    readonly isReserved: boolean;
    readonly type: "Free" | "Reserved";
  }

  /** @name PalletOffencesEvent (35) */
  interface PalletOffencesEvent extends Enum {
    readonly isOffence: boolean;
    readonly asOffence: {
      readonly kind: U8aFixed;
      readonly timeslot: Bytes;
    } & Struct;
    readonly type: "Offence";
  }

  /** @name PalletSessionEvent (37) */
  interface PalletSessionEvent extends Enum {
    readonly isNewSession: boolean;
    readonly asNewSession: {
      readonly sessionIndex: u32;
    } & Struct;
    readonly type: "NewSession";
  }

  /** @name PalletGrandpaEvent (38) */
  interface PalletGrandpaEvent extends Enum {
    readonly isNewAuthorities: boolean;
    readonly asNewAuthorities: {
      readonly authoritySet: Vec<ITuple<[SpConsensusGrandpaAppPublic, u64]>>;
    } & Struct;
    readonly isPaused: boolean;
    readonly isResumed: boolean;
    readonly type: "NewAuthorities" | "Paused" | "Resumed";
  }

  /** @name SpConsensusGrandpaAppPublic (41) */
  interface SpConsensusGrandpaAppPublic extends U8aFixed {}

  /** @name PalletTransactionPaymentEvent (42) */
  interface PalletTransactionPaymentEvent extends Enum {
    readonly isTransactionFeePaid: boolean;
    readonly asTransactionFeePaid: {
      readonly who: AccountId20;
      readonly actualFee: u128;
      readonly tip: u128;
    } & Struct;
    readonly type: "TransactionFeePaid";
  }

  /** @name PalletParametersEvent (43) */
  interface PalletParametersEvent extends Enum {
    readonly isUpdated: boolean;
    readonly asUpdated: {
      readonly key: ShSolochainEvmRuntimeConfigsRuntimeParamsRuntimeParametersKey;
      readonly oldValue: Option<ShSolochainEvmRuntimeConfigsRuntimeParamsRuntimeParametersValue>;
      readonly newValue: Option<ShSolochainEvmRuntimeConfigsRuntimeParamsRuntimeParametersValue>;
    } & Struct;
    readonly type: "Updated";
  }

  /** @name ShSolochainEvmRuntimeConfigsRuntimeParamsRuntimeParametersKey (44) */
  interface ShSolochainEvmRuntimeConfigsRuntimeParamsRuntimeParametersKey extends Enum {
    readonly isRuntimeConfig: boolean;
    readonly asRuntimeConfig: ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigParametersKey;
    readonly type: "RuntimeConfig";
  }

  /** @name ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigParametersKey (45) */
  interface ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigParametersKey
    extends Enum {
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
    readonly type:
      | "SlashAmountPerMaxFileSize"
      | "StakeToChallengePeriod"
      | "CheckpointChallengePeriod"
      | "MinChallengePeriod"
      | "SystemUtilisationLowerThresholdPercentage"
      | "SystemUtilisationUpperThresholdPercentage"
      | "MostlyStablePrice"
      | "MaxPrice"
      | "MinPrice"
      | "UpperExponentFactor"
      | "LowerExponentFactor"
      | "ZeroSizeBucketFixedRate"
      | "IdealUtilisationRate"
      | "DecayRate"
      | "MinimumTreasuryCut"
      | "MaximumTreasuryCut"
      | "BspStopStoringFilePenalty"
      | "ProviderTopUpTtl"
      | "BasicReplicationTarget"
      | "StandardReplicationTarget"
      | "HighSecurityReplicationTarget"
      | "SuperHighSecurityReplicationTarget"
      | "UltraHighSecurityReplicationTarget"
      | "MaxReplicationTarget"
      | "TickRangeToMaximumThreshold"
      | "StorageRequestTtl"
      | "MinWaitForStopStoring"
      | "MinSeedPeriod"
      | "StakeToSeedPeriod"
      | "UpfrontTicksToPay";
  }

  /** @name ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigSlashAmountPerMaxFileSize (46) */
  type ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigSlashAmountPerMaxFileSize =
    Null;

  /** @name ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigStakeToChallengePeriod (47) */
  type ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigStakeToChallengePeriod =
    Null;

  /** @name ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigCheckpointChallengePeriod (48) */
  type ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigCheckpointChallengePeriod =
    Null;

  /** @name ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMinChallengePeriod (49) */
  type ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMinChallengePeriod = Null;

  /** @name ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigSystemUtilisationLowerThresholdPercentage (50) */
  type ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigSystemUtilisationLowerThresholdPercentage =
    Null;

  /** @name ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigSystemUtilisationUpperThresholdPercentage (51) */
  type ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigSystemUtilisationUpperThresholdPercentage =
    Null;

  /** @name ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMostlyStablePrice (52) */
  type ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMostlyStablePrice = Null;

  /** @name ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMaxPrice (53) */
  type ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMaxPrice = Null;

  /** @name ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMinPrice (54) */
  type ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMinPrice = Null;

  /** @name ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigUpperExponentFactor (55) */
  type ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigUpperExponentFactor =
    Null;

  /** @name ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigLowerExponentFactor (56) */
  type ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigLowerExponentFactor =
    Null;

  /** @name ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigZeroSizeBucketFixedRate (57) */
  type ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigZeroSizeBucketFixedRate =
    Null;

  /** @name ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigIdealUtilisationRate (58) */
  type ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigIdealUtilisationRate =
    Null;

  /** @name ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigDecayRate (59) */
  type ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigDecayRate = Null;

  /** @name ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMinimumTreasuryCut (60) */
  type ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMinimumTreasuryCut = Null;

  /** @name ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMaximumTreasuryCut (61) */
  type ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMaximumTreasuryCut = Null;

  /** @name ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigBspStopStoringFilePenalty (62) */
  type ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigBspStopStoringFilePenalty =
    Null;

  /** @name ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigProviderTopUpTtl (63) */
  type ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigProviderTopUpTtl = Null;

  /** @name ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigBasicReplicationTarget (64) */
  type ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigBasicReplicationTarget =
    Null;

  /** @name ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigStandardReplicationTarget (65) */
  type ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigStandardReplicationTarget =
    Null;

  /** @name ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigHighSecurityReplicationTarget (66) */
  type ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigHighSecurityReplicationTarget =
    Null;

  /** @name ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigSuperHighSecurityReplicationTarget (67) */
  type ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigSuperHighSecurityReplicationTarget =
    Null;

  /** @name ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigUltraHighSecurityReplicationTarget (68) */
  type ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigUltraHighSecurityReplicationTarget =
    Null;

  /** @name ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMaxReplicationTarget (69) */
  type ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMaxReplicationTarget =
    Null;

  /** @name ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigTickRangeToMaximumThreshold (70) */
  type ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigTickRangeToMaximumThreshold =
    Null;

  /** @name ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigStorageRequestTtl (71) */
  type ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigStorageRequestTtl = Null;

  /** @name ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMinWaitForStopStoring (72) */
  type ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMinWaitForStopStoring =
    Null;

  /** @name ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMinSeedPeriod (73) */
  type ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMinSeedPeriod = Null;

  /** @name ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigStakeToSeedPeriod (74) */
  type ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigStakeToSeedPeriod = Null;

  /** @name ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigUpfrontTicksToPay (75) */
  type ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigUpfrontTicksToPay = Null;

  /** @name ShSolochainEvmRuntimeConfigsRuntimeParamsRuntimeParametersValue (77) */
  interface ShSolochainEvmRuntimeConfigsRuntimeParamsRuntimeParametersValue extends Enum {
    readonly isRuntimeConfig: boolean;
    readonly asRuntimeConfig: ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigParametersValue;
    readonly type: "RuntimeConfig";
  }

  /** @name ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigParametersValue (78) */
  interface ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigParametersValue
    extends Enum {
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
    readonly type:
      | "SlashAmountPerMaxFileSize"
      | "StakeToChallengePeriod"
      | "CheckpointChallengePeriod"
      | "MinChallengePeriod"
      | "SystemUtilisationLowerThresholdPercentage"
      | "SystemUtilisationUpperThresholdPercentage"
      | "MostlyStablePrice"
      | "MaxPrice"
      | "MinPrice"
      | "UpperExponentFactor"
      | "LowerExponentFactor"
      | "ZeroSizeBucketFixedRate"
      | "IdealUtilisationRate"
      | "DecayRate"
      | "MinimumTreasuryCut"
      | "MaximumTreasuryCut"
      | "BspStopStoringFilePenalty"
      | "ProviderTopUpTtl"
      | "BasicReplicationTarget"
      | "StandardReplicationTarget"
      | "HighSecurityReplicationTarget"
      | "SuperHighSecurityReplicationTarget"
      | "UltraHighSecurityReplicationTarget"
      | "MaxReplicationTarget"
      | "TickRangeToMaximumThreshold"
      | "StorageRequestTtl"
      | "MinWaitForStopStoring"
      | "MinSeedPeriod"
      | "StakeToSeedPeriod"
      | "UpfrontTicksToPay";
  }

  /** @name PalletSudoEvent (80) */
  interface PalletSudoEvent extends Enum {
    readonly isSudid: boolean;
    readonly asSudid: {
      readonly sudoResult: Result<Null, SpRuntimeDispatchError>;
    } & Struct;
    readonly isKeyChanged: boolean;
    readonly asKeyChanged: {
      readonly old: Option<AccountId20>;
      readonly new_: AccountId20;
    } & Struct;
    readonly isKeyRemoved: boolean;
    readonly isSudoAsDone: boolean;
    readonly asSudoAsDone: {
      readonly sudoResult: Result<Null, SpRuntimeDispatchError>;
    } & Struct;
    readonly type: "Sudid" | "KeyChanged" | "KeyRemoved" | "SudoAsDone";
  }

  /** @name PalletEthereumEvent (84) */
  interface PalletEthereumEvent extends Enum {
    readonly isExecuted: boolean;
    readonly asExecuted: {
      readonly from: H160;
      readonly to: H160;
      readonly transactionHash: H256;
      readonly exitReason: EvmCoreErrorExitReason;
      readonly extraData: Bytes;
    } & Struct;
    readonly type: "Executed";
  }

  /** @name EvmCoreErrorExitReason (86) */
  interface EvmCoreErrorExitReason extends Enum {
    readonly isSucceed: boolean;
    readonly asSucceed: EvmCoreErrorExitSucceed;
    readonly isError: boolean;
    readonly asError: EvmCoreErrorExitError;
    readonly isRevert: boolean;
    readonly asRevert: EvmCoreErrorExitRevert;
    readonly isFatal: boolean;
    readonly asFatal: EvmCoreErrorExitFatal;
    readonly type: "Succeed" | "Error" | "Revert" | "Fatal";
  }

  /** @name EvmCoreErrorExitSucceed (87) */
  interface EvmCoreErrorExitSucceed extends Enum {
    readonly isStopped: boolean;
    readonly isReturned: boolean;
    readonly isSuicided: boolean;
    readonly type: "Stopped" | "Returned" | "Suicided";
  }

  /** @name EvmCoreErrorExitError (88) */
  interface EvmCoreErrorExitError extends Enum {
    readonly isStackUnderflow: boolean;
    readonly isStackOverflow: boolean;
    readonly isInvalidJump: boolean;
    readonly isInvalidRange: boolean;
    readonly isDesignatedInvalid: boolean;
    readonly isCallTooDeep: boolean;
    readonly isCreateCollision: boolean;
    readonly isCreateContractLimit: boolean;
    readonly isOutOfOffset: boolean;
    readonly isOutOfGas: boolean;
    readonly isOutOfFund: boolean;
    readonly isPcUnderflow: boolean;
    readonly isCreateEmpty: boolean;
    readonly isOther: boolean;
    readonly asOther: Text;
    readonly isMaxNonce: boolean;
    readonly isInvalidCode: boolean;
    readonly asInvalidCode: u8;
    readonly type:
      | "StackUnderflow"
      | "StackOverflow"
      | "InvalidJump"
      | "InvalidRange"
      | "DesignatedInvalid"
      | "CallTooDeep"
      | "CreateCollision"
      | "CreateContractLimit"
      | "OutOfOffset"
      | "OutOfGas"
      | "OutOfFund"
      | "PcUnderflow"
      | "CreateEmpty"
      | "Other"
      | "MaxNonce"
      | "InvalidCode";
  }

  /** @name EvmCoreErrorExitRevert (92) */
  interface EvmCoreErrorExitRevert extends Enum {
    readonly isReverted: boolean;
    readonly type: "Reverted";
  }

  /** @name EvmCoreErrorExitFatal (93) */
  interface EvmCoreErrorExitFatal extends Enum {
    readonly isNotSupported: boolean;
    readonly isUnhandledInterrupt: boolean;
    readonly isCallErrorAsFatal: boolean;
    readonly asCallErrorAsFatal: EvmCoreErrorExitError;
    readonly isOther: boolean;
    readonly asOther: Text;
    readonly type: "NotSupported" | "UnhandledInterrupt" | "CallErrorAsFatal" | "Other";
  }

  /** @name PalletEvmEvent (94) */
  interface PalletEvmEvent extends Enum {
    readonly isLog: boolean;
    readonly asLog: {
      readonly log: EthereumLog;
    } & Struct;
    readonly isCreated: boolean;
    readonly asCreated: {
      readonly address: H160;
    } & Struct;
    readonly isCreatedFailed: boolean;
    readonly asCreatedFailed: {
      readonly address: H160;
    } & Struct;
    readonly isExecuted: boolean;
    readonly asExecuted: {
      readonly address: H160;
    } & Struct;
    readonly isExecutedFailed: boolean;
    readonly asExecutedFailed: {
      readonly address: H160;
    } & Struct;
    readonly type: "Log" | "Created" | "CreatedFailed" | "Executed" | "ExecutedFailed";
  }

  /** @name EthereumLog (95) */
  interface EthereumLog extends Struct {
    readonly address: H160;
    readonly topics: Vec<H256>;
    readonly data: Bytes;
  }

  /** @name PalletStorageProvidersEvent (97) */
  interface PalletStorageProvidersEvent extends Enum {
    readonly isMspRequestSignUpSuccess: boolean;
    readonly asMspRequestSignUpSuccess: {
      readonly who: AccountId20;
      readonly multiaddresses: Vec<Bytes>;
      readonly capacity: u64;
    } & Struct;
    readonly isMspSignUpSuccess: boolean;
    readonly asMspSignUpSuccess: {
      readonly who: AccountId20;
      readonly mspId: H256;
      readonly multiaddresses: Vec<Bytes>;
      readonly capacity: u64;
      readonly valueProp: PalletStorageProvidersValuePropositionWithId;
    } & Struct;
    readonly isBspRequestSignUpSuccess: boolean;
    readonly asBspRequestSignUpSuccess: {
      readonly who: AccountId20;
      readonly multiaddresses: Vec<Bytes>;
      readonly capacity: u64;
    } & Struct;
    readonly isBspSignUpSuccess: boolean;
    readonly asBspSignUpSuccess: {
      readonly who: AccountId20;
      readonly bspId: H256;
      readonly root: H256;
      readonly multiaddresses: Vec<Bytes>;
      readonly capacity: u64;
    } & Struct;
    readonly isSignUpRequestCanceled: boolean;
    readonly asSignUpRequestCanceled: {
      readonly who: AccountId20;
    } & Struct;
    readonly isMspSignOffSuccess: boolean;
    readonly asMspSignOffSuccess: {
      readonly who: AccountId20;
      readonly mspId: H256;
    } & Struct;
    readonly isBspSignOffSuccess: boolean;
    readonly asBspSignOffSuccess: {
      readonly who: AccountId20;
      readonly bspId: H256;
    } & Struct;
    readonly isCapacityChanged: boolean;
    readonly asCapacityChanged: {
      readonly who: AccountId20;
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
    readonly type:
      | "MspRequestSignUpSuccess"
      | "MspSignUpSuccess"
      | "BspRequestSignUpSuccess"
      | "BspSignUpSuccess"
      | "SignUpRequestCanceled"
      | "MspSignOffSuccess"
      | "BspSignOffSuccess"
      | "CapacityChanged"
      | "Slashed"
      | "AwaitingTopUp"
      | "TopUpFulfilled"
      | "FailedToGetOwnerAccountOfInsolventProvider"
      | "FailedToSlashInsolventProvider"
      | "FailedToStopAllCyclesForInsolventBsp"
      | "FailedToInsertProviderTopUpExpiration"
      | "ProviderInsolvent"
      | "BucketsOfInsolventMsp"
      | "BucketRootChanged"
      | "MultiAddressAdded"
      | "MultiAddressRemoved"
      | "ValuePropAdded"
      | "ValuePropUnavailable"
      | "MspDeleted"
      | "BspDeleted";
  }

  /** @name PalletStorageProvidersValuePropositionWithId (101) */
  interface PalletStorageProvidersValuePropositionWithId extends Struct {
    readonly id: H256;
    readonly valueProp: PalletStorageProvidersValueProposition;
  }

  /** @name PalletStorageProvidersValueProposition (102) */
  interface PalletStorageProvidersValueProposition extends Struct {
    readonly pricePerGigaUnitOfDataPerBlock: u128;
    readonly commitment: Bytes;
    readonly bucketDataLimit: u64;
    readonly available: bool;
  }

  /** @name PalletStorageProvidersStorageProviderId (104) */
  interface PalletStorageProvidersStorageProviderId extends Enum {
    readonly isBackupStorageProvider: boolean;
    readonly asBackupStorageProvider: H256;
    readonly isMainStorageProvider: boolean;
    readonly asMainStorageProvider: H256;
    readonly type: "BackupStorageProvider" | "MainStorageProvider";
  }

  /** @name PalletStorageProvidersTopUpMetadata (105) */
  interface PalletStorageProvidersTopUpMetadata extends Struct {
    readonly startedAt: u32;
    readonly endTickGracePeriod: u32;
  }

  /** @name PalletFileSystemEvent (106) */
  interface PalletFileSystemEvent extends Enum {
    readonly isNewBucket: boolean;
    readonly asNewBucket: {
      readonly who: AccountId20;
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
      readonly who: AccountId20;
      readonly bucketId: H256;
      readonly maybeCollectionId: Option<u32>;
    } & Struct;
    readonly isBucketPrivacyUpdated: boolean;
    readonly asBucketPrivacyUpdated: {
      readonly who: AccountId20;
      readonly bucketId: H256;
      readonly collectionId: Option<u32>;
      readonly private: bool;
    } & Struct;
    readonly isNewCollectionAndAssociation: boolean;
    readonly asNewCollectionAndAssociation: {
      readonly who: AccountId20;
      readonly bucketId: H256;
      readonly collectionId: u32;
    } & Struct;
    readonly isMoveBucketRequested: boolean;
    readonly asMoveBucketRequested: {
      readonly who: AccountId20;
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
      readonly who: AccountId20;
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
      readonly owner: AccountId20;
      readonly size_: u64;
    } & Struct;
    readonly isBspConfirmedStoring: boolean;
    readonly asBspConfirmedStoring: {
      readonly who: AccountId20;
      readonly bspId: H256;
      readonly confirmedFileKeys: Vec<ITuple<[H256, ShpFileMetadataFileMetadata]>>;
      readonly skippedFileKeys: Vec<H256>;
      readonly newRoot: H256;
    } & Struct;
    readonly isBspChallengeCycleInitialised: boolean;
    readonly asBspChallengeCycleInitialised: {
      readonly who: AccountId20;
      readonly bspId: H256;
    } & Struct;
    readonly isBspRequestedToStopStoring: boolean;
    readonly asBspRequestedToStopStoring: {
      readonly bspId: H256;
      readonly fileKey: H256;
      readonly owner: AccountId20;
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
      readonly owner: AccountId20;
      readonly bucketId: H256;
    } & Struct;
    readonly isSpStopStoringInsolventUser: boolean;
    readonly asSpStopStoringInsolventUser: {
      readonly spId: H256;
      readonly fileKey: H256;
      readonly owner: AccountId20;
      readonly location: Bytes;
      readonly newRoot: H256;
    } & Struct;
    readonly isMspStopStoringBucketInsolventUser: boolean;
    readonly asMspStopStoringBucketInsolventUser: {
      readonly mspId: H256;
      readonly owner: AccountId20;
      readonly bucketId: H256;
    } & Struct;
    readonly isFileDeletionRequested: boolean;
    readonly asFileDeletionRequested: {
      readonly signedDeleteIntention: PalletFileSystemFileOperationIntention;
      readonly signature: FpAccountEthereumSignature;
    } & Struct;
    readonly isBucketFileDeletionsCompleted: boolean;
    readonly asBucketFileDeletionsCompleted: {
      readonly user: AccountId20;
      readonly fileKeys: Vec<H256>;
      readonly bucketId: H256;
      readonly mspId: Option<H256>;
      readonly oldRoot: H256;
      readonly newRoot: H256;
    } & Struct;
    readonly isBspFileDeletionsCompleted: boolean;
    readonly asBspFileDeletionsCompleted: {
      readonly users: Vec<AccountId20>;
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
      readonly owner: AccountId20;
      readonly amountToReturn: u128;
      readonly error: SpRuntimeDispatchError;
    } & Struct;
    readonly type:
      | "NewBucket"
      | "BucketDeleted"
      | "BucketPrivacyUpdated"
      | "NewCollectionAndAssociation"
      | "MoveBucketRequested"
      | "MoveBucketRequestExpired"
      | "MoveBucketAccepted"
      | "MoveBucketRejected"
      | "NewStorageRequest"
      | "MspAcceptedStorageRequest"
      | "StorageRequestFulfilled"
      | "StorageRequestExpired"
      | "StorageRequestRevoked"
      | "StorageRequestRejected"
      | "IncompleteStorageRequest"
      | "IncompleteStorageRequestCleanedUp"
      | "AcceptedBspVolunteer"
      | "BspConfirmedStoring"
      | "BspChallengeCycleInitialised"
      | "BspRequestedToStopStoring"
      | "BspConfirmStoppedStoring"
      | "MspStoppedStoringBucket"
      | "SpStopStoringInsolventUser"
      | "MspStopStoringBucketInsolventUser"
      | "FileDeletionRequested"
      | "BucketFileDeletionsCompleted"
      | "BspFileDeletionsCompleted"
      | "UsedCapacityShouldBeZero"
      | "FailedToReleaseStorageRequestCreationDeposit";
  }

  /** @name ShpFileMetadataFileMetadata (110) */
  interface ShpFileMetadataFileMetadata extends Struct {
    readonly owner: Bytes;
    readonly bucketId: Bytes;
    readonly location: Bytes;
    readonly fileSize: Compact<u64>;
    readonly fingerprint: ShpFileMetadataFingerprint;
  }

  /** @name ShpFileMetadataFingerprint (111) */
  interface ShpFileMetadataFingerprint extends U8aFixed {}

  /** @name PalletFileSystemRejectedStorageRequestReason (112) */
  interface PalletFileSystemRejectedStorageRequestReason extends Enum {
    readonly isReachedMaximumCapacity: boolean;
    readonly isReceivedInvalidProof: boolean;
    readonly isFileKeyAlreadyStored: boolean;
    readonly isRequestExpired: boolean;
    readonly isInternalError: boolean;
    readonly type:
      | "ReachedMaximumCapacity"
      | "ReceivedInvalidProof"
      | "FileKeyAlreadyStored"
      | "RequestExpired"
      | "InternalError";
  }

  /** @name PalletFileSystemFileOperationIntention (117) */
  interface PalletFileSystemFileOperationIntention extends Struct {
    readonly fileKey: H256;
    readonly operation: PalletFileSystemFileOperation;
  }

  /** @name PalletFileSystemFileOperation (118) */
  interface PalletFileSystemFileOperation extends Enum {
    readonly isDelete: boolean;
    readonly type: "Delete";
  }

  /** @name FpAccountEthereumSignature (119) */
  interface FpAccountEthereumSignature extends U8aFixed {}

  /** @name PalletProofsDealerEvent (124) */
  interface PalletProofsDealerEvent extends Enum {
    readonly isNewChallenge: boolean;
    readonly asNewChallenge: {
      readonly who: Option<AccountId20>;
      readonly keyChallenged: H256;
    } & Struct;
    readonly isNewPriorityChallenge: boolean;
    readonly asNewPriorityChallenge: {
      readonly who: Option<AccountId20>;
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
      readonly maybeProviderAccount: Option<AccountId20>;
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
    readonly type:
      | "NewChallenge"
      | "NewPriorityChallenge"
      | "ProofAccepted"
      | "NewChallengeSeed"
      | "NewCheckpointChallenge"
      | "SlashableProvider"
      | "NoRecordOfLastSubmittedProof"
      | "NewChallengeCycleInitialised"
      | "MutationsAppliedForProvider"
      | "MutationsApplied"
      | "ChallengesTickerSet";
  }

  /** @name PalletProofsDealerProof (125) */
  interface PalletProofsDealerProof extends Struct {
    readonly forestProof: SpTrieStorageProofCompactProof;
    readonly keyProofs: BTreeMap<H256, PalletProofsDealerKeyProof>;
  }

  /** @name SpTrieStorageProofCompactProof (126) */
  interface SpTrieStorageProofCompactProof extends Struct {
    readonly encodedNodes: Vec<Bytes>;
  }

  /** @name PalletProofsDealerKeyProof (129) */
  interface PalletProofsDealerKeyProof extends Struct {
    readonly proof: ShpFileKeyVerifierFileKeyProof;
    readonly challengeCount: u32;
  }

  /** @name ShpFileKeyVerifierFileKeyProof (130) */
  interface ShpFileKeyVerifierFileKeyProof extends Struct {
    readonly fileMetadata: ShpFileMetadataFileMetadata;
    readonly proof: SpTrieStorageProofCompactProof;
  }

  /** @name PalletProofsDealerCustomChallenge (134) */
  interface PalletProofsDealerCustomChallenge extends Struct {
    readonly key: H256;
    readonly shouldRemoveKey: bool;
  }

  /** @name ShpTraitsTrieMutation (138) */
  interface ShpTraitsTrieMutation extends Enum {
    readonly isAdd: boolean;
    readonly asAdd: ShpTraitsTrieAddMutation;
    readonly isRemove: boolean;
    readonly asRemove: ShpTraitsTrieRemoveMutation;
    readonly type: "Add" | "Remove";
  }

  /** @name ShpTraitsTrieAddMutation (139) */
  interface ShpTraitsTrieAddMutation extends Struct {
    readonly value: Bytes;
  }

  /** @name ShpTraitsTrieRemoveMutation (140) */
  interface ShpTraitsTrieRemoveMutation extends Struct {
    readonly maybeValue: Option<Bytes>;
  }

  /** @name PalletRandomnessEvent (142) */
  interface PalletRandomnessEvent extends Enum {
    readonly isNewOneEpochAgoRandomnessAvailable: boolean;
    readonly asNewOneEpochAgoRandomnessAvailable: {
      readonly randomnessSeed: H256;
      readonly fromEpoch: u64;
      readonly validUntilBlock: u32;
    } & Struct;
    readonly type: "NewOneEpochAgoRandomnessAvailable";
  }

  /** @name PalletPaymentStreamsEvent (143) */
  interface PalletPaymentStreamsEvent extends Enum {
    readonly isFixedRatePaymentStreamCreated: boolean;
    readonly asFixedRatePaymentStreamCreated: {
      readonly userAccount: AccountId20;
      readonly providerId: H256;
      readonly rate: u128;
    } & Struct;
    readonly isFixedRatePaymentStreamUpdated: boolean;
    readonly asFixedRatePaymentStreamUpdated: {
      readonly userAccount: AccountId20;
      readonly providerId: H256;
      readonly newRate: u128;
    } & Struct;
    readonly isFixedRatePaymentStreamDeleted: boolean;
    readonly asFixedRatePaymentStreamDeleted: {
      readonly userAccount: AccountId20;
      readonly providerId: H256;
    } & Struct;
    readonly isDynamicRatePaymentStreamCreated: boolean;
    readonly asDynamicRatePaymentStreamCreated: {
      readonly userAccount: AccountId20;
      readonly providerId: H256;
      readonly amountProvided: u64;
    } & Struct;
    readonly isDynamicRatePaymentStreamUpdated: boolean;
    readonly asDynamicRatePaymentStreamUpdated: {
      readonly userAccount: AccountId20;
      readonly providerId: H256;
      readonly newAmountProvided: u64;
    } & Struct;
    readonly isDynamicRatePaymentStreamDeleted: boolean;
    readonly asDynamicRatePaymentStreamDeleted: {
      readonly userAccount: AccountId20;
      readonly providerId: H256;
    } & Struct;
    readonly isPaymentStreamCharged: boolean;
    readonly asPaymentStreamCharged: {
      readonly userAccount: AccountId20;
      readonly providerId: H256;
      readonly amount: u128;
      readonly lastTickCharged: u32;
      readonly chargedAtTick: u32;
    } & Struct;
    readonly isUsersCharged: boolean;
    readonly asUsersCharged: {
      readonly userAccounts: Vec<AccountId20>;
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
      readonly who: AccountId20;
    } & Struct;
    readonly isUserPaidAllDebts: boolean;
    readonly asUserPaidAllDebts: {
      readonly who: AccountId20;
    } & Struct;
    readonly isUserPaidSomeDebts: boolean;
    readonly asUserPaidSomeDebts: {
      readonly who: AccountId20;
    } & Struct;
    readonly isUserSolvent: boolean;
    readonly asUserSolvent: {
      readonly who: AccountId20;
    } & Struct;
    readonly isInconsistentTickProcessing: boolean;
    readonly asInconsistentTickProcessing: {
      readonly lastProcessedTick: u32;
      readonly tickToProcess: u32;
    } & Struct;
    readonly type:
      | "FixedRatePaymentStreamCreated"
      | "FixedRatePaymentStreamUpdated"
      | "FixedRatePaymentStreamDeleted"
      | "DynamicRatePaymentStreamCreated"
      | "DynamicRatePaymentStreamUpdated"
      | "DynamicRatePaymentStreamDeleted"
      | "PaymentStreamCharged"
      | "UsersCharged"
      | "LastChargeableInfoUpdated"
      | "UserWithoutFunds"
      | "UserPaidAllDebts"
      | "UserPaidSomeDebts"
      | "UserSolvent"
      | "InconsistentTickProcessing";
  }

  /** @name PalletBucketNftsEvent (145) */
  interface PalletBucketNftsEvent extends Enum {
    readonly isAccessShared: boolean;
    readonly asAccessShared: {
      readonly issuer: AccountId20;
      readonly recipient: AccountId20;
    } & Struct;
    readonly isItemReadAccessUpdated: boolean;
    readonly asItemReadAccessUpdated: {
      readonly admin: AccountId20;
      readonly bucket: H256;
      readonly itemId: u32;
    } & Struct;
    readonly isItemBurned: boolean;
    readonly asItemBurned: {
      readonly account: AccountId20;
      readonly bucket: H256;
      readonly itemId: u32;
    } & Struct;
    readonly type: "AccessShared" | "ItemReadAccessUpdated" | "ItemBurned";
  }

  /** @name PalletNftsEvent (146) */
  interface PalletNftsEvent extends Enum {
    readonly isCreated: boolean;
    readonly asCreated: {
      readonly collection: u32;
      readonly creator: AccountId20;
      readonly owner: AccountId20;
    } & Struct;
    readonly isForceCreated: boolean;
    readonly asForceCreated: {
      readonly collection: u32;
      readonly owner: AccountId20;
    } & Struct;
    readonly isDestroyed: boolean;
    readonly asDestroyed: {
      readonly collection: u32;
    } & Struct;
    readonly isIssued: boolean;
    readonly asIssued: {
      readonly collection: u32;
      readonly item: u32;
      readonly owner: AccountId20;
    } & Struct;
    readonly isTransferred: boolean;
    readonly asTransferred: {
      readonly collection: u32;
      readonly item: u32;
      readonly from: AccountId20;
      readonly to: AccountId20;
    } & Struct;
    readonly isBurned: boolean;
    readonly asBurned: {
      readonly collection: u32;
      readonly item: u32;
      readonly owner: AccountId20;
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
      readonly newOwner: AccountId20;
    } & Struct;
    readonly isTeamChanged: boolean;
    readonly asTeamChanged: {
      readonly collection: u32;
      readonly issuer: Option<AccountId20>;
      readonly admin: Option<AccountId20>;
      readonly freezer: Option<AccountId20>;
    } & Struct;
    readonly isTransferApproved: boolean;
    readonly asTransferApproved: {
      readonly collection: u32;
      readonly item: u32;
      readonly owner: AccountId20;
      readonly delegate: AccountId20;
      readonly deadline: Option<u32>;
    } & Struct;
    readonly isApprovalCancelled: boolean;
    readonly asApprovalCancelled: {
      readonly collection: u32;
      readonly item: u32;
      readonly owner: AccountId20;
      readonly delegate: AccountId20;
    } & Struct;
    readonly isAllApprovalsCancelled: boolean;
    readonly asAllApprovalsCancelled: {
      readonly collection: u32;
      readonly item: u32;
      readonly owner: AccountId20;
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
      readonly delegate: AccountId20;
    } & Struct;
    readonly isItemAttributesApprovalRemoved: boolean;
    readonly asItemAttributesApprovalRemoved: {
      readonly collection: u32;
      readonly item: u32;
      readonly delegate: AccountId20;
    } & Struct;
    readonly isOwnershipAcceptanceChanged: boolean;
    readonly asOwnershipAcceptanceChanged: {
      readonly who: AccountId20;
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
      readonly whitelistedBuyer: Option<AccountId20>;
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
      readonly seller: AccountId20;
      readonly buyer: AccountId20;
    } & Struct;
    readonly isTipSent: boolean;
    readonly asTipSent: {
      readonly collection: u32;
      readonly item: u32;
      readonly sender: AccountId20;
      readonly receiver: AccountId20;
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
      readonly sentItemOwner: AccountId20;
      readonly receivedCollection: u32;
      readonly receivedItem: u32;
      readonly receivedItemOwner: AccountId20;
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
    readonly type:
      | "Created"
      | "ForceCreated"
      | "Destroyed"
      | "Issued"
      | "Transferred"
      | "Burned"
      | "ItemTransferLocked"
      | "ItemTransferUnlocked"
      | "ItemPropertiesLocked"
      | "CollectionLocked"
      | "OwnerChanged"
      | "TeamChanged"
      | "TransferApproved"
      | "ApprovalCancelled"
      | "AllApprovalsCancelled"
      | "CollectionConfigChanged"
      | "CollectionMetadataSet"
      | "CollectionMetadataCleared"
      | "ItemMetadataSet"
      | "ItemMetadataCleared"
      | "Redeposited"
      | "AttributeSet"
      | "AttributeCleared"
      | "ItemAttributesApprovalAdded"
      | "ItemAttributesApprovalRemoved"
      | "OwnershipAcceptanceChanged"
      | "CollectionMaxSupplySet"
      | "CollectionMintSettingsUpdated"
      | "NextCollectionIdIncremented"
      | "ItemPriceSet"
      | "ItemPriceRemoved"
      | "ItemBought"
      | "TipSent"
      | "SwapCreated"
      | "SwapCancelled"
      | "SwapClaimed"
      | "PreSignedAttributesSet"
      | "PalletAttributeSet";
  }

  /** @name PalletNftsAttributeNamespace (150) */
  interface PalletNftsAttributeNamespace extends Enum {
    readonly isPallet: boolean;
    readonly isCollectionOwner: boolean;
    readonly isItemOwner: boolean;
    readonly isAccount: boolean;
    readonly asAccount: AccountId20;
    readonly type: "Pallet" | "CollectionOwner" | "ItemOwner" | "Account";
  }

  /** @name PalletNftsPriceWithDirection (152) */
  interface PalletNftsPriceWithDirection extends Struct {
    readonly amount: u128;
    readonly direction: PalletNftsPriceDirection;
  }

  /** @name PalletNftsPriceDirection (153) */
  interface PalletNftsPriceDirection extends Enum {
    readonly isSend: boolean;
    readonly isReceive: boolean;
    readonly type: "Send" | "Receive";
  }

  /** @name PalletNftsPalletAttributes (154) */
  interface PalletNftsPalletAttributes extends Enum {
    readonly isUsedToClaim: boolean;
    readonly asUsedToClaim: u32;
    readonly isTransferDisabled: boolean;
    readonly type: "UsedToClaim" | "TransferDisabled";
  }

  /** @name FrameSystemPhase (155) */
  interface FrameSystemPhase extends Enum {
    readonly isApplyExtrinsic: boolean;
    readonly asApplyExtrinsic: u32;
    readonly isFinalization: boolean;
    readonly isInitialization: boolean;
    readonly type: "ApplyExtrinsic" | "Finalization" | "Initialization";
  }

  /** @name FrameSystemLastRuntimeUpgradeInfo (158) */
  interface FrameSystemLastRuntimeUpgradeInfo extends Struct {
    readonly specVersion: Compact<u32>;
    readonly specName: Text;
  }

  /** @name FrameSystemCodeUpgradeAuthorization (160) */
  interface FrameSystemCodeUpgradeAuthorization extends Struct {
    readonly codeHash: H256;
    readonly checkVersion: bool;
  }

  /** @name FrameSystemCall (161) */
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
    readonly type:
      | "Remark"
      | "SetHeapPages"
      | "SetCode"
      | "SetCodeWithoutChecks"
      | "SetStorage"
      | "KillStorage"
      | "KillPrefix"
      | "RemarkWithEvent"
      | "AuthorizeUpgrade"
      | "AuthorizeUpgradeWithoutChecks"
      | "ApplyAuthorizedUpgrade";
  }

  /** @name FrameSystemLimitsBlockWeights (164) */
  interface FrameSystemLimitsBlockWeights extends Struct {
    readonly baseBlock: SpWeightsWeightV2Weight;
    readonly maxBlock: SpWeightsWeightV2Weight;
    readonly perClass: FrameSupportDispatchPerDispatchClassWeightsPerClass;
  }

  /** @name FrameSupportDispatchPerDispatchClassWeightsPerClass (165) */
  interface FrameSupportDispatchPerDispatchClassWeightsPerClass extends Struct {
    readonly normal: FrameSystemLimitsWeightsPerClass;
    readonly operational: FrameSystemLimitsWeightsPerClass;
    readonly mandatory: FrameSystemLimitsWeightsPerClass;
  }

  /** @name FrameSystemLimitsWeightsPerClass (166) */
  interface FrameSystemLimitsWeightsPerClass extends Struct {
    readonly baseExtrinsic: SpWeightsWeightV2Weight;
    readonly maxExtrinsic: Option<SpWeightsWeightV2Weight>;
    readonly maxTotal: Option<SpWeightsWeightV2Weight>;
    readonly reserved: Option<SpWeightsWeightV2Weight>;
  }

  /** @name FrameSystemLimitsBlockLength (168) */
  interface FrameSystemLimitsBlockLength extends Struct {
    readonly max: FrameSupportDispatchPerDispatchClassU32;
  }

  /** @name FrameSupportDispatchPerDispatchClassU32 (169) */
  interface FrameSupportDispatchPerDispatchClassU32 extends Struct {
    readonly normal: u32;
    readonly operational: u32;
    readonly mandatory: u32;
  }

  /** @name SpWeightsRuntimeDbWeight (170) */
  interface SpWeightsRuntimeDbWeight extends Struct {
    readonly read: u64;
    readonly write: u64;
  }

  /** @name SpVersionRuntimeVersion (171) */
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

  /** @name FrameSystemError (177) */
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
    readonly type:
      | "InvalidSpecName"
      | "SpecVersionNeedsToIncrease"
      | "FailedToExtractRuntimeVersion"
      | "NonDefaultComposite"
      | "NonZeroRefCount"
      | "CallFiltered"
      | "MultiBlockMigrationsOngoing"
      | "NothingAuthorized"
      | "Unauthorized";
  }

  /** @name SpConsensusBabeAppPublic (180) */
  interface SpConsensusBabeAppPublic extends U8aFixed {}

  /** @name SpConsensusBabeDigestsNextConfigDescriptor (183) */
  interface SpConsensusBabeDigestsNextConfigDescriptor extends Enum {
    readonly isV1: boolean;
    readonly asV1: {
      readonly c: ITuple<[u64, u64]>;
      readonly allowedSlots: SpConsensusBabeAllowedSlots;
    } & Struct;
    readonly type: "V1";
  }

  /** @name SpConsensusBabeAllowedSlots (185) */
  interface SpConsensusBabeAllowedSlots extends Enum {
    readonly isPrimarySlots: boolean;
    readonly isPrimaryAndSecondaryPlainSlots: boolean;
    readonly isPrimaryAndSecondaryVRFSlots: boolean;
    readonly type: "PrimarySlots" | "PrimaryAndSecondaryPlainSlots" | "PrimaryAndSecondaryVRFSlots";
  }

  /** @name SpConsensusBabeDigestsPreDigest (189) */
  interface SpConsensusBabeDigestsPreDigest extends Enum {
    readonly isPrimary: boolean;
    readonly asPrimary: SpConsensusBabeDigestsPrimaryPreDigest;
    readonly isSecondaryPlain: boolean;
    readonly asSecondaryPlain: SpConsensusBabeDigestsSecondaryPlainPreDigest;
    readonly isSecondaryVRF: boolean;
    readonly asSecondaryVRF: SpConsensusBabeDigestsSecondaryVRFPreDigest;
    readonly type: "Primary" | "SecondaryPlain" | "SecondaryVRF";
  }

  /** @name SpConsensusBabeDigestsPrimaryPreDigest (190) */
  interface SpConsensusBabeDigestsPrimaryPreDigest extends Struct {
    readonly authorityIndex: u32;
    readonly slot: u64;
    readonly vrfSignature: SpCoreSr25519VrfVrfSignature;
  }

  /** @name SpCoreSr25519VrfVrfSignature (191) */
  interface SpCoreSr25519VrfVrfSignature extends Struct {
    readonly preOutput: U8aFixed;
    readonly proof: U8aFixed;
  }

  /** @name SpConsensusBabeDigestsSecondaryPlainPreDigest (193) */
  interface SpConsensusBabeDigestsSecondaryPlainPreDigest extends Struct {
    readonly authorityIndex: u32;
    readonly slot: u64;
  }

  /** @name SpConsensusBabeDigestsSecondaryVRFPreDigest (194) */
  interface SpConsensusBabeDigestsSecondaryVRFPreDigest extends Struct {
    readonly authorityIndex: u32;
    readonly slot: u64;
    readonly vrfSignature: SpCoreSr25519VrfVrfSignature;
  }

  /** @name SpConsensusBabeBabeEpochConfiguration (196) */
  interface SpConsensusBabeBabeEpochConfiguration extends Struct {
    readonly c: ITuple<[u64, u64]>;
    readonly allowedSlots: SpConsensusBabeAllowedSlots;
  }

  /** @name PalletBabeCall (200) */
  interface PalletBabeCall extends Enum {
    readonly isReportEquivocation: boolean;
    readonly asReportEquivocation: {
      readonly equivocationProof: SpConsensusSlotsEquivocationProof;
      readonly keyOwnerProof: SpSessionMembershipProof;
    } & Struct;
    readonly isReportEquivocationUnsigned: boolean;
    readonly asReportEquivocationUnsigned: {
      readonly equivocationProof: SpConsensusSlotsEquivocationProof;
      readonly keyOwnerProof: SpSessionMembershipProof;
    } & Struct;
    readonly isPlanConfigChange: boolean;
    readonly asPlanConfigChange: {
      readonly config: SpConsensusBabeDigestsNextConfigDescriptor;
    } & Struct;
    readonly type: "ReportEquivocation" | "ReportEquivocationUnsigned" | "PlanConfigChange";
  }

  /** @name SpConsensusSlotsEquivocationProof (201) */
  interface SpConsensusSlotsEquivocationProof extends Struct {
    readonly offender: SpConsensusBabeAppPublic;
    readonly slot: u64;
    readonly firstHeader: SpRuntimeHeader;
    readonly secondHeader: SpRuntimeHeader;
  }

  /** @name SpRuntimeHeader (202) */
  interface SpRuntimeHeader extends Struct {
    readonly parentHash: H256;
    readonly number: Compact<u32>;
    readonly stateRoot: H256;
    readonly extrinsicsRoot: H256;
    readonly digest: SpRuntimeDigest;
  }

  /** @name SpSessionMembershipProof (203) */
  interface SpSessionMembershipProof extends Struct {
    readonly session: u32;
    readonly trieNodes: Vec<Bytes>;
    readonly validatorCount: u32;
  }

  /** @name PalletBabeError (204) */
  interface PalletBabeError extends Enum {
    readonly isInvalidEquivocationProof: boolean;
    readonly isInvalidKeyOwnershipProof: boolean;
    readonly isDuplicateOffenceReport: boolean;
    readonly isInvalidConfiguration: boolean;
    readonly type:
      | "InvalidEquivocationProof"
      | "InvalidKeyOwnershipProof"
      | "DuplicateOffenceReport"
      | "InvalidConfiguration";
  }

  /** @name PalletTimestampCall (205) */
  interface PalletTimestampCall extends Enum {
    readonly isSet: boolean;
    readonly asSet: {
      readonly now: Compact<u64>;
    } & Struct;
    readonly type: "Set";
  }

  /** @name PalletBalancesBalanceLock (207) */
  interface PalletBalancesBalanceLock extends Struct {
    readonly id: U8aFixed;
    readonly amount: u128;
    readonly reasons: PalletBalancesReasons;
  }

  /** @name PalletBalancesReasons (208) */
  interface PalletBalancesReasons extends Enum {
    readonly isFee: boolean;
    readonly isMisc: boolean;
    readonly isAll: boolean;
    readonly type: "Fee" | "Misc" | "All";
  }

  /** @name PalletBalancesReserveData (211) */
  interface PalletBalancesReserveData extends Struct {
    readonly id: U8aFixed;
    readonly amount: u128;
  }

  /** @name FrameSupportTokensMiscIdAmountRuntimeHoldReason (214) */
  interface FrameSupportTokensMiscIdAmountRuntimeHoldReason extends Struct {
    readonly id: ShSolochainEvmRuntimeRuntimeHoldReason;
    readonly amount: u128;
  }

  /** @name ShSolochainEvmRuntimeRuntimeHoldReason (215) */
  interface ShSolochainEvmRuntimeRuntimeHoldReason extends Enum {
    readonly isProviders: boolean;
    readonly asProviders: PalletStorageProvidersHoldReason;
    readonly isFileSystem: boolean;
    readonly asFileSystem: PalletFileSystemHoldReason;
    readonly isPaymentStreams: boolean;
    readonly asPaymentStreams: PalletPaymentStreamsHoldReason;
    readonly type: "Providers" | "FileSystem" | "PaymentStreams";
  }

  /** @name PalletStorageProvidersHoldReason (216) */
  interface PalletStorageProvidersHoldReason extends Enum {
    readonly isStorageProviderDeposit: boolean;
    readonly isBucketDeposit: boolean;
    readonly type: "StorageProviderDeposit" | "BucketDeposit";
  }

  /** @name PalletFileSystemHoldReason (217) */
  interface PalletFileSystemHoldReason extends Enum {
    readonly isStorageRequestCreationHold: boolean;
    readonly isFileDeletionRequestHold: boolean;
    readonly type: "StorageRequestCreationHold" | "FileDeletionRequestHold";
  }

  /** @name PalletPaymentStreamsHoldReason (218) */
  interface PalletPaymentStreamsHoldReason extends Enum {
    readonly isPaymentStreamDeposit: boolean;
    readonly type: "PaymentStreamDeposit";
  }

  /** @name FrameSupportTokensMiscIdAmountRuntimeFreezeReason (221) */
  interface FrameSupportTokensMiscIdAmountRuntimeFreezeReason extends Struct {
    readonly id: ShSolochainEvmRuntimeRuntimeFreezeReason;
    readonly amount: u128;
  }

  /** @name ShSolochainEvmRuntimeRuntimeFreezeReason (222) */
  type ShSolochainEvmRuntimeRuntimeFreezeReason = Null;

  /** @name PalletBalancesCall (224) */
  interface PalletBalancesCall extends Enum {
    readonly isTransferAllowDeath: boolean;
    readonly asTransferAllowDeath: {
      readonly dest: AccountId20;
      readonly value: Compact<u128>;
    } & Struct;
    readonly isForceTransfer: boolean;
    readonly asForceTransfer: {
      readonly source: AccountId20;
      readonly dest: AccountId20;
      readonly value: Compact<u128>;
    } & Struct;
    readonly isTransferKeepAlive: boolean;
    readonly asTransferKeepAlive: {
      readonly dest: AccountId20;
      readonly value: Compact<u128>;
    } & Struct;
    readonly isTransferAll: boolean;
    readonly asTransferAll: {
      readonly dest: AccountId20;
      readonly keepAlive: bool;
    } & Struct;
    readonly isForceUnreserve: boolean;
    readonly asForceUnreserve: {
      readonly who: AccountId20;
      readonly amount: u128;
    } & Struct;
    readonly isUpgradeAccounts: boolean;
    readonly asUpgradeAccounts: {
      readonly who: Vec<AccountId20>;
    } & Struct;
    readonly isForceSetBalance: boolean;
    readonly asForceSetBalance: {
      readonly who: AccountId20;
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
    readonly type:
      | "TransferAllowDeath"
      | "ForceTransfer"
      | "TransferKeepAlive"
      | "TransferAll"
      | "ForceUnreserve"
      | "UpgradeAccounts"
      | "ForceSetBalance"
      | "ForceAdjustTotalIssuance"
      | "Burn";
  }

  /** @name PalletBalancesAdjustmentDirection (226) */
  interface PalletBalancesAdjustmentDirection extends Enum {
    readonly isIncrease: boolean;
    readonly isDecrease: boolean;
    readonly type: "Increase" | "Decrease";
  }

  /** @name PalletBalancesError (227) */
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
    readonly type:
      | "VestingBalance"
      | "LiquidityRestrictions"
      | "InsufficientBalance"
      | "ExistentialDeposit"
      | "Expendability"
      | "ExistingVestingSchedule"
      | "DeadAccount"
      | "TooManyReserves"
      | "TooManyHolds"
      | "TooManyFreezes"
      | "IssuanceDeactivated"
      | "DeltaZero";
  }

  /** @name SpStakingOffenceOffenceDetails (228) */
  interface SpStakingOffenceOffenceDetails extends Struct {
    readonly offender: ITuple<[AccountId20, Null]>;
    readonly reporters: Vec<AccountId20>;
  }

  /** @name ShSolochainEvmRuntimeSessionKeys (234) */
  interface ShSolochainEvmRuntimeSessionKeys extends Struct {
    readonly babe: SpConsensusBabeAppPublic;
    readonly grandpa: SpConsensusGrandpaAppPublic;
  }

  /** @name SpCoreCryptoKeyTypeId (236) */
  interface SpCoreCryptoKeyTypeId extends U8aFixed {}

  /** @name PalletSessionCall (237) */
  interface PalletSessionCall extends Enum {
    readonly isSetKeys: boolean;
    readonly asSetKeys: {
      readonly keys_: ShSolochainEvmRuntimeSessionKeys;
      readonly proof: Bytes;
    } & Struct;
    readonly isPurgeKeys: boolean;
    readonly type: "SetKeys" | "PurgeKeys";
  }

  /** @name PalletSessionError (238) */
  interface PalletSessionError extends Enum {
    readonly isInvalidProof: boolean;
    readonly isNoAssociatedValidatorId: boolean;
    readonly isDuplicatedKey: boolean;
    readonly isNoKeys: boolean;
    readonly isNoAccount: boolean;
    readonly type:
      | "InvalidProof"
      | "NoAssociatedValidatorId"
      | "DuplicatedKey"
      | "NoKeys"
      | "NoAccount";
  }

  /** @name PalletGrandpaStoredState (239) */
  interface PalletGrandpaStoredState extends Enum {
    readonly isLive: boolean;
    readonly isPendingPause: boolean;
    readonly asPendingPause: {
      readonly scheduledAt: u32;
      readonly delay: u32;
    } & Struct;
    readonly isPaused: boolean;
    readonly isPendingResume: boolean;
    readonly asPendingResume: {
      readonly scheduledAt: u32;
      readonly delay: u32;
    } & Struct;
    readonly type: "Live" | "PendingPause" | "Paused" | "PendingResume";
  }

  /** @name PalletGrandpaStoredPendingChange (240) */
  interface PalletGrandpaStoredPendingChange extends Struct {
    readonly scheduledAt: u32;
    readonly delay: u32;
    readonly nextAuthorities: Vec<ITuple<[SpConsensusGrandpaAppPublic, u64]>>;
    readonly forced: Option<u32>;
  }

  /** @name PalletGrandpaCall (242) */
  interface PalletGrandpaCall extends Enum {
    readonly isReportEquivocation: boolean;
    readonly asReportEquivocation: {
      readonly equivocationProof: SpConsensusGrandpaEquivocationProof;
      readonly keyOwnerProof: SpSessionMembershipProof;
    } & Struct;
    readonly isReportEquivocationUnsigned: boolean;
    readonly asReportEquivocationUnsigned: {
      readonly equivocationProof: SpConsensusGrandpaEquivocationProof;
      readonly keyOwnerProof: SpSessionMembershipProof;
    } & Struct;
    readonly isNoteStalled: boolean;
    readonly asNoteStalled: {
      readonly delay: u32;
      readonly bestFinalizedBlockNumber: u32;
    } & Struct;
    readonly type: "ReportEquivocation" | "ReportEquivocationUnsigned" | "NoteStalled";
  }

  /** @name SpConsensusGrandpaEquivocationProof (243) */
  interface SpConsensusGrandpaEquivocationProof extends Struct {
    readonly setId: u64;
    readonly equivocation: SpConsensusGrandpaEquivocation;
  }

  /** @name SpConsensusGrandpaEquivocation (244) */
  interface SpConsensusGrandpaEquivocation extends Enum {
    readonly isPrevote: boolean;
    readonly asPrevote: FinalityGrandpaEquivocationPrevote;
    readonly isPrecommit: boolean;
    readonly asPrecommit: FinalityGrandpaEquivocationPrecommit;
    readonly type: "Prevote" | "Precommit";
  }

  /** @name FinalityGrandpaEquivocationPrevote (245) */
  interface FinalityGrandpaEquivocationPrevote extends Struct {
    readonly roundNumber: u64;
    readonly identity: SpConsensusGrandpaAppPublic;
    readonly first: ITuple<[FinalityGrandpaPrevote, SpConsensusGrandpaAppSignature]>;
    readonly second: ITuple<[FinalityGrandpaPrevote, SpConsensusGrandpaAppSignature]>;
  }

  /** @name FinalityGrandpaPrevote (246) */
  interface FinalityGrandpaPrevote extends Struct {
    readonly targetHash: H256;
    readonly targetNumber: u32;
  }

  /** @name SpConsensusGrandpaAppSignature (247) */
  interface SpConsensusGrandpaAppSignature extends U8aFixed {}

  /** @name FinalityGrandpaEquivocationPrecommit (249) */
  interface FinalityGrandpaEquivocationPrecommit extends Struct {
    readonly roundNumber: u64;
    readonly identity: SpConsensusGrandpaAppPublic;
    readonly first: ITuple<[FinalityGrandpaPrecommit, SpConsensusGrandpaAppSignature]>;
    readonly second: ITuple<[FinalityGrandpaPrecommit, SpConsensusGrandpaAppSignature]>;
  }

  /** @name FinalityGrandpaPrecommit (250) */
  interface FinalityGrandpaPrecommit extends Struct {
    readonly targetHash: H256;
    readonly targetNumber: u32;
  }

  /** @name PalletGrandpaError (252) */
  interface PalletGrandpaError extends Enum {
    readonly isPauseFailed: boolean;
    readonly isResumeFailed: boolean;
    readonly isChangePending: boolean;
    readonly isTooSoon: boolean;
    readonly isInvalidKeyOwnershipProof: boolean;
    readonly isInvalidEquivocationProof: boolean;
    readonly isDuplicateOffenceReport: boolean;
    readonly type:
      | "PauseFailed"
      | "ResumeFailed"
      | "ChangePending"
      | "TooSoon"
      | "InvalidKeyOwnershipProof"
      | "InvalidEquivocationProof"
      | "DuplicateOffenceReport";
  }

  /** @name PalletTransactionPaymentReleases (254) */
  interface PalletTransactionPaymentReleases extends Enum {
    readonly isV1Ancient: boolean;
    readonly isV2: boolean;
    readonly type: "V1Ancient" | "V2";
  }

  /** @name PalletParametersCall (255) */
  interface PalletParametersCall extends Enum {
    readonly isSetParameter: boolean;
    readonly asSetParameter: {
      readonly keyValue: ShSolochainEvmRuntimeConfigsRuntimeParamsRuntimeParameters;
    } & Struct;
    readonly type: "SetParameter";
  }

  /** @name ShSolochainEvmRuntimeConfigsRuntimeParamsRuntimeParameters (256) */
  interface ShSolochainEvmRuntimeConfigsRuntimeParamsRuntimeParameters extends Enum {
    readonly isRuntimeConfig: boolean;
    readonly asRuntimeConfig: ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigParameters;
    readonly type: "RuntimeConfig";
  }

  /** @name ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigParameters (257) */
  interface ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigParameters
    extends Enum {
    readonly isSlashAmountPerMaxFileSize: boolean;
    readonly asSlashAmountPerMaxFileSize: ITuple<
      [
        ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigSlashAmountPerMaxFileSize,
        Option<u128>
      ]
    >;
    readonly isStakeToChallengePeriod: boolean;
    readonly asStakeToChallengePeriod: ITuple<
      [
        ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigStakeToChallengePeriod,
        Option<u128>
      ]
    >;
    readonly isCheckpointChallengePeriod: boolean;
    readonly asCheckpointChallengePeriod: ITuple<
      [
        ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigCheckpointChallengePeriod,
        Option<u32>
      ]
    >;
    readonly isMinChallengePeriod: boolean;
    readonly asMinChallengePeriod: ITuple<
      [
        ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMinChallengePeriod,
        Option<u32>
      ]
    >;
    readonly isSystemUtilisationLowerThresholdPercentage: boolean;
    readonly asSystemUtilisationLowerThresholdPercentage: ITuple<
      [
        ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigSystemUtilisationLowerThresholdPercentage,
        Option<Perbill>
      ]
    >;
    readonly isSystemUtilisationUpperThresholdPercentage: boolean;
    readonly asSystemUtilisationUpperThresholdPercentage: ITuple<
      [
        ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigSystemUtilisationUpperThresholdPercentage,
        Option<Perbill>
      ]
    >;
    readonly isMostlyStablePrice: boolean;
    readonly asMostlyStablePrice: ITuple<
      [
        ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMostlyStablePrice,
        Option<u128>
      ]
    >;
    readonly isMaxPrice: boolean;
    readonly asMaxPrice: ITuple<
      [ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMaxPrice, Option<u128>]
    >;
    readonly isMinPrice: boolean;
    readonly asMinPrice: ITuple<
      [ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMinPrice, Option<u128>]
    >;
    readonly isUpperExponentFactor: boolean;
    readonly asUpperExponentFactor: ITuple<
      [
        ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigUpperExponentFactor,
        Option<u32>
      ]
    >;
    readonly isLowerExponentFactor: boolean;
    readonly asLowerExponentFactor: ITuple<
      [
        ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigLowerExponentFactor,
        Option<u32>
      ]
    >;
    readonly isZeroSizeBucketFixedRate: boolean;
    readonly asZeroSizeBucketFixedRate: ITuple<
      [
        ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigZeroSizeBucketFixedRate,
        Option<u128>
      ]
    >;
    readonly isIdealUtilisationRate: boolean;
    readonly asIdealUtilisationRate: ITuple<
      [
        ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigIdealUtilisationRate,
        Option<Perbill>
      ]
    >;
    readonly isDecayRate: boolean;
    readonly asDecayRate: ITuple<
      [
        ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigDecayRate,
        Option<Perbill>
      ]
    >;
    readonly isMinimumTreasuryCut: boolean;
    readonly asMinimumTreasuryCut: ITuple<
      [
        ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMinimumTreasuryCut,
        Option<Perbill>
      ]
    >;
    readonly isMaximumTreasuryCut: boolean;
    readonly asMaximumTreasuryCut: ITuple<
      [
        ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMaximumTreasuryCut,
        Option<Perbill>
      ]
    >;
    readonly isBspStopStoringFilePenalty: boolean;
    readonly asBspStopStoringFilePenalty: ITuple<
      [
        ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigBspStopStoringFilePenalty,
        Option<u128>
      ]
    >;
    readonly isProviderTopUpTtl: boolean;
    readonly asProviderTopUpTtl: ITuple<
      [
        ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigProviderTopUpTtl,
        Option<u32>
      ]
    >;
    readonly isBasicReplicationTarget: boolean;
    readonly asBasicReplicationTarget: ITuple<
      [
        ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigBasicReplicationTarget,
        Option<u32>
      ]
    >;
    readonly isStandardReplicationTarget: boolean;
    readonly asStandardReplicationTarget: ITuple<
      [
        ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigStandardReplicationTarget,
        Option<u32>
      ]
    >;
    readonly isHighSecurityReplicationTarget: boolean;
    readonly asHighSecurityReplicationTarget: ITuple<
      [
        ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigHighSecurityReplicationTarget,
        Option<u32>
      ]
    >;
    readonly isSuperHighSecurityReplicationTarget: boolean;
    readonly asSuperHighSecurityReplicationTarget: ITuple<
      [
        ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigSuperHighSecurityReplicationTarget,
        Option<u32>
      ]
    >;
    readonly isUltraHighSecurityReplicationTarget: boolean;
    readonly asUltraHighSecurityReplicationTarget: ITuple<
      [
        ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigUltraHighSecurityReplicationTarget,
        Option<u32>
      ]
    >;
    readonly isMaxReplicationTarget: boolean;
    readonly asMaxReplicationTarget: ITuple<
      [
        ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMaxReplicationTarget,
        Option<u32>
      ]
    >;
    readonly isTickRangeToMaximumThreshold: boolean;
    readonly asTickRangeToMaximumThreshold: ITuple<
      [
        ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigTickRangeToMaximumThreshold,
        Option<u32>
      ]
    >;
    readonly isStorageRequestTtl: boolean;
    readonly asStorageRequestTtl: ITuple<
      [
        ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigStorageRequestTtl,
        Option<u32>
      ]
    >;
    readonly isMinWaitForStopStoring: boolean;
    readonly asMinWaitForStopStoring: ITuple<
      [
        ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMinWaitForStopStoring,
        Option<u32>
      ]
    >;
    readonly isMinSeedPeriod: boolean;
    readonly asMinSeedPeriod: ITuple<
      [
        ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMinSeedPeriod,
        Option<u32>
      ]
    >;
    readonly isStakeToSeedPeriod: boolean;
    readonly asStakeToSeedPeriod: ITuple<
      [
        ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigStakeToSeedPeriod,
        Option<u128>
      ]
    >;
    readonly isUpfrontTicksToPay: boolean;
    readonly asUpfrontTicksToPay: ITuple<
      [
        ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigUpfrontTicksToPay,
        Option<u32>
      ]
    >;
    readonly type:
      | "SlashAmountPerMaxFileSize"
      | "StakeToChallengePeriod"
      | "CheckpointChallengePeriod"
      | "MinChallengePeriod"
      | "SystemUtilisationLowerThresholdPercentage"
      | "SystemUtilisationUpperThresholdPercentage"
      | "MostlyStablePrice"
      | "MaxPrice"
      | "MinPrice"
      | "UpperExponentFactor"
      | "LowerExponentFactor"
      | "ZeroSizeBucketFixedRate"
      | "IdealUtilisationRate"
      | "DecayRate"
      | "MinimumTreasuryCut"
      | "MaximumTreasuryCut"
      | "BspStopStoringFilePenalty"
      | "ProviderTopUpTtl"
      | "BasicReplicationTarget"
      | "StandardReplicationTarget"
      | "HighSecurityReplicationTarget"
      | "SuperHighSecurityReplicationTarget"
      | "UltraHighSecurityReplicationTarget"
      | "MaxReplicationTarget"
      | "TickRangeToMaximumThreshold"
      | "StorageRequestTtl"
      | "MinWaitForStopStoring"
      | "MinSeedPeriod"
      | "StakeToSeedPeriod"
      | "UpfrontTicksToPay";
  }

  /** @name PalletSudoCall (260) */
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
      readonly new_: AccountId20;
    } & Struct;
    readonly isSudoAs: boolean;
    readonly asSudoAs: {
      readonly who: AccountId20;
      readonly call: Call;
    } & Struct;
    readonly isRemoveKey: boolean;
    readonly type: "Sudo" | "SudoUncheckedWeight" | "SetKey" | "SudoAs" | "RemoveKey";
  }

  /** @name PalletEthereumCall (262) */
  interface PalletEthereumCall extends Enum {
    readonly isTransact: boolean;
    readonly asTransact: {
      readonly transaction: EthereumTransactionTransactionV2;
    } & Struct;
    readonly type: "Transact";
  }

  /** @name EthereumTransactionTransactionV2 (263) */
  interface EthereumTransactionTransactionV2 extends Enum {
    readonly isLegacy: boolean;
    readonly asLegacy: EthereumTransactionLegacyLegacyTransaction;
    readonly isEip2930: boolean;
    readonly asEip2930: EthereumTransactionEip2930Eip2930Transaction;
    readonly isEip1559: boolean;
    readonly asEip1559: EthereumTransactionEip1559Eip1559Transaction;
    readonly type: "Legacy" | "Eip2930" | "Eip1559";
  }

  /** @name EthereumTransactionLegacyLegacyTransaction (264) */
  interface EthereumTransactionLegacyLegacyTransaction extends Struct {
    readonly nonce: U256;
    readonly gasPrice: U256;
    readonly gasLimit: U256;
    readonly action: EthereumTransactionLegacyTransactionAction;
    readonly value: U256;
    readonly input: Bytes;
    readonly signature: EthereumTransactionLegacyTransactionSignature;
  }

  /** @name EthereumTransactionLegacyTransactionAction (267) */
  interface EthereumTransactionLegacyTransactionAction extends Enum {
    readonly isCall: boolean;
    readonly asCall: H160;
    readonly isCreate: boolean;
    readonly type: "Call" | "Create";
  }

  /** @name EthereumTransactionLegacyTransactionSignature (268) */
  interface EthereumTransactionLegacyTransactionSignature extends Struct {
    readonly v: u64;
    readonly r: H256;
    readonly s: H256;
  }

  /** @name EthereumTransactionEip2930Eip2930Transaction (270) */
  interface EthereumTransactionEip2930Eip2930Transaction extends Struct {
    readonly chainId: u64;
    readonly nonce: U256;
    readonly gasPrice: U256;
    readonly gasLimit: U256;
    readonly action: EthereumTransactionLegacyTransactionAction;
    readonly value: U256;
    readonly input: Bytes;
    readonly accessList: Vec<EthereumTransactionEip2930AccessListItem>;
    readonly oddYParity: bool;
    readonly r: H256;
    readonly s: H256;
  }

  /** @name EthereumTransactionEip2930AccessListItem (272) */
  interface EthereumTransactionEip2930AccessListItem extends Struct {
    readonly address: H160;
    readonly storageKeys: Vec<H256>;
  }

  /** @name EthereumTransactionEip1559Eip1559Transaction (273) */
  interface EthereumTransactionEip1559Eip1559Transaction extends Struct {
    readonly chainId: u64;
    readonly nonce: U256;
    readonly maxPriorityFeePerGas: U256;
    readonly maxFeePerGas: U256;
    readonly gasLimit: U256;
    readonly action: EthereumTransactionLegacyTransactionAction;
    readonly value: U256;
    readonly input: Bytes;
    readonly accessList: Vec<EthereumTransactionEip2930AccessListItem>;
    readonly oddYParity: bool;
    readonly r: H256;
    readonly s: H256;
  }

  /** @name PalletEvmCall (274) */
  interface PalletEvmCall extends Enum {
    readonly isWithdraw: boolean;
    readonly asWithdraw: {
      readonly address: H160;
      readonly value: u128;
    } & Struct;
    readonly isCall: boolean;
    readonly asCall: {
      readonly source: H160;
      readonly target: H160;
      readonly input: Bytes;
      readonly value: U256;
      readonly gasLimit: u64;
      readonly maxFeePerGas: U256;
      readonly maxPriorityFeePerGas: Option<U256>;
      readonly nonce: Option<U256>;
      readonly accessList: Vec<ITuple<[H160, Vec<H256>]>>;
    } & Struct;
    readonly isCreate: boolean;
    readonly asCreate: {
      readonly source: H160;
      readonly init: Bytes;
      readonly value: U256;
      readonly gasLimit: u64;
      readonly maxFeePerGas: U256;
      readonly maxPriorityFeePerGas: Option<U256>;
      readonly nonce: Option<U256>;
      readonly accessList: Vec<ITuple<[H160, Vec<H256>]>>;
    } & Struct;
    readonly isCreate2: boolean;
    readonly asCreate2: {
      readonly source: H160;
      readonly init: Bytes;
      readonly salt: H256;
      readonly value: U256;
      readonly gasLimit: u64;
      readonly maxFeePerGas: U256;
      readonly maxPriorityFeePerGas: Option<U256>;
      readonly nonce: Option<U256>;
      readonly accessList: Vec<ITuple<[H160, Vec<H256>]>>;
    } & Struct;
    readonly type: "Withdraw" | "Call" | "Create" | "Create2";
  }

  /** @name PalletStorageProvidersCall (278) */
  interface PalletStorageProvidersCall extends Enum {
    readonly isRequestMspSignUp: boolean;
    readonly asRequestMspSignUp: {
      readonly capacity: u64;
      readonly multiaddresses: Vec<Bytes>;
      readonly valuePropPricePerGigaUnitOfDataPerBlock: u128;
      readonly commitment: Bytes;
      readonly valuePropMaxDataLimit: u64;
      readonly paymentAccount: AccountId20;
    } & Struct;
    readonly isRequestBspSignUp: boolean;
    readonly asRequestBspSignUp: {
      readonly capacity: u64;
      readonly multiaddresses: Vec<Bytes>;
      readonly paymentAccount: AccountId20;
    } & Struct;
    readonly isConfirmSignUp: boolean;
    readonly asConfirmSignUp: {
      readonly providerAccount: Option<AccountId20>;
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
      readonly who: AccountId20;
      readonly mspId: H256;
      readonly capacity: u64;
      readonly multiaddresses: Vec<Bytes>;
      readonly valuePropPricePerGigaUnitOfDataPerBlock: u128;
      readonly commitment: Bytes;
      readonly valuePropMaxDataLimit: u64;
      readonly paymentAccount: AccountId20;
    } & Struct;
    readonly isForceBspSignUp: boolean;
    readonly asForceBspSignUp: {
      readonly who: AccountId20;
      readonly bspId: H256;
      readonly capacity: u64;
      readonly multiaddresses: Vec<Bytes>;
      readonly paymentAccount: AccountId20;
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
    readonly type:
      | "RequestMspSignUp"
      | "RequestBspSignUp"
      | "ConfirmSignUp"
      | "CancelSignUp"
      | "MspSignOff"
      | "BspSignOff"
      | "ChangeCapacity"
      | "AddValueProp"
      | "MakeValuePropUnavailable"
      | "AddMultiaddress"
      | "RemoveMultiaddress"
      | "ForceMspSignUp"
      | "ForceBspSignUp"
      | "Slash"
      | "TopUpDeposit"
      | "DeleteProvider"
      | "StopAllCycles";
  }

  /** @name PalletFileSystemCall (279) */
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
      readonly owner: AccountId20;
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
      readonly owner: AccountId20;
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
      readonly signature: FpAccountEthereumSignature;
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
    readonly type:
      | "CreateBucket"
      | "RequestMoveBucket"
      | "MspRespondMoveBucketRequest"
      | "UpdateBucketPrivacy"
      | "CreateAndAssociateCollectionWithBucket"
      | "DeleteBucket"
      | "IssueStorageRequest"
      | "RevokeStorageRequest"
      | "MspRespondStorageRequestsMultipleBuckets"
      | "MspStopStoringBucket"
      | "BspVolunteer"
      | "BspConfirmStoring"
      | "BspRequestStopStoring"
      | "BspConfirmStopStoring"
      | "StopStoringForInsolventUser"
      | "MspStopStoringBucketForInsolventUser"
      | "RequestDeleteFile"
      | "DeleteFiles"
      | "DeleteFilesForIncompleteStorageRequest";
  }

  /** @name PalletFileSystemBucketMoveRequestResponse (280) */
  interface PalletFileSystemBucketMoveRequestResponse extends Enum {
    readonly isAccepted: boolean;
    readonly isRejected: boolean;
    readonly type: "Accepted" | "Rejected";
  }

  /** @name PalletFileSystemReplicationTarget (281) */
  interface PalletFileSystemReplicationTarget extends Enum {
    readonly isBasic: boolean;
    readonly isStandard: boolean;
    readonly isHighSecurity: boolean;
    readonly isSuperHighSecurity: boolean;
    readonly isUltraHighSecurity: boolean;
    readonly isCustom: boolean;
    readonly asCustom: u32;
    readonly type:
      | "Basic"
      | "Standard"
      | "HighSecurity"
      | "SuperHighSecurity"
      | "UltraHighSecurity"
      | "Custom";
  }

  /** @name PalletFileSystemStorageRequestMspBucketResponse (283) */
  interface PalletFileSystemStorageRequestMspBucketResponse extends Struct {
    readonly bucketId: H256;
    readonly accept: Option<PalletFileSystemStorageRequestMspAcceptedFileKeys>;
    readonly reject: Vec<PalletFileSystemRejectedStorageRequest>;
  }

  /** @name PalletFileSystemStorageRequestMspAcceptedFileKeys (285) */
  interface PalletFileSystemStorageRequestMspAcceptedFileKeys extends Struct {
    readonly fileKeysAndProofs: Vec<PalletFileSystemFileKeyWithProof>;
    readonly forestProof: SpTrieStorageProofCompactProof;
  }

  /** @name PalletFileSystemFileKeyWithProof (287) */
  interface PalletFileSystemFileKeyWithProof extends Struct {
    readonly fileKey: H256;
    readonly proof: ShpFileKeyVerifierFileKeyProof;
  }

  /** @name PalletFileSystemRejectedStorageRequest (289) */
  interface PalletFileSystemRejectedStorageRequest extends Struct {
    readonly fileKey: H256;
    readonly reason: PalletFileSystemRejectedStorageRequestReason;
  }

  /** @name PalletFileSystemFileDeletionRequest (292) */
  interface PalletFileSystemFileDeletionRequest extends Struct {
    readonly fileOwner: AccountId20;
    readonly signedIntention: PalletFileSystemFileOperationIntention;
    readonly signature: FpAccountEthereumSignature;
    readonly bucketId: H256;
    readonly location: Bytes;
    readonly size_: u64;
    readonly fingerprint: H256;
  }

  /** @name PalletProofsDealerCall (294) */
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
    readonly type:
      | "Challenge"
      | "SubmitProof"
      | "ForceInitialiseChallengeCycle"
      | "SetPaused"
      | "PriorityChallenge";
  }

  /** @name PalletRandomnessCall (295) */
  interface PalletRandomnessCall extends Enum {
    readonly isSetBabeRandomness: boolean;
    readonly type: "SetBabeRandomness";
  }

  /** @name PalletPaymentStreamsCall (296) */
  interface PalletPaymentStreamsCall extends Enum {
    readonly isCreateFixedRatePaymentStream: boolean;
    readonly asCreateFixedRatePaymentStream: {
      readonly providerId: H256;
      readonly userAccount: AccountId20;
      readonly rate: u128;
    } & Struct;
    readonly isUpdateFixedRatePaymentStream: boolean;
    readonly asUpdateFixedRatePaymentStream: {
      readonly providerId: H256;
      readonly userAccount: AccountId20;
      readonly newRate: u128;
    } & Struct;
    readonly isDeleteFixedRatePaymentStream: boolean;
    readonly asDeleteFixedRatePaymentStream: {
      readonly providerId: H256;
      readonly userAccount: AccountId20;
    } & Struct;
    readonly isCreateDynamicRatePaymentStream: boolean;
    readonly asCreateDynamicRatePaymentStream: {
      readonly providerId: H256;
      readonly userAccount: AccountId20;
      readonly amountProvided: u64;
    } & Struct;
    readonly isUpdateDynamicRatePaymentStream: boolean;
    readonly asUpdateDynamicRatePaymentStream: {
      readonly providerId: H256;
      readonly userAccount: AccountId20;
      readonly newAmountProvided: u64;
    } & Struct;
    readonly isDeleteDynamicRatePaymentStream: boolean;
    readonly asDeleteDynamicRatePaymentStream: {
      readonly providerId: H256;
      readonly userAccount: AccountId20;
    } & Struct;
    readonly isChargePaymentStreams: boolean;
    readonly asChargePaymentStreams: {
      readonly userAccount: AccountId20;
    } & Struct;
    readonly isChargeMultipleUsersPaymentStreams: boolean;
    readonly asChargeMultipleUsersPaymentStreams: {
      readonly userAccounts: Vec<AccountId20>;
    } & Struct;
    readonly isPayOutstandingDebt: boolean;
    readonly asPayOutstandingDebt: {
      readonly providers: Vec<H256>;
    } & Struct;
    readonly isClearInsolventFlag: boolean;
    readonly type:
      | "CreateFixedRatePaymentStream"
      | "UpdateFixedRatePaymentStream"
      | "DeleteFixedRatePaymentStream"
      | "CreateDynamicRatePaymentStream"
      | "UpdateDynamicRatePaymentStream"
      | "DeleteDynamicRatePaymentStream"
      | "ChargePaymentStreams"
      | "ChargeMultipleUsersPaymentStreams"
      | "PayOutstandingDebt"
      | "ClearInsolventFlag";
  }

  /** @name PalletBucketNftsCall (297) */
  interface PalletBucketNftsCall extends Enum {
    readonly isShareAccess: boolean;
    readonly asShareAccess: {
      readonly recipient: AccountId20;
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
    readonly type: "ShareAccess" | "UpdateReadAccess";
  }

  /** @name PalletNftsCall (299) */
  interface PalletNftsCall extends Enum {
    readonly isCreate: boolean;
    readonly asCreate: {
      readonly admin: AccountId20;
      readonly config: PalletNftsCollectionConfig;
    } & Struct;
    readonly isForceCreate: boolean;
    readonly asForceCreate: {
      readonly owner: AccountId20;
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
      readonly mintTo: AccountId20;
      readonly witnessData: Option<PalletNftsMintWitness>;
    } & Struct;
    readonly isForceMint: boolean;
    readonly asForceMint: {
      readonly collection: u32;
      readonly item: u32;
      readonly mintTo: AccountId20;
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
      readonly dest: AccountId20;
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
      readonly newOwner: AccountId20;
    } & Struct;
    readonly isSetTeam: boolean;
    readonly asSetTeam: {
      readonly collection: u32;
      readonly issuer: Option<AccountId20>;
      readonly admin: Option<AccountId20>;
      readonly freezer: Option<AccountId20>;
    } & Struct;
    readonly isForceCollectionOwner: boolean;
    readonly asForceCollectionOwner: {
      readonly collection: u32;
      readonly owner: AccountId20;
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
      readonly delegate: AccountId20;
      readonly maybeDeadline: Option<u32>;
    } & Struct;
    readonly isCancelApproval: boolean;
    readonly asCancelApproval: {
      readonly collection: u32;
      readonly item: u32;
      readonly delegate: AccountId20;
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
      readonly setAs: Option<AccountId20>;
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
      readonly delegate: AccountId20;
    } & Struct;
    readonly isCancelItemAttributesApproval: boolean;
    readonly asCancelItemAttributesApproval: {
      readonly collection: u32;
      readonly item: u32;
      readonly delegate: AccountId20;
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
      readonly whitelistedBuyer: Option<AccountId20>;
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
      readonly signature: FpAccountEthereumSignature;
      readonly signer: AccountId20;
    } & Struct;
    readonly isSetAttributesPreSigned: boolean;
    readonly asSetAttributesPreSigned: {
      readonly data: PalletNftsPreSignedAttributes;
      readonly signature: FpAccountEthereumSignature;
      readonly signer: AccountId20;
    } & Struct;
    readonly type:
      | "Create"
      | "ForceCreate"
      | "Destroy"
      | "Mint"
      | "ForceMint"
      | "Burn"
      | "Transfer"
      | "Redeposit"
      | "LockItemTransfer"
      | "UnlockItemTransfer"
      | "LockCollection"
      | "TransferOwnership"
      | "SetTeam"
      | "ForceCollectionOwner"
      | "ForceCollectionConfig"
      | "ApproveTransfer"
      | "CancelApproval"
      | "ClearAllTransferApprovals"
      | "LockItemProperties"
      | "SetAttribute"
      | "ForceSetAttribute"
      | "ClearAttribute"
      | "ApproveItemAttributes"
      | "CancelItemAttributesApproval"
      | "SetMetadata"
      | "ClearMetadata"
      | "SetCollectionMetadata"
      | "ClearCollectionMetadata"
      | "SetAcceptOwnership"
      | "SetCollectionMaxSupply"
      | "UpdateMintSettings"
      | "SetPrice"
      | "BuyItem"
      | "PayTips"
      | "CreateSwap"
      | "CancelSwap"
      | "ClaimSwap"
      | "MintPreSigned"
      | "SetAttributesPreSigned";
  }

  /** @name PalletNftsCollectionConfig (300) */
  interface PalletNftsCollectionConfig extends Struct {
    readonly settings: u64;
    readonly maxSupply: Option<u32>;
    readonly mintSettings: PalletNftsMintSettings;
  }

  /** @name PalletNftsCollectionSetting (302) */
  interface PalletNftsCollectionSetting extends Enum {
    readonly isTransferableItems: boolean;
    readonly isUnlockedMetadata: boolean;
    readonly isUnlockedAttributes: boolean;
    readonly isUnlockedMaxSupply: boolean;
    readonly isDepositRequired: boolean;
    readonly type:
      | "TransferableItems"
      | "UnlockedMetadata"
      | "UnlockedAttributes"
      | "UnlockedMaxSupply"
      | "DepositRequired";
  }

  /** @name PalletNftsMintSettings (303) */
  interface PalletNftsMintSettings extends Struct {
    readonly mintType: PalletNftsMintType;
    readonly price: Option<u128>;
    readonly startBlock: Option<u32>;
    readonly endBlock: Option<u32>;
    readonly defaultItemSettings: u64;
  }

  /** @name PalletNftsMintType (304) */
  interface PalletNftsMintType extends Enum {
    readonly isIssuer: boolean;
    readonly isPublic: boolean;
    readonly isHolderOf: boolean;
    readonly asHolderOf: u32;
    readonly type: "Issuer" | "Public" | "HolderOf";
  }

  /** @name PalletNftsItemSetting (306) */
  interface PalletNftsItemSetting extends Enum {
    readonly isTransferable: boolean;
    readonly isUnlockedMetadata: boolean;
    readonly isUnlockedAttributes: boolean;
    readonly type: "Transferable" | "UnlockedMetadata" | "UnlockedAttributes";
  }

  /** @name PalletNftsDestroyWitness (307) */
  interface PalletNftsDestroyWitness extends Struct {
    readonly itemMetadatas: Compact<u32>;
    readonly itemConfigs: Compact<u32>;
    readonly attributes: Compact<u32>;
  }

  /** @name PalletNftsMintWitness (309) */
  interface PalletNftsMintWitness extends Struct {
    readonly ownedItem: Option<u32>;
    readonly mintPrice: Option<u128>;
  }

  /** @name PalletNftsItemConfig (310) */
  interface PalletNftsItemConfig extends Struct {
    readonly settings: u64;
  }

  /** @name PalletNftsCancelAttributesApprovalWitness (311) */
  interface PalletNftsCancelAttributesApprovalWitness extends Struct {
    readonly accountAttributes: u32;
  }

  /** @name PalletNftsItemTip (313) */
  interface PalletNftsItemTip extends Struct {
    readonly collection: u32;
    readonly item: u32;
    readonly receiver: AccountId20;
    readonly amount: u128;
  }

  /** @name PalletNftsPreSignedMint (315) */
  interface PalletNftsPreSignedMint extends Struct {
    readonly collection: u32;
    readonly item: u32;
    readonly attributes: Vec<ITuple<[Bytes, Bytes]>>;
    readonly metadata: Bytes;
    readonly onlyAccount: Option<AccountId20>;
    readonly deadline: u32;
    readonly mintPrice: Option<u128>;
  }

  /** @name PalletNftsPreSignedAttributes (316) */
  interface PalletNftsPreSignedAttributes extends Struct {
    readonly collection: u32;
    readonly item: u32;
    readonly attributes: Vec<ITuple<[Bytes, Bytes]>>;
    readonly namespace: PalletNftsAttributeNamespace;
    readonly deadline: u32;
  }

  /** @name PalletSudoError (317) */
  interface PalletSudoError extends Enum {
    readonly isRequireSudo: boolean;
    readonly type: "RequireSudo";
  }

  /** @name FpRpcTransactionStatus (319) */
  interface FpRpcTransactionStatus extends Struct {
    readonly transactionHash: H256;
    readonly transactionIndex: u32;
    readonly from: H160;
    readonly to: Option<H160>;
    readonly contractAddress: Option<H160>;
    readonly logs: Vec<EthereumLog>;
    readonly logsBloom: EthbloomBloom;
  }

  /** @name EthbloomBloom (322) */
  interface EthbloomBloom extends U8aFixed {}

  /** @name EthereumReceiptReceiptV3 (324) */
  interface EthereumReceiptReceiptV3 extends Enum {
    readonly isLegacy: boolean;
    readonly asLegacy: EthereumReceiptEip658ReceiptData;
    readonly isEip2930: boolean;
    readonly asEip2930: EthereumReceiptEip658ReceiptData;
    readonly isEip1559: boolean;
    readonly asEip1559: EthereumReceiptEip658ReceiptData;
    readonly type: "Legacy" | "Eip2930" | "Eip1559";
  }

  /** @name EthereumReceiptEip658ReceiptData (325) */
  interface EthereumReceiptEip658ReceiptData extends Struct {
    readonly statusCode: u8;
    readonly usedGas: U256;
    readonly logsBloom: EthbloomBloom;
    readonly logs: Vec<EthereumLog>;
  }

  /** @name EthereumBlock (326) */
  interface EthereumBlock extends Struct {
    readonly header: EthereumHeader;
    readonly transactions: Vec<EthereumTransactionTransactionV2>;
    readonly ommers: Vec<EthereumHeader>;
  }

  /** @name EthereumHeader (327) */
  interface EthereumHeader extends Struct {
    readonly parentHash: H256;
    readonly ommersHash: H256;
    readonly beneficiary: H160;
    readonly stateRoot: H256;
    readonly transactionsRoot: H256;
    readonly receiptsRoot: H256;
    readonly logsBloom: EthbloomBloom;
    readonly difficulty: U256;
    readonly number: U256;
    readonly gasLimit: U256;
    readonly gasUsed: U256;
    readonly timestamp: u64;
    readonly extraData: Bytes;
    readonly mixHash: H256;
    readonly nonce: EthereumTypesHashH64;
  }

  /** @name EthereumTypesHashH64 (328) */
  interface EthereumTypesHashH64 extends U8aFixed {}

  /** @name PalletEthereumError (333) */
  interface PalletEthereumError extends Enum {
    readonly isInvalidSignature: boolean;
    readonly isPreLogExists: boolean;
    readonly type: "InvalidSignature" | "PreLogExists";
  }

  /** @name PalletEvmCodeMetadata (334) */
  interface PalletEvmCodeMetadata extends Struct {
    readonly size_: u64;
    readonly hash_: H256;
  }

  /** @name PalletEvmError (336) */
  interface PalletEvmError extends Enum {
    readonly isBalanceLow: boolean;
    readonly isFeeOverflow: boolean;
    readonly isPaymentOverflow: boolean;
    readonly isWithdrawFailed: boolean;
    readonly isGasPriceTooLow: boolean;
    readonly isInvalidNonce: boolean;
    readonly isGasLimitTooLow: boolean;
    readonly isGasLimitTooHigh: boolean;
    readonly isInvalidChainId: boolean;
    readonly isInvalidSignature: boolean;
    readonly isReentrancy: boolean;
    readonly isTransactionMustComeFromEOA: boolean;
    readonly isUndefined: boolean;
    readonly type:
      | "BalanceLow"
      | "FeeOverflow"
      | "PaymentOverflow"
      | "WithdrawFailed"
      | "GasPriceTooLow"
      | "InvalidNonce"
      | "GasLimitTooLow"
      | "GasLimitTooHigh"
      | "InvalidChainId"
      | "InvalidSignature"
      | "Reentrancy"
      | "TransactionMustComeFromEOA"
      | "Undefined";
  }

  /** @name PalletStorageProvidersSignUpRequest (337) */
  interface PalletStorageProvidersSignUpRequest extends Struct {
    readonly spSignUpRequest: PalletStorageProvidersSignUpRequestSpParams;
    readonly at: u32;
  }

  /** @name PalletStorageProvidersSignUpRequestSpParams (338) */
  interface PalletStorageProvidersSignUpRequestSpParams extends Enum {
    readonly isBackupStorageProvider: boolean;
    readonly asBackupStorageProvider: PalletStorageProvidersBackupStorageProvider;
    readonly isMainStorageProvider: boolean;
    readonly asMainStorageProvider: PalletStorageProvidersMainStorageProviderSignUpRequest;
    readonly type: "BackupStorageProvider" | "MainStorageProvider";
  }

  /** @name PalletStorageProvidersBackupStorageProvider (339) */
  interface PalletStorageProvidersBackupStorageProvider extends Struct {
    readonly capacity: u64;
    readonly capacityUsed: u64;
    readonly multiaddresses: Vec<Bytes>;
    readonly root: H256;
    readonly lastCapacityChange: u32;
    readonly ownerAccount: AccountId20;
    readonly paymentAccount: AccountId20;
    readonly reputationWeight: u32;
    readonly signUpBlock: u32;
  }

  /** @name PalletStorageProvidersMainStorageProviderSignUpRequest (340) */
  interface PalletStorageProvidersMainStorageProviderSignUpRequest extends Struct {
    readonly mspInfo: PalletStorageProvidersMainStorageProvider;
    readonly valueProp: PalletStorageProvidersValueProposition;
  }

  /** @name PalletStorageProvidersMainStorageProvider (341) */
  interface PalletStorageProvidersMainStorageProvider extends Struct {
    readonly capacity: u64;
    readonly capacityUsed: u64;
    readonly multiaddresses: Vec<Bytes>;
    readonly amountOfBuckets: u128;
    readonly amountOfValueProps: u32;
    readonly lastCapacityChange: u32;
    readonly ownerAccount: AccountId20;
    readonly paymentAccount: AccountId20;
    readonly signUpBlock: u32;
  }

  /** @name PalletStorageProvidersBucket (342) */
  interface PalletStorageProvidersBucket extends Struct {
    readonly root: H256;
    readonly userId: AccountId20;
    readonly mspId: Option<H256>;
    readonly private: bool;
    readonly readAccessGroupId: Option<u32>;
    readonly size_: u64;
    readonly valuePropId: H256;
  }

  /** @name PalletStorageProvidersError (346) */
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
    readonly type:
      | "AlreadyRegistered"
      | "SignUpNotRequested"
      | "SignUpRequestPending"
      | "NoMultiAddress"
      | "InvalidMultiAddress"
      | "StorageTooLow"
      | "NotEnoughBalance"
      | "CannotHoldDeposit"
      | "StorageStillInUse"
      | "SignOffPeriodNotPassed"
      | "RandomnessNotValidYet"
      | "SignUpRequestExpired"
      | "NewCapacityLessThanUsedStorage"
      | "NewCapacityEqualsCurrentCapacity"
      | "NewCapacityCantBeZero"
      | "NotEnoughTimePassed"
      | "NewUsedCapacityExceedsStorageCapacity"
      | "DepositTooLow"
      | "NotRegistered"
      | "NoUserId"
      | "NoBucketId"
      | "SpRegisteredButDataNotFound"
      | "BucketNotFound"
      | "BucketAlreadyExists"
      | "BucketNotEmpty"
      | "BucketsMovedAmountMismatch"
      | "AppendBucketToMspFailed"
      | "ProviderNotSlashable"
      | "TopUpNotRequired"
      | "BucketMustHaveMspForOperation"
      | "MultiAddressesMaxAmountReached"
      | "MultiAddressNotFound"
      | "MultiAddressAlreadyExists"
      | "LastMultiAddressCantBeRemoved"
      | "ValuePropositionNotFound"
      | "ValuePropositionAlreadyExists"
      | "ValuePropositionNotAvailable"
      | "CantDeactivateLastValueProp"
      | "ValuePropositionsDeletedAmountMismatch"
      | "FixedRatePaymentStreamNotFound"
      | "MspAlreadyAssignedToBucket"
      | "BucketSizeExceedsLimit"
      | "BucketHasNoValueProposition"
      | "MaxBlockNumberReached"
      | "OperationNotAllowedForInsolventProvider"
      | "DeleteProviderConditionsNotMet"
      | "CannotStopCycleWithNonDefaultRoot"
      | "BspOnlyOperation"
      | "MspOnlyOperation"
      | "InvalidEncodedFileMetadata"
      | "InvalidEncodedAccountId"
      | "PaymentStreamNotFound";
  }

  /** @name PalletFileSystemStorageRequestMetadata (347) */
  interface PalletFileSystemStorageRequestMetadata extends Struct {
    readonly requestedAt: u32;
    readonly expiresAt: u32;
    readonly owner: AccountId20;
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

  /** @name PalletFileSystemMspStorageRequestStatus (348) */
  interface PalletFileSystemMspStorageRequestStatus extends Enum {
    readonly isNone: boolean;
    readonly isPending: boolean;
    readonly asPending: H256;
    readonly isAcceptedNewFile: boolean;
    readonly asAcceptedNewFile: H256;
    readonly isAcceptedExistingFile: boolean;
    readonly asAcceptedExistingFile: H256;
    readonly type: "None" | "Pending" | "AcceptedNewFile" | "AcceptedExistingFile";
  }

  /** @name PalletFileSystemStorageRequestBspsMetadata (349) */
  interface PalletFileSystemStorageRequestBspsMetadata extends Struct {
    readonly confirmed: bool;
  }

  /** @name PalletFileSystemPendingFileDeletionRequest (351) */
  interface PalletFileSystemPendingFileDeletionRequest extends Struct {
    readonly user: AccountId20;
    readonly fileKey: H256;
    readonly bucketId: H256;
    readonly fileSize: u64;
    readonly depositPaidForCreation: u128;
    readonly queuePriorityChallenge: bool;
  }

  /** @name PalletFileSystemPendingStopStoringRequest (353) */
  interface PalletFileSystemPendingStopStoringRequest extends Struct {
    readonly tickWhenRequested: u32;
    readonly fileOwner: AccountId20;
    readonly fileSize: u64;
  }

  /** @name PalletFileSystemMoveBucketRequestMetadata (354) */
  interface PalletFileSystemMoveBucketRequestMetadata extends Struct {
    readonly requester: AccountId20;
    readonly newMspId: H256;
    readonly newValuePropId: H256;
  }

  /** @name PalletFileSystemIncompleteStorageRequestMetadata (355) */
  interface PalletFileSystemIncompleteStorageRequestMetadata extends Struct {
    readonly owner: AccountId20;
    readonly bucketId: H256;
    readonly location: Bytes;
    readonly fileSize: u64;
    readonly fingerprint: H256;
    readonly pendingBspRemovals: Vec<H256>;
    readonly pendingBucketRemoval: bool;
  }

  /** @name PalletFileSystemError (357) */
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
    readonly type:
      | "NotABsp"
      | "NotAMsp"
      | "NotASp"
      | "StorageRequestAlreadyRegistered"
      | "StorageRequestNotFound"
      | "StorageRequestExists"
      | "StorageRequestNotAuthorized"
      | "StorageRequestBspsRequiredFulfilled"
      | "TooManyStorageRequestResponses"
      | "IncompleteStorageRequestNotFound"
      | "ReplicationTargetCannotBeZero"
      | "ReplicationTargetExceedsMaximum"
      | "BspNotVolunteered"
      | "BspNotConfirmed"
      | "BspAlreadyConfirmed"
      | "BspAlreadyVolunteered"
      | "BspNotEligibleToVolunteer"
      | "InsufficientAvailableCapacity"
      | "NoFileKeysToConfirm"
      | "MspNotStoringBucket"
      | "NotSelectedMsp"
      | "MspAlreadyConfirmed"
      | "RequestWithoutMsp"
      | "MspAlreadyStoringBucket"
      | "BucketNotFound"
      | "BucketNotEmpty"
      | "NotBucketOwner"
      | "BucketIsBeingMoved"
      | "InvalidBucketIdFileKeyPair"
      | "ValuePropositionNotAvailable"
      | "CollectionNotFound"
      | "MoveBucketRequestNotFound"
      | "InvalidFileKeyMetadata"
      | "FileSizeCannotBeZero"
      | "ProviderNotStoringFile"
      | "FileHasActiveStorageRequest"
      | "FileHasIncompleteStorageRequest"
      | "BatchFileDeletionMustContainSingleBucket"
      | "DuplicateFileKeyInBatchFileDeletion"
      | "NoFileKeysToDelete"
      | "FailedToPushFileKeyToBucketDeletionVector"
      | "FailedToPushUserToBspDeletionVector"
      | "FailedToPushFileKeyToBspDeletionVector"
      | "PendingStopStoringRequestNotFound"
      | "MinWaitForStopStoringNotReached"
      | "PendingStopStoringRequestAlreadyExists"
      | "ExpectedNonInclusionProof"
      | "ExpectedInclusionProof"
      | "FixedRatePaymentStreamNotFound"
      | "DynamicRatePaymentStreamNotFound"
      | "OperationNotAllowedWithInsolventUser"
      | "UserNotInsolvent"
      | "OperationNotAllowedForInsolventProvider"
      | "InvalidSignature"
      | "InvalidProviderID"
      | "InvalidSignedOperation"
      | "NoGlobalReputationWeightSet"
      | "NoBspReputationWeightSet"
      | "CannotHoldDeposit"
      | "MaxTickNumberReached"
      | "ThresholdArithmeticError"
      | "RootNotUpdated"
      | "ImpossibleFailedToGetValue"
      | "FailedToQueryEarliestFileVolunteerTick"
      | "FailedToGetOwnerAccount"
      | "FailedToGetPaymentAccount"
      | "FailedToComputeFileKey"
      | "FailedToCreateFileMetadata"
      | "FileMetadataProcessingQueueFull";
  }

  /** @name PalletProofsDealerProofSubmissionRecord (359) */
  interface PalletProofsDealerProofSubmissionRecord extends Struct {
    readonly lastTickProven: u32;
    readonly nextTickToSubmitProofFor: u32;
  }

  /** @name PalletProofsDealerError (366) */
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
    readonly type:
      | "NotProvider"
      | "ChallengesQueueOverflow"
      | "PriorityChallengesQueueOverflow"
      | "FeeChargeFailed"
      | "EmptyKeyProofs"
      | "ProviderRootNotFound"
      | "ZeroRoot"
      | "NoRecordOfLastSubmittedProof"
      | "ProviderStakeNotFound"
      | "ZeroStake"
      | "StakeCouldNotBeConverted"
      | "ChallengesTickNotReached"
      | "ChallengesTickTooOld"
      | "ChallengesTickTooLate"
      | "SeedNotFound"
      | "CheckpointChallengesNotFound"
      | "ForestProofVerificationFailed"
      | "IncorrectNumberOfKeyProofs"
      | "KeyProofNotFound"
      | "KeyProofVerificationFailed"
      | "FailedToApplyDelta"
      | "UnexpectedNumberOfRemoveMutations"
      | "FailedToUpdateProviderAfterKeyRemoval"
      | "TooManyValidProofSubmitters";
  }

  /** @name PalletPaymentStreamsFixedRatePaymentStream (368) */
  interface PalletPaymentStreamsFixedRatePaymentStream extends Struct {
    readonly rate: u128;
    readonly lastChargedTick: u32;
    readonly userDeposit: u128;
    readonly outOfFundsTick: Option<u32>;
  }

  /** @name PalletPaymentStreamsDynamicRatePaymentStream (369) */
  interface PalletPaymentStreamsDynamicRatePaymentStream extends Struct {
    readonly amountProvided: u64;
    readonly priceIndexWhenLastCharged: u128;
    readonly userDeposit: u128;
    readonly outOfFundsTick: Option<u32>;
  }

  /** @name PalletPaymentStreamsProviderLastChargeableInfo (370) */
  interface PalletPaymentStreamsProviderLastChargeableInfo extends Struct {
    readonly lastChargeableTick: u32;
    readonly priceIndex: u128;
  }

  /** @name PalletPaymentStreamsError (371) */
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
    readonly type:
      | "PaymentStreamAlreadyExists"
      | "PaymentStreamNotFound"
      | "NotAProvider"
      | "ProviderInconsistencyError"
      | "CannotHoldDeposit"
      | "UpdateRateToSameRate"
      | "UpdateAmountToSameAmount"
      | "RateCantBeZero"
      | "AmountProvidedCantBeZero"
      | "LastChargedGreaterThanLastChargeable"
      | "InvalidLastChargeableBlockNumber"
      | "InvalidLastChargeablePriceIndex"
      | "ChargeOverflow"
      | "UserWithoutFunds"
      | "UserNotFlaggedAsWithoutFunds"
      | "CooldownPeriodNotPassed"
      | "UserHasRemainingDebt"
      | "ProviderInsolvent";
  }

  /** @name PalletBucketNftsError (372) */
  interface PalletBucketNftsError extends Enum {
    readonly isBucketIsNotPrivate: boolean;
    readonly isNotBucketOwner: boolean;
    readonly isNoCorrespondingCollection: boolean;
    readonly isConvertBytesToBoundedVec: boolean;
    readonly type:
      | "BucketIsNotPrivate"
      | "NotBucketOwner"
      | "NoCorrespondingCollection"
      | "ConvertBytesToBoundedVec";
  }

  /** @name PalletNftsCollectionDetails (373) */
  interface PalletNftsCollectionDetails extends Struct {
    readonly owner: AccountId20;
    readonly ownerDeposit: u128;
    readonly items: u32;
    readonly itemMetadatas: u32;
    readonly itemConfigs: u32;
    readonly attributes: u32;
  }

  /** @name PalletNftsCollectionRole (378) */
  interface PalletNftsCollectionRole extends Enum {
    readonly isIssuer: boolean;
    readonly isFreezer: boolean;
    readonly isAdmin: boolean;
    readonly type: "Issuer" | "Freezer" | "Admin";
  }

  /** @name PalletNftsItemDetails (379) */
  interface PalletNftsItemDetails extends Struct {
    readonly owner: AccountId20;
    readonly approvals: BTreeMap<AccountId20, Option<u32>>;
    readonly deposit: PalletNftsItemDeposit;
  }

  /** @name PalletNftsItemDeposit (380) */
  interface PalletNftsItemDeposit extends Struct {
    readonly account: AccountId20;
    readonly amount: u128;
  }

  /** @name PalletNftsCollectionMetadata (385) */
  interface PalletNftsCollectionMetadata extends Struct {
    readonly deposit: u128;
    readonly data: Bytes;
  }

  /** @name PalletNftsItemMetadata (386) */
  interface PalletNftsItemMetadata extends Struct {
    readonly deposit: PalletNftsItemMetadataDeposit;
    readonly data: Bytes;
  }

  /** @name PalletNftsItemMetadataDeposit (387) */
  interface PalletNftsItemMetadataDeposit extends Struct {
    readonly account: Option<AccountId20>;
    readonly amount: u128;
  }

  /** @name PalletNftsAttributeDeposit (390) */
  interface PalletNftsAttributeDeposit extends Struct {
    readonly account: Option<AccountId20>;
    readonly amount: u128;
  }

  /** @name PalletNftsPendingSwap (394) */
  interface PalletNftsPendingSwap extends Struct {
    readonly desiredCollection: u32;
    readonly desiredItem: Option<u32>;
    readonly price: Option<PalletNftsPriceWithDirection>;
    readonly deadline: u32;
  }

  /** @name PalletNftsPalletFeature (396) */
  interface PalletNftsPalletFeature extends Enum {
    readonly isTrading: boolean;
    readonly isAttributes: boolean;
    readonly isApprovals: boolean;
    readonly isSwaps: boolean;
    readonly type: "Trading" | "Attributes" | "Approvals" | "Swaps";
  }

  /** @name PalletNftsError (397) */
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
    readonly type:
      | "NoPermission"
      | "UnknownCollection"
      | "AlreadyExists"
      | "ApprovalExpired"
      | "WrongOwner"
      | "BadWitness"
      | "CollectionIdInUse"
      | "ItemsNonTransferable"
      | "NotDelegate"
      | "WrongDelegate"
      | "Unapproved"
      | "Unaccepted"
      | "ItemLocked"
      | "LockedItemAttributes"
      | "LockedCollectionAttributes"
      | "LockedItemMetadata"
      | "LockedCollectionMetadata"
      | "MaxSupplyReached"
      | "MaxSupplyLocked"
      | "MaxSupplyTooSmall"
      | "UnknownItem"
      | "UnknownSwap"
      | "MetadataNotFound"
      | "AttributeNotFound"
      | "NotForSale"
      | "BidTooLow"
      | "ReachedApprovalLimit"
      | "DeadlineExpired"
      | "WrongDuration"
      | "MethodDisabled"
      | "WrongSetting"
      | "InconsistentItemConfig"
      | "NoConfig"
      | "RolesNotCleared"
      | "MintNotStarted"
      | "MintEnded"
      | "AlreadyClaimed"
      | "IncorrectData"
      | "WrongOrigin"
      | "WrongSignature"
      | "IncorrectMetadata"
      | "MaxAttributesLimitReached"
      | "WrongNamespace"
      | "CollectionNotEmpty"
      | "WitnessRequired";
  }

  /** @name FrameSystemExtensionsCheckNonZeroSender (400) */
  type FrameSystemExtensionsCheckNonZeroSender = Null;

  /** @name FrameSystemExtensionsCheckSpecVersion (401) */
  type FrameSystemExtensionsCheckSpecVersion = Null;

  /** @name FrameSystemExtensionsCheckTxVersion (402) */
  type FrameSystemExtensionsCheckTxVersion = Null;

  /** @name FrameSystemExtensionsCheckGenesis (403) */
  type FrameSystemExtensionsCheckGenesis = Null;

  /** @name FrameSystemExtensionsCheckNonce (406) */
  interface FrameSystemExtensionsCheckNonce extends Compact<u32> {}

  /** @name FrameSystemExtensionsCheckWeight (407) */
  type FrameSystemExtensionsCheckWeight = Null;

  /** @name PalletTransactionPaymentChargeTransactionPayment (408) */
  interface PalletTransactionPaymentChargeTransactionPayment extends Compact<u128> {}

  /** @name FrameMetadataHashExtensionCheckMetadataHash (409) */
  interface FrameMetadataHashExtensionCheckMetadataHash extends Struct {
    readonly mode: FrameMetadataHashExtensionMode;
  }

  /** @name FrameMetadataHashExtensionMode (410) */
  interface FrameMetadataHashExtensionMode extends Enum {
    readonly isDisabled: boolean;
    readonly isEnabled: boolean;
    readonly type: "Disabled" | "Enabled";
  }

  /** @name ShSolochainEvmRuntimeRuntime (412) */
  type ShSolochainEvmRuntimeRuntime = Null;
} // declare module
