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
   * Lookup15: sp_runtime::generic::digest::Digest
   **/
  SpRuntimeDigest: {
    logs: string;
  };
  /**
   * Lookup17: sp_runtime::generic::digest::DigestItem
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
   * Lookup20: frame_system::EventRecord<storage_hub_runtime::RuntimeEvent, primitive_types::H256>
   **/
  FrameSystemEventRecord: {
    phase: string;
    event: string;
    topics: string;
  };
  /**
   * Lookup22: frame_system::pallet::Event<T>
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
   * Lookup23: frame_support::dispatch::DispatchInfo
   **/
  FrameSupportDispatchDispatchInfo: {
    weight: string;
    class: string;
    paysFee: string;
  };
  /**
   * Lookup24: frame_support::dispatch::DispatchClass
   **/
  FrameSupportDispatchDispatchClass: {
    _enum: string[];
  };
  /**
   * Lookup25: frame_support::dispatch::Pays
   **/
  FrameSupportDispatchPays: {
    _enum: string[];
  };
  /**
   * Lookup26: sp_runtime::DispatchError
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
    };
  };
  /**
   * Lookup27: sp_runtime::ModuleError
   **/
  SpRuntimeModuleError: {
    index: string;
    error: string;
  };
  /**
   * Lookup28: sp_runtime::TokenError
   **/
  SpRuntimeTokenError: {
    _enum: string[];
  };
  /**
   * Lookup29: sp_arithmetic::ArithmeticError
   **/
  SpArithmeticArithmeticError: {
    _enum: string[];
  };
  /**
   * Lookup30: sp_runtime::TransactionalError
   **/
  SpRuntimeTransactionalError: {
    _enum: string[];
  };
  /**
   * Lookup31: cumulus_pallet_parachain_system::pallet::Event<T>
   **/
  CumulusPalletParachainSystemEvent: {
    _enum: {
      ValidationFunctionStored: string;
      ValidationFunctionApplied: {
        relayChainBlockNum: string;
      };
      ValidationFunctionDiscarded: string;
      DownwardMessagesReceived: {
        count: string;
      };
      DownwardMessagesProcessed: {
        weightUsed: string;
        dmqHead: string;
      };
      UpwardMessageSent: {
        messageHash: string;
      };
    };
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
   * Lookup35: pallet_transaction_payment::pallet::Event<T>
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
   * Lookup36: pallet_sudo::pallet::Event<T>
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
   * Lookup40: pallet_collator_selection::pallet::Event<T>
   **/
  PalletCollatorSelectionEvent: {
    _enum: {
      NewInvulnerables: {
        invulnerables: string;
      };
      InvulnerableAdded: {
        accountId: string;
      };
      InvulnerableRemoved: {
        accountId: string;
      };
      NewDesiredCandidates: {
        desiredCandidates: string;
      };
      NewCandidacyBond: {
        bondAmount: string;
      };
      CandidateAdded: {
        accountId: string;
        deposit: string;
      };
      CandidateBondUpdated: {
        accountId: string;
        deposit: string;
      };
      CandidateRemoved: {
        accountId: string;
      };
      CandidateReplaced: {
        _alias: {
          new_: string;
        };
        old: string;
        new_: string;
        deposit: string;
      };
      InvalidInvulnerableSkipped: {
        accountId: string;
      };
    };
  };
  /**
   * Lookup42: pallet_session::pallet::Event
   **/
  PalletSessionEvent: {
    _enum: {
      NewSession: {
        sessionIndex: string;
      };
    };
  };
  /**
   * Lookup43: cumulus_pallet_xcmp_queue::pallet::Event<T>
   **/
  CumulusPalletXcmpQueueEvent: {
    _enum: {
      XcmpMessageSent: {
        messageHash: string;
      };
    };
  };
  /**
   * Lookup44: pallet_xcm::pallet::Event<T>
   **/
  PalletXcmEvent: {
    _enum: {
      Attempted: {
        outcome: string;
      };
      Sent: {
        origin: string;
        destination: string;
        message: string;
        messageId: string;
      };
      UnexpectedResponse: {
        origin: string;
        queryId: string;
      };
      ResponseReady: {
        queryId: string;
        response: string;
      };
      Notified: {
        queryId: string;
        palletIndex: string;
        callIndex: string;
      };
      NotifyOverweight: {
        queryId: string;
        palletIndex: string;
        callIndex: string;
        actualWeight: string;
        maxBudgetedWeight: string;
      };
      NotifyDispatchError: {
        queryId: string;
        palletIndex: string;
        callIndex: string;
      };
      NotifyDecodeFailed: {
        queryId: string;
        palletIndex: string;
        callIndex: string;
      };
      InvalidResponder: {
        origin: string;
        queryId: string;
        expectedLocation: string;
      };
      InvalidResponderVersion: {
        origin: string;
        queryId: string;
      };
      ResponseTaken: {
        queryId: string;
      };
      AssetsTrapped: {
        _alias: {
          hash_: string;
        };
        hash_: string;
        origin: string;
        assets: string;
      };
      VersionChangeNotified: {
        destination: string;
        result: string;
        cost: string;
        messageId: string;
      };
      SupportedVersionChanged: {
        location: string;
        version: string;
      };
      NotifyTargetSendFail: {
        location: string;
        queryId: string;
        error: string;
      };
      NotifyTargetMigrationFail: {
        location: string;
        queryId: string;
      };
      InvalidQuerierVersion: {
        origin: string;
        queryId: string;
      };
      InvalidQuerier: {
        origin: string;
        queryId: string;
        expectedQuerier: string;
        maybeActualQuerier: string;
      };
      VersionNotifyStarted: {
        destination: string;
        cost: string;
        messageId: string;
      };
      VersionNotifyRequested: {
        destination: string;
        cost: string;
        messageId: string;
      };
      VersionNotifyUnrequested: {
        destination: string;
        cost: string;
        messageId: string;
      };
      FeesPaid: {
        paying: string;
        fees: string;
      };
      AssetsClaimed: {
        _alias: {
          hash_: string;
        };
        hash_: string;
        origin: string;
        assets: string;
      };
      VersionMigrationFinished: {
        version: string;
      };
    };
  };
  /**
   * Lookup45: staging_xcm::v4::traits::Outcome
   **/
  StagingXcmV4TraitsOutcome: {
    _enum: {
      Complete: {
        used: string;
      };
      Incomplete: {
        used: string;
        error: string;
      };
      Error: {
        error: string;
      };
    };
  };
  /**
   * Lookup46: xcm::v3::traits::Error
   **/
  XcmV3TraitsError: {
    _enum: {
      Overflow: string;
      Unimplemented: string;
      UntrustedReserveLocation: string;
      UntrustedTeleportLocation: string;
      LocationFull: string;
      LocationNotInvertible: string;
      BadOrigin: string;
      InvalidLocation: string;
      AssetNotFound: string;
      FailedToTransactAsset: string;
      NotWithdrawable: string;
      LocationCannotHold: string;
      ExceedsMaxMessageSize: string;
      DestinationUnsupported: string;
      Transport: string;
      Unroutable: string;
      UnknownClaim: string;
      FailedToDecode: string;
      MaxWeightInvalid: string;
      NotHoldingFees: string;
      TooExpensive: string;
      Trap: string;
      ExpectationFalse: string;
      PalletNotFound: string;
      NameMismatch: string;
      VersionIncompatible: string;
      HoldingWouldOverflow: string;
      ExportError: string;
      ReanchorFailed: string;
      NoDeal: string;
      FeesNotMet: string;
      LockError: string;
      NoPermission: string;
      Unanchored: string;
      NotDepositable: string;
      UnhandledXcmVersion: string;
      WeightLimitReached: string;
      Barrier: string;
      WeightNotComputable: string;
      ExceedsStackLimit: string;
    };
  };
  /**
   * Lookup47: staging_xcm::v4::location::Location
   **/
  StagingXcmV4Location: {
    parents: string;
    interior: string;
  };
  /**
   * Lookup48: staging_xcm::v4::junctions::Junctions
   **/
  StagingXcmV4Junctions: {
    _enum: {
      Here: string;
      X1: string;
      X2: string;
      X3: string;
      X4: string;
      X5: string;
      X6: string;
      X7: string;
      X8: string;
    };
  };
  /**
   * Lookup50: staging_xcm::v4::junction::Junction
   **/
  StagingXcmV4Junction: {
    _enum: {
      Parachain: string;
      AccountId32: {
        network: string;
        id: string;
      };
      AccountIndex64: {
        network: string;
        index: string;
      };
      AccountKey20: {
        network: string;
        key: string;
      };
      PalletInstance: string;
      GeneralIndex: string;
      GeneralKey: {
        length: string;
        data: string;
      };
      OnlyChild: string;
      Plurality: {
        id: string;
        part: string;
      };
      GlobalConsensus: string;
    };
  };
  /**
   * Lookup53: staging_xcm::v4::junction::NetworkId
   **/
  StagingXcmV4JunctionNetworkId: {
    _enum: {
      ByGenesis: string;
      ByFork: {
        blockNumber: string;
        blockHash: string;
      };
      Polkadot: string;
      Kusama: string;
      Westend: string;
      Rococo: string;
      Wococo: string;
      Ethereum: {
        chainId: string;
      };
      BitcoinCore: string;
      BitcoinCash: string;
      PolkadotBulletin: string;
    };
  };
  /**
   * Lookup56: xcm::v3::junction::BodyId
   **/
  XcmV3JunctionBodyId: {
    _enum: {
      Unit: string;
      Moniker: string;
      Index: string;
      Executive: string;
      Technical: string;
      Legislative: string;
      Judicial: string;
      Defense: string;
      Administration: string;
      Treasury: string;
    };
  };
  /**
   * Lookup57: xcm::v3::junction::BodyPart
   **/
  XcmV3JunctionBodyPart: {
    _enum: {
      Voice: string;
      Members: {
        count: string;
      };
      Fraction: {
        nom: string;
        denom: string;
      };
      AtLeastProportion: {
        nom: string;
        denom: string;
      };
      MoreThanProportion: {
        nom: string;
        denom: string;
      };
    };
  };
  /**
   * Lookup65: staging_xcm::v4::Xcm<Call>
   **/
  StagingXcmV4Xcm: string;
  /**
   * Lookup67: staging_xcm::v4::Instruction<Call>
   **/
  StagingXcmV4Instruction: {
    _enum: {
      WithdrawAsset: string;
      ReserveAssetDeposited: string;
      ReceiveTeleportedAsset: string;
      QueryResponse: {
        queryId: string;
        response: string;
        maxWeight: string;
        querier: string;
      };
      TransferAsset: {
        assets: string;
        beneficiary: string;
      };
      TransferReserveAsset: {
        assets: string;
        dest: string;
        xcm: string;
      };
      Transact: {
        originKind: string;
        requireWeightAtMost: string;
        call: string;
      };
      HrmpNewChannelOpenRequest: {
        sender: string;
        maxMessageSize: string;
        maxCapacity: string;
      };
      HrmpChannelAccepted: {
        recipient: string;
      };
      HrmpChannelClosing: {
        initiator: string;
        sender: string;
        recipient: string;
      };
      ClearOrigin: string;
      DescendOrigin: string;
      ReportError: string;
      DepositAsset: {
        assets: string;
        beneficiary: string;
      };
      DepositReserveAsset: {
        assets: string;
        dest: string;
        xcm: string;
      };
      ExchangeAsset: {
        give: string;
        want: string;
        maximal: string;
      };
      InitiateReserveWithdraw: {
        assets: string;
        reserve: string;
        xcm: string;
      };
      InitiateTeleport: {
        assets: string;
        dest: string;
        xcm: string;
      };
      ReportHolding: {
        responseInfo: string;
        assets: string;
      };
      BuyExecution: {
        fees: string;
        weightLimit: string;
      };
      RefundSurplus: string;
      SetErrorHandler: string;
      SetAppendix: string;
      ClearError: string;
      ClaimAsset: {
        assets: string;
        ticket: string;
      };
      Trap: string;
      SubscribeVersion: {
        queryId: string;
        maxResponseWeight: string;
      };
      UnsubscribeVersion: string;
      BurnAsset: string;
      ExpectAsset: string;
      ExpectOrigin: string;
      ExpectError: string;
      ExpectTransactStatus: string;
      QueryPallet: {
        moduleName: string;
        responseInfo: string;
      };
      ExpectPallet: {
        index: string;
        name: string;
        moduleName: string;
        crateMajor: string;
        minCrateMinor: string;
      };
      ReportTransactStatus: string;
      ClearTransactStatus: string;
      UniversalOrigin: string;
      ExportMessage: {
        network: string;
        destination: string;
        xcm: string;
      };
      LockAsset: {
        asset: string;
        unlocker: string;
      };
      UnlockAsset: {
        asset: string;
        target: string;
      };
      NoteUnlockable: {
        asset: string;
        owner: string;
      };
      RequestUnlock: {
        asset: string;
        locker: string;
      };
      SetFeesMode: {
        jitWithdraw: string;
      };
      SetTopic: string;
      ClearTopic: string;
      AliasOrigin: string;
      UnpaidExecution: {
        weightLimit: string;
        checkOrigin: string;
      };
    };
  };
  /**
   * Lookup68: staging_xcm::v4::asset::Assets
   **/
  StagingXcmV4AssetAssets: string;
  /**
   * Lookup70: staging_xcm::v4::asset::Asset
   **/
  StagingXcmV4Asset: {
    id: string;
    fun: string;
  };
  /**
   * Lookup71: staging_xcm::v4::asset::AssetId
   **/
  StagingXcmV4AssetAssetId: string;
  /**
   * Lookup72: staging_xcm::v4::asset::Fungibility
   **/
  StagingXcmV4AssetFungibility: {
    _enum: {
      Fungible: string;
      NonFungible: string;
    };
  };
  /**
   * Lookup73: staging_xcm::v4::asset::AssetInstance
   **/
  StagingXcmV4AssetAssetInstance: {
    _enum: {
      Undefined: string;
      Index: string;
      Array4: string;
      Array8: string;
      Array16: string;
      Array32: string;
    };
  };
  /**
   * Lookup76: staging_xcm::v4::Response
   **/
  StagingXcmV4Response: {
    _enum: {
      Null: string;
      Assets: string;
      ExecutionResult: string;
      Version: string;
      PalletsInfo: string;
      DispatchResult: string;
    };
  };
  /**
   * Lookup80: staging_xcm::v4::PalletInfo
   **/
  StagingXcmV4PalletInfo: {
    index: string;
    name: string;
    moduleName: string;
    major: string;
    minor: string;
    patch: string;
  };
  /**
   * Lookup83: xcm::v3::MaybeErrorCode
   **/
  XcmV3MaybeErrorCode: {
    _enum: {
      Success: string;
      Error: string;
      TruncatedError: string;
    };
  };
  /**
   * Lookup86: xcm::v3::OriginKind
   **/
  XcmV3OriginKind: {
    _enum: string[];
  };
  /**
   * Lookup87: xcm::double_encoded::DoubleEncoded<T>
   **/
  XcmDoubleEncoded: {
    encoded: string;
  };
  /**
   * Lookup88: staging_xcm::v4::QueryResponseInfo
   **/
  StagingXcmV4QueryResponseInfo: {
    destination: string;
    queryId: string;
    maxWeight: string;
  };
  /**
   * Lookup89: staging_xcm::v4::asset::AssetFilter
   **/
  StagingXcmV4AssetAssetFilter: {
    _enum: {
      Definite: string;
      Wild: string;
    };
  };
  /**
   * Lookup90: staging_xcm::v4::asset::WildAsset
   **/
  StagingXcmV4AssetWildAsset: {
    _enum: {
      All: string;
      AllOf: {
        id: string;
        fun: string;
      };
      AllCounted: string;
      AllOfCounted: {
        id: string;
        fun: string;
        count: string;
      };
    };
  };
  /**
   * Lookup91: staging_xcm::v4::asset::WildFungibility
   **/
  StagingXcmV4AssetWildFungibility: {
    _enum: string[];
  };
  /**
   * Lookup92: xcm::v3::WeightLimit
   **/
  XcmV3WeightLimit: {
    _enum: {
      Unlimited: string;
      Limited: string;
    };
  };
  /**
   * Lookup93: xcm::VersionedAssets
   **/
  XcmVersionedAssets: {
    _enum: {
      __Unused0: string;
      V2: string;
      __Unused2: string;
      V3: string;
      V4: string;
    };
  };
  /**
   * Lookup94: xcm::v2::multiasset::MultiAssets
   **/
  XcmV2MultiassetMultiAssets: string;
  /**
   * Lookup96: xcm::v2::multiasset::MultiAsset
   **/
  XcmV2MultiAsset: {
    id: string;
    fun: string;
  };
  /**
   * Lookup97: xcm::v2::multiasset::AssetId
   **/
  XcmV2MultiassetAssetId: {
    _enum: {
      Concrete: string;
      Abstract: string;
    };
  };
  /**
   * Lookup98: xcm::v2::multilocation::MultiLocation
   **/
  XcmV2MultiLocation: {
    parents: string;
    interior: string;
  };
  /**
   * Lookup99: xcm::v2::multilocation::Junctions
   **/
  XcmV2MultilocationJunctions: {
    _enum: {
      Here: string;
      X1: string;
      X2: string;
      X3: string;
      X4: string;
      X5: string;
      X6: string;
      X7: string;
      X8: string;
    };
  };
  /**
   * Lookup100: xcm::v2::junction::Junction
   **/
  XcmV2Junction: {
    _enum: {
      Parachain: string;
      AccountId32: {
        network: string;
        id: string;
      };
      AccountIndex64: {
        network: string;
        index: string;
      };
      AccountKey20: {
        network: string;
        key: string;
      };
      PalletInstance: string;
      GeneralIndex: string;
      GeneralKey: string;
      OnlyChild: string;
      Plurality: {
        id: string;
        part: string;
      };
    };
  };
  /**
   * Lookup101: xcm::v2::NetworkId
   **/
  XcmV2NetworkId: {
    _enum: {
      Any: string;
      Named: string;
      Polkadot: string;
      Kusama: string;
    };
  };
  /**
   * Lookup103: xcm::v2::BodyId
   **/
  XcmV2BodyId: {
    _enum: {
      Unit: string;
      Named: string;
      Index: string;
      Executive: string;
      Technical: string;
      Legislative: string;
      Judicial: string;
      Defense: string;
      Administration: string;
      Treasury: string;
    };
  };
  /**
   * Lookup104: xcm::v2::BodyPart
   **/
  XcmV2BodyPart: {
    _enum: {
      Voice: string;
      Members: {
        count: string;
      };
      Fraction: {
        nom: string;
        denom: string;
      };
      AtLeastProportion: {
        nom: string;
        denom: string;
      };
      MoreThanProportion: {
        nom: string;
        denom: string;
      };
    };
  };
  /**
   * Lookup105: xcm::v2::multiasset::Fungibility
   **/
  XcmV2MultiassetFungibility: {
    _enum: {
      Fungible: string;
      NonFungible: string;
    };
  };
  /**
   * Lookup106: xcm::v2::multiasset::AssetInstance
   **/
  XcmV2MultiassetAssetInstance: {
    _enum: {
      Undefined: string;
      Index: string;
      Array4: string;
      Array8: string;
      Array16: string;
      Array32: string;
      Blob: string;
    };
  };
  /**
   * Lookup107: xcm::v3::multiasset::MultiAssets
   **/
  XcmV3MultiassetMultiAssets: string;
  /**
   * Lookup109: xcm::v3::multiasset::MultiAsset
   **/
  XcmV3MultiAsset: {
    id: string;
    fun: string;
  };
  /**
   * Lookup110: xcm::v3::multiasset::AssetId
   **/
  XcmV3MultiassetAssetId: {
    _enum: {
      Concrete: string;
      Abstract: string;
    };
  };
  /**
   * Lookup111: staging_xcm::v3::multilocation::MultiLocation
   **/
  StagingXcmV3MultiLocation: {
    parents: string;
    interior: string;
  };
  /**
   * Lookup112: xcm::v3::junctions::Junctions
   **/
  XcmV3Junctions: {
    _enum: {
      Here: string;
      X1: string;
      X2: string;
      X3: string;
      X4: string;
      X5: string;
      X6: string;
      X7: string;
      X8: string;
    };
  };
  /**
   * Lookup113: xcm::v3::junction::Junction
   **/
  XcmV3Junction: {
    _enum: {
      Parachain: string;
      AccountId32: {
        network: string;
        id: string;
      };
      AccountIndex64: {
        network: string;
        index: string;
      };
      AccountKey20: {
        network: string;
        key: string;
      };
      PalletInstance: string;
      GeneralIndex: string;
      GeneralKey: {
        length: string;
        data: string;
      };
      OnlyChild: string;
      Plurality: {
        id: string;
        part: string;
      };
      GlobalConsensus: string;
    };
  };
  /**
   * Lookup115: xcm::v3::junction::NetworkId
   **/
  XcmV3JunctionNetworkId: {
    _enum: {
      ByGenesis: string;
      ByFork: {
        blockNumber: string;
        blockHash: string;
      };
      Polkadot: string;
      Kusama: string;
      Westend: string;
      Rococo: string;
      Wococo: string;
      Ethereum: {
        chainId: string;
      };
      BitcoinCore: string;
      BitcoinCash: string;
      PolkadotBulletin: string;
    };
  };
  /**
   * Lookup116: xcm::v3::multiasset::Fungibility
   **/
  XcmV3MultiassetFungibility: {
    _enum: {
      Fungible: string;
      NonFungible: string;
    };
  };
  /**
   * Lookup117: xcm::v3::multiasset::AssetInstance
   **/
  XcmV3MultiassetAssetInstance: {
    _enum: {
      Undefined: string;
      Index: string;
      Array4: string;
      Array8: string;
      Array16: string;
      Array32: string;
    };
  };
  /**
   * Lookup118: xcm::VersionedLocation
   **/
  XcmVersionedLocation: {
    _enum: {
      __Unused0: string;
      V2: string;
      __Unused2: string;
      V3: string;
      V4: string;
    };
  };
  /**
   * Lookup119: cumulus_pallet_xcm::pallet::Event<T>
   **/
  CumulusPalletXcmEvent: {
    _enum: {
      InvalidFormat: string;
      UnsupportedVersion: string;
      ExecutedDownward: string;
    };
  };
  /**
   * Lookup120: pallet_message_queue::pallet::Event<T>
   **/
  PalletMessageQueueEvent: {
    _enum: {
      ProcessingFailed: {
        id: string;
        origin: string;
        error: string;
      };
      Processed: {
        id: string;
        origin: string;
        weightUsed: string;
        success: string;
      };
      OverweightEnqueued: {
        id: string;
        origin: string;
        pageIndex: string;
        messageIndex: string;
      };
      PageReaped: {
        origin: string;
        index: string;
      };
    };
  };
  /**
   * Lookup121: cumulus_primitives_core::AggregateMessageOrigin
   **/
  CumulusPrimitivesCoreAggregateMessageOrigin: {
    _enum: {
      Here: string;
      Parent: string;
      Sibling: string;
    };
  };
  /**
   * Lookup123: frame_support::traits::messages::ProcessMessageError
   **/
  FrameSupportMessagesProcessMessageError: {
    _enum: {
      BadFormat: string;
      Corrupt: string;
      Unsupported: string;
      Overweight: string;
      Yield: string;
      StackLimitReached: string;
    };
  };
  /**
   * Lookup124: pallet_storage_providers::pallet::Event<T>
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
   * Lookup128: pallet_storage_providers::types::ValuePropositionWithId<T>
   **/
  PalletStorageProvidersValuePropositionWithId: {
    id: string;
    valueProp: string;
  };
  /**
   * Lookup129: pallet_storage_providers::types::ValueProposition<T>
   **/
  PalletStorageProvidersValueProposition: {
    pricePerGigaUnitOfDataPerBlock: string;
    commitment: string;
    bucketDataLimit: string;
    available: string;
  };
  /**
   * Lookup131: pallet_storage_providers::types::StorageProviderId<T>
   **/
  PalletStorageProvidersStorageProviderId: {
    _enum: {
      BackupStorageProvider: string;
      MainStorageProvider: string;
    };
  };
  /**
   * Lookup132: pallet_storage_providers::types::TopUpMetadata<T>
   **/
  PalletStorageProvidersTopUpMetadata: {
    startedAt: string;
    endTickGracePeriod: string;
  };
  /**
   * Lookup134: pallet_file_system::pallet::Event<T>
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
      MoveBucketRequested: {
        who: string;
        bucketId: string;
        newMspId: string;
        newValuePropId: string;
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
        reason: string;
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
      PriorityChallengeForFileDeletionQueued: {
        issuer: string;
        fileKey: string;
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
      FailedToQueuePriorityChallenge: {
        fileKey: string;
        error: string;
      };
      FileDeletionRequest: {
        user: string;
        fileKey: string;
        fileSize: string;
        bucketId: string;
        mspId: string;
        proofOfInclusion: string;
      };
      ProofSubmittedForPendingFileDeletionRequest: {
        user: string;
        fileKey: string;
        fileSize: string;
        bucketId: string;
        mspId: string;
        proofOfInclusion: string;
      };
      BspChallengeCycleInitialised: {
        who: string;
        bspId: string;
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
      MspStoppedStoringBucket: {
        mspId: string;
        owner: string;
        bucketId: string;
      };
      FailedToGetMspOfBucket: {
        bucketId: string;
        error: string;
      };
      FailedToDecreaseMspUsedCapacity: {
        user: string;
        mspId: string;
        fileKey: string;
        fileSize: string;
        error: string;
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
      FailedToTransferDepositFundsToBsp: {
        fileKey: string;
        owner: string;
        bspId: string;
        amountToTransfer: string;
        error: string;
      };
    };
  };
  /**
   * Lookup138: pallet_file_system::types::RejectedStorageRequestReason
   **/
  PalletFileSystemRejectedStorageRequestReason: {
    _enum: string[];
  };
  /**
   * Lookup139: pallet_file_system::types::EitherAccountIdOrMspId<T>
   **/
  PalletFileSystemEitherAccountIdOrMspId: {
    _enum: {
      AccountId: string;
      MspId: string;
    };
  };
  /**
   * Lookup141: pallet_proofs_dealer::pallet::Event<T>
   **/
  PalletProofsDealerEvent: {
    _enum: {
      NewChallenge: {
        who: string;
        keyChallenged: string;
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
   * Lookup142: pallet_proofs_dealer::types::Proof<T>
   **/
  PalletProofsDealerProof: {
    forestProof: string;
    keyProofs: string;
  };
  /**
   * Lookup143: sp_trie::storage_proof::CompactProof
   **/
  SpTrieStorageProofCompactProof: {
    encodedNodes: string;
  };
  /**
   * Lookup146: pallet_proofs_dealer::types::KeyProof<T>
   **/
  PalletProofsDealerKeyProof: {
    proof: string;
    challengeCount: string;
  };
  /**
   * Lookup147: shp_file_key_verifier::types::FileKeyProof
   **/
  ShpFileKeyVerifierFileKeyProof: {
    fileMetadata: string;
    proof: string;
  };
  /**
   * Lookup148: shp_file_metadata::FileMetadata
   **/
  ShpFileMetadataFileMetadata: {
    owner: string;
    bucketId: string;
    location: string;
    fileSize: string;
    fingerprint: string;
  };
  /**
   * Lookup149: shp_file_metadata::Fingerprint
   **/
  ShpFileMetadataFingerprint: string;
  /**
   * Lookup153: pallet_proofs_dealer::types::CustomChallenge<T>
   **/
  PalletProofsDealerCustomChallenge: {
    key: string;
    shouldRemoveKey: string;
  };
  /**
   * Lookup157: shp_traits::TrieMutation
   **/
  ShpTraitsTrieMutation: {
    _enum: {
      Add: string;
      Remove: string;
    };
  };
  /**
   * Lookup158: shp_traits::TrieAddMutation
   **/
  ShpTraitsTrieAddMutation: {
    value: string;
  };
  /**
   * Lookup159: shp_traits::TrieRemoveMutation
   **/
  ShpTraitsTrieRemoveMutation: {
    maybeValue: string;
  };
  /**
   * Lookup161: pallet_randomness::pallet::Event<T>
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
   * Lookup162: pallet_payment_streams::pallet::Event<T>
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
   * Lookup164: pallet_bucket_nfts::pallet::Event<T>
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
   * Lookup165: pallet_nfts::pallet::Event<T, I>
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
   * Lookup169: pallet_nfts::types::AttributeNamespace<sp_core::crypto::AccountId32>
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
   * Lookup171: pallet_nfts::types::PriceWithDirection<Amount>
   **/
  PalletNftsPriceWithDirection: {
    amount: string;
    direction: string;
  };
  /**
   * Lookup172: pallet_nfts::types::PriceDirection
   **/
  PalletNftsPriceDirection: {
    _enum: string[];
  };
  /**
   * Lookup173: pallet_nfts::types::PalletAttributes<CollectionId>
   **/
  PalletNftsPalletAttributes: {
    _enum: {
      UsedToClaim: string;
      TransferDisabled: string;
    };
  };
  /**
   * Lookup174: pallet_parameters::pallet::Event<T>
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
   * Lookup175: storage_hub_runtime::configs::runtime_params::RuntimeParametersKey
   **/
  StorageHubRuntimeConfigsRuntimeParamsRuntimeParametersKey: {
    _enum: {
      RuntimeConfig: string;
    };
  };
  /**
   * Lookup176: storage_hub_runtime::configs::runtime_params::dynamic_params::runtime_config::ParametersKey
   **/
  StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigParametersKey: {
    _enum: string[];
  };
  /**
   * Lookup177: storage_hub_runtime::configs::runtime_params::dynamic_params::runtime_config::SlashAmountPerMaxFileSize
   **/
  StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigSlashAmountPerMaxFileSize: string;
  /**
   * Lookup178: storage_hub_runtime::configs::runtime_params::dynamic_params::runtime_config::StakeToChallengePeriod
   **/
  StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigStakeToChallengePeriod: string;
  /**
   * Lookup179: storage_hub_runtime::configs::runtime_params::dynamic_params::runtime_config::CheckpointChallengePeriod
   **/
  StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigCheckpointChallengePeriod: string;
  /**
   * Lookup180: storage_hub_runtime::configs::runtime_params::dynamic_params::runtime_config::MinChallengePeriod
   **/
  StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMinChallengePeriod: string;
  /**
   * Lookup181: storage_hub_runtime::configs::runtime_params::dynamic_params::runtime_config::SystemUtilisationLowerThresholdPercentage
   **/
  StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigSystemUtilisationLowerThresholdPercentage: string;
  /**
   * Lookup182: storage_hub_runtime::configs::runtime_params::dynamic_params::runtime_config::SystemUtilisationUpperThresholdPercentage
   **/
  StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigSystemUtilisationUpperThresholdPercentage: string;
  /**
   * Lookup183: storage_hub_runtime::configs::runtime_params::dynamic_params::runtime_config::MostlyStablePrice
   **/
  StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMostlyStablePrice: string;
  /**
   * Lookup184: storage_hub_runtime::configs::runtime_params::dynamic_params::runtime_config::MaxPrice
   **/
  StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMaxPrice: string;
  /**
   * Lookup185: storage_hub_runtime::configs::runtime_params::dynamic_params::runtime_config::MinPrice
   **/
  StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMinPrice: string;
  /**
   * Lookup186: storage_hub_runtime::configs::runtime_params::dynamic_params::runtime_config::UpperExponentFactor
   **/
  StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigUpperExponentFactor: string;
  /**
   * Lookup187: storage_hub_runtime::configs::runtime_params::dynamic_params::runtime_config::LowerExponentFactor
   **/
  StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigLowerExponentFactor: string;
  /**
   * Lookup188: storage_hub_runtime::configs::runtime_params::dynamic_params::runtime_config::ZeroSizeBucketFixedRate
   **/
  StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigZeroSizeBucketFixedRate: string;
  /**
   * Lookup189: storage_hub_runtime::configs::runtime_params::dynamic_params::runtime_config::IdealUtilisationRate
   **/
  StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigIdealUtilisationRate: string;
  /**
   * Lookup190: storage_hub_runtime::configs::runtime_params::dynamic_params::runtime_config::DecayRate
   **/
  StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigDecayRate: string;
  /**
   * Lookup191: storage_hub_runtime::configs::runtime_params::dynamic_params::runtime_config::MinimumTreasuryCut
   **/
  StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMinimumTreasuryCut: string;
  /**
   * Lookup192: storage_hub_runtime::configs::runtime_params::dynamic_params::runtime_config::MaximumTreasuryCut
   **/
  StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMaximumTreasuryCut: string;
  /**
   * Lookup193: storage_hub_runtime::configs::runtime_params::dynamic_params::runtime_config::BspStopStoringFilePenalty
   **/
  StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigBspStopStoringFilePenalty: string;
  /**
   * Lookup194: storage_hub_runtime::configs::runtime_params::dynamic_params::runtime_config::ProviderTopUpTtl
   **/
  StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigProviderTopUpTtl: string;
  /**
   * Lookup195: storage_hub_runtime::configs::runtime_params::dynamic_params::runtime_config::BasicReplicationTarget
   **/
  StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigBasicReplicationTarget: string;
  /**
   * Lookup196: storage_hub_runtime::configs::runtime_params::dynamic_params::runtime_config::StandardReplicationTarget
   **/
  StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigStandardReplicationTarget: string;
  /**
   * Lookup197: storage_hub_runtime::configs::runtime_params::dynamic_params::runtime_config::HighSecurityReplicationTarget
   **/
  StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigHighSecurityReplicationTarget: string;
  /**
   * Lookup198: storage_hub_runtime::configs::runtime_params::dynamic_params::runtime_config::SuperHighSecurityReplicationTarget
   **/
  StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigSuperHighSecurityReplicationTarget: string;
  /**
   * Lookup199: storage_hub_runtime::configs::runtime_params::dynamic_params::runtime_config::UltraHighSecurityReplicationTarget
   **/
  StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigUltraHighSecurityReplicationTarget: string;
  /**
   * Lookup200: storage_hub_runtime::configs::runtime_params::dynamic_params::runtime_config::MaxReplicationTarget
   **/
  StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMaxReplicationTarget: string;
  /**
   * Lookup201: storage_hub_runtime::configs::runtime_params::dynamic_params::runtime_config::TickRangeToMaximumThreshold
   **/
  StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigTickRangeToMaximumThreshold: string;
  /**
   * Lookup202: storage_hub_runtime::configs::runtime_params::dynamic_params::runtime_config::StorageRequestTtl
   **/
  StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigStorageRequestTtl: string;
  /**
   * Lookup203: storage_hub_runtime::configs::runtime_params::dynamic_params::runtime_config::MinWaitForStopStoring
   **/
  StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMinWaitForStopStoring: string;
  /**
   * Lookup204: storage_hub_runtime::configs::runtime_params::dynamic_params::runtime_config::MinSeedPeriod
   **/
  StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigMinSeedPeriod: string;
  /**
   * Lookup205: storage_hub_runtime::configs::runtime_params::dynamic_params::runtime_config::StakeToSeedPeriod
   **/
  StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigStakeToSeedPeriod: string;
  /**
   * Lookup206: storage_hub_runtime::configs::runtime_params::dynamic_params::runtime_config::UpfrontTicksToPay
   **/
  StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigUpfrontTicksToPay: string;
  /**
   * Lookup208: storage_hub_runtime::configs::runtime_params::RuntimeParametersValue
   **/
  StorageHubRuntimeConfigsRuntimeParamsRuntimeParametersValue: {
    _enum: {
      RuntimeConfig: string;
    };
  };
  /**
   * Lookup209: storage_hub_runtime::configs::runtime_params::dynamic_params::runtime_config::ParametersValue
   **/
  StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigParametersValue: {
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
   * Lookup211: frame_system::Phase
   **/
  FrameSystemPhase: {
    _enum: {
      ApplyExtrinsic: string;
      Finalization: string;
      Initialization: string;
    };
  };
  /**
   * Lookup214: frame_system::LastRuntimeUpgradeInfo
   **/
  FrameSystemLastRuntimeUpgradeInfo: {
    specVersion: string;
    specName: string;
  };
  /**
   * Lookup216: frame_system::CodeUpgradeAuthorization<T>
   **/
  FrameSystemCodeUpgradeAuthorization: {
    codeHash: string;
    checkVersion: string;
  };
  /**
   * Lookup217: frame_system::pallet::Call<T>
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
   * Lookup220: frame_system::limits::BlockWeights
   **/
  FrameSystemLimitsBlockWeights: {
    baseBlock: string;
    maxBlock: string;
    perClass: string;
  };
  /**
   * Lookup221: frame_support::dispatch::PerDispatchClass<frame_system::limits::WeightsPerClass>
   **/
  FrameSupportDispatchPerDispatchClassWeightsPerClass: {
    normal: string;
    operational: string;
    mandatory: string;
  };
  /**
   * Lookup222: frame_system::limits::WeightsPerClass
   **/
  FrameSystemLimitsWeightsPerClass: {
    baseExtrinsic: string;
    maxExtrinsic: string;
    maxTotal: string;
    reserved: string;
  };
  /**
   * Lookup224: frame_system::limits::BlockLength
   **/
  FrameSystemLimitsBlockLength: {
    max: string;
  };
  /**
   * Lookup225: frame_support::dispatch::PerDispatchClass<T>
   **/
  FrameSupportDispatchPerDispatchClassU32: {
    normal: string;
    operational: string;
    mandatory: string;
  };
  /**
   * Lookup226: sp_weights::RuntimeDbWeight
   **/
  SpWeightsRuntimeDbWeight: {
    read: string;
    write: string;
  };
  /**
   * Lookup227: sp_version::RuntimeVersion
   **/
  SpVersionRuntimeVersion: {
    specName: string;
    implName: string;
    authoringVersion: string;
    specVersion: string;
    implVersion: string;
    apis: string;
    transactionVersion: string;
    stateVersion: string;
  };
  /**
   * Lookup232: frame_system::pallet::Error<T>
   **/
  FrameSystemError: {
    _enum: string[];
  };
  /**
   * Lookup234: cumulus_pallet_parachain_system::unincluded_segment::Ancestor<primitive_types::H256>
   **/
  CumulusPalletParachainSystemUnincludedSegmentAncestor: {
    usedBandwidth: string;
    paraHeadHash: string;
    consumedGoAheadSignal: string;
  };
  /**
   * Lookup235: cumulus_pallet_parachain_system::unincluded_segment::UsedBandwidth
   **/
  CumulusPalletParachainSystemUnincludedSegmentUsedBandwidth: {
    umpMsgCount: string;
    umpTotalBytes: string;
    hrmpOutgoing: string;
  };
  /**
   * Lookup237: cumulus_pallet_parachain_system::unincluded_segment::HrmpChannelUpdate
   **/
  CumulusPalletParachainSystemUnincludedSegmentHrmpChannelUpdate: {
    msgCount: string;
    totalBytes: string;
  };
  /**
   * Lookup241: polkadot_primitives::v8::UpgradeGoAhead
   **/
  PolkadotPrimitivesV8UpgradeGoAhead: {
    _enum: string[];
  };
  /**
   * Lookup242: cumulus_pallet_parachain_system::unincluded_segment::SegmentTracker<primitive_types::H256>
   **/
  CumulusPalletParachainSystemUnincludedSegmentSegmentTracker: {
    usedBandwidth: string;
    hrmpWatermark: string;
    consumedGoAheadSignal: string;
  };
  /**
   * Lookup243: polkadot_primitives::v8::PersistedValidationData<primitive_types::H256, N>
   **/
  PolkadotPrimitivesV8PersistedValidationData: {
    parentHead: string;
    relayParentNumber: string;
    relayParentStorageRoot: string;
    maxPovSize: string;
  };
  /**
   * Lookup246: polkadot_primitives::v8::UpgradeRestriction
   **/
  PolkadotPrimitivesV8UpgradeRestriction: {
    _enum: string[];
  };
  /**
   * Lookup247: sp_trie::storage_proof::StorageProof
   **/
  SpTrieStorageProof: {
    trieNodes: string;
  };
  /**
   * Lookup249: cumulus_pallet_parachain_system::relay_state_snapshot::MessagingStateSnapshot
   **/
  CumulusPalletParachainSystemRelayStateSnapshotMessagingStateSnapshot: {
    dmqMqcHead: string;
    relayDispatchQueueRemainingCapacity: string;
    ingressChannels: string;
    egressChannels: string;
  };
  /**
   * Lookup250: cumulus_pallet_parachain_system::relay_state_snapshot::RelayDispatchQueueRemainingCapacity
   **/
  CumulusPalletParachainSystemRelayStateSnapshotRelayDispatchQueueRemainingCapacity: {
    remainingCount: string;
    remainingSize: string;
  };
  /**
   * Lookup253: polkadot_primitives::v8::AbridgedHrmpChannel
   **/
  PolkadotPrimitivesV8AbridgedHrmpChannel: {
    maxCapacity: string;
    maxTotalSize: string;
    maxMessageSize: string;
    msgCount: string;
    totalSize: string;
    mqcHead: string;
  };
  /**
   * Lookup254: polkadot_primitives::v8::AbridgedHostConfiguration
   **/
  PolkadotPrimitivesV8AbridgedHostConfiguration: {
    maxCodeSize: string;
    maxHeadDataSize: string;
    maxUpwardQueueCount: string;
    maxUpwardQueueSize: string;
    maxUpwardMessageSize: string;
    maxUpwardMessageNumPerCandidate: string;
    hrmpMaxMessageNumPerCandidate: string;
    validationUpgradeCooldown: string;
    validationUpgradeDelay: string;
    asyncBackingParams: string;
  };
  /**
   * Lookup255: polkadot_primitives::v8::async_backing::AsyncBackingParams
   **/
  PolkadotPrimitivesV8AsyncBackingAsyncBackingParams: {
    maxCandidateDepth: string;
    allowedAncestryLen: string;
  };
  /**
   * Lookup261: polkadot_core_primitives::OutboundHrmpMessage<polkadot_parachain_primitives::primitives::Id>
   **/
  PolkadotCorePrimitivesOutboundHrmpMessage: {
    recipient: string;
    data: string;
  };
  /**
   * Lookup263: cumulus_pallet_parachain_system::pallet::Call<T>
   **/
  CumulusPalletParachainSystemCall: {
    _enum: {
      set_validation_data: {
        data: string;
      };
      sudo_send_upward_message: {
        message: string;
      };
    };
  };
  /**
   * Lookup264: cumulus_primitives_parachain_inherent::ParachainInherentData
   **/
  CumulusPrimitivesParachainInherentParachainInherentData: {
    validationData: string;
    relayChainState: string;
    downwardMessages: string;
    horizontalMessages: string;
  };
  /**
   * Lookup266: polkadot_core_primitives::InboundDownwardMessage<BlockNumber>
   **/
  PolkadotCorePrimitivesInboundDownwardMessage: {
    sentAt: string;
    msg: string;
  };
  /**
   * Lookup269: polkadot_core_primitives::InboundHrmpMessage<BlockNumber>
   **/
  PolkadotCorePrimitivesInboundHrmpMessage: {
    sentAt: string;
    data: string;
  };
  /**
   * Lookup272: cumulus_pallet_parachain_system::pallet::Error<T>
   **/
  CumulusPalletParachainSystemError: {
    _enum: string[];
  };
  /**
   * Lookup273: pallet_timestamp::pallet::Call<T>
   **/
  PalletTimestampCall: {
    _enum: {
      set: {
        now: string;
      };
    };
  };
  /**
   * Lookup274: staging_parachain_info::pallet::Call<T>
   **/
  StagingParachainInfoCall: string;
  /**
   * Lookup276: pallet_balances::types::BalanceLock<Balance>
   **/
  PalletBalancesBalanceLock: {
    id: string;
    amount: string;
    reasons: string;
  };
  /**
   * Lookup277: pallet_balances::types::Reasons
   **/
  PalletBalancesReasons: {
    _enum: string[];
  };
  /**
   * Lookup280: pallet_balances::types::ReserveData<ReserveIdentifier, Balance>
   **/
  PalletBalancesReserveData: {
    id: string;
    amount: string;
  };
  /**
   * Lookup284: storage_hub_runtime::RuntimeHoldReason
   **/
  StorageHubRuntimeRuntimeHoldReason: {
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
      Providers: string;
      FileSystem: string;
      __Unused42: string;
      __Unused43: string;
      PaymentStreams: string;
    };
  };
  /**
   * Lookup285: pallet_storage_providers::pallet::HoldReason
   **/
  PalletStorageProvidersHoldReason: {
    _enum: string[];
  };
  /**
   * Lookup286: pallet_file_system::pallet::HoldReason
   **/
  PalletFileSystemHoldReason: {
    _enum: string[];
  };
  /**
   * Lookup287: pallet_payment_streams::pallet::HoldReason
   **/
  PalletPaymentStreamsHoldReason: {
    _enum: string[];
  };
  /**
   * Lookup290: frame_support::traits::tokens::misc::IdAmount<Id, Balance>
   **/
  FrameSupportTokensMiscIdAmount: {
    id: string;
    amount: string;
  };
  /**
   * Lookup292: pallet_balances::pallet::Call<T, I>
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
   * Lookup295: pallet_balances::types::AdjustmentDirection
   **/
  PalletBalancesAdjustmentDirection: {
    _enum: string[];
  };
  /**
   * Lookup296: pallet_balances::pallet::Error<T, I>
   **/
  PalletBalancesError: {
    _enum: string[];
  };
  /**
   * Lookup297: pallet_transaction_payment::Releases
   **/
  PalletTransactionPaymentReleases: {
    _enum: string[];
  };
  /**
   * Lookup298: pallet_sudo::pallet::Call<T>
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
   * Lookup300: pallet_collator_selection::pallet::Call<T>
   **/
  PalletCollatorSelectionCall: {
    _enum: {
      set_invulnerables: {
        _alias: {
          new_: string;
        };
        new_: string;
      };
      set_desired_candidates: {
        max: string;
      };
      set_candidacy_bond: {
        bond: string;
      };
      register_as_candidate: string;
      leave_intent: string;
      add_invulnerable: {
        who: string;
      };
      remove_invulnerable: {
        who: string;
      };
      update_bond: {
        newDeposit: string;
      };
      take_candidate_slot: {
        deposit: string;
        target: string;
      };
    };
  };
  /**
   * Lookup301: pallet_session::pallet::Call<T>
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
   * Lookup302: storage_hub_runtime::SessionKeys
   **/
  StorageHubRuntimeSessionKeys: {
    aura: string;
  };
  /**
   * Lookup303: sp_consensus_aura::sr25519::app_sr25519::Public
   **/
  SpConsensusAuraSr25519AppSr25519Public: string;
  /**
   * Lookup304: cumulus_pallet_xcmp_queue::pallet::Call<T>
   **/
  CumulusPalletXcmpQueueCall: {
    _enum: {
      __Unused0: string;
      suspend_xcm_execution: string;
      resume_xcm_execution: string;
      update_suspend_threshold: {
        _alias: {
          new_: string;
        };
        new_: string;
      };
      update_drop_threshold: {
        _alias: {
          new_: string;
        };
        new_: string;
      };
      update_resume_threshold: {
        _alias: {
          new_: string;
        };
        new_: string;
      };
    };
  };
  /**
   * Lookup305: pallet_xcm::pallet::Call<T>
   **/
  PalletXcmCall: {
    _enum: {
      send: {
        dest: string;
        message: string;
      };
      teleport_assets: {
        dest: string;
        beneficiary: string;
        assets: string;
        feeAssetItem: string;
      };
      reserve_transfer_assets: {
        dest: string;
        beneficiary: string;
        assets: string;
        feeAssetItem: string;
      };
      execute: {
        message: string;
        maxWeight: string;
      };
      force_xcm_version: {
        location: string;
        version: string;
      };
      force_default_xcm_version: {
        maybeXcmVersion: string;
      };
      force_subscribe_version_notify: {
        location: string;
      };
      force_unsubscribe_version_notify: {
        location: string;
      };
      limited_reserve_transfer_assets: {
        dest: string;
        beneficiary: string;
        assets: string;
        feeAssetItem: string;
        weightLimit: string;
      };
      limited_teleport_assets: {
        dest: string;
        beneficiary: string;
        assets: string;
        feeAssetItem: string;
        weightLimit: string;
      };
      force_suspension: {
        suspended: string;
      };
      transfer_assets: {
        dest: string;
        beneficiary: string;
        assets: string;
        feeAssetItem: string;
        weightLimit: string;
      };
      claim_assets: {
        assets: string;
        beneficiary: string;
      };
      transfer_assets_using_type_and_then: {
        dest: string;
        assets: string;
        assetsTransferType: string;
        remoteFeesId: string;
        feesTransferType: string;
        customXcmOnDest: string;
        weightLimit: string;
      };
    };
  };
  /**
   * Lookup306: xcm::VersionedXcm<RuntimeCall>
   **/
  XcmVersionedXcm: {
    _enum: {
      __Unused0: string;
      __Unused1: string;
      V2: string;
      V3: string;
      V4: string;
    };
  };
  /**
   * Lookup307: xcm::v2::Xcm<RuntimeCall>
   **/
  XcmV2Xcm: string;
  /**
   * Lookup309: xcm::v2::Instruction<RuntimeCall>
   **/
  XcmV2Instruction: {
    _enum: {
      WithdrawAsset: string;
      ReserveAssetDeposited: string;
      ReceiveTeleportedAsset: string;
      QueryResponse: {
        queryId: string;
        response: string;
        maxWeight: string;
      };
      TransferAsset: {
        assets: string;
        beneficiary: string;
      };
      TransferReserveAsset: {
        assets: string;
        dest: string;
        xcm: string;
      };
      Transact: {
        originType: string;
        requireWeightAtMost: string;
        call: string;
      };
      HrmpNewChannelOpenRequest: {
        sender: string;
        maxMessageSize: string;
        maxCapacity: string;
      };
      HrmpChannelAccepted: {
        recipient: string;
      };
      HrmpChannelClosing: {
        initiator: string;
        sender: string;
        recipient: string;
      };
      ClearOrigin: string;
      DescendOrigin: string;
      ReportError: {
        queryId: string;
        dest: string;
        maxResponseWeight: string;
      };
      DepositAsset: {
        assets: string;
        maxAssets: string;
        beneficiary: string;
      };
      DepositReserveAsset: {
        assets: string;
        maxAssets: string;
        dest: string;
        xcm: string;
      };
      ExchangeAsset: {
        give: string;
        receive: string;
      };
      InitiateReserveWithdraw: {
        assets: string;
        reserve: string;
        xcm: string;
      };
      InitiateTeleport: {
        assets: string;
        dest: string;
        xcm: string;
      };
      QueryHolding: {
        queryId: string;
        dest: string;
        assets: string;
        maxResponseWeight: string;
      };
      BuyExecution: {
        fees: string;
        weightLimit: string;
      };
      RefundSurplus: string;
      SetErrorHandler: string;
      SetAppendix: string;
      ClearError: string;
      ClaimAsset: {
        assets: string;
        ticket: string;
      };
      Trap: string;
      SubscribeVersion: {
        queryId: string;
        maxResponseWeight: string;
      };
      UnsubscribeVersion: string;
    };
  };
  /**
   * Lookup310: xcm::v2::Response
   **/
  XcmV2Response: {
    _enum: {
      Null: string;
      Assets: string;
      ExecutionResult: string;
      Version: string;
    };
  };
  /**
   * Lookup313: xcm::v2::traits::Error
   **/
  XcmV2TraitsError: {
    _enum: {
      Overflow: string;
      Unimplemented: string;
      UntrustedReserveLocation: string;
      UntrustedTeleportLocation: string;
      MultiLocationFull: string;
      MultiLocationNotInvertible: string;
      BadOrigin: string;
      InvalidLocation: string;
      AssetNotFound: string;
      FailedToTransactAsset: string;
      NotWithdrawable: string;
      LocationCannotHold: string;
      ExceedsMaxMessageSize: string;
      DestinationUnsupported: string;
      Transport: string;
      Unroutable: string;
      UnknownClaim: string;
      FailedToDecode: string;
      MaxWeightInvalid: string;
      NotHoldingFees: string;
      TooExpensive: string;
      Trap: string;
      UnhandledXcmVersion: string;
      WeightLimitReached: string;
      Barrier: string;
      WeightNotComputable: string;
    };
  };
  /**
   * Lookup314: xcm::v2::OriginKind
   **/
  XcmV2OriginKind: {
    _enum: string[];
  };
  /**
   * Lookup315: xcm::v2::multiasset::MultiAssetFilter
   **/
  XcmV2MultiassetMultiAssetFilter: {
    _enum: {
      Definite: string;
      Wild: string;
    };
  };
  /**
   * Lookup316: xcm::v2::multiasset::WildMultiAsset
   **/
  XcmV2MultiassetWildMultiAsset: {
    _enum: {
      All: string;
      AllOf: {
        id: string;
        fun: string;
      };
    };
  };
  /**
   * Lookup317: xcm::v2::multiasset::WildFungibility
   **/
  XcmV2MultiassetWildFungibility: {
    _enum: string[];
  };
  /**
   * Lookup318: xcm::v2::WeightLimit
   **/
  XcmV2WeightLimit: {
    _enum: {
      Unlimited: string;
      Limited: string;
    };
  };
  /**
   * Lookup319: xcm::v3::Xcm<Call>
   **/
  XcmV3Xcm: string;
  /**
   * Lookup321: xcm::v3::Instruction<Call>
   **/
  XcmV3Instruction: {
    _enum: {
      WithdrawAsset: string;
      ReserveAssetDeposited: string;
      ReceiveTeleportedAsset: string;
      QueryResponse: {
        queryId: string;
        response: string;
        maxWeight: string;
        querier: string;
      };
      TransferAsset: {
        assets: string;
        beneficiary: string;
      };
      TransferReserveAsset: {
        assets: string;
        dest: string;
        xcm: string;
      };
      Transact: {
        originKind: string;
        requireWeightAtMost: string;
        call: string;
      };
      HrmpNewChannelOpenRequest: {
        sender: string;
        maxMessageSize: string;
        maxCapacity: string;
      };
      HrmpChannelAccepted: {
        recipient: string;
      };
      HrmpChannelClosing: {
        initiator: string;
        sender: string;
        recipient: string;
      };
      ClearOrigin: string;
      DescendOrigin: string;
      ReportError: string;
      DepositAsset: {
        assets: string;
        beneficiary: string;
      };
      DepositReserveAsset: {
        assets: string;
        dest: string;
        xcm: string;
      };
      ExchangeAsset: {
        give: string;
        want: string;
        maximal: string;
      };
      InitiateReserveWithdraw: {
        assets: string;
        reserve: string;
        xcm: string;
      };
      InitiateTeleport: {
        assets: string;
        dest: string;
        xcm: string;
      };
      ReportHolding: {
        responseInfo: string;
        assets: string;
      };
      BuyExecution: {
        fees: string;
        weightLimit: string;
      };
      RefundSurplus: string;
      SetErrorHandler: string;
      SetAppendix: string;
      ClearError: string;
      ClaimAsset: {
        assets: string;
        ticket: string;
      };
      Trap: string;
      SubscribeVersion: {
        queryId: string;
        maxResponseWeight: string;
      };
      UnsubscribeVersion: string;
      BurnAsset: string;
      ExpectAsset: string;
      ExpectOrigin: string;
      ExpectError: string;
      ExpectTransactStatus: string;
      QueryPallet: {
        moduleName: string;
        responseInfo: string;
      };
      ExpectPallet: {
        index: string;
        name: string;
        moduleName: string;
        crateMajor: string;
        minCrateMinor: string;
      };
      ReportTransactStatus: string;
      ClearTransactStatus: string;
      UniversalOrigin: string;
      ExportMessage: {
        network: string;
        destination: string;
        xcm: string;
      };
      LockAsset: {
        asset: string;
        unlocker: string;
      };
      UnlockAsset: {
        asset: string;
        target: string;
      };
      NoteUnlockable: {
        asset: string;
        owner: string;
      };
      RequestUnlock: {
        asset: string;
        locker: string;
      };
      SetFeesMode: {
        jitWithdraw: string;
      };
      SetTopic: string;
      ClearTopic: string;
      AliasOrigin: string;
      UnpaidExecution: {
        weightLimit: string;
        checkOrigin: string;
      };
    };
  };
  /**
   * Lookup322: xcm::v3::Response
   **/
  XcmV3Response: {
    _enum: {
      Null: string;
      Assets: string;
      ExecutionResult: string;
      Version: string;
      PalletsInfo: string;
      DispatchResult: string;
    };
  };
  /**
   * Lookup324: xcm::v3::PalletInfo
   **/
  XcmV3PalletInfo: {
    index: string;
    name: string;
    moduleName: string;
    major: string;
    minor: string;
    patch: string;
  };
  /**
   * Lookup328: xcm::v3::QueryResponseInfo
   **/
  XcmV3QueryResponseInfo: {
    destination: string;
    queryId: string;
    maxWeight: string;
  };
  /**
   * Lookup329: xcm::v3::multiasset::MultiAssetFilter
   **/
  XcmV3MultiassetMultiAssetFilter: {
    _enum: {
      Definite: string;
      Wild: string;
    };
  };
  /**
   * Lookup330: xcm::v3::multiasset::WildMultiAsset
   **/
  XcmV3MultiassetWildMultiAsset: {
    _enum: {
      All: string;
      AllOf: {
        id: string;
        fun: string;
      };
      AllCounted: string;
      AllOfCounted: {
        id: string;
        fun: string;
        count: string;
      };
    };
  };
  /**
   * Lookup331: xcm::v3::multiasset::WildFungibility
   **/
  XcmV3MultiassetWildFungibility: {
    _enum: string[];
  };
  /**
   * Lookup343: staging_xcm_executor::traits::asset_transfer::TransferType
   **/
  StagingXcmExecutorAssetTransferTransferType: {
    _enum: {
      Teleport: string;
      LocalReserve: string;
      DestinationReserve: string;
      RemoteReserve: string;
    };
  };
  /**
   * Lookup344: xcm::VersionedAssetId
   **/
  XcmVersionedAssetId: {
    _enum: {
      __Unused0: string;
      __Unused1: string;
      __Unused2: string;
      V3: string;
      V4: string;
    };
  };
  /**
   * Lookup345: cumulus_pallet_xcm::pallet::Call<T>
   **/
  CumulusPalletXcmCall: string;
  /**
   * Lookup346: pallet_message_queue::pallet::Call<T>
   **/
  PalletMessageQueueCall: {
    _enum: {
      reap_page: {
        messageOrigin: string;
        pageIndex: string;
      };
      execute_overweight: {
        messageOrigin: string;
        page: string;
        index: string;
        weightLimit: string;
      };
    };
  };
  /**
   * Lookup347: pallet_storage_providers::pallet::Call<T>
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
   * Lookup348: pallet_file_system::pallet::Call<T>
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
      delete_file: {
        _alias: {
          size_: string;
        };
        bucketId: string;
        fileKey: string;
        location: string;
        size_: string;
        fingerprint: string;
        maybeInclusionForestProof: string;
      };
      pending_file_deletion_request_submit_proof: {
        user: string;
        fileKey: string;
        fileSize: string;
        bucketId: string;
        forestProof: string;
      };
    };
  };
  /**
   * Lookup349: pallet_file_system::types::BucketMoveRequestResponse
   **/
  PalletFileSystemBucketMoveRequestResponse: {
    _enum: string[];
  };
  /**
   * Lookup350: pallet_file_system::types::ReplicationTarget<T>
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
   * Lookup352: pallet_file_system::types::StorageRequestMspBucketResponse<T>
   **/
  PalletFileSystemStorageRequestMspBucketResponse: {
    bucketId: string;
    accept: string;
    reject: string;
  };
  /**
   * Lookup354: pallet_file_system::types::StorageRequestMspAcceptedFileKeys<T>
   **/
  PalletFileSystemStorageRequestMspAcceptedFileKeys: {
    fileKeysAndProofs: string;
    forestProof: string;
  };
  /**
   * Lookup356: pallet_file_system::types::FileKeyWithProof<T>
   **/
  PalletFileSystemFileKeyWithProof: {
    fileKey: string;
    proof: string;
  };
  /**
   * Lookup358: pallet_file_system::types::RejectedStorageRequest<T>
   **/
  PalletFileSystemRejectedStorageRequest: {
    fileKey: string;
    reason: string;
  };
  /**
   * Lookup361: pallet_proofs_dealer::pallet::Call<T>
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
    };
  };
  /**
   * Lookup362: pallet_randomness::pallet::Call<T>
   **/
  PalletRandomnessCall: {
    _enum: string[];
  };
  /**
   * Lookup363: pallet_payment_streams::pallet::Call<T>
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
   * Lookup364: pallet_bucket_nfts::pallet::Call<T>
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
   * Lookup366: pallet_nfts::pallet::Call<T, I>
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
   * Lookup367: pallet_nfts::types::CollectionConfig<Price, BlockNumber, CollectionId>
   **/
  PalletNftsCollectionConfig: {
    settings: string;
    maxSupply: string;
    mintSettings: string;
  };
  /**
   * Lookup369: pallet_nfts::types::CollectionSetting
   **/
  PalletNftsCollectionSetting: {
    _enum: string[];
  };
  /**
   * Lookup370: pallet_nfts::types::MintSettings<Price, BlockNumber, CollectionId>
   **/
  PalletNftsMintSettings: {
    mintType: string;
    price: string;
    startBlock: string;
    endBlock: string;
    defaultItemSettings: string;
  };
  /**
   * Lookup371: pallet_nfts::types::MintType<CollectionId>
   **/
  PalletNftsMintType: {
    _enum: {
      Issuer: string;
      Public: string;
      HolderOf: string;
    };
  };
  /**
   * Lookup374: pallet_nfts::types::ItemSetting
   **/
  PalletNftsItemSetting: {
    _enum: string[];
  };
  /**
   * Lookup375: pallet_nfts::types::DestroyWitness
   **/
  PalletNftsDestroyWitness: {
    itemMetadatas: string;
    itemConfigs: string;
    attributes: string;
  };
  /**
   * Lookup377: pallet_nfts::types::MintWitness<ItemId, Balance>
   **/
  PalletNftsMintWitness: {
    ownedItem: string;
    mintPrice: string;
  };
  /**
   * Lookup378: pallet_nfts::types::ItemConfig
   **/
  PalletNftsItemConfig: {
    settings: string;
  };
  /**
   * Lookup380: pallet_nfts::types::CancelAttributesApprovalWitness
   **/
  PalletNftsCancelAttributesApprovalWitness: {
    accountAttributes: string;
  };
  /**
   * Lookup382: pallet_nfts::types::ItemTip<CollectionId, ItemId, sp_core::crypto::AccountId32, Amount>
   **/
  PalletNftsItemTip: {
    collection: string;
    item: string;
    receiver: string;
    amount: string;
  };
  /**
   * Lookup384: pallet_nfts::types::PreSignedMint<CollectionId, ItemId, sp_core::crypto::AccountId32, Deadline, Balance>
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
   * Lookup385: sp_runtime::MultiSignature
   **/
  SpRuntimeMultiSignature: {
    _enum: {
      Ed25519: string;
      Sr25519: string;
      Ecdsa: string;
    };
  };
  /**
   * Lookup388: pallet_nfts::types::PreSignedAttributes<CollectionId, ItemId, sp_core::crypto::AccountId32, Deadline>
   **/
  PalletNftsPreSignedAttributes: {
    collection: string;
    item: string;
    attributes: string;
    namespace: string;
    deadline: string;
  };
  /**
   * Lookup389: pallet_parameters::pallet::Call<T>
   **/
  PalletParametersCall: {
    _enum: {
      set_parameter: {
        keyValue: string;
      };
    };
  };
  /**
   * Lookup390: storage_hub_runtime::configs::runtime_params::RuntimeParameters
   **/
  StorageHubRuntimeConfigsRuntimeParamsRuntimeParameters: {
    _enum: {
      RuntimeConfig: string;
    };
  };
  /**
   * Lookup391: storage_hub_runtime::configs::runtime_params::dynamic_params::runtime_config::Parameters
   **/
  StorageHubRuntimeConfigsRuntimeParamsDynamicParamsRuntimeConfigParameters: {
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
   * Lookup393: pallet_sudo::pallet::Error<T>
   **/
  PalletSudoError: {
    _enum: string[];
  };
  /**
   * Lookup396: pallet_collator_selection::pallet::CandidateInfo<sp_core::crypto::AccountId32, Balance>
   **/
  PalletCollatorSelectionCandidateInfo: {
    who: string;
    deposit: string;
  };
  /**
   * Lookup398: pallet_collator_selection::pallet::Error<T>
   **/
  PalletCollatorSelectionError: {
    _enum: string[];
  };
  /**
   * Lookup402: sp_core::crypto::KeyTypeId
   **/
  SpCoreCryptoKeyTypeId: string;
  /**
   * Lookup403: pallet_session::pallet::Error<T>
   **/
  PalletSessionError: {
    _enum: string[];
  };
  /**
   * Lookup412: cumulus_pallet_xcmp_queue::OutboundChannelDetails
   **/
  CumulusPalletXcmpQueueOutboundChannelDetails: {
    recipient: string;
    state: string;
    signalsExist: string;
    firstIndex: string;
    lastIndex: string;
  };
  /**
   * Lookup413: cumulus_pallet_xcmp_queue::OutboundState
   **/
  CumulusPalletXcmpQueueOutboundState: {
    _enum: string[];
  };
  /**
   * Lookup417: cumulus_pallet_xcmp_queue::QueueConfigData
   **/
  CumulusPalletXcmpQueueQueueConfigData: {
    suspendThreshold: string;
    dropThreshold: string;
    resumeThreshold: string;
  };
  /**
   * Lookup418: cumulus_pallet_xcmp_queue::pallet::Error<T>
   **/
  CumulusPalletXcmpQueueError: {
    _enum: string[];
  };
  /**
   * Lookup419: pallet_xcm::pallet::QueryStatus<BlockNumber>
   **/
  PalletXcmQueryStatus: {
    _enum: {
      Pending: {
        responder: string;
        maybeMatchQuerier: string;
        maybeNotify: string;
        timeout: string;
      };
      VersionNotifier: {
        origin: string;
        isActive: string;
      };
      Ready: {
        response: string;
        at: string;
      };
    };
  };
  /**
   * Lookup423: xcm::VersionedResponse
   **/
  XcmVersionedResponse: {
    _enum: {
      __Unused0: string;
      __Unused1: string;
      V2: string;
      V3: string;
      V4: string;
    };
  };
  /**
   * Lookup429: pallet_xcm::pallet::VersionMigrationStage
   **/
  PalletXcmVersionMigrationStage: {
    _enum: {
      MigrateSupportedVersion: string;
      MigrateVersionNotifiers: string;
      NotifyCurrentTargets: string;
      MigrateAndNotifyOldTargets: string;
    };
  };
  /**
   * Lookup431: pallet_xcm::pallet::RemoteLockedFungibleRecord<ConsumerIdentifier, MaxConsumers>
   **/
  PalletXcmRemoteLockedFungibleRecord: {
    amount: string;
    owner: string;
    locker: string;
    consumers: string;
  };
  /**
   * Lookup438: pallet_xcm::pallet::Error<T>
   **/
  PalletXcmError: {
    _enum: string[];
  };
  /**
   * Lookup439: pallet_message_queue::BookState<cumulus_primitives_core::AggregateMessageOrigin>
   **/
  PalletMessageQueueBookState: {
    _alias: {
      size_: string;
    };
    begin: string;
    end: string;
    count: string;
    readyNeighbours: string;
    messageCount: string;
    size_: string;
  };
  /**
   * Lookup441: pallet_message_queue::Neighbours<cumulus_primitives_core::AggregateMessageOrigin>
   **/
  PalletMessageQueueNeighbours: {
    prev: string;
    next: string;
  };
  /**
   * Lookup443: pallet_message_queue::Page<Size, HeapSize>
   **/
  PalletMessageQueuePage: {
    remaining: string;
    remainingSize: string;
    firstIndex: string;
    first: string;
    last: string;
    heap: string;
  };
  /**
   * Lookup445: pallet_message_queue::pallet::Error<T>
   **/
  PalletMessageQueueError: {
    _enum: string[];
  };
  /**
   * Lookup446: pallet_storage_providers::types::SignUpRequest<T>
   **/
  PalletStorageProvidersSignUpRequest: {
    spSignUpRequest: string;
    at: string;
  };
  /**
   * Lookup447: pallet_storage_providers::types::SignUpRequestSpParams<T>
   **/
  PalletStorageProvidersSignUpRequestSpParams: {
    _enum: {
      BackupStorageProvider: string;
      MainStorageProvider: string;
    };
  };
  /**
   * Lookup448: pallet_storage_providers::types::BackupStorageProvider<T>
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
   * Lookup449: pallet_storage_providers::types::MainStorageProviderSignUpRequest<T>
   **/
  PalletStorageProvidersMainStorageProviderSignUpRequest: {
    mspInfo: string;
    valueProp: string;
  };
  /**
   * Lookup450: pallet_storage_providers::types::MainStorageProvider<T>
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
   * Lookup451: pallet_storage_providers::types::Bucket<T>
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
   * Lookup455: pallet_storage_providers::pallet::Error<T>
   **/
  PalletStorageProvidersError: {
    _enum: string[];
  };
  /**
   * Lookup456: pallet_file_system::types::StorageRequestMetadata<T>
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
    msp: string;
    userPeerIds: string;
    bspsRequired: string;
    bspsConfirmed: string;
    bspsVolunteered: string;
    depositPaid: string;
  };
  /**
   * Lookup459: pallet_file_system::types::StorageRequestBspsMetadata<T>
   **/
  PalletFileSystemStorageRequestBspsMetadata: {
    confirmed: string;
  };
  /**
   * Lookup462: pallet_file_system::types::PendingFileDeletionRequest<T>
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
   * Lookup464: pallet_file_system::types::PendingStopStoringRequest<T>
   **/
  PalletFileSystemPendingStopStoringRequest: {
    tickWhenRequested: string;
    fileOwner: string;
    fileSize: string;
  };
  /**
   * Lookup465: pallet_file_system::types::MoveBucketRequestMetadata<T>
   **/
  PalletFileSystemMoveBucketRequestMetadata: {
    requester: string;
    newMspId: string;
    newValuePropId: string;
  };
  /**
   * Lookup466: pallet_file_system::pallet::Error<T>
   **/
  PalletFileSystemError: {
    _enum: string[];
  };
  /**
   * Lookup468: pallet_proofs_dealer::types::ProofSubmissionRecord<T>
   **/
  PalletProofsDealerProofSubmissionRecord: {
    lastTickProven: string;
    nextTickToSubmitProofFor: string;
  };
  /**
   * Lookup475: pallet_proofs_dealer::pallet::Error<T>
   **/
  PalletProofsDealerError: {
    _enum: string[];
  };
  /**
   * Lookup478: pallet_payment_streams::types::FixedRatePaymentStream<T>
   **/
  PalletPaymentStreamsFixedRatePaymentStream: {
    rate: string;
    lastChargedTick: string;
    userDeposit: string;
    outOfFundsTick: string;
  };
  /**
   * Lookup479: pallet_payment_streams::types::DynamicRatePaymentStream<T>
   **/
  PalletPaymentStreamsDynamicRatePaymentStream: {
    amountProvided: string;
    priceIndexWhenLastCharged: string;
    userDeposit: string;
    outOfFundsTick: string;
  };
  /**
   * Lookup480: pallet_payment_streams::types::ProviderLastChargeableInfo<T>
   **/
  PalletPaymentStreamsProviderLastChargeableInfo: {
    lastChargeableTick: string;
    priceIndex: string;
  };
  /**
   * Lookup481: pallet_payment_streams::pallet::Error<T>
   **/
  PalletPaymentStreamsError: {
    _enum: string[];
  };
  /**
   * Lookup482: pallet_bucket_nfts::pallet::Error<T>
   **/
  PalletBucketNftsError: {
    _enum: string[];
  };
  /**
   * Lookup483: pallet_nfts::types::CollectionDetails<sp_core::crypto::AccountId32, DepositBalance>
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
   * Lookup488: pallet_nfts::types::CollectionRole
   **/
  PalletNftsCollectionRole: {
    _enum: string[];
  };
  /**
   * Lookup489: pallet_nfts::types::ItemDetails<sp_core::crypto::AccountId32, pallet_nfts::types::ItemDeposit<DepositBalance, sp_core::crypto::AccountId32>, bounded_collections::bounded_btree_map::BoundedBTreeMap<sp_core::crypto::AccountId32, Option<T>, S>>
   **/
  PalletNftsItemDetails: {
    owner: string;
    approvals: string;
    deposit: string;
  };
  /**
   * Lookup490: pallet_nfts::types::ItemDeposit<DepositBalance, sp_core::crypto::AccountId32>
   **/
  PalletNftsItemDeposit: {
    account: string;
    amount: string;
  };
  /**
   * Lookup495: pallet_nfts::types::CollectionMetadata<Deposit, StringLimit>
   **/
  PalletNftsCollectionMetadata: {
    deposit: string;
    data: string;
  };
  /**
   * Lookup496: pallet_nfts::types::ItemMetadata<pallet_nfts::types::ItemMetadataDeposit<DepositBalance, sp_core::crypto::AccountId32>, StringLimit>
   **/
  PalletNftsItemMetadata: {
    deposit: string;
    data: string;
  };
  /**
   * Lookup497: pallet_nfts::types::ItemMetadataDeposit<DepositBalance, sp_core::crypto::AccountId32>
   **/
  PalletNftsItemMetadataDeposit: {
    account: string;
    amount: string;
  };
  /**
   * Lookup500: pallet_nfts::types::AttributeDeposit<DepositBalance, sp_core::crypto::AccountId32>
   **/
  PalletNftsAttributeDeposit: {
    account: string;
    amount: string;
  };
  /**
   * Lookup504: pallet_nfts::types::PendingSwap<CollectionId, ItemId, pallet_nfts::types::PriceWithDirection<Amount>, Deadline>
   **/
  PalletNftsPendingSwap: {
    desiredCollection: string;
    desiredItem: string;
    price: string;
    deadline: string;
  };
  /**
   * Lookup506: pallet_nfts::types::PalletFeature
   **/
  PalletNftsPalletFeature: {
    _enum: string[];
  };
  /**
   * Lookup507: pallet_nfts::pallet::Error<T, I>
   **/
  PalletNftsError: {
    _enum: string[];
  };
  /**
   * Lookup510: frame_system::extensions::check_non_zero_sender::CheckNonZeroSender<T>
   **/
  FrameSystemExtensionsCheckNonZeroSender: string;
  /**
   * Lookup511: frame_system::extensions::check_spec_version::CheckSpecVersion<T>
   **/
  FrameSystemExtensionsCheckSpecVersion: string;
  /**
   * Lookup512: frame_system::extensions::check_tx_version::CheckTxVersion<T>
   **/
  FrameSystemExtensionsCheckTxVersion: string;
  /**
   * Lookup513: frame_system::extensions::check_genesis::CheckGenesis<T>
   **/
  FrameSystemExtensionsCheckGenesis: string;
  /**
   * Lookup516: frame_system::extensions::check_nonce::CheckNonce<T>
   **/
  FrameSystemExtensionsCheckNonce: string;
  /**
   * Lookup517: frame_system::extensions::check_weight::CheckWeight<T>
   **/
  FrameSystemExtensionsCheckWeight: string;
  /**
   * Lookup518: pallet_transaction_payment::ChargeTransactionPayment<T>
   **/
  PalletTransactionPaymentChargeTransactionPayment: string;
  /**
   * Lookup519: cumulus_primitives_storage_weight_reclaim::StorageWeightReclaim<T>
   **/
  CumulusPrimitivesStorageWeightReclaimStorageWeightReclaim: string;
  /**
   * Lookup520: frame_metadata_hash_extension::CheckMetadataHash<T>
   **/
  FrameMetadataHashExtensionCheckMetadataHash: {
    mode: string;
  };
  /**
   * Lookup521: frame_metadata_hash_extension::Mode
   **/
  FrameMetadataHashExtensionMode: {
    _enum: string[];
  };
  /**
   * Lookup522: storage_hub_runtime::Runtime
   **/
  StorageHubRuntimeRuntime: string;
};
export default _default;
