declare const _default: {
    /**
     * Lookup3: frame_system::AccountInfo<Nonce, pallet_balances::types::AccountData<Balance>>
     **/
    FrameSystemAccountInfo: {
        nonce: string;
        consumers: string;
        providers: string;
        sufficients: string;
        data: string;
    };
    /**
     * Lookup5: pallet_balances::types::AccountData<Balance>
     **/
    PalletBalancesAccountData: {
        free: string;
        reserved: string;
        frozen: string;
        flags: string;
    };
    /**
     * Lookup9: frame_support::dispatch::PerDispatchClass<sp_weights::weight_v2::Weight>
     **/
    FrameSupportDispatchPerDispatchClassWeight: {
        normal: string;
        operational: string;
        mandatory: string;
    };
    /**
     * Lookup10: sp_weights::weight_v2::Weight
     **/
    SpWeightsWeightV2Weight: {
        refTime: string;
        proofSize: string;
    };
    /**
     * Lookup16: sp_runtime::generic::digest::Digest
     **/
    SpRuntimeDigest: {
        logs: string;
    };
    /**
     * Lookup18: sp_runtime::generic::digest::DigestItem
     **/
    SpRuntimeDigestDigestItem: {
        _enum: {
            Other: string;
            __Unused1: string;
            __Unused2: string;
            __Unused3: string;
            Consensus: string;
            Seal: string;
            PreRuntime: string;
            __Unused7: string;
            RuntimeEnvironmentUpdated: string;
        };
    };
    /**
     * Lookup21: frame_system::EventRecord<sh_solochain_evm_runtime::RuntimeEvent, primitive_types::H256>
     **/
    FrameSystemEventRecord: {
        phase: string;
        event: string;
        topics: string;
    };
    /**
     * Lookup23: frame_system::pallet::Event<T>
     **/
    FrameSystemEvent: {
        _enum: {
            ExtrinsicSuccess: {
                dispatchInfo: string;
            };
            ExtrinsicFailed: {
                dispatchError: string;
                dispatchInfo: string;
            };
            CodeUpdated: string;
            NewAccount: {
                account: string;
            };
            KilledAccount: {
                account: string;
            };
            Remarked: {
                _alias: {
                    hash_: string;
                };
                sender: string;
                hash_: string;
            };
            UpgradeAuthorized: {
                codeHash: string;
                checkVersion: string;
            };
        };
    };
    /**
     * Lookup24: frame_system::DispatchEventInfo
     **/
    FrameSystemDispatchEventInfo: {
        weight: string;
        class: string;
        paysFee: string;
    };
    /**
     * Lookup25: frame_support::dispatch::DispatchClass
     **/
    FrameSupportDispatchDispatchClass: {
        _enum: string[];
    };
    /**
     * Lookup26: frame_support::dispatch::Pays
     **/
    FrameSupportDispatchPays: {
        _enum: string[];
    };
    /**
     * Lookup27: sp_runtime::DispatchError
     **/
    SpRuntimeDispatchError: {
        _enum: {
            Other: string;
            CannotLookup: string;
            BadOrigin: string;
            Module: string;
            ConsumerRemaining: string;
            NoProviders: string;
            TooManyConsumers: string;
            Token: string;
            Arithmetic: string;
            Transactional: string;
            Exhausted: string;
            Corruption: string;
            Unavailable: string;
            RootNotAllowed: string;
            Trie: string;
        };
    };
    /**
     * Lookup28: sp_runtime::ModuleError
     **/
    SpRuntimeModuleError: {
        index: string;
        error: string;
    };
    /**
     * Lookup29: sp_runtime::TokenError
     **/
    SpRuntimeTokenError: {
        _enum: string[];
    };
    /**
     * Lookup30: sp_arithmetic::ArithmeticError
     **/
    SpArithmeticArithmeticError: {
        _enum: string[];
    };
    /**
     * Lookup31: sp_runtime::TransactionalError
     **/
    SpRuntimeTransactionalError: {
        _enum: string[];
    };
    /**
     * Lookup32: sp_runtime::proving_trie::TrieError
     **/
    SpRuntimeProvingTrieTrieError: {
        _enum: string[];
    };
    /**
     * Lookup33: pallet_balances::pallet::Event<T, I>
     **/
    PalletBalancesEvent: {
        _enum: {
            Endowed: {
                account: string;
                freeBalance: string;
            };
            DustLost: {
                account: string;
                amount: string;
            };
            Transfer: {
                from: string;
                to: string;
                amount: string;
            };
            BalanceSet: {
                who: string;
                free: string;
            };
            Reserved: {
                who: string;
                amount: string;
            };
            Unreserved: {
                who: string;
                amount: string;
            };
            ReserveRepatriated: {
                from: string;
                to: string;
                amount: string;
                destinationStatus: string;
            };
            Deposit: {
                who: string;
                amount: string;
            };
            Withdraw: {
                who: string;
                amount: string;
            };
            Slashed: {
                who: string;
                amount: string;
            };
            Minted: {
                who: string;
                amount: string;
            };
            Burned: {
                who: string;
                amount: string;
            };
            Suspended: {
                who: string;
                amount: string;
            };
            Restored: {
                who: string;
                amount: string;
            };
            Upgraded: {
                who: string;
            };
            Issued: {
                amount: string;
            };
            Rescinded: {
                amount: string;
            };
            Locked: {
                who: string;
                amount: string;
            };
            Unlocked: {
                who: string;
                amount: string;
            };
            Frozen: {
                who: string;
                amount: string;
            };
            Thawed: {
                who: string;
                amount: string;
            };
            TotalIssuanceForced: {
                _alias: {
                    new_: string;
                };
                old: string;
                new_: string;
            };
        };
    };
    /**
     * Lookup34: frame_support::traits::tokens::misc::BalanceStatus
     **/
    FrameSupportTokensMiscBalanceStatus: {
        _enum: string[];
    };
    /**
     * Lookup35: pallet_offences::pallet::Event
     **/
    PalletOffencesEvent: {
        _enum: {
            Offence: {
                kind: string;
                timeslot: string;
            };
        };
    };
    /**
     * Lookup37: pallet_session::pallet::Event
     **/
    PalletSessionEvent: {
        _enum: {
            NewSession: {
                sessionIndex: string;
            };
        };
    };
    /**
     * Lookup38: pallet_grandpa::pallet::Event
     **/
    PalletGrandpaEvent: {
        _enum: {
            NewAuthorities: {
                authoritySet: string;
            };
            Paused: string;
            Resumed: string;
        };
    };
    /**
     * Lookup41: sp_consensus_grandpa::app::Public
     **/
    SpConsensusGrandpaAppPublic: string;
    /**
     * Lookup42: pallet_transaction_payment::pallet::Event<T>
     **/
    PalletTransactionPaymentEvent: {
        _enum: {
            TransactionFeePaid: {
                who: string;
                actualFee: string;
                tip: string;
            };
        };
    };
    /**
     * Lookup43: pallet_parameters::pallet::Event<T>
     **/
    PalletParametersEvent: {
        _enum: {
            Updated: {
                key: string;
                oldValue: string;
                newValue: string;
            };
        };
    };
    /**
     * Lookup44: sh_solochain_evm_runtime::configs::runtime_params::RuntimeParametersKey
     **/
    ShSolochainEvmRuntimeConfigsRuntimeParamsRuntimeParametersKey: {
        _enum: {
            RuntimeConfig: string;
        };
    };
    /**
     * Lookup45: sh_solochain_evm_runtime::configs::runtime_params::dynamic_params::runtime_config::ParametersKey
     **/
    ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigParametersKey: {
        _enum: string[];
    };
    /**
     * Lookup46: sh_solochain_evm_runtime::configs::runtime_params::dynamic_params::runtime_config::SlashAmountPerMaxFileSize
     **/
    ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigSlashAmountPerMaxFileSize: string;
    /**
     * Lookup47: sh_solochain_evm_runtime::configs::runtime_params::dynamic_params::runtime_config::StakeToChallengePeriod
     **/
    ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigStakeToChallengePeriod: string;
    /**
     * Lookup48: sh_solochain_evm_runtime::configs::runtime_params::dynamic_params::runtime_config::CheckpointChallengePeriod
     **/
    ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigCheckpointChallengePeriod: string;
    /**
     * Lookup49: sh_solochain_evm_runtime::configs::runtime_params::dynamic_params::runtime_config::MinChallengePeriod
     **/
    ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMinChallengePeriod: string;
    /**
     * Lookup50: sh_solochain_evm_runtime::configs::runtime_params::dynamic_params::runtime_config::SystemUtilisationLowerThresholdPercentage
     **/
    ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigSystemUtilisationLowerThresholdPercentage: string;
    /**
     * Lookup51: sh_solochain_evm_runtime::configs::runtime_params::dynamic_params::runtime_config::SystemUtilisationUpperThresholdPercentage
     **/
    ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigSystemUtilisationUpperThresholdPercentage: string;
    /**
     * Lookup52: sh_solochain_evm_runtime::configs::runtime_params::dynamic_params::runtime_config::MostlyStablePrice
     **/
    ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMostlyStablePrice: string;
    /**
     * Lookup53: sh_solochain_evm_runtime::configs::runtime_params::dynamic_params::runtime_config::MaxPrice
     **/
    ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMaxPrice: string;
    /**
     * Lookup54: sh_solochain_evm_runtime::configs::runtime_params::dynamic_params::runtime_config::MinPrice
     **/
    ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMinPrice: string;
    /**
     * Lookup55: sh_solochain_evm_runtime::configs::runtime_params::dynamic_params::runtime_config::UpperExponentFactor
     **/
    ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigUpperExponentFactor: string;
    /**
     * Lookup56: sh_solochain_evm_runtime::configs::runtime_params::dynamic_params::runtime_config::LowerExponentFactor
     **/
    ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigLowerExponentFactor: string;
    /**
     * Lookup57: sh_solochain_evm_runtime::configs::runtime_params::dynamic_params::runtime_config::ZeroSizeBucketFixedRate
     **/
    ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigZeroSizeBucketFixedRate: string;
    /**
     * Lookup58: sh_solochain_evm_runtime::configs::runtime_params::dynamic_params::runtime_config::IdealUtilisationRate
     **/
    ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigIdealUtilisationRate: string;
    /**
     * Lookup59: sh_solochain_evm_runtime::configs::runtime_params::dynamic_params::runtime_config::DecayRate
     **/
    ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigDecayRate: string;
    /**
     * Lookup60: sh_solochain_evm_runtime::configs::runtime_params::dynamic_params::runtime_config::MinimumTreasuryCut
     **/
    ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMinimumTreasuryCut: string;
    /**
     * Lookup61: sh_solochain_evm_runtime::configs::runtime_params::dynamic_params::runtime_config::MaximumTreasuryCut
     **/
    ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMaximumTreasuryCut: string;
    /**
     * Lookup62: sh_solochain_evm_runtime::configs::runtime_params::dynamic_params::runtime_config::BspStopStoringFilePenalty
     **/
    ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigBspStopStoringFilePenalty: string;
    /**
     * Lookup63: sh_solochain_evm_runtime::configs::runtime_params::dynamic_params::runtime_config::ProviderTopUpTtl
     **/
    ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigProviderTopUpTtl: string;
    /**
     * Lookup64: sh_solochain_evm_runtime::configs::runtime_params::dynamic_params::runtime_config::BasicReplicationTarget
     **/
    ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigBasicReplicationTarget: string;
    /**
     * Lookup65: sh_solochain_evm_runtime::configs::runtime_params::dynamic_params::runtime_config::StandardReplicationTarget
     **/
    ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigStandardReplicationTarget: string;
    /**
     * Lookup66: sh_solochain_evm_runtime::configs::runtime_params::dynamic_params::runtime_config::HighSecurityReplicationTarget
     **/
    ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigHighSecurityReplicationTarget: string;
    /**
     * Lookup67: sh_solochain_evm_runtime::configs::runtime_params::dynamic_params::runtime_config::SuperHighSecurityReplicationTarget
     **/
    ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigSuperHighSecurityReplicationTarget: string;
    /**
     * Lookup68: sh_solochain_evm_runtime::configs::runtime_params::dynamic_params::runtime_config::UltraHighSecurityReplicationTarget
     **/
    ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigUltraHighSecurityReplicationTarget: string;
    /**
     * Lookup69: sh_solochain_evm_runtime::configs::runtime_params::dynamic_params::runtime_config::MaxReplicationTarget
     **/
    ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMaxReplicationTarget: string;
    /**
     * Lookup70: sh_solochain_evm_runtime::configs::runtime_params::dynamic_params::runtime_config::TickRangeToMaximumThreshold
     **/
    ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigTickRangeToMaximumThreshold: string;
    /**
     * Lookup71: sh_solochain_evm_runtime::configs::runtime_params::dynamic_params::runtime_config::StorageRequestTtl
     **/
    ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigStorageRequestTtl: string;
    /**
     * Lookup72: sh_solochain_evm_runtime::configs::runtime_params::dynamic_params::runtime_config::MinWaitForStopStoring
     **/
    ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMinWaitForStopStoring: string;
    /**
     * Lookup73: sh_solochain_evm_runtime::configs::runtime_params::dynamic_params::runtime_config::MinSeedPeriod
     **/
    ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMinSeedPeriod: string;
    /**
     * Lookup74: sh_solochain_evm_runtime::configs::runtime_params::dynamic_params::runtime_config::StakeToSeedPeriod
     **/
    ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigStakeToSeedPeriod: string;
    /**
     * Lookup75: sh_solochain_evm_runtime::configs::runtime_params::dynamic_params::runtime_config::UpfrontTicksToPay
     **/
    ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigUpfrontTicksToPay: string;
    /**
     * Lookup77: sh_solochain_evm_runtime::configs::runtime_params::RuntimeParametersValue
     **/
    ShSolochainEvmRuntimeConfigsRuntimeParamsRuntimeParametersValue: {
        _enum: {
            RuntimeConfig: string;
        };
    };
    /**
     * Lookup78: sh_solochain_evm_runtime::configs::runtime_params::dynamic_params::runtime_config::ParametersValue
     **/
    ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigParametersValue: {
        _enum: {
            SlashAmountPerMaxFileSize: string;
            StakeToChallengePeriod: string;
            CheckpointChallengePeriod: string;
            MinChallengePeriod: string;
            SystemUtilisationLowerThresholdPercentage: string;
            SystemUtilisationUpperThresholdPercentage: string;
            MostlyStablePrice: string;
            MaxPrice: string;
            MinPrice: string;
            UpperExponentFactor: string;
            LowerExponentFactor: string;
            ZeroSizeBucketFixedRate: string;
            IdealUtilisationRate: string;
            DecayRate: string;
            MinimumTreasuryCut: string;
            MaximumTreasuryCut: string;
            BspStopStoringFilePenalty: string;
            ProviderTopUpTtl: string;
            BasicReplicationTarget: string;
            StandardReplicationTarget: string;
            HighSecurityReplicationTarget: string;
            SuperHighSecurityReplicationTarget: string;
            UltraHighSecurityReplicationTarget: string;
            MaxReplicationTarget: string;
            TickRangeToMaximumThreshold: string;
            StorageRequestTtl: string;
            MinWaitForStopStoring: string;
            MinSeedPeriod: string;
            StakeToSeedPeriod: string;
            UpfrontTicksToPay: string;
        };
    };
    /**
     * Lookup80: pallet_sudo::pallet::Event<T>
     **/
    PalletSudoEvent: {
        _enum: {
            Sudid: {
                sudoResult: string;
            };
            KeyChanged: {
                _alias: {
                    new_: string;
                };
                old: string;
                new_: string;
            };
            KeyRemoved: string;
            SudoAsDone: {
                sudoResult: string;
            };
        };
    };
    /**
     * Lookup84: pallet_ethereum::pallet::Event
     **/
    PalletEthereumEvent: {
        _enum: {
            Executed: {
                from: string;
                to: string;
                transactionHash: string;
                exitReason: string;
                extraData: string;
            };
        };
    };
    /**
     * Lookup86: evm_core::error::ExitReason
     **/
    EvmCoreErrorExitReason: {
        _enum: {
            Succeed: string;
            Error: string;
            Revert: string;
            Fatal: string;
        };
    };
    /**
     * Lookup87: evm_core::error::ExitSucceed
     **/
    EvmCoreErrorExitSucceed: {
        _enum: string[];
    };
    /**
     * Lookup88: evm_core::error::ExitError
     **/
    EvmCoreErrorExitError: {
        _enum: {
            StackUnderflow: string;
            StackOverflow: string;
            InvalidJump: string;
            InvalidRange: string;
            DesignatedInvalid: string;
            CallTooDeep: string;
            CreateCollision: string;
            CreateContractLimit: string;
            OutOfOffset: string;
            OutOfGas: string;
            OutOfFund: string;
            PCUnderflow: string;
            CreateEmpty: string;
            Other: string;
            MaxNonce: string;
            InvalidCode: string;
        };
    };
    /**
     * Lookup92: evm_core::error::ExitRevert
     **/
    EvmCoreErrorExitRevert: {
        _enum: string[];
    };
    /**
     * Lookup93: evm_core::error::ExitFatal
     **/
    EvmCoreErrorExitFatal: {
        _enum: {
            NotSupported: string;
            UnhandledInterrupt: string;
            CallErrorAsFatal: string;
            Other: string;
        };
    };
    /**
     * Lookup94: pallet_evm::pallet::Event<T>
     **/
    PalletEvmEvent: {
        _enum: {
            Log: {
                log: string;
            };
            Created: {
                address: string;
            };
            CreatedFailed: {
                address: string;
            };
            Executed: {
                address: string;
            };
            ExecutedFailed: {
                address: string;
            };
        };
    };
    /**
     * Lookup95: ethereum::log::Log
     **/
    EthereumLog: {
        address: string;
        topics: string;
        data: string;
    };
    /**
     * Lookup97: pallet_storage_providers::pallet::Event<T>
     **/
    PalletStorageProvidersEvent: {
        _enum: {
            MspRequestSignUpSuccess: {
                who: string;
                multiaddresses: string;
                capacity: string;
            };
            MspSignUpSuccess: {
                who: string;
                mspId: string;
                multiaddresses: string;
                capacity: string;
                valueProp: string;
            };
            BspRequestSignUpSuccess: {
                who: string;
                multiaddresses: string;
                capacity: string;
            };
            BspSignUpSuccess: {
                who: string;
                bspId: string;
                root: string;
                multiaddresses: string;
                capacity: string;
            };
            SignUpRequestCanceled: {
                who: string;
            };
            MspSignOffSuccess: {
                who: string;
                mspId: string;
            };
            BspSignOffSuccess: {
                who: string;
                bspId: string;
            };
            CapacityChanged: {
                who: string;
                providerId: string;
                oldCapacity: string;
                newCapacity: string;
                nextBlockWhenChangeAllowed: string;
            };
            Slashed: {
                providerId: string;
                amount: string;
            };
            AwaitingTopUp: {
                providerId: string;
                topUpMetadata: string;
            };
            TopUpFulfilled: {
                providerId: string;
                amount: string;
            };
            FailedToGetOwnerAccountOfInsolventProvider: {
                providerId: string;
            };
            FailedToSlashInsolventProvider: {
                providerId: string;
                amountToSlash: string;
                error: string;
            };
            FailedToStopAllCyclesForInsolventBsp: {
                providerId: string;
                error: string;
            };
            FailedToInsertProviderTopUpExpiration: {
                providerId: string;
                expirationTick: string;
            };
            ProviderInsolvent: {
                providerId: string;
            };
            BucketsOfInsolventMsp: {
                mspId: string;
                buckets: string;
            };
            BucketRootChanged: {
                bucketId: string;
                oldRoot: string;
                newRoot: string;
            };
            MultiAddressAdded: {
                providerId: string;
                newMultiaddress: string;
            };
            MultiAddressRemoved: {
                providerId: string;
                removedMultiaddress: string;
            };
            ValuePropAdded: {
                mspId: string;
                valuePropId: string;
                valueProp: string;
            };
            ValuePropUnavailable: {
                mspId: string;
                valuePropId: string;
            };
            MspDeleted: {
                providerId: string;
            };
            BspDeleted: {
                providerId: string;
            };
        };
    };
    /**
     * Lookup101: pallet_storage_providers::types::ValuePropositionWithId<T>
     **/
    PalletStorageProvidersValuePropositionWithId: {
        id: string;
        valueProp: string;
    };
    /**
     * Lookup102: pallet_storage_providers::types::ValueProposition<T>
     **/
    PalletStorageProvidersValueProposition: {
        pricePerGigaUnitOfDataPerBlock: string;
        commitment: string;
        bucketDataLimit: string;
        available: string;
    };
    /**
     * Lookup104: pallet_storage_providers::types::StorageProviderId<T>
     **/
    PalletStorageProvidersStorageProviderId: {
        _enum: {
            BackupStorageProvider: string;
            MainStorageProvider: string;
        };
    };
    /**
     * Lookup105: pallet_storage_providers::types::TopUpMetadata<T>
     **/
    PalletStorageProvidersTopUpMetadata: {
        startedAt: string;
        endTickGracePeriod: string;
    };
    /**
     * Lookup106: pallet_file_system::pallet::Event<T>
     **/
    PalletFileSystemEvent: {
        _enum: {
            NewBucket: {
                who: string;
                mspId: string;
                bucketId: string;
                name: string;
                root: string;
                collectionId: string;
                private: string;
                valuePropId: string;
            };
            BucketDeleted: {
                who: string;
                bucketId: string;
                maybeCollectionId: string;
            };
            BucketPrivacyUpdated: {
                who: string;
                bucketId: string;
                collectionId: string;
                private: string;
            };
            NewCollectionAndAssociation: {
                who: string;
                bucketId: string;
                collectionId: string;
            };
            MoveBucketRequested: {
                who: string;
                bucketId: string;
                newMspId: string;
                newValuePropId: string;
            };
            MoveBucketRequestExpired: {
                bucketId: string;
            };
            MoveBucketAccepted: {
                bucketId: string;
                oldMspId: string;
                newMspId: string;
                valuePropId: string;
            };
            MoveBucketRejected: {
                bucketId: string;
                oldMspId: string;
                newMspId: string;
            };
            NewStorageRequest: {
                _alias: {
                    size_: string;
                };
                who: string;
                fileKey: string;
                bucketId: string;
                location: string;
                fingerprint: string;
                size_: string;
                peerIds: string;
                expiresAt: string;
            };
            MspAcceptedStorageRequest: {
                fileKey: string;
                fileMetadata: string;
            };
            StorageRequestFulfilled: {
                fileKey: string;
            };
            StorageRequestExpired: {
                fileKey: string;
            };
            StorageRequestRevoked: {
                fileKey: string;
            };
            StorageRequestRejected: {
                fileKey: string;
                mspId: string;
                bucketId: string;
                reason: string;
            };
            IncompleteStorageRequest: {
                fileKey: string;
            };
            IncompleteStorageRequestCleanedUp: {
                fileKey: string;
            };
            AcceptedBspVolunteer: {
                _alias: {
                    size_: string;
                };
                bspId: string;
                bucketId: string;
                location: string;
                fingerprint: string;
                multiaddresses: string;
                owner: string;
                size_: string;
            };
            BspConfirmedStoring: {
                who: string;
                bspId: string;
                confirmedFileKeys: string;
                skippedFileKeys: string;
                newRoot: string;
            };
            BspChallengeCycleInitialised: {
                who: string;
                bspId: string;
            };
            BspRequestedToStopStoring: {
                bspId: string;
                fileKey: string;
                owner: string;
                location: string;
            };
            BspConfirmStoppedStoring: {
                bspId: string;
                fileKey: string;
                newRoot: string;
            };
            MspStoppedStoringBucket: {
                mspId: string;
                owner: string;
                bucketId: string;
            };
            SpStopStoringInsolventUser: {
                spId: string;
                fileKey: string;
                owner: string;
                location: string;
                newRoot: string;
            };
            MspStopStoringBucketInsolventUser: {
                mspId: string;
                owner: string;
                bucketId: string;
            };
            FileDeletionRequested: {
                signedDeleteIntention: string;
                signature: string;
            };
            BucketFileDeletionsCompleted: {
                user: string;
                fileKeys: string;
                bucketId: string;
                mspId: string;
                oldRoot: string;
                newRoot: string;
            };
            BspFileDeletionsCompleted: {
                users: string;
                fileKeys: string;
                bspId: string;
                oldRoot: string;
                newRoot: string;
            };
            UsedCapacityShouldBeZero: {
                actualUsedCapacity: string;
            };
            FailedToReleaseStorageRequestCreationDeposit: {
                fileKey: string;
                owner: string;
                amountToReturn: string;
                error: string;
            };
        };
    };
    /**
     * Lookup110: shp_file_metadata::FileMetadata
     **/
    ShpFileMetadataFileMetadata: {
        owner: string;
        bucketId: string;
        location: string;
        fileSize: string;
        fingerprint: string;
    };
    /**
     * Lookup111: shp_file_metadata::Fingerprint
     **/
    ShpFileMetadataFingerprint: string;
    /**
     * Lookup112: pallet_file_system::types::RejectedStorageRequestReason
     **/
    PalletFileSystemRejectedStorageRequestReason: {
        _enum: string[];
    };
    /**
     * Lookup117: pallet_file_system::types::FileOperationIntention<T>
     **/
    PalletFileSystemFileOperationIntention: {
        fileKey: string;
        operation: string;
    };
    /**
     * Lookup118: pallet_file_system::types::FileOperation
     **/
    PalletFileSystemFileOperation: {
        _enum: string[];
    };
    /**
     * Lookup119: fp_account::EthereumSignature
     **/
    FpAccountEthereumSignature: string;
    /**
     * Lookup124: pallet_proofs_dealer::pallet::Event<T>
     **/
    PalletProofsDealerEvent: {
        _enum: {
            NewChallenge: {
                who: string;
                keyChallenged: string;
            };
            NewPriorityChallenge: {
                who: string;
                keyChallenged: string;
                shouldRemoveKey: string;
            };
            ProofAccepted: {
                providerId: string;
                proof: string;
                lastTickProven: string;
            };
            NewChallengeSeed: {
                challengesTicker: string;
                seed: string;
            };
            NewCheckpointChallenge: {
                challengesTicker: string;
                challenges: string;
            };
            SlashableProvider: {
                provider: string;
                nextChallengeDeadline: string;
            };
            NoRecordOfLastSubmittedProof: {
                provider: string;
            };
            NewChallengeCycleInitialised: {
                currentTick: string;
                nextChallengeDeadline: string;
                provider: string;
                maybeProviderAccount: string;
            };
            MutationsAppliedForProvider: {
                providerId: string;
                mutations: string;
                oldRoot: string;
                newRoot: string;
            };
            MutationsApplied: {
                mutations: string;
                oldRoot: string;
                newRoot: string;
                eventInfo: string;
            };
            ChallengesTickerSet: {
                paused: string;
            };
        };
    };
    /**
     * Lookup125: pallet_proofs_dealer::types::Proof<T>
     **/
    PalletProofsDealerProof: {
        forestProof: string;
        keyProofs: string;
    };
    /**
     * Lookup126: sp_trie::storage_proof::CompactProof
     **/
    SpTrieStorageProofCompactProof: {
        encodedNodes: string;
    };
    /**
     * Lookup129: pallet_proofs_dealer::types::KeyProof<T>
     **/
    PalletProofsDealerKeyProof: {
        proof: string;
        challengeCount: string;
    };
    /**
     * Lookup130: shp_file_key_verifier::types::FileKeyProof
     **/
    ShpFileKeyVerifierFileKeyProof: {
        fileMetadata: string;
        proof: string;
    };
    /**
     * Lookup134: pallet_proofs_dealer::types::CustomChallenge<T>
     **/
    PalletProofsDealerCustomChallenge: {
        key: string;
        shouldRemoveKey: string;
    };
    /**
     * Lookup138: shp_traits::TrieMutation
     **/
    ShpTraitsTrieMutation: {
        _enum: {
            Add: string;
            Remove: string;
        };
    };
    /**
     * Lookup139: shp_traits::TrieAddMutation
     **/
    ShpTraitsTrieAddMutation: {
        value: string;
    };
    /**
     * Lookup140: shp_traits::TrieRemoveMutation
     **/
    ShpTraitsTrieRemoveMutation: {
        maybeValue: string;
    };
    /**
     * Lookup142: pallet_randomness::pallet::Event<T>
     **/
    PalletRandomnessEvent: {
        _enum: {
            NewOneEpochAgoRandomnessAvailable: {
                randomnessSeed: string;
                fromEpoch: string;
                validUntilBlock: string;
            };
        };
    };
    /**
     * Lookup143: pallet_payment_streams::pallet::Event<T>
     **/
    PalletPaymentStreamsEvent: {
        _enum: {
            FixedRatePaymentStreamCreated: {
                userAccount: string;
                providerId: string;
                rate: string;
            };
            FixedRatePaymentStreamUpdated: {
                userAccount: string;
                providerId: string;
                newRate: string;
            };
            FixedRatePaymentStreamDeleted: {
                userAccount: string;
                providerId: string;
            };
            DynamicRatePaymentStreamCreated: {
                userAccount: string;
                providerId: string;
                amountProvided: string;
            };
            DynamicRatePaymentStreamUpdated: {
                userAccount: string;
                providerId: string;
                newAmountProvided: string;
            };
            DynamicRatePaymentStreamDeleted: {
                userAccount: string;
                providerId: string;
            };
            PaymentStreamCharged: {
                userAccount: string;
                providerId: string;
                amount: string;
                lastTickCharged: string;
                chargedAtTick: string;
            };
            UsersCharged: {
                userAccounts: string;
                providerId: string;
                chargedAtTick: string;
            };
            LastChargeableInfoUpdated: {
                providerId: string;
                lastChargeableTick: string;
                lastChargeablePriceIndex: string;
            };
            UserWithoutFunds: {
                who: string;
            };
            UserPaidAllDebts: {
                who: string;
            };
            UserPaidSomeDebts: {
                who: string;
            };
            UserSolvent: {
                who: string;
            };
            InconsistentTickProcessing: {
                lastProcessedTick: string;
                tickToProcess: string;
            };
        };
    };
    /**
     * Lookup145: pallet_bucket_nfts::pallet::Event<T>
     **/
    PalletBucketNftsEvent: {
        _enum: {
            AccessShared: {
                issuer: string;
                recipient: string;
            };
            ItemReadAccessUpdated: {
                admin: string;
                bucket: string;
                itemId: string;
            };
            ItemBurned: {
                account: string;
                bucket: string;
                itemId: string;
            };
        };
    };
    /**
     * Lookup146: pallet_nfts::pallet::Event<T, I>
     **/
    PalletNftsEvent: {
        _enum: {
            Created: {
                collection: string;
                creator: string;
                owner: string;
            };
            ForceCreated: {
                collection: string;
                owner: string;
            };
            Destroyed: {
                collection: string;
            };
            Issued: {
                collection: string;
                item: string;
                owner: string;
            };
            Transferred: {
                collection: string;
                item: string;
                from: string;
                to: string;
            };
            Burned: {
                collection: string;
                item: string;
                owner: string;
            };
            ItemTransferLocked: {
                collection: string;
                item: string;
            };
            ItemTransferUnlocked: {
                collection: string;
                item: string;
            };
            ItemPropertiesLocked: {
                collection: string;
                item: string;
                lockMetadata: string;
                lockAttributes: string;
            };
            CollectionLocked: {
                collection: string;
            };
            OwnerChanged: {
                collection: string;
                newOwner: string;
            };
            TeamChanged: {
                collection: string;
                issuer: string;
                admin: string;
                freezer: string;
            };
            TransferApproved: {
                collection: string;
                item: string;
                owner: string;
                delegate: string;
                deadline: string;
            };
            ApprovalCancelled: {
                collection: string;
                item: string;
                owner: string;
                delegate: string;
            };
            AllApprovalsCancelled: {
                collection: string;
                item: string;
                owner: string;
            };
            CollectionConfigChanged: {
                collection: string;
            };
            CollectionMetadataSet: {
                collection: string;
                data: string;
            };
            CollectionMetadataCleared: {
                collection: string;
            };
            ItemMetadataSet: {
                collection: string;
                item: string;
                data: string;
            };
            ItemMetadataCleared: {
                collection: string;
                item: string;
            };
            Redeposited: {
                collection: string;
                successfulItems: string;
            };
            AttributeSet: {
                collection: string;
                maybeItem: string;
                key: string;
                value: string;
                namespace: string;
            };
            AttributeCleared: {
                collection: string;
                maybeItem: string;
                key: string;
                namespace: string;
            };
            ItemAttributesApprovalAdded: {
                collection: string;
                item: string;
                delegate: string;
            };
            ItemAttributesApprovalRemoved: {
                collection: string;
                item: string;
                delegate: string;
            };
            OwnershipAcceptanceChanged: {
                who: string;
                maybeCollection: string;
            };
            CollectionMaxSupplySet: {
                collection: string;
                maxSupply: string;
            };
            CollectionMintSettingsUpdated: {
                collection: string;
            };
            NextCollectionIdIncremented: {
                nextId: string;
            };
            ItemPriceSet: {
                collection: string;
                item: string;
                price: string;
                whitelistedBuyer: string;
            };
            ItemPriceRemoved: {
                collection: string;
                item: string;
            };
            ItemBought: {
                collection: string;
                item: string;
                price: string;
                seller: string;
                buyer: string;
            };
            TipSent: {
                collection: string;
                item: string;
                sender: string;
                receiver: string;
                amount: string;
            };
            SwapCreated: {
                offeredCollection: string;
                offeredItem: string;
                desiredCollection: string;
                desiredItem: string;
                price: string;
                deadline: string;
            };
            SwapCancelled: {
                offeredCollection: string;
                offeredItem: string;
                desiredCollection: string;
                desiredItem: string;
                price: string;
                deadline: string;
            };
            SwapClaimed: {
                sentCollection: string;
                sentItem: string;
                sentItemOwner: string;
                receivedCollection: string;
                receivedItem: string;
                receivedItemOwner: string;
                price: string;
                deadline: string;
            };
            PreSignedAttributesSet: {
                collection: string;
                item: string;
                namespace: string;
            };
            PalletAttributeSet: {
                collection: string;
                item: string;
                attribute: string;
                value: string;
            };
        };
    };
    /**
     * Lookup150: pallet_nfts::types::AttributeNamespace<fp_account::AccountId20>
     **/
    PalletNftsAttributeNamespace: {
        _enum: {
            Pallet: string;
            CollectionOwner: string;
            ItemOwner: string;
            Account: string;
        };
    };
    /**
     * Lookup152: pallet_nfts::types::PriceWithDirection<Amount>
     **/
    PalletNftsPriceWithDirection: {
        amount: string;
        direction: string;
    };
    /**
     * Lookup153: pallet_nfts::types::PriceDirection
     **/
    PalletNftsPriceDirection: {
        _enum: string[];
    };
    /**
     * Lookup154: pallet_nfts::types::PalletAttributes<CollectionId>
     **/
    PalletNftsPalletAttributes: {
        _enum: {
            UsedToClaim: string;
            TransferDisabled: string;
        };
    };
    /**
     * Lookup155: frame_system::Phase
     **/
    FrameSystemPhase: {
        _enum: {
            ApplyExtrinsic: string;
            Finalization: string;
            Initialization: string;
        };
    };
    /**
     * Lookup158: frame_system::LastRuntimeUpgradeInfo
     **/
    FrameSystemLastRuntimeUpgradeInfo: {
        specVersion: string;
        specName: string;
    };
    /**
     * Lookup160: frame_system::CodeUpgradeAuthorization<T>
     **/
    FrameSystemCodeUpgradeAuthorization: {
        codeHash: string;
        checkVersion: string;
    };
    /**
     * Lookup161: frame_system::pallet::Call<T>
     **/
    FrameSystemCall: {
        _enum: {
            remark: {
                remark: string;
            };
            set_heap_pages: {
                pages: string;
            };
            set_code: {
                code: string;
            };
            set_code_without_checks: {
                code: string;
            };
            set_storage: {
                items: string;
            };
            kill_storage: {
                _alias: {
                    keys_: string;
                };
                keys_: string;
            };
            kill_prefix: {
                prefix: string;
                subkeys: string;
            };
            remark_with_event: {
                remark: string;
            };
            __Unused8: string;
            authorize_upgrade: {
                codeHash: string;
            };
            authorize_upgrade_without_checks: {
                codeHash: string;
            };
            apply_authorized_upgrade: {
                code: string;
            };
        };
    };
    /**
     * Lookup164: frame_system::limits::BlockWeights
     **/
    FrameSystemLimitsBlockWeights: {
        baseBlock: string;
        maxBlock: string;
        perClass: string;
    };
    /**
     * Lookup165: frame_support::dispatch::PerDispatchClass<frame_system::limits::WeightsPerClass>
     **/
    FrameSupportDispatchPerDispatchClassWeightsPerClass: {
        normal: string;
        operational: string;
        mandatory: string;
    };
    /**
     * Lookup166: frame_system::limits::WeightsPerClass
     **/
    FrameSystemLimitsWeightsPerClass: {
        baseExtrinsic: string;
        maxExtrinsic: string;
        maxTotal: string;
        reserved: string;
    };
    /**
     * Lookup168: frame_system::limits::BlockLength
     **/
    FrameSystemLimitsBlockLength: {
        max: string;
    };
    /**
     * Lookup169: frame_support::dispatch::PerDispatchClass<T>
     **/
    FrameSupportDispatchPerDispatchClassU32: {
        normal: string;
        operational: string;
        mandatory: string;
    };
    /**
     * Lookup170: sp_weights::RuntimeDbWeight
     **/
    SpWeightsRuntimeDbWeight: {
        read: string;
        write: string;
    };
    /**
     * Lookup171: sp_version::RuntimeVersion
     **/
    SpVersionRuntimeVersion: {
        specName: string;
        implName: string;
        authoringVersion: string;
        specVersion: string;
        implVersion: string;
        apis: string;
        transactionVersion: string;
        systemVersion: string;
    };
    /**
     * Lookup177: frame_system::pallet::Error<T>
     **/
    FrameSystemError: {
        _enum: string[];
    };
    /**
     * Lookup180: sp_consensus_babe::app::Public
     **/
    SpConsensusBabeAppPublic: string;
    /**
     * Lookup183: sp_consensus_babe::digests::NextConfigDescriptor
     **/
    SpConsensusBabeDigestsNextConfigDescriptor: {
        _enum: {
            __Unused0: string;
            V1: {
                c: string;
                allowedSlots: string;
            };
        };
    };
    /**
     * Lookup185: sp_consensus_babe::AllowedSlots
     **/
    SpConsensusBabeAllowedSlots: {
        _enum: string[];
    };
    /**
     * Lookup189: sp_consensus_babe::digests::PreDigest
     **/
    SpConsensusBabeDigestsPreDigest: {
        _enum: {
            __Unused0: string;
            Primary: string;
            SecondaryPlain: string;
            SecondaryVRF: string;
        };
    };
    /**
     * Lookup190: sp_consensus_babe::digests::PrimaryPreDigest
     **/
    SpConsensusBabeDigestsPrimaryPreDigest: {
        authorityIndex: string;
        slot: string;
        vrfSignature: string;
    };
    /**
     * Lookup191: sp_core::sr25519::vrf::VrfSignature
     **/
    SpCoreSr25519VrfVrfSignature: {
        preOutput: string;
        proof: string;
    };
    /**
     * Lookup193: sp_consensus_babe::digests::SecondaryPlainPreDigest
     **/
    SpConsensusBabeDigestsSecondaryPlainPreDigest: {
        authorityIndex: string;
        slot: string;
    };
    /**
     * Lookup194: sp_consensus_babe::digests::SecondaryVRFPreDigest
     **/
    SpConsensusBabeDigestsSecondaryVRFPreDigest: {
        authorityIndex: string;
        slot: string;
        vrfSignature: string;
    };
    /**
     * Lookup196: sp_consensus_babe::BabeEpochConfiguration
     **/
    SpConsensusBabeBabeEpochConfiguration: {
        c: string;
        allowedSlots: string;
    };
    /**
     * Lookup200: pallet_babe::pallet::Call<T>
     **/
    PalletBabeCall: {
        _enum: {
            report_equivocation: {
                equivocationProof: string;
                keyOwnerProof: string;
            };
            report_equivocation_unsigned: {
                equivocationProof: string;
                keyOwnerProof: string;
            };
            plan_config_change: {
                config: string;
            };
        };
    };
    /**
     * Lookup201: sp_consensus_slots::EquivocationProof<sp_runtime::generic::header::Header<Number, Hash>, sp_consensus_babe::app::Public>
     **/
    SpConsensusSlotsEquivocationProof: {
        offender: string;
        slot: string;
        firstHeader: string;
        secondHeader: string;
    };
    /**
     * Lookup202: sp_runtime::generic::header::Header<Number, Hash>
     **/
    SpRuntimeHeader: {
        parentHash: string;
        number: string;
        stateRoot: string;
        extrinsicsRoot: string;
        digest: string;
    };
    /**
     * Lookup203: sp_session::MembershipProof
     **/
    SpSessionMembershipProof: {
        session: string;
        trieNodes: string;
        validatorCount: string;
    };
    /**
     * Lookup204: pallet_babe::pallet::Error<T>
     **/
    PalletBabeError: {
        _enum: string[];
    };
    /**
     * Lookup205: pallet_timestamp::pallet::Call<T>
     **/
    PalletTimestampCall: {
        _enum: {
            set: {
                now: string;
            };
        };
    };
    /**
     * Lookup207: pallet_balances::types::BalanceLock<Balance>
     **/
    PalletBalancesBalanceLock: {
        id: string;
        amount: string;
        reasons: string;
    };
    /**
     * Lookup208: pallet_balances::types::Reasons
     **/
    PalletBalancesReasons: {
        _enum: string[];
    };
    /**
     * Lookup211: pallet_balances::types::ReserveData<ReserveIdentifier, Balance>
     **/
    PalletBalancesReserveData: {
        id: string;
        amount: string;
    };
    /**
     * Lookup214: frame_support::traits::tokens::misc::IdAmount<sh_solochain_evm_runtime::RuntimeHoldReason, Balance>
     **/
    FrameSupportTokensMiscIdAmountRuntimeHoldReason: {
        id: string;
        amount: string;
    };
    /**
     * Lookup215: sh_solochain_evm_runtime::RuntimeHoldReason
     **/
    ShSolochainEvmRuntimeRuntimeHoldReason: {
        _enum: {
            __Unused0: string;
            __Unused1: string;
            __Unused2: string;
            __Unused3: string;
            __Unused4: string;
            __Unused5: string;
            __Unused6: string;
            __Unused7: string;
            __Unused8: string;
            __Unused9: string;
            __Unused10: string;
            __Unused11: string;
            __Unused12: string;
            __Unused13: string;
            __Unused14: string;
            __Unused15: string;
            __Unused16: string;
            __Unused17: string;
            __Unused18: string;
            __Unused19: string;
            __Unused20: string;
            __Unused21: string;
            __Unused22: string;
            __Unused23: string;
            __Unused24: string;
            __Unused25: string;
            __Unused26: string;
            __Unused27: string;
            __Unused28: string;
            __Unused29: string;
            __Unused30: string;
            __Unused31: string;
            __Unused32: string;
            __Unused33: string;
            __Unused34: string;
            __Unused35: string;
            __Unused36: string;
            __Unused37: string;
            __Unused38: string;
            __Unused39: string;
            __Unused40: string;
            __Unused41: string;
            __Unused42: string;
            __Unused43: string;
            __Unused44: string;
            __Unused45: string;
            __Unused46: string;
            __Unused47: string;
            __Unused48: string;
            __Unused49: string;
            __Unused50: string;
            __Unused51: string;
            __Unused52: string;
            __Unused53: string;
            __Unused54: string;
            __Unused55: string;
            __Unused56: string;
            __Unused57: string;
            __Unused58: string;
            __Unused59: string;
            __Unused60: string;
            __Unused61: string;
            __Unused62: string;
            __Unused63: string;
            __Unused64: string;
            __Unused65: string;
            __Unused66: string;
            __Unused67: string;
            __Unused68: string;
            __Unused69: string;
            __Unused70: string;
            __Unused71: string;
            __Unused72: string;
            __Unused73: string;
            __Unused74: string;
            __Unused75: string;
            __Unused76: string;
            __Unused77: string;
            __Unused78: string;
            __Unused79: string;
            Providers: string;
            FileSystem: string;
            __Unused82: string;
            __Unused83: string;
            PaymentStreams: string;
        };
    };
    /**
     * Lookup216: pallet_storage_providers::pallet::HoldReason
     **/
    PalletStorageProvidersHoldReason: {
        _enum: string[];
    };
    /**
     * Lookup217: pallet_file_system::pallet::HoldReason
     **/
    PalletFileSystemHoldReason: {
        _enum: string[];
    };
    /**
     * Lookup218: pallet_payment_streams::pallet::HoldReason
     **/
    PalletPaymentStreamsHoldReason: {
        _enum: string[];
    };
    /**
     * Lookup221: frame_support::traits::tokens::misc::IdAmount<sh_solochain_evm_runtime::RuntimeFreezeReason, Balance>
     **/
    FrameSupportTokensMiscIdAmountRuntimeFreezeReason: {
        id: string;
        amount: string;
    };
    /**
     * Lookup222: sh_solochain_evm_runtime::RuntimeFreezeReason
     **/
    ShSolochainEvmRuntimeRuntimeFreezeReason: string;
    /**
     * Lookup224: pallet_balances::pallet::Call<T, I>
     **/
    PalletBalancesCall: {
        _enum: {
            transfer_allow_death: {
                dest: string;
                value: string;
            };
            __Unused1: string;
            force_transfer: {
                source: string;
                dest: string;
                value: string;
            };
            transfer_keep_alive: {
                dest: string;
                value: string;
            };
            transfer_all: {
                dest: string;
                keepAlive: string;
            };
            force_unreserve: {
                who: string;
                amount: string;
            };
            upgrade_accounts: {
                who: string;
            };
            __Unused7: string;
            force_set_balance: {
                who: string;
                newFree: string;
            };
            force_adjust_total_issuance: {
                direction: string;
                delta: string;
            };
            burn: {
                value: string;
                keepAlive: string;
            };
        };
    };
    /**
     * Lookup226: pallet_balances::types::AdjustmentDirection
     **/
    PalletBalancesAdjustmentDirection: {
        _enum: string[];
    };
    /**
     * Lookup227: pallet_balances::pallet::Error<T, I>
     **/
    PalletBalancesError: {
        _enum: string[];
    };
    /**
     * Lookup228: sp_staking::offence::OffenceDetails<fp_account::AccountId20, Offender>
     **/
    SpStakingOffenceOffenceDetails: {
        offender: string;
        reporters: string;
    };
    /**
     * Lookup234: sh_solochain_evm_runtime::SessionKeys
     **/
    ShSolochainEvmRuntimeSessionKeys: {
        babe: string;
        grandpa: string;
    };
    /**
     * Lookup236: sp_core::crypto::KeyTypeId
     **/
    SpCoreCryptoKeyTypeId: string;
    /**
     * Lookup237: pallet_session::pallet::Call<T>
     **/
    PalletSessionCall: {
        _enum: {
            set_keys: {
                _alias: {
                    keys_: string;
                };
                keys_: string;
                proof: string;
            };
            purge_keys: string;
        };
    };
    /**
     * Lookup238: pallet_session::pallet::Error<T>
     **/
    PalletSessionError: {
        _enum: string[];
    };
    /**
     * Lookup239: pallet_grandpa::StoredState<N>
     **/
    PalletGrandpaStoredState: {
        _enum: {
            Live: string;
            PendingPause: {
                scheduledAt: string;
                delay: string;
            };
            Paused: string;
            PendingResume: {
                scheduledAt: string;
                delay: string;
            };
        };
    };
    /**
     * Lookup240: pallet_grandpa::StoredPendingChange<N, Limit>
     **/
    PalletGrandpaStoredPendingChange: {
        scheduledAt: string;
        delay: string;
        nextAuthorities: string;
        forced: string;
    };
    /**
     * Lookup242: pallet_grandpa::pallet::Call<T>
     **/
    PalletGrandpaCall: {
        _enum: {
            report_equivocation: {
                equivocationProof: string;
                keyOwnerProof: string;
            };
            report_equivocation_unsigned: {
                equivocationProof: string;
                keyOwnerProof: string;
            };
            note_stalled: {
                delay: string;
                bestFinalizedBlockNumber: string;
            };
        };
    };
    /**
     * Lookup243: sp_consensus_grandpa::EquivocationProof<primitive_types::H256, N>
     **/
    SpConsensusGrandpaEquivocationProof: {
        setId: string;
        equivocation: string;
    };
    /**
     * Lookup244: sp_consensus_grandpa::Equivocation<primitive_types::H256, N>
     **/
    SpConsensusGrandpaEquivocation: {
        _enum: {
            Prevote: string;
            Precommit: string;
        };
    };
    /**
     * Lookup245: finality_grandpa::Equivocation<sp_consensus_grandpa::app::Public, finality_grandpa::Prevote<primitive_types::H256, N>, sp_consensus_grandpa::app::Signature>
     **/
    FinalityGrandpaEquivocationPrevote: {
        roundNumber: string;
        identity: string;
        first: string;
        second: string;
    };
    /**
     * Lookup246: finality_grandpa::Prevote<primitive_types::H256, N>
     **/
    FinalityGrandpaPrevote: {
        targetHash: string;
        targetNumber: string;
    };
    /**
     * Lookup247: sp_consensus_grandpa::app::Signature
     **/
    SpConsensusGrandpaAppSignature: string;
    /**
     * Lookup249: finality_grandpa::Equivocation<sp_consensus_grandpa::app::Public, finality_grandpa::Precommit<primitive_types::H256, N>, sp_consensus_grandpa::app::Signature>
     **/
    FinalityGrandpaEquivocationPrecommit: {
        roundNumber: string;
        identity: string;
        first: string;
        second: string;
    };
    /**
     * Lookup250: finality_grandpa::Precommit<primitive_types::H256, N>
     **/
    FinalityGrandpaPrecommit: {
        targetHash: string;
        targetNumber: string;
    };
    /**
     * Lookup252: pallet_grandpa::pallet::Error<T>
     **/
    PalletGrandpaError: {
        _enum: string[];
    };
    /**
     * Lookup254: pallet_transaction_payment::Releases
     **/
    PalletTransactionPaymentReleases: {
        _enum: string[];
    };
    /**
     * Lookup255: pallet_parameters::pallet::Call<T>
     **/
    PalletParametersCall: {
        _enum: {
            set_parameter: {
                keyValue: string;
            };
        };
    };
    /**
     * Lookup256: sh_solochain_evm_runtime::configs::runtime_params::RuntimeParameters
     **/
    ShSolochainEvmRuntimeConfigsRuntimeParamsRuntimeParameters: {
        _enum: {
            RuntimeConfig: string;
        };
    };
    /**
     * Lookup257: sh_solochain_evm_runtime::configs::runtime_params::dynamic_params::runtime_config::Parameters
     **/
    ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigParameters: {
        _enum: {
            SlashAmountPerMaxFileSize: string;
            StakeToChallengePeriod: string;
            CheckpointChallengePeriod: string;
            MinChallengePeriod: string;
            SystemUtilisationLowerThresholdPercentage: string;
            SystemUtilisationUpperThresholdPercentage: string;
            MostlyStablePrice: string;
            MaxPrice: string;
            MinPrice: string;
            UpperExponentFactor: string;
            LowerExponentFactor: string;
            ZeroSizeBucketFixedRate: string;
            IdealUtilisationRate: string;
            DecayRate: string;
            MinimumTreasuryCut: string;
            MaximumTreasuryCut: string;
            BspStopStoringFilePenalty: string;
            ProviderTopUpTtl: string;
            BasicReplicationTarget: string;
            StandardReplicationTarget: string;
            HighSecurityReplicationTarget: string;
            SuperHighSecurityReplicationTarget: string;
            UltraHighSecurityReplicationTarget: string;
            MaxReplicationTarget: string;
            TickRangeToMaximumThreshold: string;
            StorageRequestTtl: string;
            MinWaitForStopStoring: string;
            MinSeedPeriod: string;
            StakeToSeedPeriod: string;
            UpfrontTicksToPay: string;
        };
    };
    /**
     * Lookup260: pallet_sudo::pallet::Call<T>
     **/
    PalletSudoCall: {
        _enum: {
            sudo: {
                call: string;
            };
            sudo_unchecked_weight: {
                call: string;
                weight: string;
            };
            set_key: {
                _alias: {
                    new_: string;
                };
                new_: string;
            };
            sudo_as: {
                who: string;
                call: string;
            };
            remove_key: string;
        };
    };
    /**
     * Lookup262: pallet_ethereum::pallet::Call<T>
     **/
    PalletEthereumCall: {
        _enum: {
            transact: {
                transaction: string;
            };
        };
    };
    /**
     * Lookup263: ethereum::transaction::TransactionV2
     **/
    EthereumTransactionTransactionV2: {
        _enum: {
            Legacy: string;
            EIP2930: string;
            EIP1559: string;
        };
    };
    /**
     * Lookup264: ethereum::transaction::legacy::LegacyTransaction
     **/
    EthereumTransactionLegacyLegacyTransaction: {
        nonce: string;
        gasPrice: string;
        gasLimit: string;
        action: string;
        value: string;
        input: string;
        signature: string;
    };
    /**
     * Lookup267: ethereum::transaction::legacy::TransactionAction
     **/
    EthereumTransactionLegacyTransactionAction: {
        _enum: {
            Call: string;
            Create: string;
        };
    };
    /**
     * Lookup268: ethereum::transaction::legacy::TransactionSignature
     **/
    EthereumTransactionLegacyTransactionSignature: {
        v: string;
        r: string;
        s: string;
    };
    /**
     * Lookup270: ethereum::transaction::eip2930::EIP2930Transaction
     **/
    EthereumTransactionEip2930Eip2930Transaction: {
        chainId: string;
        nonce: string;
        gasPrice: string;
        gasLimit: string;
        action: string;
        value: string;
        input: string;
        accessList: string;
        oddYParity: string;
        r: string;
        s: string;
    };
    /**
     * Lookup272: ethereum::transaction::eip2930::AccessListItem
     **/
    EthereumTransactionEip2930AccessListItem: {
        address: string;
        storageKeys: string;
    };
    /**
     * Lookup273: ethereum::transaction::eip1559::EIP1559Transaction
     **/
    EthereumTransactionEip1559Eip1559Transaction: {
        chainId: string;
        nonce: string;
        maxPriorityFeePerGas: string;
        maxFeePerGas: string;
        gasLimit: string;
        action: string;
        value: string;
        input: string;
        accessList: string;
        oddYParity: string;
        r: string;
        s: string;
    };
    /**
     * Lookup274: pallet_evm::pallet::Call<T>
     **/
    PalletEvmCall: {
        _enum: {
            withdraw: {
                address: string;
                value: string;
            };
            call: {
                source: string;
                target: string;
                input: string;
                value: string;
                gasLimit: string;
                maxFeePerGas: string;
                maxPriorityFeePerGas: string;
                nonce: string;
                accessList: string;
            };
            create: {
                source: string;
                init: string;
                value: string;
                gasLimit: string;
                maxFeePerGas: string;
                maxPriorityFeePerGas: string;
                nonce: string;
                accessList: string;
            };
            create2: {
                source: string;
                init: string;
                salt: string;
                value: string;
                gasLimit: string;
                maxFeePerGas: string;
                maxPriorityFeePerGas: string;
                nonce: string;
                accessList: string;
            };
        };
    };
    /**
     * Lookup278: pallet_storage_providers::pallet::Call<T>
     **/
    PalletStorageProvidersCall: {
        _enum: {
            request_msp_sign_up: {
                capacity: string;
                multiaddresses: string;
                valuePropPricePerGigaUnitOfDataPerBlock: string;
                commitment: string;
                valuePropMaxDataLimit: string;
                paymentAccount: string;
            };
            request_bsp_sign_up: {
                capacity: string;
                multiaddresses: string;
                paymentAccount: string;
            };
            confirm_sign_up: {
                providerAccount: string;
            };
            cancel_sign_up: string;
            msp_sign_off: {
                mspId: string;
            };
            bsp_sign_off: string;
            change_capacity: {
                newCapacity: string;
            };
            add_value_prop: {
                pricePerGigaUnitOfDataPerBlock: string;
                commitment: string;
                bucketDataLimit: string;
            };
            make_value_prop_unavailable: {
                valuePropId: string;
            };
            add_multiaddress: {
                newMultiaddress: string;
            };
            remove_multiaddress: {
                multiaddress: string;
            };
            force_msp_sign_up: {
                who: string;
                mspId: string;
                capacity: string;
                multiaddresses: string;
                valuePropPricePerGigaUnitOfDataPerBlock: string;
                commitment: string;
                valuePropMaxDataLimit: string;
                paymentAccount: string;
            };
            force_bsp_sign_up: {
                who: string;
                bspId: string;
                capacity: string;
                multiaddresses: string;
                paymentAccount: string;
                weight: string;
            };
            slash: {
                providerId: string;
            };
            top_up_deposit: string;
            delete_provider: {
                providerId: string;
            };
            stop_all_cycles: string;
        };
    };
    /**
     * Lookup279: pallet_file_system::pallet::Call<T>
     **/
    PalletFileSystemCall: {
        _enum: {
            create_bucket: {
                mspId: string;
                name: string;
                private: string;
                valuePropId: string;
            };
            request_move_bucket: {
                bucketId: string;
                newMspId: string;
                newValuePropId: string;
            };
            msp_respond_move_bucket_request: {
                bucketId: string;
                response: string;
            };
            update_bucket_privacy: {
                bucketId: string;
                private: string;
            };
            create_and_associate_collection_with_bucket: {
                bucketId: string;
            };
            delete_bucket: {
                bucketId: string;
            };
            issue_storage_request: {
                _alias: {
                    size_: string;
                };
                bucketId: string;
                location: string;
                fingerprint: string;
                size_: string;
                mspId: string;
                peerIds: string;
                replicationTarget: string;
            };
            revoke_storage_request: {
                fileKey: string;
            };
            msp_respond_storage_requests_multiple_buckets: {
                storageRequestMspResponse: string;
            };
            msp_stop_storing_bucket: {
                bucketId: string;
            };
            bsp_volunteer: {
                fileKey: string;
            };
            bsp_confirm_storing: {
                nonInclusionForestProof: string;
                fileKeysAndProofs: string;
            };
            bsp_request_stop_storing: {
                _alias: {
                    size_: string;
                };
                fileKey: string;
                bucketId: string;
                location: string;
                owner: string;
                fingerprint: string;
                size_: string;
                canServe: string;
                inclusionForestProof: string;
            };
            bsp_confirm_stop_storing: {
                fileKey: string;
                inclusionForestProof: string;
            };
            stop_storing_for_insolvent_user: {
                _alias: {
                    size_: string;
                };
                fileKey: string;
                bucketId: string;
                location: string;
                owner: string;
                fingerprint: string;
                size_: string;
                inclusionForestProof: string;
            };
            msp_stop_storing_bucket_for_insolvent_user: {
                bucketId: string;
            };
            request_delete_file: {
                _alias: {
                    size_: string;
                };
                signedIntention: string;
                signature: string;
                bucketId: string;
                location: string;
                size_: string;
                fingerprint: string;
            };
            delete_files: {
                fileDeletions: string;
                bspId: string;
                forestProof: string;
            };
            delete_files_for_incomplete_storage_request: {
                fileKeys: string;
                bspId: string;
                forestProof: string;
            };
        };
    };
    /**
     * Lookup280: pallet_file_system::types::BucketMoveRequestResponse
     **/
    PalletFileSystemBucketMoveRequestResponse: {
        _enum: string[];
    };
    /**
     * Lookup281: pallet_file_system::types::ReplicationTarget<T>
     **/
    PalletFileSystemReplicationTarget: {
        _enum: {
            Basic: string;
            Standard: string;
            HighSecurity: string;
            SuperHighSecurity: string;
            UltraHighSecurity: string;
            Custom: string;
        };
    };
    /**
     * Lookup283: pallet_file_system::types::StorageRequestMspBucketResponse<T>
     **/
    PalletFileSystemStorageRequestMspBucketResponse: {
        bucketId: string;
        accept: string;
        reject: string;
    };
    /**
     * Lookup285: pallet_file_system::types::StorageRequestMspAcceptedFileKeys<T>
     **/
    PalletFileSystemStorageRequestMspAcceptedFileKeys: {
        fileKeysAndProofs: string;
        forestProof: string;
    };
    /**
     * Lookup287: pallet_file_system::types::FileKeyWithProof<T>
     **/
    PalletFileSystemFileKeyWithProof: {
        fileKey: string;
        proof: string;
    };
    /**
     * Lookup289: pallet_file_system::types::RejectedStorageRequest<T>
     **/
    PalletFileSystemRejectedStorageRequest: {
        fileKey: string;
        reason: string;
    };
    /**
     * Lookup292: pallet_file_system::types::FileDeletionRequest<T>
     **/
    PalletFileSystemFileDeletionRequest: {
        _alias: {
            size_: string;
        };
        fileOwner: string;
        signedIntention: string;
        signature: string;
        bucketId: string;
        location: string;
        size_: string;
        fingerprint: string;
    };
    /**
     * Lookup294: pallet_proofs_dealer::pallet::Call<T>
     **/
    PalletProofsDealerCall: {
        _enum: {
            challenge: {
                key: string;
            };
            submit_proof: {
                proof: string;
                provider: string;
            };
            force_initialise_challenge_cycle: {
                provider: string;
            };
            set_paused: {
                paused: string;
            };
            priority_challenge: {
                key: string;
                shouldRemoveKey: string;
            };
        };
    };
    /**
     * Lookup295: pallet_randomness::pallet::Call<T>
     **/
    PalletRandomnessCall: {
        _enum: string[];
    };
    /**
     * Lookup296: pallet_payment_streams::pallet::Call<T>
     **/
    PalletPaymentStreamsCall: {
        _enum: {
            create_fixed_rate_payment_stream: {
                providerId: string;
                userAccount: string;
                rate: string;
            };
            update_fixed_rate_payment_stream: {
                providerId: string;
                userAccount: string;
                newRate: string;
            };
            delete_fixed_rate_payment_stream: {
                providerId: string;
                userAccount: string;
            };
            create_dynamic_rate_payment_stream: {
                providerId: string;
                userAccount: string;
                amountProvided: string;
            };
            update_dynamic_rate_payment_stream: {
                providerId: string;
                userAccount: string;
                newAmountProvided: string;
            };
            delete_dynamic_rate_payment_stream: {
                providerId: string;
                userAccount: string;
            };
            charge_payment_streams: {
                userAccount: string;
            };
            charge_multiple_users_payment_streams: {
                userAccounts: string;
            };
            pay_outstanding_debt: {
                providers: string;
            };
            clear_insolvent_flag: string;
        };
    };
    /**
     * Lookup297: pallet_bucket_nfts::pallet::Call<T>
     **/
    PalletBucketNftsCall: {
        _enum: {
            share_access: {
                recipient: string;
                bucket: string;
                itemId: string;
                readAccessRegex: string;
            };
            update_read_access: {
                bucket: string;
                itemId: string;
                readAccessRegex: string;
            };
        };
    };
    /**
     * Lookup299: pallet_nfts::pallet::Call<T, I>
     **/
    PalletNftsCall: {
        _enum: {
            create: {
                admin: string;
                config: string;
            };
            force_create: {
                owner: string;
                config: string;
            };
            destroy: {
                collection: string;
                witness: string;
            };
            mint: {
                collection: string;
                item: string;
                mintTo: string;
                witnessData: string;
            };
            force_mint: {
                collection: string;
                item: string;
                mintTo: string;
                itemConfig: string;
            };
            burn: {
                collection: string;
                item: string;
            };
            transfer: {
                collection: string;
                item: string;
                dest: string;
            };
            redeposit: {
                collection: string;
                items: string;
            };
            lock_item_transfer: {
                collection: string;
                item: string;
            };
            unlock_item_transfer: {
                collection: string;
                item: string;
            };
            lock_collection: {
                collection: string;
                lockSettings: string;
            };
            transfer_ownership: {
                collection: string;
                newOwner: string;
            };
            set_team: {
                collection: string;
                issuer: string;
                admin: string;
                freezer: string;
            };
            force_collection_owner: {
                collection: string;
                owner: string;
            };
            force_collection_config: {
                collection: string;
                config: string;
            };
            approve_transfer: {
                collection: string;
                item: string;
                delegate: string;
                maybeDeadline: string;
            };
            cancel_approval: {
                collection: string;
                item: string;
                delegate: string;
            };
            clear_all_transfer_approvals: {
                collection: string;
                item: string;
            };
            lock_item_properties: {
                collection: string;
                item: string;
                lockMetadata: string;
                lockAttributes: string;
            };
            set_attribute: {
                collection: string;
                maybeItem: string;
                namespace: string;
                key: string;
                value: string;
            };
            force_set_attribute: {
                setAs: string;
                collection: string;
                maybeItem: string;
                namespace: string;
                key: string;
                value: string;
            };
            clear_attribute: {
                collection: string;
                maybeItem: string;
                namespace: string;
                key: string;
            };
            approve_item_attributes: {
                collection: string;
                item: string;
                delegate: string;
            };
            cancel_item_attributes_approval: {
                collection: string;
                item: string;
                delegate: string;
                witness: string;
            };
            set_metadata: {
                collection: string;
                item: string;
                data: string;
            };
            clear_metadata: {
                collection: string;
                item: string;
            };
            set_collection_metadata: {
                collection: string;
                data: string;
            };
            clear_collection_metadata: {
                collection: string;
            };
            set_accept_ownership: {
                maybeCollection: string;
            };
            set_collection_max_supply: {
                collection: string;
                maxSupply: string;
            };
            update_mint_settings: {
                collection: string;
                mintSettings: string;
            };
            set_price: {
                collection: string;
                item: string;
                price: string;
                whitelistedBuyer: string;
            };
            buy_item: {
                collection: string;
                item: string;
                bidPrice: string;
            };
            pay_tips: {
                tips: string;
            };
            create_swap: {
                offeredCollection: string;
                offeredItem: string;
                desiredCollection: string;
                maybeDesiredItem: string;
                maybePrice: string;
                duration: string;
            };
            cancel_swap: {
                offeredCollection: string;
                offeredItem: string;
            };
            claim_swap: {
                sendCollection: string;
                sendItem: string;
                receiveCollection: string;
                receiveItem: string;
                witnessPrice: string;
            };
            mint_pre_signed: {
                mintData: string;
                signature: string;
                signer: string;
            };
            set_attributes_pre_signed: {
                data: string;
                signature: string;
                signer: string;
            };
        };
    };
    /**
     * Lookup300: pallet_nfts::types::CollectionConfig<Price, BlockNumber, CollectionId>
     **/
    PalletNftsCollectionConfig: {
        settings: string;
        maxSupply: string;
        mintSettings: string;
    };
    /**
     * Lookup302: pallet_nfts::types::CollectionSetting
     **/
    PalletNftsCollectionSetting: {
        _enum: string[];
    };
    /**
     * Lookup303: pallet_nfts::types::MintSettings<Price, BlockNumber, CollectionId>
     **/
    PalletNftsMintSettings: {
        mintType: string;
        price: string;
        startBlock: string;
        endBlock: string;
        defaultItemSettings: string;
    };
    /**
     * Lookup304: pallet_nfts::types::MintType<CollectionId>
     **/
    PalletNftsMintType: {
        _enum: {
            Issuer: string;
            Public: string;
            HolderOf: string;
        };
    };
    /**
     * Lookup306: pallet_nfts::types::ItemSetting
     **/
    PalletNftsItemSetting: {
        _enum: string[];
    };
    /**
     * Lookup307: pallet_nfts::types::DestroyWitness
     **/
    PalletNftsDestroyWitness: {
        itemMetadatas: string;
        itemConfigs: string;
        attributes: string;
    };
    /**
     * Lookup309: pallet_nfts::types::MintWitness<ItemId, Balance>
     **/
    PalletNftsMintWitness: {
        ownedItem: string;
        mintPrice: string;
    };
    /**
     * Lookup310: pallet_nfts::types::ItemConfig
     **/
    PalletNftsItemConfig: {
        settings: string;
    };
    /**
     * Lookup311: pallet_nfts::types::CancelAttributesApprovalWitness
     **/
    PalletNftsCancelAttributesApprovalWitness: {
        accountAttributes: string;
    };
    /**
     * Lookup313: pallet_nfts::types::ItemTip<CollectionId, ItemId, fp_account::AccountId20, Amount>
     **/
    PalletNftsItemTip: {
        collection: string;
        item: string;
        receiver: string;
        amount: string;
    };
    /**
     * Lookup315: pallet_nfts::types::PreSignedMint<CollectionId, ItemId, fp_account::AccountId20, Deadline, Balance>
     **/
    PalletNftsPreSignedMint: {
        collection: string;
        item: string;
        attributes: string;
        metadata: string;
        onlyAccount: string;
        deadline: string;
        mintPrice: string;
    };
    /**
     * Lookup316: pallet_nfts::types::PreSignedAttributes<CollectionId, ItemId, fp_account::AccountId20, Deadline>
     **/
    PalletNftsPreSignedAttributes: {
        collection: string;
        item: string;
        attributes: string;
        namespace: string;
        deadline: string;
    };
    /**
     * Lookup317: pallet_sudo::pallet::Error<T>
     **/
    PalletSudoError: {
        _enum: string[];
    };
    /**
     * Lookup319: fp_rpc::TransactionStatus
     **/
    FpRpcTransactionStatus: {
        transactionHash: string;
        transactionIndex: string;
        from: string;
        to: string;
        contractAddress: string;
        logs: string;
        logsBloom: string;
    };
    /**
     * Lookup322: ethbloom::Bloom
     **/
    EthbloomBloom: string;
    /**
     * Lookup324: ethereum::receipt::ReceiptV3
     **/
    EthereumReceiptReceiptV3: {
        _enum: {
            Legacy: string;
            EIP2930: string;
            EIP1559: string;
        };
    };
    /**
     * Lookup325: ethereum::receipt::EIP658ReceiptData
     **/
    EthereumReceiptEip658ReceiptData: {
        statusCode: string;
        usedGas: string;
        logsBloom: string;
        logs: string;
    };
    /**
     * Lookup326: ethereum::block::Block<ethereum::transaction::TransactionV2>
     **/
    EthereumBlock: {
        header: string;
        transactions: string;
        ommers: string;
    };
    /**
     * Lookup327: ethereum::header::Header
     **/
    EthereumHeader: {
        parentHash: string;
        ommersHash: string;
        beneficiary: string;
        stateRoot: string;
        transactionsRoot: string;
        receiptsRoot: string;
        logsBloom: string;
        difficulty: string;
        number: string;
        gasLimit: string;
        gasUsed: string;
        timestamp: string;
        extraData: string;
        mixHash: string;
        nonce: string;
    };
    /**
     * Lookup328: ethereum_types::hash::H64
     **/
    EthereumTypesHashH64: string;
    /**
     * Lookup333: pallet_ethereum::pallet::Error<T>
     **/
    PalletEthereumError: {
        _enum: string[];
    };
    /**
     * Lookup334: pallet_evm::CodeMetadata
     **/
    PalletEvmCodeMetadata: {
        _alias: {
            size_: string;
            hash_: string;
        };
        size_: string;
        hash_: string;
    };
    /**
     * Lookup336: pallet_evm::pallet::Error<T>
     **/
    PalletEvmError: {
        _enum: string[];
    };
    /**
     * Lookup337: pallet_storage_providers::types::SignUpRequest<T>
     **/
    PalletStorageProvidersSignUpRequest: {
        spSignUpRequest: string;
        at: string;
    };
    /**
     * Lookup338: pallet_storage_providers::types::SignUpRequestSpParams<T>
     **/
    PalletStorageProvidersSignUpRequestSpParams: {
        _enum: {
            BackupStorageProvider: string;
            MainStorageProvider: string;
        };
    };
    /**
     * Lookup339: pallet_storage_providers::types::BackupStorageProvider<T>
     **/
    PalletStorageProvidersBackupStorageProvider: {
        capacity: string;
        capacityUsed: string;
        multiaddresses: string;
        root: string;
        lastCapacityChange: string;
        ownerAccount: string;
        paymentAccount: string;
        reputationWeight: string;
        signUpBlock: string;
    };
    /**
     * Lookup340: pallet_storage_providers::types::MainStorageProviderSignUpRequest<T>
     **/
    PalletStorageProvidersMainStorageProviderSignUpRequest: {
        mspInfo: string;
        valueProp: string;
    };
    /**
     * Lookup341: pallet_storage_providers::types::MainStorageProvider<T>
     **/
    PalletStorageProvidersMainStorageProvider: {
        capacity: string;
        capacityUsed: string;
        multiaddresses: string;
        amountOfBuckets: string;
        amountOfValueProps: string;
        lastCapacityChange: string;
        ownerAccount: string;
        paymentAccount: string;
        signUpBlock: string;
    };
    /**
     * Lookup342: pallet_storage_providers::types::Bucket<T>
     **/
    PalletStorageProvidersBucket: {
        _alias: {
            size_: string;
        };
        root: string;
        userId: string;
        mspId: string;
        private: string;
        readAccessGroupId: string;
        size_: string;
        valuePropId: string;
    };
    /**
     * Lookup346: pallet_storage_providers::pallet::Error<T>
     **/
    PalletStorageProvidersError: {
        _enum: string[];
    };
    /**
     * Lookup347: pallet_file_system::types::StorageRequestMetadata<T>
     **/
    PalletFileSystemStorageRequestMetadata: {
        _alias: {
            size_: string;
        };
        requestedAt: string;
        expiresAt: string;
        owner: string;
        bucketId: string;
        location: string;
        fingerprint: string;
        size_: string;
        mspStatus: string;
        userPeerIds: string;
        bspsRequired: string;
        bspsConfirmed: string;
        bspsVolunteered: string;
        depositPaid: string;
    };
    /**
     * Lookup348: pallet_file_system::types::MspStorageRequestStatus<T>
     **/
    PalletFileSystemMspStorageRequestStatus: {
        _enum: {
            None: string;
            Pending: string;
            AcceptedNewFile: string;
            AcceptedExistingFile: string;
        };
    };
    /**
     * Lookup349: pallet_file_system::types::StorageRequestBspsMetadata<T>
     **/
    PalletFileSystemStorageRequestBspsMetadata: {
        confirmed: string;
    };
    /**
     * Lookup351: pallet_file_system::types::PendingFileDeletionRequest<T>
     **/
    PalletFileSystemPendingFileDeletionRequest: {
        user: string;
        fileKey: string;
        bucketId: string;
        fileSize: string;
        depositPaidForCreation: string;
        queuePriorityChallenge: string;
    };
    /**
     * Lookup353: pallet_file_system::types::PendingStopStoringRequest<T>
     **/
    PalletFileSystemPendingStopStoringRequest: {
        tickWhenRequested: string;
        fileOwner: string;
        fileSize: string;
    };
    /**
     * Lookup354: pallet_file_system::types::MoveBucketRequestMetadata<T>
     **/
    PalletFileSystemMoveBucketRequestMetadata: {
        requester: string;
        newMspId: string;
        newValuePropId: string;
    };
    /**
     * Lookup355: pallet_file_system::types::IncompleteStorageRequestMetadata<T>
     **/
    PalletFileSystemIncompleteStorageRequestMetadata: {
        owner: string;
        bucketId: string;
        location: string;
        fileSize: string;
        fingerprint: string;
        pendingBspRemovals: string;
        pendingBucketRemoval: string;
    };
    /**
     * Lookup357: pallet_file_system::pallet::Error<T>
     **/
    PalletFileSystemError: {
        _enum: string[];
    };
    /**
     * Lookup359: pallet_proofs_dealer::types::ProofSubmissionRecord<T>
     **/
    PalletProofsDealerProofSubmissionRecord: {
        lastTickProven: string;
        nextTickToSubmitProofFor: string;
    };
    /**
     * Lookup366: pallet_proofs_dealer::pallet::Error<T>
     **/
    PalletProofsDealerError: {
        _enum: string[];
    };
    /**
     * Lookup368: pallet_payment_streams::types::FixedRatePaymentStream<T>
     **/
    PalletPaymentStreamsFixedRatePaymentStream: {
        rate: string;
        lastChargedTick: string;
        userDeposit: string;
        outOfFundsTick: string;
    };
    /**
     * Lookup369: pallet_payment_streams::types::DynamicRatePaymentStream<T>
     **/
    PalletPaymentStreamsDynamicRatePaymentStream: {
        amountProvided: string;
        priceIndexWhenLastCharged: string;
        userDeposit: string;
        outOfFundsTick: string;
    };
    /**
     * Lookup370: pallet_payment_streams::types::ProviderLastChargeableInfo<T>
     **/
    PalletPaymentStreamsProviderLastChargeableInfo: {
        lastChargeableTick: string;
        priceIndex: string;
    };
    /**
     * Lookup371: pallet_payment_streams::pallet::Error<T>
     **/
    PalletPaymentStreamsError: {
        _enum: string[];
    };
    /**
     * Lookup372: pallet_bucket_nfts::pallet::Error<T>
     **/
    PalletBucketNftsError: {
        _enum: string[];
    };
    /**
     * Lookup373: pallet_nfts::types::CollectionDetails<fp_account::AccountId20, DepositBalance>
     **/
    PalletNftsCollectionDetails: {
        owner: string;
        ownerDeposit: string;
        items: string;
        itemMetadatas: string;
        itemConfigs: string;
        attributes: string;
    };
    /**
     * Lookup378: pallet_nfts::types::CollectionRole
     **/
    PalletNftsCollectionRole: {
        _enum: string[];
    };
    /**
     * Lookup379: pallet_nfts::types::ItemDetails<fp_account::AccountId20, pallet_nfts::types::ItemDeposit<DepositBalance, fp_account::AccountId20>, bounded_collections::bounded_btree_map::BoundedBTreeMap<fp_account::AccountId20, Option<T>, S>>
     **/
    PalletNftsItemDetails: {
        owner: string;
        approvals: string;
        deposit: string;
    };
    /**
     * Lookup380: pallet_nfts::types::ItemDeposit<DepositBalance, fp_account::AccountId20>
     **/
    PalletNftsItemDeposit: {
        account: string;
        amount: string;
    };
    /**
     * Lookup385: pallet_nfts::types::CollectionMetadata<Deposit, StringLimit>
     **/
    PalletNftsCollectionMetadata: {
        deposit: string;
        data: string;
    };
    /**
     * Lookup386: pallet_nfts::types::ItemMetadata<pallet_nfts::types::ItemMetadataDeposit<DepositBalance, fp_account::AccountId20>, StringLimit>
     **/
    PalletNftsItemMetadata: {
        deposit: string;
        data: string;
    };
    /**
     * Lookup387: pallet_nfts::types::ItemMetadataDeposit<DepositBalance, fp_account::AccountId20>
     **/
    PalletNftsItemMetadataDeposit: {
        account: string;
        amount: string;
    };
    /**
     * Lookup390: pallet_nfts::types::AttributeDeposit<DepositBalance, fp_account::AccountId20>
     **/
    PalletNftsAttributeDeposit: {
        account: string;
        amount: string;
    };
    /**
     * Lookup394: pallet_nfts::types::PendingSwap<CollectionId, ItemId, pallet_nfts::types::PriceWithDirection<Amount>, Deadline>
     **/
    PalletNftsPendingSwap: {
        desiredCollection: string;
        desiredItem: string;
        price: string;
        deadline: string;
    };
    /**
     * Lookup396: pallet_nfts::types::PalletFeature
     **/
    PalletNftsPalletFeature: {
        _enum: string[];
    };
    /**
     * Lookup397: pallet_nfts::pallet::Error<T, I>
     **/
    PalletNftsError: {
        _enum: string[];
    };
    /**
     * Lookup400: frame_system::extensions::check_non_zero_sender::CheckNonZeroSender<T>
     **/
    FrameSystemExtensionsCheckNonZeroSender: string;
    /**
     * Lookup401: frame_system::extensions::check_spec_version::CheckSpecVersion<T>
     **/
    FrameSystemExtensionsCheckSpecVersion: string;
    /**
     * Lookup402: frame_system::extensions::check_tx_version::CheckTxVersion<T>
     **/
    FrameSystemExtensionsCheckTxVersion: string;
    /**
     * Lookup403: frame_system::extensions::check_genesis::CheckGenesis<T>
     **/
    FrameSystemExtensionsCheckGenesis: string;
    /**
     * Lookup406: frame_system::extensions::check_nonce::CheckNonce<T>
     **/
    FrameSystemExtensionsCheckNonce: string;
    /**
     * Lookup407: frame_system::extensions::check_weight::CheckWeight<T>
     **/
    FrameSystemExtensionsCheckWeight: string;
    /**
     * Lookup408: pallet_transaction_payment::ChargeTransactionPayment<T>
     **/
    PalletTransactionPaymentChargeTransactionPayment: string;
    /**
     * Lookup409: frame_metadata_hash_extension::CheckMetadataHash<T>
     **/
    FrameMetadataHashExtensionCheckMetadataHash: {
        mode: string;
    };
    /**
     * Lookup410: frame_metadata_hash_extension::Mode
     **/
    FrameMetadataHashExtensionMode: {
        _enum: string[];
    };
    /**
     * Lookup412: sh_solochain_evm_runtime::Runtime
     **/
    ShSolochainEvmRuntimeRuntime: string;
};
export default _default;
