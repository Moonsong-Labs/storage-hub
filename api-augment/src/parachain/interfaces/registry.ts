// Auto-generated via `yarn polkadot-types-from-defs`, do not edit
/* eslint-disable */

// import type lookup before we augment - in some environments
// this is required to allow for ambient/previous definitions
import "@polkadot/types/types/registry";

import type {
  CumulusPalletParachainSystemCall,
  CumulusPalletParachainSystemError,
  CumulusPalletParachainSystemEvent,
  CumulusPalletParachainSystemRelayStateSnapshotMessagingStateSnapshot,
  CumulusPalletParachainSystemRelayStateSnapshotRelayDispatchQueueRemainingCapacity,
  CumulusPalletParachainSystemUnincludedSegmentAncestor,
  CumulusPalletParachainSystemUnincludedSegmentHrmpChannelUpdate,
  CumulusPalletParachainSystemUnincludedSegmentSegmentTracker,
  CumulusPalletParachainSystemUnincludedSegmentUsedBandwidth,
  CumulusPalletXcmCall,
  CumulusPalletXcmEvent,
  CumulusPalletXcmpQueueCall,
  CumulusPalletXcmpQueueError,
  CumulusPalletXcmpQueueEvent,
  CumulusPalletXcmpQueueOutboundChannelDetails,
  CumulusPalletXcmpQueueOutboundState,
  CumulusPalletXcmpQueueQueueConfigData,
  CumulusPrimitivesCoreAggregateMessageOrigin,
  CumulusPrimitivesParachainInherentParachainInherentData,
  CumulusPrimitivesStorageWeightReclaimStorageWeightReclaim,
  FrameMetadataHashExtensionCheckMetadataHash,
  FrameMetadataHashExtensionMode,
  FrameSupportDispatchDispatchClass,
  FrameSupportDispatchPays,
  FrameSupportDispatchPerDispatchClassU32,
  FrameSupportDispatchPerDispatchClassWeight,
  FrameSupportDispatchPerDispatchClassWeightsPerClass,
  FrameSupportMessagesProcessMessageError,
  FrameSupportTokensMiscBalanceStatus,
  FrameSupportTokensMiscIdAmount,
  FrameSystemAccountInfo,
  FrameSystemCall,
  FrameSystemCodeUpgradeAuthorization,
  FrameSystemDispatchEventInfo,
  FrameSystemError,
  FrameSystemEvent,
  FrameSystemEventRecord,
  FrameSystemExtensionsCheckGenesis,
  FrameSystemExtensionsCheckNonZeroSender,
  FrameSystemExtensionsCheckNonce,
  FrameSystemExtensionsCheckSpecVersion,
  FrameSystemExtensionsCheckTxVersion,
  FrameSystemExtensionsCheckWeight,
  FrameSystemLastRuntimeUpgradeInfo,
  FrameSystemLimitsBlockLength,
  FrameSystemLimitsBlockWeights,
  FrameSystemLimitsWeightsPerClass,
  FrameSystemPhase,
  PalletBalancesAccountData,
  PalletBalancesAdjustmentDirection,
  PalletBalancesBalanceLock,
  PalletBalancesCall,
  PalletBalancesError,
  PalletBalancesEvent,
  PalletBalancesReasons,
  PalletBalancesReserveData,
  PalletBucketNftsCall,
  PalletBucketNftsError,
  PalletBucketNftsEvent,
  PalletCollatorSelectionCall,
  PalletCollatorSelectionCandidateInfo,
  PalletCollatorSelectionError,
  PalletCollatorSelectionEvent,
  PalletFileSystemBucketMoveRequestResponse,
  PalletFileSystemCall,
  PalletFileSystemError,
  PalletFileSystemEvent,
  PalletFileSystemFileDeletionRequest,
  PalletFileSystemFileKeyWithProof,
  PalletFileSystemFileOperation,
  PalletFileSystemFileOperationIntention,
  PalletFileSystemHoldReason,
  PalletFileSystemIncompleteStorageRequestMetadata,
  PalletFileSystemMoveBucketRequestMetadata,
  PalletFileSystemMspStorageRequestStatus,
  PalletFileSystemPendingFileDeletionRequest,
  PalletFileSystemPendingStopStoringRequest,
  PalletFileSystemRejectedStorageRequest,
  PalletFileSystemRejectedStorageRequestReason,
  PalletFileSystemReplicationTarget,
  PalletFileSystemStorageRequestBspsMetadata,
  PalletFileSystemStorageRequestMetadata,
  PalletFileSystemStorageRequestMspAcceptedFileKeys,
  PalletFileSystemStorageRequestMspBucketResponse,
  PalletMessageQueueBookState,
  PalletMessageQueueCall,
  PalletMessageQueueError,
  PalletMessageQueueEvent,
  PalletMessageQueueNeighbours,
  PalletMessageQueuePage,
  PalletNftsAttributeDeposit,
  PalletNftsAttributeNamespace,
  PalletNftsCall,
  PalletNftsCancelAttributesApprovalWitness,
  PalletNftsCollectionConfig,
  PalletNftsCollectionDetails,
  PalletNftsCollectionMetadata,
  PalletNftsCollectionRole,
  PalletNftsCollectionSetting,
  PalletNftsDestroyWitness,
  PalletNftsError,
  PalletNftsEvent,
  PalletNftsItemConfig,
  PalletNftsItemDeposit,
  PalletNftsItemDetails,
  PalletNftsItemMetadata,
  PalletNftsItemMetadataDeposit,
  PalletNftsItemSetting,
  PalletNftsItemTip,
  PalletNftsMintSettings,
  PalletNftsMintType,
  PalletNftsMintWitness,
  PalletNftsPalletAttributes,
  PalletNftsPalletFeature,
  PalletNftsPendingSwap,
  PalletNftsPreSignedAttributes,
  PalletNftsPreSignedMint,
  PalletNftsPriceDirection,
  PalletNftsPriceWithDirection,
  PalletParametersCall,
  PalletParametersEvent,
  PalletPaymentStreamsCall,
  PalletPaymentStreamsDynamicRatePaymentStream,
  PalletPaymentStreamsError,
  PalletPaymentStreamsEvent,
  PalletPaymentStreamsFixedRatePaymentStream,
  PalletPaymentStreamsHoldReason,
  PalletPaymentStreamsProviderLastChargeableInfo,
  PalletProofsDealerCall,
  PalletProofsDealerCustomChallenge,
  PalletProofsDealerError,
  PalletProofsDealerEvent,
  PalletProofsDealerKeyProof,
  PalletProofsDealerProof,
  PalletProofsDealerProofSubmissionRecord,
  PalletRandomnessCall,
  PalletRandomnessEvent,
  PalletSessionCall,
  PalletSessionError,
  PalletSessionEvent,
  PalletStorageProvidersBackupStorageProvider,
  PalletStorageProvidersBucket,
  PalletStorageProvidersCall,
  PalletStorageProvidersError,
  PalletStorageProvidersEvent,
  PalletStorageProvidersHoldReason,
  PalletStorageProvidersMainStorageProvider,
  PalletStorageProvidersMainStorageProviderSignUpRequest,
  PalletStorageProvidersSignUpRequest,
  PalletStorageProvidersSignUpRequestSpParams,
  PalletStorageProvidersStorageProviderId,
  PalletStorageProvidersTopUpMetadata,
  PalletStorageProvidersValueProposition,
  PalletStorageProvidersValuePropositionWithId,
  PalletSudoCall,
  PalletSudoError,
  PalletSudoEvent,
  PalletTimestampCall,
  PalletTransactionPaymentChargeTransactionPayment,
  PalletTransactionPaymentEvent,
  PalletTransactionPaymentReleases,
  PalletXcmCall,
  PalletXcmError,
  PalletXcmEvent,
  PalletXcmQueryStatus,
  PalletXcmRemoteLockedFungibleRecord,
  PalletXcmVersionMigrationStage,
  PolkadotCorePrimitivesInboundDownwardMessage,
  PolkadotCorePrimitivesInboundHrmpMessage,
  PolkadotCorePrimitivesOutboundHrmpMessage,
  PolkadotPrimitivesV8AbridgedHostConfiguration,
  PolkadotPrimitivesV8AbridgedHrmpChannel,
  PolkadotPrimitivesV8AsyncBackingAsyncBackingParams,
  PolkadotPrimitivesV8PersistedValidationData,
  PolkadotPrimitivesV8UpgradeGoAhead,
  PolkadotPrimitivesV8UpgradeRestriction,
  ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigBasicReplicationTarget,
  ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigBspStopStoringFilePenalty,
  ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigCheckpointChallengePeriod,
  ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigDecayRate,
  ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigHighSecurityReplicationTarget,
  ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigIdealUtilisationRate,
  ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigLowerExponentFactor,
  ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMaxPrice,
  ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMaxReplicationTarget,
  ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMaximumTreasuryCut,
  ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMinChallengePeriod,
  ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMinPrice,
  ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMinSeedPeriod,
  ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMinWaitForStopStoring,
  ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMinimumTreasuryCut,
  ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMostlyStablePrice,
  ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigParameters,
  ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigParametersKey,
  ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigParametersValue,
  ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigProviderTopUpTtl,
  ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigSlashAmountPerMaxFileSize,
  ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigStakeToChallengePeriod,
  ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigStakeToSeedPeriod,
  ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigStandardReplicationTarget,
  ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigStorageRequestTtl,
  ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigSuperHighSecurityReplicationTarget,
  ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigSystemUtilisationLowerThresholdPercentage,
  ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigSystemUtilisationUpperThresholdPercentage,
  ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigTickRangeToMaximumThreshold,
  ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigUltraHighSecurityReplicationTarget,
  ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigUpfrontTicksToPay,
  ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigUpperExponentFactor,
  ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigZeroSizeBucketFixedRate,
  ShParachainRuntimeConfigsRuntimeParamsRuntimeParameters,
  ShParachainRuntimeConfigsRuntimeParamsRuntimeParametersKey,
  ShParachainRuntimeConfigsRuntimeParamsRuntimeParametersValue,
  ShParachainRuntimeRuntime,
  ShParachainRuntimeRuntimeHoldReason,
  ShParachainRuntimeSessionKeys,
  ShpFileKeyVerifierFileKeyProof,
  ShpFileMetadataFileMetadata,
  ShpFileMetadataFingerprint,
  ShpTraitsTrieAddMutation,
  ShpTraitsTrieMutation,
  ShpTraitsTrieRemoveMutation,
  SpArithmeticArithmeticError,
  SpConsensusAuraSr25519AppSr25519Public,
  SpCoreCryptoKeyTypeId,
  SpRuntimeDigest,
  SpRuntimeDigestDigestItem,
  SpRuntimeDispatchError,
  SpRuntimeModuleError,
  SpRuntimeMultiSignature,
  SpRuntimeProvingTrieTrieError,
  SpRuntimeTokenError,
  SpRuntimeTransactionalError,
  SpTrieStorageProof,
  SpTrieStorageProofCompactProof,
  SpVersionRuntimeVersion,
  SpWeightsRuntimeDbWeight,
  SpWeightsWeightV2Weight,
  StagingParachainInfoCall,
  StagingXcmExecutorAssetTransferTransferType,
  StagingXcmV3MultiLocation,
  StagingXcmV4Asset,
  StagingXcmV4AssetAssetFilter,
  StagingXcmV4AssetAssetId,
  StagingXcmV4AssetAssetInstance,
  StagingXcmV4AssetAssets,
  StagingXcmV4AssetFungibility,
  StagingXcmV4AssetWildAsset,
  StagingXcmV4AssetWildFungibility,
  StagingXcmV4Instruction,
  StagingXcmV4Junction,
  StagingXcmV4JunctionNetworkId,
  StagingXcmV4Junctions,
  StagingXcmV4Location,
  StagingXcmV4PalletInfo,
  StagingXcmV4QueryResponseInfo,
  StagingXcmV4Response,
  StagingXcmV4Xcm,
  StagingXcmV5Asset,
  StagingXcmV5AssetAssetFilter,
  StagingXcmV5AssetAssetId,
  StagingXcmV5AssetAssetInstance,
  StagingXcmV5AssetAssetTransferFilter,
  StagingXcmV5AssetAssets,
  StagingXcmV5AssetFungibility,
  StagingXcmV5AssetWildAsset,
  StagingXcmV5AssetWildFungibility,
  StagingXcmV5Hint,
  StagingXcmV5Instruction,
  StagingXcmV5Junction,
  StagingXcmV5JunctionNetworkId,
  StagingXcmV5Junctions,
  StagingXcmV5Location,
  StagingXcmV5PalletInfo,
  StagingXcmV5QueryResponseInfo,
  StagingXcmV5Response,
  StagingXcmV5TraitsOutcome,
  StagingXcmV5Xcm,
  XcmDoubleEncoded,
  XcmV3Instruction,
  XcmV3Junction,
  XcmV3JunctionBodyId,
  XcmV3JunctionBodyPart,
  XcmV3JunctionNetworkId,
  XcmV3Junctions,
  XcmV3MaybeErrorCode,
  XcmV3MultiAsset,
  XcmV3MultiassetAssetId,
  XcmV3MultiassetAssetInstance,
  XcmV3MultiassetFungibility,
  XcmV3MultiassetMultiAssetFilter,
  XcmV3MultiassetMultiAssets,
  XcmV3MultiassetWildFungibility,
  XcmV3MultiassetWildMultiAsset,
  XcmV3OriginKind,
  XcmV3PalletInfo,
  XcmV3QueryResponseInfo,
  XcmV3Response,
  XcmV3TraitsError,
  XcmV3WeightLimit,
  XcmV3Xcm,
  XcmV5TraitsError,
  XcmVersionedAssetId,
  XcmVersionedAssets,
  XcmVersionedLocation,
  XcmVersionedResponse,
  XcmVersionedXcm
} from "@polkadot/types/lookup";

declare module "@polkadot/types/types/registry" {
  interface InterfaceTypes {
    CumulusPalletParachainSystemCall: CumulusPalletParachainSystemCall;
    CumulusPalletParachainSystemError: CumulusPalletParachainSystemError;
    CumulusPalletParachainSystemEvent: CumulusPalletParachainSystemEvent;
    CumulusPalletParachainSystemRelayStateSnapshotMessagingStateSnapshot: CumulusPalletParachainSystemRelayStateSnapshotMessagingStateSnapshot;
    CumulusPalletParachainSystemRelayStateSnapshotRelayDispatchQueueRemainingCapacity: CumulusPalletParachainSystemRelayStateSnapshotRelayDispatchQueueRemainingCapacity;
    CumulusPalletParachainSystemUnincludedSegmentAncestor: CumulusPalletParachainSystemUnincludedSegmentAncestor;
    CumulusPalletParachainSystemUnincludedSegmentHrmpChannelUpdate: CumulusPalletParachainSystemUnincludedSegmentHrmpChannelUpdate;
    CumulusPalletParachainSystemUnincludedSegmentSegmentTracker: CumulusPalletParachainSystemUnincludedSegmentSegmentTracker;
    CumulusPalletParachainSystemUnincludedSegmentUsedBandwidth: CumulusPalletParachainSystemUnincludedSegmentUsedBandwidth;
    CumulusPalletXcmCall: CumulusPalletXcmCall;
    CumulusPalletXcmEvent: CumulusPalletXcmEvent;
    CumulusPalletXcmpQueueCall: CumulusPalletXcmpQueueCall;
    CumulusPalletXcmpQueueError: CumulusPalletXcmpQueueError;
    CumulusPalletXcmpQueueEvent: CumulusPalletXcmpQueueEvent;
    CumulusPalletXcmpQueueOutboundChannelDetails: CumulusPalletXcmpQueueOutboundChannelDetails;
    CumulusPalletXcmpQueueOutboundState: CumulusPalletXcmpQueueOutboundState;
    CumulusPalletXcmpQueueQueueConfigData: CumulusPalletXcmpQueueQueueConfigData;
    CumulusPrimitivesCoreAggregateMessageOrigin: CumulusPrimitivesCoreAggregateMessageOrigin;
    CumulusPrimitivesParachainInherentParachainInherentData: CumulusPrimitivesParachainInherentParachainInherentData;
    CumulusPrimitivesStorageWeightReclaimStorageWeightReclaim: CumulusPrimitivesStorageWeightReclaimStorageWeightReclaim;
    FrameMetadataHashExtensionCheckMetadataHash: FrameMetadataHashExtensionCheckMetadataHash;
    FrameMetadataHashExtensionMode: FrameMetadataHashExtensionMode;
    FrameSupportDispatchDispatchClass: FrameSupportDispatchDispatchClass;
    FrameSupportDispatchPays: FrameSupportDispatchPays;
    FrameSupportDispatchPerDispatchClassU32: FrameSupportDispatchPerDispatchClassU32;
    FrameSupportDispatchPerDispatchClassWeight: FrameSupportDispatchPerDispatchClassWeight;
    FrameSupportDispatchPerDispatchClassWeightsPerClass: FrameSupportDispatchPerDispatchClassWeightsPerClass;
    FrameSupportMessagesProcessMessageError: FrameSupportMessagesProcessMessageError;
    FrameSupportTokensMiscBalanceStatus: FrameSupportTokensMiscBalanceStatus;
    FrameSupportTokensMiscIdAmount: FrameSupportTokensMiscIdAmount;
    FrameSystemAccountInfo: FrameSystemAccountInfo;
    FrameSystemCall: FrameSystemCall;
    FrameSystemCodeUpgradeAuthorization: FrameSystemCodeUpgradeAuthorization;
    FrameSystemDispatchEventInfo: FrameSystemDispatchEventInfo;
    FrameSystemError: FrameSystemError;
    FrameSystemEvent: FrameSystemEvent;
    FrameSystemEventRecord: FrameSystemEventRecord;
    FrameSystemExtensionsCheckGenesis: FrameSystemExtensionsCheckGenesis;
    FrameSystemExtensionsCheckNonZeroSender: FrameSystemExtensionsCheckNonZeroSender;
    FrameSystemExtensionsCheckNonce: FrameSystemExtensionsCheckNonce;
    FrameSystemExtensionsCheckSpecVersion: FrameSystemExtensionsCheckSpecVersion;
    FrameSystemExtensionsCheckTxVersion: FrameSystemExtensionsCheckTxVersion;
    FrameSystemExtensionsCheckWeight: FrameSystemExtensionsCheckWeight;
    FrameSystemLastRuntimeUpgradeInfo: FrameSystemLastRuntimeUpgradeInfo;
    FrameSystemLimitsBlockLength: FrameSystemLimitsBlockLength;
    FrameSystemLimitsBlockWeights: FrameSystemLimitsBlockWeights;
    FrameSystemLimitsWeightsPerClass: FrameSystemLimitsWeightsPerClass;
    FrameSystemPhase: FrameSystemPhase;
    PalletBalancesAccountData: PalletBalancesAccountData;
    PalletBalancesAdjustmentDirection: PalletBalancesAdjustmentDirection;
    PalletBalancesBalanceLock: PalletBalancesBalanceLock;
    PalletBalancesCall: PalletBalancesCall;
    PalletBalancesError: PalletBalancesError;
    PalletBalancesEvent: PalletBalancesEvent;
    PalletBalancesReasons: PalletBalancesReasons;
    PalletBalancesReserveData: PalletBalancesReserveData;
    PalletBucketNftsCall: PalletBucketNftsCall;
    PalletBucketNftsError: PalletBucketNftsError;
    PalletBucketNftsEvent: PalletBucketNftsEvent;
    PalletCollatorSelectionCall: PalletCollatorSelectionCall;
    PalletCollatorSelectionCandidateInfo: PalletCollatorSelectionCandidateInfo;
    PalletCollatorSelectionError: PalletCollatorSelectionError;
    PalletCollatorSelectionEvent: PalletCollatorSelectionEvent;
    PalletFileSystemBucketMoveRequestResponse: PalletFileSystemBucketMoveRequestResponse;
    PalletFileSystemCall: PalletFileSystemCall;
    PalletFileSystemError: PalletFileSystemError;
    PalletFileSystemEvent: PalletFileSystemEvent;
    PalletFileSystemFileDeletionRequest: PalletFileSystemFileDeletionRequest;
    PalletFileSystemFileKeyWithProof: PalletFileSystemFileKeyWithProof;
    PalletFileSystemFileOperation: PalletFileSystemFileOperation;
    PalletFileSystemFileOperationIntention: PalletFileSystemFileOperationIntention;
    PalletFileSystemHoldReason: PalletFileSystemHoldReason;
    PalletFileSystemIncompleteStorageRequestMetadata: PalletFileSystemIncompleteStorageRequestMetadata;
    PalletFileSystemMoveBucketRequestMetadata: PalletFileSystemMoveBucketRequestMetadata;
    PalletFileSystemMspStorageRequestStatus: PalletFileSystemMspStorageRequestStatus;
    PalletFileSystemPendingFileDeletionRequest: PalletFileSystemPendingFileDeletionRequest;
    PalletFileSystemPendingStopStoringRequest: PalletFileSystemPendingStopStoringRequest;
    PalletFileSystemRejectedStorageRequest: PalletFileSystemRejectedStorageRequest;
    PalletFileSystemRejectedStorageRequestReason: PalletFileSystemRejectedStorageRequestReason;
    PalletFileSystemReplicationTarget: PalletFileSystemReplicationTarget;
    PalletFileSystemStorageRequestBspsMetadata: PalletFileSystemStorageRequestBspsMetadata;
    PalletFileSystemStorageRequestMetadata: PalletFileSystemStorageRequestMetadata;
    PalletFileSystemStorageRequestMspAcceptedFileKeys: PalletFileSystemStorageRequestMspAcceptedFileKeys;
    PalletFileSystemStorageRequestMspBucketResponse: PalletFileSystemStorageRequestMspBucketResponse;
    PalletMessageQueueBookState: PalletMessageQueueBookState;
    PalletMessageQueueCall: PalletMessageQueueCall;
    PalletMessageQueueError: PalletMessageQueueError;
    PalletMessageQueueEvent: PalletMessageQueueEvent;
    PalletMessageQueueNeighbours: PalletMessageQueueNeighbours;
    PalletMessageQueuePage: PalletMessageQueuePage;
    PalletNftsAttributeDeposit: PalletNftsAttributeDeposit;
    PalletNftsAttributeNamespace: PalletNftsAttributeNamespace;
    PalletNftsCall: PalletNftsCall;
    PalletNftsCancelAttributesApprovalWitness: PalletNftsCancelAttributesApprovalWitness;
    PalletNftsCollectionConfig: PalletNftsCollectionConfig;
    PalletNftsCollectionDetails: PalletNftsCollectionDetails;
    PalletNftsCollectionMetadata: PalletNftsCollectionMetadata;
    PalletNftsCollectionRole: PalletNftsCollectionRole;
    PalletNftsCollectionSetting: PalletNftsCollectionSetting;
    PalletNftsDestroyWitness: PalletNftsDestroyWitness;
    PalletNftsError: PalletNftsError;
    PalletNftsEvent: PalletNftsEvent;
    PalletNftsItemConfig: PalletNftsItemConfig;
    PalletNftsItemDeposit: PalletNftsItemDeposit;
    PalletNftsItemDetails: PalletNftsItemDetails;
    PalletNftsItemMetadata: PalletNftsItemMetadata;
    PalletNftsItemMetadataDeposit: PalletNftsItemMetadataDeposit;
    PalletNftsItemSetting: PalletNftsItemSetting;
    PalletNftsItemTip: PalletNftsItemTip;
    PalletNftsMintSettings: PalletNftsMintSettings;
    PalletNftsMintType: PalletNftsMintType;
    PalletNftsMintWitness: PalletNftsMintWitness;
    PalletNftsPalletAttributes: PalletNftsPalletAttributes;
    PalletNftsPalletFeature: PalletNftsPalletFeature;
    PalletNftsPendingSwap: PalletNftsPendingSwap;
    PalletNftsPreSignedAttributes: PalletNftsPreSignedAttributes;
    PalletNftsPreSignedMint: PalletNftsPreSignedMint;
    PalletNftsPriceDirection: PalletNftsPriceDirection;
    PalletNftsPriceWithDirection: PalletNftsPriceWithDirection;
    PalletParametersCall: PalletParametersCall;
    PalletParametersEvent: PalletParametersEvent;
    PalletPaymentStreamsCall: PalletPaymentStreamsCall;
    PalletPaymentStreamsDynamicRatePaymentStream: PalletPaymentStreamsDynamicRatePaymentStream;
    PalletPaymentStreamsError: PalletPaymentStreamsError;
    PalletPaymentStreamsEvent: PalletPaymentStreamsEvent;
    PalletPaymentStreamsFixedRatePaymentStream: PalletPaymentStreamsFixedRatePaymentStream;
    PalletPaymentStreamsHoldReason: PalletPaymentStreamsHoldReason;
    PalletPaymentStreamsProviderLastChargeableInfo: PalletPaymentStreamsProviderLastChargeableInfo;
    PalletProofsDealerCall: PalletProofsDealerCall;
    PalletProofsDealerCustomChallenge: PalletProofsDealerCustomChallenge;
    PalletProofsDealerError: PalletProofsDealerError;
    PalletProofsDealerEvent: PalletProofsDealerEvent;
    PalletProofsDealerKeyProof: PalletProofsDealerKeyProof;
    PalletProofsDealerProof: PalletProofsDealerProof;
    PalletProofsDealerProofSubmissionRecord: PalletProofsDealerProofSubmissionRecord;
    PalletRandomnessCall: PalletRandomnessCall;
    PalletRandomnessEvent: PalletRandomnessEvent;
    PalletSessionCall: PalletSessionCall;
    PalletSessionError: PalletSessionError;
    PalletSessionEvent: PalletSessionEvent;
    PalletStorageProvidersBackupStorageProvider: PalletStorageProvidersBackupStorageProvider;
    PalletStorageProvidersBucket: PalletStorageProvidersBucket;
    PalletStorageProvidersCall: PalletStorageProvidersCall;
    PalletStorageProvidersError: PalletStorageProvidersError;
    PalletStorageProvidersEvent: PalletStorageProvidersEvent;
    PalletStorageProvidersHoldReason: PalletStorageProvidersHoldReason;
    PalletStorageProvidersMainStorageProvider: PalletStorageProvidersMainStorageProvider;
    PalletStorageProvidersMainStorageProviderSignUpRequest: PalletStorageProvidersMainStorageProviderSignUpRequest;
    PalletStorageProvidersSignUpRequest: PalletStorageProvidersSignUpRequest;
    PalletStorageProvidersSignUpRequestSpParams: PalletStorageProvidersSignUpRequestSpParams;
    PalletStorageProvidersStorageProviderId: PalletStorageProvidersStorageProviderId;
    PalletStorageProvidersTopUpMetadata: PalletStorageProvidersTopUpMetadata;
    PalletStorageProvidersValueProposition: PalletStorageProvidersValueProposition;
    PalletStorageProvidersValuePropositionWithId: PalletStorageProvidersValuePropositionWithId;
    PalletSudoCall: PalletSudoCall;
    PalletSudoError: PalletSudoError;
    PalletSudoEvent: PalletSudoEvent;
    PalletTimestampCall: PalletTimestampCall;
    PalletTransactionPaymentChargeTransactionPayment: PalletTransactionPaymentChargeTransactionPayment;
    PalletTransactionPaymentEvent: PalletTransactionPaymentEvent;
    PalletTransactionPaymentReleases: PalletTransactionPaymentReleases;
    PalletXcmCall: PalletXcmCall;
    PalletXcmError: PalletXcmError;
    PalletXcmEvent: PalletXcmEvent;
    PalletXcmQueryStatus: PalletXcmQueryStatus;
    PalletXcmRemoteLockedFungibleRecord: PalletXcmRemoteLockedFungibleRecord;
    PalletXcmVersionMigrationStage: PalletXcmVersionMigrationStage;
    PolkadotCorePrimitivesInboundDownwardMessage: PolkadotCorePrimitivesInboundDownwardMessage;
    PolkadotCorePrimitivesInboundHrmpMessage: PolkadotCorePrimitivesInboundHrmpMessage;
    PolkadotCorePrimitivesOutboundHrmpMessage: PolkadotCorePrimitivesOutboundHrmpMessage;
    PolkadotPrimitivesV8AbridgedHostConfiguration: PolkadotPrimitivesV8AbridgedHostConfiguration;
    PolkadotPrimitivesV8AbridgedHrmpChannel: PolkadotPrimitivesV8AbridgedHrmpChannel;
    PolkadotPrimitivesV8AsyncBackingAsyncBackingParams: PolkadotPrimitivesV8AsyncBackingAsyncBackingParams;
    PolkadotPrimitivesV8PersistedValidationData: PolkadotPrimitivesV8PersistedValidationData;
    PolkadotPrimitivesV8UpgradeGoAhead: PolkadotPrimitivesV8UpgradeGoAhead;
    PolkadotPrimitivesV8UpgradeRestriction: PolkadotPrimitivesV8UpgradeRestriction;
    ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigBasicReplicationTarget: ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigBasicReplicationTarget;
    ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigBspStopStoringFilePenalty: ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigBspStopStoringFilePenalty;
    ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigCheckpointChallengePeriod: ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigCheckpointChallengePeriod;
    ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigDecayRate: ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigDecayRate;
    ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigHighSecurityReplicationTarget: ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigHighSecurityReplicationTarget;
    ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigIdealUtilisationRate: ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigIdealUtilisationRate;
    ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigLowerExponentFactor: ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigLowerExponentFactor;
    ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMaxPrice: ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMaxPrice;
    ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMaxReplicationTarget: ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMaxReplicationTarget;
    ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMaximumTreasuryCut: ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMaximumTreasuryCut;
    ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMinChallengePeriod: ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMinChallengePeriod;
    ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMinPrice: ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMinPrice;
    ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMinSeedPeriod: ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMinSeedPeriod;
    ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMinWaitForStopStoring: ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMinWaitForStopStoring;
    ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMinimumTreasuryCut: ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMinimumTreasuryCut;
    ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMostlyStablePrice: ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMostlyStablePrice;
    ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigParameters: ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigParameters;
    ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigParametersKey: ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigParametersKey;
    ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigParametersValue: ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigParametersValue;
    ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigProviderTopUpTtl: ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigProviderTopUpTtl;
    ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigSlashAmountPerMaxFileSize: ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigSlashAmountPerMaxFileSize;
    ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigStakeToChallengePeriod: ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigStakeToChallengePeriod;
    ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigStakeToSeedPeriod: ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigStakeToSeedPeriod;
    ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigStandardReplicationTarget: ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigStandardReplicationTarget;
    ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigStorageRequestTtl: ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigStorageRequestTtl;
    ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigSuperHighSecurityReplicationTarget: ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigSuperHighSecurityReplicationTarget;
    ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigSystemUtilisationLowerThresholdPercentage: ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigSystemUtilisationLowerThresholdPercentage;
    ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigSystemUtilisationUpperThresholdPercentage: ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigSystemUtilisationUpperThresholdPercentage;
    ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigTickRangeToMaximumThreshold: ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigTickRangeToMaximumThreshold;
    ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigUltraHighSecurityReplicationTarget: ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigUltraHighSecurityReplicationTarget;
    ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigUpfrontTicksToPay: ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigUpfrontTicksToPay;
    ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigUpperExponentFactor: ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigUpperExponentFactor;
    ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigZeroSizeBucketFixedRate: ShParachainRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigZeroSizeBucketFixedRate;
    ShParachainRuntimeConfigsRuntimeParamsRuntimeParameters: ShParachainRuntimeConfigsRuntimeParamsRuntimeParameters;
    ShParachainRuntimeConfigsRuntimeParamsRuntimeParametersKey: ShParachainRuntimeConfigsRuntimeParamsRuntimeParametersKey;
    ShParachainRuntimeConfigsRuntimeParamsRuntimeParametersValue: ShParachainRuntimeConfigsRuntimeParamsRuntimeParametersValue;
    ShParachainRuntimeRuntime: ShParachainRuntimeRuntime;
    ShParachainRuntimeRuntimeHoldReason: ShParachainRuntimeRuntimeHoldReason;
    ShParachainRuntimeSessionKeys: ShParachainRuntimeSessionKeys;
    ShpFileKeyVerifierFileKeyProof: ShpFileKeyVerifierFileKeyProof;
    ShpFileMetadataFileMetadata: ShpFileMetadataFileMetadata;
    ShpFileMetadataFingerprint: ShpFileMetadataFingerprint;
    ShpTraitsTrieAddMutation: ShpTraitsTrieAddMutation;
    ShpTraitsTrieMutation: ShpTraitsTrieMutation;
    ShpTraitsTrieRemoveMutation: ShpTraitsTrieRemoveMutation;
    SpArithmeticArithmeticError: SpArithmeticArithmeticError;
    SpConsensusAuraSr25519AppSr25519Public: SpConsensusAuraSr25519AppSr25519Public;
    SpCoreCryptoKeyTypeId: SpCoreCryptoKeyTypeId;
    SpRuntimeDigest: SpRuntimeDigest;
    SpRuntimeDigestDigestItem: SpRuntimeDigestDigestItem;
    SpRuntimeDispatchError: SpRuntimeDispatchError;
    SpRuntimeModuleError: SpRuntimeModuleError;
    SpRuntimeMultiSignature: SpRuntimeMultiSignature;
    SpRuntimeProvingTrieTrieError: SpRuntimeProvingTrieTrieError;
    SpRuntimeTokenError: SpRuntimeTokenError;
    SpRuntimeTransactionalError: SpRuntimeTransactionalError;
    SpTrieStorageProof: SpTrieStorageProof;
    SpTrieStorageProofCompactProof: SpTrieStorageProofCompactProof;
    SpVersionRuntimeVersion: SpVersionRuntimeVersion;
    SpWeightsRuntimeDbWeight: SpWeightsRuntimeDbWeight;
    SpWeightsWeightV2Weight: SpWeightsWeightV2Weight;
    StagingParachainInfoCall: StagingParachainInfoCall;
    StagingXcmExecutorAssetTransferTransferType: StagingXcmExecutorAssetTransferTransferType;
    StagingXcmV3MultiLocation: StagingXcmV3MultiLocation;
    StagingXcmV4Asset: StagingXcmV4Asset;
    StagingXcmV4AssetAssetFilter: StagingXcmV4AssetAssetFilter;
    StagingXcmV4AssetAssetId: StagingXcmV4AssetAssetId;
    StagingXcmV4AssetAssetInstance: StagingXcmV4AssetAssetInstance;
    StagingXcmV4AssetAssets: StagingXcmV4AssetAssets;
    StagingXcmV4AssetFungibility: StagingXcmV4AssetFungibility;
    StagingXcmV4AssetWildAsset: StagingXcmV4AssetWildAsset;
    StagingXcmV4AssetWildFungibility: StagingXcmV4AssetWildFungibility;
    StagingXcmV4Instruction: StagingXcmV4Instruction;
    StagingXcmV4Junction: StagingXcmV4Junction;
    StagingXcmV4JunctionNetworkId: StagingXcmV4JunctionNetworkId;
    StagingXcmV4Junctions: StagingXcmV4Junctions;
    StagingXcmV4Location: StagingXcmV4Location;
    StagingXcmV4PalletInfo: StagingXcmV4PalletInfo;
    StagingXcmV4QueryResponseInfo: StagingXcmV4QueryResponseInfo;
    StagingXcmV4Response: StagingXcmV4Response;
    StagingXcmV4Xcm: StagingXcmV4Xcm;
    StagingXcmV5Asset: StagingXcmV5Asset;
    StagingXcmV5AssetAssetFilter: StagingXcmV5AssetAssetFilter;
    StagingXcmV5AssetAssetId: StagingXcmV5AssetAssetId;
    StagingXcmV5AssetAssetInstance: StagingXcmV5AssetAssetInstance;
    StagingXcmV5AssetAssetTransferFilter: StagingXcmV5AssetAssetTransferFilter;
    StagingXcmV5AssetAssets: StagingXcmV5AssetAssets;
    StagingXcmV5AssetFungibility: StagingXcmV5AssetFungibility;
    StagingXcmV5AssetWildAsset: StagingXcmV5AssetWildAsset;
    StagingXcmV5AssetWildFungibility: StagingXcmV5AssetWildFungibility;
    StagingXcmV5Hint: StagingXcmV5Hint;
    StagingXcmV5Instruction: StagingXcmV5Instruction;
    StagingXcmV5Junction: StagingXcmV5Junction;
    StagingXcmV5JunctionNetworkId: StagingXcmV5JunctionNetworkId;
    StagingXcmV5Junctions: StagingXcmV5Junctions;
    StagingXcmV5Location: StagingXcmV5Location;
    StagingXcmV5PalletInfo: StagingXcmV5PalletInfo;
    StagingXcmV5QueryResponseInfo: StagingXcmV5QueryResponseInfo;
    StagingXcmV5Response: StagingXcmV5Response;
    StagingXcmV5TraitsOutcome: StagingXcmV5TraitsOutcome;
    StagingXcmV5Xcm: StagingXcmV5Xcm;
    XcmDoubleEncoded: XcmDoubleEncoded;
    XcmV3Instruction: XcmV3Instruction;
    XcmV3Junction: XcmV3Junction;
    XcmV3JunctionBodyId: XcmV3JunctionBodyId;
    XcmV3JunctionBodyPart: XcmV3JunctionBodyPart;
    XcmV3JunctionNetworkId: XcmV3JunctionNetworkId;
    XcmV3Junctions: XcmV3Junctions;
    XcmV3MaybeErrorCode: XcmV3MaybeErrorCode;
    XcmV3MultiAsset: XcmV3MultiAsset;
    XcmV3MultiassetAssetId: XcmV3MultiassetAssetId;
    XcmV3MultiassetAssetInstance: XcmV3MultiassetAssetInstance;
    XcmV3MultiassetFungibility: XcmV3MultiassetFungibility;
    XcmV3MultiassetMultiAssetFilter: XcmV3MultiassetMultiAssetFilter;
    XcmV3MultiassetMultiAssets: XcmV3MultiassetMultiAssets;
    XcmV3MultiassetWildFungibility: XcmV3MultiassetWildFungibility;
    XcmV3MultiassetWildMultiAsset: XcmV3MultiassetWildMultiAsset;
    XcmV3OriginKind: XcmV3OriginKind;
    XcmV3PalletInfo: XcmV3PalletInfo;
    XcmV3QueryResponseInfo: XcmV3QueryResponseInfo;
    XcmV3Response: XcmV3Response;
    XcmV3TraitsError: XcmV3TraitsError;
    XcmV3WeightLimit: XcmV3WeightLimit;
    XcmV3Xcm: XcmV3Xcm;
    XcmV5TraitsError: XcmV5TraitsError;
    XcmVersionedAssetId: XcmVersionedAssetId;
    XcmVersionedAssets: XcmVersionedAssets;
    XcmVersionedLocation: XcmVersionedLocation;
    XcmVersionedResponse: XcmVersionedResponse;
    XcmVersionedXcm: XcmVersionedXcm;
  } // InterfaceTypes
} // declare module
