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
  FrameSupportDispatchDispatchInfo,
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
  PalletFileSystemAcceptedStorageRequestParameters,
  PalletFileSystemBatchResponses,
  PalletFileSystemBucketMoveRequestResponse,
  PalletFileSystemCall,
  PalletFileSystemEitherAccountIdOrMspId,
  PalletFileSystemError,
  PalletFileSystemEvent,
  PalletFileSystemHoldReason,
  PalletFileSystemMoveBucketRequestMetadata,
  PalletFileSystemMspAcceptedBatchStorageRequests,
  PalletFileSystemMspFailedBatchStorageRequests,
  PalletFileSystemMspRejectedBatchStorageRequests,
  PalletFileSystemMspRespondStorageRequestsResult,
  PalletFileSystemMspStorageRequestResponse,
  PalletFileSystemRejectedStorageRequestReason,
  PalletFileSystemStorageRequestBspsMetadata,
  PalletFileSystemStorageRequestMetadata,
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
  PalletProofsDealerError,
  PalletProofsDealerEvent,
  PalletProofsDealerKeyProof,
  PalletProofsDealerProof,
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
  ShpFileKeyVerifierFileKeyProof,
  ShpFileMetadataFileMetadata,
  ShpFileMetadataFingerprint,
  ShpTraitsTrieRemoveMutation,
  SpArithmeticArithmeticError,
  SpConsensusAuraSr25519AppSr25519Public,
  SpCoreCryptoKeyTypeId,
  SpRuntimeDigest,
  SpRuntimeDigestDigestItem,
  SpRuntimeDispatchError,
  SpRuntimeModuleError,
  SpRuntimeMultiSignature,
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
  StagingXcmV4TraitsOutcome,
  StagingXcmV4Xcm,
  StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigBspStopStoringFilePenalty,
  StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigCheckpointChallengePeriod,
  StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigDecayRate,
  StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigIdealUtilisationRate,
  StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigLowerExponentFactor,
  StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMaxPrice,
  StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMaximumTreasuryCut,
  StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMinChallengePeriod,
  StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMinPrice,
  StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMinimumTreasuryCut,
  StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMostlyStablePrice,
  StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigParameters,
  StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigParametersKey,
  StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigParametersValue,
  StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigSlashAmountPerMaxFileSize,
  StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigStakeToChallengePeriod,
  StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigSystemUtilisationLowerThresholdPercentage,
  StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigSystemUtilisationUpperThresholdPercentage,
  StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigUpperExponentFactor,
  StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigZeroSizeBucketFixedRate,
  StorageHubRuntimeConfigsRuntimeParamsRuntimeParameters,
  StorageHubRuntimeConfigsRuntimeParamsRuntimeParametersKey,
  StorageHubRuntimeConfigsRuntimeParamsRuntimeParametersValue,
  StorageHubRuntimeRuntime,
  StorageHubRuntimeRuntimeHoldReason,
  StorageHubRuntimeSessionKeys,
  XcmDoubleEncoded,
  XcmV2BodyId,
  XcmV2BodyPart,
  XcmV2Instruction,
  XcmV2Junction,
  XcmV2MultiAsset,
  XcmV2MultiLocation,
  XcmV2MultiassetAssetId,
  XcmV2MultiassetAssetInstance,
  XcmV2MultiassetFungibility,
  XcmV2MultiassetMultiAssetFilter,
  XcmV2MultiassetMultiAssets,
  XcmV2MultiassetWildFungibility,
  XcmV2MultiassetWildMultiAsset,
  XcmV2MultilocationJunctions,
  XcmV2NetworkId,
  XcmV2OriginKind,
  XcmV2Response,
  XcmV2TraitsError,
  XcmV2WeightLimit,
  XcmV2Xcm,
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
    FrameSupportDispatchDispatchInfo: FrameSupportDispatchDispatchInfo;
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
    PalletFileSystemAcceptedStorageRequestParameters: PalletFileSystemAcceptedStorageRequestParameters;
    PalletFileSystemBatchResponses: PalletFileSystemBatchResponses;
    PalletFileSystemBucketMoveRequestResponse: PalletFileSystemBucketMoveRequestResponse;
    PalletFileSystemCall: PalletFileSystemCall;
    PalletFileSystemEitherAccountIdOrMspId: PalletFileSystemEitherAccountIdOrMspId;
    PalletFileSystemError: PalletFileSystemError;
    PalletFileSystemEvent: PalletFileSystemEvent;
    PalletFileSystemHoldReason: PalletFileSystemHoldReason;
    PalletFileSystemMoveBucketRequestMetadata: PalletFileSystemMoveBucketRequestMetadata;
    PalletFileSystemMspAcceptedBatchStorageRequests: PalletFileSystemMspAcceptedBatchStorageRequests;
    PalletFileSystemMspFailedBatchStorageRequests: PalletFileSystemMspFailedBatchStorageRequests;
    PalletFileSystemMspRejectedBatchStorageRequests: PalletFileSystemMspRejectedBatchStorageRequests;
    PalletFileSystemMspRespondStorageRequestsResult: PalletFileSystemMspRespondStorageRequestsResult;
    PalletFileSystemMspStorageRequestResponse: PalletFileSystemMspStorageRequestResponse;
    PalletFileSystemRejectedStorageRequestReason: PalletFileSystemRejectedStorageRequestReason;
    PalletFileSystemStorageRequestBspsMetadata: PalletFileSystemStorageRequestBspsMetadata;
    PalletFileSystemStorageRequestMetadata: PalletFileSystemStorageRequestMetadata;
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
    PalletProofsDealerError: PalletProofsDealerError;
    PalletProofsDealerEvent: PalletProofsDealerEvent;
    PalletProofsDealerKeyProof: PalletProofsDealerKeyProof;
    PalletProofsDealerProof: PalletProofsDealerProof;
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
    ShpFileKeyVerifierFileKeyProof: ShpFileKeyVerifierFileKeyProof;
    ShpFileMetadataFileMetadata: ShpFileMetadataFileMetadata;
    ShpFileMetadataFingerprint: ShpFileMetadataFingerprint;
    ShpTraitsTrieRemoveMutation: ShpTraitsTrieRemoveMutation;
    SpArithmeticArithmeticError: SpArithmeticArithmeticError;
    SpConsensusAuraSr25519AppSr25519Public: SpConsensusAuraSr25519AppSr25519Public;
    SpCoreCryptoKeyTypeId: SpCoreCryptoKeyTypeId;
    SpRuntimeDigest: SpRuntimeDigest;
    SpRuntimeDigestDigestItem: SpRuntimeDigestDigestItem;
    SpRuntimeDispatchError: SpRuntimeDispatchError;
    SpRuntimeModuleError: SpRuntimeModuleError;
    SpRuntimeMultiSignature: SpRuntimeMultiSignature;
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
    StagingXcmV4TraitsOutcome: StagingXcmV4TraitsOutcome;
    StagingXcmV4Xcm: StagingXcmV4Xcm;
    StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigBspStopStoringFilePenalty: StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigBspStopStoringFilePenalty;
    StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigCheckpointChallengePeriod: StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigCheckpointChallengePeriod;
    StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigDecayRate: StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigDecayRate;
    StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigIdealUtilisationRate: StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigIdealUtilisationRate;
    StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigLowerExponentFactor: StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigLowerExponentFactor;
    StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMaxPrice: StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMaxPrice;
    StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMaximumTreasuryCut: StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMaximumTreasuryCut;
    StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMinChallengePeriod: StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMinChallengePeriod;
    StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMinPrice: StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMinPrice;
    StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMinimumTreasuryCut: StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMinimumTreasuryCut;
    StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMostlyStablePrice: StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMostlyStablePrice;
    StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigParameters: StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigParameters;
    StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigParametersKey: StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigParametersKey;
    StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigParametersValue: StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigParametersValue;
    StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigSlashAmountPerMaxFileSize: StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigSlashAmountPerMaxFileSize;
    StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigStakeToChallengePeriod: StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigStakeToChallengePeriod;
    StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigSystemUtilisationLowerThresholdPercentage: StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigSystemUtilisationLowerThresholdPercentage;
    StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigSystemUtilisationUpperThresholdPercentage: StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigSystemUtilisationUpperThresholdPercentage;
    StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigUpperExponentFactor: StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigUpperExponentFactor;
    StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigZeroSizeBucketFixedRate: StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigZeroSizeBucketFixedRate;
    StorageHubRuntimeConfigsRuntimeParamsRuntimeParameters: StorageHubRuntimeConfigsRuntimeParamsRuntimeParameters;
    StorageHubRuntimeConfigsRuntimeParamsRuntimeParametersKey: StorageHubRuntimeConfigsRuntimeParamsRuntimeParametersKey;
    StorageHubRuntimeConfigsRuntimeParamsRuntimeParametersValue: StorageHubRuntimeConfigsRuntimeParamsRuntimeParametersValue;
    StorageHubRuntimeRuntime: StorageHubRuntimeRuntime;
    StorageHubRuntimeRuntimeHoldReason: StorageHubRuntimeRuntimeHoldReason;
    StorageHubRuntimeSessionKeys: StorageHubRuntimeSessionKeys;
    XcmDoubleEncoded: XcmDoubleEncoded;
    XcmV2BodyId: XcmV2BodyId;
    XcmV2BodyPart: XcmV2BodyPart;
    XcmV2Instruction: XcmV2Instruction;
    XcmV2Junction: XcmV2Junction;
    XcmV2MultiAsset: XcmV2MultiAsset;
    XcmV2MultiLocation: XcmV2MultiLocation;
    XcmV2MultiassetAssetId: XcmV2MultiassetAssetId;
    XcmV2MultiassetAssetInstance: XcmV2MultiassetAssetInstance;
    XcmV2MultiassetFungibility: XcmV2MultiassetFungibility;
    XcmV2MultiassetMultiAssetFilter: XcmV2MultiassetMultiAssetFilter;
    XcmV2MultiassetMultiAssets: XcmV2MultiassetMultiAssets;
    XcmV2MultiassetWildFungibility: XcmV2MultiassetWildFungibility;
    XcmV2MultiassetWildMultiAsset: XcmV2MultiassetWildMultiAsset;
    XcmV2MultilocationJunctions: XcmV2MultilocationJunctions;
    XcmV2NetworkId: XcmV2NetworkId;
    XcmV2OriginKind: XcmV2OriginKind;
    XcmV2Response: XcmV2Response;
    XcmV2TraitsError: XcmV2TraitsError;
    XcmV2WeightLimit: XcmV2WeightLimit;
    XcmV2Xcm: XcmV2Xcm;
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
    XcmVersionedAssetId: XcmVersionedAssetId;
    XcmVersionedAssets: XcmVersionedAssets;
    XcmVersionedLocation: XcmVersionedLocation;
    XcmVersionedResponse: XcmVersionedResponse;
    XcmVersionedXcm: XcmVersionedXcm;
  }
}
