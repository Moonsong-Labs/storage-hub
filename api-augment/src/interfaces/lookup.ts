// Auto-generated via `yarn polkadot-types-from-defs`, do not edit
/* eslint-disable */

/* eslint-disable sort-keys */

export default {
  /**
   * Lookup3: frame_system::AccountInfo<Nonce, pallet_balances::types::AccountData<Balance>>
   **/
  FrameSystemAccountInfo: {
    nonce: "u32",
    consumers: "u32",
    providers: "u32",
    sufficients: "u32",
    data: "PalletBalancesAccountData"
  },
  /**
   * Lookup5: pallet_balances::types::AccountData<Balance>
   **/
  PalletBalancesAccountData: {
    free: "u128",
    reserved: "u128",
    frozen: "u128",
    flags: "u128"
  },
  /**
   * Lookup9: frame_support::dispatch::PerDispatchClass<sp_weights::weight_v2::Weight>
   **/
  FrameSupportDispatchPerDispatchClassWeight: {
    normal: "SpWeightsWeightV2Weight",
    operational: "SpWeightsWeightV2Weight",
    mandatory: "SpWeightsWeightV2Weight"
  },
  /**
   * Lookup10: sp_weights::weight_v2::Weight
   **/
  SpWeightsWeightV2Weight: {
    refTime: "Compact<u64>",
    proofSize: "Compact<u64>"
  },
  /**
   * Lookup15: sp_runtime::generic::digest::Digest
   **/
  SpRuntimeDigest: {
    logs: "Vec<SpRuntimeDigestDigestItem>"
  },
  /**
   * Lookup17: sp_runtime::generic::digest::DigestItem
   **/
  SpRuntimeDigestDigestItem: {
    _enum: {
      Other: "Bytes",
      __Unused1: "Null",
      __Unused2: "Null",
      __Unused3: "Null",
      Consensus: "([u8;4],Bytes)",
      Seal: "([u8;4],Bytes)",
      PreRuntime: "([u8;4],Bytes)",
      __Unused7: "Null",
      RuntimeEnvironmentUpdated: "Null"
    }
  },
  /**
   * Lookup20: frame_system::EventRecord<storage_hub_runtime::RuntimeEvent, primitive_types::H256>
   **/
  FrameSystemEventRecord: {
    phase: "FrameSystemPhase",
    event: "Event",
    topics: "Vec<H256>"
  },
  /**
   * Lookup22: frame_system::pallet::Event<T>
   **/
  FrameSystemEvent: {
    _enum: {
      ExtrinsicSuccess: {
        dispatchInfo: "FrameSupportDispatchDispatchInfo"
      },
      ExtrinsicFailed: {
        dispatchError: "SpRuntimeDispatchError",
        dispatchInfo: "FrameSupportDispatchDispatchInfo"
      },
      CodeUpdated: "Null",
      NewAccount: {
        account: "AccountId32"
      },
      KilledAccount: {
        account: "AccountId32"
      },
      Remarked: {
        _alias: {
          hash_: "hash"
        },
        sender: "AccountId32",
        hash_: "H256"
      },
      UpgradeAuthorized: {
        codeHash: "H256",
        checkVersion: "bool"
      }
    }
  },
  /**
   * Lookup23: frame_support::dispatch::DispatchInfo
   **/
  FrameSupportDispatchDispatchInfo: {
    weight: "SpWeightsWeightV2Weight",
    class: "FrameSupportDispatchDispatchClass",
    paysFee: "FrameSupportDispatchPays"
  },
  /**
   * Lookup24: frame_support::dispatch::DispatchClass
   **/
  FrameSupportDispatchDispatchClass: {
    _enum: ["Normal", "Operational", "Mandatory"]
  },
  /**
   * Lookup25: frame_support::dispatch::Pays
   **/
  FrameSupportDispatchPays: {
    _enum: ["Yes", "No"]
  },
  /**
   * Lookup26: sp_runtime::DispatchError
   **/
  SpRuntimeDispatchError: {
    _enum: {
      Other: "Null",
      CannotLookup: "Null",
      BadOrigin: "Null",
      Module: "SpRuntimeModuleError",
      ConsumerRemaining: "Null",
      NoProviders: "Null",
      TooManyConsumers: "Null",
      Token: "SpRuntimeTokenError",
      Arithmetic: "SpArithmeticArithmeticError",
      Transactional: "SpRuntimeTransactionalError",
      Exhausted: "Null",
      Corruption: "Null",
      Unavailable: "Null",
      RootNotAllowed: "Null"
    }
  },
  /**
   * Lookup27: sp_runtime::ModuleError
   **/
  SpRuntimeModuleError: {
    index: "u8",
    error: "[u8;4]"
  },
  /**
   * Lookup28: sp_runtime::TokenError
   **/
  SpRuntimeTokenError: {
    _enum: [
      "FundsUnavailable",
      "OnlyProvider",
      "BelowMinimum",
      "CannotCreate",
      "UnknownAsset",
      "Frozen",
      "Unsupported",
      "CannotCreateHold",
      "NotExpendable",
      "Blocked"
    ]
  },
  /**
   * Lookup29: sp_arithmetic::ArithmeticError
   **/
  SpArithmeticArithmeticError: {
    _enum: ["Underflow", "Overflow", "DivisionByZero"]
  },
  /**
   * Lookup30: sp_runtime::TransactionalError
   **/
  SpRuntimeTransactionalError: {
    _enum: ["LimitReached", "NoLayer"]
  },
  /**
   * Lookup31: cumulus_pallet_parachain_system::pallet::Event<T>
   **/
  CumulusPalletParachainSystemEvent: {
    _enum: {
      ValidationFunctionStored: "Null",
      ValidationFunctionApplied: {
        relayChainBlockNum: "u32"
      },
      ValidationFunctionDiscarded: "Null",
      DownwardMessagesReceived: {
        count: "u32"
      },
      DownwardMessagesProcessed: {
        weightUsed: "SpWeightsWeightV2Weight",
        dmqHead: "H256"
      },
      UpwardMessageSent: {
        messageHash: "Option<[u8;32]>"
      }
    }
  },
  /**
   * Lookup33: pallet_balances::pallet::Event<T, I>
   **/
  PalletBalancesEvent: {
    _enum: {
      Endowed: {
        account: "AccountId32",
        freeBalance: "u128"
      },
      DustLost: {
        account: "AccountId32",
        amount: "u128"
      },
      Transfer: {
        from: "AccountId32",
        to: "AccountId32",
        amount: "u128"
      },
      BalanceSet: {
        who: "AccountId32",
        free: "u128"
      },
      Reserved: {
        who: "AccountId32",
        amount: "u128"
      },
      Unreserved: {
        who: "AccountId32",
        amount: "u128"
      },
      ReserveRepatriated: {
        from: "AccountId32",
        to: "AccountId32",
        amount: "u128",
        destinationStatus: "FrameSupportTokensMiscBalanceStatus"
      },
      Deposit: {
        who: "AccountId32",
        amount: "u128"
      },
      Withdraw: {
        who: "AccountId32",
        amount: "u128"
      },
      Slashed: {
        who: "AccountId32",
        amount: "u128"
      },
      Minted: {
        who: "AccountId32",
        amount: "u128"
      },
      Burned: {
        who: "AccountId32",
        amount: "u128"
      },
      Suspended: {
        who: "AccountId32",
        amount: "u128"
      },
      Restored: {
        who: "AccountId32",
        amount: "u128"
      },
      Upgraded: {
        who: "AccountId32"
      },
      Issued: {
        amount: "u128"
      },
      Rescinded: {
        amount: "u128"
      },
      Locked: {
        who: "AccountId32",
        amount: "u128"
      },
      Unlocked: {
        who: "AccountId32",
        amount: "u128"
      },
      Frozen: {
        who: "AccountId32",
        amount: "u128"
      },
      Thawed: {
        who: "AccountId32",
        amount: "u128"
      },
      TotalIssuanceForced: {
        _alias: {
          new_: "new"
        },
        old: "u128",
        new_: "u128"
      }
    }
  },
  /**
   * Lookup34: frame_support::traits::tokens::misc::BalanceStatus
   **/
  FrameSupportTokensMiscBalanceStatus: {
    _enum: ["Free", "Reserved"]
  },
  /**
   * Lookup35: pallet_transaction_payment::pallet::Event<T>
   **/
  PalletTransactionPaymentEvent: {
    _enum: {
      TransactionFeePaid: {
        who: "AccountId32",
        actualFee: "u128",
        tip: "u128"
      }
    }
  },
  /**
   * Lookup36: pallet_sudo::pallet::Event<T>
   **/
  PalletSudoEvent: {
    _enum: {
      Sudid: {
        sudoResult: "Result<Null, SpRuntimeDispatchError>"
      },
      KeyChanged: {
        _alias: {
          new_: "new"
        },
        old: "Option<AccountId32>",
        new_: "AccountId32"
      },
      KeyRemoved: "Null",
      SudoAsDone: {
        sudoResult: "Result<Null, SpRuntimeDispatchError>"
      }
    }
  },
  /**
   * Lookup40: pallet_collator_selection::pallet::Event<T>
   **/
  PalletCollatorSelectionEvent: {
    _enum: {
      NewInvulnerables: {
        invulnerables: "Vec<AccountId32>"
      },
      InvulnerableAdded: {
        accountId: "AccountId32"
      },
      InvulnerableRemoved: {
        accountId: "AccountId32"
      },
      NewDesiredCandidates: {
        desiredCandidates: "u32"
      },
      NewCandidacyBond: {
        bondAmount: "u128"
      },
      CandidateAdded: {
        accountId: "AccountId32",
        deposit: "u128"
      },
      CandidateBondUpdated: {
        accountId: "AccountId32",
        deposit: "u128"
      },
      CandidateRemoved: {
        accountId: "AccountId32"
      },
      CandidateReplaced: {
        _alias: {
          new_: "new"
        },
        old: "AccountId32",
        new_: "AccountId32",
        deposit: "u128"
      },
      InvalidInvulnerableSkipped: {
        accountId: "AccountId32"
      }
    }
  },
  /**
   * Lookup42: pallet_session::pallet::Event
   **/
  PalletSessionEvent: {
    _enum: {
      NewSession: {
        sessionIndex: "u32"
      }
    }
  },
  /**
   * Lookup43: cumulus_pallet_xcmp_queue::pallet::Event<T>
   **/
  CumulusPalletXcmpQueueEvent: {
    _enum: {
      XcmpMessageSent: {
        messageHash: "[u8;32]"
      }
    }
  },
  /**
   * Lookup44: pallet_xcm::pallet::Event<T>
   **/
  PalletXcmEvent: {
    _enum: {
      Attempted: {
        outcome: "StagingXcmV4TraitsOutcome"
      },
      Sent: {
        origin: "StagingXcmV4Location",
        destination: "StagingXcmV4Location",
        message: "StagingXcmV4Xcm",
        messageId: "[u8;32]"
      },
      UnexpectedResponse: {
        origin: "StagingXcmV4Location",
        queryId: "u64"
      },
      ResponseReady: {
        queryId: "u64",
        response: "StagingXcmV4Response"
      },
      Notified: {
        queryId: "u64",
        palletIndex: "u8",
        callIndex: "u8"
      },
      NotifyOverweight: {
        queryId: "u64",
        palletIndex: "u8",
        callIndex: "u8",
        actualWeight: "SpWeightsWeightV2Weight",
        maxBudgetedWeight: "SpWeightsWeightV2Weight"
      },
      NotifyDispatchError: {
        queryId: "u64",
        palletIndex: "u8",
        callIndex: "u8"
      },
      NotifyDecodeFailed: {
        queryId: "u64",
        palletIndex: "u8",
        callIndex: "u8"
      },
      InvalidResponder: {
        origin: "StagingXcmV4Location",
        queryId: "u64",
        expectedLocation: "Option<StagingXcmV4Location>"
      },
      InvalidResponderVersion: {
        origin: "StagingXcmV4Location",
        queryId: "u64"
      },
      ResponseTaken: {
        queryId: "u64"
      },
      AssetsTrapped: {
        _alias: {
          hash_: "hash"
        },
        hash_: "H256",
        origin: "StagingXcmV4Location",
        assets: "XcmVersionedAssets"
      },
      VersionChangeNotified: {
        destination: "StagingXcmV4Location",
        result: "u32",
        cost: "StagingXcmV4AssetAssets",
        messageId: "[u8;32]"
      },
      SupportedVersionChanged: {
        location: "StagingXcmV4Location",
        version: "u32"
      },
      NotifyTargetSendFail: {
        location: "StagingXcmV4Location",
        queryId: "u64",
        error: "XcmV3TraitsError"
      },
      NotifyTargetMigrationFail: {
        location: "XcmVersionedLocation",
        queryId: "u64"
      },
      InvalidQuerierVersion: {
        origin: "StagingXcmV4Location",
        queryId: "u64"
      },
      InvalidQuerier: {
        origin: "StagingXcmV4Location",
        queryId: "u64",
        expectedQuerier: "StagingXcmV4Location",
        maybeActualQuerier: "Option<StagingXcmV4Location>"
      },
      VersionNotifyStarted: {
        destination: "StagingXcmV4Location",
        cost: "StagingXcmV4AssetAssets",
        messageId: "[u8;32]"
      },
      VersionNotifyRequested: {
        destination: "StagingXcmV4Location",
        cost: "StagingXcmV4AssetAssets",
        messageId: "[u8;32]"
      },
      VersionNotifyUnrequested: {
        destination: "StagingXcmV4Location",
        cost: "StagingXcmV4AssetAssets",
        messageId: "[u8;32]"
      },
      FeesPaid: {
        paying: "StagingXcmV4Location",
        fees: "StagingXcmV4AssetAssets"
      },
      AssetsClaimed: {
        _alias: {
          hash_: "hash"
        },
        hash_: "H256",
        origin: "StagingXcmV4Location",
        assets: "XcmVersionedAssets"
      },
      VersionMigrationFinished: {
        version: "u32"
      }
    }
  },
  /**
   * Lookup45: staging_xcm::v4::traits::Outcome
   **/
  StagingXcmV4TraitsOutcome: {
    _enum: {
      Complete: {
        used: "SpWeightsWeightV2Weight"
      },
      Incomplete: {
        used: "SpWeightsWeightV2Weight",
        error: "XcmV3TraitsError"
      },
      Error: {
        error: "XcmV3TraitsError"
      }
    }
  },
  /**
   * Lookup46: xcm::v3::traits::Error
   **/
  XcmV3TraitsError: {
    _enum: {
      Overflow: "Null",
      Unimplemented: "Null",
      UntrustedReserveLocation: "Null",
      UntrustedTeleportLocation: "Null",
      LocationFull: "Null",
      LocationNotInvertible: "Null",
      BadOrigin: "Null",
      InvalidLocation: "Null",
      AssetNotFound: "Null",
      FailedToTransactAsset: "Null",
      NotWithdrawable: "Null",
      LocationCannotHold: "Null",
      ExceedsMaxMessageSize: "Null",
      DestinationUnsupported: "Null",
      Transport: "Null",
      Unroutable: "Null",
      UnknownClaim: "Null",
      FailedToDecode: "Null",
      MaxWeightInvalid: "Null",
      NotHoldingFees: "Null",
      TooExpensive: "Null",
      Trap: "u64",
      ExpectationFalse: "Null",
      PalletNotFound: "Null",
      NameMismatch: "Null",
      VersionIncompatible: "Null",
      HoldingWouldOverflow: "Null",
      ExportError: "Null",
      ReanchorFailed: "Null",
      NoDeal: "Null",
      FeesNotMet: "Null",
      LockError: "Null",
      NoPermission: "Null",
      Unanchored: "Null",
      NotDepositable: "Null",
      UnhandledXcmVersion: "Null",
      WeightLimitReached: "SpWeightsWeightV2Weight",
      Barrier: "Null",
      WeightNotComputable: "Null",
      ExceedsStackLimit: "Null"
    }
  },
  /**
   * Lookup47: staging_xcm::v4::location::Location
   **/
  StagingXcmV4Location: {
    parents: "u8",
    interior: "StagingXcmV4Junctions"
  },
  /**
   * Lookup48: staging_xcm::v4::junctions::Junctions
   **/
  StagingXcmV4Junctions: {
    _enum: {
      Here: "Null",
      X1: "[Lookup50;1]",
      X2: "[Lookup50;2]",
      X3: "[Lookup50;3]",
      X4: "[Lookup50;4]",
      X5: "[Lookup50;5]",
      X6: "[Lookup50;6]",
      X7: "[Lookup50;7]",
      X8: "[Lookup50;8]"
    }
  },
  /**
   * Lookup50: staging_xcm::v4::junction::Junction
   **/
  StagingXcmV4Junction: {
    _enum: {
      Parachain: "Compact<u32>",
      AccountId32: {
        network: "Option<StagingXcmV4JunctionNetworkId>",
        id: "[u8;32]"
      },
      AccountIndex64: {
        network: "Option<StagingXcmV4JunctionNetworkId>",
        index: "Compact<u64>"
      },
      AccountKey20: {
        network: "Option<StagingXcmV4JunctionNetworkId>",
        key: "[u8;20]"
      },
      PalletInstance: "u8",
      GeneralIndex: "Compact<u128>",
      GeneralKey: {
        length: "u8",
        data: "[u8;32]"
      },
      OnlyChild: "Null",
      Plurality: {
        id: "XcmV3JunctionBodyId",
        part: "XcmV3JunctionBodyPart"
      },
      GlobalConsensus: "StagingXcmV4JunctionNetworkId"
    }
  },
  /**
   * Lookup53: staging_xcm::v4::junction::NetworkId
   **/
  StagingXcmV4JunctionNetworkId: {
    _enum: {
      ByGenesis: "[u8;32]",
      ByFork: {
        blockNumber: "u64",
        blockHash: "[u8;32]"
      },
      Polkadot: "Null",
      Kusama: "Null",
      Westend: "Null",
      Rococo: "Null",
      Wococo: "Null",
      Ethereum: {
        chainId: "Compact<u64>"
      },
      BitcoinCore: "Null",
      BitcoinCash: "Null",
      PolkadotBulletin: "Null"
    }
  },
  /**
   * Lookup56: xcm::v3::junction::BodyId
   **/
  XcmV3JunctionBodyId: {
    _enum: {
      Unit: "Null",
      Moniker: "[u8;4]",
      Index: "Compact<u32>",
      Executive: "Null",
      Technical: "Null",
      Legislative: "Null",
      Judicial: "Null",
      Defense: "Null",
      Administration: "Null",
      Treasury: "Null"
    }
  },
  /**
   * Lookup57: xcm::v3::junction::BodyPart
   **/
  XcmV3JunctionBodyPart: {
    _enum: {
      Voice: "Null",
      Members: {
        count: "Compact<u32>"
      },
      Fraction: {
        nom: "Compact<u32>",
        denom: "Compact<u32>"
      },
      AtLeastProportion: {
        nom: "Compact<u32>",
        denom: "Compact<u32>"
      },
      MoreThanProportion: {
        nom: "Compact<u32>",
        denom: "Compact<u32>"
      }
    }
  },
  /**
   * Lookup65: staging_xcm::v4::Xcm<Call>
   **/
  StagingXcmV4Xcm: "Vec<StagingXcmV4Instruction>",
  /**
   * Lookup67: staging_xcm::v4::Instruction<Call>
   **/
  StagingXcmV4Instruction: {
    _enum: {
      WithdrawAsset: "StagingXcmV4AssetAssets",
      ReserveAssetDeposited: "StagingXcmV4AssetAssets",
      ReceiveTeleportedAsset: "StagingXcmV4AssetAssets",
      QueryResponse: {
        queryId: "Compact<u64>",
        response: "StagingXcmV4Response",
        maxWeight: "SpWeightsWeightV2Weight",
        querier: "Option<StagingXcmV4Location>"
      },
      TransferAsset: {
        assets: "StagingXcmV4AssetAssets",
        beneficiary: "StagingXcmV4Location"
      },
      TransferReserveAsset: {
        assets: "StagingXcmV4AssetAssets",
        dest: "StagingXcmV4Location",
        xcm: "StagingXcmV4Xcm"
      },
      Transact: {
        originKind: "XcmV2OriginKind",
        requireWeightAtMost: "SpWeightsWeightV2Weight",
        call: "XcmDoubleEncoded"
      },
      HrmpNewChannelOpenRequest: {
        sender: "Compact<u32>",
        maxMessageSize: "Compact<u32>",
        maxCapacity: "Compact<u32>"
      },
      HrmpChannelAccepted: {
        recipient: "Compact<u32>"
      },
      HrmpChannelClosing: {
        initiator: "Compact<u32>",
        sender: "Compact<u32>",
        recipient: "Compact<u32>"
      },
      ClearOrigin: "Null",
      DescendOrigin: "StagingXcmV4Junctions",
      ReportError: "StagingXcmV4QueryResponseInfo",
      DepositAsset: {
        assets: "StagingXcmV4AssetAssetFilter",
        beneficiary: "StagingXcmV4Location"
      },
      DepositReserveAsset: {
        assets: "StagingXcmV4AssetAssetFilter",
        dest: "StagingXcmV4Location",
        xcm: "StagingXcmV4Xcm"
      },
      ExchangeAsset: {
        give: "StagingXcmV4AssetAssetFilter",
        want: "StagingXcmV4AssetAssets",
        maximal: "bool"
      },
      InitiateReserveWithdraw: {
        assets: "StagingXcmV4AssetAssetFilter",
        reserve: "StagingXcmV4Location",
        xcm: "StagingXcmV4Xcm"
      },
      InitiateTeleport: {
        assets: "StagingXcmV4AssetAssetFilter",
        dest: "StagingXcmV4Location",
        xcm: "StagingXcmV4Xcm"
      },
      ReportHolding: {
        responseInfo: "StagingXcmV4QueryResponseInfo",
        assets: "StagingXcmV4AssetAssetFilter"
      },
      BuyExecution: {
        fees: "StagingXcmV4Asset",
        weightLimit: "XcmV3WeightLimit"
      },
      RefundSurplus: "Null",
      SetErrorHandler: "StagingXcmV4Xcm",
      SetAppendix: "StagingXcmV4Xcm",
      ClearError: "Null",
      ClaimAsset: {
        assets: "StagingXcmV4AssetAssets",
        ticket: "StagingXcmV4Location"
      },
      Trap: "Compact<u64>",
      SubscribeVersion: {
        queryId: "Compact<u64>",
        maxResponseWeight: "SpWeightsWeightV2Weight"
      },
      UnsubscribeVersion: "Null",
      BurnAsset: "StagingXcmV4AssetAssets",
      ExpectAsset: "StagingXcmV4AssetAssets",
      ExpectOrigin: "Option<StagingXcmV4Location>",
      ExpectError: "Option<(u32,XcmV3TraitsError)>",
      ExpectTransactStatus: "XcmV3MaybeErrorCode",
      QueryPallet: {
        moduleName: "Bytes",
        responseInfo: "StagingXcmV4QueryResponseInfo"
      },
      ExpectPallet: {
        index: "Compact<u32>",
        name: "Bytes",
        moduleName: "Bytes",
        crateMajor: "Compact<u32>",
        minCrateMinor: "Compact<u32>"
      },
      ReportTransactStatus: "StagingXcmV4QueryResponseInfo",
      ClearTransactStatus: "Null",
      UniversalOrigin: "StagingXcmV4Junction",
      ExportMessage: {
        network: "StagingXcmV4JunctionNetworkId",
        destination: "StagingXcmV4Junctions",
        xcm: "StagingXcmV4Xcm"
      },
      LockAsset: {
        asset: "StagingXcmV4Asset",
        unlocker: "StagingXcmV4Location"
      },
      UnlockAsset: {
        asset: "StagingXcmV4Asset",
        target: "StagingXcmV4Location"
      },
      NoteUnlockable: {
        asset: "StagingXcmV4Asset",
        owner: "StagingXcmV4Location"
      },
      RequestUnlock: {
        asset: "StagingXcmV4Asset",
        locker: "StagingXcmV4Location"
      },
      SetFeesMode: {
        jitWithdraw: "bool"
      },
      SetTopic: "[u8;32]",
      ClearTopic: "Null",
      AliasOrigin: "StagingXcmV4Location",
      UnpaidExecution: {
        weightLimit: "XcmV3WeightLimit",
        checkOrigin: "Option<StagingXcmV4Location>"
      }
    }
  },
  /**
   * Lookup68: staging_xcm::v4::asset::Assets
   **/
  StagingXcmV4AssetAssets: "Vec<StagingXcmV4Asset>",
  /**
   * Lookup70: staging_xcm::v4::asset::Asset
   **/
  StagingXcmV4Asset: {
    id: "StagingXcmV4AssetAssetId",
    fun: "StagingXcmV4AssetFungibility"
  },
  /**
   * Lookup71: staging_xcm::v4::asset::AssetId
   **/
  StagingXcmV4AssetAssetId: "StagingXcmV4Location",
  /**
   * Lookup72: staging_xcm::v4::asset::Fungibility
   **/
  StagingXcmV4AssetFungibility: {
    _enum: {
      Fungible: "Compact<u128>",
      NonFungible: "StagingXcmV4AssetAssetInstance"
    }
  },
  /**
   * Lookup73: staging_xcm::v4::asset::AssetInstance
   **/
  StagingXcmV4AssetAssetInstance: {
    _enum: {
      Undefined: "Null",
      Index: "Compact<u128>",
      Array4: "[u8;4]",
      Array8: "[u8;8]",
      Array16: "[u8;16]",
      Array32: "[u8;32]"
    }
  },
  /**
   * Lookup76: staging_xcm::v4::Response
   **/
  StagingXcmV4Response: {
    _enum: {
      Null: "Null",
      Assets: "StagingXcmV4AssetAssets",
      ExecutionResult: "Option<(u32,XcmV3TraitsError)>",
      Version: "u32",
      PalletsInfo: "Vec<StagingXcmV4PalletInfo>",
      DispatchResult: "XcmV3MaybeErrorCode"
    }
  },
  /**
   * Lookup80: staging_xcm::v4::PalletInfo
   **/
  StagingXcmV4PalletInfo: {
    index: "Compact<u32>",
    name: "Bytes",
    moduleName: "Bytes",
    major: "Compact<u32>",
    minor: "Compact<u32>",
    patch: "Compact<u32>"
  },
  /**
   * Lookup83: xcm::v3::MaybeErrorCode
   **/
  XcmV3MaybeErrorCode: {
    _enum: {
      Success: "Null",
      Error: "Bytes",
      TruncatedError: "Bytes"
    }
  },
  /**
   * Lookup86: xcm::v2::OriginKind
   **/
  XcmV2OriginKind: {
    _enum: ["Native", "SovereignAccount", "Superuser", "Xcm"]
  },
  /**
   * Lookup87: xcm::double_encoded::DoubleEncoded<T>
   **/
  XcmDoubleEncoded: {
    encoded: "Bytes"
  },
  /**
   * Lookup88: staging_xcm::v4::QueryResponseInfo
   **/
  StagingXcmV4QueryResponseInfo: {
    destination: "StagingXcmV4Location",
    queryId: "Compact<u64>",
    maxWeight: "SpWeightsWeightV2Weight"
  },
  /**
   * Lookup89: staging_xcm::v4::asset::AssetFilter
   **/
  StagingXcmV4AssetAssetFilter: {
    _enum: {
      Definite: "StagingXcmV4AssetAssets",
      Wild: "StagingXcmV4AssetWildAsset"
    }
  },
  /**
   * Lookup90: staging_xcm::v4::asset::WildAsset
   **/
  StagingXcmV4AssetWildAsset: {
    _enum: {
      All: "Null",
      AllOf: {
        id: "StagingXcmV4AssetAssetId",
        fun: "StagingXcmV4AssetWildFungibility"
      },
      AllCounted: "Compact<u32>",
      AllOfCounted: {
        id: "StagingXcmV4AssetAssetId",
        fun: "StagingXcmV4AssetWildFungibility",
        count: "Compact<u32>"
      }
    }
  },
  /**
   * Lookup91: staging_xcm::v4::asset::WildFungibility
   **/
  StagingXcmV4AssetWildFungibility: {
    _enum: ["Fungible", "NonFungible"]
  },
  /**
   * Lookup92: xcm::v3::WeightLimit
   **/
  XcmV3WeightLimit: {
    _enum: {
      Unlimited: "Null",
      Limited: "SpWeightsWeightV2Weight"
    }
  },
  /**
   * Lookup93: xcm::VersionedAssets
   **/
  XcmVersionedAssets: {
    _enum: {
      __Unused0: "Null",
      V2: "XcmV2MultiassetMultiAssets",
      __Unused2: "Null",
      V3: "XcmV3MultiassetMultiAssets",
      V4: "StagingXcmV4AssetAssets"
    }
  },
  /**
   * Lookup94: xcm::v2::multiasset::MultiAssets
   **/
  XcmV2MultiassetMultiAssets: "Vec<XcmV2MultiAsset>",
  /**
   * Lookup96: xcm::v2::multiasset::MultiAsset
   **/
  XcmV2MultiAsset: {
    id: "XcmV2MultiassetAssetId",
    fun: "XcmV2MultiassetFungibility"
  },
  /**
   * Lookup97: xcm::v2::multiasset::AssetId
   **/
  XcmV2MultiassetAssetId: {
    _enum: {
      Concrete: "XcmV2MultiLocation",
      Abstract: "Bytes"
    }
  },
  /**
   * Lookup98: xcm::v2::multilocation::MultiLocation
   **/
  XcmV2MultiLocation: {
    parents: "u8",
    interior: "XcmV2MultilocationJunctions"
  },
  /**
   * Lookup99: xcm::v2::multilocation::Junctions
   **/
  XcmV2MultilocationJunctions: {
    _enum: {
      Here: "Null",
      X1: "XcmV2Junction",
      X2: "(XcmV2Junction,XcmV2Junction)",
      X3: "(XcmV2Junction,XcmV2Junction,XcmV2Junction)",
      X4: "(XcmV2Junction,XcmV2Junction,XcmV2Junction,XcmV2Junction)",
      X5: "(XcmV2Junction,XcmV2Junction,XcmV2Junction,XcmV2Junction,XcmV2Junction)",
      X6: "(XcmV2Junction,XcmV2Junction,XcmV2Junction,XcmV2Junction,XcmV2Junction,XcmV2Junction)",
      X7: "(XcmV2Junction,XcmV2Junction,XcmV2Junction,XcmV2Junction,XcmV2Junction,XcmV2Junction,XcmV2Junction)",
      X8: "(XcmV2Junction,XcmV2Junction,XcmV2Junction,XcmV2Junction,XcmV2Junction,XcmV2Junction,XcmV2Junction,XcmV2Junction)"
    }
  },
  /**
   * Lookup100: xcm::v2::junction::Junction
   **/
  XcmV2Junction: {
    _enum: {
      Parachain: "Compact<u32>",
      AccountId32: {
        network: "XcmV2NetworkId",
        id: "[u8;32]"
      },
      AccountIndex64: {
        network: "XcmV2NetworkId",
        index: "Compact<u64>"
      },
      AccountKey20: {
        network: "XcmV2NetworkId",
        key: "[u8;20]"
      },
      PalletInstance: "u8",
      GeneralIndex: "Compact<u128>",
      GeneralKey: "Bytes",
      OnlyChild: "Null",
      Plurality: {
        id: "XcmV2BodyId",
        part: "XcmV2BodyPart"
      }
    }
  },
  /**
   * Lookup101: xcm::v2::NetworkId
   **/
  XcmV2NetworkId: {
    _enum: {
      Any: "Null",
      Named: "Bytes",
      Polkadot: "Null",
      Kusama: "Null"
    }
  },
  /**
   * Lookup103: xcm::v2::BodyId
   **/
  XcmV2BodyId: {
    _enum: {
      Unit: "Null",
      Named: "Bytes",
      Index: "Compact<u32>",
      Executive: "Null",
      Technical: "Null",
      Legislative: "Null",
      Judicial: "Null",
      Defense: "Null",
      Administration: "Null",
      Treasury: "Null"
    }
  },
  /**
   * Lookup104: xcm::v2::BodyPart
   **/
  XcmV2BodyPart: {
    _enum: {
      Voice: "Null",
      Members: {
        count: "Compact<u32>"
      },
      Fraction: {
        nom: "Compact<u32>",
        denom: "Compact<u32>"
      },
      AtLeastProportion: {
        nom: "Compact<u32>",
        denom: "Compact<u32>"
      },
      MoreThanProportion: {
        nom: "Compact<u32>",
        denom: "Compact<u32>"
      }
    }
  },
  /**
   * Lookup105: xcm::v2::multiasset::Fungibility
   **/
  XcmV2MultiassetFungibility: {
    _enum: {
      Fungible: "Compact<u128>",
      NonFungible: "XcmV2MultiassetAssetInstance"
    }
  },
  /**
   * Lookup106: xcm::v2::multiasset::AssetInstance
   **/
  XcmV2MultiassetAssetInstance: {
    _enum: {
      Undefined: "Null",
      Index: "Compact<u128>",
      Array4: "[u8;4]",
      Array8: "[u8;8]",
      Array16: "[u8;16]",
      Array32: "[u8;32]",
      Blob: "Bytes"
    }
  },
  /**
   * Lookup107: xcm::v3::multiasset::MultiAssets
   **/
  XcmV3MultiassetMultiAssets: "Vec<XcmV3MultiAsset>",
  /**
   * Lookup109: xcm::v3::multiasset::MultiAsset
   **/
  XcmV3MultiAsset: {
    id: "XcmV3MultiassetAssetId",
    fun: "XcmV3MultiassetFungibility"
  },
  /**
   * Lookup110: xcm::v3::multiasset::AssetId
   **/
  XcmV3MultiassetAssetId: {
    _enum: {
      Concrete: "StagingXcmV3MultiLocation",
      Abstract: "[u8;32]"
    }
  },
  /**
   * Lookup111: staging_xcm::v3::multilocation::MultiLocation
   **/
  StagingXcmV3MultiLocation: {
    parents: "u8",
    interior: "XcmV3Junctions"
  },
  /**
   * Lookup112: xcm::v3::junctions::Junctions
   **/
  XcmV3Junctions: {
    _enum: {
      Here: "Null",
      X1: "XcmV3Junction",
      X2: "(XcmV3Junction,XcmV3Junction)",
      X3: "(XcmV3Junction,XcmV3Junction,XcmV3Junction)",
      X4: "(XcmV3Junction,XcmV3Junction,XcmV3Junction,XcmV3Junction)",
      X5: "(XcmV3Junction,XcmV3Junction,XcmV3Junction,XcmV3Junction,XcmV3Junction)",
      X6: "(XcmV3Junction,XcmV3Junction,XcmV3Junction,XcmV3Junction,XcmV3Junction,XcmV3Junction)",
      X7: "(XcmV3Junction,XcmV3Junction,XcmV3Junction,XcmV3Junction,XcmV3Junction,XcmV3Junction,XcmV3Junction)",
      X8: "(XcmV3Junction,XcmV3Junction,XcmV3Junction,XcmV3Junction,XcmV3Junction,XcmV3Junction,XcmV3Junction,XcmV3Junction)"
    }
  },
  /**
   * Lookup113: xcm::v3::junction::Junction
   **/
  XcmV3Junction: {
    _enum: {
      Parachain: "Compact<u32>",
      AccountId32: {
        network: "Option<XcmV3JunctionNetworkId>",
        id: "[u8;32]"
      },
      AccountIndex64: {
        network: "Option<XcmV3JunctionNetworkId>",
        index: "Compact<u64>"
      },
      AccountKey20: {
        network: "Option<XcmV3JunctionNetworkId>",
        key: "[u8;20]"
      },
      PalletInstance: "u8",
      GeneralIndex: "Compact<u128>",
      GeneralKey: {
        length: "u8",
        data: "[u8;32]"
      },
      OnlyChild: "Null",
      Plurality: {
        id: "XcmV3JunctionBodyId",
        part: "XcmV3JunctionBodyPart"
      },
      GlobalConsensus: "XcmV3JunctionNetworkId"
    }
  },
  /**
   * Lookup115: xcm::v3::junction::NetworkId
   **/
  XcmV3JunctionNetworkId: {
    _enum: {
      ByGenesis: "[u8;32]",
      ByFork: {
        blockNumber: "u64",
        blockHash: "[u8;32]"
      },
      Polkadot: "Null",
      Kusama: "Null",
      Westend: "Null",
      Rococo: "Null",
      Wococo: "Null",
      Ethereum: {
        chainId: "Compact<u64>"
      },
      BitcoinCore: "Null",
      BitcoinCash: "Null",
      PolkadotBulletin: "Null"
    }
  },
  /**
   * Lookup116: xcm::v3::multiasset::Fungibility
   **/
  XcmV3MultiassetFungibility: {
    _enum: {
      Fungible: "Compact<u128>",
      NonFungible: "XcmV3MultiassetAssetInstance"
    }
  },
  /**
   * Lookup117: xcm::v3::multiasset::AssetInstance
   **/
  XcmV3MultiassetAssetInstance: {
    _enum: {
      Undefined: "Null",
      Index: "Compact<u128>",
      Array4: "[u8;4]",
      Array8: "[u8;8]",
      Array16: "[u8;16]",
      Array32: "[u8;32]"
    }
  },
  /**
   * Lookup118: xcm::VersionedLocation
   **/
  XcmVersionedLocation: {
    _enum: {
      __Unused0: "Null",
      V2: "XcmV2MultiLocation",
      __Unused2: "Null",
      V3: "StagingXcmV3MultiLocation",
      V4: "StagingXcmV4Location"
    }
  },
  /**
   * Lookup119: cumulus_pallet_xcm::pallet::Event<T>
   **/
  CumulusPalletXcmEvent: {
    _enum: {
      InvalidFormat: "[u8;32]",
      UnsupportedVersion: "[u8;32]",
      ExecutedDownward: "([u8;32],StagingXcmV4TraitsOutcome)"
    }
  },
  /**
   * Lookup120: pallet_message_queue::pallet::Event<T>
   **/
  PalletMessageQueueEvent: {
    _enum: {
      ProcessingFailed: {
        id: "H256",
        origin: "CumulusPrimitivesCoreAggregateMessageOrigin",
        error: "FrameSupportMessagesProcessMessageError"
      },
      Processed: {
        id: "H256",
        origin: "CumulusPrimitivesCoreAggregateMessageOrigin",
        weightUsed: "SpWeightsWeightV2Weight",
        success: "bool"
      },
      OverweightEnqueued: {
        id: "[u8;32]",
        origin: "CumulusPrimitivesCoreAggregateMessageOrigin",
        pageIndex: "u32",
        messageIndex: "u32"
      },
      PageReaped: {
        origin: "CumulusPrimitivesCoreAggregateMessageOrigin",
        index: "u32"
      }
    }
  },
  /**
   * Lookup121: cumulus_primitives_core::AggregateMessageOrigin
   **/
  CumulusPrimitivesCoreAggregateMessageOrigin: {
    _enum: {
      Here: "Null",
      Parent: "Null",
      Sibling: "u32"
    }
  },
  /**
   * Lookup123: frame_support::traits::messages::ProcessMessageError
   **/
  FrameSupportMessagesProcessMessageError: {
    _enum: {
      BadFormat: "Null",
      Corrupt: "Null",
      Unsupported: "Null",
      Overweight: "SpWeightsWeightV2Weight",
      Yield: "Null"
    }
  },
  /**
   * Lookup124: pallet_storage_providers::pallet::Event<T>
   **/
  PalletStorageProvidersEvent: {
    _enum: {
      MspRequestSignUpSuccess: {
        who: "AccountId32",
        multiaddresses: "Vec<Bytes>",
        capacity: "u32",
        valueProp: "PalletStorageProvidersValueProposition"
      },
      MspSignUpSuccess: {
        who: "AccountId32",
        multiaddresses: "Vec<Bytes>",
        capacity: "u32",
        valueProp: "PalletStorageProvidersValueProposition"
      },
      BspRequestSignUpSuccess: {
        who: "AccountId32",
        multiaddresses: "Vec<Bytes>",
        capacity: "u32"
      },
      BspSignUpSuccess: {
        who: "AccountId32",
        multiaddresses: "Vec<Bytes>",
        capacity: "u32"
      },
      SignUpRequestCanceled: {
        who: "AccountId32"
      },
      MspSignOffSuccess: {
        who: "AccountId32"
      },
      BspSignOffSuccess: {
        who: "AccountId32"
      },
      CapacityChanged: {
        who: "AccountId32",
        oldCapacity: "u32",
        newCapacity: "u32",
        nextBlockWhenChangeAllowed: "u32"
      },
      Slashed: {
        providerId: "H256",
        amountSlashed: "u128"
      }
    }
  },
  /**
   * Lookup128: pallet_storage_providers::types::ValueProposition<T>
   **/
  PalletStorageProvidersValueProposition: {
    identifier: "H256",
    dataLimit: "u32",
    protocols: "Vec<Bytes>"
  },
  /**
   * Lookup130: pallet_file_system::pallet::Event<T>
   **/
  PalletFileSystemEvent: {
    _enum: {
      NewBucket: {
        who: "AccountId32",
        mspId: "H256",
        bucketId: "H256",
        name: "Bytes",
        collectionId: "Option<u32>",
        private: "bool"
      },
      BucketPrivacyUpdated: {
        who: "AccountId32",
        bucketId: "H256",
        collectionId: "Option<u32>",
        private: "bool"
      },
      NewCollectionAndAssociation: {
        who: "AccountId32",
        bucketId: "H256",
        collectionId: "u32"
      },
      NewStorageRequest: {
        _alias: {
          size_: "size"
        },
        who: "AccountId32",
        fileKey: "H256",
        bucketId: "H256",
        location: "Bytes",
        fingerprint: "H256",
        size_: "u32",
        peerIds: "Vec<Bytes>"
      },
      AcceptedBspVolunteer: {
        _alias: {
          size_: "size"
        },
        bspId: "H256",
        bucketId: "H256",
        location: "Bytes",
        fingerprint: "H256",
        multiaddresses: "Vec<Bytes>",
        owner: "AccountId32",
        size_: "u32"
      },
      BspConfirmedStoring: {
        who: "AccountId32",
        bspId: "H256",
        fileKeys: "Vec<H256>",
        newRoot: "H256"
      },
      StorageRequestExpired: {
        fileKey: "H256"
      },
      StorageRequestRevoked: {
        fileKey: "H256"
      },
      BspStoppedStoring: {
        bspId: "H256",
        fileKey: "H256",
        newRoot: "H256",
        owner: "AccountId32",
        location: "Bytes"
      },
      FailedToQueuePriorityChallenge: {
        user: "AccountId32",
        fileKey: "H256"
      },
      FileDeletionRequest: {
        user: "AccountId32",
        fileKey: "H256",
        bucketId: "H256",
        mspId: "H256",
        proofOfInclusion: "bool"
      },
      ProofSubmittedForPendingFileDeletionRequest: {
        mspId: "H256",
        user: "AccountId32",
        fileKey: "H256",
        bucketId: "H256",
        proofOfInclusion: "bool"
      },
      BspChallengeCycleInitialised: {
        who: "AccountId32",
        bspId: "H256"
      }
    }
  },
  /**
   * Lookup135: pallet_proofs_dealer::pallet::Event<T>
   **/
  PalletProofsDealerEvent: {
    _enum: {
      NewChallenge: {
        who: "AccountId32",
        keyChallenged: "H256"
      },
      ProofAccepted: {
        provider: "H256",
        proof: "PalletProofsDealerProof"
      },
      NewChallengeSeed: {
        challengesTicker: "u32",
        seed: "H256"
      },
      NewCheckpointChallenge: {
        challengesTicker: "u32",
        challenges: "Vec<(H256,Option<ShpTraitsTrieRemoveMutation>)>"
      },
      SlashableProvider: {
        provider: "H256"
      },
      NewChallengeCycleInitialised: {
        currentTick: "u32",
        provider: "H256",
        maybeProviderAccount: "Option<AccountId32>"
      }
    }
  },
  /**
   * Lookup136: pallet_proofs_dealer::types::Proof<T>
   **/
  PalletProofsDealerProof: {
    forestProof: "SpTrieStorageProofCompactProof",
    keyProofs: "BTreeMap<H256, PalletProofsDealerKeyProof>"
  },
  /**
   * Lookup137: sp_trie::storage_proof::CompactProof
   **/
  SpTrieStorageProofCompactProof: {
    encodedNodes: "Vec<Bytes>"
  },
  /**
   * Lookup140: pallet_proofs_dealer::types::KeyProof<T>
   **/
  PalletProofsDealerKeyProof: {
    proof: "ShpFileKeyVerifierFileKeyProof",
    challengeCount: "u32"
  },
  /**
   * Lookup141: shp_file_key_verifier::types::FileKeyProof
   **/
  ShpFileKeyVerifierFileKeyProof: {
    fileMetadata: "ShpFileMetadataFileMetadata",
    proof: "SpTrieStorageProofCompactProof"
  },
  /**
   * Lookup142: shp_file_metadata::FileMetadata
   **/
  ShpFileMetadataFileMetadata: {
    owner: "Bytes",
    bucketId: "Bytes",
    location: "Bytes",
    fileSize: "Compact<u64>",
    fingerprint: "ShpFileMetadataFingerprint"
  },
  /**
   * Lookup143: shp_file_metadata::Fingerprint
   **/
  ShpFileMetadataFingerprint: "[u8;32]",
  /**
   * Lookup149: shp_traits::TrieRemoveMutation
   **/
  ShpTraitsTrieRemoveMutation: "Null",
  /**
   * Lookup151: pallet_randomness::pallet::Event<T>
   **/
  PalletRandomnessEvent: {
    _enum: {
      NewOneEpochAgoRandomnessAvailable: {
        randomnessSeed: "H256",
        fromEpoch: "u64",
        validUntilBlock: "u32"
      }
    }
  },
  /**
   * Lookup152: pallet_payment_streams::pallet::Event<T>
   **/
  PalletPaymentStreamsEvent: {
    _enum: {
      FixedRatePaymentStreamCreated: {
        userAccount: "AccountId32",
        providerId: "H256",
        rate: "u128"
      },
      FixedRatePaymentStreamUpdated: {
        userAccount: "AccountId32",
        providerId: "H256",
        newRate: "u128"
      },
      FixedRatePaymentStreamDeleted: {
        userAccount: "AccountId32",
        providerId: "H256"
      },
      DynamicRatePaymentStreamCreated: {
        userAccount: "AccountId32",
        providerId: "H256",
        amountProvided: "u32"
      },
      DynamicRatePaymentStreamUpdated: {
        userAccount: "AccountId32",
        providerId: "H256",
        newAmountProvided: "u32"
      },
      DynamicRatePaymentStreamDeleted: {
        userAccount: "AccountId32",
        providerId: "H256"
      },
      PaymentStreamCharged: {
        userAccount: "AccountId32",
        providerId: "H256",
        amount: "u128"
      },
      LastChargeableInfoUpdated: {
        providerId: "H256",
        lastChargeableBlock: "u32",
        lastChargeablePriceIndex: "u128"
      },
      UserWithoutFunds: {
        who: "AccountId32"
      }
    }
  },
  /**
   * Lookup153: pallet_bucket_nfts::pallet::Event<T>
   **/
  PalletBucketNftsEvent: {
    _enum: {
      AccessShared: {
        issuer: "AccountId32",
        recipient: "AccountId32"
      },
      ItemReadAccessUpdated: {
        admin: "AccountId32",
        bucket: "H256",
        itemId: "u32"
      },
      ItemBurned: {
        account: "AccountId32",
        bucket: "H256",
        itemId: "u32"
      }
    }
  },
  /**
   * Lookup154: pallet_nfts::pallet::Event<T, I>
   **/
  PalletNftsEvent: {
    _enum: {
      Created: {
        collection: "u32",
        creator: "AccountId32",
        owner: "AccountId32"
      },
      ForceCreated: {
        collection: "u32",
        owner: "AccountId32"
      },
      Destroyed: {
        collection: "u32"
      },
      Issued: {
        collection: "u32",
        item: "u32",
        owner: "AccountId32"
      },
      Transferred: {
        collection: "u32",
        item: "u32",
        from: "AccountId32",
        to: "AccountId32"
      },
      Burned: {
        collection: "u32",
        item: "u32",
        owner: "AccountId32"
      },
      ItemTransferLocked: {
        collection: "u32",
        item: "u32"
      },
      ItemTransferUnlocked: {
        collection: "u32",
        item: "u32"
      },
      ItemPropertiesLocked: {
        collection: "u32",
        item: "u32",
        lockMetadata: "bool",
        lockAttributes: "bool"
      },
      CollectionLocked: {
        collection: "u32"
      },
      OwnerChanged: {
        collection: "u32",
        newOwner: "AccountId32"
      },
      TeamChanged: {
        collection: "u32",
        issuer: "Option<AccountId32>",
        admin: "Option<AccountId32>",
        freezer: "Option<AccountId32>"
      },
      TransferApproved: {
        collection: "u32",
        item: "u32",
        owner: "AccountId32",
        delegate: "AccountId32",
        deadline: "Option<u32>"
      },
      ApprovalCancelled: {
        collection: "u32",
        item: "u32",
        owner: "AccountId32",
        delegate: "AccountId32"
      },
      AllApprovalsCancelled: {
        collection: "u32",
        item: "u32",
        owner: "AccountId32"
      },
      CollectionConfigChanged: {
        collection: "u32"
      },
      CollectionMetadataSet: {
        collection: "u32",
        data: "Bytes"
      },
      CollectionMetadataCleared: {
        collection: "u32"
      },
      ItemMetadataSet: {
        collection: "u32",
        item: "u32",
        data: "Bytes"
      },
      ItemMetadataCleared: {
        collection: "u32",
        item: "u32"
      },
      Redeposited: {
        collection: "u32",
        successfulItems: "Vec<u32>"
      },
      AttributeSet: {
        collection: "u32",
        maybeItem: "Option<u32>",
        key: "Bytes",
        value: "Bytes",
        namespace: "PalletNftsAttributeNamespace"
      },
      AttributeCleared: {
        collection: "u32",
        maybeItem: "Option<u32>",
        key: "Bytes",
        namespace: "PalletNftsAttributeNamespace"
      },
      ItemAttributesApprovalAdded: {
        collection: "u32",
        item: "u32",
        delegate: "AccountId32"
      },
      ItemAttributesApprovalRemoved: {
        collection: "u32",
        item: "u32",
        delegate: "AccountId32"
      },
      OwnershipAcceptanceChanged: {
        who: "AccountId32",
        maybeCollection: "Option<u32>"
      },
      CollectionMaxSupplySet: {
        collection: "u32",
        maxSupply: "u32"
      },
      CollectionMintSettingsUpdated: {
        collection: "u32"
      },
      NextCollectionIdIncremented: {
        nextId: "Option<u32>"
      },
      ItemPriceSet: {
        collection: "u32",
        item: "u32",
        price: "u128",
        whitelistedBuyer: "Option<AccountId32>"
      },
      ItemPriceRemoved: {
        collection: "u32",
        item: "u32"
      },
      ItemBought: {
        collection: "u32",
        item: "u32",
        price: "u128",
        seller: "AccountId32",
        buyer: "AccountId32"
      },
      TipSent: {
        collection: "u32",
        item: "u32",
        sender: "AccountId32",
        receiver: "AccountId32",
        amount: "u128"
      },
      SwapCreated: {
        offeredCollection: "u32",
        offeredItem: "u32",
        desiredCollection: "u32",
        desiredItem: "Option<u32>",
        price: "Option<PalletNftsPriceWithDirection>",
        deadline: "u32"
      },
      SwapCancelled: {
        offeredCollection: "u32",
        offeredItem: "u32",
        desiredCollection: "u32",
        desiredItem: "Option<u32>",
        price: "Option<PalletNftsPriceWithDirection>",
        deadline: "u32"
      },
      SwapClaimed: {
        sentCollection: "u32",
        sentItem: "u32",
        sentItemOwner: "AccountId32",
        receivedCollection: "u32",
        receivedItem: "u32",
        receivedItemOwner: "AccountId32",
        price: "Option<PalletNftsPriceWithDirection>",
        deadline: "u32"
      },
      PreSignedAttributesSet: {
        collection: "u32",
        item: "u32",
        namespace: "PalletNftsAttributeNamespace"
      },
      PalletAttributeSet: {
        collection: "u32",
        item: "Option<u32>",
        attribute: "PalletNftsPalletAttributes",
        value: "Bytes"
      }
    }
  },
  /**
   * Lookup158: pallet_nfts::types::AttributeNamespace<sp_core::crypto::AccountId32>
   **/
  PalletNftsAttributeNamespace: {
    _enum: {
      Pallet: "Null",
      CollectionOwner: "Null",
      ItemOwner: "Null",
      Account: "AccountId32"
    }
  },
  /**
   * Lookup160: pallet_nfts::types::PriceWithDirection<Amount>
   **/
  PalletNftsPriceWithDirection: {
    amount: "u128",
    direction: "PalletNftsPriceDirection"
  },
  /**
   * Lookup161: pallet_nfts::types::PriceDirection
   **/
  PalletNftsPriceDirection: {
    _enum: ["Send", "Receive"]
  },
  /**
   * Lookup162: pallet_nfts::types::PalletAttributes<CollectionId>
   **/
  PalletNftsPalletAttributes: {
    _enum: {
      UsedToClaim: "u32",
      TransferDisabled: "Null"
    }
  },
  /**
   * Lookup163: frame_system::Phase
   **/
  FrameSystemPhase: {
    _enum: {
      ApplyExtrinsic: "u32",
      Finalization: "Null",
      Initialization: "Null"
    }
  },
  /**
   * Lookup166: frame_system::LastRuntimeUpgradeInfo
   **/
  FrameSystemLastRuntimeUpgradeInfo: {
    specVersion: "Compact<u32>",
    specName: "Text"
  },
  /**
   * Lookup168: frame_system::CodeUpgradeAuthorization<T>
   **/
  FrameSystemCodeUpgradeAuthorization: {
    codeHash: "H256",
    checkVersion: "bool"
  },
  /**
   * Lookup169: frame_system::pallet::Call<T>
   **/
  FrameSystemCall: {
    _enum: {
      remark: {
        remark: "Bytes"
      },
      set_heap_pages: {
        pages: "u64"
      },
      set_code: {
        code: "Bytes"
      },
      set_code_without_checks: {
        code: "Bytes"
      },
      set_storage: {
        items: "Vec<(Bytes,Bytes)>"
      },
      kill_storage: {
        _alias: {
          keys_: "keys"
        },
        keys_: "Vec<Bytes>"
      },
      kill_prefix: {
        prefix: "Bytes",
        subkeys: "u32"
      },
      remark_with_event: {
        remark: "Bytes"
      },
      __Unused8: "Null",
      authorize_upgrade: {
        codeHash: "H256"
      },
      authorize_upgrade_without_checks: {
        codeHash: "H256"
      },
      apply_authorized_upgrade: {
        code: "Bytes"
      }
    }
  },
  /**
   * Lookup172: frame_system::limits::BlockWeights
   **/
  FrameSystemLimitsBlockWeights: {
    baseBlock: "SpWeightsWeightV2Weight",
    maxBlock: "SpWeightsWeightV2Weight",
    perClass: "FrameSupportDispatchPerDispatchClassWeightsPerClass"
  },
  /**
   * Lookup173: frame_support::dispatch::PerDispatchClass<frame_system::limits::WeightsPerClass>
   **/
  FrameSupportDispatchPerDispatchClassWeightsPerClass: {
    normal: "FrameSystemLimitsWeightsPerClass",
    operational: "FrameSystemLimitsWeightsPerClass",
    mandatory: "FrameSystemLimitsWeightsPerClass"
  },
  /**
   * Lookup174: frame_system::limits::WeightsPerClass
   **/
  FrameSystemLimitsWeightsPerClass: {
    baseExtrinsic: "SpWeightsWeightV2Weight",
    maxExtrinsic: "Option<SpWeightsWeightV2Weight>",
    maxTotal: "Option<SpWeightsWeightV2Weight>",
    reserved: "Option<SpWeightsWeightV2Weight>"
  },
  /**
   * Lookup176: frame_system::limits::BlockLength
   **/
  FrameSystemLimitsBlockLength: {
    max: "FrameSupportDispatchPerDispatchClassU32"
  },
  /**
   * Lookup177: frame_support::dispatch::PerDispatchClass<T>
   **/
  FrameSupportDispatchPerDispatchClassU32: {
    normal: "u32",
    operational: "u32",
    mandatory: "u32"
  },
  /**
   * Lookup178: sp_weights::RuntimeDbWeight
   **/
  SpWeightsRuntimeDbWeight: {
    read: "u64",
    write: "u64"
  },
  /**
   * Lookup179: sp_version::RuntimeVersion
   **/
  SpVersionRuntimeVersion: {
    specName: "Text",
    implName: "Text",
    authoringVersion: "u32",
    specVersion: "u32",
    implVersion: "u32",
    apis: "Vec<([u8;8],u32)>",
    transactionVersion: "u32",
    stateVersion: "u8"
  },
  /**
   * Lookup184: frame_system::pallet::Error<T>
   **/
  FrameSystemError: {
    _enum: [
      "InvalidSpecName",
      "SpecVersionNeedsToIncrease",
      "FailedToExtractRuntimeVersion",
      "NonDefaultComposite",
      "NonZeroRefCount",
      "CallFiltered",
      "MultiBlockMigrationsOngoing",
      "NothingAuthorized",
      "Unauthorized"
    ]
  },
  /**
   * Lookup186: cumulus_pallet_parachain_system::unincluded_segment::Ancestor<primitive_types::H256>
   **/
  CumulusPalletParachainSystemUnincludedSegmentAncestor: {
    usedBandwidth: "CumulusPalletParachainSystemUnincludedSegmentUsedBandwidth",
    paraHeadHash: "Option<H256>",
    consumedGoAheadSignal: "Option<PolkadotPrimitivesV6UpgradeGoAhead>"
  },
  /**
   * Lookup187: cumulus_pallet_parachain_system::unincluded_segment::UsedBandwidth
   **/
  CumulusPalletParachainSystemUnincludedSegmentUsedBandwidth: {
    umpMsgCount: "u32",
    umpTotalBytes: "u32",
    hrmpOutgoing: "BTreeMap<u32, CumulusPalletParachainSystemUnincludedSegmentHrmpChannelUpdate>"
  },
  /**
   * Lookup189: cumulus_pallet_parachain_system::unincluded_segment::HrmpChannelUpdate
   **/
  CumulusPalletParachainSystemUnincludedSegmentHrmpChannelUpdate: {
    msgCount: "u32",
    totalBytes: "u32"
  },
  /**
   * Lookup194: polkadot_primitives::v6::UpgradeGoAhead
   **/
  PolkadotPrimitivesV6UpgradeGoAhead: {
    _enum: ["Abort", "GoAhead"]
  },
  /**
   * Lookup195: cumulus_pallet_parachain_system::unincluded_segment::SegmentTracker<primitive_types::H256>
   **/
  CumulusPalletParachainSystemUnincludedSegmentSegmentTracker: {
    usedBandwidth: "CumulusPalletParachainSystemUnincludedSegmentUsedBandwidth",
    hrmpWatermark: "Option<u32>",
    consumedGoAheadSignal: "Option<PolkadotPrimitivesV6UpgradeGoAhead>"
  },
  /**
   * Lookup196: polkadot_primitives::v6::PersistedValidationData<primitive_types::H256, N>
   **/
  PolkadotPrimitivesV6PersistedValidationData: {
    parentHead: "Bytes",
    relayParentNumber: "u32",
    relayParentStorageRoot: "H256",
    maxPovSize: "u32"
  },
  /**
   * Lookup199: polkadot_primitives::v6::UpgradeRestriction
   **/
  PolkadotPrimitivesV6UpgradeRestriction: {
    _enum: ["Present"]
  },
  /**
   * Lookup200: sp_trie::storage_proof::StorageProof
   **/
  SpTrieStorageProof: {
    trieNodes: "BTreeSet<Bytes>"
  },
  /**
   * Lookup202: cumulus_pallet_parachain_system::relay_state_snapshot::MessagingStateSnapshot
   **/
  CumulusPalletParachainSystemRelayStateSnapshotMessagingStateSnapshot: {
    dmqMqcHead: "H256",
    relayDispatchQueueRemainingCapacity:
      "CumulusPalletParachainSystemRelayStateSnapshotRelayDispatchQueueRemainingCapacity",
    ingressChannels: "Vec<(u32,PolkadotPrimitivesV6AbridgedHrmpChannel)>",
    egressChannels: "Vec<(u32,PolkadotPrimitivesV6AbridgedHrmpChannel)>"
  },
  /**
   * Lookup203: cumulus_pallet_parachain_system::relay_state_snapshot::RelayDispatchQueueRemainingCapacity
   **/
  CumulusPalletParachainSystemRelayStateSnapshotRelayDispatchQueueRemainingCapacity: {
    remainingCount: "u32",
    remainingSize: "u32"
  },
  /**
   * Lookup206: polkadot_primitives::v6::AbridgedHrmpChannel
   **/
  PolkadotPrimitivesV6AbridgedHrmpChannel: {
    maxCapacity: "u32",
    maxTotalSize: "u32",
    maxMessageSize: "u32",
    msgCount: "u32",
    totalSize: "u32",
    mqcHead: "Option<H256>"
  },
  /**
   * Lookup207: polkadot_primitives::v6::AbridgedHostConfiguration
   **/
  PolkadotPrimitivesV6AbridgedHostConfiguration: {
    maxCodeSize: "u32",
    maxHeadDataSize: "u32",
    maxUpwardQueueCount: "u32",
    maxUpwardQueueSize: "u32",
    maxUpwardMessageSize: "u32",
    maxUpwardMessageNumPerCandidate: "u32",
    hrmpMaxMessageNumPerCandidate: "u32",
    validationUpgradeCooldown: "u32",
    validationUpgradeDelay: "u32",
    asyncBackingParams: "PolkadotPrimitivesV6AsyncBackingAsyncBackingParams"
  },
  /**
   * Lookup208: polkadot_primitives::v6::async_backing::AsyncBackingParams
   **/
  PolkadotPrimitivesV6AsyncBackingAsyncBackingParams: {
    maxCandidateDepth: "u32",
    allowedAncestryLen: "u32"
  },
  /**
   * Lookup214: polkadot_core_primitives::OutboundHrmpMessage<polkadot_parachain_primitives::primitives::Id>
   **/
  PolkadotCorePrimitivesOutboundHrmpMessage: {
    recipient: "u32",
    data: "Bytes"
  },
  /**
   * Lookup216: cumulus_pallet_parachain_system::pallet::Call<T>
   **/
  CumulusPalletParachainSystemCall: {
    _enum: {
      set_validation_data: {
        data: "CumulusPrimitivesParachainInherentParachainInherentData"
      },
      sudo_send_upward_message: {
        message: "Bytes"
      },
      authorize_upgrade: {
        codeHash: "H256",
        checkVersion: "bool"
      },
      enact_authorized_upgrade: {
        code: "Bytes"
      }
    }
  },
  /**
   * Lookup217: cumulus_primitives_parachain_inherent::ParachainInherentData
   **/
  CumulusPrimitivesParachainInherentParachainInherentData: {
    validationData: "PolkadotPrimitivesV6PersistedValidationData",
    relayChainState: "SpTrieStorageProof",
    downwardMessages: "Vec<PolkadotCorePrimitivesInboundDownwardMessage>",
    horizontalMessages: "BTreeMap<u32, Vec<PolkadotCorePrimitivesInboundHrmpMessage>>"
  },
  /**
   * Lookup219: polkadot_core_primitives::InboundDownwardMessage<BlockNumber>
   **/
  PolkadotCorePrimitivesInboundDownwardMessage: {
    sentAt: "u32",
    msg: "Bytes"
  },
  /**
   * Lookup222: polkadot_core_primitives::InboundHrmpMessage<BlockNumber>
   **/
  PolkadotCorePrimitivesInboundHrmpMessage: {
    sentAt: "u32",
    data: "Bytes"
  },
  /**
   * Lookup225: cumulus_pallet_parachain_system::pallet::Error<T>
   **/
  CumulusPalletParachainSystemError: {
    _enum: [
      "OverlappingUpgrades",
      "ProhibitedByPolkadot",
      "TooBig",
      "ValidationDataNotAvailable",
      "HostConfigurationNotAvailable",
      "NotScheduled",
      "NothingAuthorized",
      "Unauthorized"
    ]
  },
  /**
   * Lookup226: pallet_timestamp::pallet::Call<T>
   **/
  PalletTimestampCall: {
    _enum: {
      set: {
        now: "Compact<u64>"
      }
    }
  },
  /**
   * Lookup227: staging_parachain_info::pallet::Call<T>
   **/
  StagingParachainInfoCall: "Null",
  /**
   * Lookup229: pallet_balances::types::BalanceLock<Balance>
   **/
  PalletBalancesBalanceLock: {
    id: "[u8;8]",
    amount: "u128",
    reasons: "PalletBalancesReasons"
  },
  /**
   * Lookup230: pallet_balances::types::Reasons
   **/
  PalletBalancesReasons: {
    _enum: ["Fee", "Misc", "All"]
  },
  /**
   * Lookup233: pallet_balances::types::ReserveData<ReserveIdentifier, Balance>
   **/
  PalletBalancesReserveData: {
    id: "[u8;8]",
    amount: "u128"
  },
  /**
   * Lookup237: storage_hub_runtime::RuntimeHoldReason
   **/
  StorageHubRuntimeRuntimeHoldReason: {
    _enum: {
      __Unused0: "Null",
      __Unused1: "Null",
      __Unused2: "Null",
      __Unused3: "Null",
      __Unused4: "Null",
      __Unused5: "Null",
      __Unused6: "Null",
      __Unused7: "Null",
      __Unused8: "Null",
      __Unused9: "Null",
      __Unused10: "Null",
      __Unused11: "Null",
      __Unused12: "Null",
      __Unused13: "Null",
      __Unused14: "Null",
      __Unused15: "Null",
      __Unused16: "Null",
      __Unused17: "Null",
      __Unused18: "Null",
      __Unused19: "Null",
      __Unused20: "Null",
      __Unused21: "Null",
      __Unused22: "Null",
      __Unused23: "Null",
      __Unused24: "Null",
      __Unused25: "Null",
      __Unused26: "Null",
      __Unused27: "Null",
      __Unused28: "Null",
      __Unused29: "Null",
      __Unused30: "Null",
      __Unused31: "Null",
      __Unused32: "Null",
      __Unused33: "Null",
      __Unused34: "Null",
      __Unused35: "Null",
      __Unused36: "Null",
      __Unused37: "Null",
      __Unused38: "Null",
      __Unused39: "Null",
      Providers: "PalletStorageProvidersHoldReason",
      __Unused41: "Null",
      __Unused42: "Null",
      __Unused43: "Null",
      PaymentStreams: "PalletPaymentStreamsHoldReason"
    }
  },
  /**
   * Lookup238: pallet_storage_providers::pallet::HoldReason
   **/
  PalletStorageProvidersHoldReason: {
    _enum: ["StorageProviderDeposit", "BucketDeposit"]
  },
  /**
   * Lookup239: pallet_payment_streams::pallet::HoldReason
   **/
  PalletPaymentStreamsHoldReason: {
    _enum: ["PaymentStreamDeposit"]
  },
  /**
   * Lookup242: pallet_balances::types::IdAmount<Id, Balance>
   **/
  PalletBalancesIdAmount: {
    id: "Null",
    amount: "u128"
  },
  /**
   * Lookup244: pallet_balances::pallet::Call<T, I>
   **/
  PalletBalancesCall: {
    _enum: {
      transfer_allow_death: {
        dest: "MultiAddress",
        value: "Compact<u128>"
      },
      __Unused1: "Null",
      force_transfer: {
        source: "MultiAddress",
        dest: "MultiAddress",
        value: "Compact<u128>"
      },
      transfer_keep_alive: {
        dest: "MultiAddress",
        value: "Compact<u128>"
      },
      transfer_all: {
        dest: "MultiAddress",
        keepAlive: "bool"
      },
      force_unreserve: {
        who: "MultiAddress",
        amount: "u128"
      },
      upgrade_accounts: {
        who: "Vec<AccountId32>"
      },
      __Unused7: "Null",
      force_set_balance: {
        who: "MultiAddress",
        newFree: "Compact<u128>"
      },
      force_adjust_total_issuance: {
        direction: "PalletBalancesAdjustmentDirection",
        delta: "Compact<u128>"
      }
    }
  },
  /**
   * Lookup247: pallet_balances::types::AdjustmentDirection
   **/
  PalletBalancesAdjustmentDirection: {
    _enum: ["Increase", "Decrease"]
  },
  /**
   * Lookup248: pallet_balances::pallet::Error<T, I>
   **/
  PalletBalancesError: {
    _enum: [
      "VestingBalance",
      "LiquidityRestrictions",
      "InsufficientBalance",
      "ExistentialDeposit",
      "Expendability",
      "ExistingVestingSchedule",
      "DeadAccount",
      "TooManyReserves",
      "TooManyHolds",
      "TooManyFreezes",
      "IssuanceDeactivated",
      "DeltaZero"
    ]
  },
  /**
   * Lookup249: pallet_transaction_payment::Releases
   **/
  PalletTransactionPaymentReleases: {
    _enum: ["V1Ancient", "V2"]
  },
  /**
   * Lookup250: pallet_sudo::pallet::Call<T>
   **/
  PalletSudoCall: {
    _enum: {
      sudo: {
        call: "Call"
      },
      sudo_unchecked_weight: {
        call: "Call",
        weight: "SpWeightsWeightV2Weight"
      },
      set_key: {
        _alias: {
          new_: "new"
        },
        new_: "MultiAddress"
      },
      sudo_as: {
        who: "MultiAddress",
        call: "Call"
      },
      remove_key: "Null"
    }
  },
  /**
   * Lookup252: pallet_collator_selection::pallet::Call<T>
   **/
  PalletCollatorSelectionCall: {
    _enum: {
      set_invulnerables: {
        _alias: {
          new_: "new"
        },
        new_: "Vec<AccountId32>"
      },
      set_desired_candidates: {
        max: "u32"
      },
      set_candidacy_bond: {
        bond: "u128"
      },
      register_as_candidate: "Null",
      leave_intent: "Null",
      add_invulnerable: {
        who: "AccountId32"
      },
      remove_invulnerable: {
        who: "AccountId32"
      },
      update_bond: {
        newDeposit: "u128"
      },
      take_candidate_slot: {
        deposit: "u128",
        target: "AccountId32"
      }
    }
  },
  /**
   * Lookup253: pallet_session::pallet::Call<T>
   **/
  PalletSessionCall: {
    _enum: {
      set_keys: {
        _alias: {
          keys_: "keys"
        },
        keys_: "StorageHubRuntimeSessionKeys",
        proof: "Bytes"
      },
      purge_keys: "Null"
    }
  },
  /**
   * Lookup254: storage_hub_runtime::SessionKeys
   **/
  StorageHubRuntimeSessionKeys: {
    aura: "SpConsensusAuraSr25519AppSr25519Public"
  },
  /**
   * Lookup255: sp_consensus_aura::sr25519::app_sr25519::Public
   **/
  SpConsensusAuraSr25519AppSr25519Public: "SpCoreSr25519Public",
  /**
   * Lookup256: sp_core::sr25519::Public
   **/
  SpCoreSr25519Public: "[u8;32]",
  /**
   * Lookup257: cumulus_pallet_xcmp_queue::pallet::Call<T>
   **/
  CumulusPalletXcmpQueueCall: {
    _enum: {
      __Unused0: "Null",
      suspend_xcm_execution: "Null",
      resume_xcm_execution: "Null",
      update_suspend_threshold: {
        _alias: {
          new_: "new"
        },
        new_: "u32"
      },
      update_drop_threshold: {
        _alias: {
          new_: "new"
        },
        new_: "u32"
      },
      update_resume_threshold: {
        _alias: {
          new_: "new"
        },
        new_: "u32"
      }
    }
  },
  /**
   * Lookup258: pallet_xcm::pallet::Call<T>
   **/
  PalletXcmCall: {
    _enum: {
      send: {
        dest: "XcmVersionedLocation",
        message: "XcmVersionedXcm"
      },
      teleport_assets: {
        dest: "XcmVersionedLocation",
        beneficiary: "XcmVersionedLocation",
        assets: "XcmVersionedAssets",
        feeAssetItem: "u32"
      },
      reserve_transfer_assets: {
        dest: "XcmVersionedLocation",
        beneficiary: "XcmVersionedLocation",
        assets: "XcmVersionedAssets",
        feeAssetItem: "u32"
      },
      execute: {
        message: "XcmVersionedXcm",
        maxWeight: "SpWeightsWeightV2Weight"
      },
      force_xcm_version: {
        location: "StagingXcmV4Location",
        version: "u32"
      },
      force_default_xcm_version: {
        maybeXcmVersion: "Option<u32>"
      },
      force_subscribe_version_notify: {
        location: "XcmVersionedLocation"
      },
      force_unsubscribe_version_notify: {
        location: "XcmVersionedLocation"
      },
      limited_reserve_transfer_assets: {
        dest: "XcmVersionedLocation",
        beneficiary: "XcmVersionedLocation",
        assets: "XcmVersionedAssets",
        feeAssetItem: "u32",
        weightLimit: "XcmV3WeightLimit"
      },
      limited_teleport_assets: {
        dest: "XcmVersionedLocation",
        beneficiary: "XcmVersionedLocation",
        assets: "XcmVersionedAssets",
        feeAssetItem: "u32",
        weightLimit: "XcmV3WeightLimit"
      },
      force_suspension: {
        suspended: "bool"
      },
      transfer_assets: {
        dest: "XcmVersionedLocation",
        beneficiary: "XcmVersionedLocation",
        assets: "XcmVersionedAssets",
        feeAssetItem: "u32",
        weightLimit: "XcmV3WeightLimit"
      },
      claim_assets: {
        assets: "XcmVersionedAssets",
        beneficiary: "XcmVersionedLocation"
      }
    }
  },
  /**
   * Lookup259: xcm::VersionedXcm<RuntimeCall>
   **/
  XcmVersionedXcm: {
    _enum: {
      __Unused0: "Null",
      __Unused1: "Null",
      V2: "XcmV2Xcm",
      V3: "XcmV3Xcm",
      V4: "StagingXcmV4Xcm"
    }
  },
  /**
   * Lookup260: xcm::v2::Xcm<RuntimeCall>
   **/
  XcmV2Xcm: "Vec<XcmV2Instruction>",
  /**
   * Lookup262: xcm::v2::Instruction<RuntimeCall>
   **/
  XcmV2Instruction: {
    _enum: {
      WithdrawAsset: "XcmV2MultiassetMultiAssets",
      ReserveAssetDeposited: "XcmV2MultiassetMultiAssets",
      ReceiveTeleportedAsset: "XcmV2MultiassetMultiAssets",
      QueryResponse: {
        queryId: "Compact<u64>",
        response: "XcmV2Response",
        maxWeight: "Compact<u64>"
      },
      TransferAsset: {
        assets: "XcmV2MultiassetMultiAssets",
        beneficiary: "XcmV2MultiLocation"
      },
      TransferReserveAsset: {
        assets: "XcmV2MultiassetMultiAssets",
        dest: "XcmV2MultiLocation",
        xcm: "XcmV2Xcm"
      },
      Transact: {
        originType: "XcmV2OriginKind",
        requireWeightAtMost: "Compact<u64>",
        call: "XcmDoubleEncoded"
      },
      HrmpNewChannelOpenRequest: {
        sender: "Compact<u32>",
        maxMessageSize: "Compact<u32>",
        maxCapacity: "Compact<u32>"
      },
      HrmpChannelAccepted: {
        recipient: "Compact<u32>"
      },
      HrmpChannelClosing: {
        initiator: "Compact<u32>",
        sender: "Compact<u32>",
        recipient: "Compact<u32>"
      },
      ClearOrigin: "Null",
      DescendOrigin: "XcmV2MultilocationJunctions",
      ReportError: {
        queryId: "Compact<u64>",
        dest: "XcmV2MultiLocation",
        maxResponseWeight: "Compact<u64>"
      },
      DepositAsset: {
        assets: "XcmV2MultiassetMultiAssetFilter",
        maxAssets: "Compact<u32>",
        beneficiary: "XcmV2MultiLocation"
      },
      DepositReserveAsset: {
        assets: "XcmV2MultiassetMultiAssetFilter",
        maxAssets: "Compact<u32>",
        dest: "XcmV2MultiLocation",
        xcm: "XcmV2Xcm"
      },
      ExchangeAsset: {
        give: "XcmV2MultiassetMultiAssetFilter",
        receive: "XcmV2MultiassetMultiAssets"
      },
      InitiateReserveWithdraw: {
        assets: "XcmV2MultiassetMultiAssetFilter",
        reserve: "XcmV2MultiLocation",
        xcm: "XcmV2Xcm"
      },
      InitiateTeleport: {
        assets: "XcmV2MultiassetMultiAssetFilter",
        dest: "XcmV2MultiLocation",
        xcm: "XcmV2Xcm"
      },
      QueryHolding: {
        queryId: "Compact<u64>",
        dest: "XcmV2MultiLocation",
        assets: "XcmV2MultiassetMultiAssetFilter",
        maxResponseWeight: "Compact<u64>"
      },
      BuyExecution: {
        fees: "XcmV2MultiAsset",
        weightLimit: "XcmV2WeightLimit"
      },
      RefundSurplus: "Null",
      SetErrorHandler: "XcmV2Xcm",
      SetAppendix: "XcmV2Xcm",
      ClearError: "Null",
      ClaimAsset: {
        assets: "XcmV2MultiassetMultiAssets",
        ticket: "XcmV2MultiLocation"
      },
      Trap: "Compact<u64>",
      SubscribeVersion: {
        queryId: "Compact<u64>",
        maxResponseWeight: "Compact<u64>"
      },
      UnsubscribeVersion: "Null"
    }
  },
  /**
   * Lookup263: xcm::v2::Response
   **/
  XcmV2Response: {
    _enum: {
      Null: "Null",
      Assets: "XcmV2MultiassetMultiAssets",
      ExecutionResult: "Option<(u32,XcmV2TraitsError)>",
      Version: "u32"
    }
  },
  /**
   * Lookup266: xcm::v2::traits::Error
   **/
  XcmV2TraitsError: {
    _enum: {
      Overflow: "Null",
      Unimplemented: "Null",
      UntrustedReserveLocation: "Null",
      UntrustedTeleportLocation: "Null",
      MultiLocationFull: "Null",
      MultiLocationNotInvertible: "Null",
      BadOrigin: "Null",
      InvalidLocation: "Null",
      AssetNotFound: "Null",
      FailedToTransactAsset: "Null",
      NotWithdrawable: "Null",
      LocationCannotHold: "Null",
      ExceedsMaxMessageSize: "Null",
      DestinationUnsupported: "Null",
      Transport: "Null",
      Unroutable: "Null",
      UnknownClaim: "Null",
      FailedToDecode: "Null",
      MaxWeightInvalid: "Null",
      NotHoldingFees: "Null",
      TooExpensive: "Null",
      Trap: "u64",
      UnhandledXcmVersion: "Null",
      WeightLimitReached: "u64",
      Barrier: "Null",
      WeightNotComputable: "Null"
    }
  },
  /**
   * Lookup267: xcm::v2::multiasset::MultiAssetFilter
   **/
  XcmV2MultiassetMultiAssetFilter: {
    _enum: {
      Definite: "XcmV2MultiassetMultiAssets",
      Wild: "XcmV2MultiassetWildMultiAsset"
    }
  },
  /**
   * Lookup268: xcm::v2::multiasset::WildMultiAsset
   **/
  XcmV2MultiassetWildMultiAsset: {
    _enum: {
      All: "Null",
      AllOf: {
        id: "XcmV2MultiassetAssetId",
        fun: "XcmV2MultiassetWildFungibility"
      }
    }
  },
  /**
   * Lookup269: xcm::v2::multiasset::WildFungibility
   **/
  XcmV2MultiassetWildFungibility: {
    _enum: ["Fungible", "NonFungible"]
  },
  /**
   * Lookup270: xcm::v2::WeightLimit
   **/
  XcmV2WeightLimit: {
    _enum: {
      Unlimited: "Null",
      Limited: "Compact<u64>"
    }
  },
  /**
   * Lookup271: xcm::v3::Xcm<Call>
   **/
  XcmV3Xcm: "Vec<XcmV3Instruction>",
  /**
   * Lookup273: xcm::v3::Instruction<Call>
   **/
  XcmV3Instruction: {
    _enum: {
      WithdrawAsset: "XcmV3MultiassetMultiAssets",
      ReserveAssetDeposited: "XcmV3MultiassetMultiAssets",
      ReceiveTeleportedAsset: "XcmV3MultiassetMultiAssets",
      QueryResponse: {
        queryId: "Compact<u64>",
        response: "XcmV3Response",
        maxWeight: "SpWeightsWeightV2Weight",
        querier: "Option<StagingXcmV3MultiLocation>"
      },
      TransferAsset: {
        assets: "XcmV3MultiassetMultiAssets",
        beneficiary: "StagingXcmV3MultiLocation"
      },
      TransferReserveAsset: {
        assets: "XcmV3MultiassetMultiAssets",
        dest: "StagingXcmV3MultiLocation",
        xcm: "XcmV3Xcm"
      },
      Transact: {
        originKind: "XcmV2OriginKind",
        requireWeightAtMost: "SpWeightsWeightV2Weight",
        call: "XcmDoubleEncoded"
      },
      HrmpNewChannelOpenRequest: {
        sender: "Compact<u32>",
        maxMessageSize: "Compact<u32>",
        maxCapacity: "Compact<u32>"
      },
      HrmpChannelAccepted: {
        recipient: "Compact<u32>"
      },
      HrmpChannelClosing: {
        initiator: "Compact<u32>",
        sender: "Compact<u32>",
        recipient: "Compact<u32>"
      },
      ClearOrigin: "Null",
      DescendOrigin: "XcmV3Junctions",
      ReportError: "XcmV3QueryResponseInfo",
      DepositAsset: {
        assets: "XcmV3MultiassetMultiAssetFilter",
        beneficiary: "StagingXcmV3MultiLocation"
      },
      DepositReserveAsset: {
        assets: "XcmV3MultiassetMultiAssetFilter",
        dest: "StagingXcmV3MultiLocation",
        xcm: "XcmV3Xcm"
      },
      ExchangeAsset: {
        give: "XcmV3MultiassetMultiAssetFilter",
        want: "XcmV3MultiassetMultiAssets",
        maximal: "bool"
      },
      InitiateReserveWithdraw: {
        assets: "XcmV3MultiassetMultiAssetFilter",
        reserve: "StagingXcmV3MultiLocation",
        xcm: "XcmV3Xcm"
      },
      InitiateTeleport: {
        assets: "XcmV3MultiassetMultiAssetFilter",
        dest: "StagingXcmV3MultiLocation",
        xcm: "XcmV3Xcm"
      },
      ReportHolding: {
        responseInfo: "XcmV3QueryResponseInfo",
        assets: "XcmV3MultiassetMultiAssetFilter"
      },
      BuyExecution: {
        fees: "XcmV3MultiAsset",
        weightLimit: "XcmV3WeightLimit"
      },
      RefundSurplus: "Null",
      SetErrorHandler: "XcmV3Xcm",
      SetAppendix: "XcmV3Xcm",
      ClearError: "Null",
      ClaimAsset: {
        assets: "XcmV3MultiassetMultiAssets",
        ticket: "StagingXcmV3MultiLocation"
      },
      Trap: "Compact<u64>",
      SubscribeVersion: {
        queryId: "Compact<u64>",
        maxResponseWeight: "SpWeightsWeightV2Weight"
      },
      UnsubscribeVersion: "Null",
      BurnAsset: "XcmV3MultiassetMultiAssets",
      ExpectAsset: "XcmV3MultiassetMultiAssets",
      ExpectOrigin: "Option<StagingXcmV3MultiLocation>",
      ExpectError: "Option<(u32,XcmV3TraitsError)>",
      ExpectTransactStatus: "XcmV3MaybeErrorCode",
      QueryPallet: {
        moduleName: "Bytes",
        responseInfo: "XcmV3QueryResponseInfo"
      },
      ExpectPallet: {
        index: "Compact<u32>",
        name: "Bytes",
        moduleName: "Bytes",
        crateMajor: "Compact<u32>",
        minCrateMinor: "Compact<u32>"
      },
      ReportTransactStatus: "XcmV3QueryResponseInfo",
      ClearTransactStatus: "Null",
      UniversalOrigin: "XcmV3Junction",
      ExportMessage: {
        network: "XcmV3JunctionNetworkId",
        destination: "XcmV3Junctions",
        xcm: "XcmV3Xcm"
      },
      LockAsset: {
        asset: "XcmV3MultiAsset",
        unlocker: "StagingXcmV3MultiLocation"
      },
      UnlockAsset: {
        asset: "XcmV3MultiAsset",
        target: "StagingXcmV3MultiLocation"
      },
      NoteUnlockable: {
        asset: "XcmV3MultiAsset",
        owner: "StagingXcmV3MultiLocation"
      },
      RequestUnlock: {
        asset: "XcmV3MultiAsset",
        locker: "StagingXcmV3MultiLocation"
      },
      SetFeesMode: {
        jitWithdraw: "bool"
      },
      SetTopic: "[u8;32]",
      ClearTopic: "Null",
      AliasOrigin: "StagingXcmV3MultiLocation",
      UnpaidExecution: {
        weightLimit: "XcmV3WeightLimit",
        checkOrigin: "Option<StagingXcmV3MultiLocation>"
      }
    }
  },
  /**
   * Lookup274: xcm::v3::Response
   **/
  XcmV3Response: {
    _enum: {
      Null: "Null",
      Assets: "XcmV3MultiassetMultiAssets",
      ExecutionResult: "Option<(u32,XcmV3TraitsError)>",
      Version: "u32",
      PalletsInfo: "Vec<XcmV3PalletInfo>",
      DispatchResult: "XcmV3MaybeErrorCode"
    }
  },
  /**
   * Lookup276: xcm::v3::PalletInfo
   **/
  XcmV3PalletInfo: {
    index: "Compact<u32>",
    name: "Bytes",
    moduleName: "Bytes",
    major: "Compact<u32>",
    minor: "Compact<u32>",
    patch: "Compact<u32>"
  },
  /**
   * Lookup280: xcm::v3::QueryResponseInfo
   **/
  XcmV3QueryResponseInfo: {
    destination: "StagingXcmV3MultiLocation",
    queryId: "Compact<u64>",
    maxWeight: "SpWeightsWeightV2Weight"
  },
  /**
   * Lookup281: xcm::v3::multiasset::MultiAssetFilter
   **/
  XcmV3MultiassetMultiAssetFilter: {
    _enum: {
      Definite: "XcmV3MultiassetMultiAssets",
      Wild: "XcmV3MultiassetWildMultiAsset"
    }
  },
  /**
   * Lookup282: xcm::v3::multiasset::WildMultiAsset
   **/
  XcmV3MultiassetWildMultiAsset: {
    _enum: {
      All: "Null",
      AllOf: {
        id: "XcmV3MultiassetAssetId",
        fun: "XcmV3MultiassetWildFungibility"
      },
      AllCounted: "Compact<u32>",
      AllOfCounted: {
        id: "XcmV3MultiassetAssetId",
        fun: "XcmV3MultiassetWildFungibility",
        count: "Compact<u32>"
      }
    }
  },
  /**
   * Lookup283: xcm::v3::multiasset::WildFungibility
   **/
  XcmV3MultiassetWildFungibility: {
    _enum: ["Fungible", "NonFungible"]
  },
  /**
   * Lookup295: cumulus_pallet_xcm::pallet::Call<T>
   **/
  CumulusPalletXcmCall: "Null",
  /**
   * Lookup296: pallet_message_queue::pallet::Call<T>
   **/
  PalletMessageQueueCall: {
    _enum: {
      reap_page: {
        messageOrigin: "CumulusPrimitivesCoreAggregateMessageOrigin",
        pageIndex: "u32"
      },
      execute_overweight: {
        messageOrigin: "CumulusPrimitivesCoreAggregateMessageOrigin",
        page: "u32",
        index: "u32",
        weightLimit: "SpWeightsWeightV2Weight"
      }
    }
  },
  /**
   * Lookup297: pallet_storage_providers::pallet::Call<T>
   **/
  PalletStorageProvidersCall: {
    _enum: {
      request_msp_sign_up: {
        capacity: "u32",
        multiaddresses: "Vec<Bytes>",
        valueProp: "PalletStorageProvidersValueProposition",
        paymentAccount: "AccountId32"
      },
      request_bsp_sign_up: {
        capacity: "u32",
        multiaddresses: "Vec<Bytes>",
        paymentAccount: "AccountId32"
      },
      confirm_sign_up: {
        providerAccount: "Option<AccountId32>"
      },
      cancel_sign_up: "Null",
      msp_sign_off: "Null",
      bsp_sign_off: "Null",
      change_capacity: {
        newCapacity: "u32"
      },
      add_value_prop: {
        newValueProp: "PalletStorageProvidersValueProposition"
      },
      force_msp_sign_up: {
        who: "AccountId32",
        mspId: "H256",
        capacity: "u32",
        multiaddresses: "Vec<Bytes>",
        valueProp: "PalletStorageProvidersValueProposition",
        paymentAccount: "AccountId32"
      },
      force_bsp_sign_up: {
        who: "AccountId32",
        bspId: "H256",
        capacity: "u32",
        multiaddresses: "Vec<Bytes>",
        paymentAccount: "AccountId32"
      },
      slash: {
        providerAccountId: "AccountId32"
      }
    }
  },
  /**
   * Lookup298: pallet_file_system::pallet::Call<T>
   **/
  PalletFileSystemCall: {
    _enum: {
      create_bucket: {
        mspId: "H256",
        name: "Bytes",
        private: "bool"
      },
      update_bucket_privacy: {
        bucketId: "H256",
        private: "bool"
      },
      create_and_associate_collection_with_bucket: {
        bucketId: "H256"
      },
      issue_storage_request: {
        _alias: {
          size_: "size"
        },
        bucketId: "H256",
        location: "Bytes",
        fingerprint: "H256",
        size_: "u32",
        mspId: "H256",
        peerIds: "Vec<Bytes>"
      },
      revoke_storage_request: {
        fileKey: "H256"
      },
      bsp_volunteer: {
        fileKey: "H256"
      },
      bsp_confirm_storing: {
        nonInclusionForestProof: "SpTrieStorageProofCompactProof",
        fileKeysAndProofs: "Vec<(H256,ShpFileKeyVerifierFileKeyProof)>"
      },
      bsp_stop_storing: {
        _alias: {
          size_: "size"
        },
        fileKey: "H256",
        bucketId: "H256",
        location: "Bytes",
        owner: "AccountId32",
        fingerprint: "H256",
        size_: "u32",
        canServe: "bool",
        inclusionForestProof: "SpTrieStorageProofCompactProof"
      },
      delete_file: {
        _alias: {
          size_: "size"
        },
        bucketId: "H256",
        fileKey: "H256",
        location: "Bytes",
        size_: "u32",
        fingerprint: "H256",
        maybeInclusionForestProof: "Option<SpTrieStorageProofCompactProof>"
      },
      pending_file_deletion_request_submit_proof: {
        user: "AccountId32",
        fileKey: "H256",
        bucketId: "H256",
        forestProof: "SpTrieStorageProofCompactProof"
      },
      force_update_bsps_assignment_threshold: {
        bspAssignmentThreshold: "u128"
      }
    }
  },
  /**
   * Lookup303: pallet_proofs_dealer::pallet::Call<T>
   **/
  PalletProofsDealerCall: {
    _enum: {
      challenge: {
        key: "H256"
      },
      submit_proof: {
        proof: "PalletProofsDealerProof",
        provider: "Option<H256>"
      },
      force_initialise_challenge_cycle: {
        provider: "H256"
      }
    }
  },
  /**
   * Lookup304: pallet_randomness::pallet::Call<T>
   **/
  PalletRandomnessCall: {
    _enum: ["set_babe_randomness"]
  },
  /**
   * Lookup305: pallet_payment_streams::pallet::Call<T>
   **/
  PalletPaymentStreamsCall: {
    _enum: {
      create_fixed_rate_payment_stream: {
        providerId: "H256",
        userAccount: "AccountId32",
        rate: "u128"
      },
      update_fixed_rate_payment_stream: {
        providerId: "H256",
        userAccount: "AccountId32",
        newRate: "u128"
      },
      delete_fixed_rate_payment_stream: {
        providerId: "H256",
        userAccount: "AccountId32"
      },
      create_dynamic_rate_payment_stream: {
        providerId: "H256",
        userAccount: "AccountId32",
        amountProvided: "u32",
        currentPrice: "u128",
        currentAccumulatedPriceIndex: "u128"
      },
      update_dynamic_rate_payment_stream: {
        providerId: "H256",
        userAccount: "AccountId32",
        newAmountProvided: "u32",
        currentPrice: "u128"
      },
      delete_dynamic_rate_payment_stream: {
        providerId: "H256",
        userAccount: "AccountId32"
      },
      charge_payment_streams: {
        userAccount: "AccountId32"
      }
    }
  },
  /**
   * Lookup306: pallet_bucket_nfts::pallet::Call<T>
   **/
  PalletBucketNftsCall: {
    _enum: {
      share_access: {
        recipient: "MultiAddress",
        bucket: "H256",
        itemId: "u32",
        readAccessRegex: "Option<Bytes>"
      },
      update_read_access: {
        bucket: "H256",
        itemId: "u32",
        readAccessRegex: "Option<Bytes>"
      }
    }
  },
  /**
   * Lookup308: pallet_nfts::pallet::Call<T, I>
   **/
  PalletNftsCall: {
    _enum: {
      create: {
        admin: "MultiAddress",
        config: "PalletNftsCollectionConfig"
      },
      force_create: {
        owner: "MultiAddress",
        config: "PalletNftsCollectionConfig"
      },
      destroy: {
        collection: "u32",
        witness: "PalletNftsDestroyWitness"
      },
      mint: {
        collection: "u32",
        item: "u32",
        mintTo: "MultiAddress",
        witnessData: "Option<PalletNftsMintWitness>"
      },
      force_mint: {
        collection: "u32",
        item: "u32",
        mintTo: "MultiAddress",
        itemConfig: "PalletNftsItemConfig"
      },
      burn: {
        collection: "u32",
        item: "u32"
      },
      transfer: {
        collection: "u32",
        item: "u32",
        dest: "MultiAddress"
      },
      redeposit: {
        collection: "u32",
        items: "Vec<u32>"
      },
      lock_item_transfer: {
        collection: "u32",
        item: "u32"
      },
      unlock_item_transfer: {
        collection: "u32",
        item: "u32"
      },
      lock_collection: {
        collection: "u32",
        lockSettings: "u64"
      },
      transfer_ownership: {
        collection: "u32",
        newOwner: "MultiAddress"
      },
      set_team: {
        collection: "u32",
        issuer: "Option<MultiAddress>",
        admin: "Option<MultiAddress>",
        freezer: "Option<MultiAddress>"
      },
      force_collection_owner: {
        collection: "u32",
        owner: "MultiAddress"
      },
      force_collection_config: {
        collection: "u32",
        config: "PalletNftsCollectionConfig"
      },
      approve_transfer: {
        collection: "u32",
        item: "u32",
        delegate: "MultiAddress",
        maybeDeadline: "Option<u32>"
      },
      cancel_approval: {
        collection: "u32",
        item: "u32",
        delegate: "MultiAddress"
      },
      clear_all_transfer_approvals: {
        collection: "u32",
        item: "u32"
      },
      lock_item_properties: {
        collection: "u32",
        item: "u32",
        lockMetadata: "bool",
        lockAttributes: "bool"
      },
      set_attribute: {
        collection: "u32",
        maybeItem: "Option<u32>",
        namespace: "PalletNftsAttributeNamespace",
        key: "Bytes",
        value: "Bytes"
      },
      force_set_attribute: {
        setAs: "Option<AccountId32>",
        collection: "u32",
        maybeItem: "Option<u32>",
        namespace: "PalletNftsAttributeNamespace",
        key: "Bytes",
        value: "Bytes"
      },
      clear_attribute: {
        collection: "u32",
        maybeItem: "Option<u32>",
        namespace: "PalletNftsAttributeNamespace",
        key: "Bytes"
      },
      approve_item_attributes: {
        collection: "u32",
        item: "u32",
        delegate: "MultiAddress"
      },
      cancel_item_attributes_approval: {
        collection: "u32",
        item: "u32",
        delegate: "MultiAddress",
        witness: "PalletNftsCancelAttributesApprovalWitness"
      },
      set_metadata: {
        collection: "u32",
        item: "u32",
        data: "Bytes"
      },
      clear_metadata: {
        collection: "u32",
        item: "u32"
      },
      set_collection_metadata: {
        collection: "u32",
        data: "Bytes"
      },
      clear_collection_metadata: {
        collection: "u32"
      },
      set_accept_ownership: {
        maybeCollection: "Option<u32>"
      },
      set_collection_max_supply: {
        collection: "u32",
        maxSupply: "u32"
      },
      update_mint_settings: {
        collection: "u32",
        mintSettings: "PalletNftsMintSettings"
      },
      set_price: {
        collection: "u32",
        item: "u32",
        price: "Option<u128>",
        whitelistedBuyer: "Option<MultiAddress>"
      },
      buy_item: {
        collection: "u32",
        item: "u32",
        bidPrice: "u128"
      },
      pay_tips: {
        tips: "Vec<PalletNftsItemTip>"
      },
      create_swap: {
        offeredCollection: "u32",
        offeredItem: "u32",
        desiredCollection: "u32",
        maybeDesiredItem: "Option<u32>",
        maybePrice: "Option<PalletNftsPriceWithDirection>",
        duration: "u32"
      },
      cancel_swap: {
        offeredCollection: "u32",
        offeredItem: "u32"
      },
      claim_swap: {
        sendCollection: "u32",
        sendItem: "u32",
        receiveCollection: "u32",
        receiveItem: "u32",
        witnessPrice: "Option<PalletNftsPriceWithDirection>"
      },
      mint_pre_signed: {
        mintData: "PalletNftsPreSignedMint",
        signature: "SpRuntimeMultiSignature",
        signer: "AccountId32"
      },
      set_attributes_pre_signed: {
        data: "PalletNftsPreSignedAttributes",
        signature: "SpRuntimeMultiSignature",
        signer: "AccountId32"
      }
    }
  },
  /**
   * Lookup309: pallet_nfts::types::CollectionConfig<Price, BlockNumber, CollectionId>
   **/
  PalletNftsCollectionConfig: {
    settings: "u64",
    maxSupply: "Option<u32>",
    mintSettings: "PalletNftsMintSettings"
  },
  /**
   * Lookup311: pallet_nfts::types::CollectionSetting
   **/
  PalletNftsCollectionSetting: {
    _enum: [
      "__Unused0",
      "TransferableItems",
      "UnlockedMetadata",
      "__Unused3",
      "UnlockedAttributes",
      "__Unused5",
      "__Unused6",
      "__Unused7",
      "UnlockedMaxSupply",
      "__Unused9",
      "__Unused10",
      "__Unused11",
      "__Unused12",
      "__Unused13",
      "__Unused14",
      "__Unused15",
      "DepositRequired"
    ]
  },
  /**
   * Lookup312: pallet_nfts::types::MintSettings<Price, BlockNumber, CollectionId>
   **/
  PalletNftsMintSettings: {
    mintType: "PalletNftsMintType",
    price: "Option<u128>",
    startBlock: "Option<u32>",
    endBlock: "Option<u32>",
    defaultItemSettings: "u64"
  },
  /**
   * Lookup313: pallet_nfts::types::MintType<CollectionId>
   **/
  PalletNftsMintType: {
    _enum: {
      Issuer: "Null",
      Public: "Null",
      HolderOf: "u32"
    }
  },
  /**
   * Lookup316: pallet_nfts::types::ItemSetting
   **/
  PalletNftsItemSetting: {
    _enum: ["__Unused0", "Transferable", "UnlockedMetadata", "__Unused3", "UnlockedAttributes"]
  },
  /**
   * Lookup317: pallet_nfts::types::DestroyWitness
   **/
  PalletNftsDestroyWitness: {
    itemMetadatas: "Compact<u32>",
    itemConfigs: "Compact<u32>",
    attributes: "Compact<u32>"
  },
  /**
   * Lookup319: pallet_nfts::types::MintWitness<ItemId, Balance>
   **/
  PalletNftsMintWitness: {
    ownedItem: "Option<u32>",
    mintPrice: "Option<u128>"
  },
  /**
   * Lookup320: pallet_nfts::types::ItemConfig
   **/
  PalletNftsItemConfig: {
    settings: "u64"
  },
  /**
   * Lookup322: pallet_nfts::types::CancelAttributesApprovalWitness
   **/
  PalletNftsCancelAttributesApprovalWitness: {
    accountAttributes: "u32"
  },
  /**
   * Lookup324: pallet_nfts::types::ItemTip<CollectionId, ItemId, sp_core::crypto::AccountId32, Amount>
   **/
  PalletNftsItemTip: {
    collection: "u32",
    item: "u32",
    receiver: "AccountId32",
    amount: "u128"
  },
  /**
   * Lookup326: pallet_nfts::types::PreSignedMint<CollectionId, ItemId, sp_core::crypto::AccountId32, Deadline, Balance>
   **/
  PalletNftsPreSignedMint: {
    collection: "u32",
    item: "u32",
    attributes: "Vec<(Bytes,Bytes)>",
    metadata: "Bytes",
    onlyAccount: "Option<AccountId32>",
    deadline: "u32",
    mintPrice: "Option<u128>"
  },
  /**
   * Lookup327: sp_runtime::MultiSignature
   **/
  SpRuntimeMultiSignature: {
    _enum: {
      Ed25519: "SpCoreEd25519Signature",
      Sr25519: "SpCoreSr25519Signature",
      Ecdsa: "SpCoreEcdsaSignature"
    }
  },
  /**
   * Lookup328: sp_core::ed25519::Signature
   **/
  SpCoreEd25519Signature: "[u8;64]",
  /**
   * Lookup330: sp_core::sr25519::Signature
   **/
  SpCoreSr25519Signature: "[u8;64]",
  /**
   * Lookup331: sp_core::ecdsa::Signature
   **/
  SpCoreEcdsaSignature: "[u8;65]",
  /**
   * Lookup333: pallet_nfts::types::PreSignedAttributes<CollectionId, ItemId, sp_core::crypto::AccountId32, Deadline>
   **/
  PalletNftsPreSignedAttributes: {
    collection: "u32",
    item: "u32",
    attributes: "Vec<(Bytes,Bytes)>",
    namespace: "PalletNftsAttributeNamespace",
    deadline: "u32"
  },
  /**
   * Lookup334: pallet_sudo::pallet::Error<T>
   **/
  PalletSudoError: {
    _enum: ["RequireSudo"]
  },
  /**
   * Lookup337: pallet_collator_selection::pallet::CandidateInfo<sp_core::crypto::AccountId32, Balance>
   **/
  PalletCollatorSelectionCandidateInfo: {
    who: "AccountId32",
    deposit: "u128"
  },
  /**
   * Lookup339: pallet_collator_selection::pallet::Error<T>
   **/
  PalletCollatorSelectionError: {
    _enum: [
      "TooManyCandidates",
      "TooFewEligibleCollators",
      "AlreadyCandidate",
      "NotCandidate",
      "TooManyInvulnerables",
      "AlreadyInvulnerable",
      "NotInvulnerable",
      "NoAssociatedValidatorId",
      "ValidatorNotRegistered",
      "InsertToCandidateListFailed",
      "RemoveFromCandidateListFailed",
      "DepositTooLow",
      "UpdateCandidateListFailed",
      "InsufficientBond",
      "TargetIsNotCandidate",
      "IdenticalDeposit",
      "InvalidUnreserve"
    ]
  },
  /**
   * Lookup343: sp_core::crypto::KeyTypeId
   **/
  SpCoreCryptoKeyTypeId: "[u8;4]",
  /**
   * Lookup344: pallet_session::pallet::Error<T>
   **/
  PalletSessionError: {
    _enum: ["InvalidProof", "NoAssociatedValidatorId", "DuplicatedKey", "NoKeys", "NoAccount"]
  },
  /**
   * Lookup353: cumulus_pallet_xcmp_queue::OutboundChannelDetails
   **/
  CumulusPalletXcmpQueueOutboundChannelDetails: {
    recipient: "u32",
    state: "CumulusPalletXcmpQueueOutboundState",
    signalsExist: "bool",
    firstIndex: "u16",
    lastIndex: "u16"
  },
  /**
   * Lookup354: cumulus_pallet_xcmp_queue::OutboundState
   **/
  CumulusPalletXcmpQueueOutboundState: {
    _enum: ["Ok", "Suspended"]
  },
  /**
   * Lookup356: cumulus_pallet_xcmp_queue::QueueConfigData
   **/
  CumulusPalletXcmpQueueQueueConfigData: {
    suspendThreshold: "u32",
    dropThreshold: "u32",
    resumeThreshold: "u32"
  },
  /**
   * Lookup357: cumulus_pallet_xcmp_queue::pallet::Error<T>
   **/
  CumulusPalletXcmpQueueError: {
    _enum: ["BadQueueConfig", "AlreadySuspended", "AlreadyResumed"]
  },
  /**
   * Lookup358: pallet_xcm::pallet::QueryStatus<BlockNumber>
   **/
  PalletXcmQueryStatus: {
    _enum: {
      Pending: {
        responder: "XcmVersionedLocation",
        maybeMatchQuerier: "Option<XcmVersionedLocation>",
        maybeNotify: "Option<(u8,u8)>",
        timeout: "u32"
      },
      VersionNotifier: {
        origin: "XcmVersionedLocation",
        isActive: "bool"
      },
      Ready: {
        response: "XcmVersionedResponse",
        at: "u32"
      }
    }
  },
  /**
   * Lookup362: xcm::VersionedResponse
   **/
  XcmVersionedResponse: {
    _enum: {
      __Unused0: "Null",
      __Unused1: "Null",
      V2: "XcmV2Response",
      V3: "XcmV3Response",
      V4: "StagingXcmV4Response"
    }
  },
  /**
   * Lookup368: pallet_xcm::pallet::VersionMigrationStage
   **/
  PalletXcmVersionMigrationStage: {
    _enum: {
      MigrateSupportedVersion: "Null",
      MigrateVersionNotifiers: "Null",
      NotifyCurrentTargets: "Option<Bytes>",
      MigrateAndNotifyOldTargets: "Null"
    }
  },
  /**
   * Lookup371: xcm::VersionedAssetId
   **/
  XcmVersionedAssetId: {
    _enum: {
      __Unused0: "Null",
      __Unused1: "Null",
      __Unused2: "Null",
      V3: "XcmV3MultiassetAssetId",
      V4: "StagingXcmV4AssetAssetId"
    }
  },
  /**
   * Lookup372: pallet_xcm::pallet::RemoteLockedFungibleRecord<ConsumerIdentifier, MaxConsumers>
   **/
  PalletXcmRemoteLockedFungibleRecord: {
    amount: "u128",
    owner: "XcmVersionedLocation",
    locker: "XcmVersionedLocation",
    consumers: "Vec<(Null,u128)>"
  },
  /**
   * Lookup379: pallet_xcm::pallet::Error<T>
   **/
  PalletXcmError: {
    _enum: [
      "Unreachable",
      "SendFailure",
      "Filtered",
      "UnweighableMessage",
      "DestinationNotInvertible",
      "Empty",
      "CannotReanchor",
      "TooManyAssets",
      "InvalidOrigin",
      "BadVersion",
      "BadLocation",
      "NoSubscription",
      "AlreadySubscribed",
      "CannotCheckOutTeleport",
      "LowBalance",
      "TooManyLocks",
      "AccountNotSovereign",
      "FeesNotMet",
      "LockNotFound",
      "InUse",
      "InvalidAssetNotConcrete",
      "InvalidAssetUnknownReserve",
      "InvalidAssetUnsupportedReserve",
      "TooManyReserves",
      "LocalExecutionIncomplete"
    ]
  },
  /**
   * Lookup380: pallet_message_queue::BookState<cumulus_primitives_core::AggregateMessageOrigin>
   **/
  PalletMessageQueueBookState: {
    _alias: {
      size_: "size"
    },
    begin: "u32",
    end: "u32",
    count: "u32",
    readyNeighbours: "Option<PalletMessageQueueNeighbours>",
    messageCount: "u64",
    size_: "u64"
  },
  /**
   * Lookup382: pallet_message_queue::Neighbours<cumulus_primitives_core::AggregateMessageOrigin>
   **/
  PalletMessageQueueNeighbours: {
    prev: "CumulusPrimitivesCoreAggregateMessageOrigin",
    next: "CumulusPrimitivesCoreAggregateMessageOrigin"
  },
  /**
   * Lookup384: pallet_message_queue::Page<Size, HeapSize>
   **/
  PalletMessageQueuePage: {
    remaining: "u32",
    remainingSize: "u32",
    firstIndex: "u32",
    first: "u32",
    last: "u32",
    heap: "Bytes"
  },
  /**
   * Lookup386: pallet_message_queue::pallet::Error<T>
   **/
  PalletMessageQueueError: {
    _enum: [
      "NotReapable",
      "NoPage",
      "NoMessage",
      "AlreadyProcessed",
      "Queued",
      "InsufficientWeight",
      "TemporarilyUnprocessable",
      "QueuePaused",
      "RecursiveDisallowed"
    ]
  },
  /**
   * Lookup388: pallet_storage_providers::types::StorageProvider<T>
   **/
  PalletStorageProvidersStorageProvider: {
    _enum: {
      BackupStorageProvider: "PalletStorageProvidersBackupStorageProvider",
      MainStorageProvider: "PalletStorageProvidersMainStorageProvider"
    }
  },
  /**
   * Lookup389: pallet_storage_providers::types::BackupStorageProvider<T>
   **/
  PalletStorageProvidersBackupStorageProvider: {
    capacity: "u32",
    dataUsed: "u32",
    multiaddresses: "Vec<Bytes>",
    root: "H256",
    lastCapacityChange: "u32",
    ownerAccount: "AccountId32",
    paymentAccount: "AccountId32"
  },
  /**
   * Lookup390: pallet_storage_providers::types::MainStorageProvider<T>
   **/
  PalletStorageProvidersMainStorageProvider: {
    buckets: "Vec<PalletStorageProvidersBucket>",
    capacity: "u32",
    dataUsed: "u32",
    multiaddresses: "Vec<Bytes>",
    valueProp: "PalletStorageProvidersValueProposition",
    lastCapacityChange: "u32",
    ownerAccount: "AccountId32",
    paymentAccount: "AccountId32"
  },
  /**
   * Lookup392: pallet_storage_providers::types::Bucket<T>
   **/
  PalletStorageProvidersBucket: {
    root: "H256",
    userId: "AccountId32",
    mspId: "H256",
    private: "bool",
    readAccessGroupId: "Option<u32>"
  },
  /**
   * Lookup395: pallet_storage_providers::pallet::Error<T>
   **/
  PalletStorageProvidersError: {
    _enum: [
      "AlreadyRegistered",
      "SignUpNotRequested",
      "SignUpRequestPending",
      "NoMultiAddress",
      "InvalidMultiAddress",
      "StorageTooLow",
      "NotEnoughBalance",
      "CannotHoldDeposit",
      "StorageStillInUse",
      "RandomnessNotValidYet",
      "SignUpRequestExpired",
      "NewCapacityLessThanUsedStorage",
      "NewCapacityEqualsCurrentCapacity",
      "NewCapacityCantBeZero",
      "NotEnoughTimePassed",
      "NotRegistered",
      "NoUserId",
      "NoBucketId",
      "SpRegisteredButDataNotFound",
      "BucketNotFound",
      "BucketAlreadyExists",
      "AppendBucketToMspFailed",
      "ProviderNotSlashable"
    ]
  },
  /**
   * Lookup396: pallet_file_system::types::StorageRequestMetadata<T>
   **/
  PalletFileSystemStorageRequestMetadata: {
    _alias: {
      size_: "size"
    },
    requestedAt: "u32",
    owner: "AccountId32",
    bucketId: "H256",
    location: "Bytes",
    fingerprint: "H256",
    size_: "u32",
    msp: "Option<H256>",
    userPeerIds: "Vec<Bytes>",
    dataServerSps: "Vec<H256>",
    bspsRequired: "u32",
    bspsConfirmed: "u32",
    bspsVolunteered: "u32"
  },
  /**
   * Lookup399: pallet_file_system::types::StorageRequestBspsMetadata<T>
   **/
  PalletFileSystemStorageRequestBspsMetadata: {
    confirmed: "bool"
  },
  /**
   * Lookup401: pallet_file_system::types::ExpiredItems<T>
   **/
  PalletFileSystemExpiredItems: {
    _enum: {
      StorageRequest: "H256",
      PendingFileDeletionRequests: "(AccountId32,H256)"
    }
  },
  /**
   * Lookup406: pallet_file_system::pallet::Error<T>
   **/
  PalletFileSystemError: {
    _enum: [
      "StorageRequestAlreadyRegistered",
      "StorageRequestNotFound",
      "BspsRequiredCannotBeZero",
      "BspsRequiredExceedsMax",
      "NotABsp",
      "NotAMsp",
      "BspNotVolunteered",
      "BspNotConfirmed",
      "BspAlreadyConfirmed",
      "StorageRequestBspsRequiredFulfilled",
      "BspAlreadyVolunteered",
      "UnexpectedNumberOfRemovedVolunteeredBsps",
      "StorageRequestExpiredNoSlotAvailable",
      "StorageRequestNotAuthorized",
      "MaxBlockNumberReached",
      "FailedToEncodeBsp",
      "FailedToEncodeFingerprint",
      "FailedToDecodeThreshold",
      "AboveThreshold",
      "FailedToConvertBlockNumber",
      "ThresholdArithmeticError",
      "FailedTypeConversion",
      "DividedByZero",
      "ImpossibleFailedToGetValue",
      "BucketIsNotPrivate",
      "BucketNotFound",
      "NotBucketOwner",
      "ProviderRootNotFound",
      "ExpectedNonInclusionProof",
      "ExpectedInclusionProof",
      "InvalidFileKeyMetadata",
      "ThresholdBelowAsymptote",
      "NotFileOwner",
      "FileKeyAlreadyPendingDeletion",
      "MaxUserPendingDeletionRequestsReached",
      "MspNotStoringBucket",
      "FileKeyNotPendingDeletion",
      "FileSizeCannotBeZero"
    ]
  },
  /**
   * Lookup412: pallet_proofs_dealer::pallet::Error<T>
   **/
  PalletProofsDealerError: {
    _enum: [
      "NotProvider",
      "ChallengesQueueOverflow",
      "PriorityChallengesQueueOverflow",
      "FeeChargeFailed",
      "EmptyKeyProofs",
      "ProviderRootNotFound",
      "ZeroRoot",
      "NoRecordOfLastSubmittedProof",
      "ProviderStakeNotFound",
      "ZeroStake",
      "StakeCouldNotBeConverted",
      "ChallengesTickNotReached",
      "ChallengesTickTooOld",
      "ChallengesTickTooLate",
      "SeedNotFound",
      "CheckpointChallengesNotFound",
      "ForestProofVerificationFailed",
      "KeyProofNotFound",
      "KeyProofVerificationFailed",
      "FailedToApplyDelta",
      "TooManyValidProofSubmitters"
    ]
  },
  /**
   * Lookup415: pallet_payment_streams::types::FixedRatePaymentStream<T>
   **/
  PalletPaymentStreamsFixedRatePaymentStream: {
    rate: "u128",
    lastChargedBlock: "u32",
    userDeposit: "u128"
  },
  /**
   * Lookup416: pallet_payment_streams::types::DynamicRatePaymentStream<T>
   **/
  PalletPaymentStreamsDynamicRatePaymentStream: {
    amountProvided: "u32",
    priceIndexWhenLastCharged: "u128",
    userDeposit: "u128"
  },
  /**
   * Lookup417: pallet_payment_streams::types::ProviderLastChargeable<T>
   **/
  PalletPaymentStreamsProviderLastChargeable: {
    lastChargeableBlock: "u32",
    priceIndex: "u128"
  },
  /**
   * Lookup418: pallet_payment_streams::pallet::Error<T>
   **/
  PalletPaymentStreamsError: {
    _enum: [
      "PaymentStreamAlreadyExists",
      "PaymentStreamNotFound",
      "NotAProvider",
      "ProviderInconsistencyError",
      "CannotHoldDeposit",
      "UpdateRateToSameRate",
      "UpdateAmountToSameAmount",
      "RateCantBeZero",
      "AmountProvidedCantBeZero",
      "LastChargedGreaterThanLastChargeable",
      "InvalidLastChargeableBlockNumber",
      "InvalidLastChargeablePriceIndex",
      "ChargeOverflow",
      "UserWithoutFunds"
    ]
  },
  /**
   * Lookup419: pallet_bucket_nfts::pallet::Error<T>
   **/
  PalletBucketNftsError: {
    _enum: [
      "BucketIsNotPrivate",
      "NotBucketOwner",
      "NoCorrespondingCollection",
      "ConvertBytesToBoundedVec"
    ]
  },
  /**
   * Lookup420: pallet_nfts::types::CollectionDetails<sp_core::crypto::AccountId32, DepositBalance>
   **/
  PalletNftsCollectionDetails: {
    owner: "AccountId32",
    ownerDeposit: "u128",
    items: "u32",
    itemMetadatas: "u32",
    itemConfigs: "u32",
    attributes: "u32"
  },
  /**
   * Lookup425: pallet_nfts::types::CollectionRole
   **/
  PalletNftsCollectionRole: {
    _enum: ["__Unused0", "Issuer", "Freezer", "__Unused3", "Admin"]
  },
  /**
   * Lookup426: pallet_nfts::types::ItemDetails<sp_core::crypto::AccountId32, pallet_nfts::types::ItemDeposit<DepositBalance, sp_core::crypto::AccountId32>, bounded_collections::bounded_btree_map::BoundedBTreeMap<sp_core::crypto::AccountId32, Option<T>, S>>
   **/
  PalletNftsItemDetails: {
    owner: "AccountId32",
    approvals: "BTreeMap<AccountId32, Option<u32>>",
    deposit: "PalletNftsItemDeposit"
  },
  /**
   * Lookup427: pallet_nfts::types::ItemDeposit<DepositBalance, sp_core::crypto::AccountId32>
   **/
  PalletNftsItemDeposit: {
    account: "AccountId32",
    amount: "u128"
  },
  /**
   * Lookup432: pallet_nfts::types::CollectionMetadata<Deposit, StringLimit>
   **/
  PalletNftsCollectionMetadata: {
    deposit: "u128",
    data: "Bytes"
  },
  /**
   * Lookup433: pallet_nfts::types::ItemMetadata<pallet_nfts::types::ItemMetadataDeposit<DepositBalance, sp_core::crypto::AccountId32>, StringLimit>
   **/
  PalletNftsItemMetadata: {
    deposit: "PalletNftsItemMetadataDeposit",
    data: "Bytes"
  },
  /**
   * Lookup434: pallet_nfts::types::ItemMetadataDeposit<DepositBalance, sp_core::crypto::AccountId32>
   **/
  PalletNftsItemMetadataDeposit: {
    account: "Option<AccountId32>",
    amount: "u128"
  },
  /**
   * Lookup437: pallet_nfts::types::AttributeDeposit<DepositBalance, sp_core::crypto::AccountId32>
   **/
  PalletNftsAttributeDeposit: {
    account: "Option<AccountId32>",
    amount: "u128"
  },
  /**
   * Lookup441: pallet_nfts::types::PendingSwap<CollectionId, ItemId, pallet_nfts::types::PriceWithDirection<Amount>, Deadline>
   **/
  PalletNftsPendingSwap: {
    desiredCollection: "u32",
    desiredItem: "Option<u32>",
    price: "Option<PalletNftsPriceWithDirection>",
    deadline: "u32"
  },
  /**
   * Lookup443: pallet_nfts::types::PalletFeature
   **/
  PalletNftsPalletFeature: {
    _enum: [
      "__Unused0",
      "Trading",
      "Attributes",
      "__Unused3",
      "Approvals",
      "__Unused5",
      "__Unused6",
      "__Unused7",
      "Swaps"
    ]
  },
  /**
   * Lookup444: pallet_nfts::pallet::Error<T, I>
   **/
  PalletNftsError: {
    _enum: [
      "NoPermission",
      "UnknownCollection",
      "AlreadyExists",
      "ApprovalExpired",
      "WrongOwner",
      "BadWitness",
      "CollectionIdInUse",
      "ItemsNonTransferable",
      "NotDelegate",
      "WrongDelegate",
      "Unapproved",
      "Unaccepted",
      "ItemLocked",
      "LockedItemAttributes",
      "LockedCollectionAttributes",
      "LockedItemMetadata",
      "LockedCollectionMetadata",
      "MaxSupplyReached",
      "MaxSupplyLocked",
      "MaxSupplyTooSmall",
      "UnknownItem",
      "UnknownSwap",
      "MetadataNotFound",
      "AttributeNotFound",
      "NotForSale",
      "BidTooLow",
      "ReachedApprovalLimit",
      "DeadlineExpired",
      "WrongDuration",
      "MethodDisabled",
      "WrongSetting",
      "InconsistentItemConfig",
      "NoConfig",
      "RolesNotCleared",
      "MintNotStarted",
      "MintEnded",
      "AlreadyClaimed",
      "IncorrectData",
      "WrongOrigin",
      "WrongSignature",
      "IncorrectMetadata",
      "MaxAttributesLimitReached",
      "WrongNamespace",
      "CollectionNotEmpty",
      "WitnessRequired"
    ]
  },
  /**
   * Lookup447: frame_system::extensions::check_non_zero_sender::CheckNonZeroSender<T>
   **/
  FrameSystemExtensionsCheckNonZeroSender: "Null",
  /**
   * Lookup448: frame_system::extensions::check_spec_version::CheckSpecVersion<T>
   **/
  FrameSystemExtensionsCheckSpecVersion: "Null",
  /**
   * Lookup449: frame_system::extensions::check_tx_version::CheckTxVersion<T>
   **/
  FrameSystemExtensionsCheckTxVersion: "Null",
  /**
   * Lookup450: frame_system::extensions::check_genesis::CheckGenesis<T>
   **/
  FrameSystemExtensionsCheckGenesis: "Null",
  /**
   * Lookup453: frame_system::extensions::check_nonce::CheckNonce<T>
   **/
  FrameSystemExtensionsCheckNonce: "Compact<u32>",
  /**
   * Lookup454: frame_system::extensions::check_weight::CheckWeight<T>
   **/
  FrameSystemExtensionsCheckWeight: "Null",
  /**
   * Lookup455: pallet_transaction_payment::ChargeTransactionPayment<T>
   **/
  PalletTransactionPaymentChargeTransactionPayment: "Compact<u128>",
  /**
   * Lookup456: cumulus_primitives_storage_weight_reclaim::StorageWeightReclaim<T>
   **/
  CumulusPrimitivesStorageWeightReclaimStorageWeightReclaim: "Null",
  /**
   * Lookup457: storage_hub_runtime::Runtime
   **/
  StorageHubRuntimeRuntime: "Null"
};
