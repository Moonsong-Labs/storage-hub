// Auto-generated via `yarn polkadot-types-from-defs`, do not edit
/* eslint-disable */
/* eslint-disable sort-keys */
export default {
    /**
     * Lookup3: frame_system::AccountInfo<Nonce, pallet_balances::types::AccountData<Balance>>
     **/
    FrameSystemAccountInfo: {
        nonce: 'u32',
        consumers: 'u32',
        providers: 'u32',
        sufficients: 'u32',
        data: 'PalletBalancesAccountData'
    },
    /**
     * Lookup5: pallet_balances::types::AccountData<Balance>
     **/
    PalletBalancesAccountData: {
        free: 'u128',
        reserved: 'u128',
        frozen: 'u128',
        flags: 'u128'
    },
    /**
     * Lookup9: frame_support::dispatch::PerDispatchClass<sp_weights::weight_v2::Weight>
     **/
    FrameSupportDispatchPerDispatchClassWeight: {
        normal: 'SpWeightsWeightV2Weight',
        operational: 'SpWeightsWeightV2Weight',
        mandatory: 'SpWeightsWeightV2Weight'
    },
    /**
     * Lookup10: sp_weights::weight_v2::Weight
     **/
    SpWeightsWeightV2Weight: {
        refTime: 'Compact<u64>',
        proofSize: 'Compact<u64>'
    },
    /**
     * Lookup16: sp_runtime::generic::digest::Digest
     **/
    SpRuntimeDigest: {
        logs: 'Vec<SpRuntimeDigestDigestItem>'
    },
    /**
     * Lookup18: sp_runtime::generic::digest::DigestItem
     **/
    SpRuntimeDigestDigestItem: {
        _enum: {
            Other: 'Bytes',
            __Unused1: 'Null',
            __Unused2: 'Null',
            __Unused3: 'Null',
            Consensus: '([u8;4],Bytes)',
            Seal: '([u8;4],Bytes)',
            PreRuntime: '([u8;4],Bytes)',
            __Unused7: 'Null',
            RuntimeEnvironmentUpdated: 'Null'
        }
    },
    /**
     * Lookup21: frame_system::EventRecord<sh_solochain_evm_runtime::RuntimeEvent, primitive_types::H256>
     **/
    FrameSystemEventRecord: {
        phase: 'FrameSystemPhase',
        event: 'Event',
        topics: 'Vec<H256>'
    },
    /**
     * Lookup23: frame_system::pallet::Event<T>
     **/
    FrameSystemEvent: {
        _enum: {
            ExtrinsicSuccess: {
                dispatchInfo: 'FrameSystemDispatchEventInfo',
            },
            ExtrinsicFailed: {
                dispatchError: 'SpRuntimeDispatchError',
                dispatchInfo: 'FrameSystemDispatchEventInfo',
            },
            CodeUpdated: 'Null',
            NewAccount: {
                account: 'AccountId20',
            },
            KilledAccount: {
                account: 'AccountId20',
            },
            Remarked: {
                _alias: {
                    hash_: 'hash',
                },
                sender: 'AccountId20',
                hash_: 'H256',
            },
            UpgradeAuthorized: {
                codeHash: 'H256',
                checkVersion: 'bool'
            }
        }
    },
    /**
     * Lookup24: frame_system::DispatchEventInfo
     **/
    FrameSystemDispatchEventInfo: {
        weight: 'SpWeightsWeightV2Weight',
        class: 'FrameSupportDispatchDispatchClass',
        paysFee: 'FrameSupportDispatchPays'
    },
    /**
     * Lookup25: frame_support::dispatch::DispatchClass
     **/
    FrameSupportDispatchDispatchClass: {
        _enum: ['Normal', 'Operational', 'Mandatory']
    },
    /**
     * Lookup26: frame_support::dispatch::Pays
     **/
    FrameSupportDispatchPays: {
        _enum: ['Yes', 'No']
    },
    /**
     * Lookup27: sp_runtime::DispatchError
     **/
    SpRuntimeDispatchError: {
        _enum: {
            Other: 'Null',
            CannotLookup: 'Null',
            BadOrigin: 'Null',
            Module: 'SpRuntimeModuleError',
            ConsumerRemaining: 'Null',
            NoProviders: 'Null',
            TooManyConsumers: 'Null',
            Token: 'SpRuntimeTokenError',
            Arithmetic: 'SpArithmeticArithmeticError',
            Transactional: 'SpRuntimeTransactionalError',
            Exhausted: 'Null',
            Corruption: 'Null',
            Unavailable: 'Null',
            RootNotAllowed: 'Null',
            Trie: 'SpRuntimeProvingTrieTrieError'
        }
    },
    /**
     * Lookup28: sp_runtime::ModuleError
     **/
    SpRuntimeModuleError: {
        index: 'u8',
        error: '[u8;4]'
    },
    /**
     * Lookup29: sp_runtime::TokenError
     **/
    SpRuntimeTokenError: {
        _enum: ['FundsUnavailable', 'OnlyProvider', 'BelowMinimum', 'CannotCreate', 'UnknownAsset', 'Frozen', 'Unsupported', 'CannotCreateHold', 'NotExpendable', 'Blocked']
    },
    /**
     * Lookup30: sp_arithmetic::ArithmeticError
     **/
    SpArithmeticArithmeticError: {
        _enum: ['Underflow', 'Overflow', 'DivisionByZero']
    },
    /**
     * Lookup31: sp_runtime::TransactionalError
     **/
    SpRuntimeTransactionalError: {
        _enum: ['LimitReached', 'NoLayer']
    },
    /**
     * Lookup32: sp_runtime::proving_trie::TrieError
     **/
    SpRuntimeProvingTrieTrieError: {
        _enum: ['InvalidStateRoot', 'IncompleteDatabase', 'ValueAtIncompleteKey', 'DecoderError', 'InvalidHash', 'DuplicateKey', 'ExtraneousNode', 'ExtraneousValue', 'ExtraneousHashReference', 'InvalidChildReference', 'ValueMismatch', 'IncompleteProof', 'RootMismatch', 'DecodeError']
    },
    /**
     * Lookup33: pallet_balances::pallet::Event<T, I>
     **/
    PalletBalancesEvent: {
        _enum: {
            Endowed: {
                account: 'AccountId20',
                freeBalance: 'u128',
            },
            DustLost: {
                account: 'AccountId20',
                amount: 'u128',
            },
            Transfer: {
                from: 'AccountId20',
                to: 'AccountId20',
                amount: 'u128',
            },
            BalanceSet: {
                who: 'AccountId20',
                free: 'u128',
            },
            Reserved: {
                who: 'AccountId20',
                amount: 'u128',
            },
            Unreserved: {
                who: 'AccountId20',
                amount: 'u128',
            },
            ReserveRepatriated: {
                from: 'AccountId20',
                to: 'AccountId20',
                amount: 'u128',
                destinationStatus: 'FrameSupportTokensMiscBalanceStatus',
            },
            Deposit: {
                who: 'AccountId20',
                amount: 'u128',
            },
            Withdraw: {
                who: 'AccountId20',
                amount: 'u128',
            },
            Slashed: {
                who: 'AccountId20',
                amount: 'u128',
            },
            Minted: {
                who: 'AccountId20',
                amount: 'u128',
            },
            Burned: {
                who: 'AccountId20',
                amount: 'u128',
            },
            Suspended: {
                who: 'AccountId20',
                amount: 'u128',
            },
            Restored: {
                who: 'AccountId20',
                amount: 'u128',
            },
            Upgraded: {
                who: 'AccountId20',
            },
            Issued: {
                amount: 'u128',
            },
            Rescinded: {
                amount: 'u128',
            },
            Locked: {
                who: 'AccountId20',
                amount: 'u128',
            },
            Unlocked: {
                who: 'AccountId20',
                amount: 'u128',
            },
            Frozen: {
                who: 'AccountId20',
                amount: 'u128',
            },
            Thawed: {
                who: 'AccountId20',
                amount: 'u128',
            },
            TotalIssuanceForced: {
                _alias: {
                    new_: 'new',
                },
                old: 'u128',
                new_: 'u128'
            }
        }
    },
    /**
     * Lookup34: frame_support::traits::tokens::misc::BalanceStatus
     **/
    FrameSupportTokensMiscBalanceStatus: {
        _enum: ['Free', 'Reserved']
    },
    /**
     * Lookup35: pallet_offences::pallet::Event
     **/
    PalletOffencesEvent: {
        _enum: {
            Offence: {
                kind: '[u8;16]',
                timeslot: 'Bytes'
            }
        }
    },
    /**
     * Lookup37: pallet_session::pallet::Event
     **/
    PalletSessionEvent: {
        _enum: {
            NewSession: {
                sessionIndex: 'u32'
            }
        }
    },
    /**
     * Lookup38: pallet_grandpa::pallet::Event
     **/
    PalletGrandpaEvent: {
        _enum: {
            NewAuthorities: {
                authoritySet: 'Vec<(SpConsensusGrandpaAppPublic,u64)>',
            },
            Paused: 'Null',
            Resumed: 'Null'
        }
    },
    /**
     * Lookup41: sp_consensus_grandpa::app::Public
     **/
    SpConsensusGrandpaAppPublic: '[u8;32]',
    /**
     * Lookup42: pallet_transaction_payment::pallet::Event<T>
     **/
    PalletTransactionPaymentEvent: {
        _enum: {
            TransactionFeePaid: {
                who: 'AccountId20',
                actualFee: 'u128',
                tip: 'u128'
            }
        }
    },
    /**
     * Lookup43: pallet_parameters::pallet::Event<T>
     **/
    PalletParametersEvent: {
        _enum: {
            Updated: {
                key: 'ShSolochainEvmRuntimeConfigsRuntimeParamsRuntimeParametersKey',
                oldValue: 'Option<ShSolochainEvmRuntimeConfigsRuntimeParamsRuntimeParametersValue>',
                newValue: 'Option<ShSolochainEvmRuntimeConfigsRuntimeParamsRuntimeParametersValue>'
            }
        }
    },
    /**
     * Lookup44: sh_solochain_evm_runtime::configs::runtime_params::RuntimeParametersKey
     **/
    ShSolochainEvmRuntimeConfigsRuntimeParamsRuntimeParametersKey: {
        _enum: {
            RuntimeConfig: 'ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigParametersKey'
        }
    },
    /**
     * Lookup45: sh_solochain_evm_runtime::configs::runtime_params::dynamic_params::runtime_config::ParametersKey
     **/
    ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigParametersKey: {
        _enum: ['SlashAmountPerMaxFileSize', 'StakeToChallengePeriod', 'CheckpointChallengePeriod', 'MinChallengePeriod', 'SystemUtilisationLowerThresholdPercentage', 'SystemUtilisationUpperThresholdPercentage', 'MostlyStablePrice', 'MaxPrice', 'MinPrice', 'UpperExponentFactor', 'LowerExponentFactor', 'ZeroSizeBucketFixedRate', 'IdealUtilisationRate', 'DecayRate', 'MinimumTreasuryCut', 'MaximumTreasuryCut', 'BspStopStoringFilePenalty', 'ProviderTopUpTtl', 'BasicReplicationTarget', 'StandardReplicationTarget', 'HighSecurityReplicationTarget', 'SuperHighSecurityReplicationTarget', 'UltraHighSecurityReplicationTarget', 'MaxReplicationTarget', 'TickRangeToMaximumThreshold', 'StorageRequestTtl', 'MinWaitForStopStoring', 'MinSeedPeriod', 'StakeToSeedPeriod', 'UpfrontTicksToPay']
    },
    /**
     * Lookup46: sh_solochain_evm_runtime::configs::runtime_params::dynamic_params::runtime_config::SlashAmountPerMaxFileSize
     **/
    ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigSlashAmountPerMaxFileSize: 'Null',
    /**
     * Lookup47: sh_solochain_evm_runtime::configs::runtime_params::dynamic_params::runtime_config::StakeToChallengePeriod
     **/
    ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigStakeToChallengePeriod: 'Null',
    /**
     * Lookup48: sh_solochain_evm_runtime::configs::runtime_params::dynamic_params::runtime_config::CheckpointChallengePeriod
     **/
    ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigCheckpointChallengePeriod: 'Null',
    /**
     * Lookup49: sh_solochain_evm_runtime::configs::runtime_params::dynamic_params::runtime_config::MinChallengePeriod
     **/
    ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMinChallengePeriod: 'Null',
    /**
     * Lookup50: sh_solochain_evm_runtime::configs::runtime_params::dynamic_params::runtime_config::SystemUtilisationLowerThresholdPercentage
     **/
    ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigSystemUtilisationLowerThresholdPercentage: 'Null',
    /**
     * Lookup51: sh_solochain_evm_runtime::configs::runtime_params::dynamic_params::runtime_config::SystemUtilisationUpperThresholdPercentage
     **/
    ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigSystemUtilisationUpperThresholdPercentage: 'Null',
    /**
     * Lookup52: sh_solochain_evm_runtime::configs::runtime_params::dynamic_params::runtime_config::MostlyStablePrice
     **/
    ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMostlyStablePrice: 'Null',
    /**
     * Lookup53: sh_solochain_evm_runtime::configs::runtime_params::dynamic_params::runtime_config::MaxPrice
     **/
    ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMaxPrice: 'Null',
    /**
     * Lookup54: sh_solochain_evm_runtime::configs::runtime_params::dynamic_params::runtime_config::MinPrice
     **/
    ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMinPrice: 'Null',
    /**
     * Lookup55: sh_solochain_evm_runtime::configs::runtime_params::dynamic_params::runtime_config::UpperExponentFactor
     **/
    ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigUpperExponentFactor: 'Null',
    /**
     * Lookup56: sh_solochain_evm_runtime::configs::runtime_params::dynamic_params::runtime_config::LowerExponentFactor
     **/
    ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigLowerExponentFactor: 'Null',
    /**
     * Lookup57: sh_solochain_evm_runtime::configs::runtime_params::dynamic_params::runtime_config::ZeroSizeBucketFixedRate
     **/
    ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigZeroSizeBucketFixedRate: 'Null',
    /**
     * Lookup58: sh_solochain_evm_runtime::configs::runtime_params::dynamic_params::runtime_config::IdealUtilisationRate
     **/
    ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigIdealUtilisationRate: 'Null',
    /**
     * Lookup59: sh_solochain_evm_runtime::configs::runtime_params::dynamic_params::runtime_config::DecayRate
     **/
    ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigDecayRate: 'Null',
    /**
     * Lookup60: sh_solochain_evm_runtime::configs::runtime_params::dynamic_params::runtime_config::MinimumTreasuryCut
     **/
    ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMinimumTreasuryCut: 'Null',
    /**
     * Lookup61: sh_solochain_evm_runtime::configs::runtime_params::dynamic_params::runtime_config::MaximumTreasuryCut
     **/
    ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMaximumTreasuryCut: 'Null',
    /**
     * Lookup62: sh_solochain_evm_runtime::configs::runtime_params::dynamic_params::runtime_config::BspStopStoringFilePenalty
     **/
    ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigBspStopStoringFilePenalty: 'Null',
    /**
     * Lookup63: sh_solochain_evm_runtime::configs::runtime_params::dynamic_params::runtime_config::ProviderTopUpTtl
     **/
    ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigProviderTopUpTtl: 'Null',
    /**
     * Lookup64: sh_solochain_evm_runtime::configs::runtime_params::dynamic_params::runtime_config::BasicReplicationTarget
     **/
    ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigBasicReplicationTarget: 'Null',
    /**
     * Lookup65: sh_solochain_evm_runtime::configs::runtime_params::dynamic_params::runtime_config::StandardReplicationTarget
     **/
    ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigStandardReplicationTarget: 'Null',
    /**
     * Lookup66: sh_solochain_evm_runtime::configs::runtime_params::dynamic_params::runtime_config::HighSecurityReplicationTarget
     **/
    ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigHighSecurityReplicationTarget: 'Null',
    /**
     * Lookup67: sh_solochain_evm_runtime::configs::runtime_params::dynamic_params::runtime_config::SuperHighSecurityReplicationTarget
     **/
    ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigSuperHighSecurityReplicationTarget: 'Null',
    /**
     * Lookup68: sh_solochain_evm_runtime::configs::runtime_params::dynamic_params::runtime_config::UltraHighSecurityReplicationTarget
     **/
    ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigUltraHighSecurityReplicationTarget: 'Null',
    /**
     * Lookup69: sh_solochain_evm_runtime::configs::runtime_params::dynamic_params::runtime_config::MaxReplicationTarget
     **/
    ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMaxReplicationTarget: 'Null',
    /**
     * Lookup70: sh_solochain_evm_runtime::configs::runtime_params::dynamic_params::runtime_config::TickRangeToMaximumThreshold
     **/
    ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigTickRangeToMaximumThreshold: 'Null',
    /**
     * Lookup71: sh_solochain_evm_runtime::configs::runtime_params::dynamic_params::runtime_config::StorageRequestTtl
     **/
    ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigStorageRequestTtl: 'Null',
    /**
     * Lookup72: sh_solochain_evm_runtime::configs::runtime_params::dynamic_params::runtime_config::MinWaitForStopStoring
     **/
    ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMinWaitForStopStoring: 'Null',
    /**
     * Lookup73: sh_solochain_evm_runtime::configs::runtime_params::dynamic_params::runtime_config::MinSeedPeriod
     **/
    ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMinSeedPeriod: 'Null',
    /**
     * Lookup74: sh_solochain_evm_runtime::configs::runtime_params::dynamic_params::runtime_config::StakeToSeedPeriod
     **/
    ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigStakeToSeedPeriod: 'Null',
    /**
     * Lookup75: sh_solochain_evm_runtime::configs::runtime_params::dynamic_params::runtime_config::UpfrontTicksToPay
     **/
    ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigUpfrontTicksToPay: 'Null',
    /**
     * Lookup77: sh_solochain_evm_runtime::configs::runtime_params::RuntimeParametersValue
     **/
    ShSolochainEvmRuntimeConfigsRuntimeParamsRuntimeParametersValue: {
        _enum: {
            RuntimeConfig: 'ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigParametersValue'
        }
    },
    /**
     * Lookup78: sh_solochain_evm_runtime::configs::runtime_params::dynamic_params::runtime_config::ParametersValue
     **/
    ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigParametersValue: {
        _enum: {
            SlashAmountPerMaxFileSize: 'u128',
            StakeToChallengePeriod: 'u128',
            CheckpointChallengePeriod: 'u32',
            MinChallengePeriod: 'u32',
            SystemUtilisationLowerThresholdPercentage: 'Perbill',
            SystemUtilisationUpperThresholdPercentage: 'Perbill',
            MostlyStablePrice: 'u128',
            MaxPrice: 'u128',
            MinPrice: 'u128',
            UpperExponentFactor: 'u32',
            LowerExponentFactor: 'u32',
            ZeroSizeBucketFixedRate: 'u128',
            IdealUtilisationRate: 'Perbill',
            DecayRate: 'Perbill',
            MinimumTreasuryCut: 'Perbill',
            MaximumTreasuryCut: 'Perbill',
            BspStopStoringFilePenalty: 'u128',
            ProviderTopUpTtl: 'u32',
            BasicReplicationTarget: 'u32',
            StandardReplicationTarget: 'u32',
            HighSecurityReplicationTarget: 'u32',
            SuperHighSecurityReplicationTarget: 'u32',
            UltraHighSecurityReplicationTarget: 'u32',
            MaxReplicationTarget: 'u32',
            TickRangeToMaximumThreshold: 'u32',
            StorageRequestTtl: 'u32',
            MinWaitForStopStoring: 'u32',
            MinSeedPeriod: 'u32',
            StakeToSeedPeriod: 'u128',
            UpfrontTicksToPay: 'u32'
        }
    },
    /**
     * Lookup80: pallet_sudo::pallet::Event<T>
     **/
    PalletSudoEvent: {
        _enum: {
            Sudid: {
                sudoResult: 'Result<Null, SpRuntimeDispatchError>',
            },
            KeyChanged: {
                _alias: {
                    new_: 'new',
                },
                old: 'Option<AccountId20>',
                new_: 'AccountId20',
            },
            KeyRemoved: 'Null',
            SudoAsDone: {
                sudoResult: 'Result<Null, SpRuntimeDispatchError>'
            }
        }
    },
    /**
     * Lookup84: pallet_ethereum::pallet::Event
     **/
    PalletEthereumEvent: {
        _enum: {
            Executed: {
                from: 'H160',
                to: 'H160',
                transactionHash: 'H256',
                exitReason: 'EvmCoreErrorExitReason',
                extraData: 'Bytes'
            }
        }
    },
    /**
     * Lookup86: evm_core::error::ExitReason
     **/
    EvmCoreErrorExitReason: {
        _enum: {
            Succeed: 'EvmCoreErrorExitSucceed',
            Error: 'EvmCoreErrorExitError',
            Revert: 'EvmCoreErrorExitRevert',
            Fatal: 'EvmCoreErrorExitFatal'
        }
    },
    /**
     * Lookup87: evm_core::error::ExitSucceed
     **/
    EvmCoreErrorExitSucceed: {
        _enum: ['Stopped', 'Returned', 'Suicided']
    },
    /**
     * Lookup88: evm_core::error::ExitError
     **/
    EvmCoreErrorExitError: {
        _enum: {
            StackUnderflow: 'Null',
            StackOverflow: 'Null',
            InvalidJump: 'Null',
            InvalidRange: 'Null',
            DesignatedInvalid: 'Null',
            CallTooDeep: 'Null',
            CreateCollision: 'Null',
            CreateContractLimit: 'Null',
            OutOfOffset: 'Null',
            OutOfGas: 'Null',
            OutOfFund: 'Null',
            PCUnderflow: 'Null',
            CreateEmpty: 'Null',
            Other: 'Text',
            MaxNonce: 'Null',
            InvalidCode: 'u8'
        }
    },
    /**
     * Lookup92: evm_core::error::ExitRevert
     **/
    EvmCoreErrorExitRevert: {
        _enum: ['Reverted']
    },
    /**
     * Lookup93: evm_core::error::ExitFatal
     **/
    EvmCoreErrorExitFatal: {
        _enum: {
            NotSupported: 'Null',
            UnhandledInterrupt: 'Null',
            CallErrorAsFatal: 'EvmCoreErrorExitError',
            Other: 'Text'
        }
    },
    /**
     * Lookup94: pallet_evm::pallet::Event<T>
     **/
    PalletEvmEvent: {
        _enum: {
            Log: {
                log: 'EthereumLog',
            },
            Created: {
                address: 'H160',
            },
            CreatedFailed: {
                address: 'H160',
            },
            Executed: {
                address: 'H160',
            },
            ExecutedFailed: {
                address: 'H160'
            }
        }
    },
    /**
     * Lookup95: ethereum::log::Log
     **/
    EthereumLog: {
        address: 'H160',
        topics: 'Vec<H256>',
        data: 'Bytes'
    },
    /**
     * Lookup97: pallet_storage_providers::pallet::Event<T>
     **/
    PalletStorageProvidersEvent: {
        _enum: {
            MspRequestSignUpSuccess: {
                who: 'AccountId20',
                multiaddresses: 'Vec<Bytes>',
                capacity: 'u64',
            },
            MspSignUpSuccess: {
                who: 'AccountId20',
                mspId: 'H256',
                multiaddresses: 'Vec<Bytes>',
                capacity: 'u64',
                valueProp: 'PalletStorageProvidersValuePropositionWithId',
            },
            BspRequestSignUpSuccess: {
                who: 'AccountId20',
                multiaddresses: 'Vec<Bytes>',
                capacity: 'u64',
            },
            BspSignUpSuccess: {
                who: 'AccountId20',
                bspId: 'H256',
                root: 'H256',
                multiaddresses: 'Vec<Bytes>',
                capacity: 'u64',
            },
            SignUpRequestCanceled: {
                who: 'AccountId20',
            },
            MspSignOffSuccess: {
                who: 'AccountId20',
                mspId: 'H256',
            },
            BspSignOffSuccess: {
                who: 'AccountId20',
                bspId: 'H256',
            },
            CapacityChanged: {
                who: 'AccountId20',
                providerId: 'PalletStorageProvidersStorageProviderId',
                oldCapacity: 'u64',
                newCapacity: 'u64',
                nextBlockWhenChangeAllowed: 'u32',
            },
            Slashed: {
                providerId: 'H256',
                amount: 'u128',
            },
            AwaitingTopUp: {
                providerId: 'H256',
                topUpMetadata: 'PalletStorageProvidersTopUpMetadata',
            },
            TopUpFulfilled: {
                providerId: 'H256',
                amount: 'u128',
            },
            FailedToGetOwnerAccountOfInsolventProvider: {
                providerId: 'H256',
            },
            FailedToSlashInsolventProvider: {
                providerId: 'H256',
                amountToSlash: 'u128',
                error: 'SpRuntimeDispatchError',
            },
            FailedToStopAllCyclesForInsolventBsp: {
                providerId: 'H256',
                error: 'SpRuntimeDispatchError',
            },
            FailedToInsertProviderTopUpExpiration: {
                providerId: 'H256',
                expirationTick: 'u32',
            },
            ProviderInsolvent: {
                providerId: 'H256',
            },
            BucketsOfInsolventMsp: {
                mspId: 'H256',
                buckets: 'Vec<H256>',
            },
            BucketRootChanged: {
                bucketId: 'H256',
                oldRoot: 'H256',
                newRoot: 'H256',
            },
            MultiAddressAdded: {
                providerId: 'H256',
                newMultiaddress: 'Bytes',
            },
            MultiAddressRemoved: {
                providerId: 'H256',
                removedMultiaddress: 'Bytes',
            },
            ValuePropAdded: {
                mspId: 'H256',
                valuePropId: 'H256',
                valueProp: 'PalletStorageProvidersValueProposition',
            },
            ValuePropUnavailable: {
                mspId: 'H256',
                valuePropId: 'H256',
            },
            MspDeleted: {
                providerId: 'H256',
            },
            BspDeleted: {
                providerId: 'H256'
            }
        }
    },
    /**
     * Lookup101: pallet_storage_providers::types::ValuePropositionWithId<T>
     **/
    PalletStorageProvidersValuePropositionWithId: {
        id: 'H256',
        valueProp: 'PalletStorageProvidersValueProposition'
    },
    /**
     * Lookup102: pallet_storage_providers::types::ValueProposition<T>
     **/
    PalletStorageProvidersValueProposition: {
        pricePerGigaUnitOfDataPerBlock: 'u128',
        commitment: 'Bytes',
        bucketDataLimit: 'u64',
        available: 'bool'
    },
    /**
     * Lookup104: pallet_storage_providers::types::StorageProviderId<T>
     **/
    PalletStorageProvidersStorageProviderId: {
        _enum: {
            BackupStorageProvider: 'H256',
            MainStorageProvider: 'H256'
        }
    },
    /**
     * Lookup105: pallet_storage_providers::types::TopUpMetadata<T>
     **/
    PalletStorageProvidersTopUpMetadata: {
        startedAt: 'u32',
        endTickGracePeriod: 'u32'
    },
    /**
     * Lookup106: pallet_file_system::pallet::Event<T>
     **/
    PalletFileSystemEvent: {
        _enum: {
            NewBucket: {
                who: 'AccountId20',
                mspId: 'H256',
                bucketId: 'H256',
                name: 'Bytes',
                root: 'H256',
                collectionId: 'Option<u32>',
                private: 'bool',
                valuePropId: 'H256',
            },
            BucketDeleted: {
                who: 'AccountId20',
                bucketId: 'H256',
                maybeCollectionId: 'Option<u32>',
            },
            BucketPrivacyUpdated: {
                who: 'AccountId20',
                bucketId: 'H256',
                collectionId: 'Option<u32>',
                private: 'bool',
            },
            NewCollectionAndAssociation: {
                who: 'AccountId20',
                bucketId: 'H256',
                collectionId: 'u32',
            },
            MoveBucketRequested: {
                who: 'AccountId20',
                bucketId: 'H256',
                newMspId: 'H256',
                newValuePropId: 'H256',
            },
            MoveBucketRequestExpired: {
                bucketId: 'H256',
            },
            MoveBucketAccepted: {
                bucketId: 'H256',
                oldMspId: 'Option<H256>',
                newMspId: 'H256',
                valuePropId: 'H256',
            },
            MoveBucketRejected: {
                bucketId: 'H256',
                oldMspId: 'Option<H256>',
                newMspId: 'H256',
            },
            NewStorageRequest: {
                _alias: {
                    size_: 'size',
                },
                who: 'AccountId20',
                fileKey: 'H256',
                bucketId: 'H256',
                location: 'Bytes',
                fingerprint: 'H256',
                size_: 'u64',
                peerIds: 'Vec<Bytes>',
                expiresAt: 'u32',
            },
            MspAcceptedStorageRequest: {
                fileKey: 'H256',
                fileMetadata: 'ShpFileMetadataFileMetadata',
            },
            StorageRequestFulfilled: {
                fileKey: 'H256',
            },
            StorageRequestExpired: {
                fileKey: 'H256',
            },
            StorageRequestRevoked: {
                fileKey: 'H256',
            },
            StorageRequestRejected: {
                fileKey: 'H256',
                mspId: 'H256',
                bucketId: 'H256',
                reason: 'PalletFileSystemRejectedStorageRequestReason',
            },
            IncompleteStorageRequest: {
                fileKey: 'H256',
            },
            IncompleteStorageRequestCleanedUp: {
                fileKey: 'H256',
            },
            AcceptedBspVolunteer: {
                _alias: {
                    size_: 'size',
                },
                bspId: 'H256',
                bucketId: 'H256',
                location: 'Bytes',
                fingerprint: 'H256',
                multiaddresses: 'Vec<Bytes>',
                owner: 'AccountId20',
                size_: 'u64',
            },
            BspConfirmedStoring: {
                who: 'AccountId20',
                bspId: 'H256',
                confirmedFileKeys: 'Vec<(H256,ShpFileMetadataFileMetadata)>',
                skippedFileKeys: 'Vec<H256>',
                newRoot: 'H256',
            },
            BspChallengeCycleInitialised: {
                who: 'AccountId20',
                bspId: 'H256',
            },
            BspRequestedToStopStoring: {
                bspId: 'H256',
                fileKey: 'H256',
                owner: 'AccountId20',
                location: 'Bytes',
            },
            BspConfirmStoppedStoring: {
                bspId: 'H256',
                fileKey: 'H256',
                newRoot: 'H256',
            },
            MspStoppedStoringBucket: {
                mspId: 'H256',
                owner: 'AccountId20',
                bucketId: 'H256',
            },
            SpStopStoringInsolventUser: {
                spId: 'H256',
                fileKey: 'H256',
                owner: 'AccountId20',
                location: 'Bytes',
                newRoot: 'H256',
            },
            MspStopStoringBucketInsolventUser: {
                mspId: 'H256',
                owner: 'AccountId20',
                bucketId: 'H256',
            },
            FileDeletionRequested: {
                signedDeleteIntention: 'PalletFileSystemFileOperationIntention',
                signature: 'FpAccountEthereumSignature',
            },
            BucketFileDeletionsCompleted: {
                user: 'AccountId20',
                fileKeys: 'Vec<H256>',
                bucketId: 'H256',
                mspId: 'Option<H256>',
                oldRoot: 'H256',
                newRoot: 'H256',
            },
            BspFileDeletionsCompleted: {
                users: 'Vec<AccountId20>',
                fileKeys: 'Vec<H256>',
                bspId: 'H256',
                oldRoot: 'H256',
                newRoot: 'H256',
            },
            UsedCapacityShouldBeZero: {
                actualUsedCapacity: 'u64',
            },
            FailedToReleaseStorageRequestCreationDeposit: {
                fileKey: 'H256',
                owner: 'AccountId20',
                amountToReturn: 'u128',
                error: 'SpRuntimeDispatchError'
            }
        }
    },
    /**
     * Lookup110: shp_file_metadata::FileMetadata
     **/
    ShpFileMetadataFileMetadata: {
        owner: 'Bytes',
        bucketId: 'Bytes',
        location: 'Bytes',
        fileSize: 'Compact<u64>',
        fingerprint: 'ShpFileMetadataFingerprint'
    },
    /**
     * Lookup111: shp_file_metadata::Fingerprint
     **/
    ShpFileMetadataFingerprint: '[u8;32]',
    /**
     * Lookup112: pallet_file_system::types::RejectedStorageRequestReason
     **/
    PalletFileSystemRejectedStorageRequestReason: {
        _enum: ['ReachedMaximumCapacity', 'ReceivedInvalidProof', 'FileKeyAlreadyStored', 'RequestExpired', 'InternalError']
    },
    /**
     * Lookup117: pallet_file_system::types::FileOperationIntention<T>
     **/
    PalletFileSystemFileOperationIntention: {
        fileKey: 'H256',
        operation: 'PalletFileSystemFileOperation'
    },
    /**
     * Lookup118: pallet_file_system::types::FileOperation
     **/
    PalletFileSystemFileOperation: {
        _enum: ['Delete']
    },
    /**
     * Lookup119: fp_account::EthereumSignature
     **/
    FpAccountEthereumSignature: '[u8;65]',
    /**
     * Lookup124: pallet_proofs_dealer::pallet::Event<T>
     **/
    PalletProofsDealerEvent: {
        _enum: {
            NewChallenge: {
                who: 'Option<AccountId20>',
                keyChallenged: 'H256',
            },
            NewPriorityChallenge: {
                who: 'Option<AccountId20>',
                keyChallenged: 'H256',
                shouldRemoveKey: 'bool',
            },
            ProofAccepted: {
                providerId: 'H256',
                proof: 'PalletProofsDealerProof',
                lastTickProven: 'u32',
            },
            NewChallengeSeed: {
                challengesTicker: 'u32',
                seed: 'H256',
            },
            NewCheckpointChallenge: {
                challengesTicker: 'u32',
                challenges: 'Vec<PalletProofsDealerCustomChallenge>',
            },
            SlashableProvider: {
                provider: 'H256',
                nextChallengeDeadline: 'u32',
            },
            NoRecordOfLastSubmittedProof: {
                provider: 'H256',
            },
            NewChallengeCycleInitialised: {
                currentTick: 'u32',
                nextChallengeDeadline: 'u32',
                provider: 'H256',
                maybeProviderAccount: 'Option<AccountId20>',
            },
            MutationsAppliedForProvider: {
                providerId: 'H256',
                mutations: 'Vec<(H256,ShpTraitsTrieMutation)>',
                oldRoot: 'H256',
                newRoot: 'H256',
            },
            MutationsApplied: {
                mutations: 'Vec<(H256,ShpTraitsTrieMutation)>',
                oldRoot: 'H256',
                newRoot: 'H256',
                eventInfo: 'Option<Bytes>',
            },
            ChallengesTickerSet: {
                paused: 'bool'
            }
        }
    },
    /**
     * Lookup125: pallet_proofs_dealer::types::Proof<T>
     **/
    PalletProofsDealerProof: {
        forestProof: 'SpTrieStorageProofCompactProof',
        keyProofs: 'BTreeMap<H256, PalletProofsDealerKeyProof>'
    },
    /**
     * Lookup126: sp_trie::storage_proof::CompactProof
     **/
    SpTrieStorageProofCompactProof: {
        encodedNodes: 'Vec<Bytes>'
    },
    /**
     * Lookup129: pallet_proofs_dealer::types::KeyProof<T>
     **/
    PalletProofsDealerKeyProof: {
        proof: 'ShpFileKeyVerifierFileKeyProof',
        challengeCount: 'u32'
    },
    /**
     * Lookup130: shp_file_key_verifier::types::FileKeyProof
     **/
    ShpFileKeyVerifierFileKeyProof: {
        fileMetadata: 'ShpFileMetadataFileMetadata',
        proof: 'SpTrieStorageProofCompactProof'
    },
    /**
     * Lookup134: pallet_proofs_dealer::types::CustomChallenge<T>
     **/
    PalletProofsDealerCustomChallenge: {
        key: 'H256',
        shouldRemoveKey: 'bool'
    },
    /**
     * Lookup138: shp_traits::TrieMutation
     **/
    ShpTraitsTrieMutation: {
        _enum: {
            Add: 'ShpTraitsTrieAddMutation',
            Remove: 'ShpTraitsTrieRemoveMutation'
        }
    },
    /**
     * Lookup139: shp_traits::TrieAddMutation
     **/
    ShpTraitsTrieAddMutation: {
        value: 'Bytes'
    },
    /**
     * Lookup140: shp_traits::TrieRemoveMutation
     **/
    ShpTraitsTrieRemoveMutation: {
        maybeValue: 'Option<Bytes>'
    },
    /**
     * Lookup142: pallet_randomness::pallet::Event<T>
     **/
    PalletRandomnessEvent: {
        _enum: {
            NewOneEpochAgoRandomnessAvailable: {
                randomnessSeed: 'H256',
                fromEpoch: 'u64',
                validUntilBlock: 'u32'
            }
        }
    },
    /**
     * Lookup143: pallet_payment_streams::pallet::Event<T>
     **/
    PalletPaymentStreamsEvent: {
        _enum: {
            FixedRatePaymentStreamCreated: {
                userAccount: 'AccountId20',
                providerId: 'H256',
                rate: 'u128',
            },
            FixedRatePaymentStreamUpdated: {
                userAccount: 'AccountId20',
                providerId: 'H256',
                newRate: 'u128',
            },
            FixedRatePaymentStreamDeleted: {
                userAccount: 'AccountId20',
                providerId: 'H256',
            },
            DynamicRatePaymentStreamCreated: {
                userAccount: 'AccountId20',
                providerId: 'H256',
                amountProvided: 'u64',
            },
            DynamicRatePaymentStreamUpdated: {
                userAccount: 'AccountId20',
                providerId: 'H256',
                newAmountProvided: 'u64',
            },
            DynamicRatePaymentStreamDeleted: {
                userAccount: 'AccountId20',
                providerId: 'H256',
            },
            PaymentStreamCharged: {
                userAccount: 'AccountId20',
                providerId: 'H256',
                amount: 'u128',
                lastTickCharged: 'u32',
                chargedAtTick: 'u32',
            },
            UsersCharged: {
                userAccounts: 'Vec<AccountId20>',
                providerId: 'H256',
                chargedAtTick: 'u32',
            },
            LastChargeableInfoUpdated: {
                providerId: 'H256',
                lastChargeableTick: 'u32',
                lastChargeablePriceIndex: 'u128',
            },
            UserWithoutFunds: {
                who: 'AccountId20',
            },
            UserPaidAllDebts: {
                who: 'AccountId20',
            },
            UserPaidSomeDebts: {
                who: 'AccountId20',
            },
            UserSolvent: {
                who: 'AccountId20',
            },
            InconsistentTickProcessing: {
                lastProcessedTick: 'u32',
                tickToProcess: 'u32'
            }
        }
    },
    /**
     * Lookup145: pallet_bucket_nfts::pallet::Event<T>
     **/
    PalletBucketNftsEvent: {
        _enum: {
            AccessShared: {
                issuer: 'AccountId20',
                recipient: 'AccountId20',
            },
            ItemReadAccessUpdated: {
                admin: 'AccountId20',
                bucket: 'H256',
                itemId: 'u32',
            },
            ItemBurned: {
                account: 'AccountId20',
                bucket: 'H256',
                itemId: 'u32'
            }
        }
    },
    /**
     * Lookup146: pallet_nfts::pallet::Event<T, I>
     **/
    PalletNftsEvent: {
        _enum: {
            Created: {
                collection: 'u32',
                creator: 'AccountId20',
                owner: 'AccountId20',
            },
            ForceCreated: {
                collection: 'u32',
                owner: 'AccountId20',
            },
            Destroyed: {
                collection: 'u32',
            },
            Issued: {
                collection: 'u32',
                item: 'u32',
                owner: 'AccountId20',
            },
            Transferred: {
                collection: 'u32',
                item: 'u32',
                from: 'AccountId20',
                to: 'AccountId20',
            },
            Burned: {
                collection: 'u32',
                item: 'u32',
                owner: 'AccountId20',
            },
            ItemTransferLocked: {
                collection: 'u32',
                item: 'u32',
            },
            ItemTransferUnlocked: {
                collection: 'u32',
                item: 'u32',
            },
            ItemPropertiesLocked: {
                collection: 'u32',
                item: 'u32',
                lockMetadata: 'bool',
                lockAttributes: 'bool',
            },
            CollectionLocked: {
                collection: 'u32',
            },
            OwnerChanged: {
                collection: 'u32',
                newOwner: 'AccountId20',
            },
            TeamChanged: {
                collection: 'u32',
                issuer: 'Option<AccountId20>',
                admin: 'Option<AccountId20>',
                freezer: 'Option<AccountId20>',
            },
            TransferApproved: {
                collection: 'u32',
                item: 'u32',
                owner: 'AccountId20',
                delegate: 'AccountId20',
                deadline: 'Option<u32>',
            },
            ApprovalCancelled: {
                collection: 'u32',
                item: 'u32',
                owner: 'AccountId20',
                delegate: 'AccountId20',
            },
            AllApprovalsCancelled: {
                collection: 'u32',
                item: 'u32',
                owner: 'AccountId20',
            },
            CollectionConfigChanged: {
                collection: 'u32',
            },
            CollectionMetadataSet: {
                collection: 'u32',
                data: 'Bytes',
            },
            CollectionMetadataCleared: {
                collection: 'u32',
            },
            ItemMetadataSet: {
                collection: 'u32',
                item: 'u32',
                data: 'Bytes',
            },
            ItemMetadataCleared: {
                collection: 'u32',
                item: 'u32',
            },
            Redeposited: {
                collection: 'u32',
                successfulItems: 'Vec<u32>',
            },
            AttributeSet: {
                collection: 'u32',
                maybeItem: 'Option<u32>',
                key: 'Bytes',
                value: 'Bytes',
                namespace: 'PalletNftsAttributeNamespace',
            },
            AttributeCleared: {
                collection: 'u32',
                maybeItem: 'Option<u32>',
                key: 'Bytes',
                namespace: 'PalletNftsAttributeNamespace',
            },
            ItemAttributesApprovalAdded: {
                collection: 'u32',
                item: 'u32',
                delegate: 'AccountId20',
            },
            ItemAttributesApprovalRemoved: {
                collection: 'u32',
                item: 'u32',
                delegate: 'AccountId20',
            },
            OwnershipAcceptanceChanged: {
                who: 'AccountId20',
                maybeCollection: 'Option<u32>',
            },
            CollectionMaxSupplySet: {
                collection: 'u32',
                maxSupply: 'u32',
            },
            CollectionMintSettingsUpdated: {
                collection: 'u32',
            },
            NextCollectionIdIncremented: {
                nextId: 'Option<u32>',
            },
            ItemPriceSet: {
                collection: 'u32',
                item: 'u32',
                price: 'u128',
                whitelistedBuyer: 'Option<AccountId20>',
            },
            ItemPriceRemoved: {
                collection: 'u32',
                item: 'u32',
            },
            ItemBought: {
                collection: 'u32',
                item: 'u32',
                price: 'u128',
                seller: 'AccountId20',
                buyer: 'AccountId20',
            },
            TipSent: {
                collection: 'u32',
                item: 'u32',
                sender: 'AccountId20',
                receiver: 'AccountId20',
                amount: 'u128',
            },
            SwapCreated: {
                offeredCollection: 'u32',
                offeredItem: 'u32',
                desiredCollection: 'u32',
                desiredItem: 'Option<u32>',
                price: 'Option<PalletNftsPriceWithDirection>',
                deadline: 'u32',
            },
            SwapCancelled: {
                offeredCollection: 'u32',
                offeredItem: 'u32',
                desiredCollection: 'u32',
                desiredItem: 'Option<u32>',
                price: 'Option<PalletNftsPriceWithDirection>',
                deadline: 'u32',
            },
            SwapClaimed: {
                sentCollection: 'u32',
                sentItem: 'u32',
                sentItemOwner: 'AccountId20',
                receivedCollection: 'u32',
                receivedItem: 'u32',
                receivedItemOwner: 'AccountId20',
                price: 'Option<PalletNftsPriceWithDirection>',
                deadline: 'u32',
            },
            PreSignedAttributesSet: {
                collection: 'u32',
                item: 'u32',
                namespace: 'PalletNftsAttributeNamespace',
            },
            PalletAttributeSet: {
                collection: 'u32',
                item: 'Option<u32>',
                attribute: 'PalletNftsPalletAttributes',
                value: 'Bytes'
            }
        }
    },
    /**
     * Lookup150: pallet_nfts::types::AttributeNamespace<fp_account::AccountId20>
     **/
    PalletNftsAttributeNamespace: {
        _enum: {
            Pallet: 'Null',
            CollectionOwner: 'Null',
            ItemOwner: 'Null',
            Account: 'AccountId20'
        }
    },
    /**
     * Lookup152: pallet_nfts::types::PriceWithDirection<Amount>
     **/
    PalletNftsPriceWithDirection: {
        amount: 'u128',
        direction: 'PalletNftsPriceDirection'
    },
    /**
     * Lookup153: pallet_nfts::types::PriceDirection
     **/
    PalletNftsPriceDirection: {
        _enum: ['Send', 'Receive']
    },
    /**
     * Lookup154: pallet_nfts::types::PalletAttributes<CollectionId>
     **/
    PalletNftsPalletAttributes: {
        _enum: {
            UsedToClaim: 'u32',
            TransferDisabled: 'Null'
        }
    },
    /**
     * Lookup155: frame_system::Phase
     **/
    FrameSystemPhase: {
        _enum: {
            ApplyExtrinsic: 'u32',
            Finalization: 'Null',
            Initialization: 'Null'
        }
    },
    /**
     * Lookup158: frame_system::LastRuntimeUpgradeInfo
     **/
    FrameSystemLastRuntimeUpgradeInfo: {
        specVersion: 'Compact<u32>',
        specName: 'Text'
    },
    /**
     * Lookup160: frame_system::CodeUpgradeAuthorization<T>
     **/
    FrameSystemCodeUpgradeAuthorization: {
        codeHash: 'H256',
        checkVersion: 'bool'
    },
    /**
     * Lookup161: frame_system::pallet::Call<T>
     **/
    FrameSystemCall: {
        _enum: {
            remark: {
                remark: 'Bytes',
            },
            set_heap_pages: {
                pages: 'u64',
            },
            set_code: {
                code: 'Bytes',
            },
            set_code_without_checks: {
                code: 'Bytes',
            },
            set_storage: {
                items: 'Vec<(Bytes,Bytes)>',
            },
            kill_storage: {
                _alias: {
                    keys_: 'keys',
                },
                keys_: 'Vec<Bytes>',
            },
            kill_prefix: {
                prefix: 'Bytes',
                subkeys: 'u32',
            },
            remark_with_event: {
                remark: 'Bytes',
            },
            __Unused8: 'Null',
            authorize_upgrade: {
                codeHash: 'H256',
            },
            authorize_upgrade_without_checks: {
                codeHash: 'H256',
            },
            apply_authorized_upgrade: {
                code: 'Bytes'
            }
        }
    },
    /**
     * Lookup164: frame_system::limits::BlockWeights
     **/
    FrameSystemLimitsBlockWeights: {
        baseBlock: 'SpWeightsWeightV2Weight',
        maxBlock: 'SpWeightsWeightV2Weight',
        perClass: 'FrameSupportDispatchPerDispatchClassWeightsPerClass'
    },
    /**
     * Lookup165: frame_support::dispatch::PerDispatchClass<frame_system::limits::WeightsPerClass>
     **/
    FrameSupportDispatchPerDispatchClassWeightsPerClass: {
        normal: 'FrameSystemLimitsWeightsPerClass',
        operational: 'FrameSystemLimitsWeightsPerClass',
        mandatory: 'FrameSystemLimitsWeightsPerClass'
    },
    /**
     * Lookup166: frame_system::limits::WeightsPerClass
     **/
    FrameSystemLimitsWeightsPerClass: {
        baseExtrinsic: 'SpWeightsWeightV2Weight',
        maxExtrinsic: 'Option<SpWeightsWeightV2Weight>',
        maxTotal: 'Option<SpWeightsWeightV2Weight>',
        reserved: 'Option<SpWeightsWeightV2Weight>'
    },
    /**
     * Lookup168: frame_system::limits::BlockLength
     **/
    FrameSystemLimitsBlockLength: {
        max: 'FrameSupportDispatchPerDispatchClassU32'
    },
    /**
     * Lookup169: frame_support::dispatch::PerDispatchClass<T>
     **/
    FrameSupportDispatchPerDispatchClassU32: {
        normal: 'u32',
        operational: 'u32',
        mandatory: 'u32'
    },
    /**
     * Lookup170: sp_weights::RuntimeDbWeight
     **/
    SpWeightsRuntimeDbWeight: {
        read: 'u64',
        write: 'u64'
    },
    /**
     * Lookup171: sp_version::RuntimeVersion
     **/
    SpVersionRuntimeVersion: {
        specName: 'Text',
        implName: 'Text',
        authoringVersion: 'u32',
        specVersion: 'u32',
        implVersion: 'u32',
        apis: 'Vec<([u8;8],u32)>',
        transactionVersion: 'u32',
        systemVersion: 'u8'
    },
    /**
     * Lookup177: frame_system::pallet::Error<T>
     **/
    FrameSystemError: {
        _enum: ['InvalidSpecName', 'SpecVersionNeedsToIncrease', 'FailedToExtractRuntimeVersion', 'NonDefaultComposite', 'NonZeroRefCount', 'CallFiltered', 'MultiBlockMigrationsOngoing', 'NothingAuthorized', 'Unauthorized']
    },
    /**
     * Lookup180: sp_consensus_babe::app::Public
     **/
    SpConsensusBabeAppPublic: '[u8;32]',
    /**
     * Lookup183: sp_consensus_babe::digests::NextConfigDescriptor
     **/
    SpConsensusBabeDigestsNextConfigDescriptor: {
        _enum: {
            __Unused0: 'Null',
            V1: {
                c: '(u64,u64)',
                allowedSlots: 'SpConsensusBabeAllowedSlots'
            }
        }
    },
    /**
     * Lookup185: sp_consensus_babe::AllowedSlots
     **/
    SpConsensusBabeAllowedSlots: {
        _enum: ['PrimarySlots', 'PrimaryAndSecondaryPlainSlots', 'PrimaryAndSecondaryVRFSlots']
    },
    /**
     * Lookup189: sp_consensus_babe::digests::PreDigest
     **/
    SpConsensusBabeDigestsPreDigest: {
        _enum: {
            __Unused0: 'Null',
            Primary: 'SpConsensusBabeDigestsPrimaryPreDigest',
            SecondaryPlain: 'SpConsensusBabeDigestsSecondaryPlainPreDigest',
            SecondaryVRF: 'SpConsensusBabeDigestsSecondaryVRFPreDigest'
        }
    },
    /**
     * Lookup190: sp_consensus_babe::digests::PrimaryPreDigest
     **/
    SpConsensusBabeDigestsPrimaryPreDigest: {
        authorityIndex: 'u32',
        slot: 'u64',
        vrfSignature: 'SpCoreSr25519VrfVrfSignature'
    },
    /**
     * Lookup191: sp_core::sr25519::vrf::VrfSignature
     **/
    SpCoreSr25519VrfVrfSignature: {
        preOutput: '[u8;32]',
        proof: '[u8;64]'
    },
    /**
     * Lookup193: sp_consensus_babe::digests::SecondaryPlainPreDigest
     **/
    SpConsensusBabeDigestsSecondaryPlainPreDigest: {
        authorityIndex: 'u32',
        slot: 'u64'
    },
    /**
     * Lookup194: sp_consensus_babe::digests::SecondaryVRFPreDigest
     **/
    SpConsensusBabeDigestsSecondaryVRFPreDigest: {
        authorityIndex: 'u32',
        slot: 'u64',
        vrfSignature: 'SpCoreSr25519VrfVrfSignature'
    },
    /**
     * Lookup196: sp_consensus_babe::BabeEpochConfiguration
     **/
    SpConsensusBabeBabeEpochConfiguration: {
        c: '(u64,u64)',
        allowedSlots: 'SpConsensusBabeAllowedSlots'
    },
    /**
     * Lookup200: pallet_babe::pallet::Call<T>
     **/
    PalletBabeCall: {
        _enum: {
            report_equivocation: {
                equivocationProof: 'SpConsensusSlotsEquivocationProof',
                keyOwnerProof: 'SpSessionMembershipProof',
            },
            report_equivocation_unsigned: {
                equivocationProof: 'SpConsensusSlotsEquivocationProof',
                keyOwnerProof: 'SpSessionMembershipProof',
            },
            plan_config_change: {
                config: 'SpConsensusBabeDigestsNextConfigDescriptor'
            }
        }
    },
    /**
     * Lookup201: sp_consensus_slots::EquivocationProof<sp_runtime::generic::header::Header<Number, Hash>, sp_consensus_babe::app::Public>
     **/
    SpConsensusSlotsEquivocationProof: {
        offender: 'SpConsensusBabeAppPublic',
        slot: 'u64',
        firstHeader: 'SpRuntimeHeader',
        secondHeader: 'SpRuntimeHeader'
    },
    /**
     * Lookup202: sp_runtime::generic::header::Header<Number, Hash>
     **/
    SpRuntimeHeader: {
        parentHash: 'H256',
        number: 'Compact<u32>',
        stateRoot: 'H256',
        extrinsicsRoot: 'H256',
        digest: 'SpRuntimeDigest'
    },
    /**
     * Lookup203: sp_session::MembershipProof
     **/
    SpSessionMembershipProof: {
        session: 'u32',
        trieNodes: 'Vec<Bytes>',
        validatorCount: 'u32'
    },
    /**
     * Lookup204: pallet_babe::pallet::Error<T>
     **/
    PalletBabeError: {
        _enum: ['InvalidEquivocationProof', 'InvalidKeyOwnershipProof', 'DuplicateOffenceReport', 'InvalidConfiguration']
    },
    /**
     * Lookup205: pallet_timestamp::pallet::Call<T>
     **/
    PalletTimestampCall: {
        _enum: {
            set: {
                now: 'Compact<u64>'
            }
        }
    },
    /**
     * Lookup207: pallet_balances::types::BalanceLock<Balance>
     **/
    PalletBalancesBalanceLock: {
        id: '[u8;8]',
        amount: 'u128',
        reasons: 'PalletBalancesReasons'
    },
    /**
     * Lookup208: pallet_balances::types::Reasons
     **/
    PalletBalancesReasons: {
        _enum: ['Fee', 'Misc', 'All']
    },
    /**
     * Lookup211: pallet_balances::types::ReserveData<ReserveIdentifier, Balance>
     **/
    PalletBalancesReserveData: {
        id: '[u8;8]',
        amount: 'u128'
    },
    /**
     * Lookup214: frame_support::traits::tokens::misc::IdAmount<sh_solochain_evm_runtime::RuntimeHoldReason, Balance>
     **/
    FrameSupportTokensMiscIdAmountRuntimeHoldReason: {
        id: 'ShSolochainEvmRuntimeRuntimeHoldReason',
        amount: 'u128'
    },
    /**
     * Lookup215: sh_solochain_evm_runtime::RuntimeHoldReason
     **/
    ShSolochainEvmRuntimeRuntimeHoldReason: {
        _enum: {
            __Unused0: 'Null',
            __Unused1: 'Null',
            __Unused2: 'Null',
            __Unused3: 'Null',
            __Unused4: 'Null',
            __Unused5: 'Null',
            __Unused6: 'Null',
            __Unused7: 'Null',
            __Unused8: 'Null',
            __Unused9: 'Null',
            __Unused10: 'Null',
            __Unused11: 'Null',
            __Unused12: 'Null',
            __Unused13: 'Null',
            __Unused14: 'Null',
            __Unused15: 'Null',
            __Unused16: 'Null',
            __Unused17: 'Null',
            __Unused18: 'Null',
            __Unused19: 'Null',
            __Unused20: 'Null',
            __Unused21: 'Null',
            __Unused22: 'Null',
            __Unused23: 'Null',
            __Unused24: 'Null',
            __Unused25: 'Null',
            __Unused26: 'Null',
            __Unused27: 'Null',
            __Unused28: 'Null',
            __Unused29: 'Null',
            __Unused30: 'Null',
            __Unused31: 'Null',
            __Unused32: 'Null',
            __Unused33: 'Null',
            __Unused34: 'Null',
            __Unused35: 'Null',
            __Unused36: 'Null',
            __Unused37: 'Null',
            __Unused38: 'Null',
            __Unused39: 'Null',
            __Unused40: 'Null',
            __Unused41: 'Null',
            __Unused42: 'Null',
            __Unused43: 'Null',
            __Unused44: 'Null',
            __Unused45: 'Null',
            __Unused46: 'Null',
            __Unused47: 'Null',
            __Unused48: 'Null',
            __Unused49: 'Null',
            __Unused50: 'Null',
            __Unused51: 'Null',
            __Unused52: 'Null',
            __Unused53: 'Null',
            __Unused54: 'Null',
            __Unused55: 'Null',
            __Unused56: 'Null',
            __Unused57: 'Null',
            __Unused58: 'Null',
            __Unused59: 'Null',
            __Unused60: 'Null',
            __Unused61: 'Null',
            __Unused62: 'Null',
            __Unused63: 'Null',
            __Unused64: 'Null',
            __Unused65: 'Null',
            __Unused66: 'Null',
            __Unused67: 'Null',
            __Unused68: 'Null',
            __Unused69: 'Null',
            __Unused70: 'Null',
            __Unused71: 'Null',
            __Unused72: 'Null',
            __Unused73: 'Null',
            __Unused74: 'Null',
            __Unused75: 'Null',
            __Unused76: 'Null',
            __Unused77: 'Null',
            __Unused78: 'Null',
            __Unused79: 'Null',
            Providers: 'PalletStorageProvidersHoldReason',
            FileSystem: 'PalletFileSystemHoldReason',
            __Unused82: 'Null',
            __Unused83: 'Null',
            PaymentStreams: 'PalletPaymentStreamsHoldReason'
        }
    },
    /**
     * Lookup216: pallet_storage_providers::pallet::HoldReason
     **/
    PalletStorageProvidersHoldReason: {
        _enum: ['StorageProviderDeposit', 'BucketDeposit']
    },
    /**
     * Lookup217: pallet_file_system::pallet::HoldReason
     **/
    PalletFileSystemHoldReason: {
        _enum: ['StorageRequestCreationHold', 'FileDeletionRequestHold']
    },
    /**
     * Lookup218: pallet_payment_streams::pallet::HoldReason
     **/
    PalletPaymentStreamsHoldReason: {
        _enum: ['PaymentStreamDeposit']
    },
    /**
     * Lookup221: frame_support::traits::tokens::misc::IdAmount<sh_solochain_evm_runtime::RuntimeFreezeReason, Balance>
     **/
    FrameSupportTokensMiscIdAmountRuntimeFreezeReason: {
        id: 'ShSolochainEvmRuntimeRuntimeFreezeReason',
        amount: 'u128'
    },
    /**
     * Lookup222: sh_solochain_evm_runtime::RuntimeFreezeReason
     **/
    ShSolochainEvmRuntimeRuntimeFreezeReason: 'Null',
    /**
     * Lookup224: pallet_balances::pallet::Call<T, I>
     **/
    PalletBalancesCall: {
        _enum: {
            transfer_allow_death: {
                dest: 'AccountId20',
                value: 'Compact<u128>',
            },
            __Unused1: 'Null',
            force_transfer: {
                source: 'AccountId20',
                dest: 'AccountId20',
                value: 'Compact<u128>',
            },
            transfer_keep_alive: {
                dest: 'AccountId20',
                value: 'Compact<u128>',
            },
            transfer_all: {
                dest: 'AccountId20',
                keepAlive: 'bool',
            },
            force_unreserve: {
                who: 'AccountId20',
                amount: 'u128',
            },
            upgrade_accounts: {
                who: 'Vec<AccountId20>',
            },
            __Unused7: 'Null',
            force_set_balance: {
                who: 'AccountId20',
                newFree: 'Compact<u128>',
            },
            force_adjust_total_issuance: {
                direction: 'PalletBalancesAdjustmentDirection',
                delta: 'Compact<u128>',
            },
            burn: {
                value: 'Compact<u128>',
                keepAlive: 'bool'
            }
        }
    },
    /**
     * Lookup226: pallet_balances::types::AdjustmentDirection
     **/
    PalletBalancesAdjustmentDirection: {
        _enum: ['Increase', 'Decrease']
    },
    /**
     * Lookup227: pallet_balances::pallet::Error<T, I>
     **/
    PalletBalancesError: {
        _enum: ['VestingBalance', 'LiquidityRestrictions', 'InsufficientBalance', 'ExistentialDeposit', 'Expendability', 'ExistingVestingSchedule', 'DeadAccount', 'TooManyReserves', 'TooManyHolds', 'TooManyFreezes', 'IssuanceDeactivated', 'DeltaZero']
    },
    /**
     * Lookup228: sp_staking::offence::OffenceDetails<fp_account::AccountId20, Offender>
     **/
    SpStakingOffenceOffenceDetails: {
        offender: '(AccountId20,Null)',
        reporters: 'Vec<AccountId20>'
    },
    /**
     * Lookup234: sh_solochain_evm_runtime::SessionKeys
     **/
    ShSolochainEvmRuntimeSessionKeys: {
        babe: 'SpConsensusBabeAppPublic',
        grandpa: 'SpConsensusGrandpaAppPublic'
    },
    /**
     * Lookup236: sp_core::crypto::KeyTypeId
     **/
    SpCoreCryptoKeyTypeId: '[u8;4]',
    /**
     * Lookup237: pallet_session::pallet::Call<T>
     **/
    PalletSessionCall: {
        _enum: {
            set_keys: {
                _alias: {
                    keys_: 'keys',
                },
                keys_: 'ShSolochainEvmRuntimeSessionKeys',
                proof: 'Bytes',
            },
            purge_keys: 'Null'
        }
    },
    /**
     * Lookup238: pallet_session::pallet::Error<T>
     **/
    PalletSessionError: {
        _enum: ['InvalidProof', 'NoAssociatedValidatorId', 'DuplicatedKey', 'NoKeys', 'NoAccount']
    },
    /**
     * Lookup239: pallet_grandpa::StoredState<N>
     **/
    PalletGrandpaStoredState: {
        _enum: {
            Live: 'Null',
            PendingPause: {
                scheduledAt: 'u32',
                delay: 'u32',
            },
            Paused: 'Null',
            PendingResume: {
                scheduledAt: 'u32',
                delay: 'u32'
            }
        }
    },
    /**
     * Lookup240: pallet_grandpa::StoredPendingChange<N, Limit>
     **/
    PalletGrandpaStoredPendingChange: {
        scheduledAt: 'u32',
        delay: 'u32',
        nextAuthorities: 'Vec<(SpConsensusGrandpaAppPublic,u64)>',
        forced: 'Option<u32>'
    },
    /**
     * Lookup242: pallet_grandpa::pallet::Call<T>
     **/
    PalletGrandpaCall: {
        _enum: {
            report_equivocation: {
                equivocationProof: 'SpConsensusGrandpaEquivocationProof',
                keyOwnerProof: 'SpSessionMembershipProof',
            },
            report_equivocation_unsigned: {
                equivocationProof: 'SpConsensusGrandpaEquivocationProof',
                keyOwnerProof: 'SpSessionMembershipProof',
            },
            note_stalled: {
                delay: 'u32',
                bestFinalizedBlockNumber: 'u32'
            }
        }
    },
    /**
     * Lookup243: sp_consensus_grandpa::EquivocationProof<primitive_types::H256, N>
     **/
    SpConsensusGrandpaEquivocationProof: {
        setId: 'u64',
        equivocation: 'SpConsensusGrandpaEquivocation'
    },
    /**
     * Lookup244: sp_consensus_grandpa::Equivocation<primitive_types::H256, N>
     **/
    SpConsensusGrandpaEquivocation: {
        _enum: {
            Prevote: 'FinalityGrandpaEquivocationPrevote',
            Precommit: 'FinalityGrandpaEquivocationPrecommit'
        }
    },
    /**
     * Lookup245: finality_grandpa::Equivocation<sp_consensus_grandpa::app::Public, finality_grandpa::Prevote<primitive_types::H256, N>, sp_consensus_grandpa::app::Signature>
     **/
    FinalityGrandpaEquivocationPrevote: {
        roundNumber: 'u64',
        identity: 'SpConsensusGrandpaAppPublic',
        first: '(FinalityGrandpaPrevote,SpConsensusGrandpaAppSignature)',
        second: '(FinalityGrandpaPrevote,SpConsensusGrandpaAppSignature)'
    },
    /**
     * Lookup246: finality_grandpa::Prevote<primitive_types::H256, N>
     **/
    FinalityGrandpaPrevote: {
        targetHash: 'H256',
        targetNumber: 'u32'
    },
    /**
     * Lookup247: sp_consensus_grandpa::app::Signature
     **/
    SpConsensusGrandpaAppSignature: '[u8;64]',
    /**
     * Lookup249: finality_grandpa::Equivocation<sp_consensus_grandpa::app::Public, finality_grandpa::Precommit<primitive_types::H256, N>, sp_consensus_grandpa::app::Signature>
     **/
    FinalityGrandpaEquivocationPrecommit: {
        roundNumber: 'u64',
        identity: 'SpConsensusGrandpaAppPublic',
        first: '(FinalityGrandpaPrecommit,SpConsensusGrandpaAppSignature)',
        second: '(FinalityGrandpaPrecommit,SpConsensusGrandpaAppSignature)'
    },
    /**
     * Lookup250: finality_grandpa::Precommit<primitive_types::H256, N>
     **/
    FinalityGrandpaPrecommit: {
        targetHash: 'H256',
        targetNumber: 'u32'
    },
    /**
     * Lookup252: pallet_grandpa::pallet::Error<T>
     **/
    PalletGrandpaError: {
        _enum: ['PauseFailed', 'ResumeFailed', 'ChangePending', 'TooSoon', 'InvalidKeyOwnershipProof', 'InvalidEquivocationProof', 'DuplicateOffenceReport']
    },
    /**
     * Lookup254: pallet_transaction_payment::Releases
     **/
    PalletTransactionPaymentReleases: {
        _enum: ['V1Ancient', 'V2']
    },
    /**
     * Lookup255: pallet_parameters::pallet::Call<T>
     **/
    PalletParametersCall: {
        _enum: {
            set_parameter: {
                keyValue: 'ShSolochainEvmRuntimeConfigsRuntimeParamsRuntimeParameters'
            }
        }
    },
    /**
     * Lookup256: sh_solochain_evm_runtime::configs::runtime_params::RuntimeParameters
     **/
    ShSolochainEvmRuntimeConfigsRuntimeParamsRuntimeParameters: {
        _enum: {
            RuntimeConfig: 'ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigParameters'
        }
    },
    /**
     * Lookup257: sh_solochain_evm_runtime::configs::runtime_params::dynamic_params::runtime_config::Parameters
     **/
    ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigParameters: {
        _enum: {
            SlashAmountPerMaxFileSize: '(ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigSlashAmountPerMaxFileSize,Option<u128>)',
            StakeToChallengePeriod: '(ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigStakeToChallengePeriod,Option<u128>)',
            CheckpointChallengePeriod: '(ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigCheckpointChallengePeriod,Option<u32>)',
            MinChallengePeriod: '(ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMinChallengePeriod,Option<u32>)',
            SystemUtilisationLowerThresholdPercentage: '(ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigSystemUtilisationLowerThresholdPercentage,Option<Perbill>)',
            SystemUtilisationUpperThresholdPercentage: '(ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigSystemUtilisationUpperThresholdPercentage,Option<Perbill>)',
            MostlyStablePrice: '(ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMostlyStablePrice,Option<u128>)',
            MaxPrice: '(ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMaxPrice,Option<u128>)',
            MinPrice: '(ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMinPrice,Option<u128>)',
            UpperExponentFactor: '(ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigUpperExponentFactor,Option<u32>)',
            LowerExponentFactor: '(ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigLowerExponentFactor,Option<u32>)',
            ZeroSizeBucketFixedRate: '(ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigZeroSizeBucketFixedRate,Option<u128>)',
            IdealUtilisationRate: '(ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigIdealUtilisationRate,Option<Perbill>)',
            DecayRate: '(ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigDecayRate,Option<Perbill>)',
            MinimumTreasuryCut: '(ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMinimumTreasuryCut,Option<Perbill>)',
            MaximumTreasuryCut: '(ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMaximumTreasuryCut,Option<Perbill>)',
            BspStopStoringFilePenalty: '(ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigBspStopStoringFilePenalty,Option<u128>)',
            ProviderTopUpTtl: '(ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigProviderTopUpTtl,Option<u32>)',
            BasicReplicationTarget: '(ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigBasicReplicationTarget,Option<u32>)',
            StandardReplicationTarget: '(ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigStandardReplicationTarget,Option<u32>)',
            HighSecurityReplicationTarget: '(ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigHighSecurityReplicationTarget,Option<u32>)',
            SuperHighSecurityReplicationTarget: '(ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigSuperHighSecurityReplicationTarget,Option<u32>)',
            UltraHighSecurityReplicationTarget: '(ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigUltraHighSecurityReplicationTarget,Option<u32>)',
            MaxReplicationTarget: '(ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMaxReplicationTarget,Option<u32>)',
            TickRangeToMaximumThreshold: '(ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigTickRangeToMaximumThreshold,Option<u32>)',
            StorageRequestTtl: '(ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigStorageRequestTtl,Option<u32>)',
            MinWaitForStopStoring: '(ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMinWaitForStopStoring,Option<u32>)',
            MinSeedPeriod: '(ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMinSeedPeriod,Option<u32>)',
            StakeToSeedPeriod: '(ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigStakeToSeedPeriod,Option<u128>)',
            UpfrontTicksToPay: '(ShSolochainEvmRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigUpfrontTicksToPay,Option<u32>)'
        }
    },
    /**
     * Lookup260: pallet_sudo::pallet::Call<T>
     **/
    PalletSudoCall: {
        _enum: {
            sudo: {
                call: 'Call',
            },
            sudo_unchecked_weight: {
                call: 'Call',
                weight: 'SpWeightsWeightV2Weight',
            },
            set_key: {
                _alias: {
                    new_: 'new',
                },
                new_: 'AccountId20',
            },
            sudo_as: {
                who: 'AccountId20',
                call: 'Call',
            },
            remove_key: 'Null'
        }
    },
    /**
     * Lookup262: pallet_ethereum::pallet::Call<T>
     **/
    PalletEthereumCall: {
        _enum: {
            transact: {
                transaction: 'EthereumTransactionTransactionV2'
            }
        }
    },
    /**
     * Lookup263: ethereum::transaction::TransactionV2
     **/
    EthereumTransactionTransactionV2: {
        _enum: {
            Legacy: 'EthereumTransactionLegacyLegacyTransaction',
            EIP2930: 'EthereumTransactionEip2930Eip2930Transaction',
            EIP1559: 'EthereumTransactionEip1559Eip1559Transaction'
        }
    },
    /**
     * Lookup264: ethereum::transaction::legacy::LegacyTransaction
     **/
    EthereumTransactionLegacyLegacyTransaction: {
        nonce: 'U256',
        gasPrice: 'U256',
        gasLimit: 'U256',
        action: 'EthereumTransactionLegacyTransactionAction',
        value: 'U256',
        input: 'Bytes',
        signature: 'EthereumTransactionLegacyTransactionSignature'
    },
    /**
     * Lookup267: ethereum::transaction::legacy::TransactionAction
     **/
    EthereumTransactionLegacyTransactionAction: {
        _enum: {
            Call: 'H160',
            Create: 'Null'
        }
    },
    /**
     * Lookup268: ethereum::transaction::legacy::TransactionSignature
     **/
    EthereumTransactionLegacyTransactionSignature: {
        v: 'u64',
        r: 'H256',
        s: 'H256'
    },
    /**
     * Lookup270: ethereum::transaction::eip2930::EIP2930Transaction
     **/
    EthereumTransactionEip2930Eip2930Transaction: {
        chainId: 'u64',
        nonce: 'U256',
        gasPrice: 'U256',
        gasLimit: 'U256',
        action: 'EthereumTransactionLegacyTransactionAction',
        value: 'U256',
        input: 'Bytes',
        accessList: 'Vec<EthereumTransactionEip2930AccessListItem>',
        oddYParity: 'bool',
        r: 'H256',
        s: 'H256'
    },
    /**
     * Lookup272: ethereum::transaction::eip2930::AccessListItem
     **/
    EthereumTransactionEip2930AccessListItem: {
        address: 'H160',
        storageKeys: 'Vec<H256>'
    },
    /**
     * Lookup273: ethereum::transaction::eip1559::EIP1559Transaction
     **/
    EthereumTransactionEip1559Eip1559Transaction: {
        chainId: 'u64',
        nonce: 'U256',
        maxPriorityFeePerGas: 'U256',
        maxFeePerGas: 'U256',
        gasLimit: 'U256',
        action: 'EthereumTransactionLegacyTransactionAction',
        value: 'U256',
        input: 'Bytes',
        accessList: 'Vec<EthereumTransactionEip2930AccessListItem>',
        oddYParity: 'bool',
        r: 'H256',
        s: 'H256'
    },
    /**
     * Lookup274: pallet_evm::pallet::Call<T>
     **/
    PalletEvmCall: {
        _enum: {
            withdraw: {
                address: 'H160',
                value: 'u128',
            },
            call: {
                source: 'H160',
                target: 'H160',
                input: 'Bytes',
                value: 'U256',
                gasLimit: 'u64',
                maxFeePerGas: 'U256',
                maxPriorityFeePerGas: 'Option<U256>',
                nonce: 'Option<U256>',
                accessList: 'Vec<(H160,Vec<H256>)>',
            },
            create: {
                source: 'H160',
                init: 'Bytes',
                value: 'U256',
                gasLimit: 'u64',
                maxFeePerGas: 'U256',
                maxPriorityFeePerGas: 'Option<U256>',
                nonce: 'Option<U256>',
                accessList: 'Vec<(H160,Vec<H256>)>',
            },
            create2: {
                source: 'H160',
                init: 'Bytes',
                salt: 'H256',
                value: 'U256',
                gasLimit: 'u64',
                maxFeePerGas: 'U256',
                maxPriorityFeePerGas: 'Option<U256>',
                nonce: 'Option<U256>',
                accessList: 'Vec<(H160,Vec<H256>)>'
            }
        }
    },
    /**
     * Lookup278: pallet_storage_providers::pallet::Call<T>
     **/
    PalletStorageProvidersCall: {
        _enum: {
            request_msp_sign_up: {
                capacity: 'u64',
                multiaddresses: 'Vec<Bytes>',
                valuePropPricePerGigaUnitOfDataPerBlock: 'u128',
                commitment: 'Bytes',
                valuePropMaxDataLimit: 'u64',
                paymentAccount: 'AccountId20',
            },
            request_bsp_sign_up: {
                capacity: 'u64',
                multiaddresses: 'Vec<Bytes>',
                paymentAccount: 'AccountId20',
            },
            confirm_sign_up: {
                providerAccount: 'Option<AccountId20>',
            },
            cancel_sign_up: 'Null',
            msp_sign_off: {
                mspId: 'H256',
            },
            bsp_sign_off: 'Null',
            change_capacity: {
                newCapacity: 'u64',
            },
            add_value_prop: {
                pricePerGigaUnitOfDataPerBlock: 'u128',
                commitment: 'Bytes',
                bucketDataLimit: 'u64',
            },
            make_value_prop_unavailable: {
                valuePropId: 'H256',
            },
            add_multiaddress: {
                newMultiaddress: 'Bytes',
            },
            remove_multiaddress: {
                multiaddress: 'Bytes',
            },
            force_msp_sign_up: {
                who: 'AccountId20',
                mspId: 'H256',
                capacity: 'u64',
                multiaddresses: 'Vec<Bytes>',
                valuePropPricePerGigaUnitOfDataPerBlock: 'u128',
                commitment: 'Bytes',
                valuePropMaxDataLimit: 'u64',
                paymentAccount: 'AccountId20',
            },
            force_bsp_sign_up: {
                who: 'AccountId20',
                bspId: 'H256',
                capacity: 'u64',
                multiaddresses: 'Vec<Bytes>',
                paymentAccount: 'AccountId20',
                weight: 'Option<u32>',
            },
            slash: {
                providerId: 'H256',
            },
            top_up_deposit: 'Null',
            delete_provider: {
                providerId: 'H256',
            },
            stop_all_cycles: 'Null'
        }
    },
    /**
     * Lookup279: pallet_file_system::pallet::Call<T>
     **/
    PalletFileSystemCall: {
        _enum: {
            create_bucket: {
                mspId: 'H256',
                name: 'Bytes',
                private: 'bool',
                valuePropId: 'H256',
            },
            request_move_bucket: {
                bucketId: 'H256',
                newMspId: 'H256',
                newValuePropId: 'H256',
            },
            msp_respond_move_bucket_request: {
                bucketId: 'H256',
                response: 'PalletFileSystemBucketMoveRequestResponse',
            },
            update_bucket_privacy: {
                bucketId: 'H256',
                private: 'bool',
            },
            create_and_associate_collection_with_bucket: {
                bucketId: 'H256',
            },
            delete_bucket: {
                bucketId: 'H256',
            },
            issue_storage_request: {
                _alias: {
                    size_: 'size',
                },
                bucketId: 'H256',
                location: 'Bytes',
                fingerprint: 'H256',
                size_: 'u64',
                mspId: 'H256',
                peerIds: 'Vec<Bytes>',
                replicationTarget: 'PalletFileSystemReplicationTarget',
            },
            revoke_storage_request: {
                fileKey: 'H256',
            },
            msp_respond_storage_requests_multiple_buckets: {
                storageRequestMspResponse: 'Vec<PalletFileSystemStorageRequestMspBucketResponse>',
            },
            msp_stop_storing_bucket: {
                bucketId: 'H256',
            },
            bsp_volunteer: {
                fileKey: 'H256',
            },
            bsp_confirm_storing: {
                nonInclusionForestProof: 'SpTrieStorageProofCompactProof',
                fileKeysAndProofs: 'Vec<PalletFileSystemFileKeyWithProof>',
            },
            bsp_request_stop_storing: {
                _alias: {
                    size_: 'size',
                },
                fileKey: 'H256',
                bucketId: 'H256',
                location: 'Bytes',
                owner: 'AccountId20',
                fingerprint: 'H256',
                size_: 'u64',
                canServe: 'bool',
                inclusionForestProof: 'SpTrieStorageProofCompactProof',
            },
            bsp_confirm_stop_storing: {
                fileKey: 'H256',
                inclusionForestProof: 'SpTrieStorageProofCompactProof',
            },
            stop_storing_for_insolvent_user: {
                _alias: {
                    size_: 'size',
                },
                fileKey: 'H256',
                bucketId: 'H256',
                location: 'Bytes',
                owner: 'AccountId20',
                fingerprint: 'H256',
                size_: 'u64',
                inclusionForestProof: 'SpTrieStorageProofCompactProof',
            },
            msp_stop_storing_bucket_for_insolvent_user: {
                bucketId: 'H256',
            },
            request_delete_file: {
                _alias: {
                    size_: 'size',
                },
                signedIntention: 'PalletFileSystemFileOperationIntention',
                signature: 'FpAccountEthereumSignature',
                bucketId: 'H256',
                location: 'Bytes',
                size_: 'u64',
                fingerprint: 'H256',
            },
            delete_files: {
                fileDeletions: 'Vec<PalletFileSystemFileDeletionRequest>',
                bspId: 'Option<H256>',
                forestProof: 'SpTrieStorageProofCompactProof',
            },
            delete_files_for_incomplete_storage_request: {
                fileKeys: 'Vec<H256>',
                bspId: 'Option<H256>',
                forestProof: 'SpTrieStorageProofCompactProof'
            }
        }
    },
    /**
     * Lookup280: pallet_file_system::types::BucketMoveRequestResponse
     **/
    PalletFileSystemBucketMoveRequestResponse: {
        _enum: ['Accepted', 'Rejected']
    },
    /**
     * Lookup281: pallet_file_system::types::ReplicationTarget<T>
     **/
    PalletFileSystemReplicationTarget: {
        _enum: {
            Basic: 'Null',
            Standard: 'Null',
            HighSecurity: 'Null',
            SuperHighSecurity: 'Null',
            UltraHighSecurity: 'Null',
            Custom: 'u32'
        }
    },
    /**
     * Lookup283: pallet_file_system::types::StorageRequestMspBucketResponse<T>
     **/
    PalletFileSystemStorageRequestMspBucketResponse: {
        bucketId: 'H256',
        accept: 'Option<PalletFileSystemStorageRequestMspAcceptedFileKeys>',
        reject: 'Vec<PalletFileSystemRejectedStorageRequest>'
    },
    /**
     * Lookup285: pallet_file_system::types::StorageRequestMspAcceptedFileKeys<T>
     **/
    PalletFileSystemStorageRequestMspAcceptedFileKeys: {
        fileKeysAndProofs: 'Vec<PalletFileSystemFileKeyWithProof>',
        forestProof: 'SpTrieStorageProofCompactProof'
    },
    /**
     * Lookup287: pallet_file_system::types::FileKeyWithProof<T>
     **/
    PalletFileSystemFileKeyWithProof: {
        fileKey: 'H256',
        proof: 'ShpFileKeyVerifierFileKeyProof'
    },
    /**
     * Lookup289: pallet_file_system::types::RejectedStorageRequest<T>
     **/
    PalletFileSystemRejectedStorageRequest: {
        fileKey: 'H256',
        reason: 'PalletFileSystemRejectedStorageRequestReason'
    },
    /**
     * Lookup292: pallet_file_system::types::FileDeletionRequest<T>
     **/
    PalletFileSystemFileDeletionRequest: {
        _alias: {
            size_: 'size'
        },
        fileOwner: 'AccountId20',
        signedIntention: 'PalletFileSystemFileOperationIntention',
        signature: 'FpAccountEthereumSignature',
        bucketId: 'H256',
        location: 'Bytes',
        size_: 'u64',
        fingerprint: 'H256'
    },
    /**
     * Lookup294: pallet_proofs_dealer::pallet::Call<T>
     **/
    PalletProofsDealerCall: {
        _enum: {
            challenge: {
                key: 'H256',
            },
            submit_proof: {
                proof: 'PalletProofsDealerProof',
                provider: 'Option<H256>',
            },
            force_initialise_challenge_cycle: {
                provider: 'H256',
            },
            set_paused: {
                paused: 'bool',
            },
            priority_challenge: {
                key: 'H256',
                shouldRemoveKey: 'bool'
            }
        }
    },
    /**
     * Lookup295: pallet_randomness::pallet::Call<T>
     **/
    PalletRandomnessCall: {
        _enum: ['set_babe_randomness']
    },
    /**
     * Lookup296: pallet_payment_streams::pallet::Call<T>
     **/
    PalletPaymentStreamsCall: {
        _enum: {
            create_fixed_rate_payment_stream: {
                providerId: 'H256',
                userAccount: 'AccountId20',
                rate: 'u128',
            },
            update_fixed_rate_payment_stream: {
                providerId: 'H256',
                userAccount: 'AccountId20',
                newRate: 'u128',
            },
            delete_fixed_rate_payment_stream: {
                providerId: 'H256',
                userAccount: 'AccountId20',
            },
            create_dynamic_rate_payment_stream: {
                providerId: 'H256',
                userAccount: 'AccountId20',
                amountProvided: 'u64',
            },
            update_dynamic_rate_payment_stream: {
                providerId: 'H256',
                userAccount: 'AccountId20',
                newAmountProvided: 'u64',
            },
            delete_dynamic_rate_payment_stream: {
                providerId: 'H256',
                userAccount: 'AccountId20',
            },
            charge_payment_streams: {
                userAccount: 'AccountId20',
            },
            charge_multiple_users_payment_streams: {
                userAccounts: 'Vec<AccountId20>',
            },
            pay_outstanding_debt: {
                providers: 'Vec<H256>',
            },
            clear_insolvent_flag: 'Null'
        }
    },
    /**
     * Lookup297: pallet_bucket_nfts::pallet::Call<T>
     **/
    PalletBucketNftsCall: {
        _enum: {
            share_access: {
                recipient: 'AccountId20',
                bucket: 'H256',
                itemId: 'u32',
                readAccessRegex: 'Option<Bytes>',
            },
            update_read_access: {
                bucket: 'H256',
                itemId: 'u32',
                readAccessRegex: 'Option<Bytes>'
            }
        }
    },
    /**
     * Lookup299: pallet_nfts::pallet::Call<T, I>
     **/
    PalletNftsCall: {
        _enum: {
            create: {
                admin: 'AccountId20',
                config: 'PalletNftsCollectionConfig',
            },
            force_create: {
                owner: 'AccountId20',
                config: 'PalletNftsCollectionConfig',
            },
            destroy: {
                collection: 'u32',
                witness: 'PalletNftsDestroyWitness',
            },
            mint: {
                collection: 'u32',
                item: 'u32',
                mintTo: 'AccountId20',
                witnessData: 'Option<PalletNftsMintWitness>',
            },
            force_mint: {
                collection: 'u32',
                item: 'u32',
                mintTo: 'AccountId20',
                itemConfig: 'PalletNftsItemConfig',
            },
            burn: {
                collection: 'u32',
                item: 'u32',
            },
            transfer: {
                collection: 'u32',
                item: 'u32',
                dest: 'AccountId20',
            },
            redeposit: {
                collection: 'u32',
                items: 'Vec<u32>',
            },
            lock_item_transfer: {
                collection: 'u32',
                item: 'u32',
            },
            unlock_item_transfer: {
                collection: 'u32',
                item: 'u32',
            },
            lock_collection: {
                collection: 'u32',
                lockSettings: 'u64',
            },
            transfer_ownership: {
                collection: 'u32',
                newOwner: 'AccountId20',
            },
            set_team: {
                collection: 'u32',
                issuer: 'Option<AccountId20>',
                admin: 'Option<AccountId20>',
                freezer: 'Option<AccountId20>',
            },
            force_collection_owner: {
                collection: 'u32',
                owner: 'AccountId20',
            },
            force_collection_config: {
                collection: 'u32',
                config: 'PalletNftsCollectionConfig',
            },
            approve_transfer: {
                collection: 'u32',
                item: 'u32',
                delegate: 'AccountId20',
                maybeDeadline: 'Option<u32>',
            },
            cancel_approval: {
                collection: 'u32',
                item: 'u32',
                delegate: 'AccountId20',
            },
            clear_all_transfer_approvals: {
                collection: 'u32',
                item: 'u32',
            },
            lock_item_properties: {
                collection: 'u32',
                item: 'u32',
                lockMetadata: 'bool',
                lockAttributes: 'bool',
            },
            set_attribute: {
                collection: 'u32',
                maybeItem: 'Option<u32>',
                namespace: 'PalletNftsAttributeNamespace',
                key: 'Bytes',
                value: 'Bytes',
            },
            force_set_attribute: {
                setAs: 'Option<AccountId20>',
                collection: 'u32',
                maybeItem: 'Option<u32>',
                namespace: 'PalletNftsAttributeNamespace',
                key: 'Bytes',
                value: 'Bytes',
            },
            clear_attribute: {
                collection: 'u32',
                maybeItem: 'Option<u32>',
                namespace: 'PalletNftsAttributeNamespace',
                key: 'Bytes',
            },
            approve_item_attributes: {
                collection: 'u32',
                item: 'u32',
                delegate: 'AccountId20',
            },
            cancel_item_attributes_approval: {
                collection: 'u32',
                item: 'u32',
                delegate: 'AccountId20',
                witness: 'PalletNftsCancelAttributesApprovalWitness',
            },
            set_metadata: {
                collection: 'u32',
                item: 'u32',
                data: 'Bytes',
            },
            clear_metadata: {
                collection: 'u32',
                item: 'u32',
            },
            set_collection_metadata: {
                collection: 'u32',
                data: 'Bytes',
            },
            clear_collection_metadata: {
                collection: 'u32',
            },
            set_accept_ownership: {
                maybeCollection: 'Option<u32>',
            },
            set_collection_max_supply: {
                collection: 'u32',
                maxSupply: 'u32',
            },
            update_mint_settings: {
                collection: 'u32',
                mintSettings: 'PalletNftsMintSettings',
            },
            set_price: {
                collection: 'u32',
                item: 'u32',
                price: 'Option<u128>',
                whitelistedBuyer: 'Option<AccountId20>',
            },
            buy_item: {
                collection: 'u32',
                item: 'u32',
                bidPrice: 'u128',
            },
            pay_tips: {
                tips: 'Vec<PalletNftsItemTip>',
            },
            create_swap: {
                offeredCollection: 'u32',
                offeredItem: 'u32',
                desiredCollection: 'u32',
                maybeDesiredItem: 'Option<u32>',
                maybePrice: 'Option<PalletNftsPriceWithDirection>',
                duration: 'u32',
            },
            cancel_swap: {
                offeredCollection: 'u32',
                offeredItem: 'u32',
            },
            claim_swap: {
                sendCollection: 'u32',
                sendItem: 'u32',
                receiveCollection: 'u32',
                receiveItem: 'u32',
                witnessPrice: 'Option<PalletNftsPriceWithDirection>',
            },
            mint_pre_signed: {
                mintData: 'PalletNftsPreSignedMint',
                signature: 'FpAccountEthereumSignature',
                signer: 'AccountId20',
            },
            set_attributes_pre_signed: {
                data: 'PalletNftsPreSignedAttributes',
                signature: 'FpAccountEthereumSignature',
                signer: 'AccountId20'
            }
        }
    },
    /**
     * Lookup300: pallet_nfts::types::CollectionConfig<Price, BlockNumber, CollectionId>
     **/
    PalletNftsCollectionConfig: {
        settings: 'u64',
        maxSupply: 'Option<u32>',
        mintSettings: 'PalletNftsMintSettings'
    },
    /**
     * Lookup302: pallet_nfts::types::CollectionSetting
     **/
    PalletNftsCollectionSetting: {
        _enum: ['__Unused0', 'TransferableItems', 'UnlockedMetadata', '__Unused3', 'UnlockedAttributes', '__Unused5', '__Unused6', '__Unused7', 'UnlockedMaxSupply', '__Unused9', '__Unused10', '__Unused11', '__Unused12', '__Unused13', '__Unused14', '__Unused15', 'DepositRequired']
    },
    /**
     * Lookup303: pallet_nfts::types::MintSettings<Price, BlockNumber, CollectionId>
     **/
    PalletNftsMintSettings: {
        mintType: 'PalletNftsMintType',
        price: 'Option<u128>',
        startBlock: 'Option<u32>',
        endBlock: 'Option<u32>',
        defaultItemSettings: 'u64'
    },
    /**
     * Lookup304: pallet_nfts::types::MintType<CollectionId>
     **/
    PalletNftsMintType: {
        _enum: {
            Issuer: 'Null',
            Public: 'Null',
            HolderOf: 'u32'
        }
    },
    /**
     * Lookup306: pallet_nfts::types::ItemSetting
     **/
    PalletNftsItemSetting: {
        _enum: ['__Unused0', 'Transferable', 'UnlockedMetadata', '__Unused3', 'UnlockedAttributes']
    },
    /**
     * Lookup307: pallet_nfts::types::DestroyWitness
     **/
    PalletNftsDestroyWitness: {
        itemMetadatas: 'Compact<u32>',
        itemConfigs: 'Compact<u32>',
        attributes: 'Compact<u32>'
    },
    /**
     * Lookup309: pallet_nfts::types::MintWitness<ItemId, Balance>
     **/
    PalletNftsMintWitness: {
        ownedItem: 'Option<u32>',
        mintPrice: 'Option<u128>'
    },
    /**
     * Lookup310: pallet_nfts::types::ItemConfig
     **/
    PalletNftsItemConfig: {
        settings: 'u64'
    },
    /**
     * Lookup311: pallet_nfts::types::CancelAttributesApprovalWitness
     **/
    PalletNftsCancelAttributesApprovalWitness: {
        accountAttributes: 'u32'
    },
    /**
     * Lookup313: pallet_nfts::types::ItemTip<CollectionId, ItemId, fp_account::AccountId20, Amount>
     **/
    PalletNftsItemTip: {
        collection: 'u32',
        item: 'u32',
        receiver: 'AccountId20',
        amount: 'u128'
    },
    /**
     * Lookup315: pallet_nfts::types::PreSignedMint<CollectionId, ItemId, fp_account::AccountId20, Deadline, Balance>
     **/
    PalletNftsPreSignedMint: {
        collection: 'u32',
        item: 'u32',
        attributes: 'Vec<(Bytes,Bytes)>',
        metadata: 'Bytes',
        onlyAccount: 'Option<AccountId20>',
        deadline: 'u32',
        mintPrice: 'Option<u128>'
    },
    /**
     * Lookup316: pallet_nfts::types::PreSignedAttributes<CollectionId, ItemId, fp_account::AccountId20, Deadline>
     **/
    PalletNftsPreSignedAttributes: {
        collection: 'u32',
        item: 'u32',
        attributes: 'Vec<(Bytes,Bytes)>',
        namespace: 'PalletNftsAttributeNamespace',
        deadline: 'u32'
    },
    /**
     * Lookup317: pallet_sudo::pallet::Error<T>
     **/
    PalletSudoError: {
        _enum: ['RequireSudo']
    },
    /**
     * Lookup319: fp_rpc::TransactionStatus
     **/
    FpRpcTransactionStatus: {
        transactionHash: 'H256',
        transactionIndex: 'u32',
        from: 'H160',
        to: 'Option<H160>',
        contractAddress: 'Option<H160>',
        logs: 'Vec<EthereumLog>',
        logsBloom: 'EthbloomBloom'
    },
    /**
     * Lookup322: ethbloom::Bloom
     **/
    EthbloomBloom: '[u8;256]',
    /**
     * Lookup324: ethereum::receipt::ReceiptV3
     **/
    EthereumReceiptReceiptV3: {
        _enum: {
            Legacy: 'EthereumReceiptEip658ReceiptData',
            EIP2930: 'EthereumReceiptEip658ReceiptData',
            EIP1559: 'EthereumReceiptEip658ReceiptData'
        }
    },
    /**
     * Lookup325: ethereum::receipt::EIP658ReceiptData
     **/
    EthereumReceiptEip658ReceiptData: {
        statusCode: 'u8',
        usedGas: 'U256',
        logsBloom: 'EthbloomBloom',
        logs: 'Vec<EthereumLog>'
    },
    /**
     * Lookup326: ethereum::block::Block<ethereum::transaction::TransactionV2>
     **/
    EthereumBlock: {
        header: 'EthereumHeader',
        transactions: 'Vec<EthereumTransactionTransactionV2>',
        ommers: 'Vec<EthereumHeader>'
    },
    /**
     * Lookup327: ethereum::header::Header
     **/
    EthereumHeader: {
        parentHash: 'H256',
        ommersHash: 'H256',
        beneficiary: 'H160',
        stateRoot: 'H256',
        transactionsRoot: 'H256',
        receiptsRoot: 'H256',
        logsBloom: 'EthbloomBloom',
        difficulty: 'U256',
        number: 'U256',
        gasLimit: 'U256',
        gasUsed: 'U256',
        timestamp: 'u64',
        extraData: 'Bytes',
        mixHash: 'H256',
        nonce: 'EthereumTypesHashH64'
    },
    /**
     * Lookup328: ethereum_types::hash::H64
     **/
    EthereumTypesHashH64: '[u8;8]',
    /**
     * Lookup333: pallet_ethereum::pallet::Error<T>
     **/
    PalletEthereumError: {
        _enum: ['InvalidSignature', 'PreLogExists']
    },
    /**
     * Lookup334: pallet_evm::CodeMetadata
     **/
    PalletEvmCodeMetadata: {
        _alias: {
            size_: 'size',
            hash_: 'hash'
        },
        size_: 'u64',
        hash_: 'H256'
    },
    /**
     * Lookup336: pallet_evm::pallet::Error<T>
     **/
    PalletEvmError: {
        _enum: ['BalanceLow', 'FeeOverflow', 'PaymentOverflow', 'WithdrawFailed', 'GasPriceTooLow', 'InvalidNonce', 'GasLimitTooLow', 'GasLimitTooHigh', 'InvalidChainId', 'InvalidSignature', 'Reentrancy', 'TransactionMustComeFromEOA', 'Undefined']
    },
    /**
     * Lookup337: pallet_storage_providers::types::SignUpRequest<T>
     **/
    PalletStorageProvidersSignUpRequest: {
        spSignUpRequest: 'PalletStorageProvidersSignUpRequestSpParams',
        at: 'u32'
    },
    /**
     * Lookup338: pallet_storage_providers::types::SignUpRequestSpParams<T>
     **/
    PalletStorageProvidersSignUpRequestSpParams: {
        _enum: {
            BackupStorageProvider: 'PalletStorageProvidersBackupStorageProvider',
            MainStorageProvider: 'PalletStorageProvidersMainStorageProviderSignUpRequest'
        }
    },
    /**
     * Lookup339: pallet_storage_providers::types::BackupStorageProvider<T>
     **/
    PalletStorageProvidersBackupStorageProvider: {
        capacity: 'u64',
        capacityUsed: 'u64',
        multiaddresses: 'Vec<Bytes>',
        root: 'H256',
        lastCapacityChange: 'u32',
        ownerAccount: 'AccountId20',
        paymentAccount: 'AccountId20',
        reputationWeight: 'u32',
        signUpBlock: 'u32'
    },
    /**
     * Lookup340: pallet_storage_providers::types::MainStorageProviderSignUpRequest<T>
     **/
    PalletStorageProvidersMainStorageProviderSignUpRequest: {
        mspInfo: 'PalletStorageProvidersMainStorageProvider',
        valueProp: 'PalletStorageProvidersValueProposition'
    },
    /**
     * Lookup341: pallet_storage_providers::types::MainStorageProvider<T>
     **/
    PalletStorageProvidersMainStorageProvider: {
        capacity: 'u64',
        capacityUsed: 'u64',
        multiaddresses: 'Vec<Bytes>',
        amountOfBuckets: 'u128',
        amountOfValueProps: 'u32',
        lastCapacityChange: 'u32',
        ownerAccount: 'AccountId20',
        paymentAccount: 'AccountId20',
        signUpBlock: 'u32'
    },
    /**
     * Lookup342: pallet_storage_providers::types::Bucket<T>
     **/
    PalletStorageProvidersBucket: {
        _alias: {
            size_: 'size'
        },
        root: 'H256',
        userId: 'AccountId20',
        mspId: 'Option<H256>',
        private: 'bool',
        readAccessGroupId: 'Option<u32>',
        size_: 'u64',
        valuePropId: 'H256'
    },
    /**
     * Lookup346: pallet_storage_providers::pallet::Error<T>
     **/
    PalletStorageProvidersError: {
        _enum: ['AlreadyRegistered', 'SignUpNotRequested', 'SignUpRequestPending', 'NoMultiAddress', 'InvalidMultiAddress', 'StorageTooLow', 'NotEnoughBalance', 'CannotHoldDeposit', 'StorageStillInUse', 'SignOffPeriodNotPassed', 'RandomnessNotValidYet', 'SignUpRequestExpired', 'NewCapacityLessThanUsedStorage', 'NewCapacityEqualsCurrentCapacity', 'NewCapacityCantBeZero', 'NotEnoughTimePassed', 'NewUsedCapacityExceedsStorageCapacity', 'DepositTooLow', 'NotRegistered', 'NoUserId', 'NoBucketId', 'SpRegisteredButDataNotFound', 'BucketNotFound', 'BucketAlreadyExists', 'BucketNotEmpty', 'BucketsMovedAmountMismatch', 'AppendBucketToMspFailed', 'ProviderNotSlashable', 'TopUpNotRequired', 'BucketMustHaveMspForOperation', 'MultiAddressesMaxAmountReached', 'MultiAddressNotFound', 'MultiAddressAlreadyExists', 'LastMultiAddressCantBeRemoved', 'ValuePropositionNotFound', 'ValuePropositionAlreadyExists', 'ValuePropositionNotAvailable', 'CantDeactivateLastValueProp', 'ValuePropositionsDeletedAmountMismatch', 'FixedRatePaymentStreamNotFound', 'MspAlreadyAssignedToBucket', 'BucketSizeExceedsLimit', 'BucketHasNoValueProposition', 'MaxBlockNumberReached', 'OperationNotAllowedForInsolventProvider', 'DeleteProviderConditionsNotMet', 'CannotStopCycleWithNonDefaultRoot', 'BspOnlyOperation', 'MspOnlyOperation', 'InvalidEncodedFileMetadata', 'InvalidEncodedAccountId', 'PaymentStreamNotFound']
    },
    /**
     * Lookup347: pallet_file_system::types::StorageRequestMetadata<T>
     **/
    PalletFileSystemStorageRequestMetadata: {
        _alias: {
            size_: 'size'
        },
        requestedAt: 'u32',
        expiresAt: 'u32',
        owner: 'AccountId20',
        bucketId: 'H256',
        location: 'Bytes',
        fingerprint: 'H256',
        size_: 'u64',
        mspStatus: 'PalletFileSystemMspStorageRequestStatus',
        userPeerIds: 'Vec<Bytes>',
        bspsRequired: 'u32',
        bspsConfirmed: 'u32',
        bspsVolunteered: 'u32',
        depositPaid: 'u128'
    },
    /**
     * Lookup348: pallet_file_system::types::MspStorageRequestStatus<T>
     **/
    PalletFileSystemMspStorageRequestStatus: {
        _enum: {
            None: 'Null',
            Pending: 'H256',
            AcceptedNewFile: 'H256',
            AcceptedExistingFile: 'H256'
        }
    },
    /**
     * Lookup349: pallet_file_system::types::StorageRequestBspsMetadata<T>
     **/
    PalletFileSystemStorageRequestBspsMetadata: {
        confirmed: 'bool'
    },
    /**
     * Lookup351: pallet_file_system::types::PendingFileDeletionRequest<T>
     **/
    PalletFileSystemPendingFileDeletionRequest: {
        user: 'AccountId20',
        fileKey: 'H256',
        bucketId: 'H256',
        fileSize: 'u64',
        depositPaidForCreation: 'u128',
        queuePriorityChallenge: 'bool'
    },
    /**
     * Lookup353: pallet_file_system::types::PendingStopStoringRequest<T>
     **/
    PalletFileSystemPendingStopStoringRequest: {
        tickWhenRequested: 'u32',
        fileOwner: 'AccountId20',
        fileSize: 'u64'
    },
    /**
     * Lookup354: pallet_file_system::types::MoveBucketRequestMetadata<T>
     **/
    PalletFileSystemMoveBucketRequestMetadata: {
        requester: 'AccountId20',
        newMspId: 'H256',
        newValuePropId: 'H256'
    },
    /**
     * Lookup355: pallet_file_system::types::IncompleteStorageRequestMetadata<T>
     **/
    PalletFileSystemIncompleteStorageRequestMetadata: {
        owner: 'AccountId20',
        bucketId: 'H256',
        location: 'Bytes',
        fileSize: 'u64',
        fingerprint: 'H256',
        pendingBspRemovals: 'Vec<H256>',
        pendingBucketRemoval: 'bool'
    },
    /**
     * Lookup357: pallet_file_system::pallet::Error<T>
     **/
    PalletFileSystemError: {
        _enum: ['NotABsp', 'NotAMsp', 'NotASp', 'StorageRequestAlreadyRegistered', 'StorageRequestNotFound', 'StorageRequestExists', 'StorageRequestNotAuthorized', 'StorageRequestBspsRequiredFulfilled', 'TooManyStorageRequestResponses', 'IncompleteStorageRequestNotFound', 'ReplicationTargetCannotBeZero', 'ReplicationTargetExceedsMaximum', 'BspNotVolunteered', 'BspNotConfirmed', 'BspAlreadyConfirmed', 'BspAlreadyVolunteered', 'BspNotEligibleToVolunteer', 'InsufficientAvailableCapacity', 'NoFileKeysToConfirm', 'MspNotStoringBucket', 'NotSelectedMsp', 'MspAlreadyConfirmed', 'RequestWithoutMsp', 'MspAlreadyStoringBucket', 'BucketNotFound', 'BucketNotEmpty', 'NotBucketOwner', 'BucketIsBeingMoved', 'InvalidBucketIdFileKeyPair', 'ValuePropositionNotAvailable', 'CollectionNotFound', 'MoveBucketRequestNotFound', 'InvalidFileKeyMetadata', 'FileSizeCannotBeZero', 'ProviderNotStoringFile', 'FileHasActiveStorageRequest', 'FileHasIncompleteStorageRequest', 'BatchFileDeletionMustContainSingleBucket', 'DuplicateFileKeyInBatchFileDeletion', 'NoFileKeysToDelete', 'FailedToPushFileKeyToBucketDeletionVector', 'FailedToPushUserToBspDeletionVector', 'FailedToPushFileKeyToBspDeletionVector', 'PendingStopStoringRequestNotFound', 'MinWaitForStopStoringNotReached', 'PendingStopStoringRequestAlreadyExists', 'ExpectedNonInclusionProof', 'ExpectedInclusionProof', 'FixedRatePaymentStreamNotFound', 'DynamicRatePaymentStreamNotFound', 'OperationNotAllowedWithInsolventUser', 'UserNotInsolvent', 'OperationNotAllowedForInsolventProvider', 'InvalidSignature', 'InvalidProviderID', 'InvalidSignedOperation', 'NoGlobalReputationWeightSet', 'NoBspReputationWeightSet', 'CannotHoldDeposit', 'MaxTickNumberReached', 'ThresholdArithmeticError', 'RootNotUpdated', 'ImpossibleFailedToGetValue', 'FailedToQueryEarliestFileVolunteerTick', 'FailedToGetOwnerAccount', 'FailedToGetPaymentAccount', 'FailedToComputeFileKey', 'FailedToCreateFileMetadata', 'FileMetadataProcessingQueueFull']
    },
    /**
     * Lookup359: pallet_proofs_dealer::types::ProofSubmissionRecord<T>
     **/
    PalletProofsDealerProofSubmissionRecord: {
        lastTickProven: 'u32',
        nextTickToSubmitProofFor: 'u32'
    },
    /**
     * Lookup366: pallet_proofs_dealer::pallet::Error<T>
     **/
    PalletProofsDealerError: {
        _enum: ['NotProvider', 'ChallengesQueueOverflow', 'PriorityChallengesQueueOverflow', 'FeeChargeFailed', 'EmptyKeyProofs', 'ProviderRootNotFound', 'ZeroRoot', 'NoRecordOfLastSubmittedProof', 'ProviderStakeNotFound', 'ZeroStake', 'StakeCouldNotBeConverted', 'ChallengesTickNotReached', 'ChallengesTickTooOld', 'ChallengesTickTooLate', 'SeedNotFound', 'CheckpointChallengesNotFound', 'ForestProofVerificationFailed', 'IncorrectNumberOfKeyProofs', 'KeyProofNotFound', 'KeyProofVerificationFailed', 'FailedToApplyDelta', 'UnexpectedNumberOfRemoveMutations', 'FailedToUpdateProviderAfterKeyRemoval', 'TooManyValidProofSubmitters']
    },
    /**
     * Lookup368: pallet_payment_streams::types::FixedRatePaymentStream<T>
     **/
    PalletPaymentStreamsFixedRatePaymentStream: {
        rate: 'u128',
        lastChargedTick: 'u32',
        userDeposit: 'u128',
        outOfFundsTick: 'Option<u32>'
    },
    /**
     * Lookup369: pallet_payment_streams::types::DynamicRatePaymentStream<T>
     **/
    PalletPaymentStreamsDynamicRatePaymentStream: {
        amountProvided: 'u64',
        priceIndexWhenLastCharged: 'u128',
        userDeposit: 'u128',
        outOfFundsTick: 'Option<u32>'
    },
    /**
     * Lookup370: pallet_payment_streams::types::ProviderLastChargeableInfo<T>
     **/
    PalletPaymentStreamsProviderLastChargeableInfo: {
        lastChargeableTick: 'u32',
        priceIndex: 'u128'
    },
    /**
     * Lookup371: pallet_payment_streams::pallet::Error<T>
     **/
    PalletPaymentStreamsError: {
        _enum: ['PaymentStreamAlreadyExists', 'PaymentStreamNotFound', 'NotAProvider', 'ProviderInconsistencyError', 'CannotHoldDeposit', 'UpdateRateToSameRate', 'UpdateAmountToSameAmount', 'RateCantBeZero', 'AmountProvidedCantBeZero', 'LastChargedGreaterThanLastChargeable', 'InvalidLastChargeableBlockNumber', 'InvalidLastChargeablePriceIndex', 'ChargeOverflow', 'UserWithoutFunds', 'UserNotFlaggedAsWithoutFunds', 'CooldownPeriodNotPassed', 'UserHasRemainingDebt', 'ProviderInsolvent']
    },
    /**
     * Lookup372: pallet_bucket_nfts::pallet::Error<T>
     **/
    PalletBucketNftsError: {
        _enum: ['BucketIsNotPrivate', 'NotBucketOwner', 'NoCorrespondingCollection', 'ConvertBytesToBoundedVec']
    },
    /**
     * Lookup373: pallet_nfts::types::CollectionDetails<fp_account::AccountId20, DepositBalance>
     **/
    PalletNftsCollectionDetails: {
        owner: 'AccountId20',
        ownerDeposit: 'u128',
        items: 'u32',
        itemMetadatas: 'u32',
        itemConfigs: 'u32',
        attributes: 'u32'
    },
    /**
     * Lookup378: pallet_nfts::types::CollectionRole
     **/
    PalletNftsCollectionRole: {
        _enum: ['__Unused0', 'Issuer', 'Freezer', '__Unused3', 'Admin']
    },
    /**
     * Lookup379: pallet_nfts::types::ItemDetails<fp_account::AccountId20, pallet_nfts::types::ItemDeposit<DepositBalance, fp_account::AccountId20>, bounded_collections::bounded_btree_map::BoundedBTreeMap<fp_account::AccountId20, Option<T>, S>>
     **/
    PalletNftsItemDetails: {
        owner: 'AccountId20',
        approvals: 'BTreeMap<AccountId20, Option<u32>>',
        deposit: 'PalletNftsItemDeposit'
    },
    /**
     * Lookup380: pallet_nfts::types::ItemDeposit<DepositBalance, fp_account::AccountId20>
     **/
    PalletNftsItemDeposit: {
        account: 'AccountId20',
        amount: 'u128'
    },
    /**
     * Lookup385: pallet_nfts::types::CollectionMetadata<Deposit, StringLimit>
     **/
    PalletNftsCollectionMetadata: {
        deposit: 'u128',
        data: 'Bytes'
    },
    /**
     * Lookup386: pallet_nfts::types::ItemMetadata<pallet_nfts::types::ItemMetadataDeposit<DepositBalance, fp_account::AccountId20>, StringLimit>
     **/
    PalletNftsItemMetadata: {
        deposit: 'PalletNftsItemMetadataDeposit',
        data: 'Bytes'
    },
    /**
     * Lookup387: pallet_nfts::types::ItemMetadataDeposit<DepositBalance, fp_account::AccountId20>
     **/
    PalletNftsItemMetadataDeposit: {
        account: 'Option<AccountId20>',
        amount: 'u128'
    },
    /**
     * Lookup390: pallet_nfts::types::AttributeDeposit<DepositBalance, fp_account::AccountId20>
     **/
    PalletNftsAttributeDeposit: {
        account: 'Option<AccountId20>',
        amount: 'u128'
    },
    /**
     * Lookup394: pallet_nfts::types::PendingSwap<CollectionId, ItemId, pallet_nfts::types::PriceWithDirection<Amount>, Deadline>
     **/
    PalletNftsPendingSwap: {
        desiredCollection: 'u32',
        desiredItem: 'Option<u32>',
        price: 'Option<PalletNftsPriceWithDirection>',
        deadline: 'u32'
    },
    /**
     * Lookup396: pallet_nfts::types::PalletFeature
     **/
    PalletNftsPalletFeature: {
        _enum: ['__Unused0', 'Trading', 'Attributes', '__Unused3', 'Approvals', '__Unused5', '__Unused6', '__Unused7', 'Swaps']
    },
    /**
     * Lookup397: pallet_nfts::pallet::Error<T, I>
     **/
    PalletNftsError: {
        _enum: ['NoPermission', 'UnknownCollection', 'AlreadyExists', 'ApprovalExpired', 'WrongOwner', 'BadWitness', 'CollectionIdInUse', 'ItemsNonTransferable', 'NotDelegate', 'WrongDelegate', 'Unapproved', 'Unaccepted', 'ItemLocked', 'LockedItemAttributes', 'LockedCollectionAttributes', 'LockedItemMetadata', 'LockedCollectionMetadata', 'MaxSupplyReached', 'MaxSupplyLocked', 'MaxSupplyTooSmall', 'UnknownItem', 'UnknownSwap', 'MetadataNotFound', 'AttributeNotFound', 'NotForSale', 'BidTooLow', 'ReachedApprovalLimit', 'DeadlineExpired', 'WrongDuration', 'MethodDisabled', 'WrongSetting', 'InconsistentItemConfig', 'NoConfig', 'RolesNotCleared', 'MintNotStarted', 'MintEnded', 'AlreadyClaimed', 'IncorrectData', 'WrongOrigin', 'WrongSignature', 'IncorrectMetadata', 'MaxAttributesLimitReached', 'WrongNamespace', 'CollectionNotEmpty', 'WitnessRequired']
    },
    /**
     * Lookup400: frame_system::extensions::check_non_zero_sender::CheckNonZeroSender<T>
     **/
    FrameSystemExtensionsCheckNonZeroSender: 'Null',
    /**
     * Lookup401: frame_system::extensions::check_spec_version::CheckSpecVersion<T>
     **/
    FrameSystemExtensionsCheckSpecVersion: 'Null',
    /**
     * Lookup402: frame_system::extensions::check_tx_version::CheckTxVersion<T>
     **/
    FrameSystemExtensionsCheckTxVersion: 'Null',
    /**
     * Lookup403: frame_system::extensions::check_genesis::CheckGenesis<T>
     **/
    FrameSystemExtensionsCheckGenesis: 'Null',
    /**
     * Lookup406: frame_system::extensions::check_nonce::CheckNonce<T>
     **/
    FrameSystemExtensionsCheckNonce: 'Compact<u32>',
    /**
     * Lookup407: frame_system::extensions::check_weight::CheckWeight<T>
     **/
    FrameSystemExtensionsCheckWeight: 'Null',
    /**
     * Lookup408: pallet_transaction_payment::ChargeTransactionPayment<T>
     **/
    PalletTransactionPaymentChargeTransactionPayment: 'Compact<u128>',
    /**
     * Lookup409: frame_metadata_hash_extension::CheckMetadataHash<T>
     **/
    FrameMetadataHashExtensionCheckMetadataHash: {
        mode: 'FrameMetadataHashExtensionMode'
    },
    /**
     * Lookup410: frame_metadata_hash_extension::Mode
     **/
    FrameMetadataHashExtensionMode: {
        _enum: ['Disabled', 'Enabled']
    },
    /**
     * Lookup412: sh_solochain_evm_runtime::Runtime
     **/
    ShSolochainEvmRuntimeRuntime: 'Null'
};
//# sourceMappingURL=lookup.js.map