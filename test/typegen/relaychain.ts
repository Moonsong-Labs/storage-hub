import {
  SS58String,
  Binary,
  ResultPayload,
  StorageDescriptor,
  PlainDescriptor,
  TxDescriptor,
  RuntimeDescriptor,
  Enum,
  _Enum,
  GetEnum,
  QueryFromDescriptors,
  TxFromDescriptors,
  EventsFromDescriptors,
  ErrorsFromDescriptors,
  ConstFromDescriptors,
} from "@polkadot-api/client";

type AnonymousEnum<T extends {}> = T & {
  __anonymous: true;
};

type IEnum<T extends {}> = Enum<
  {
    [K in keyof T & string]: { type: K; value: T[K] };
  }[keyof T & string]
>;

type MyTuple<T> = [T, ...T[]];

type SeparateUndefined<T> = undefined extends T ? undefined | Exclude<T, undefined> : T;

type Anonymize<T> = SeparateUndefined<
  T extends
    | string
    | number
    | bigint
    | boolean
    | void
    | undefined
    | null
    | symbol
    | Binary
    | Uint8Array
    | Enum<{ type: string; value: any }>
    ? T
    : T extends AnonymousEnum<infer V>
      ? IEnum<V>
      : T extends MyTuple<any>
        ? {
            [K in keyof T]: T[K];
          }
        : T extends []
          ? []
          : T extends Array<infer A>
            ? Array<A>
            : {
                [K in keyof T & string]: T[K];
              }
>;

type I5sesotjlssv2d = {
  nonce: number;
  consumers: number;
  providers: number;
  sufficients: number;
  data: Anonymize<I1q8tnt1cluu5j>;
};

type I1q8tnt1cluu5j = { free: bigint; reserved: bigint; frozen: bigint; flags: bigint };

type Iffmde3ekjedi9 = {
  normal: Anonymize<I4q39t5hn830vp>;
  operational: Anonymize<I4q39t5hn830vp>;
  mandatory: Anonymize<I4q39t5hn830vp>;
};

type I4q39t5hn830vp = { ref_time: bigint; proof_size: bigint };

type Idin6nhq46lvdj = Array<DigestItem>;

export type DigestItem = Enum<
  | { type: "PreRuntime"; value: Anonymize<Idhk5e7nto8mrb> }
  | { type: "Consensus"; value: Anonymize<Idhk5e7nto8mrb> }
  | { type: "Seal"; value: Anonymize<Idhk5e7nto8mrb> }
  | { type: "Other"; value: Anonymize<Binary> }
  | { type: "RuntimeEnvironmentUpdated"; value: undefined }
>;
export const DigestItem = _Enum as unknown as GetEnum<DigestItem>;

type Idhk5e7nto8mrb = [Binary, Binary];

type I2oomvrncvb8jh = Array<Anonymize<I9pt9i86qkqfgf>>;

type I9pt9i86qkqfgf = {
  phase: Phase;
  event: Anonymize<I5b093r0uvbtnc>;
  topics: Anonymize<Idhnf6rtqoslea>;
};

export type Phase = Enum<
  | { type: "ApplyExtrinsic"; value: Anonymize<number> }
  | { type: "Finalization"; value: undefined }
  | { type: "Initialization"; value: undefined }
>;
export const Phase = _Enum as unknown as GetEnum<Phase>;

type I5b093r0uvbtnc = AnonymousEnum<{
  System: Anonymize<PalletEvent>;
  Indices: Anonymize<IndicesEvent>;
  Balances: Anonymize<BalancesEvent>;
  TransactionPayment: Anonymize<TransactionPaymentEvent>;
  Offences: Anonymize<OffencesEvent>;
  Session: Anonymize<SessionEvent>;
  Grandpa: Anonymize<GrandpaEvent>;
  Treasury: Anonymize<TreasuryEvent>;
  ConvictionVoting: Anonymize<ConvictionVotingEvent>;
  Referenda: Anonymize<ReferendaEvent>;
  FellowshipCollective: Anonymize<I8s9ti4a44j0bp>;
  FellowshipReferenda: Anonymize<Idun3t58do26lr>;
  Whitelist: Anonymize<WhitelistEvent>;
  Claims: Anonymize<CommonClaimsEvent>;
  Utility: Anonymize<UtilityEvent>;
  Identity: Anonymize<IdentityEvent>;
  Society: Anonymize<Ifhcq78pr9md4e>;
  Recovery: Anonymize<RecoveryEvent>;
  Vesting: Anonymize<VestingEvent>;
  Scheduler: Anonymize<SchedulerEvent>;
  Proxy: Anonymize<I61uqajj8pqisn>;
  Multisig: Anonymize<MultisigEvent>;
  Preimage: Anonymize<PreimageEvent>;
  AssetRate: Anonymize<AssetRateEvent>;
  Bounties: Anonymize<Iei6k1tdcht3q>;
  ChildBounties: Anonymize<ChildBountiesEvent>;
  Nis: Anonymize<I9dmlhfape78ke>;
  NisCounterpartBalances: Anonymize<BalancesEvent>;
  ParaInclusion: Anonymize<ParachainsInclusionEvent>;
  Paras: Anonymize<ParachainsParasEvent>;
  Hrmp: Anonymize<PolkadotRuntimeParachainsHrmpEvent>;
  ParasDisputes: Anonymize<ParachainsDisputesEvent>;
  MessageQueue: Anonymize<MessageQueueEvent>;
  OnDemandAssignmentProvider: Anonymize<PolkadotRuntimeParachainsAssignerOnDemandEvent>;
  Registrar: Anonymize<CommonParasRegistrarEvent>;
  Slots: Anonymize<CommonSlotsEvent>;
  Auctions: Anonymize<CommonAuctionsEvent>;
  Crowdloan: Anonymize<CommonCrowdloanEvent>;
  Coretime: Anonymize<PolkadotRuntimeParachainsCoretimeEvent>;
  XcmPallet: Anonymize<XcmEvent>;
  IdentityMigrator: Anonymize<PolkadotRuntimeCommonIdentityMigratorEvent>;
  AssignedSlots: Anonymize<PolkadotRuntimeCommonAssignedSlotsEvent>;
  ValidatorManager: Anonymize<I6p26ej1p3cokp>;
  StateTrieMigration: Anonymize<I1iphd51t39ha0>;
  RootTesting: Anonymize<RootTestingEvent>;
  Sudo: Anonymize<SudoEvent>;
}>;

export type PalletEvent = Enum<
  | { type: "ExtrinsicSuccess"; value: Anonymize<Iede1ukavoderd> }
  | { type: "ExtrinsicFailed"; value: Anonymize<Iennefu6o2bgdm> }
  | { type: "CodeUpdated"; value: undefined }
  | { type: "NewAccount"; value: Anonymize<Icbccs0ug47ilf> }
  | { type: "KilledAccount"; value: Anonymize<Icbccs0ug47ilf> }
  | { type: "Remarked"; value: Anonymize<Ieob37pbjnvmkj> }
  | { type: "UpgradeAuthorized"; value: Anonymize<Ib3qt1mgvgmbgi> }
>;
export const PalletEvent = _Enum as unknown as GetEnum<PalletEvent>;

type Iede1ukavoderd = { dispatch_info: Anonymize<Ia2iiohca2et6f> };

type Ia2iiohca2et6f = {
  weight: Anonymize<I4q39t5hn830vp>;
  class: DispatchClass;
  pays_fee: DispatchPays;
};

export type DispatchClass = Enum<
  | { type: "Normal"; value: undefined }
  | { type: "Operational"; value: undefined }
  | { type: "Mandatory"; value: undefined }
>;
export const DispatchClass = _Enum as unknown as GetEnum<DispatchClass>;

export type DispatchPays = Enum<
  { type: "Yes"; value: undefined } | { type: "No"; value: undefined }
>;
export const DispatchPays = _Enum as unknown as GetEnum<DispatchPays>;

type Iennefu6o2bgdm = { dispatch_error: DispatchError; dispatch_info: Anonymize<Ia2iiohca2et6f> };

export type DispatchError = Enum<
  | { type: "Other"; value: undefined }
  | { type: "CannotLookup"; value: undefined }
  | { type: "BadOrigin"; value: undefined }
  | { type: "Module"; value: Anonymize<I9mtpf03dt7lqs> }
  | { type: "ConsumerRemaining"; value: undefined }
  | { type: "NoProviders"; value: undefined }
  | { type: "TooManyConsumers"; value: undefined }
  | { type: "Token"; value: Anonymize<TokenError> }
  | { type: "Arithmetic"; value: Anonymize<ArithmeticError> }
  | { type: "Transactional"; value: Anonymize<TransactionalError> }
  | { type: "Exhausted"; value: undefined }
  | { type: "Corruption"; value: undefined }
  | { type: "Unavailable"; value: undefined }
  | { type: "RootNotAllowed"; value: undefined }
>;
export const DispatchError = _Enum as unknown as GetEnum<DispatchError>;

type I9mtpf03dt7lqs = { index: number; error: Binary };

export type TokenError = Enum<
  | { type: "FundsUnavailable"; value: undefined }
  | { type: "OnlyProvider"; value: undefined }
  | { type: "BelowMinimum"; value: undefined }
  | { type: "CannotCreate"; value: undefined }
  | { type: "UnknownAsset"; value: undefined }
  | { type: "Frozen"; value: undefined }
  | { type: "Unsupported"; value: undefined }
  | { type: "CannotCreateHold"; value: undefined }
  | { type: "NotExpendable"; value: undefined }
  | { type: "Blocked"; value: undefined }
>;
export const TokenError = _Enum as unknown as GetEnum<TokenError>;

export type ArithmeticError = Enum<
  | { type: "Underflow"; value: undefined }
  | { type: "Overflow"; value: undefined }
  | { type: "DivisionByZero"; value: undefined }
>;
export const ArithmeticError = _Enum as unknown as GetEnum<ArithmeticError>;

export type TransactionalError = Enum<
  { type: "LimitReached"; value: undefined } | { type: "NoLayer"; value: undefined }
>;
export const TransactionalError = _Enum as unknown as GetEnum<TransactionalError>;

type Icbccs0ug47ilf = { account: SS58String };

type Ieob37pbjnvmkj = { sender: SS58String; hash: Binary };

type Ib3qt1mgvgmbgi = { code_hash: Binary; check_version: boolean };

export type IndicesEvent = Enum<
  | { type: "IndexAssigned"; value: Anonymize<Ia1u3jll6a06ae> }
  | { type: "IndexFreed"; value: Anonymize<I666bl2fqjkejo> }
  | { type: "IndexFrozen"; value: Anonymize<Ia1u3jll6a06ae> }
>;
export const IndicesEvent = _Enum as unknown as GetEnum<IndicesEvent>;

type Ia1u3jll6a06ae = { who: SS58String; index: number };

type I666bl2fqjkejo = { index: number };

export type BalancesEvent = Enum<
  | { type: "Endowed"; value: Anonymize<Icv68aq8841478> }
  | { type: "DustLost"; value: Anonymize<Ic262ibdoec56a> }
  | { type: "Transfer"; value: Anonymize<Iflcfm9b6nlmdd> }
  | { type: "BalanceSet"; value: Anonymize<Ijrsf4mnp3eka> }
  | { type: "Reserved"; value: Anonymize<Id5fm4p8lj5qgi> }
  | { type: "Unreserved"; value: Anonymize<Id5fm4p8lj5qgi> }
  | { type: "ReserveRepatriated"; value: Anonymize<Idm5rqp3duosod> }
  | { type: "Deposit"; value: Anonymize<Id5fm4p8lj5qgi> }
  | { type: "Withdraw"; value: Anonymize<Id5fm4p8lj5qgi> }
  | { type: "Slashed"; value: Anonymize<Id5fm4p8lj5qgi> }
  | { type: "Minted"; value: Anonymize<Id5fm4p8lj5qgi> }
  | { type: "Burned"; value: Anonymize<Id5fm4p8lj5qgi> }
  | { type: "Suspended"; value: Anonymize<Id5fm4p8lj5qgi> }
  | { type: "Restored"; value: Anonymize<Id5fm4p8lj5qgi> }
  | { type: "Upgraded"; value: Anonymize<I4cbvqmqadhrea> }
  | { type: "Issued"; value: Anonymize<I3qt1hgg4djhgb> }
  | { type: "Rescinded"; value: Anonymize<I3qt1hgg4djhgb> }
  | { type: "Locked"; value: Anonymize<Id5fm4p8lj5qgi> }
  | { type: "Unlocked"; value: Anonymize<Id5fm4p8lj5qgi> }
  | { type: "Frozen"; value: Anonymize<Id5fm4p8lj5qgi> }
  | { type: "Thawed"; value: Anonymize<Id5fm4p8lj5qgi> }
  | { type: "TotalIssuanceForced"; value: Anonymize<I4fooe9dun9o0t> }
>;
export const BalancesEvent = _Enum as unknown as GetEnum<BalancesEvent>;

type Icv68aq8841478 = { account: SS58String; free_balance: bigint };

type Ic262ibdoec56a = { account: SS58String; amount: bigint };

type Iflcfm9b6nlmdd = { from: SS58String; to: SS58String; amount: bigint };

type Ijrsf4mnp3eka = { who: SS58String; free: bigint };

type Id5fm4p8lj5qgi = { who: SS58String; amount: bigint };

type Idm5rqp3duosod = {
  from: SS58String;
  to: SS58String;
  amount: bigint;
  destination_status: BalanceStatus;
};

export type BalanceStatus = Enum<
  { type: "Free"; value: undefined } | { type: "Reserved"; value: undefined }
>;
export const BalanceStatus = _Enum as unknown as GetEnum<BalanceStatus>;

type I4cbvqmqadhrea = { who: SS58String };

type I3qt1hgg4djhgb = { amount: bigint };

type I4fooe9dun9o0t = { old: bigint; new: bigint };

export type TransactionPaymentEvent = Enum<{
  type: "TransactionFeePaid";
  value: Anonymize<Ier2cke86dqbr2>;
}>;
export const TransactionPaymentEvent = _Enum as unknown as GetEnum<TransactionPaymentEvent>;

type Ier2cke86dqbr2 = { who: SS58String; actual_fee: bigint; tip: bigint };

export type OffencesEvent = Enum<{ type: "Offence"; value: Anonymize<I41n4hddrgegvb> }>;
export const OffencesEvent = _Enum as unknown as GetEnum<OffencesEvent>;

type I41n4hddrgegvb = { kind: Binary; timeslot: Binary };

export type SessionEvent = Enum<{ type: "NewSession"; value: Anonymize<I2hq50pu2kdjpo> }>;
export const SessionEvent = _Enum as unknown as GetEnum<SessionEvent>;

type I2hq50pu2kdjpo = { session_index: number };

export type GrandpaEvent = Enum<
  | { type: "NewAuthorities"; value: Anonymize<Ib31jedabim0q7> }
  | { type: "Paused"; value: undefined }
  | { type: "Resumed"; value: undefined }
>;
export const GrandpaEvent = _Enum as unknown as GetEnum<GrandpaEvent>;

type Ib31jedabim0q7 = { authority_set: Anonymize<I2qinct8jq4bqe> };

type I2qinct8jq4bqe = Array<Anonymize<I3iuggguvi9njj>>;

type I3iuggguvi9njj = [Binary, bigint];

export type TreasuryEvent = Enum<
  | { type: "Proposed"; value: Anonymize<I44hc4lgsn4o1j> }
  | { type: "Spending"; value: Anonymize<I8iksqi3eani0a> }
  | { type: "Awarded"; value: Anonymize<I16enopmju1p0q> }
  | { type: "Rejected"; value: Anonymize<Ifgqhle2413de7> }
  | { type: "Burnt"; value: Anonymize<I43kq8qudg7pq9> }
  | { type: "Rollover"; value: Anonymize<I76riseemre533> }
  | { type: "Deposit"; value: Anonymize<Ie5v6njpckr05b> }
  | { type: "SpendApproved"; value: Anonymize<I38bmcrmh852rk> }
  | { type: "UpdatedInactive"; value: Anonymize<I4hcillge8de5f> }
  | { type: "AssetSpendApproved"; value: Anonymize<I5nk68406dirvd> }
  | { type: "AssetSpendVoided"; value: Anonymize<I666bl2fqjkejo> }
  | { type: "Paid"; value: Anonymize<Iek7v4hrgnq6iv> }
  | { type: "PaymentFailed"; value: Anonymize<Iek7v4hrgnq6iv> }
  | { type: "SpendProcessed"; value: Anonymize<I666bl2fqjkejo> }
>;
export const TreasuryEvent = _Enum as unknown as GetEnum<TreasuryEvent>;

type I44hc4lgsn4o1j = { proposal_index: number };

type I8iksqi3eani0a = { budget_remaining: bigint };

type I16enopmju1p0q = { proposal_index: number; award: bigint; account: SS58String };

type Ifgqhle2413de7 = { proposal_index: number; slashed: bigint };

type I43kq8qudg7pq9 = { burnt_funds: bigint };

type I76riseemre533 = { rollover_balance: bigint };

type Ie5v6njpckr05b = { value: bigint };

type I38bmcrmh852rk = { proposal_index: number; amount: bigint; beneficiary: SS58String };

type I4hcillge8de5f = { reactivated: bigint; deactivated: bigint };

type I5nk68406dirvd = {
  index: number;
  asset_kind: PolkadotRuntimeCommonImplsVersionedLocatableAsset;
  amount: bigint;
  beneficiary: XcmVersionedLocation;
  valid_from: number;
  expire_at: number;
};

export type PolkadotRuntimeCommonImplsVersionedLocatableAsset = Enum<
  | { type: "V3"; value: Anonymize<Ibkg14c9hm3bes> }
  | { type: "V4"; value: Anonymize<I845t8avig771m> }
>;
export const PolkadotRuntimeCommonImplsVersionedLocatableAsset =
  _Enum as unknown as GetEnum<PolkadotRuntimeCommonImplsVersionedLocatableAsset>;

type Ibkg14c9hm3bes = { location: Anonymize<Ie897ubj3a1vaq>; asset_id: XcmV3MultiassetAssetId };

type Ie897ubj3a1vaq = { parents: number; interior: XcmV3Junctions };

export type XcmV3Junctions = Enum<
  | { type: "Here"; value: undefined }
  | { type: "X1"; value: Anonymize<XcmV4Junction> }
  | { type: "X2"; value: Anonymize<I42l4nthiehb7> }
  | { type: "X3"; value: Anonymize<I2jk9pdm4ajs0n> }
  | { type: "X4"; value: Anonymize<I293rauivpnv4n> }
  | { type: "X5"; value: Anonymize<Id42rc2s9m61aa> }
  | { type: "X6"; value: Anonymize<Ibe9k3j6og3ch4> }
  | { type: "X7"; value: Anonymize<I3vkvorkiqho0h> }
  | { type: "X8"; value: Anonymize<Icmb7nn8ip4qrt> }
>;
export const XcmV3Junctions = _Enum as unknown as GetEnum<XcmV3Junctions>;

export type XcmV4Junction = Enum<
  | { type: "Parachain"; value: Anonymize<number> }
  | { type: "AccountId32"; value: Anonymize<I5891blicehaji> }
  | { type: "AccountIndex64"; value: Anonymize<Idrke3qhmim88u> }
  | { type: "AccountKey20"; value: Anonymize<I3liki1s5lgett> }
  | { type: "PalletInstance"; value: Anonymize<number> }
  | { type: "GeneralIndex"; value: Anonymize<bigint> }
  | { type: "GeneralKey"; value: Anonymize<Ic1rqnlu0a9i3k> }
  | { type: "OnlyChild"; value: undefined }
  | { type: "Plurality"; value: Anonymize<Ibb5u0oo9gtas> }
  | { type: "GlobalConsensus"; value: Anonymize<XcmV4JunctionNetworkId> }
>;
export const XcmV4Junction = _Enum as unknown as GetEnum<XcmV4Junction>;

type I5891blicehaji = { network: Anonymize<I41adbd3kv9dad>; id: Binary };

type I41adbd3kv9dad = XcmV4JunctionNetworkId | undefined;

export type XcmV4JunctionNetworkId = Enum<
  | { type: "ByGenesis"; value: Anonymize<Binary> }
  | { type: "ByFork"; value: Anonymize<I83hg7ig5d74ok> }
  | { type: "Polkadot"; value: undefined }
  | { type: "Kusama"; value: undefined }
  | { type: "Westend"; value: undefined }
  | { type: "Rococo"; value: undefined }
  | { type: "Wococo"; value: undefined }
  | { type: "Ethereum"; value: Anonymize<I623eo8t3jrbeo> }
  | { type: "BitcoinCore"; value: undefined }
  | { type: "BitcoinCash"; value: undefined }
  | { type: "PolkadotBulletin"; value: undefined }
>;
export const XcmV4JunctionNetworkId = _Enum as unknown as GetEnum<XcmV4JunctionNetworkId>;

type I83hg7ig5d74ok = { block_number: bigint; block_hash: Binary };

type I623eo8t3jrbeo = { chain_id: bigint };

type Idrke3qhmim88u = { network: Anonymize<I41adbd3kv9dad>; index: bigint };

type I3liki1s5lgett = { network: Anonymize<I41adbd3kv9dad>; key: Binary };

type Ic1rqnlu0a9i3k = { length: number; data: Binary };

type Ibb5u0oo9gtas = { id: XcmV3JunctionBodyId; part: XcmV3JunctionBodyPart };

export type XcmV3JunctionBodyId = Enum<
  | { type: "Unit"; value: undefined }
  | { type: "Moniker"; value: Anonymize<Binary> }
  | { type: "Index"; value: Anonymize<number> }
  | { type: "Executive"; value: undefined }
  | { type: "Technical"; value: undefined }
  | { type: "Legislative"; value: undefined }
  | { type: "Judicial"; value: undefined }
  | { type: "Defense"; value: undefined }
  | { type: "Administration"; value: undefined }
  | { type: "Treasury"; value: undefined }
>;
export const XcmV3JunctionBodyId = _Enum as unknown as GetEnum<XcmV3JunctionBodyId>;

export type XcmV3JunctionBodyPart = Enum<
  | { type: "Voice"; value: undefined }
  | { type: "Members"; value: Anonymize<Iafscmv8tjf0ou> }
  | { type: "Fraction"; value: Anonymize<Idif02efq16j92> }
  | { type: "AtLeastProportion"; value: Anonymize<Idif02efq16j92> }
  | { type: "MoreThanProportion"; value: Anonymize<Idif02efq16j92> }
>;
export const XcmV3JunctionBodyPart = _Enum as unknown as GetEnum<XcmV3JunctionBodyPart>;

type Iafscmv8tjf0ou = { count: number };

type Idif02efq16j92 = { nom: number; denom: number };

type I42l4nthiehb7 = [XcmV4Junction, XcmV4Junction];

type I2jk9pdm4ajs0n = [XcmV4Junction, XcmV4Junction, XcmV4Junction];

type I293rauivpnv4n = [XcmV4Junction, XcmV4Junction, XcmV4Junction, XcmV4Junction];

type Id42rc2s9m61aa = [XcmV4Junction, XcmV4Junction, XcmV4Junction, XcmV4Junction, XcmV4Junction];

type Ibe9k3j6og3ch4 = [
  XcmV4Junction,
  XcmV4Junction,
  XcmV4Junction,
  XcmV4Junction,
  XcmV4Junction,
  XcmV4Junction,
];

type I3vkvorkiqho0h = [
  XcmV4Junction,
  XcmV4Junction,
  XcmV4Junction,
  XcmV4Junction,
  XcmV4Junction,
  XcmV4Junction,
  XcmV4Junction,
];

type Icmb7nn8ip4qrt = [
  XcmV4Junction,
  XcmV4Junction,
  XcmV4Junction,
  XcmV4Junction,
  XcmV4Junction,
  XcmV4Junction,
  XcmV4Junction,
  XcmV4Junction,
];

export type XcmV3MultiassetAssetId = Enum<
  | { type: "Concrete"; value: Anonymize<Ie897ubj3a1vaq> }
  | { type: "Abstract"; value: Anonymize<Binary> }
>;
export const XcmV3MultiassetAssetId = _Enum as unknown as GetEnum<XcmV3MultiassetAssetId>;

type I845t8avig771m = { location: Anonymize<Ieq428hdmibf6m>; asset_id: Anonymize<Ieq428hdmibf6m> };

type Ieq428hdmibf6m = { parents: number; interior: XcmV4Junctions };

export type XcmV4Junctions = Enum<
  | { type: "Here"; value: undefined }
  | { type: "X1"; value: Anonymize<I195jkrdcofru6> }
  | { type: "X2"; value: Anonymize<I2r72qrr8ri0h9> }
  | { type: "X3"; value: Anonymize<Icv3jj12gfoetb> }
  | { type: "X4"; value: Anonymize<Iave13es0hoqgd> }
  | { type: "X5"; value: Anonymize<I2ql8i3uonf9kj> }
  | { type: "X6"; value: Anonymize<I943rdr8ugir4l> }
  | { type: "X7"; value: Anonymize<I34epuncb6gh8o> }
  | { type: "X8"; value: Anonymize<Idf411dtmcm4dm> }
>;
export const XcmV4Junctions = _Enum as unknown as GetEnum<XcmV4Junctions>;

type I195jkrdcofru6 = Array<XcmV4Junction>;

type I2r72qrr8ri0h9 = Array<XcmV4Junction>;

type Icv3jj12gfoetb = Array<XcmV4Junction>;

type Iave13es0hoqgd = Array<XcmV4Junction>;

type I2ql8i3uonf9kj = Array<XcmV4Junction>;

type I943rdr8ugir4l = Array<XcmV4Junction>;

type I34epuncb6gh8o = Array<XcmV4Junction>;

type Idf411dtmcm4dm = Array<XcmV4Junction>;

export type XcmVersionedLocation = Enum<
  | { type: "V2"; value: Anonymize<Ibki0d249v3ojt> }
  | { type: "V3"; value: Anonymize<Ie897ubj3a1vaq> }
  | { type: "V4"; value: Anonymize<Ieq428hdmibf6m> }
>;
export const XcmVersionedLocation = _Enum as unknown as GetEnum<XcmVersionedLocation>;

type Ibki0d249v3ojt = { parents: number; interior: XcmV2MultilocationJunctions };

export type XcmV2MultilocationJunctions = Enum<
  | { type: "Here"; value: undefined }
  | { type: "X1"; value: Anonymize<XcmV2Junction> }
  | { type: "X2"; value: Anonymize<I4jsker1kbjfdl> }
  | { type: "X3"; value: Anonymize<I13maq674kd1pa> }
  | { type: "X4"; value: Anonymize<Id88bctcqlqla7> }
  | { type: "X5"; value: Anonymize<I3d9nac7g0r3eq> }
  | { type: "X6"; value: Anonymize<I5q5ti9n9anvcm> }
  | { type: "X7"; value: Anonymize<I1famu3nq9knji> }
  | { type: "X8"; value: Anonymize<Idlq59tbqpri0l> }
>;
export const XcmV2MultilocationJunctions = _Enum as unknown as GetEnum<XcmV2MultilocationJunctions>;

export type XcmV2Junction = Enum<
  | { type: "Parachain"; value: Anonymize<number> }
  | { type: "AccountId32"; value: Anonymize<I92r3c354plrou> }
  | { type: "AccountIndex64"; value: Anonymize<I1i2pf35t6tqc0> }
  | { type: "AccountKey20"; value: Anonymize<I9llkpmu569f8r> }
  | { type: "PalletInstance"; value: Anonymize<number> }
  | { type: "GeneralIndex"; value: Anonymize<bigint> }
  | { type: "GeneralKey"; value: Anonymize<Binary> }
  | { type: "OnlyChild"; value: undefined }
  | { type: "Plurality"; value: Anonymize<Icud1kgafcboq0> }
>;
export const XcmV2Junction = _Enum as unknown as GetEnum<XcmV2Junction>;

type I92r3c354plrou = { network: XcmV2NetworkId; id: Binary };

export type XcmV2NetworkId = Enum<
  | { type: "Any"; value: undefined }
  | { type: "Named"; value: Anonymize<Binary> }
  | { type: "Polkadot"; value: undefined }
  | { type: "Kusama"; value: undefined }
>;
export const XcmV2NetworkId = _Enum as unknown as GetEnum<XcmV2NetworkId>;

type I1i2pf35t6tqc0 = { network: XcmV2NetworkId; index: bigint };

type I9llkpmu569f8r = { network: XcmV2NetworkId; key: Binary };

type Icud1kgafcboq0 = { id: XcmV2BodyId; part: XcmV3JunctionBodyPart };

export type XcmV2BodyId = Enum<
  | { type: "Unit"; value: undefined }
  | { type: "Named"; value: Anonymize<Binary> }
  | { type: "Index"; value: Anonymize<number> }
  | { type: "Executive"; value: undefined }
  | { type: "Technical"; value: undefined }
  | { type: "Legislative"; value: undefined }
  | { type: "Judicial"; value: undefined }
  | { type: "Defense"; value: undefined }
  | { type: "Administration"; value: undefined }
  | { type: "Treasury"; value: undefined }
>;
export const XcmV2BodyId = _Enum as unknown as GetEnum<XcmV2BodyId>;

type I4jsker1kbjfdl = [XcmV2Junction, XcmV2Junction];

type I13maq674kd1pa = [XcmV2Junction, XcmV2Junction, XcmV2Junction];

type Id88bctcqlqla7 = [XcmV2Junction, XcmV2Junction, XcmV2Junction, XcmV2Junction];

type I3d9nac7g0r3eq = [XcmV2Junction, XcmV2Junction, XcmV2Junction, XcmV2Junction, XcmV2Junction];

type I5q5ti9n9anvcm = [
  XcmV2Junction,
  XcmV2Junction,
  XcmV2Junction,
  XcmV2Junction,
  XcmV2Junction,
  XcmV2Junction,
];

type I1famu3nq9knji = [
  XcmV2Junction,
  XcmV2Junction,
  XcmV2Junction,
  XcmV2Junction,
  XcmV2Junction,
  XcmV2Junction,
  XcmV2Junction,
];

type Idlq59tbqpri0l = [
  XcmV2Junction,
  XcmV2Junction,
  XcmV2Junction,
  XcmV2Junction,
  XcmV2Junction,
  XcmV2Junction,
  XcmV2Junction,
  XcmV2Junction,
];

type Iek7v4hrgnq6iv = { index: number; payment_id: bigint };

export type ConvictionVotingEvent = Enum<
  | { type: "Delegated"; value: Anonymize<Ic5oktqtdlvdvq> }
  | { type: "Undelegated"; value: Anonymize<SS58String> }
>;
export const ConvictionVotingEvent = _Enum as unknown as GetEnum<ConvictionVotingEvent>;

type Ic5oktqtdlvdvq = [SS58String, SS58String];

export type ReferendaEvent = Enum<
  | { type: "Submitted"; value: Anonymize<Idhr9v8mlnjej> }
  | { type: "DecisionDepositPlaced"; value: Anonymize<I62nte77gksm0f> }
  | { type: "DecisionDepositRefunded"; value: Anonymize<I62nte77gksm0f> }
  | { type: "DepositSlashed"; value: Anonymize<Id5fm4p8lj5qgi> }
  | { type: "DecisionStarted"; value: Anonymize<I932allgc83a4a> }
  | { type: "ConfirmStarted"; value: Anonymize<I666bl2fqjkejo> }
  | { type: "ConfirmAborted"; value: Anonymize<I666bl2fqjkejo> }
  | { type: "Confirmed"; value: Anonymize<Ilhp45uime5tp> }
  | { type: "Approved"; value: Anonymize<I666bl2fqjkejo> }
  | { type: "Rejected"; value: Anonymize<Ilhp45uime5tp> }
  | { type: "TimedOut"; value: Anonymize<Ilhp45uime5tp> }
  | { type: "Cancelled"; value: Anonymize<Ilhp45uime5tp> }
  | { type: "Killed"; value: Anonymize<Ilhp45uime5tp> }
  | { type: "SubmissionDepositRefunded"; value: Anonymize<I62nte77gksm0f> }
  | { type: "MetadataSet"; value: Anonymize<I50aq0q2l1cdkr> }
  | { type: "MetadataCleared"; value: Anonymize<I50aq0q2l1cdkr> }
>;
export const ReferendaEvent = _Enum as unknown as GetEnum<ReferendaEvent>;

type Idhr9v8mlnjej = { index: number; track: number; proposal: PreimagesBounded };

export type PreimagesBounded = Enum<
  | { type: "Legacy"; value: Anonymize<Id9d48vaes3c53> }
  | { type: "Inline"; value: Anonymize<Binary> }
  | { type: "Lookup"; value: Anonymize<Ie4qb7tq0r9uel> }
>;
export const PreimagesBounded = _Enum as unknown as GetEnum<PreimagesBounded>;

type Id9d48vaes3c53 = { hash: Binary };

type Ie4qb7tq0r9uel = { hash: Binary; len: number };

type I62nte77gksm0f = { index: number; who: SS58String; amount: bigint };

type I932allgc83a4a = {
  index: number;
  track: number;
  proposal: PreimagesBounded;
  tally: Anonymize<Ifsk7cbmtit1jd>;
};

type Ifsk7cbmtit1jd = { ayes: bigint; nays: bigint; support: bigint };

type Ilhp45uime5tp = { index: number; tally: Anonymize<Ifsk7cbmtit1jd> };

type I50aq0q2l1cdkr = { index: number; hash: Binary };

type I8s9ti4a44j0bp = AnonymousEnum<{
  MemberAdded: Anonymize<I4cbvqmqadhrea>;
  RankChanged: Anonymize<Im1pm2vf6llcn>;
  MemberRemoved: Anonymize<Im1pm2vf6llcn>;
  Voted: Anonymize<I31vg1u4v55v90>;
  MemberExchanged: Anonymize<Ier6ck0tpfo7>;
}>;

type Im1pm2vf6llcn = { who: SS58String; rank: number };

type I31vg1u4v55v90 = {
  who: SS58String;
  poll: number;
  vote: Anonymize<I996080trg6urn>;
  tally: Anonymize<I6d634btv7misu>;
};

type I996080trg6urn = AnonymousEnum<{ Aye: Anonymize<number>; Nay: Anonymize<number> }>;

type I6d634btv7misu = { bare_ayes: number; ayes: number; nays: number };

type Ier6ck0tpfo7 = { who: SS58String; new_who: SS58String };

type Idun3t58do26lr = AnonymousEnum<{
  Submitted: Anonymize<Idhr9v8mlnjej>;
  DecisionDepositPlaced: Anonymize<I62nte77gksm0f>;
  DecisionDepositRefunded: Anonymize<I62nte77gksm0f>;
  DepositSlashed: Anonymize<Id5fm4p8lj5qgi>;
  DecisionStarted: Anonymize<Iuu6tledtt64n>;
  ConfirmStarted: Anonymize<I666bl2fqjkejo>;
  ConfirmAborted: Anonymize<I666bl2fqjkejo>;
  Confirmed: Anonymize<I27notaksll8qt>;
  Approved: Anonymize<I666bl2fqjkejo>;
  Rejected: Anonymize<I27notaksll8qt>;
  TimedOut: Anonymize<I27notaksll8qt>;
  Cancelled: Anonymize<I27notaksll8qt>;
  Killed: Anonymize<I27notaksll8qt>;
  SubmissionDepositRefunded: Anonymize<I62nte77gksm0f>;
  MetadataSet: Anonymize<I50aq0q2l1cdkr>;
  MetadataCleared: Anonymize<I50aq0q2l1cdkr>;
}>;

type Iuu6tledtt64n = {
  index: number;
  track: number;
  proposal: PreimagesBounded;
  tally: Anonymize<I6d634btv7misu>;
};

type I27notaksll8qt = { index: number; tally: Anonymize<I6d634btv7misu> };

export type WhitelistEvent = Enum<
  | { type: "CallWhitelisted"; value: Anonymize<I8413rb6im3iko> }
  | { type: "WhitelistedCallRemoved"; value: Anonymize<I8413rb6im3iko> }
  | { type: "WhitelistedCallDispatched"; value: Anonymize<I7b8pe56shlide> }
>;
export const WhitelistEvent = _Enum as unknown as GetEnum<WhitelistEvent>;

type I8413rb6im3iko = { call_hash: Binary };

type I7b8pe56shlide = { call_hash: Binary; result: Anonymize<Idurpak8qagatr> };

type Idurpak8qagatr = ResultPayload<Anonymize<I42817lesmslbu>, Anonymize<I9jhevvtlg11ei>>;

type I42817lesmslbu = { actual_weight: Anonymize<Iasb8k6ash5mjn>; pays_fee: DispatchPays };

type Iasb8k6ash5mjn = Anonymize<I4q39t5hn830vp> | undefined;

type I9jhevvtlg11ei = { post_info: Anonymize<I42817lesmslbu>; error: DispatchError };

export type CommonClaimsEvent = Enum<{ type: "Claimed"; value: Anonymize<Idhjiuhlaei3db> }>;
export const CommonClaimsEvent = _Enum as unknown as GetEnum<CommonClaimsEvent>;

type Idhjiuhlaei3db = { who: SS58String; ethereum_address: Binary; amount: bigint };

export type UtilityEvent = Enum<
  | { type: "BatchInterrupted"; value: Anonymize<I6tn8e5lqr339o> }
  | { type: "BatchCompleted"; value: undefined }
  | { type: "BatchCompletedWithErrors"; value: undefined }
  | { type: "ItemCompleted"; value: undefined }
  | { type: "ItemFailed"; value: Anonymize<I11lb9o37qkk4f> }
  | { type: "DispatchedAs"; value: Anonymize<Ie5i8qqljk3tjb> }
>;
export const UtilityEvent = _Enum as unknown as GetEnum<UtilityEvent>;

type I6tn8e5lqr339o = { index: number; error: DispatchError };

type I11lb9o37qkk4f = { error: DispatchError };

type Ie5i8qqljk3tjb = { result: Anonymize<Idtdr91jmq5g4i> };

type Idtdr91jmq5g4i = ResultPayload<undefined, DispatchError>;

export type IdentityEvent = Enum<
  | { type: "IdentitySet"; value: Anonymize<I4cbvqmqadhrea> }
  | { type: "IdentityCleared"; value: Anonymize<Iep1lmt6q3s6r3> }
  | { type: "IdentityKilled"; value: Anonymize<Iep1lmt6q3s6r3> }
  | { type: "JudgementRequested"; value: Anonymize<I1fac16213rie2> }
  | { type: "JudgementUnrequested"; value: Anonymize<I1fac16213rie2> }
  | { type: "JudgementGiven"; value: Anonymize<Ifjt77oc391o43> }
  | { type: "RegistrarAdded"; value: Anonymize<Itvt1jsipv0lc> }
  | { type: "SubIdentityAdded"; value: Anonymize<Ick3mveut33f44> }
  | { type: "SubIdentityRemoved"; value: Anonymize<Ick3mveut33f44> }
  | { type: "SubIdentityRevoked"; value: Anonymize<Ick3mveut33f44> }
  | { type: "AuthorityAdded"; value: Anonymize<I2rg5btjrsqec0> }
  | { type: "AuthorityRemoved"; value: Anonymize<I2rg5btjrsqec0> }
  | { type: "UsernameSet"; value: Anonymize<Ibdqerrooruuq9> }
  | { type: "UsernameQueued"; value: Anonymize<I8u2ba9jeiu6q0> }
  | { type: "PreapprovalExpired"; value: Anonymize<I7ieadb293k6b4> }
  | { type: "PrimaryUsernameSet"; value: Anonymize<Ibdqerrooruuq9> }
  | { type: "DanglingUsernameRemoved"; value: Anonymize<Ibdqerrooruuq9> }
>;
export const IdentityEvent = _Enum as unknown as GetEnum<IdentityEvent>;

type Iep1lmt6q3s6r3 = { who: SS58String; deposit: bigint };

type I1fac16213rie2 = { who: SS58String; registrar_index: number };

type Ifjt77oc391o43 = { target: SS58String; registrar_index: number };

type Itvt1jsipv0lc = { registrar_index: number };

type Ick3mveut33f44 = { sub: SS58String; main: SS58String; deposit: bigint };

type I2rg5btjrsqec0 = { authority: SS58String };

type Ibdqerrooruuq9 = { who: SS58String; username: Binary };

type I8u2ba9jeiu6q0 = { who: SS58String; username: Binary; expiration: number };

type I7ieadb293k6b4 = { whose: SS58String };

type Ifhcq78pr9md4e = AnonymousEnum<{
  Founded: Anonymize<I8ircspqpe79k7>;
  Bid: Anonymize<I1vlt8l6nvnen6>;
  Vouch: Anonymize<I72vf0j9juc1df>;
  AutoUnbid: Anonymize<I4b66js88p45m8>;
  Unbid: Anonymize<I4b66js88p45m8>;
  Unvouch: Anonymize<I4b66js88p45m8>;
  Inducted: Anonymize<Ie0phfcvllr4od>;
  SuspendedMemberJudgement: Anonymize<I476ep66p6ddg1>;
  CandidateSuspended: Anonymize<I4b66js88p45m8>;
  MemberSuspended: Anonymize<Ie3gphha4ejh40>;
  Challenged: Anonymize<Ie3gphha4ejh40>;
  Vote: Anonymize<If0btdsid69rrm>;
  DefenderVote: Anonymize<Ictusacdcqj61u>;
  NewParams: Anonymize<Ibsolu3c3q1lb4>;
  Unfounded: Anonymize<I8ircspqpe79k7>;
  Deposit: Anonymize<Ie5v6njpckr05b>;
  Elevated: Anonymize<Ib116djgo9u4uo>;
}>;

type I8ircspqpe79k7 = { founder: SS58String };

type I1vlt8l6nvnen6 = { candidate_id: SS58String; offer: bigint };

type I72vf0j9juc1df = { candidate_id: SS58String; offer: bigint; vouching: SS58String };

type I4b66js88p45m8 = { candidate: SS58String };

type Ie0phfcvllr4od = { primary: SS58String; candidates: Anonymize<Ia2lhg7l2hilo3> };

type Ia2lhg7l2hilo3 = Array<SS58String>;

type I476ep66p6ddg1 = { who: SS58String; judged: boolean };

type Ie3gphha4ejh40 = { member: SS58String };

type If0btdsid69rrm = { candidate: SS58String; voter: SS58String; vote: boolean };

type Ictusacdcqj61u = { voter: SS58String; vote: boolean };

type Ibsolu3c3q1lb4 = { params: Anonymize<I9hkvi4k3kh916> };

type I9hkvi4k3kh916 = {
  max_members: number;
  max_intake: number;
  max_strikes: number;
  candidate_deposit: bigint;
};

type Ib116djgo9u4uo = { member: SS58String; rank: number };

export type RecoveryEvent = Enum<
  | { type: "RecoveryCreated"; value: Anonymize<Icbccs0ug47ilf> }
  | { type: "RecoveryInitiated"; value: Anonymize<I9vkkue6cq74et> }
  | { type: "RecoveryVouched"; value: Anonymize<Ibu56t5h1q49i4> }
  | { type: "RecoveryClosed"; value: Anonymize<I9vkkue6cq74et> }
  | { type: "AccountRecovered"; value: Anonymize<I9vkkue6cq74et> }
  | { type: "RecoveryRemoved"; value: Anonymize<I1dmtl5t34b9g> }
>;
export const RecoveryEvent = _Enum as unknown as GetEnum<RecoveryEvent>;

type I9vkkue6cq74et = { lost_account: SS58String; rescuer_account: SS58String };

type Ibu56t5h1q49i4 = { lost_account: SS58String; rescuer_account: SS58String; sender: SS58String };

type I1dmtl5t34b9g = { lost_account: SS58String };

export type VestingEvent = Enum<
  | { type: "VestingUpdated"; value: Anonymize<Ievr89968437gm> }
  | { type: "VestingCompleted"; value: Anonymize<Icbccs0ug47ilf> }
>;
export const VestingEvent = _Enum as unknown as GetEnum<VestingEvent>;

type Ievr89968437gm = { account: SS58String; unvested: bigint };

export type SchedulerEvent = Enum<
  | { type: "Scheduled"; value: Anonymize<I5n4sebgkfr760> }
  | { type: "Canceled"; value: Anonymize<I5n4sebgkfr760> }
  | { type: "Dispatched"; value: Anonymize<Idv8erd9m7jvse> }
  | { type: "RetrySet"; value: Anonymize<I1fj2d8s3noh0v> }
  | { type: "RetryCancelled"; value: Anonymize<Ibkv7dijodoblp> }
  | { type: "CallUnavailable"; value: Anonymize<Ibkv7dijodoblp> }
  | { type: "PeriodicFailed"; value: Anonymize<Ibkv7dijodoblp> }
  | { type: "RetryFailed"; value: Anonymize<Ibkv7dijodoblp> }
  | { type: "PermanentlyOverweight"; value: Anonymize<Ibkv7dijodoblp> }
>;
export const SchedulerEvent = _Enum as unknown as GetEnum<SchedulerEvent>;

type I5n4sebgkfr760 = { when: number; index: number };

type Idv8erd9m7jvse = {
  task: Anonymize<I5g2vv0ckl2m8b>;
  id: Anonymize<I17k3ujudqd5df>;
  result: Anonymize<Idtdr91jmq5g4i>;
};

type I5g2vv0ckl2m8b = [number, number];

type I17k3ujudqd5df = Binary | undefined;

type I1fj2d8s3noh0v = {
  task: Anonymize<I5g2vv0ckl2m8b>;
  id: Anonymize<I17k3ujudqd5df>;
  period: number;
  retries: number;
};

type Ibkv7dijodoblp = { task: Anonymize<I5g2vv0ckl2m8b>; id: Anonymize<I17k3ujudqd5df> };

type I61uqajj8pqisn = AnonymousEnum<{
  ProxyExecuted: Anonymize<Ie5i8qqljk3tjb>;
  PureCreated: Anonymize<I2bf4tmdcees31>;
  Announced: Anonymize<Idbjbboh0q507r>;
  ProxyAdded: Anonymize<I3ce0ol160n343>;
  ProxyRemoved: Anonymize<I3ce0ol160n343>;
}>;

type I2bf4tmdcees31 = {
  pure: SS58String;
  who: SS58String;
  proxy_type: Anonymize<I90b4nim1kup1v>;
  disambiguation_index: number;
};

type I90b4nim1kup1v = AnonymousEnum<{
  Any: undefined;
  NonTransfer: undefined;
  Governance: undefined;
  IdentityJudgement: undefined;
  CancelProxy: undefined;
  Auction: undefined;
  Society: undefined;
  OnDemandOrdering: undefined;
}>;

type Idbjbboh0q507r = { real: SS58String; proxy: SS58String; call_hash: Binary };

type I3ce0ol160n343 = {
  delegator: SS58String;
  delegatee: SS58String;
  proxy_type: Anonymize<I90b4nim1kup1v>;
  delay: number;
};

export type MultisigEvent = Enum<
  | { type: "NewMultisig"; value: Anonymize<Ibvv58de7m7rsi> }
  | { type: "MultisigApproval"; value: Anonymize<I4uo2dg1jvbdtg> }
  | { type: "MultisigExecuted"; value: Anonymize<Ifbo6gts4g8u33> }
  | { type: "MultisigCancelled"; value: Anonymize<I82jp3a00f0f8k> }
>;
export const MultisigEvent = _Enum as unknown as GetEnum<MultisigEvent>;

type Ibvv58de7m7rsi = { approving: SS58String; multisig: SS58String; call_hash: Binary };

type I4uo2dg1jvbdtg = {
  approving: SS58String;
  timepoint: Anonymize<Itvprrpb0nm3o>;
  multisig: SS58String;
  call_hash: Binary;
};

type Itvprrpb0nm3o = { height: number; index: number };

type Ifbo6gts4g8u33 = {
  approving: SS58String;
  timepoint: Anonymize<Itvprrpb0nm3o>;
  multisig: SS58String;
  call_hash: Binary;
  result: Anonymize<Idtdr91jmq5g4i>;
};

type I82jp3a00f0f8k = {
  cancelling: SS58String;
  timepoint: Anonymize<Itvprrpb0nm3o>;
  multisig: SS58String;
  call_hash: Binary;
};

export type PreimageEvent = Enum<
  | { type: "Noted"; value: Anonymize<Id9d48vaes3c53> }
  | { type: "Requested"; value: Anonymize<Id9d48vaes3c53> }
  | { type: "Cleared"; value: Anonymize<Id9d48vaes3c53> }
>;
export const PreimageEvent = _Enum as unknown as GetEnum<PreimageEvent>;

export type AssetRateEvent = Enum<
  | { type: "AssetRateCreated"; value: Anonymize<I4digl36cac7si> }
  | { type: "AssetRateRemoved"; value: Anonymize<Idi8jsg8hkprub> }
  | { type: "AssetRateUpdated"; value: Anonymize<Iasmn1g877fblj> }
>;
export const AssetRateEvent = _Enum as unknown as GetEnum<AssetRateEvent>;

type I4digl36cac7si = {
  asset_kind: PolkadotRuntimeCommonImplsVersionedLocatableAsset;
  rate: bigint;
};

type Idi8jsg8hkprub = { asset_kind: PolkadotRuntimeCommonImplsVersionedLocatableAsset };

type Iasmn1g877fblj = {
  asset_kind: PolkadotRuntimeCommonImplsVersionedLocatableAsset;
  old: bigint;
  new: bigint;
};

type Iei6k1tdcht3q = AnonymousEnum<{
  BountyProposed: Anonymize<I666bl2fqjkejo>;
  BountyRejected: Anonymize<Id9idaj83175f9>;
  BountyBecameActive: Anonymize<I666bl2fqjkejo>;
  BountyAwarded: Anonymize<Ie1semicfuv5uu>;
  BountyClaimed: Anonymize<If25fjs9o37co1>;
  BountyCanceled: Anonymize<I666bl2fqjkejo>;
  BountyExtended: Anonymize<I666bl2fqjkejo>;
  BountyApproved: Anonymize<I666bl2fqjkejo>;
  CuratorProposed: Anonymize<I70sc1pdo8vtos>;
  CuratorUnassigned: Anonymize<Ia9p5bg6p18r0i>;
  CuratorAccepted: Anonymize<I70sc1pdo8vtos>;
}>;

type Id9idaj83175f9 = { index: number; bond: bigint };

type Ie1semicfuv5uu = { index: number; beneficiary: SS58String };

type If25fjs9o37co1 = { index: number; payout: bigint; beneficiary: SS58String };

type I70sc1pdo8vtos = { bounty_id: number; curator: SS58String };

type Ia9p5bg6p18r0i = { bounty_id: number };

export type ChildBountiesEvent = Enum<
  | { type: "Added"; value: Anonymize<I60p8l86a8cm59> }
  | { type: "Awarded"; value: Anonymize<I3m3sk2lgcabvp> }
  | { type: "Claimed"; value: Anonymize<I5pf572duh4oeg> }
  | { type: "Canceled"; value: Anonymize<I60p8l86a8cm59> }
>;
export const ChildBountiesEvent = _Enum as unknown as GetEnum<ChildBountiesEvent>;

type I60p8l86a8cm59 = { index: number; child_index: number };

type I3m3sk2lgcabvp = { index: number; child_index: number; beneficiary: SS58String };

type I5pf572duh4oeg = {
  index: number;
  child_index: number;
  payout: bigint;
  beneficiary: SS58String;
};

type I9dmlhfape78ke = AnonymousEnum<{
  BidPlaced: Anonymize<Ie2vmgag6qiue4>;
  BidRetracted: Anonymize<Ie2vmgag6qiue4>;
  BidDropped: Anonymize<Ie2vmgag6qiue4>;
  Issued: Anonymize<I1kpuet4lkcdpf>;
  Thawed: Anonymize<I5tr1gp2t9i6oh>;
  Funded: Anonymize<I7ls1r4u61csif>;
  Transferred: Anonymize<I3gjgtso69eopr>;
}>;

type Ie2vmgag6qiue4 = { who: SS58String; amount: bigint; duration: number };

type I1kpuet4lkcdpf = {
  index: number;
  expiry: number;
  who: SS58String;
  proportion: bigint;
  amount: bigint;
};

type I5tr1gp2t9i6oh = {
  index: number;
  who: SS58String;
  proportion: bigint;
  amount: bigint;
  dropped: boolean;
};

type I7ls1r4u61csif = { deficit: bigint };

type I3gjgtso69eopr = { from: SS58String; to: SS58String; index: number };

export type ParachainsInclusionEvent = Enum<
  | { type: "CandidateBacked"; value: Anonymize<Ieno5vn1m65ng2> }
  | { type: "CandidateIncluded"; value: Anonymize<Ieno5vn1m65ng2> }
  | { type: "CandidateTimedOut"; value: Anonymize<Ievbvtucck5gnq> }
  | { type: "UpwardMessagesReceived"; value: Anonymize<Ic8i89mfkmn3n7> }
>;
export const ParachainsInclusionEvent = _Enum as unknown as GetEnum<ParachainsInclusionEvent>;

type Ieno5vn1m65ng2 = [Anonymize<I4vjld3472quct>, Binary, number, number];

type I4vjld3472quct = { descriptor: Anonymize<Ib2u20s6roco9i>; commitments_hash: Binary };

type Ib2u20s6roco9i = {
  para_id: number;
  relay_parent: Binary;
  collator: Binary;
  persisted_validation_data_hash: Binary;
  pov_hash: Binary;
  erasure_root: Binary;
  signature: Binary;
  para_head: Binary;
  validation_code_hash: Binary;
};

type Ievbvtucck5gnq = [Anonymize<I4vjld3472quct>, Binary, number];

type Ic8i89mfkmn3n7 = { from: number; count: number };

export type ParachainsParasEvent = Enum<
  | { type: "CurrentCodeUpdated"; value: Anonymize<number> }
  | { type: "CurrentHeadUpdated"; value: Anonymize<number> }
  | { type: "CodeUpgradeScheduled"; value: Anonymize<number> }
  | { type: "NewHeadNoted"; value: Anonymize<number> }
  | { type: "ActionQueued"; value: Anonymize<I5g2vv0ckl2m8b> }
  | { type: "PvfCheckStarted"; value: Anonymize<I64gm4hrq7urum> }
  | { type: "PvfCheckAccepted"; value: Anonymize<I64gm4hrq7urum> }
  | { type: "PvfCheckRejected"; value: Anonymize<I64gm4hrq7urum> }
>;
export const ParachainsParasEvent = _Enum as unknown as GetEnum<ParachainsParasEvent>;

type I64gm4hrq7urum = [Binary, number];

export type PolkadotRuntimeParachainsHrmpEvent = Enum<
  | { type: "OpenChannelRequested"; value: Anonymize<Id2bej717ckub0> }
  | { type: "OpenChannelCanceled"; value: Anonymize<I545vo2e86o5i4> }
  | { type: "OpenChannelAccepted"; value: Anonymize<I50mrcbubp554e> }
  | { type: "ChannelClosed"; value: Anonymize<I545vo2e86o5i4> }
  | { type: "HrmpChannelForceOpened"; value: Anonymize<Id2bej717ckub0> }
  | { type: "HrmpSystemChannelOpened"; value: Anonymize<Id2bej717ckub0> }
  | { type: "OpenChannelDepositsUpdated"; value: Anonymize<I50mrcbubp554e> }
>;
export const PolkadotRuntimeParachainsHrmpEvent =
  _Enum as unknown as GetEnum<PolkadotRuntimeParachainsHrmpEvent>;

type Id2bej717ckub0 = {
  sender: number;
  recipient: number;
  proposed_max_capacity: number;
  proposed_max_message_size: number;
};

type I545vo2e86o5i4 = { by_parachain: number; channel_id: Anonymize<I50mrcbubp554e> };

type I50mrcbubp554e = { sender: number; recipient: number };

export type ParachainsDisputesEvent = Enum<
  | { type: "DisputeInitiated"; value: Anonymize<I3o099fcusuh31> }
  | { type: "DisputeConcluded"; value: Anonymize<Ifr2e7vm3bun8k> }
  | { type: "Revert"; value: Anonymize<number> }
>;
export const ParachainsDisputesEvent = _Enum as unknown as GetEnum<ParachainsDisputesEvent>;

type I3o099fcusuh31 = [Binary, ParachainsDisputesDisputeLocation];

export type ParachainsDisputesDisputeLocation = Enum<
  { type: "Local"; value: undefined } | { type: "Remote"; value: undefined }
>;
export const ParachainsDisputesDisputeLocation =
  _Enum as unknown as GetEnum<ParachainsDisputesDisputeLocation>;

type Ifr2e7vm3bun8k = [Binary, ParachainsDisputesDisputeResult];

export type ParachainsDisputesDisputeResult = Enum<
  { type: "Valid"; value: undefined } | { type: "Invalid"; value: undefined }
>;
export const ParachainsDisputesDisputeResult =
  _Enum as unknown as GetEnum<ParachainsDisputesDisputeResult>;

export type MessageQueueEvent = Enum<
  | { type: "ProcessingFailed"; value: Anonymize<I6ian27okrbc15> }
  | { type: "Processed"; value: Anonymize<I74b8cu68dfbfr> }
  | { type: "OverweightEnqueued"; value: Anonymize<Iacc6dee8ffsrh> }
  | { type: "PageReaped"; value: Anonymize<Ielsom2b1kkdm6> }
>;
export const MessageQueueEvent = _Enum as unknown as GetEnum<MessageQueueEvent>;

type I6ian27okrbc15 = {
  id: Binary;
  origin: ParachainsInclusionAggregateMessageOrigin;
  error: ProcessMessageError;
};

export type ParachainsInclusionAggregateMessageOrigin = Enum<{
  type: "Ump";
  value: Anonymize<ParachainsInclusionUmpQueueId>;
}>;
export const ParachainsInclusionAggregateMessageOrigin =
  _Enum as unknown as GetEnum<ParachainsInclusionAggregateMessageOrigin>;

export type ParachainsInclusionUmpQueueId = Enum<{ type: "Para"; value: Anonymize<number> }>;
export const ParachainsInclusionUmpQueueId =
  _Enum as unknown as GetEnum<ParachainsInclusionUmpQueueId>;

export type ProcessMessageError = Enum<
  | { type: "BadFormat"; value: undefined }
  | { type: "Corrupt"; value: undefined }
  | { type: "Unsupported"; value: undefined }
  | { type: "Overweight"; value: Anonymize<I4q39t5hn830vp> }
  | { type: "Yield"; value: undefined }
>;
export const ProcessMessageError = _Enum as unknown as GetEnum<ProcessMessageError>;

type I74b8cu68dfbfr = {
  id: Binary;
  origin: ParachainsInclusionAggregateMessageOrigin;
  weight_used: Anonymize<I4q39t5hn830vp>;
  success: boolean;
};

type Iacc6dee8ffsrh = {
  id: Binary;
  origin: ParachainsInclusionAggregateMessageOrigin;
  page_index: number;
  message_index: number;
};

type Ielsom2b1kkdm6 = { origin: ParachainsInclusionAggregateMessageOrigin; index: number };

export type PolkadotRuntimeParachainsAssignerOnDemandEvent = Enum<
  | { type: "OnDemandOrderPlaced"; value: Anonymize<I89dbspj67f63r> }
  | { type: "SpotTrafficSet"; value: Anonymize<Ia7efn5eqruldn> }
>;
export const PolkadotRuntimeParachainsAssignerOnDemandEvent =
  _Enum as unknown as GetEnum<PolkadotRuntimeParachainsAssignerOnDemandEvent>;

type I89dbspj67f63r = { para_id: number; spot_price: bigint };

type Ia7efn5eqruldn = { traffic: bigint };

export type CommonParasRegistrarEvent = Enum<
  | { type: "Registered"; value: Anonymize<Ibs22tt76qp5bi> }
  | { type: "Deregistered"; value: Anonymize<I37r4bdai8o9mp> }
  | { type: "Reserved"; value: Anonymize<Idn2ghub1o4i40> }
  | { type: "Swapped"; value: Anonymize<I48u78djt89dod> }
>;
export const CommonParasRegistrarEvent = _Enum as unknown as GetEnum<CommonParasRegistrarEvent>;

type Ibs22tt76qp5bi = { para_id: number; manager: SS58String };

type I37r4bdai8o9mp = { para_id: number };

type Idn2ghub1o4i40 = { para_id: number; who: SS58String };

type I48u78djt89dod = { para_id: number; other_id: number };

export type CommonSlotsEvent = Enum<
  | { type: "NewLeasePeriod"; value: Anonymize<Ib85m5kfbepu2t> }
  | { type: "Leased"; value: Anonymize<Idaml5bdhsfcsl> }
>;
export const CommonSlotsEvent = _Enum as unknown as GetEnum<CommonSlotsEvent>;

type Ib85m5kfbepu2t = { lease_period: number };

type Idaml5bdhsfcsl = {
  para_id: number;
  leaser: SS58String;
  period_begin: number;
  period_count: number;
  extra_reserved: bigint;
  total_amount: bigint;
};

export type CommonAuctionsEvent = Enum<
  | { type: "AuctionStarted"; value: Anonymize<Ieec0cu336gteb> }
  | { type: "AuctionClosed"; value: Anonymize<I815d5k4ij85nv> }
  | { type: "Reserved"; value: Anonymize<Ifi98fgi9o46v7> }
  | { type: "Unreserved"; value: Anonymize<Ic0oj9tok33uap> }
  | { type: "ReserveConfiscated"; value: Anonymize<I3tdutpfjuk32j> }
  | { type: "BidAccepted"; value: Anonymize<I1esdujrkdacpb> }
  | { type: "WinningOffset"; value: Anonymize<I9g1d820jf9m2s> }
>;
export const CommonAuctionsEvent = _Enum as unknown as GetEnum<CommonAuctionsEvent>;

type Ieec0cu336gteb = { auction_index: number; lease_period: number; ending: number };

type I815d5k4ij85nv = { auction_index: number };

type Ifi98fgi9o46v7 = { bidder: SS58String; extra_reserved: bigint; total_amount: bigint };

type Ic0oj9tok33uap = { bidder: SS58String; amount: bigint };

type I3tdutpfjuk32j = { para_id: number; leaser: SS58String; amount: bigint };

type I1esdujrkdacpb = {
  bidder: SS58String;
  para_id: number;
  amount: bigint;
  first_slot: number;
  last_slot: number;
};

type I9g1d820jf9m2s = { auction_index: number; block_number: number };

export type CommonCrowdloanEvent = Enum<
  | { type: "Created"; value: Anonymize<I37r4bdai8o9mp> }
  | { type: "Contributed"; value: Anonymize<I8ve4g3egaln6a> }
  | { type: "Withdrew"; value: Anonymize<I8ve4g3egaln6a> }
  | { type: "PartiallyRefunded"; value: Anonymize<I37r4bdai8o9mp> }
  | { type: "AllRefunded"; value: Anonymize<I37r4bdai8o9mp> }
  | { type: "Dissolved"; value: Anonymize<I37r4bdai8o9mp> }
  | { type: "HandleBidResult"; value: Anonymize<If9e3ujpsfl4g7> }
  | { type: "Edited"; value: Anonymize<I37r4bdai8o9mp> }
  | { type: "MemoUpdated"; value: Anonymize<If4hvqaeoqq5us> }
  | { type: "AddedToNewRaise"; value: Anonymize<I37r4bdai8o9mp> }
>;
export const CommonCrowdloanEvent = _Enum as unknown as GetEnum<CommonCrowdloanEvent>;

type I8ve4g3egaln6a = { who: SS58String; fund_index: number; amount: bigint };

type If9e3ujpsfl4g7 = { para_id: number; result: Anonymize<Idtdr91jmq5g4i> };

type If4hvqaeoqq5us = { who: SS58String; para_id: number; memo: Binary };

export type PolkadotRuntimeParachainsCoretimeEvent = Enum<
  | { type: "RevenueInfoRequested"; value: Anonymize<Ibtsa3docbr9el> }
  | { type: "CoreAssigned"; value: Anonymize<Iaiqv5prlisjkg> }
>;
export const PolkadotRuntimeParachainsCoretimeEvent =
  _Enum as unknown as GetEnum<PolkadotRuntimeParachainsCoretimeEvent>;

type Ibtsa3docbr9el = { when: number };

type Iaiqv5prlisjkg = { core: number };

export type XcmEvent = Enum<
  | { type: "Attempted"; value: Anonymize<I39j4hmqmtbefm> }
  | { type: "Sent"; value: Anonymize<I3gb27uk4ul5am> }
  | { type: "UnexpectedResponse"; value: Anonymize<Ic8gqph940s142> }
  | { type: "ResponseReady"; value: Anonymize<I9k3jmtgmoiqqf> }
  | { type: "Notified"; value: Anonymize<I2uqmls7kcdnii> }
  | { type: "NotifyOverweight"; value: Anonymize<Idg69klialbkb8> }
  | { type: "NotifyDispatchError"; value: Anonymize<I2uqmls7kcdnii> }
  | { type: "NotifyDecodeFailed"; value: Anonymize<I2uqmls7kcdnii> }
  | { type: "InvalidResponder"; value: Anonymize<I24et57o2f2i2m> }
  | { type: "InvalidResponderVersion"; value: Anonymize<Ic8gqph940s142> }
  | { type: "ResponseTaken"; value: Anonymize<I30pg328m00nr3> }
  | { type: "AssetsTrapped"; value: Anonymize<I20naetn2lovo9> }
  | { type: "VersionChangeNotified"; value: Anonymize<I4ks28k7d44l1p> }
  | { type: "SupportedVersionChanged"; value: Anonymize<Iergfrmj3c9dor> }
  | { type: "NotifyTargetSendFail"; value: Anonymize<I9jji4pfgcqc9b> }
  | { type: "NotifyTargetMigrationFail"; value: Anonymize<I6djqr04u42c2g> }
  | { type: "InvalidQuerierVersion"; value: Anonymize<Ic8gqph940s142> }
  | { type: "InvalidQuerier"; value: Anonymize<I6ctg8b4bcjdek> }
  | { type: "VersionNotifyStarted"; value: Anonymize<I4fi6oaheho56l> }
  | { type: "VersionNotifyRequested"; value: Anonymize<I4fi6oaheho56l> }
  | { type: "VersionNotifyUnrequested"; value: Anonymize<I4fi6oaheho56l> }
  | { type: "FeesPaid"; value: Anonymize<I89pfbi9d7lm5p> }
  | { type: "AssetsClaimed"; value: Anonymize<I20naetn2lovo9> }
  | { type: "VersionMigrationFinished"; value: Anonymize<I6s1nbislhk619> }
>;
export const XcmEvent = _Enum as unknown as GetEnum<XcmEvent>;

type I39j4hmqmtbefm = { outcome: XcmV4TraitsOutcome };

export type XcmV4TraitsOutcome = Enum<
  | { type: "Complete"; value: Anonymize<I30iff2d192eu7> }
  | { type: "Incomplete"; value: Anonymize<I2fhe61h00l6nt> }
  | { type: "Error"; value: Anonymize<I5m6kclhssq59a> }
>;
export const XcmV4TraitsOutcome = _Enum as unknown as GetEnum<XcmV4TraitsOutcome>;

type I30iff2d192eu7 = { used: Anonymize<I4q39t5hn830vp> };

type I2fhe61h00l6nt = { used: Anonymize<I4q39t5hn830vp>; error: XcmV3TraitsError };

export type XcmV3TraitsError = Enum<
  | { type: "Overflow"; value: undefined }
  | { type: "Unimplemented"; value: undefined }
  | { type: "UntrustedReserveLocation"; value: undefined }
  | { type: "UntrustedTeleportLocation"; value: undefined }
  | { type: "LocationFull"; value: undefined }
  | { type: "LocationNotInvertible"; value: undefined }
  | { type: "BadOrigin"; value: undefined }
  | { type: "InvalidLocation"; value: undefined }
  | { type: "AssetNotFound"; value: undefined }
  | { type: "FailedToTransactAsset"; value: undefined }
  | { type: "NotWithdrawable"; value: undefined }
  | { type: "LocationCannotHold"; value: undefined }
  | { type: "ExceedsMaxMessageSize"; value: undefined }
  | { type: "DestinationUnsupported"; value: undefined }
  | { type: "Transport"; value: undefined }
  | { type: "Unroutable"; value: undefined }
  | { type: "UnknownClaim"; value: undefined }
  | { type: "FailedToDecode"; value: undefined }
  | { type: "MaxWeightInvalid"; value: undefined }
  | { type: "NotHoldingFees"; value: undefined }
  | { type: "TooExpensive"; value: undefined }
  | { type: "Trap"; value: Anonymize<bigint> }
  | { type: "ExpectationFalse"; value: undefined }
  | { type: "PalletNotFound"; value: undefined }
  | { type: "NameMismatch"; value: undefined }
  | { type: "VersionIncompatible"; value: undefined }
  | { type: "HoldingWouldOverflow"; value: undefined }
  | { type: "ExportError"; value: undefined }
  | { type: "ReanchorFailed"; value: undefined }
  | { type: "NoDeal"; value: undefined }
  | { type: "FeesNotMet"; value: undefined }
  | { type: "LockError"; value: undefined }
  | { type: "NoPermission"; value: undefined }
  | { type: "Unanchored"; value: undefined }
  | { type: "NotDepositable"; value: undefined }
  | { type: "UnhandledXcmVersion"; value: undefined }
  | { type: "WeightLimitReached"; value: Anonymize<I4q39t5hn830vp> }
  | { type: "Barrier"; value: undefined }
  | { type: "WeightNotComputable"; value: undefined }
  | { type: "ExceedsStackLimit"; value: undefined }
>;
export const XcmV3TraitsError = _Enum as unknown as GetEnum<XcmV3TraitsError>;

type I5m6kclhssq59a = { error: XcmV3TraitsError };

type I3gb27uk4ul5am = {
  origin: Anonymize<Ieq428hdmibf6m>;
  destination: Anonymize<Ieq428hdmibf6m>;
  message: Anonymize<I9uem0hkif9mb7>;
  message_id: Binary;
};

type I9uem0hkif9mb7 = Array<XcmV4Instruction>;

export type XcmV4Instruction = Enum<
  | { type: "WithdrawAsset"; value: Anonymize<Ia04r3ent3ib9l> }
  | { type: "ReserveAssetDeposited"; value: Anonymize<Ia04r3ent3ib9l> }
  | { type: "ReceiveTeleportedAsset"; value: Anonymize<Ia04r3ent3ib9l> }
  | { type: "QueryResponse"; value: Anonymize<If5f25il1ge6a5> }
  | { type: "TransferAsset"; value: Anonymize<I9o6pmhgf6qb2n> }
  | { type: "TransferReserveAsset"; value: Anonymize<Ia3b5a2vfbfo8f> }
  | { type: "Transact"; value: Anonymize<I4sfmje1omkmem> }
  | { type: "HrmpNewChannelOpenRequest"; value: Anonymize<I5uhhrjqfuo4e5> }
  | { type: "HrmpChannelAccepted"; value: Anonymize<Ifij4jam0o7sub> }
  | { type: "HrmpChannelClosing"; value: Anonymize<Ieeb4svd9i8fji> }
  | { type: "ClearOrigin"; value: undefined }
  | { type: "DescendOrigin"; value: Anonymize<XcmV4Junctions> }
  | { type: "ReportError"; value: Anonymize<I2t1argld0e8ha> }
  | { type: "DepositAsset"; value: Anonymize<I1jvdebe5g7h7j> }
  | { type: "DepositReserveAsset"; value: Anonymize<Ip007io6ag12n> }
  | { type: "ExchangeAsset"; value: Anonymize<Ifblc1cck9cd37> }
  | { type: "InitiateReserveWithdraw"; value: Anonymize<I6p8gvucdlt5k7> }
  | { type: "InitiateTeleport"; value: Anonymize<Ip007io6ag12n> }
  | { type: "ReportHolding"; value: Anonymize<I674ulrvvjnhs0> }
  | { type: "BuyExecution"; value: Anonymize<Idbkkieivqk1gi> }
  | { type: "RefundSurplus"; value: undefined }
  | { type: "SetErrorHandler"; value: Anonymize<I9uem0hkif9mb7> }
  | { type: "SetAppendix"; value: Anonymize<I9uem0hkif9mb7> }
  | { type: "ClearError"; value: undefined }
  | { type: "ClaimAsset"; value: Anonymize<I66uqjrc5dmf2e> }
  | { type: "Trap"; value: Anonymize<bigint> }
  | { type: "SubscribeVersion"; value: Anonymize<Ieprdqqu7ildvr> }
  | { type: "UnsubscribeVersion"; value: undefined }
  | { type: "BurnAsset"; value: Anonymize<Ia04r3ent3ib9l> }
  | { type: "ExpectAsset"; value: Anonymize<Ia04r3ent3ib9l> }
  | { type: "ExpectOrigin"; value: Anonymize<Iehbh3m350i09b> }
  | { type: "ExpectError"; value: Anonymize<I8j770n2arfq59> }
  | { type: "ExpectTransactStatus"; value: Anonymize<XcmV3MaybeErrorCode> }
  | { type: "QueryPallet"; value: Anonymize<Ibltfjhsfj1s9i> }
  | { type: "ExpectPallet"; value: Anonymize<Id7mf37dkpgfjs> }
  | { type: "ReportTransactStatus"; value: Anonymize<I2t1argld0e8ha> }
  | { type: "ClearTransactStatus"; value: undefined }
  | { type: "UniversalOrigin"; value: Anonymize<XcmV4Junction> }
  | { type: "ExportMessage"; value: Anonymize<Ibrkdv8bf9uaq7> }
  | { type: "LockAsset"; value: Anonymize<Id49srnpojvmi6> }
  | { type: "UnlockAsset"; value: Anonymize<I3lc2qil81b3dt> }
  | { type: "NoteUnlockable"; value: Anonymize<Ib2aloe249nr2> }
  | { type: "RequestUnlock"; value: Anonymize<I2ida362kv5gol> }
  | { type: "SetFeesMode"; value: Anonymize<I4nae9rsql8fa7> }
  | { type: "SetTopic"; value: Anonymize<Binary> }
  | { type: "ClearTopic"; value: undefined }
  | { type: "AliasOrigin"; value: Anonymize<Ieq428hdmibf6m> }
  | { type: "UnpaidExecution"; value: Anonymize<I943mf7c2b9qmm> }
>;
export const XcmV4Instruction = _Enum as unknown as GetEnum<XcmV4Instruction>;

type Ia04r3ent3ib9l = Array<Anonymize<Ib6ueef3pthi1j>>;

type Ib6ueef3pthi1j = { id: Anonymize<Ieq428hdmibf6m>; fun: XcmV3MultiassetFungibility };

export type XcmV3MultiassetFungibility = Enum<
  | { type: "Fungible"; value: Anonymize<bigint> }
  | { type: "NonFungible"; value: Anonymize<XcmV3MultiassetAssetInstance> }
>;
export const XcmV3MultiassetFungibility = _Enum as unknown as GetEnum<XcmV3MultiassetFungibility>;

export type XcmV3MultiassetAssetInstance = Enum<
  | { type: "Undefined"; value: undefined }
  | { type: "Index"; value: Anonymize<bigint> }
  | { type: "Array4"; value: Anonymize<Binary> }
  | { type: "Array8"; value: Anonymize<Binary> }
  | { type: "Array16"; value: Anonymize<Binary> }
  | { type: "Array32"; value: Anonymize<Binary> }
>;
export const XcmV3MultiassetAssetInstance =
  _Enum as unknown as GetEnum<XcmV3MultiassetAssetInstance>;

type If5f25il1ge6a5 = {
  query_id: bigint;
  response: XcmV4Response;
  max_weight: Anonymize<I4q39t5hn830vp>;
  querier: Anonymize<Iehbh3m350i09b>;
};

export type XcmV4Response = Enum<
  | { type: "Null"; value: undefined }
  | { type: "Assets"; value: Anonymize<Ia04r3ent3ib9l> }
  | { type: "ExecutionResult"; value: Anonymize<I8j770n2arfq59> }
  | { type: "Version"; value: Anonymize<number> }
  | { type: "PalletsInfo"; value: Anonymize<I599u7h20b52at> }
  | { type: "DispatchResult"; value: Anonymize<XcmV3MaybeErrorCode> }
>;
export const XcmV4Response = _Enum as unknown as GetEnum<XcmV4Response>;

type I8j770n2arfq59 = Anonymize<Ibgcthk0mc326i> | undefined;

type Ibgcthk0mc326i = [number, XcmV3TraitsError];

type I599u7h20b52at = Array<Anonymize<Ift5r9b1bvoh16>>;

type Ift5r9b1bvoh16 = {
  index: number;
  name: Binary;
  module_name: Binary;
  major: number;
  minor: number;
  patch: number;
};

export type XcmV3MaybeErrorCode = Enum<
  | { type: "Success"; value: undefined }
  | { type: "Error"; value: Anonymize<Binary> }
  | { type: "TruncatedError"; value: Anonymize<Binary> }
>;
export const XcmV3MaybeErrorCode = _Enum as unknown as GetEnum<XcmV3MaybeErrorCode>;

type Iehbh3m350i09b = Anonymize<Ieq428hdmibf6m> | undefined;

type I9o6pmhgf6qb2n = { assets: Anonymize<Ia04r3ent3ib9l>; beneficiary: Anonymize<Ieq428hdmibf6m> };

type Ia3b5a2vfbfo8f = {
  assets: Anonymize<Ia04r3ent3ib9l>;
  dest: Anonymize<Ieq428hdmibf6m>;
  xcm: Anonymize<I9uem0hkif9mb7>;
};

type I4sfmje1omkmem = {
  origin_kind: XcmV2OriginKind;
  require_weight_at_most: Anonymize<I4q39t5hn830vp>;
  call: Binary;
};

export type XcmV2OriginKind = Enum<
  | { type: "Native"; value: undefined }
  | { type: "SovereignAccount"; value: undefined }
  | { type: "Superuser"; value: undefined }
  | { type: "Xcm"; value: undefined }
>;
export const XcmV2OriginKind = _Enum as unknown as GetEnum<XcmV2OriginKind>;

type I5uhhrjqfuo4e5 = { sender: number; max_message_size: number; max_capacity: number };

type Ifij4jam0o7sub = { recipient: number };

type Ieeb4svd9i8fji = { initiator: number; sender: number; recipient: number };

type I2t1argld0e8ha = {
  destination: Anonymize<Ieq428hdmibf6m>;
  query_id: bigint;
  max_weight: Anonymize<I4q39t5hn830vp>;
};

type I1jvdebe5g7h7j = { assets: XcmV4AssetAssetFilter; beneficiary: Anonymize<Ieq428hdmibf6m> };

export type XcmV4AssetAssetFilter = Enum<
  | { type: "Definite"; value: Anonymize<Ia04r3ent3ib9l> }
  | { type: "Wild"; value: Anonymize<XcmV4AssetWildAsset> }
>;
export const XcmV4AssetAssetFilter = _Enum as unknown as GetEnum<XcmV4AssetAssetFilter>;

export type XcmV4AssetWildAsset = Enum<
  | { type: "All"; value: undefined }
  | { type: "AllOf"; value: Anonymize<I2csjl19ng8cbf> }
  | { type: "AllCounted"; value: Anonymize<number> }
  | { type: "AllOfCounted"; value: Anonymize<I79ln7nq4vjajd> }
>;
export const XcmV4AssetWildAsset = _Enum as unknown as GetEnum<XcmV4AssetWildAsset>;

type I2csjl19ng8cbf = { id: Anonymize<Ieq428hdmibf6m>; fun: XcmV2MultiassetWildFungibility };

export type XcmV2MultiassetWildFungibility = Enum<
  { type: "Fungible"; value: undefined } | { type: "NonFungible"; value: undefined }
>;
export const XcmV2MultiassetWildFungibility =
  _Enum as unknown as GetEnum<XcmV2MultiassetWildFungibility>;

type I79ln7nq4vjajd = {
  id: Anonymize<Ieq428hdmibf6m>;
  fun: XcmV2MultiassetWildFungibility;
  count: number;
};

type Ip007io6ag12n = {
  assets: XcmV4AssetAssetFilter;
  dest: Anonymize<Ieq428hdmibf6m>;
  xcm: Anonymize<I9uem0hkif9mb7>;
};

type Ifblc1cck9cd37 = {
  give: XcmV4AssetAssetFilter;
  want: Anonymize<Ia04r3ent3ib9l>;
  maximal: boolean;
};

type I6p8gvucdlt5k7 = {
  assets: XcmV4AssetAssetFilter;
  reserve: Anonymize<Ieq428hdmibf6m>;
  xcm: Anonymize<I9uem0hkif9mb7>;
};

type I674ulrvvjnhs0 = { response_info: Anonymize<I2t1argld0e8ha>; assets: XcmV4AssetAssetFilter };

type Idbkkieivqk1gi = { fees: Anonymize<Ib6ueef3pthi1j>; weight_limit: XcmV3WeightLimit };

export type XcmV3WeightLimit = Enum<
  { type: "Unlimited"; value: undefined } | { type: "Limited"; value: Anonymize<I4q39t5hn830vp> }
>;
export const XcmV3WeightLimit = _Enum as unknown as GetEnum<XcmV3WeightLimit>;

type I66uqjrc5dmf2e = { assets: Anonymize<Ia04r3ent3ib9l>; ticket: Anonymize<Ieq428hdmibf6m> };

type Ieprdqqu7ildvr = { query_id: bigint; max_response_weight: Anonymize<I4q39t5hn830vp> };

type Ibltfjhsfj1s9i = { module_name: Binary; response_info: Anonymize<I2t1argld0e8ha> };

type Id7mf37dkpgfjs = {
  index: number;
  name: Binary;
  module_name: Binary;
  crate_major: number;
  min_crate_minor: number;
};

type Ibrkdv8bf9uaq7 = {
  network: XcmV4JunctionNetworkId;
  destination: XcmV4Junctions;
  xcm: Anonymize<I9uem0hkif9mb7>;
};

type Id49srnpojvmi6 = { asset: Anonymize<Ib6ueef3pthi1j>; unlocker: Anonymize<Ieq428hdmibf6m> };

type I3lc2qil81b3dt = { asset: Anonymize<Ib6ueef3pthi1j>; target: Anonymize<Ieq428hdmibf6m> };

type Ib2aloe249nr2 = { asset: Anonymize<Ib6ueef3pthi1j>; owner: Anonymize<Ieq428hdmibf6m> };

type I2ida362kv5gol = { asset: Anonymize<Ib6ueef3pthi1j>; locker: Anonymize<Ieq428hdmibf6m> };

type I4nae9rsql8fa7 = { jit_withdraw: boolean };

type I943mf7c2b9qmm = { weight_limit: XcmV3WeightLimit; check_origin: Anonymize<Iehbh3m350i09b> };

type Ic8gqph940s142 = { origin: Anonymize<Ieq428hdmibf6m>; query_id: bigint };

type I9k3jmtgmoiqqf = { query_id: bigint; response: XcmV4Response };

type I2uqmls7kcdnii = { query_id: bigint; pallet_index: number; call_index: number };

type Idg69klialbkb8 = {
  query_id: bigint;
  pallet_index: number;
  call_index: number;
  actual_weight: Anonymize<I4q39t5hn830vp>;
  max_budgeted_weight: Anonymize<I4q39t5hn830vp>;
};

type I24et57o2f2i2m = {
  origin: Anonymize<Ieq428hdmibf6m>;
  query_id: bigint;
  expected_location: Anonymize<Iehbh3m350i09b>;
};

type I30pg328m00nr3 = { query_id: bigint };

type I20naetn2lovo9 = {
  hash: Binary;
  origin: Anonymize<Ieq428hdmibf6m>;
  assets: XcmVersionedAssets;
};

export type XcmVersionedAssets = Enum<
  | { type: "V2"; value: Anonymize<Ia3ggl9eghkufh> }
  | { type: "V3"; value: Anonymize<I2pdjq1umlp617> }
  | { type: "V4"; value: Anonymize<Ia04r3ent3ib9l> }
>;
export const XcmVersionedAssets = _Enum as unknown as GetEnum<XcmVersionedAssets>;

type Ia3ggl9eghkufh = Array<Anonymize<I16mc4mv5bb0qd>>;

type I16mc4mv5bb0qd = { id: XcmV2MultiassetAssetId; fun: XcmV2MultiassetFungibility };

export type XcmV2MultiassetAssetId = Enum<
  | { type: "Concrete"; value: Anonymize<Ibki0d249v3ojt> }
  | { type: "Abstract"; value: Anonymize<Binary> }
>;
export const XcmV2MultiassetAssetId = _Enum as unknown as GetEnum<XcmV2MultiassetAssetId>;

export type XcmV2MultiassetFungibility = Enum<
  | { type: "Fungible"; value: Anonymize<bigint> }
  | { type: "NonFungible"; value: Anonymize<XcmV2MultiassetAssetInstance> }
>;
export const XcmV2MultiassetFungibility = _Enum as unknown as GetEnum<XcmV2MultiassetFungibility>;

export type XcmV2MultiassetAssetInstance = Enum<
  | { type: "Undefined"; value: undefined }
  | { type: "Index"; value: Anonymize<bigint> }
  | { type: "Array4"; value: Anonymize<Binary> }
  | { type: "Array8"; value: Anonymize<Binary> }
  | { type: "Array16"; value: Anonymize<Binary> }
  | { type: "Array32"; value: Anonymize<Binary> }
  | { type: "Blob"; value: Anonymize<Binary> }
>;
export const XcmV2MultiassetAssetInstance =
  _Enum as unknown as GetEnum<XcmV2MultiassetAssetInstance>;

type I2pdjq1umlp617 = Array<Anonymize<Isj6qus1lv5t9>>;

type Isj6qus1lv5t9 = { id: XcmV3MultiassetAssetId; fun: XcmV3MultiassetFungibility };

type I4ks28k7d44l1p = {
  destination: Anonymize<Ieq428hdmibf6m>;
  result: number;
  cost: Anonymize<Ia04r3ent3ib9l>;
  message_id: Binary;
};

type Iergfrmj3c9dor = { location: Anonymize<Ieq428hdmibf6m>; version: number };

type I9jji4pfgcqc9b = {
  location: Anonymize<Ieq428hdmibf6m>;
  query_id: bigint;
  error: XcmV3TraitsError;
};

type I6djqr04u42c2g = { location: XcmVersionedLocation; query_id: bigint };

type I6ctg8b4bcjdek = {
  origin: Anonymize<Ieq428hdmibf6m>;
  query_id: bigint;
  expected_querier: Anonymize<Ieq428hdmibf6m>;
  maybe_actual_querier: Anonymize<Iehbh3m350i09b>;
};

type I4fi6oaheho56l = {
  destination: Anonymize<Ieq428hdmibf6m>;
  cost: Anonymize<Ia04r3ent3ib9l>;
  message_id: Binary;
};

type I89pfbi9d7lm5p = { paying: Anonymize<Ieq428hdmibf6m>; fees: Anonymize<Ia04r3ent3ib9l> };

type I6s1nbislhk619 = { version: number };

export type PolkadotRuntimeCommonIdentityMigratorEvent = Enum<
  | { type: "IdentityReaped"; value: Anonymize<I4cbvqmqadhrea> }
  | { type: "DepositUpdated"; value: Anonymize<I4i3u9uui7ktsd> }
>;
export const PolkadotRuntimeCommonIdentityMigratorEvent =
  _Enum as unknown as GetEnum<PolkadotRuntimeCommonIdentityMigratorEvent>;

type I4i3u9uui7ktsd = { who: SS58String; identity: bigint; subs: bigint };

export type PolkadotRuntimeCommonAssignedSlotsEvent = Enum<
  | { type: "PermanentSlotAssigned"; value: Anonymize<number> }
  | { type: "TemporarySlotAssigned"; value: Anonymize<number> }
  | { type: "MaxPermanentSlotsChanged"; value: Anonymize<I9d5h5irbki7mm> }
  | { type: "MaxTemporarySlotsChanged"; value: Anonymize<I9d5h5irbki7mm> }
>;
export const PolkadotRuntimeCommonAssignedSlotsEvent =
  _Enum as unknown as GetEnum<PolkadotRuntimeCommonAssignedSlotsEvent>;

type I9d5h5irbki7mm = { slots: number };

type I6p26ej1p3cokp = AnonymousEnum<{
  ValidatorsRegistered: Anonymize<Ia2lhg7l2hilo3>;
  ValidatorsDeregistered: Anonymize<Ia2lhg7l2hilo3>;
}>;

type I1iphd51t39ha0 = AnonymousEnum<{
  Migrated: Anonymize<I3m87ve6mm5k6n>;
  Slashed: Anonymize<Id5fm4p8lj5qgi>;
  AutoMigrationFinished: undefined;
  Halted: Anonymize<I73c6rq6fkseh2>;
}>;

type I3m87ve6mm5k6n = { top: number; child: number; compute: Anonymize<I2os86rbc98tr0> };

type I2os86rbc98tr0 = AnonymousEnum<{ Signed: undefined; Auto: undefined }>;

type I73c6rq6fkseh2 = { error: Anonymize<Ihok5kol9m3tk> };

type Ihok5kol9m3tk = AnonymousEnum<{
  MaxSignedLimits: undefined;
  KeyTooLong: undefined;
  NotEnoughFunds: undefined;
  BadWitness: undefined;
  SignedMigrationNotAllowed: undefined;
  BadChildRoot: undefined;
}>;

export type RootTestingEvent = Enum<{ type: "DefensiveTestCall"; value: undefined }>;
export const RootTestingEvent = _Enum as unknown as GetEnum<RootTestingEvent>;

export type SudoEvent = Enum<
  | { type: "Sudid"; value: Anonymize<I331o7t2g0ooi9> }
  | { type: "KeyChanged"; value: Anonymize<I5rtkmhm2dng4u> }
  | { type: "KeyRemoved"; value: undefined }
  | { type: "SudoAsDone"; value: Anonymize<I331o7t2g0ooi9> }
>;
export const SudoEvent = _Enum as unknown as GetEnum<SudoEvent>;

type I331o7t2g0ooi9 = { sudo_result: Anonymize<Idtdr91jmq5g4i> };

type I5rtkmhm2dng4u = { old: Anonymize<Ihfphjolmsqq1>; new: SS58String };

type Ihfphjolmsqq1 = SS58String | undefined;

type Idhnf6rtqoslea = Array<Binary>;

type Iep4uo61810hfs = Array<Anonymize<I5g2vv0ckl2m8b>>;

type Ieniouoqkq4icf = { spec_version: number; spec_name: string };

export type BabeDigestsNextConfigDescriptor = Enum<{
  type: "V1";
  value: Anonymize<Idkva8q2m9meg0>;
}>;
export const BabeDigestsNextConfigDescriptor =
  _Enum as unknown as GetEnum<BabeDigestsNextConfigDescriptor>;

type Idkva8q2m9meg0 = { c: Anonymize<I2j729bmgsdiuo>; allowed_slots: BabeAllowedSlots };

type I2j729bmgsdiuo = [bigint, bigint];

export type BabeAllowedSlots = Enum<
  | { type: "PrimarySlots"; value: undefined }
  | { type: "PrimaryAndSecondaryPlainSlots"; value: undefined }
  | { type: "PrimaryAndSecondaryVRFSlots"; value: undefined }
>;
export const BabeAllowedSlots = _Enum as unknown as GetEnum<BabeAllowedSlots>;

type I7eh0hrps1dlgp = BabeDigestsPreDigest | undefined;

export type BabeDigestsPreDigest = Enum<
  | { type: "Primary"; value: Anonymize<I5rf8dkqp7clbd> }
  | { type: "SecondaryPlain"; value: Anonymize<Ieiaevc5q41ard> }
  | { type: "SecondaryVRF"; value: Anonymize<I5rf8dkqp7clbd> }
>;
export const BabeDigestsPreDigest = _Enum as unknown as GetEnum<BabeDigestsPreDigest>;

type I5rf8dkqp7clbd = {
  authority_index: number;
  slot: bigint;
  vrf_signature: Anonymize<I86q4svsgk48rm>;
};

type I86q4svsgk48rm = { pre_output: Binary; proof: Binary };

type Ieiaevc5q41ard = { authority_index: number; slot: bigint };

type Ifip05kcrl65am = Array<Anonymize<I6cs1itejju2vv>>;

type I6cs1itejju2vv = [bigint, number];

type Iff9heri56m1mb = [SS58String, bigint, boolean];

type Ibtifm27unhnjf = Array<Anonymize<I5b29v4qfq4tu7>>;

type I5b29v4qfq4tu7 = { id: Binary; amount: bigint; reasons: BalancesTypesReasons };

export type BalancesTypesReasons = Enum<
  | { type: "Fee"; value: undefined }
  | { type: "Misc"; value: undefined }
  | { type: "All"; value: undefined }
>;
export const BalancesTypesReasons = _Enum as unknown as GetEnum<BalancesTypesReasons>;

type Iactubn52acokl = Array<Anonymize<I32btm6htd9bck>>;

type I32btm6htd9bck = { id: Binary; amount: bigint };

type If53nbqqmg1310 = Array<Anonymize<Iemrlqc7oc4k5g>>;

type Iemrlqc7oc4k5g = { id: Anonymize<I24f41ie28n4vi>; amount: bigint };

type I24f41ie28n4vi = AnonymousEnum<{
  Preimage: Anonymize<PreimagePalletHoldReason>;
  Nis: Anonymize<Iatea4m2ji88k0>;
  StateTrieMigration: Anonymize<I49rq474dsd2ta>;
}>;

export type PreimagePalletHoldReason = Enum<{ type: "Preimage"; value: undefined }>;
export const PreimagePalletHoldReason = _Enum as unknown as GetEnum<PreimagePalletHoldReason>;

type Iatea4m2ji88k0 = AnonymousEnum<{ NftReceipt: undefined }>;

type I49rq474dsd2ta = AnonymousEnum<{ SlashForMigrate: undefined }>;

type I3fe7nt2ne6qtf = Array<Anonymize<I7qdm60946h5u9>>;

type I7qdm60946h5u9 = { id: undefined; amount: bigint };

export type TransactionPaymentReleases = Enum<
  { type: "V1Ancient"; value: undefined } | { type: "V2"; value: undefined }
>;
export const TransactionPaymentReleases = _Enum as unknown as GetEnum<TransactionPaymentReleases>;

type I1ajd3ic8vp3kt = { offender: Anonymize<Iidji8iemeki4>; reporters: Anonymize<Ia2lhg7l2hilo3> };

type Iidji8iemeki4 = [SS58String, undefined];

type Iasase33a12046 = [Binary, Binary];

type Iavru0osmavfj6 = Array<Anonymize<I9mnddod4k7nfj>>;

type I9mnddod4k7nfj = [SS58String, Anonymize<Ieup5s9ka3brpi>];

type Ieup5s9ka3brpi = {
  grandpa: Binary;
  babe: Binary;
  para_validator: Binary;
  para_assignment: Binary;
  authority_discovery: Binary;
  beefy: Binary;
};

type Icgljjb6j82uhn = Array<number>;

export type GrandpaStoredState = Enum<
  | { type: "Live"; value: undefined }
  | { type: "PendingPause"; value: Anonymize<Ib95oqfalvjqfe> }
  | { type: "Paused"; value: undefined }
  | { type: "PendingResume"; value: Anonymize<Ib95oqfalvjqfe> }
>;
export const GrandpaStoredState = _Enum as unknown as GetEnum<GrandpaStoredState>;

type Ib95oqfalvjqfe = { scheduled_at: number; delay: number };

type Ibu50q5vhsfq5e = {
  scheduled_at: number;
  delay: number;
  next_authorities: Anonymize<I2qinct8jq4bqe>;
  forced: Anonymize<I4arjljr6dpflb>;
};

type I4arjljr6dpflb = number | undefined;

type Iegmj7n48sc3am = {
  proposer: SS58String;
  value: bigint;
  beneficiary: SS58String;
  bond: bigint;
};

type Icm1qakfrk6lcb = {
  asset_kind: PolkadotRuntimeCommonImplsVersionedLocatableAsset;
  amount: bigint;
  beneficiary: XcmVersionedLocation;
  valid_from: number;
  expire_at: number;
  status: TreasuryPaymentState;
};

export type TreasuryPaymentState = Enum<
  | { type: "Pending"; value: undefined }
  | { type: "Attempted"; value: Anonymize<I4ov6e94l79mbg> }
  | { type: "Failed"; value: undefined }
>;
export const TreasuryPaymentState = _Enum as unknown as GetEnum<TreasuryPaymentState>;

type I4ov6e94l79mbg = { id: bigint };

export type ConvictionVotingVoteVoting = Enum<
  | { type: "Casting"; value: Anonymize<If52hjr5c5nrc5> }
  | { type: "Delegating"; value: Anonymize<I77dj6ku8n5d58> }
>;
export const ConvictionVotingVoteVoting = _Enum as unknown as GetEnum<ConvictionVotingVoteVoting>;

type If52hjr5c5nrc5 = {
  votes: Anonymize<I42jj1su7asrm9>;
  delegations: Anonymize<I538qha8r4j3ii>;
  prior: Anonymize<I4ojmnsk1dchql>;
};

type I42jj1su7asrm9 = Array<Anonymize<I7mk5ivue8lr2m>>;

type I7mk5ivue8lr2m = [number, ConvictionVotingVoteAccountVote];

export type ConvictionVotingVoteAccountVote = Enum<
  | { type: "Standard"; value: Anonymize<Ib024p97ls1cla> }
  | { type: "Split"; value: Anonymize<I5pi71t9bosoiv> }
  | { type: "SplitAbstain"; value: Anonymize<I89irppcaqmf1i> }
>;
export const ConvictionVotingVoteAccountVote =
  _Enum as unknown as GetEnum<ConvictionVotingVoteAccountVote>;

type Ib024p97ls1cla = { vote: number; balance: bigint };

type I5pi71t9bosoiv = { aye: bigint; nay: bigint };

type I89irppcaqmf1i = { aye: bigint; nay: bigint; abstain: bigint };

type I538qha8r4j3ii = { votes: bigint; capital: bigint };

type I4ojmnsk1dchql = [number, bigint];

type I77dj6ku8n5d58 = {
  balance: bigint;
  target: SS58String;
  conviction: VotingConviction;
  delegations: Anonymize<I538qha8r4j3ii>;
  prior: Anonymize<I4ojmnsk1dchql>;
};

export type VotingConviction = Enum<
  | { type: "None"; value: undefined }
  | { type: "Locked1x"; value: undefined }
  | { type: "Locked2x"; value: undefined }
  | { type: "Locked3x"; value: undefined }
  | { type: "Locked4x"; value: undefined }
  | { type: "Locked5x"; value: undefined }
  | { type: "Locked6x"; value: undefined }
>;
export const VotingConviction = _Enum as unknown as GetEnum<VotingConviction>;

type I6ouflveob4eli = [SS58String, number];

type If9jidduiuq7vv = Array<Anonymize<I4ojmnsk1dchql>>;

export type ReferendaTypesReferendumInfo = Enum<
  | { type: "Ongoing"; value: Anonymize<Iamhkqqbrf9050> }
  | { type: "Approved"; value: Anonymize<Ini94eljn5lj8> }
  | { type: "Rejected"; value: Anonymize<Ini94eljn5lj8> }
  | { type: "Cancelled"; value: Anonymize<Ini94eljn5lj8> }
  | { type: "TimedOut"; value: Anonymize<Ini94eljn5lj8> }
  | { type: "Killed"; value: Anonymize<number> }
>;
export const ReferendaTypesReferendumInfo =
  _Enum as unknown as GetEnum<ReferendaTypesReferendumInfo>;

type Iamhkqqbrf9050 = {
  track: number;
  origin: WestendRuntimeOriginCaller;
  proposal: PreimagesBounded;
  enactment: TraitsScheduleDispatchTime;
  submitted: number;
  submission_deposit: Anonymize<Id5fm4p8lj5qgi>;
  decision_deposit: Anonymize<Ibd24caul84kv2>;
  deciding: Anonymize<Ibcbcndfmk0jd9>;
  tally: Anonymize<Ifsk7cbmtit1jd>;
  in_queue: boolean;
  alarm: Anonymize<I653443ft89b0e>;
};

export type WestendRuntimeOriginCaller = Enum<
  | { type: "system"; value: Anonymize<DispatchRawOrigin> }
  | { type: "Origins"; value: Anonymize<WestendRuntimeGovernanceOriginsPalletCustomOriginsOrigin> }
  | { type: "ParachainsOrigin"; value: Anonymize<ParachainsOrigin> }
  | { type: "XcmPallet"; value: Anonymize<XcmPalletOrigin> }
  | { type: "Void"; value: Anonymize<undefined> }
>;
export const WestendRuntimeOriginCaller = _Enum as unknown as GetEnum<WestendRuntimeOriginCaller>;

export type DispatchRawOrigin = Enum<
  | { type: "Root"; value: undefined }
  | { type: "Signed"; value: Anonymize<SS58String> }
  | { type: "None"; value: undefined }
>;
export const DispatchRawOrigin = _Enum as unknown as GetEnum<DispatchRawOrigin>;

export type WestendRuntimeGovernanceOriginsPalletCustomOriginsOrigin = Enum<
  | { type: "StakingAdmin"; value: undefined }
  | { type: "Treasurer"; value: undefined }
  | { type: "FellowshipAdmin"; value: undefined }
  | { type: "GeneralAdmin"; value: undefined }
  | { type: "AuctionAdmin"; value: undefined }
  | { type: "LeaseAdmin"; value: undefined }
  | { type: "ReferendumCanceller"; value: undefined }
  | { type: "ReferendumKiller"; value: undefined }
  | { type: "SmallTipper"; value: undefined }
  | { type: "BigTipper"; value: undefined }
  | { type: "SmallSpender"; value: undefined }
  | { type: "MediumSpender"; value: undefined }
  | { type: "BigSpender"; value: undefined }
  | { type: "WhitelistedCaller"; value: undefined }
  | { type: "FellowshipInitiates"; value: undefined }
  | { type: "Fellows"; value: undefined }
  | { type: "FellowshipExperts"; value: undefined }
  | { type: "FellowshipMasters"; value: undefined }
  | { type: "Fellowship1Dan"; value: undefined }
  | { type: "Fellowship2Dan"; value: undefined }
  | { type: "Fellowship3Dan"; value: undefined }
  | { type: "Fellowship4Dan"; value: undefined }
  | { type: "Fellowship5Dan"; value: undefined }
  | { type: "Fellowship6Dan"; value: undefined }
  | { type: "Fellowship7Dan"; value: undefined }
  | { type: "Fellowship8Dan"; value: undefined }
  | { type: "Fellowship9Dan"; value: undefined }
>;
export const WestendRuntimeGovernanceOriginsPalletCustomOriginsOrigin =
  _Enum as unknown as GetEnum<WestendRuntimeGovernanceOriginsPalletCustomOriginsOrigin>;

export type ParachainsOrigin = Enum<{ type: "Parachain"; value: Anonymize<number> }>;
export const ParachainsOrigin = _Enum as unknown as GetEnum<ParachainsOrigin>;

export type XcmPalletOrigin = Enum<
  | { type: "Xcm"; value: Anonymize<Ieq428hdmibf6m> }
  | { type: "Response"; value: Anonymize<Ieq428hdmibf6m> }
>;
export const XcmPalletOrigin = _Enum as unknown as GetEnum<XcmPalletOrigin>;

export type TraitsScheduleDispatchTime = Enum<
  { type: "At"; value: Anonymize<number> } | { type: "After"; value: Anonymize<number> }
>;
export const TraitsScheduleDispatchTime = _Enum as unknown as GetEnum<TraitsScheduleDispatchTime>;

type Ibd24caul84kv2 = Anonymize<Id5fm4p8lj5qgi> | undefined;

type Ibcbcndfmk0jd9 = Anonymize<I4a0pk3ivg0trh> | undefined;

type I4a0pk3ivg0trh = { since: number; confirming: Anonymize<I4arjljr6dpflb> };

type I653443ft89b0e = Anonymize<I52d1f0503mqv7> | undefined;

type I52d1f0503mqv7 = [number, Anonymize<I5g2vv0ckl2m8b>];

type Ini94eljn5lj8 = [number, Anonymize<Ibd24caul84kv2>, Anonymize<Ibd24caul84kv2>];

type I7svnfko10tq2e = [number, SS58String];

type Icscq172tqkto1 = AnonymousEnum<{
  Ongoing: Anonymize<Ieeijqsusb4a5r>;
  Approved: Anonymize<Ini94eljn5lj8>;
  Rejected: Anonymize<Ini94eljn5lj8>;
  Cancelled: Anonymize<Ini94eljn5lj8>;
  TimedOut: Anonymize<Ini94eljn5lj8>;
  Killed: Anonymize<number>;
}>;

type Ieeijqsusb4a5r = {
  track: number;
  origin: WestendRuntimeOriginCaller;
  proposal: PreimagesBounded;
  enactment: TraitsScheduleDispatchTime;
  submitted: number;
  submission_deposit: Anonymize<Id5fm4p8lj5qgi>;
  decision_deposit: Anonymize<Ibd24caul84kv2>;
  deciding: Anonymize<Ibcbcndfmk0jd9>;
  tally: Anonymize<I6d634btv7misu>;
  in_queue: boolean;
  alarm: Anonymize<I653443ft89b0e>;
};

type I2phecamkn3pej = [bigint, bigint, number];

export type ClaimsStatementKind = Enum<
  { type: "Regular"; value: undefined } | { type: "Saft"; value: undefined }
>;
export const ClaimsStatementKind = _Enum as unknown as GetEnum<ClaimsStatementKind>;

type I6vu1e4jp91ilu = [Anonymize<I60cr97f9of5s5>, Anonymize<Iabpgqcjikia83>];

type I60cr97f9of5s5 = {
  judgements: Anonymize<I8lr6mvrp6262n>;
  deposit: bigint;
  info: Anonymize<I939m6d6t3dsgm>;
};

type I8lr6mvrp6262n = Array<Anonymize<Icjt4rtq0era77>>;

type Icjt4rtq0era77 = [number, IdentityJudgement];

export type IdentityJudgement = Enum<
  | { type: "Unknown"; value: undefined }
  | { type: "FeePaid"; value: Anonymize<bigint> }
  | { type: "Reasonable"; value: undefined }
  | { type: "KnownGood"; value: undefined }
  | { type: "OutOfDate"; value: undefined }
  | { type: "LowQuality"; value: undefined }
  | { type: "Erroneous"; value: undefined }
>;
export const IdentityJudgement = _Enum as unknown as GetEnum<IdentityJudgement>;

type I939m6d6t3dsgm = {
  additional: Anonymize<I33d2rrv01sdlq>;
  display: IdentityTypesData;
  legal: IdentityTypesData;
  web: IdentityTypesData;
  riot: IdentityTypesData;
  email: IdentityTypesData;
  pgp_fingerprint: Anonymize<I9k5avl0v1ch15>;
  image: IdentityTypesData;
  twitter: IdentityTypesData;
};

type I33d2rrv01sdlq = Array<Anonymize<I22blct134ortt>>;

type I22blct134ortt = [IdentityTypesData, IdentityTypesData];

export type IdentityTypesData = Enum<
  | { type: "None"; value: undefined }
  | { type: "Raw0"; value: Anonymize<Binary> }
  | { type: "Raw1"; value: Anonymize<Binary> }
  | { type: "Raw2"; value: Anonymize<Binary> }
  | { type: "Raw3"; value: Anonymize<Binary> }
  | { type: "Raw4"; value: Anonymize<Binary> }
  | { type: "Raw5"; value: Anonymize<Binary> }
  | { type: "Raw6"; value: Anonymize<Binary> }
  | { type: "Raw7"; value: Anonymize<Binary> }
  | { type: "Raw8"; value: Anonymize<Binary> }
  | { type: "Raw9"; value: Anonymize<Binary> }
  | { type: "Raw10"; value: Anonymize<Binary> }
  | { type: "Raw11"; value: Anonymize<Binary> }
  | { type: "Raw12"; value: Anonymize<Binary> }
  | { type: "Raw13"; value: Anonymize<Binary> }
  | { type: "Raw14"; value: Anonymize<Binary> }
  | { type: "Raw15"; value: Anonymize<Binary> }
  | { type: "Raw16"; value: Anonymize<Binary> }
  | { type: "Raw17"; value: Anonymize<Binary> }
  | { type: "Raw18"; value: Anonymize<Binary> }
  | { type: "Raw19"; value: Anonymize<Binary> }
  | { type: "Raw20"; value: Anonymize<Binary> }
  | { type: "Raw21"; value: Anonymize<Binary> }
  | { type: "Raw22"; value: Anonymize<Binary> }
  | { type: "Raw23"; value: Anonymize<Binary> }
  | { type: "Raw24"; value: Anonymize<Binary> }
  | { type: "Raw25"; value: Anonymize<Binary> }
  | { type: "Raw26"; value: Anonymize<Binary> }
  | { type: "Raw27"; value: Anonymize<Binary> }
  | { type: "Raw28"; value: Anonymize<Binary> }
  | { type: "Raw29"; value: Anonymize<Binary> }
  | { type: "Raw30"; value: Anonymize<Binary> }
  | { type: "Raw31"; value: Anonymize<Binary> }
  | { type: "Raw32"; value: Anonymize<Binary> }
  | { type: "BlakeTwo256"; value: Anonymize<Binary> }
  | { type: "Sha256"; value: Anonymize<Binary> }
  | { type: "Keccak256"; value: Anonymize<Binary> }
  | { type: "ShaThree256"; value: Anonymize<Binary> }
>;
export const IdentityTypesData = _Enum as unknown as GetEnum<IdentityTypesData>;

type I9k5avl0v1ch15 = Binary | undefined;

type Iabpgqcjikia83 = Binary | undefined;

type Ib22937a5d3pt0 = [SS58String, IdentityTypesData];

type I4nfjdef0ibh44 = [bigint, Anonymize<Ia2lhg7l2hilo3>];

type I74af64m08r6as = Array<Anonymize<I48v3sekdprq30>>;

type I48v3sekdprq30 = Anonymize<Icj8lp9f0lq0bm> | undefined;

type Icj8lp9f0lq0bm = { account: SS58String; fee: bigint; fields: bigint };

type I9bhbof2vim227 = { suffix: Binary; allocation: number };

type I1hqsuequ24m0k = {
  rank: number;
  strikes: number;
  vouching: Anonymize<Iceunvs0fkhdlv>;
  index: number;
};

type Iceunvs0fkhdlv = Anonymize<Iejj37cuq6lpoo> | undefined;

type Iejj37cuq6lpoo = AnonymousEnum<{ Vouching: undefined; Banned: undefined }>;

type I1659sf24o6ga8 = { paid: bigint; payouts: Anonymize<If9jidduiuq7vv> };

type Iaru90csm3chpv = Array<Anonymize<I5cap79lpfupvc>>;

type I5cap79lpfupvc = { who: SS58String; kind: Anonymize<Idvcmh0j8tecg6>; value: bigint };

type Idvcmh0j8tecg6 = AnonymousEnum<{
  Deposit: Anonymize<bigint>;
  Vouch: Anonymize<I95l2k9b1re95f>;
}>;

type I95l2k9b1re95f = [SS58String, bigint];

type Idv2mjfqbsi6cn = {
  round: number;
  kind: Anonymize<Idvcmh0j8tecg6>;
  bid: bigint;
  tally: Anonymize<Ic4n54m09tc5na>;
  skeptic_struck: boolean;
};

type Ic4n54m09tc5na = { approvals: number; rejections: number };

type I7d29ossbog0d3 = { approve: boolean; weight: number };

type I7mgf69gvksvu2 = { who: SS58String; bid: bigint; round: number };

type I3sfg6klg56vmk = [SS58String, SS58String, Anonymize<Ic4n54m09tc5na>];

type Ibprd8oi8phm62 = {
  delay_period: number;
  deposit: bigint;
  friends: Anonymize<Ia2lhg7l2hilo3>;
  threshold: number;
};

type Idlqqo993i780l = { created: number; deposit: bigint; friends: Anonymize<Ia2lhg7l2hilo3> };

type Ifble4juuml5ig = Array<Anonymize<I4aro1m78pdrtt>>;

type I4aro1m78pdrtt = { locked: bigint; per_block: bigint; starting_block: number };

export type VestingReleases = Enum<
  { type: "V0"; value: undefined } | { type: "V1"; value: undefined }
>;
export const VestingReleases = _Enum as unknown as GetEnum<VestingReleases>;

type I1kvl80ro579be = Array<Anonymize<Ife7f8jq02avq2>>;

type Ife7f8jq02avq2 = Anonymize<Is68j4infrv00> | undefined;

type Is68j4infrv00 = {
  maybe_id: Anonymize<I17k3ujudqd5df>;
  priority: number;
  call: PreimagesBounded;
  maybe_periodic: Anonymize<I34gtdjipdmjpt>;
  origin: WestendRuntimeOriginCaller;
};

type I34gtdjipdmjpt = Anonymize<I5g2vv0ckl2m8b> | undefined;

type I56u24ncejr5kt = { total_retries: number; remaining: number; period: number };

type Icuqvtlu62viqh = [Anonymize<I4n424tm4s3ajv>, bigint];

type I4n424tm4s3ajv = Array<Anonymize<Iftel5vp12cst9>>;

type Iftel5vp12cst9 = {
  delegate: SS58String;
  proxy_type: Anonymize<I90b4nim1kup1v>;
  delay: number;
};

type I3tjio59962rg2 = [Anonymize<I99svmvk5amkcq>, bigint];

type I99svmvk5amkcq = Array<Anonymize<I5gk1hg65t79fm>>;

type I5gk1hg65t79fm = { real: SS58String; call_hash: Binary; height: number };

type Iag146hmjgqfgj = {
  when: Anonymize<Itvprrpb0nm3o>;
  deposit: bigint;
  depositor: SS58String;
  approvals: Anonymize<Ia2lhg7l2hilo3>;
};

type I73gble6tmb52f = [SS58String, Binary];

export type PreimageRequestStatus = Enum<
  | { type: "Unrequested"; value: Anonymize<I5jej6bvdjrisr> }
  | { type: "Requested"; value: Anonymize<Is7sg1rr9u2nm> }
>;
export const PreimageRequestStatus = _Enum as unknown as GetEnum<PreimageRequestStatus>;

type I5jej6bvdjrisr = { deposit: Anonymize<I95l2k9b1re95f>; len: number };

type Is7sg1rr9u2nm = {
  deposit: Anonymize<I92hdo1clkbp4g>;
  count: number;
  len: Anonymize<I4arjljr6dpflb>;
};

type I92hdo1clkbp4g = Anonymize<I95l2k9b1re95f> | undefined;

export type PreimageRequestStatus1 = Enum<
  | { type: "Unrequested"; value: Anonymize<Idvcv8961o32th> }
  | { type: "Requested"; value: Anonymize<In82i9avte5re> }
>;
export const PreimageRequestStatus1 = _Enum as unknown as GetEnum<PreimageRequestStatus1>;

type Idvcv8961o32th = { ticket: Anonymize<I95l2k9b1re95f>; len: number };

type In82i9avte5re = {
  maybe_ticket: Anonymize<I92hdo1clkbp4g>;
  count: number;
  maybe_len: Anonymize<I4arjljr6dpflb>;
};

type Ic28sp53lp2v4a = {
  proposer: SS58String;
  value: bigint;
  fee: bigint;
  curator_deposit: bigint;
  bond: bigint;
  status: BountiesBountyStatus;
};

export type BountiesBountyStatus = Enum<
  | { type: "Proposed"; value: undefined }
  | { type: "Approved"; value: undefined }
  | { type: "Funded"; value: undefined }
  | { type: "CuratorProposed"; value: Anonymize<I846573mdj1pfn> }
  | { type: "Active"; value: Anonymize<I5s3sqq6r9nt63> }
  | { type: "PendingPayout"; value: Anonymize<I4aulgjqrdphrm> }
>;
export const BountiesBountyStatus = _Enum as unknown as GetEnum<BountiesBountyStatus>;

type I846573mdj1pfn = { curator: SS58String };

type I5s3sqq6r9nt63 = { curator: SS58String; update_due: number };

type I4aulgjqrdphrm = { curator: SS58String; beneficiary: SS58String; unlock_at: number };

type Idvn49j11itec9 = {
  parent_bounty: number;
  value: bigint;
  fee: bigint;
  curator_deposit: bigint;
  status: ChildBountyStatus;
};

export type ChildBountyStatus = Enum<
  | { type: "Added"; value: undefined }
  | { type: "CuratorProposed"; value: Anonymize<I846573mdj1pfn> }
  | { type: "Active"; value: Anonymize<I846573mdj1pfn> }
  | { type: "PendingPayout"; value: Anonymize<I4aulgjqrdphrm> }
>;
export const ChildBountyStatus = _Enum as unknown as GetEnum<ChildBountyStatus>;

type I2cqtnj49ogo81 = Array<Anonymize<Id5fm4p8lj5qgi>>;

type I7btpfdeli5ea8 = {
  proportion_owed: bigint;
  index: number;
  thawed: bigint;
  last_period: number;
  receipts_on_hold: bigint;
};

type Ic79q40j9qptgh = { proportion: bigint; owner: Anonymize<I92hdo1clkbp4g>; expiry: number };

type Ib8600fv9p32lu = {
  max_code_size: number;
  max_head_data_size: number;
  max_upward_queue_count: number;
  max_upward_queue_size: number;
  max_upward_message_size: number;
  max_upward_message_num_per_candidate: number;
  hrmp_max_message_num_per_candidate: number;
  validation_upgrade_cooldown: number;
  validation_upgrade_delay: number;
  async_backing_params: Anonymize<Iavuvfkop6318c>;
  max_pov_size: number;
  max_downward_message_size: number;
  hrmp_max_parachain_outbound_channels: number;
  hrmp_sender_deposit: bigint;
  hrmp_recipient_deposit: bigint;
  hrmp_channel_max_capacity: number;
  hrmp_channel_max_total_size: number;
  hrmp_max_parachain_inbound_channels: number;
  hrmp_channel_max_message_size: number;
  executor_params: Anonymize<I79n5nka727sg9>;
  code_retention_period: number;
  coretime_cores: number;
  on_demand_retries: number;
  on_demand_queue_max_size: number;
  on_demand_target_queue_utilization: number;
  on_demand_fee_variability: number;
  on_demand_base_fee: bigint;
  on_demand_ttl: number;
  group_rotation_frequency: number;
  paras_availability_period: number;
  scheduling_lookahead: number;
  max_validators_per_core: Anonymize<I4arjljr6dpflb>;
  max_validators: Anonymize<I4arjljr6dpflb>;
  dispute_period: number;
  dispute_post_conclusion_acceptance_period: number;
  no_show_slots: number;
  n_delay_tranches: number;
  zeroth_delay_tranche_width: number;
  needed_approvals: number;
  relay_vrf_modulo_samples: number;
  pvf_voting_ttl: number;
  minimum_validation_upgrade_delay: number;
  minimum_backing_votes: number;
  node_features: { bytes: Uint8Array; bitsLen: number };
  approval_voting_params: number;
};

type Iavuvfkop6318c = { max_candidate_depth: number; allowed_ancestry_len: number };

type I79n5nka727sg9 = Array<PolkadotPrimitivesV6ExecutorParamsExecutorParam>;

export type PolkadotPrimitivesV6ExecutorParamsExecutorParam = Enum<
  | { type: "MaxMemoryPages"; value: Anonymize<number> }
  | { type: "StackLogicalMax"; value: Anonymize<number> }
  | { type: "StackNativeMax"; value: Anonymize<number> }
  | { type: "PrecheckingMaxMemory"; value: Anonymize<bigint> }
  | { type: "PvfPrepTimeout"; value: Anonymize<I9jq69l21omt1q> }
  | { type: "PvfExecTimeout"; value: Anonymize<I3fsjjoj6u1h98> }
  | { type: "WasmExtBulkMemory"; value: undefined }
>;
export const PolkadotPrimitivesV6ExecutorParamsExecutorParam =
  _Enum as unknown as GetEnum<PolkadotPrimitivesV6ExecutorParamsExecutorParam>;

type I9jq69l21omt1q = [PolkadotPrimitivesV6PvfPrepKind, bigint];

export type PolkadotPrimitivesV6PvfPrepKind = Enum<
  { type: "Precheck"; value: undefined } | { type: "Prepare"; value: undefined }
>;
export const PolkadotPrimitivesV6PvfPrepKind =
  _Enum as unknown as GetEnum<PolkadotPrimitivesV6PvfPrepKind>;

type I3fsjjoj6u1h98 = [PolkadotPrimitivesV5PvfExecTimeoutKind, bigint];

export type PolkadotPrimitivesV5PvfExecTimeoutKind = Enum<
  { type: "Backing"; value: undefined } | { type: "Approval"; value: undefined }
>;
export const PolkadotPrimitivesV5PvfExecTimeoutKind =
  _Enum as unknown as GetEnum<PolkadotPrimitivesV5PvfExecTimeoutKind>;

type I2rsa7njg1k6v1 = Array<Anonymize<If59lb4asqkhca>>;

type If59lb4asqkhca = [number, Anonymize<Ib8600fv9p32lu>];

type I663ddds93n8hc = { buffer: Anonymize<Idm6djpsp7gtcm>; latest_number: number };

type Idm6djpsp7gtcm = Array<Anonymize<Idc5h3lsmctk6r>>;

type Idc5h3lsmctk6r = [Binary, Binary];

type I78e99376i3249 = { bitfield: { bytes: Uint8Array; bitsLen: number }; submitted_at: number };

type Ic4ms979ng98e4 = {
  core: number;
  hash: Binary;
  descriptor: Anonymize<Ib2u20s6roco9i>;
  availability_votes: { bytes: Uint8Array; bitsLen: number };
  backers: { bytes: Uint8Array; bitsLen: number };
  relay_parent_number: number;
  backed_in_number: number;
  backing_group: number;
};

type Ic1d4u2opv3fst = {
  upward_messages: Anonymize<Itom7fk49o0c9>;
  horizontal_messages: Anonymize<I6r5cbv8ttrb09>;
  new_validation_code: Anonymize<Iabpgqcjikia83>;
  head_data: Binary;
  processed_downward_messages: number;
  hrmp_watermark: number;
};

type Itom7fk49o0c9 = Array<Binary>;

type I6r5cbv8ttrb09 = Array<Anonymize<I958l48g4qg5rf>>;

type I958l48g4qg5rf = { recipient: number; data: Binary };

type Ie7c40q4va8u7r = {
  session: number;
  backing_validators_per_candidate: Anonymize<Ibabtlc0psj69a>;
  disputes: Anonymize<Ib08u599nb52js>;
};

type Ibabtlc0psj69a = Array<Anonymize<I2sj9sob0g74s5>>;

type I2sj9sob0g74s5 = [Anonymize<I4vjld3472quct>, Anonymize<Icl1tha16t1g7m>];

type Icl1tha16t1g7m = Array<Anonymize<I63c844nk8i073>>;

type I63c844nk8i073 = [number, PolkadotPrimitivesV5ValidityAttestation];

export type PolkadotPrimitivesV5ValidityAttestation = Enum<
  { type: "Implicit"; value: Anonymize<Binary> } | { type: "Explicit"; value: Anonymize<Binary> }
>;
export const PolkadotPrimitivesV5ValidityAttestation =
  _Enum as unknown as GetEnum<PolkadotPrimitivesV5ValidityAttestation>;

type Ib08u599nb52js = Array<Anonymize<I79rvm2gfh6orv>>;

type I79rvm2gfh6orv = {
  candidate_hash: Binary;
  session: number;
  statements: Anonymize<Ian2b8h9fhvtli>;
};

type Ian2b8h9fhvtli = Array<Anonymize<I9egl7bc3isu9u>>;

type I9egl7bc3isu9u = [PolkadotPrimitivesV6DisputeStatement, number, Binary];

export type PolkadotPrimitivesV6DisputeStatement = Enum<
  | { type: "Valid"; value: Anonymize<PolkadotPrimitivesV6ValidDisputeStatementKind> }
  | { type: "Invalid"; value: Anonymize<PolkadotPrimitivesV5InvalidDisputeStatementKind> }
>;
export const PolkadotPrimitivesV6DisputeStatement =
  _Enum as unknown as GetEnum<PolkadotPrimitivesV6DisputeStatement>;

export type PolkadotPrimitivesV6ValidDisputeStatementKind = Enum<
  | { type: "Explicit"; value: undefined }
  | { type: "BackingSeconded"; value: Anonymize<Binary> }
  | { type: "BackingValid"; value: Anonymize<Binary> }
  | { type: "ApprovalChecking"; value: undefined }
  | { type: "ApprovalCheckingMultipleCandidates"; value: Anonymize<Idhnf6rtqoslea> }
>;
export const PolkadotPrimitivesV6ValidDisputeStatementKind =
  _Enum as unknown as GetEnum<PolkadotPrimitivesV6ValidDisputeStatementKind>;

export type PolkadotPrimitivesV5InvalidDisputeStatementKind = Enum<{
  type: "Explicit";
  value: undefined;
}>;
export const PolkadotPrimitivesV5InvalidDisputeStatementKind =
  _Enum as unknown as GetEnum<PolkadotPrimitivesV5InvalidDisputeStatementKind>;

type Iarlj3qd8u1v13 = Array<Anonymize<Icgljjb6j82uhn>>;

type I9gvbjb5fb4hhm = Array<PolkadotRuntimeParachainsSchedulerPalletCoreOccupied>;

export type PolkadotRuntimeParachainsSchedulerPalletCoreOccupied = Enum<
  { type: "Free"; value: undefined } | { type: "Paras"; value: Anonymize<I56tdvk9bkbdga> }
>;
export const PolkadotRuntimeParachainsSchedulerPalletCoreOccupied =
  _Enum as unknown as GetEnum<PolkadotRuntimeParachainsSchedulerPalletCoreOccupied>;

type I56tdvk9bkbdga = {
  assignment: PolkadotRuntimeParachainsSchedulerCommonAssignment;
  availability_timeouts: number;
  ttl: number;
};

export type PolkadotRuntimeParachainsSchedulerCommonAssignment = Enum<
  { type: "Pool"; value: Anonymize<I51g4s1s9hjmnm> } | { type: "Bulk"; value: Anonymize<number> }
>;
export const PolkadotRuntimeParachainsSchedulerCommonAssignment =
  _Enum as unknown as GetEnum<PolkadotRuntimeParachainsSchedulerCommonAssignment>;

type I51g4s1s9hjmnm = { para_id: number; core_index: number };

type Ic136fqousb794 = Array<Anonymize<Iekknq5gev7jf7>>;

type Iekknq5gev7jf7 = [number, Anonymize<I7p6q0rlaqosnu>];

type I7p6q0rlaqosnu = Array<Anonymize<I56tdvk9bkbdga>>;

type Iaja1oeag5u4b3 = {
  votes_accept: { bytes: Uint8Array; bitsLen: number };
  votes_reject: { bytes: Uint8Array; bitsLen: number };
  age: number;
  created_at: number;
  causes: Anonymize<Ia0a9586d8d811>;
};

type Ia0a9586d8d811 = Array<PolkadotRuntimeParachainsParasPvfCheckCause>;

export type PolkadotRuntimeParachainsParasPvfCheckCause = Enum<
  | { type: "Onboarding"; value: Anonymize<number> }
  | { type: "Upgrade"; value: Anonymize<Ic9rr4c78iumk2> }
>;
export const PolkadotRuntimeParachainsParasPvfCheckCause =
  _Enum as unknown as GetEnum<PolkadotRuntimeParachainsParasPvfCheckCause>;

type Ic9rr4c78iumk2 = { id: number; included_at: number; set_go_ahead: DispatchPays };

export type ParachainsParasParaLifecycle = Enum<
  | { type: "Onboarding"; value: undefined }
  | { type: "Parathread"; value: undefined }
  | { type: "Parachain"; value: undefined }
  | { type: "UpgradingParathread"; value: undefined }
  | { type: "DowngradingParachain"; value: undefined }
  | { type: "OffboardingParathread"; value: undefined }
  | { type: "OffboardingParachain"; value: undefined }
>;
export const ParachainsParasParaLifecycle =
  _Enum as unknown as GetEnum<ParachainsParasParaLifecycle>;

type I79cs1p3m59mo7 = {
  upgrade_times: Anonymize<I2v6n2k262gqsq>;
  last_pruned: Anonymize<I4arjljr6dpflb>;
};

type I2v6n2k262gqsq = Array<Anonymize<Ioham9r6hhu19>>;

type Ioham9r6hhu19 = { expected_at: number; activated_at: number };

export type PolkadotPrimitivesV5UpgradeGoAhead = Enum<
  { type: "Abort"; value: undefined } | { type: "GoAhead"; value: undefined }
>;
export const PolkadotPrimitivesV5UpgradeGoAhead =
  _Enum as unknown as GetEnum<PolkadotPrimitivesV5UpgradeGoAhead>;

export type PolkadotPrimitivesV5UpgradeRestriction = Enum<{ type: "Present"; value: undefined }>;
export const PolkadotPrimitivesV5UpgradeRestriction =
  _Enum as unknown as GetEnum<PolkadotPrimitivesV5UpgradeRestriction>;

type I2duhnt686rv0q = { genesis_head: Binary; validation_code: Binary; para_kind: boolean };

type I14hoekog5s49k = Array<Anonymize<I36mfku1ea0i8t>>;

type I36mfku1ea0i8t = {
  validators: Anonymize<Idhnf6rtqoslea>;
  queued: Anonymize<Idhnf6rtqoslea>;
  session_index: number;
};

type I6ljjd4b5fa4ov = Array<Anonymize<I60847k37jfcc6>>;

type I60847k37jfcc6 = { sent_at: number; msg: Binary };

type Ibhmrlkcu01imb = {
  confirmed: boolean;
  _age: number;
  sender_deposit: bigint;
  max_message_size: number;
  max_capacity: number;
  max_total_size: number;
};

type Id43g4eveajpkl = Array<Anonymize<I50mrcbubp554e>>;

type I6u4labiv1bntm = {
  max_capacity: number;
  max_total_size: number;
  max_message_size: number;
  msg_count: number;
  total_size: number;
  mqc_head: Anonymize<I17k3ujudqd5df>;
  sender_deposit: bigint;
  recipient_deposit: bigint;
};

type Iev3u09i2vqn93 = Array<Anonymize<I409qo0sfkbh16>>;

type I409qo0sfkbh16 = { sent_at: number; data: Binary };

type I9olhgo2o08h7b = Array<Anonymize<I8pg2rpr4ldgp9>>;

type I8pg2rpr4ldgp9 = [number, Anonymize<Icgljjb6j82uhn>];

type I7k9oi9p83j43l = {
  active_validator_indices: Anonymize<Icgljjb6j82uhn>;
  random_seed: Binary;
  dispute_period: number;
  validators: Anonymize<Idhnf6rtqoslea>;
  discovery_keys: Anonymize<Idhnf6rtqoslea>;
  assignment_keys: Anonymize<Idhnf6rtqoslea>;
  validator_groups: Anonymize<Iarlj3qd8u1v13>;
  n_cores: number;
  zeroth_delay_tranche_width: number;
  relay_vrf_modulo_samples: number;
  n_delay_tranches: number;
  no_show_slots: number;
  needed_approvals: number;
};

type I87u7jalc0lhah = {
  validators_for: { bytes: Uint8Array; bitsLen: number };
  validators_against: { bytes: Uint8Array; bitsLen: number };
  start: number;
  concluded_at: Anonymize<I4arjljr6dpflb>;
};

type If89923vhoiaim = [number, Binary];

type I3g2jv3qmtkrbe = {
  keys: Anonymize<I93ssha9egqq23>;
  kind: PolkadotPrimitivesV5SlashingSlashingOffenceKind;
};

type I93ssha9egqq23 = Array<Anonymize<If89923vhoiaim>>;

export type PolkadotPrimitivesV5SlashingSlashingOffenceKind = Enum<
  { type: "ForInvalid"; value: undefined } | { type: "AgainstValid"; value: undefined }
>;
export const PolkadotPrimitivesV5SlashingSlashingOffenceKind =
  _Enum as unknown as GetEnum<PolkadotPrimitivesV5SlashingSlashingOffenceKind>;

type Ic9ppo8st33eqb = {
  begin: number;
  end: number;
  count: number;
  ready_neighbours: Anonymize<Iav3cdf9g9n9fp>;
  message_count: bigint;
  size: bigint;
};

type Iav3cdf9g9n9fp = Anonymize<I4b0p76ud6qst3> | undefined;

type I4b0p76ud6qst3 = {
  prev: ParachainsInclusionAggregateMessageOrigin;
  next: ParachainsInclusionAggregateMessageOrigin;
};

type I53esa2ms463bk = {
  remaining: number;
  remaining_size: number;
  first_index: number;
  first: number;
  last: number;
  heap: Binary;
};

type Ibg270mmpf1f2q = [ParachainsInclusionAggregateMessageOrigin, number];

type Idgino88phkfa9 = { core_idx: number; count: number };

type I5ofut0ah1losi = {
  assignments: Anonymize<Id1dkuivtmc10j>;
  end_hint: Anonymize<I4arjljr6dpflb>;
  next_schedule: Anonymize<I4arjljr6dpflb>;
};

type Id1dkuivtmc10j = Array<Anonymize<Idkhbpjddkhhoj>>;

type Idkhbpjddkhhoj = [BrokerCoretimeInterfaceCoreAssignment, number];

export type BrokerCoretimeInterfaceCoreAssignment = Enum<
  | { type: "Idle"; value: undefined }
  | { type: "Pool"; value: undefined }
  | { type: "Task"; value: Anonymize<number> }
>;
export const BrokerCoretimeInterfaceCoreAssignment =
  _Enum as unknown as GetEnum<BrokerCoretimeInterfaceCoreAssignment>;

type I2kcol1jg0edkq = { queue: Anonymize<If4sm31aq7v0i8>; current_work: Anonymize<I154iog8qmouro> };

type If4sm31aq7v0i8 = Anonymize<Icjkaf41q9ga6t> | undefined;

type Icjkaf41q9ga6t = { first: number; last: number };

type I154iog8qmouro = Anonymize<Idtlivoehu7g94> | undefined;

type Idtlivoehu7g94 = {
  assignments: Anonymize<I2llo3fkue6vs3>;
  end_hint: Anonymize<I4arjljr6dpflb>;
  pos: number;
  step: number;
};

type I2llo3fkue6vs3 = Array<Anonymize<I6n90i6ucc4fn1>>;

type I6n90i6ucc4fn1 = [BrokerCoretimeInterfaceCoreAssignment, Anonymize<Ifrejtuk4pctgo>];

type Ifrejtuk4pctgo = { ratio: number; remaining: number };

type I3av628q6dt6mq = { manager: SS58String; deposit: bigint; locked: Anonymize<I8ie0dco0kcuq5> };

type I8ie0dco0kcuq5 = boolean | undefined;

type Ifmaahl40gom3g = Array<Anonymize<I92hdo1clkbp4g>>;

type I70iuri2ilha1f = Array<Anonymize<I1qlf98109qt29>>;

type I1qlf98109qt29 = Anonymize<I7fcree6lak6uv> | undefined;

type I7fcree6lak6uv = [SS58String, number, bigint];

type I2neqklu8gtd27 = {
  depositor: SS58String;
  verifier: Anonymize<I8t18p6mokc3s4>;
  deposit: bigint;
  raised: bigint;
  end: number;
  cap: bigint;
  last_contribution: CommonCrowdloanLastContribution;
  first_period: number;
  last_period: number;
  fund_index: number;
};

type I8t18p6mokc3s4 = MultiSigner | undefined;

export type MultiSigner = Enum<
  | { type: "Ed25519"; value: Anonymize<Binary> }
  | { type: "Sr25519"; value: Anonymize<Binary> }
  | { type: "Ecdsa"; value: Anonymize<Binary> }
>;
export const MultiSigner = _Enum as unknown as GetEnum<MultiSigner>;

export type CommonCrowdloanLastContribution = Enum<
  | { type: "Never"; value: undefined }
  | { type: "PreEnding"; value: Anonymize<number> }
  | { type: "Ending"; value: Anonymize<number> }
>;
export const CommonCrowdloanLastContribution =
  _Enum as unknown as GetEnum<CommonCrowdloanLastContribution>;

export type XcmPalletQueryStatus = Enum<
  | { type: "Pending"; value: Anonymize<Iavvrc5pg62eqm> }
  | { type: "VersionNotifier"; value: Anonymize<I77chjqda69096> }
  | { type: "Ready"; value: Anonymize<I7knkdachsejp6> }
>;
export const XcmPalletQueryStatus = _Enum as unknown as GetEnum<XcmPalletQueryStatus>;

type Iavvrc5pg62eqm = {
  responder: XcmVersionedLocation;
  maybe_match_querier: Anonymize<Iauoeri1jolror>;
  maybe_notify: Anonymize<I34gtdjipdmjpt>;
  timeout: number;
};

type Iauoeri1jolror = XcmVersionedLocation | undefined;

type I77chjqda69096 = { origin: XcmVersionedLocation; is_active: boolean };

type I7knkdachsejp6 = { response: XcmVersionedResponse; at: number };

export type XcmVersionedResponse = Enum<
  | { type: "V2"; value: Anonymize<XcmV2Response> }
  | { type: "V3"; value: Anonymize<XcmV3Response> }
  | { type: "V4"; value: Anonymize<XcmV4Response> }
>;
export const XcmVersionedResponse = _Enum as unknown as GetEnum<XcmVersionedResponse>;

export type XcmV2Response = Enum<
  | { type: "Null"; value: undefined }
  | { type: "Assets"; value: Anonymize<Ia3ggl9eghkufh> }
  | { type: "ExecutionResult"; value: Anonymize<I17i9gqt27hetc> }
  | { type: "Version"; value: Anonymize<number> }
>;
export const XcmV2Response = _Enum as unknown as GetEnum<XcmV2Response>;

type I17i9gqt27hetc = Anonymize<I8l8ileau3j9jv> | undefined;

type I8l8ileau3j9jv = [number, XcmV2TraitsError];

export type XcmV2TraitsError = Enum<
  | { type: "Overflow"; value: undefined }
  | { type: "Unimplemented"; value: undefined }
  | { type: "UntrustedReserveLocation"; value: undefined }
  | { type: "UntrustedTeleportLocation"; value: undefined }
  | { type: "MultiLocationFull"; value: undefined }
  | { type: "MultiLocationNotInvertible"; value: undefined }
  | { type: "BadOrigin"; value: undefined }
  | { type: "InvalidLocation"; value: undefined }
  | { type: "AssetNotFound"; value: undefined }
  | { type: "FailedToTransactAsset"; value: undefined }
  | { type: "NotWithdrawable"; value: undefined }
  | { type: "LocationCannotHold"; value: undefined }
  | { type: "ExceedsMaxMessageSize"; value: undefined }
  | { type: "DestinationUnsupported"; value: undefined }
  | { type: "Transport"; value: undefined }
  | { type: "Unroutable"; value: undefined }
  | { type: "UnknownClaim"; value: undefined }
  | { type: "FailedToDecode"; value: undefined }
  | { type: "MaxWeightInvalid"; value: undefined }
  | { type: "NotHoldingFees"; value: undefined }
  | { type: "TooExpensive"; value: undefined }
  | { type: "Trap"; value: Anonymize<bigint> }
  | { type: "UnhandledXcmVersion"; value: undefined }
  | { type: "WeightLimitReached"; value: Anonymize<bigint> }
  | { type: "Barrier"; value: undefined }
  | { type: "WeightNotComputable"; value: undefined }
>;
export const XcmV2TraitsError = _Enum as unknown as GetEnum<XcmV2TraitsError>;

export type XcmV3Response = Enum<
  | { type: "Null"; value: undefined }
  | { type: "Assets"; value: Anonymize<I2pdjq1umlp617> }
  | { type: "ExecutionResult"; value: Anonymize<I8j770n2arfq59> }
  | { type: "Version"; value: Anonymize<number> }
  | { type: "PalletsInfo"; value: Anonymize<I599u7h20b52at> }
  | { type: "DispatchResult"; value: Anonymize<XcmV3MaybeErrorCode> }
>;
export const XcmV3Response = _Enum as unknown as GetEnum<XcmV3Response>;

type I7d8h5fojckk24 = [number, XcmVersionedLocation];

type I7vlvrrl2pnbgk = [bigint, Anonymize<I4q39t5hn830vp>, number];

type I6g82vlc185ggp = Array<Anonymize<Iak31q8r4mqtse>>;

type Iak31q8r4mqtse = [XcmVersionedLocation, number];

export type XcmPalletVersionMigrationStage = Enum<
  | { type: "MigrateSupportedVersion"; value: undefined }
  | { type: "MigrateVersionNotifiers"; value: undefined }
  | { type: "NotifyCurrentTargets"; value: Anonymize<Iabpgqcjikia83> }
  | { type: "MigrateAndNotifyOldTargets"; value: undefined }
>;
export const XcmPalletVersionMigrationStage =
  _Enum as unknown as GetEnum<XcmPalletVersionMigrationStage>;

type I3if777simg78u = {
  amount: bigint;
  owner: XcmVersionedLocation;
  locker: XcmVersionedLocation;
  consumers: Anonymize<I48jka0f0ufl6q>;
};

type I48jka0f0ufl6q = Array<Anonymize<I2jndntq8n8661>>;

type I2jndntq8n8661 = [undefined, bigint];

type Idi9lki0fmepj5 = [number, SS58String, XcmVersionedAssetId];

export type XcmVersionedAssetId = Enum<
  | { type: "V3"; value: Anonymize<XcmV3MultiassetAssetId> }
  | { type: "V4"; value: Anonymize<Ieq428hdmibf6m> }
>;
export const XcmVersionedAssetId = _Enum as unknown as GetEnum<XcmVersionedAssetId>;

type Iafu2aq0kg568j = Array<Anonymize<I5pu2dsc6d3iv6>>;

type I5pu2dsc6d3iv6 = [bigint, XcmVersionedLocation];

type Ie5mvl0hn85mkc = Array<Binary>;

type I3qm5u05tioq05 = { id: bigint; len: number; keyset_commitment: Binary };

type I6ucbdbrsslk4l = {
  manager: SS58String;
  period_begin: number;
  period_count: number;
  last_lease: Anonymize<I4arjljr6dpflb>;
  lease_count: number;
};

type I2sqgqj7ib3ine = {
  progress_top: Anonymize<I2jvfaenrpg1ai>;
  progress_child: Anonymize<I2jvfaenrpg1ai>;
  size: number;
  top_items: number;
  child_items: number;
};

type I2jvfaenrpg1ai = AnonymousEnum<{
  ToStart: undefined;
  LastKey: Anonymize<Binary>;
  Complete: undefined;
}>;

type Ib17t3992hb64n = Anonymize<I215mkl885p4da> | undefined;

type I215mkl885p4da = { size: number; item: number };

type In7a38730s6qs = {
  base_block: Anonymize<I4q39t5hn830vp>;
  max_block: Anonymize<I4q39t5hn830vp>;
  per_class: Anonymize<I79te2qqsklnbd>;
};

type I79te2qqsklnbd = {
  normal: Anonymize<Ia78ef0a3p5958>;
  operational: Anonymize<Ia78ef0a3p5958>;
  mandatory: Anonymize<Ia78ef0a3p5958>;
};

type Ia78ef0a3p5958 = {
  base_extrinsic: Anonymize<I4q39t5hn830vp>;
  max_extrinsic: Anonymize<Iasb8k6ash5mjn>;
  max_total: Anonymize<Iasb8k6ash5mjn>;
  reserved: Anonymize<Iasb8k6ash5mjn>;
};

type If15el53dd76v9 = { normal: number; operational: number; mandatory: number };

type I9s0ave7t0vnrk = { read: bigint; write: bigint };

type Ib5fo0fakl8rog = {
  spec_name: string;
  impl_name: string;
  authoring_version: number;
  spec_version: number;
  impl_version: number;
  apis: Anonymize<I1st1p92iu8h7e>;
  transaction_version: number;
  state_version: number;
};

type I1st1p92iu8h7e = Array<Anonymize<If6q1i5gkbpmkc>>;

type If6q1i5gkbpmkc = [Binary, number];

type I35p85j063s0il = bigint | undefined;

type Ibafpkl9hhno69 = Array<Anonymize<Ida9vhl30l98p4>>;

type Ida9vhl30l98p4 = [number, Anonymize<I6s1tg2sl5nvmp>];

type I6s1tg2sl5nvmp = {
  name: string;
  max_deciding: number;
  decision_deposit: bigint;
  prepare_period: number;
  decision_period: number;
  confirm_period: number;
  min_enactment_period: number;
  min_approval: ReferendaTypesCurve;
  min_support: ReferendaTypesCurve;
};

export type ReferendaTypesCurve = Enum<
  | { type: "LinearDecreasing"; value: Anonymize<Idcpso832hml3u> }
  | { type: "SteppedDecreasing"; value: Anonymize<I5qiv0grkufa8l> }
  | { type: "Reciprocal"; value: Anonymize<I58l93su2gte4i> }
>;
export const ReferendaTypesCurve = _Enum as unknown as GetEnum<ReferendaTypesCurve>;

type Idcpso832hml3u = { length: number; floor: number; ceil: number };

type I5qiv0grkufa8l = { begin: number; end: number; step: number; period: number };

type I58l93su2gte4i = { factor: bigint; x_offset: bigint; y_offset: bigint };

export type PalletCall = Enum<
  | { type: "remark"; value: Anonymize<I8ofcg5rbj0g2c> }
  | { type: "set_heap_pages"; value: Anonymize<I4adgbll7gku4i> }
  | { type: "set_code"; value: Anonymize<I6pjjpfvhvcfru> }
  | { type: "set_code_without_checks"; value: Anonymize<I6pjjpfvhvcfru> }
  | { type: "set_storage"; value: Anonymize<I8qrhskdehbu57> }
  | { type: "kill_storage"; value: Anonymize<I39uah9nss64h9> }
  | { type: "kill_prefix"; value: Anonymize<Ik64dknsq7k08> }
  | { type: "remark_with_event"; value: Anonymize<I8ofcg5rbj0g2c> }
  | { type: "authorize_upgrade"; value: Anonymize<I6a5n5ij3gomuo> }
  | { type: "authorize_upgrade_without_checks"; value: Anonymize<I6a5n5ij3gomuo> }
  | { type: "apply_authorized_upgrade"; value: Anonymize<I6pjjpfvhvcfru> }
>;
export const PalletCall = _Enum as unknown as GetEnum<PalletCall>;

type I8ofcg5rbj0g2c = { remark: Binary };

type I4adgbll7gku4i = { pages: bigint };

type I6pjjpfvhvcfru = { code: Binary };

type I8qrhskdehbu57 = { items: Anonymize<I5g1ftt6bt65bl> };

type I5g1ftt6bt65bl = Array<Anonymize<Ief9tkec59fktv>>;

type Ief9tkec59fktv = [Binary, Binary];

type I39uah9nss64h9 = { keys: Anonymize<Itom7fk49o0c9> };

type Ik64dknsq7k08 = { prefix: Binary; subkeys: number };

type I6a5n5ij3gomuo = { code_hash: Binary };

export type BabePalletCall = Enum<
  | { type: "report_equivocation"; value: Anonymize<I7mmbgd20nut80> }
  | { type: "report_equivocation_unsigned"; value: Anonymize<I7mmbgd20nut80> }
  | { type: "plan_config_change"; value: Anonymize<I2dcpbss9027dl> }
>;
export const BabePalletCall = _Enum as unknown as GetEnum<BabePalletCall>;

type I7mmbgd20nut80 = {
  equivocation_proof: Anonymize<I7bek4s9acs8nl>;
  key_owner_proof: Anonymize<I3ia7aufsoj0l1>;
};

type I7bek4s9acs8nl = {
  offender: Binary;
  slot: bigint;
  first_header: Anonymize<I6t1nedlt7mobn>;
  second_header: Anonymize<I6t1nedlt7mobn>;
};

type I6t1nedlt7mobn = {
  parent_hash: Binary;
  number: number;
  state_root: Binary;
  extrinsics_root: Binary;
  digest: Anonymize<Idin6nhq46lvdj>;
};

type I3ia7aufsoj0l1 = {
  session: number;
  trie_nodes: Anonymize<Itom7fk49o0c9>;
  validator_count: number;
};

type I2dcpbss9027dl = { config: BabeDigestsNextConfigDescriptor };

export type TimestampPalletCall = Enum<{ type: "set"; value: Anonymize<Idcr6u6361oad9> }>;
export const TimestampPalletCall = _Enum as unknown as GetEnum<TimestampPalletCall>;

type Idcr6u6361oad9 = { now: bigint };

export type IndicesPalletCall = Enum<
  | { type: "claim"; value: Anonymize<I666bl2fqjkejo> }
  | { type: "transfer"; value: Anonymize<Idge7gk9m5car0> }
  | { type: "free"; value: Anonymize<I666bl2fqjkejo> }
  | { type: "force_transfer"; value: Anonymize<I34pbimt2i69v7> }
  | { type: "freeze"; value: Anonymize<I666bl2fqjkejo> }
>;
export const IndicesPalletCall = _Enum as unknown as GetEnum<IndicesPalletCall>;

type Idge7gk9m5car0 = { index: number; new: MultiAddress };

export type MultiAddress = Enum<
  | { type: "Id"; value: Anonymize<SS58String> }
  | { type: "Index"; value: Anonymize<number> }
  | { type: "Raw"; value: Anonymize<Binary> }
  | { type: "Address32"; value: Anonymize<Binary> }
  | { type: "Address20"; value: Anonymize<Binary> }
>;
export const MultiAddress = _Enum as unknown as GetEnum<MultiAddress>;

type I34pbimt2i69v7 = { new: MultiAddress; index: number; freeze: boolean };

export type BalancesPalletCall = Enum<
  | { type: "transfer_allow_death"; value: Anonymize<Ien6q0lasi0m7i> }
  | { type: "force_transfer"; value: Anonymize<Icacgruoo9j3r2> }
  | { type: "transfer_keep_alive"; value: Anonymize<Ien6q0lasi0m7i> }
  | { type: "transfer_all"; value: Anonymize<I7dgmo7im9hljo> }
  | { type: "force_unreserve"; value: Anonymize<Iargojp1sv9icj> }
  | { type: "upgrade_accounts"; value: Anonymize<Ibmr18suc9ikh9> }
  | { type: "force_set_balance"; value: Anonymize<Ie0io91hk7pejj> }
  | { type: "force_adjust_total_issuance"; value: Anonymize<I3u7ci02rtsn3o> }
>;
export const BalancesPalletCall = _Enum as unknown as GetEnum<BalancesPalletCall>;

type Ien6q0lasi0m7i = { dest: MultiAddress; value: bigint };

type Icacgruoo9j3r2 = { source: MultiAddress; dest: MultiAddress; value: bigint };

type I7dgmo7im9hljo = { dest: MultiAddress; keep_alive: boolean };

type Iargojp1sv9icj = { who: MultiAddress; amount: bigint };

type Ibmr18suc9ikh9 = { who: Anonymize<Ia2lhg7l2hilo3> };

type Ie0io91hk7pejj = { who: MultiAddress; new_free: bigint };

type I3u7ci02rtsn3o = { direction: BalancesTypesAdjustmentDirection; delta: bigint };

export type BalancesTypesAdjustmentDirection = Enum<
  { type: "Increase"; value: undefined } | { type: "Decrease"; value: undefined }
>;
export const BalancesTypesAdjustmentDirection =
  _Enum as unknown as GetEnum<BalancesTypesAdjustmentDirection>;

export type SessionPalletCall = Enum<
  { type: "set_keys"; value: Anonymize<Idl8sn1uiguh1s> } | { type: "purge_keys"; value: undefined }
>;
export const SessionPalletCall = _Enum as unknown as GetEnum<SessionPalletCall>;

type Idl8sn1uiguh1s = { keys: Anonymize<Ieup5s9ka3brpi>; proof: Binary };

export type GrandpaPalletCall = Enum<
  | { type: "report_equivocation"; value: Anonymize<I4kjek1q6rj24q> }
  | { type: "report_equivocation_unsigned"; value: Anonymize<I4kjek1q6rj24q> }
  | { type: "note_stalled"; value: Anonymize<I2hviml3snvhhn> }
>;
export const GrandpaPalletCall = _Enum as unknown as GetEnum<GrandpaPalletCall>;

type I4kjek1q6rj24q = {
  equivocation_proof: Anonymize<I95fr9lj1pb8v8>;
  key_owner_proof: Anonymize<I3ia7aufsoj0l1>;
};

type I95fr9lj1pb8v8 = { set_id: bigint; equivocation: GrandpaEquivocation };

export type GrandpaEquivocation = Enum<
  | { type: "Prevote"; value: Anonymize<Igd938ojs7e2l> }
  | { type: "Precommit"; value: Anonymize<Igd938ojs7e2l> }
>;
export const GrandpaEquivocation = _Enum as unknown as GetEnum<GrandpaEquivocation>;

type Igd938ojs7e2l = {
  round_number: bigint;
  identity: Binary;
  first: Anonymize<I8f8rup5807mac>;
  second: Anonymize<I8f8rup5807mac>;
};

type I8f8rup5807mac = [Anonymize<I11vulp7gu5007>, Binary];

type I11vulp7gu5007 = { target_hash: Binary; target_number: number };

type I2hviml3snvhhn = { delay: number; best_finalized_block_number: number };

export type TreasuryPalletCall = Enum<
  | { type: "propose_spend"; value: Anonymize<I5c883qnnqciv8> }
  | { type: "reject_proposal"; value: Anonymize<Icm9m0qeemu66d> }
  | { type: "approve_proposal"; value: Anonymize<Icm9m0qeemu66d> }
  | { type: "spend_local"; value: Anonymize<Idpn74s0i9cdvp> }
  | { type: "remove_approval"; value: Anonymize<Icm9m0qeemu66d> }
  | { type: "spend"; value: Anonymize<I7vlce2m9b88vv> }
  | { type: "payout"; value: Anonymize<I666bl2fqjkejo> }
  | { type: "check_status"; value: Anonymize<I666bl2fqjkejo> }
  | { type: "void_spend"; value: Anonymize<I666bl2fqjkejo> }
>;
export const TreasuryPalletCall = _Enum as unknown as GetEnum<TreasuryPalletCall>;

type I5c883qnnqciv8 = { value: bigint; beneficiary: MultiAddress };

type Icm9m0qeemu66d = { proposal_id: number };

type Idpn74s0i9cdvp = { amount: bigint; beneficiary: MultiAddress };

type I7vlce2m9b88vv = {
  asset_kind: PolkadotRuntimeCommonImplsVersionedLocatableAsset;
  amount: bigint;
  beneficiary: XcmVersionedLocation;
  valid_from: Anonymize<I4arjljr6dpflb>;
};

export type ConvictionVotingPalletCall = Enum<
  | { type: "vote"; value: Anonymize<Idnsr2pndm36h0> }
  | { type: "delegate"; value: Anonymize<Id7ut33dljf52c> }
  | { type: "undelegate"; value: Anonymize<I8steo882k7qns> }
  | { type: "unlock"; value: Anonymize<I1vc8h4t228bot> }
  | { type: "remove_vote"; value: Anonymize<I5f178ab6b89t3> }
  | { type: "remove_other_vote"; value: Anonymize<I5fak1u82ohqtm> }
>;
export const ConvictionVotingPalletCall = _Enum as unknown as GetEnum<ConvictionVotingPalletCall>;

type Idnsr2pndm36h0 = { poll_index: number; vote: ConvictionVotingVoteAccountVote };

type Id7ut33dljf52c = {
  class: number;
  to: MultiAddress;
  conviction: VotingConviction;
  balance: bigint;
};

type I8steo882k7qns = { class: number };

type I1vc8h4t228bot = { class: number; target: MultiAddress };

type I5f178ab6b89t3 = { class: Anonymize<I4arjljr6dpflb>; index: number };

type I5fak1u82ohqtm = { target: MultiAddress; class: number; index: number };

export type ReferendaPalletCall = Enum<
  | { type: "submit"; value: Anonymize<I8lsplvtf2fhfc> }
  | { type: "place_decision_deposit"; value: Anonymize<I666bl2fqjkejo> }
  | { type: "refund_decision_deposit"; value: Anonymize<I666bl2fqjkejo> }
  | { type: "cancel"; value: Anonymize<I666bl2fqjkejo> }
  | { type: "kill"; value: Anonymize<I666bl2fqjkejo> }
  | { type: "nudge_referendum"; value: Anonymize<I666bl2fqjkejo> }
  | { type: "one_fewer_deciding"; value: Anonymize<Icbio0e1f0034b> }
  | { type: "refund_submission_deposit"; value: Anonymize<I666bl2fqjkejo> }
  | { type: "set_metadata"; value: Anonymize<Ifml0k0sf0mu2g> }
>;
export const ReferendaPalletCall = _Enum as unknown as GetEnum<ReferendaPalletCall>;

type I8lsplvtf2fhfc = {
  proposal_origin: WestendRuntimeOriginCaller;
  proposal: PreimagesBounded;
  enactment_moment: TraitsScheduleDispatchTime;
};

type Icbio0e1f0034b = { track: number };

type Ifml0k0sf0mu2g = { index: number; maybe_hash: Anonymize<I17k3ujudqd5df> };

type I4u3gasi51kja5 = AnonymousEnum<{
  add_member: Anonymize<I2nmm2fprtp4kl>;
  promote_member: Anonymize<I2nmm2fprtp4kl>;
  demote_member: Anonymize<I2nmm2fprtp4kl>;
  remove_member: Anonymize<I8c23ih8fpmjma>;
  vote: Anonymize<I8bvk21lpmah75>;
  cleanup_poll: Anonymize<I449n3riv6jbum>;
  exchange_member: Anonymize<I6nasb41qm5s5m>;
}>;

type I2nmm2fprtp4kl = { who: MultiAddress };

type I8c23ih8fpmjma = { who: MultiAddress; min_rank: number };

type I8bvk21lpmah75 = { poll: number; aye: boolean };

type I449n3riv6jbum = { poll_index: number; max: number };

type I6nasb41qm5s5m = { who: MultiAddress; new_who: MultiAddress };

type I2ckfpqesa6mom = AnonymousEnum<{
  whitelist_call: Anonymize<I8413rb6im3iko>;
  remove_whitelisted_call: Anonymize<I8413rb6im3iko>;
  dispatch_whitelisted_call: Anonymize<Id3s9pakjjc472>;
  dispatch_whitelisted_call_with_preimage: Anonymize<Ib6n6j9tk6ut3b>;
}>;

type Id3s9pakjjc472 = {
  call_hash: Binary;
  call_encoded_len: number;
  call_weight_witness: Anonymize<I4q39t5hn830vp>;
};

type Ib6n6j9tk6ut3b = { call: Anonymize<I5v7s5dm78b0mh> };

type I5v7s5dm78b0mh = AnonymousEnum<{
  System: Anonymize<PalletCall>;
  Babe: Anonymize<BabePalletCall>;
  Timestamp: Anonymize<TimestampPalletCall>;
  Indices: Anonymize<IndicesPalletCall>;
  Balances: Anonymize<BalancesPalletCall>;
  Session: Anonymize<SessionPalletCall>;
  Grandpa: Anonymize<GrandpaPalletCall>;
  Treasury: Anonymize<TreasuryPalletCall>;
  ConvictionVoting: Anonymize<ConvictionVotingPalletCall>;
  Referenda: Anonymize<ReferendaPalletCall>;
  FellowshipCollective: Anonymize<I4u3gasi51kja5>;
  FellowshipReferenda: Anonymize<ReferendaPalletCall>;
  Whitelist: Anonymize<I2ckfpqesa6mom>;
  Claims: Anonymize<ClaimsPalletCall>;
  Utility: Anonymize<Iev2pskfofprup>;
  Identity: Anonymize<IdentityPalletCall>;
  Society: Anonymize<I207299j9uhq1e>;
  Recovery: Anonymize<Ib5mv4uipibinu>;
  Vesting: Anonymize<VestingPalletCall>;
  Scheduler: Anonymize<I7i92s493unmka>;
  Proxy: Anonymize<I45j9knpag7cl4>;
  Multisig: Anonymize<I204r9i9c46u0l>;
  Preimage: Anonymize<PreimagePalletCall>;
  AssetRate: Anonymize<AssetRatePalletCall>;
  Bounties: Anonymize<BountiesPalletCall>;
  ChildBounties: Anonymize<ChildBountiesPalletCall>;
  Nis: Anonymize<I7bmrug49rorp1>;
  NisCounterpartBalances: Anonymize<BalancesPalletCall>;
  Configuration: Anonymize<PolkadotRuntimeParachainsConfigurationPalletCall>;
  ParasShared: Anonymize<undefined>;
  ParaInclusion: Anonymize<undefined>;
  ParaInherent: Anonymize<PolkadotRuntimeParachainsParasInherentPalletCall>;
  Paras: Anonymize<ParachainsParasPalletCall>;
  Initializer: Anonymize<ParachainsInitializerPalletCall>;
  Hrmp: Anonymize<PolkadotRuntimeParachainsHrmpPalletCall>;
  ParasDisputes: Anonymize<ParachainsDisputesPalletCall>;
  ParasSlashing: Anonymize<ParachainsDisputesSlashingPalletCall>;
  MessageQueue: Anonymize<MessageQueuePalletCall>;
  OnDemandAssignmentProvider: Anonymize<PolkadotRuntimeParachainsAssignerOnDemandPalletCall>;
  Registrar: Anonymize<CommonParasRegistrarPalletCall>;
  Slots: Anonymize<CommonSlotsPalletCall>;
  Auctions: Anonymize<CommonAuctionsPalletCall>;
  Crowdloan: Anonymize<CommonCrowdloanPalletCall>;
  Coretime: Anonymize<PolkadotRuntimeParachainsCoretimePalletCall>;
  XcmPallet: Anonymize<XcmPalletCall>;
  Beefy: Anonymize<BeefyPalletCall>;
  IdentityMigrator: Anonymize<PolkadotRuntimeCommonIdentityMigratorPalletCall>;
  ParasSudoWrapper: Anonymize<PolkadotRuntimeCommonParasSudoWrapperPalletCall>;
  AssignedSlots: Anonymize<PolkadotRuntimeCommonAssignedSlotsPalletCall>;
  ValidatorManager: Anonymize<I8rur6c856tfev>;
  StateTrieMigration: Anonymize<Idu3no1672nmsk>;
  RootTesting: Anonymize<RootTestingPalletCall>;
  Sudo: Anonymize<I8aedjs0th135a>;
}>;

export type ClaimsPalletCall = Enum<
  | { type: "claim"; value: Anonymize<I1u3s4gbjnre15> }
  | { type: "mint_claim"; value: Anonymize<I20qiajmn4c5d4> }
  | { type: "claim_attest"; value: Anonymize<Ie3aplba76d794> }
  | { type: "attest"; value: Anonymize<I1ntko0oih7v1a> }
  | { type: "move_claim"; value: Anonymize<I193pigt6gtjff> }
>;
export const ClaimsPalletCall = _Enum as unknown as GetEnum<ClaimsPalletCall>;

type I1u3s4gbjnre15 = { dest: SS58String; ethereum_signature: Binary };

type I20qiajmn4c5d4 = {
  who: Binary;
  value: bigint;
  vesting_schedule: Anonymize<I70kqehrkegc98>;
  statement: Anonymize<I6rcr4im2g3gv9>;
};

type I70kqehrkegc98 = Anonymize<I2phecamkn3pej> | undefined;

type I6rcr4im2g3gv9 = ClaimsStatementKind | undefined;

type Ie3aplba76d794 = { dest: SS58String; ethereum_signature: Binary; statement: Binary };

type I1ntko0oih7v1a = { statement: Binary };

type I193pigt6gtjff = { old: Binary; new: Binary; maybe_preclaim: Anonymize<Ihfphjolmsqq1> };

type Iev2pskfofprup = AnonymousEnum<{
  batch: Anonymize<I1u2b1ro792h6p>;
  as_derivative: Anonymize<Icc61753r53l80>;
  batch_all: Anonymize<I1u2b1ro792h6p>;
  dispatch_as: Anonymize<Ife2vmvsk5r0gl>;
  force_batch: Anonymize<I1u2b1ro792h6p>;
  with_weight: Anonymize<Ie1uvn9e6ki8dd>;
}>;

type I1u2b1ro792h6p = { calls: Anonymize<I52mcb5hepi70g> };

type I52mcb5hepi70g = Array<Anonymize<I5v7s5dm78b0mh>>;

type Icc61753r53l80 = { index: number; call: Anonymize<I5v7s5dm78b0mh> };

type Ife2vmvsk5r0gl = { as_origin: WestendRuntimeOriginCaller; call: Anonymize<I5v7s5dm78b0mh> };

type Ie1uvn9e6ki8dd = { call: Anonymize<I5v7s5dm78b0mh>; weight: Anonymize<I4q39t5hn830vp> };

export type IdentityPalletCall = Enum<
  | { type: "add_registrar"; value: Anonymize<Ibsu2pfvipmui6> }
  | { type: "set_identity"; value: Anonymize<I621gpns74tp1f> }
  | { type: "set_subs"; value: Anonymize<I5100vdjbepcoj> }
  | { type: "clear_identity"; value: undefined }
  | { type: "request_judgement"; value: Anonymize<I9l2s4klu0831o> }
  | { type: "cancel_request"; value: Anonymize<I2ctrt5nqb8o7c> }
  | { type: "set_fee"; value: Anonymize<I711qahikocb1c> }
  | { type: "set_account_id"; value: Anonymize<Idge7gk9m5car0> }
  | { type: "set_fields"; value: Anonymize<Id6gojh30v9ib2> }
  | { type: "provide_judgement"; value: Anonymize<I2g5s5rvm0mfuf> }
  | { type: "kill_identity"; value: Anonymize<I29er5j74l8bu> }
  | { type: "add_sub"; value: Anonymize<Iclf5v4qsadc12> }
  | { type: "rename_sub"; value: Anonymize<Iclf5v4qsadc12> }
  | { type: "remove_sub"; value: Anonymize<Ifcc5t6ed1elfd> }
  | { type: "quit_sub"; value: undefined }
  | { type: "add_username_authority"; value: Anonymize<Ibb4j9hgamcrbc> }
  | { type: "remove_username_authority"; value: Anonymize<I8b1tfbsa65tdb> }
  | { type: "set_username_for"; value: Anonymize<I419rt0n0pbfjf> }
  | { type: "accept_username"; value: Anonymize<Ie5l999tf7t2te> }
  | { type: "remove_expired_approval"; value: Anonymize<Ie5l999tf7t2te> }
  | { type: "set_primary_username"; value: Anonymize<Ie5l999tf7t2te> }
  | { type: "remove_dangling_username"; value: Anonymize<Ie5l999tf7t2te> }
>;
export const IdentityPalletCall = _Enum as unknown as GetEnum<IdentityPalletCall>;

type Ibsu2pfvipmui6 = { account: MultiAddress };

type I621gpns74tp1f = { info: Anonymize<I939m6d6t3dsgm> };

type I5100vdjbepcoj = { subs: Anonymize<I47e5e4dh41s5v> };

type I47e5e4dh41s5v = Array<Anonymize<Ib22937a5d3pt0>>;

type I9l2s4klu0831o = { reg_index: number; max_fee: bigint };

type I2ctrt5nqb8o7c = { reg_index: number };

type I711qahikocb1c = { index: number; fee: bigint };

type Id6gojh30v9ib2 = { index: number; fields: bigint };

type I2g5s5rvm0mfuf = {
  reg_index: number;
  target: MultiAddress;
  judgement: IdentityJudgement;
  identity: Binary;
};

type I29er5j74l8bu = { target: MultiAddress };

type Iclf5v4qsadc12 = { sub: MultiAddress; data: IdentityTypesData };

type Ifcc5t6ed1elfd = { sub: MultiAddress };

type Ibb4j9hgamcrbc = { authority: MultiAddress; suffix: Binary; allocation: number };

type I8b1tfbsa65tdb = { authority: MultiAddress };

type I419rt0n0pbfjf = { who: MultiAddress; username: Binary; signature: Anonymize<I7us28h09nc5sv> };

type I7us28h09nc5sv = MultiSignature | undefined;

export type MultiSignature = Enum<
  | { type: "Ed25519"; value: Anonymize<Binary> }
  | { type: "Sr25519"; value: Anonymize<Binary> }
  | { type: "Ecdsa"; value: Anonymize<Binary> }
>;
export const MultiSignature = _Enum as unknown as GetEnum<MultiSignature>;

type Ie5l999tf7t2te = { username: Binary };

type I207299j9uhq1e = AnonymousEnum<{
  bid: Anonymize<Ie5v6njpckr05b>;
  unbid: undefined;
  vouch: Anonymize<I7tjn67p2i76ft>;
  unvouch: undefined;
  vote: Anonymize<I8263q56dd34vq>;
  defender_vote: Anonymize<I26anifrfnljoi>;
  payout: undefined;
  waive_repay: Anonymize<I3qt1hgg4djhgb>;
  found_society: Anonymize<Ig0li41oi4nn6>;
  dissolve: undefined;
  judge_suspended_member: Anonymize<I6jkhgb7bmmdqp>;
  set_parameters: Anonymize<I9hkvi4k3kh916>;
  punish_skeptic: undefined;
  claim_membership: undefined;
  bestow_membership: Anonymize<I4b66js88p45m8>;
  kick_candidate: Anonymize<I4b66js88p45m8>;
  resign_candidacy: undefined;
  drop_candidate: Anonymize<I4b66js88p45m8>;
  cleanup_candidacy: Anonymize<Ieh40op1lafir9>;
  cleanup_challenge: Anonymize<Icjp7f9u9fno72>;
}>;

type I7tjn67p2i76ft = { who: MultiAddress; value: bigint; tip: bigint };

type I8263q56dd34vq = { candidate: MultiAddress; approve: boolean };

type I26anifrfnljoi = { approve: boolean };

type Ig0li41oi4nn6 = {
  founder: MultiAddress;
  max_members: number;
  max_intake: number;
  max_strikes: number;
  candidate_deposit: bigint;
  rules: Binary;
};

type I6jkhgb7bmmdqp = { who: MultiAddress; forgive: boolean };

type Ieh40op1lafir9 = { candidate: SS58String; max: number };

type Icjp7f9u9fno72 = { challenge_round: number; max: number };

type Ib5mv4uipibinu = AnonymousEnum<{
  as_recovered: Anonymize<Ibkasvcbhle02a>;
  set_recovered: Anonymize<Ifddpcgiumovp6>;
  create_recovery: Anonymize<I6s6ihmfj6j5qq>;
  initiate_recovery: Anonymize<Ibsu2pfvipmui6>;
  vouch_recovery: Anonymize<Ifddpcgiumovp6>;
  claim_recovery: Anonymize<Ibsu2pfvipmui6>;
  close_recovery: Anonymize<I8p14f9e765cn8>;
  remove_recovery: undefined;
  cancel_recovered: Anonymize<Ibsu2pfvipmui6>;
}>;

type Ibkasvcbhle02a = { account: MultiAddress; call: Anonymize<I5v7s5dm78b0mh> };

type Ifddpcgiumovp6 = { lost: MultiAddress; rescuer: MultiAddress };

type I6s6ihmfj6j5qq = {
  friends: Anonymize<Ia2lhg7l2hilo3>;
  threshold: number;
  delay_period: number;
};

type I8p14f9e765cn8 = { rescuer: MultiAddress };

export type VestingPalletCall = Enum<
  | { type: "vest"; value: undefined }
  | { type: "vest_other"; value: Anonymize<I29er5j74l8bu> }
  | { type: "vested_transfer"; value: Anonymize<I9l9kkok4o3ekh> }
  | { type: "force_vested_transfer"; value: Anonymize<I50ve0bbda0j1r> }
  | { type: "merge_schedules"; value: Anonymize<Ict9ivhr2c5hv0> }
  | { type: "force_remove_vesting_schedule"; value: Anonymize<Ieihemrjk2piff> }
>;
export const VestingPalletCall = _Enum as unknown as GetEnum<VestingPalletCall>;

type I9l9kkok4o3ekh = { target: MultiAddress; schedule: Anonymize<I4aro1m78pdrtt> };

type I50ve0bbda0j1r = {
  source: MultiAddress;
  target: MultiAddress;
  schedule: Anonymize<I4aro1m78pdrtt>;
};

type Ict9ivhr2c5hv0 = { schedule1_index: number; schedule2_index: number };

type Ieihemrjk2piff = { target: MultiAddress; schedule_index: number };

type I7i92s493unmka = AnonymousEnum<{
  schedule: Anonymize<Iihs4sg3g01bu>;
  cancel: Anonymize<I5n4sebgkfr760>;
  schedule_named: Anonymize<I72fgprrpc6paj>;
  cancel_named: Anonymize<Idsdstalforb09>;
  schedule_after: Anonymize<I2erle9fi4utli>;
  schedule_named_after: Anonymize<I9u9iqnbk7726p>;
  set_retry: Anonymize<I4f96q1hbj65r4>;
  set_retry_named: Anonymize<Ic9l30ktlnnckb>;
  cancel_retry: Anonymize<I7dqo5l1v1ae92>;
  cancel_retry_named: Anonymize<Idsdstalforb09>;
}>;

type Iihs4sg3g01bu = {
  when: number;
  maybe_periodic: Anonymize<I34gtdjipdmjpt>;
  priority: number;
  call: Anonymize<I5v7s5dm78b0mh>;
};

type I72fgprrpc6paj = {
  id: Binary;
  when: number;
  maybe_periodic: Anonymize<I34gtdjipdmjpt>;
  priority: number;
  call: Anonymize<I5v7s5dm78b0mh>;
};

type Idsdstalforb09 = { id: Binary };

type I2erle9fi4utli = {
  after: number;
  maybe_periodic: Anonymize<I34gtdjipdmjpt>;
  priority: number;
  call: Anonymize<I5v7s5dm78b0mh>;
};

type I9u9iqnbk7726p = {
  id: Binary;
  after: number;
  maybe_periodic: Anonymize<I34gtdjipdmjpt>;
  priority: number;
  call: Anonymize<I5v7s5dm78b0mh>;
};

type I4f96q1hbj65r4 = { task: Anonymize<I5g2vv0ckl2m8b>; retries: number; period: number };

type Ic9l30ktlnnckb = { id: Binary; retries: number; period: number };

type I7dqo5l1v1ae92 = { task: Anonymize<I5g2vv0ckl2m8b> };

type I45j9knpag7cl4 = AnonymousEnum<{
  proxy: Anonymize<I5a5u61se9rl1f>;
  add_proxy: Anonymize<I7mmcnc01jgbim>;
  remove_proxy: Anonymize<I7mmcnc01jgbim>;
  remove_proxies: undefined;
  create_pure: Anonymize<Iporp0oj935c>;
  kill_pure: Anonymize<If2c1ulh9jgo8t>;
  announce: Anonymize<Id3bpmvju2iqi5>;
  remove_announcement: Anonymize<Id3bpmvju2iqi5>;
  reject_announcement: Anonymize<Ietdab69eu3c4e>;
  proxy_announced: Anonymize<Iffuoq5s44a6st>;
}>;

type I5a5u61se9rl1f = {
  real: MultiAddress;
  force_proxy_type: Anonymize<I6adql68ljr6ip>;
  call: Anonymize<I5v7s5dm78b0mh>;
};

type I6adql68ljr6ip = Anonymize<I90b4nim1kup1v> | undefined;

type I7mmcnc01jgbim = {
  delegate: MultiAddress;
  proxy_type: Anonymize<I90b4nim1kup1v>;
  delay: number;
};

type Iporp0oj935c = { proxy_type: Anonymize<I90b4nim1kup1v>; delay: number; index: number };

type If2c1ulh9jgo8t = {
  spawner: MultiAddress;
  proxy_type: Anonymize<I90b4nim1kup1v>;
  index: number;
  height: number;
  ext_index: number;
};

type Id3bpmvju2iqi5 = { real: MultiAddress; call_hash: Binary };

type Ietdab69eu3c4e = { delegate: MultiAddress; call_hash: Binary };

type Iffuoq5s44a6st = {
  delegate: MultiAddress;
  real: MultiAddress;
  force_proxy_type: Anonymize<I6adql68ljr6ip>;
  call: Anonymize<I5v7s5dm78b0mh>;
};

type I204r9i9c46u0l = AnonymousEnum<{
  as_multi_threshold_1: Anonymize<I9o3rcvcaeatto>;
  as_multi: Anonymize<Iav5h5sjlg3ds7>;
  approve_as_multi: Anonymize<I349bg0i7n8ohu>;
  cancel_as_multi: Anonymize<I8plicv234mqe5>;
}>;

type I9o3rcvcaeatto = {
  other_signatories: Anonymize<Ia2lhg7l2hilo3>;
  call: Anonymize<I5v7s5dm78b0mh>;
};

type Iav5h5sjlg3ds7 = {
  threshold: number;
  other_signatories: Anonymize<Ia2lhg7l2hilo3>;
  maybe_timepoint: Anonymize<I95jfd8j5cr5eh>;
  call: Anonymize<I5v7s5dm78b0mh>;
  max_weight: Anonymize<I4q39t5hn830vp>;
};

type I95jfd8j5cr5eh = Anonymize<Itvprrpb0nm3o> | undefined;

type I349bg0i7n8ohu = {
  threshold: number;
  other_signatories: Anonymize<Ia2lhg7l2hilo3>;
  maybe_timepoint: Anonymize<I95jfd8j5cr5eh>;
  call_hash: Binary;
  max_weight: Anonymize<I4q39t5hn830vp>;
};

type I8plicv234mqe5 = {
  threshold: number;
  other_signatories: Anonymize<Ia2lhg7l2hilo3>;
  timepoint: Anonymize<Itvprrpb0nm3o>;
  call_hash: Binary;
};

export type PreimagePalletCall = Enum<
  | { type: "note_preimage"; value: Anonymize<I82nfqfkd48n10> }
  | { type: "unnote_preimage"; value: Anonymize<Id9d48vaes3c53> }
  | { type: "request_preimage"; value: Anonymize<Id9d48vaes3c53> }
  | { type: "unrequest_preimage"; value: Anonymize<Id9d48vaes3c53> }
  | { type: "ensure_updated"; value: Anonymize<Idaor7tajt0l3k> }
>;
export const PreimagePalletCall = _Enum as unknown as GetEnum<PreimagePalletCall>;

type I82nfqfkd48n10 = { bytes: Binary };

type Idaor7tajt0l3k = { hashes: Anonymize<Idhnf6rtqoslea> };

export type AssetRatePalletCall = Enum<
  | { type: "create"; value: Anonymize<I4digl36cac7si> }
  | { type: "update"; value: Anonymize<I4digl36cac7si> }
  | { type: "remove"; value: Anonymize<Idi8jsg8hkprub> }
>;
export const AssetRatePalletCall = _Enum as unknown as GetEnum<AssetRatePalletCall>;

export type BountiesPalletCall = Enum<
  | { type: "propose_bounty"; value: Anonymize<I2a839vbf5817q> }
  | { type: "approve_bounty"; value: Anonymize<Ia9p5bg6p18r0i> }
  | { type: "propose_curator"; value: Anonymize<I86gbm3avnuhcj> }
  | { type: "unassign_curator"; value: Anonymize<Ia9p5bg6p18r0i> }
  | { type: "accept_curator"; value: Anonymize<Ia9p5bg6p18r0i> }
  | { type: "award_bounty"; value: Anonymize<I9khudebied2et> }
  | { type: "claim_bounty"; value: Anonymize<Ia9p5bg6p18r0i> }
  | { type: "close_bounty"; value: Anonymize<Ia9p5bg6p18r0i> }
  | { type: "extend_bounty_expiry"; value: Anonymize<I90n6nnkpdahrh> }
>;
export const BountiesPalletCall = _Enum as unknown as GetEnum<BountiesPalletCall>;

type I2a839vbf5817q = { value: bigint; description: Binary };

type I86gbm3avnuhcj = { bounty_id: number; curator: MultiAddress; fee: bigint };

type I9khudebied2et = { bounty_id: number; beneficiary: MultiAddress };

type I90n6nnkpdahrh = { bounty_id: number; remark: Binary };

export type ChildBountiesPalletCall = Enum<
  | { type: "add_child_bounty"; value: Anonymize<I8mk5kjgn02hi8> }
  | { type: "propose_curator"; value: Anonymize<I113qogfj9ii7a> }
  | { type: "accept_curator"; value: Anonymize<I2gr10p66od9ch> }
  | { type: "unassign_curator"; value: Anonymize<I2gr10p66od9ch> }
  | { type: "award_child_bounty"; value: Anonymize<I6okbrc1o6b331> }
  | { type: "claim_child_bounty"; value: Anonymize<I2gr10p66od9ch> }
  | { type: "close_child_bounty"; value: Anonymize<I2gr10p66od9ch> }
>;
export const ChildBountiesPalletCall = _Enum as unknown as GetEnum<ChildBountiesPalletCall>;

type I8mk5kjgn02hi8 = { parent_bounty_id: number; value: bigint; description: Binary };

type I113qogfj9ii7a = {
  parent_bounty_id: number;
  child_bounty_id: number;
  curator: MultiAddress;
  fee: bigint;
};

type I2gr10p66od9ch = { parent_bounty_id: number; child_bounty_id: number };

type I6okbrc1o6b331 = {
  parent_bounty_id: number;
  child_bounty_id: number;
  beneficiary: MultiAddress;
};

type I7bmrug49rorp1 = AnonymousEnum<{
  place_bid: Anonymize<Icckjru8f8flpj>;
  retract_bid: Anonymize<Icckjru8f8flpj>;
  fund_deficit: undefined;
  thaw_private: Anonymize<Iba38lgbjrs7je>;
  thaw_communal: Anonymize<I666bl2fqjkejo>;
  communify: Anonymize<I666bl2fqjkejo>;
  privatize: Anonymize<I666bl2fqjkejo>;
}>;

type Icckjru8f8flpj = { amount: bigint; duration: number };

type Iba38lgbjrs7je = { index: number; maybe_proportion: Anonymize<I35p85j063s0il> };

export type PolkadotRuntimeParachainsConfigurationPalletCall = Enum<
  | { type: "set_validation_upgrade_cooldown"; value: Anonymize<I3vh014cqgmrfd> }
  | { type: "set_validation_upgrade_delay"; value: Anonymize<I3vh014cqgmrfd> }
  | { type: "set_code_retention_period"; value: Anonymize<I3vh014cqgmrfd> }
  | { type: "set_max_code_size"; value: Anonymize<I3vh014cqgmrfd> }
  | { type: "set_max_pov_size"; value: Anonymize<I3vh014cqgmrfd> }
  | { type: "set_max_head_data_size"; value: Anonymize<I3vh014cqgmrfd> }
  | { type: "set_coretime_cores"; value: Anonymize<I3vh014cqgmrfd> }
  | { type: "set_on_demand_retries"; value: Anonymize<I3vh014cqgmrfd> }
  | { type: "set_group_rotation_frequency"; value: Anonymize<I3vh014cqgmrfd> }
  | { type: "set_paras_availability_period"; value: Anonymize<I3vh014cqgmrfd> }
  | { type: "set_scheduling_lookahead"; value: Anonymize<I3vh014cqgmrfd> }
  | { type: "set_max_validators_per_core"; value: Anonymize<Id581arok0b1nj> }
  | { type: "set_max_validators"; value: Anonymize<Id581arok0b1nj> }
  | { type: "set_dispute_period"; value: Anonymize<I3vh014cqgmrfd> }
  | { type: "set_dispute_post_conclusion_acceptance_period"; value: Anonymize<I3vh014cqgmrfd> }
  | { type: "set_no_show_slots"; value: Anonymize<I3vh014cqgmrfd> }
  | { type: "set_n_delay_tranches"; value: Anonymize<I3vh014cqgmrfd> }
  | { type: "set_zeroth_delay_tranche_width"; value: Anonymize<I3vh014cqgmrfd> }
  | { type: "set_needed_approvals"; value: Anonymize<I3vh014cqgmrfd> }
  | { type: "set_relay_vrf_modulo_samples"; value: Anonymize<I3vh014cqgmrfd> }
  | { type: "set_max_upward_queue_count"; value: Anonymize<I3vh014cqgmrfd> }
  | { type: "set_max_upward_queue_size"; value: Anonymize<I3vh014cqgmrfd> }
  | { type: "set_max_downward_message_size"; value: Anonymize<I3vh014cqgmrfd> }
  | { type: "set_max_upward_message_size"; value: Anonymize<I3vh014cqgmrfd> }
  | { type: "set_max_upward_message_num_per_candidate"; value: Anonymize<I3vh014cqgmrfd> }
  | { type: "set_hrmp_open_request_ttl"; value: Anonymize<I3vh014cqgmrfd> }
  | { type: "set_hrmp_sender_deposit"; value: Anonymize<I9jsikd1ghmc7l> }
  | { type: "set_hrmp_recipient_deposit"; value: Anonymize<I9jsikd1ghmc7l> }
  | { type: "set_hrmp_channel_max_capacity"; value: Anonymize<I3vh014cqgmrfd> }
  | { type: "set_hrmp_channel_max_total_size"; value: Anonymize<I3vh014cqgmrfd> }
  | { type: "set_hrmp_max_parachain_inbound_channels"; value: Anonymize<I3vh014cqgmrfd> }
  | { type: "set_hrmp_channel_max_message_size"; value: Anonymize<I3vh014cqgmrfd> }
  | { type: "set_hrmp_max_parachain_outbound_channels"; value: Anonymize<I3vh014cqgmrfd> }
  | { type: "set_hrmp_max_message_num_per_candidate"; value: Anonymize<I3vh014cqgmrfd> }
  | { type: "set_pvf_voting_ttl"; value: Anonymize<I3vh014cqgmrfd> }
  | { type: "set_minimum_validation_upgrade_delay"; value: Anonymize<I3vh014cqgmrfd> }
  | { type: "set_bypass_consistency_check"; value: Anonymize<I2f6mha3v4ooda> }
  | { type: "set_async_backing_params"; value: Anonymize<Iasqjdhasi408s> }
  | { type: "set_executor_params"; value: Anonymize<Ibgp9od4kojpki> }
  | { type: "set_on_demand_base_fee"; value: Anonymize<I9jsikd1ghmc7l> }
  | { type: "set_on_demand_fee_variability"; value: Anonymize<I3vh014cqgmrfd> }
  | { type: "set_on_demand_queue_max_size"; value: Anonymize<I3vh014cqgmrfd> }
  | { type: "set_on_demand_target_queue_utilization"; value: Anonymize<I3vh014cqgmrfd> }
  | { type: "set_on_demand_ttl"; value: Anonymize<I3vh014cqgmrfd> }
  | { type: "set_minimum_backing_votes"; value: Anonymize<I3vh014cqgmrfd> }
  | { type: "set_node_feature"; value: Anonymize<Iaid4btmkr5thp> }
  | { type: "set_approval_voting_params"; value: Anonymize<I3vh014cqgmrfd> }
>;
export const PolkadotRuntimeParachainsConfigurationPalletCall =
  _Enum as unknown as GetEnum<PolkadotRuntimeParachainsConfigurationPalletCall>;

type I3vh014cqgmrfd = { new: number };

type Id581arok0b1nj = { new: Anonymize<I4arjljr6dpflb> };

type I9jsikd1ghmc7l = { new: bigint };

type I2f6mha3v4ooda = { new: boolean };

type Iasqjdhasi408s = { new: Anonymize<Iavuvfkop6318c> };

type Ibgp9od4kojpki = { new: Anonymize<I79n5nka727sg9> };

type Iaid4btmkr5thp = { index: number; value: boolean };

export type PolkadotRuntimeParachainsParasInherentPalletCall = Enum<{
  type: "enter";
  value: Anonymize<I4ej35gb01m5ai>;
}>;
export const PolkadotRuntimeParachainsParasInherentPalletCall =
  _Enum as unknown as GetEnum<PolkadotRuntimeParachainsParasInherentPalletCall>;

type I4ej35gb01m5ai = { data: Anonymize<I9hpj128t7mg6a> };

type I9hpj128t7mg6a = {
  bitfields: Anonymize<I429k1bjdoi8o7>;
  backed_candidates: Anonymize<I2dmlgatlidtsb>;
  disputes: Anonymize<Ib08u599nb52js>;
  parent_header: Anonymize<I6t1nedlt7mobn>;
};

type I429k1bjdoi8o7 = Array<Anonymize<Ie6rq4t789eicm>>;

type Ie6rq4t789eicm = {
  payload: { bytes: Uint8Array; bitsLen: number };
  validator_index: number;
  signature: Binary;
};

type I2dmlgatlidtsb = Array<Anonymize<I2f1tpeknmk3ja>>;

type I2f1tpeknmk3ja = {
  candidate: Anonymize<Iedmhjqij0hr8g>;
  validity_votes: Anonymize<Ibq6bocu5i9pjs>;
  validator_indices: { bytes: Uint8Array; bitsLen: number };
};

type Iedmhjqij0hr8g = {
  descriptor: Anonymize<Ib2u20s6roco9i>;
  commitments: Anonymize<Ic1d4u2opv3fst>;
};

type Ibq6bocu5i9pjs = Array<PolkadotPrimitivesV5ValidityAttestation>;

export type ParachainsParasPalletCall = Enum<
  | { type: "force_set_current_code"; value: Anonymize<I1k3urvkqqshbc> }
  | { type: "force_set_current_head"; value: Anonymize<I2ff0ffsh15vej> }
  | { type: "force_schedule_code_upgrade"; value: Anonymize<I1orfg86bkg123> }
  | { type: "force_note_new_head"; value: Anonymize<I2ff0ffsh15vej> }
  | { type: "force_queue_action"; value: Anonymize<Iaus4cb3drhu9q> }
  | { type: "add_trusted_validation_code"; value: Anonymize<Ivnsat10lv9d6> }
  | { type: "poke_unused_validation_code"; value: Anonymize<Ifqm1da2k7es4d> }
  | { type: "include_pvf_check_statement"; value: Anonymize<I4aouqiv2fh67c> }
  | { type: "force_set_most_recent_context"; value: Anonymize<I9tmok5kceg2bg> }
>;
export const ParachainsParasPalletCall = _Enum as unknown as GetEnum<ParachainsParasPalletCall>;

type I1k3urvkqqshbc = { para: number; new_code: Binary };

type I2ff0ffsh15vej = { para: number; new_head: Binary };

type I1orfg86bkg123 = { para: number; new_code: Binary; relay_parent_number: number };

type Iaus4cb3drhu9q = { para: number };

type Ivnsat10lv9d6 = { validation_code: Binary };

type Ifqm1da2k7es4d = { validation_code_hash: Binary };

type I4aouqiv2fh67c = { stmt: Anonymize<I3h1ccufdk38ej>; signature: Binary };

type I3h1ccufdk38ej = {
  accept: boolean;
  subject: Binary;
  session_index: number;
  validator_index: number;
};

type I9tmok5kceg2bg = { para: number; context: number };

export type ParachainsInitializerPalletCall = Enum<{
  type: "force_approve";
  value: Anonymize<I85icj2qbjeqbe>;
}>;
export const ParachainsInitializerPalletCall =
  _Enum as unknown as GetEnum<ParachainsInitializerPalletCall>;

type I85icj2qbjeqbe = { up_to: number };

export type PolkadotRuntimeParachainsHrmpPalletCall = Enum<
  | { type: "hrmp_init_open_channel"; value: Anonymize<Ibuhbp68e6tkct> }
  | { type: "hrmp_accept_open_channel"; value: Anonymize<Idrevppfiubhve> }
  | { type: "hrmp_close_channel"; value: Anonymize<I9s2h36kr71vk9> }
  | { type: "force_clean_hrmp"; value: Anonymize<I4lkbiubo9ogq9> }
  | { type: "force_process_hrmp_open"; value: Anonymize<Id1baei7m8gkhk> }
  | { type: "force_process_hrmp_close"; value: Anonymize<Id1baei7m8gkhk> }
  | { type: "hrmp_cancel_open_request"; value: Anonymize<I96ftepqm4vs7m> }
  | { type: "force_open_hrmp_channel"; value: Anonymize<Ic3430470j4mbv> }
  | { type: "establish_system_channel"; value: Anonymize<I50mrcbubp554e> }
  | { type: "poke_channel_deposits"; value: Anonymize<I50mrcbubp554e> }
>;
export const PolkadotRuntimeParachainsHrmpPalletCall =
  _Enum as unknown as GetEnum<PolkadotRuntimeParachainsHrmpPalletCall>;

type Ibuhbp68e6tkct = {
  recipient: number;
  proposed_max_capacity: number;
  proposed_max_message_size: number;
};

type Idrevppfiubhve = { sender: number };

type I9s2h36kr71vk9 = { channel_id: Anonymize<I50mrcbubp554e> };

type I4lkbiubo9ogq9 = { para: number; num_inbound: number; num_outbound: number };

type Id1baei7m8gkhk = { channels: number };

type I96ftepqm4vs7m = { channel_id: Anonymize<I50mrcbubp554e>; open_requests: number };

type Ic3430470j4mbv = {
  sender: number;
  recipient: number;
  max_capacity: number;
  max_message_size: number;
};

export type ParachainsDisputesPalletCall = Enum<{ type: "force_unfreeze"; value: undefined }>;
export const ParachainsDisputesPalletCall =
  _Enum as unknown as GetEnum<ParachainsDisputesPalletCall>;

export type ParachainsDisputesSlashingPalletCall = Enum<{
  type: "report_dispute_lost_unsigned";
  value: Anonymize<I1ur1874hp9ar5>;
}>;
export const ParachainsDisputesSlashingPalletCall =
  _Enum as unknown as GetEnum<ParachainsDisputesSlashingPalletCall>;

type I1ur1874hp9ar5 = {
  dispute_proof: Anonymize<Iag14tqe65tvpf>;
  key_owner_proof: Anonymize<I3ia7aufsoj0l1>;
};

type Iag14tqe65tvpf = {
  time_slot: Anonymize<Ib0p1u8q0nho37>;
  kind: PolkadotPrimitivesV5SlashingSlashingOffenceKind;
  validator_index: number;
  validator_id: Binary;
};

type Ib0p1u8q0nho37 = { session_index: number; candidate_hash: Binary };

export type MessageQueuePalletCall = Enum<
  | { type: "reap_page"; value: Anonymize<I3f1tq7m3kurig> }
  | { type: "execute_overweight"; value: Anonymize<Ifhnflnkf9f278> }
>;
export const MessageQueuePalletCall = _Enum as unknown as GetEnum<MessageQueuePalletCall>;

type I3f1tq7m3kurig = {
  message_origin: ParachainsInclusionAggregateMessageOrigin;
  page_index: number;
};

type Ifhnflnkf9f278 = {
  message_origin: ParachainsInclusionAggregateMessageOrigin;
  page: number;
  index: number;
  weight_limit: Anonymize<I4q39t5hn830vp>;
};

export type PolkadotRuntimeParachainsAssignerOnDemandPalletCall = Enum<
  | { type: "place_order_allow_death"; value: Anonymize<Iaa7g3f5tlv3gf> }
  | { type: "place_order_keep_alive"; value: Anonymize<Iaa7g3f5tlv3gf> }
>;
export const PolkadotRuntimeParachainsAssignerOnDemandPalletCall =
  _Enum as unknown as GetEnum<PolkadotRuntimeParachainsAssignerOnDemandPalletCall>;

type Iaa7g3f5tlv3gf = { max_amount: bigint; para_id: number };

export type CommonParasRegistrarPalletCall = Enum<
  | { type: "register"; value: Anonymize<I7mf0sij342109> }
  | { type: "force_register"; value: Anonymize<Ibvirp862qkkup> }
  | { type: "deregister"; value: Anonymize<Ic5b47dj4coa3r> }
  | { type: "swap"; value: Anonymize<Idehabrqi23sc0> }
  | { type: "remove_lock"; value: Anonymize<Iaus4cb3drhu9q> }
  | { type: "reserve"; value: undefined }
  | { type: "add_lock"; value: Anonymize<Iaus4cb3drhu9q> }
  | { type: "schedule_code_upgrade"; value: Anonymize<I1k3urvkqqshbc> }
  | { type: "set_current_head"; value: Anonymize<I2ff0ffsh15vej> }
>;
export const CommonParasRegistrarPalletCall =
  _Enum as unknown as GetEnum<CommonParasRegistrarPalletCall>;

type I7mf0sij342109 = { id: number; genesis_head: Binary; validation_code: Binary };

type Ibvirp862qkkup = {
  who: SS58String;
  deposit: bigint;
  id: number;
  genesis_head: Binary;
  validation_code: Binary;
};

type Ic5b47dj4coa3r = { id: number };

type Idehabrqi23sc0 = { id: number; other: number };

export type CommonSlotsPalletCall = Enum<
  | { type: "force_lease"; value: Anonymize<Idfpo6162k0hq> }
  | { type: "clear_all_leases"; value: Anonymize<Iaus4cb3drhu9q> }
  | { type: "trigger_onboard"; value: Anonymize<Iaus4cb3drhu9q> }
>;
export const CommonSlotsPalletCall = _Enum as unknown as GetEnum<CommonSlotsPalletCall>;

type Idfpo6162k0hq = {
  para: number;
  leaser: SS58String;
  amount: bigint;
  period_begin: number;
  period_count: number;
};

export type CommonAuctionsPalletCall = Enum<
  | { type: "new_auction"; value: Anonymize<I19hvnphoaj44l> }
  | { type: "bid"; value: Anonymize<I1ng31ej27mh4k> }
  | { type: "cancel_auction"; value: undefined }
>;
export const CommonAuctionsPalletCall = _Enum as unknown as GetEnum<CommonAuctionsPalletCall>;

type I19hvnphoaj44l = { duration: number; lease_period_index: number };

type I1ng31ej27mh4k = {
  para: number;
  auction_index: number;
  first_slot: number;
  last_slot: number;
  amount: bigint;
};

export type CommonCrowdloanPalletCall = Enum<
  | { type: "create"; value: Anonymize<I3js6c9fubdele> }
  | { type: "contribute"; value: Anonymize<I6n5jj22t7mm7i> }
  | { type: "withdraw"; value: Anonymize<Ia1u3jll6a06ae> }
  | { type: "refund"; value: Anonymize<I666bl2fqjkejo> }
  | { type: "dissolve"; value: Anonymize<I666bl2fqjkejo> }
  | { type: "edit"; value: Anonymize<I3js6c9fubdele> }
  | { type: "add_memo"; value: Anonymize<I7cl9esn1l72m7> }
  | { type: "poke"; value: Anonymize<I666bl2fqjkejo> }
  | { type: "contribute_all"; value: Anonymize<I3k27o64k49es2> }
>;
export const CommonCrowdloanPalletCall = _Enum as unknown as GetEnum<CommonCrowdloanPalletCall>;

type I3js6c9fubdele = {
  index: number;
  cap: bigint;
  first_period: number;
  last_period: number;
  end: number;
  verifier: Anonymize<I8t18p6mokc3s4>;
};

type I6n5jj22t7mm7i = { index: number; value: bigint; signature: Anonymize<I7us28h09nc5sv> };

type I7cl9esn1l72m7 = { index: number; memo: Binary };

type I3k27o64k49es2 = { index: number; signature: Anonymize<I7us28h09nc5sv> };

export type PolkadotRuntimeParachainsCoretimePalletCall = Enum<
  | { type: "request_core_count"; value: Anonymize<Iafscmv8tjf0ou> }
  | { type: "assign_core"; value: Anonymize<I6ls6t1gggjtv6> }
>;
export const PolkadotRuntimeParachainsCoretimePalletCall =
  _Enum as unknown as GetEnum<PolkadotRuntimeParachainsCoretimePalletCall>;

type I6ls6t1gggjtv6 = {
  core: number;
  begin: number;
  assignment: Anonymize<Id1dkuivtmc10j>;
  end_hint: Anonymize<I4arjljr6dpflb>;
};

export type XcmPalletCall = Enum<
  | { type: "send"; value: Anonymize<I3d5n20t9iopr1> }
  | { type: "teleport_assets"; value: Anonymize<Iqg2kc7rjuace> }
  | { type: "reserve_transfer_assets"; value: Anonymize<Iqg2kc7rjuace> }
  | { type: "execute"; value: Anonymize<I94f2uf779bu0k> }
  | { type: "force_xcm_version"; value: Anonymize<Iergfrmj3c9dor> }
  | { type: "force_default_xcm_version"; value: Anonymize<Ic76kfh5ebqkpl> }
  | { type: "force_subscribe_version_notify"; value: Anonymize<Ib67j9jdqr7ekj> }
  | { type: "force_unsubscribe_version_notify"; value: Anonymize<Ib67j9jdqr7ekj> }
  | { type: "limited_reserve_transfer_assets"; value: Anonymize<Ibk7a73iljhv98> }
  | { type: "limited_teleport_assets"; value: Anonymize<Ibk7a73iljhv98> }
  | { type: "force_suspension"; value: Anonymize<Ibgm4rnf22lal1> }
  | { type: "transfer_assets"; value: Anonymize<Ibk7a73iljhv98> }
>;
export const XcmPalletCall = _Enum as unknown as GetEnum<XcmPalletCall>;

type I3d5n20t9iopr1 = { dest: XcmVersionedLocation; message: XcmVersionedXcm };

export type XcmVersionedXcm = Enum<
  | { type: "V2"; value: Anonymize<I797ibmv93o8n9> }
  | { type: "V3"; value: Anonymize<I50ghg3dhe8sh3> }
  | { type: "V4"; value: Anonymize<I9uem0hkif9mb7> }
>;
export const XcmVersionedXcm = _Enum as unknown as GetEnum<XcmVersionedXcm>;

type I797ibmv93o8n9 = Array<XcmV2Instruction>;

export type XcmV2Instruction = Enum<
  | { type: "WithdrawAsset"; value: Anonymize<Ia3ggl9eghkufh> }
  | { type: "ReserveAssetDeposited"; value: Anonymize<Ia3ggl9eghkufh> }
  | { type: "ReceiveTeleportedAsset"; value: Anonymize<Ia3ggl9eghkufh> }
  | { type: "QueryResponse"; value: Anonymize<I7adp6ofrfskbq> }
  | { type: "TransferAsset"; value: Anonymize<I55b7rvmacg132> }
  | { type: "TransferReserveAsset"; value: Anonymize<I87p6gu1rs00b9> }
  | { type: "Transact"; value: Anonymize<I61kq38r93nm9u> }
  | { type: "HrmpNewChannelOpenRequest"; value: Anonymize<I5uhhrjqfuo4e5> }
  | { type: "HrmpChannelAccepted"; value: Anonymize<Ifij4jam0o7sub> }
  | { type: "HrmpChannelClosing"; value: Anonymize<Ieeb4svd9i8fji> }
  | { type: "ClearOrigin"; value: undefined }
  | { type: "DescendOrigin"; value: Anonymize<XcmV2MultilocationJunctions> }
  | { type: "ReportError"; value: Anonymize<I99o59cf77uo81> }
  | { type: "DepositAsset"; value: Anonymize<I2fdiqplld7l4b> }
  | { type: "DepositReserveAsset"; value: Anonymize<I4e86ltq2coupq> }
  | { type: "ExchangeAsset"; value: Anonymize<I8i9t5akp4s2qr> }
  | { type: "InitiateReserveWithdraw"; value: Anonymize<I3rvvq2i351pp4> }
  | { type: "InitiateTeleport"; value: Anonymize<I2eh04tsbsec6v> }
  | { type: "QueryHolding"; value: Anonymize<Iih6kp60v9gan> }
  | { type: "BuyExecution"; value: Anonymize<I2u6ut68eoldqa> }
  | { type: "RefundSurplus"; value: undefined }
  | { type: "SetErrorHandler"; value: Anonymize<I797ibmv93o8n9> }
  | { type: "SetAppendix"; value: Anonymize<I797ibmv93o8n9> }
  | { type: "ClearError"; value: undefined }
  | { type: "ClaimAsset"; value: Anonymize<I60l7lelr2s5kd> }
  | { type: "Trap"; value: Anonymize<bigint> }
  | { type: "SubscribeVersion"; value: Anonymize<Ido2s48ntevurj> }
  | { type: "UnsubscribeVersion"; value: undefined }
>;
export const XcmV2Instruction = _Enum as unknown as GetEnum<XcmV2Instruction>;

type I7adp6ofrfskbq = { query_id: bigint; response: XcmV2Response; max_weight: bigint };

type I55b7rvmacg132 = { assets: Anonymize<Ia3ggl9eghkufh>; beneficiary: Anonymize<Ibki0d249v3ojt> };

type I87p6gu1rs00b9 = {
  assets: Anonymize<Ia3ggl9eghkufh>;
  dest: Anonymize<Ibki0d249v3ojt>;
  xcm: Anonymize<I797ibmv93o8n9>;
};

type I61kq38r93nm9u = {
  origin_type: XcmV2OriginKind;
  require_weight_at_most: bigint;
  call: Binary;
};

type I99o59cf77uo81 = {
  query_id: bigint;
  dest: Anonymize<Ibki0d249v3ojt>;
  max_response_weight: bigint;
};

type I2fdiqplld7l4b = {
  assets: XcmV2MultiAssetFilter;
  max_assets: number;
  beneficiary: Anonymize<Ibki0d249v3ojt>;
};

export type XcmV2MultiAssetFilter = Enum<
  | { type: "Definite"; value: Anonymize<Ia3ggl9eghkufh> }
  | { type: "Wild"; value: Anonymize<XcmV2MultiassetWildMultiAsset> }
>;
export const XcmV2MultiAssetFilter = _Enum as unknown as GetEnum<XcmV2MultiAssetFilter>;

export type XcmV2MultiassetWildMultiAsset = Enum<
  { type: "All"; value: undefined } | { type: "AllOf"; value: Anonymize<I96k6616d81u1u> }
>;
export const XcmV2MultiassetWildMultiAsset =
  _Enum as unknown as GetEnum<XcmV2MultiassetWildMultiAsset>;

type I96k6616d81u1u = { id: XcmV2MultiassetAssetId; fun: XcmV2MultiassetWildFungibility };

type I4e86ltq2coupq = {
  assets: XcmV2MultiAssetFilter;
  max_assets: number;
  dest: Anonymize<Ibki0d249v3ojt>;
  xcm: Anonymize<I797ibmv93o8n9>;
};

type I8i9t5akp4s2qr = { give: XcmV2MultiAssetFilter; receive: Anonymize<Ia3ggl9eghkufh> };

type I3rvvq2i351pp4 = {
  assets: XcmV2MultiAssetFilter;
  reserve: Anonymize<Ibki0d249v3ojt>;
  xcm: Anonymize<I797ibmv93o8n9>;
};

type I2eh04tsbsec6v = {
  assets: XcmV2MultiAssetFilter;
  dest: Anonymize<Ibki0d249v3ojt>;
  xcm: Anonymize<I797ibmv93o8n9>;
};

type Iih6kp60v9gan = {
  query_id: bigint;
  dest: Anonymize<Ibki0d249v3ojt>;
  assets: XcmV2MultiAssetFilter;
  max_response_weight: bigint;
};

type I2u6ut68eoldqa = { fees: Anonymize<I16mc4mv5bb0qd>; weight_limit: XcmV2WeightLimit };

export type XcmV2WeightLimit = Enum<
  { type: "Unlimited"; value: undefined } | { type: "Limited"; value: Anonymize<bigint> }
>;
export const XcmV2WeightLimit = _Enum as unknown as GetEnum<XcmV2WeightLimit>;

type I60l7lelr2s5kd = { assets: Anonymize<Ia3ggl9eghkufh>; ticket: Anonymize<Ibki0d249v3ojt> };

type Ido2s48ntevurj = { query_id: bigint; max_response_weight: bigint };

type I50ghg3dhe8sh3 = Array<XcmV3Instruction>;

export type XcmV3Instruction = Enum<
  | { type: "WithdrawAsset"; value: Anonymize<I2pdjq1umlp617> }
  | { type: "ReserveAssetDeposited"; value: Anonymize<I2pdjq1umlp617> }
  | { type: "ReceiveTeleportedAsset"; value: Anonymize<I2pdjq1umlp617> }
  | { type: "QueryResponse"; value: Anonymize<Ifcbfhsum5pdt8> }
  | { type: "TransferAsset"; value: Anonymize<Iciun0t2v4pn9s> }
  | { type: "TransferReserveAsset"; value: Anonymize<I4gomd50gf1sdo> }
  | { type: "Transact"; value: Anonymize<I4sfmje1omkmem> }
  | { type: "HrmpNewChannelOpenRequest"; value: Anonymize<I5uhhrjqfuo4e5> }
  | { type: "HrmpChannelAccepted"; value: Anonymize<Ifij4jam0o7sub> }
  | { type: "HrmpChannelClosing"; value: Anonymize<Ieeb4svd9i8fji> }
  | { type: "ClearOrigin"; value: undefined }
  | { type: "DescendOrigin"; value: Anonymize<XcmV3Junctions> }
  | { type: "ReportError"; value: Anonymize<I8iu73oulmbcl6> }
  | { type: "DepositAsset"; value: Anonymize<I68v077ao044c4> }
  | { type: "DepositReserveAsset"; value: Anonymize<Iehlmrpch57np8> }
  | { type: "ExchangeAsset"; value: Anonymize<Ic6p876kf5qu6l> }
  | { type: "InitiateReserveWithdraw"; value: Anonymize<I6njvicgem6gam> }
  | { type: "InitiateTeleport"; value: Anonymize<Iehlmrpch57np8> }
  | { type: "ReportHolding"; value: Anonymize<Ictq7qpggrhev0> }
  | { type: "BuyExecution"; value: Anonymize<I5a4kvfk1c5e9> }
  | { type: "RefundSurplus"; value: undefined }
  | { type: "SetErrorHandler"; value: Anonymize<I50ghg3dhe8sh3> }
  | { type: "SetAppendix"; value: Anonymize<I50ghg3dhe8sh3> }
  | { type: "ClearError"; value: undefined }
  | { type: "ClaimAsset"; value: Anonymize<Iatoh41hlqpeff> }
  | { type: "Trap"; value: Anonymize<bigint> }
  | { type: "SubscribeVersion"; value: Anonymize<Ieprdqqu7ildvr> }
  | { type: "UnsubscribeVersion"; value: undefined }
  | { type: "BurnAsset"; value: Anonymize<I2pdjq1umlp617> }
  | { type: "ExpectAsset"; value: Anonymize<I2pdjq1umlp617> }
  | { type: "ExpectOrigin"; value: Anonymize<I189rbbmttkf8v> }
  | { type: "ExpectError"; value: Anonymize<I8j770n2arfq59> }
  | { type: "ExpectTransactStatus"; value: Anonymize<XcmV3MaybeErrorCode> }
  | { type: "QueryPallet"; value: Anonymize<I9o6j30dnhmlg9> }
  | { type: "ExpectPallet"; value: Anonymize<Id7mf37dkpgfjs> }
  | { type: "ReportTransactStatus"; value: Anonymize<I8iu73oulmbcl6> }
  | { type: "ClearTransactStatus"; value: undefined }
  | { type: "UniversalOrigin"; value: Anonymize<XcmV4Junction> }
  | { type: "ExportMessage"; value: Anonymize<Iatj898em490l6> }
  | { type: "LockAsset"; value: Anonymize<Ifgane16e7gi0u> }
  | { type: "UnlockAsset"; value: Anonymize<Ibs9ci5muat0jn> }
  | { type: "NoteUnlockable"; value: Anonymize<I9pln3upoovp5l> }
  | { type: "RequestUnlock"; value: Anonymize<Ibqteslvkvmmol> }
  | { type: "SetFeesMode"; value: Anonymize<I4nae9rsql8fa7> }
  | { type: "SetTopic"; value: Anonymize<Binary> }
  | { type: "ClearTopic"; value: undefined }
  | { type: "AliasOrigin"; value: Anonymize<Ie897ubj3a1vaq> }
  | { type: "UnpaidExecution"; value: Anonymize<I8b0u1467piq8o> }
>;
export const XcmV3Instruction = _Enum as unknown as GetEnum<XcmV3Instruction>;

type Ifcbfhsum5pdt8 = {
  query_id: bigint;
  response: XcmV3Response;
  max_weight: Anonymize<I4q39t5hn830vp>;
  querier: Anonymize<I189rbbmttkf8v>;
};

type I189rbbmttkf8v = Anonymize<Ie897ubj3a1vaq> | undefined;

type Iciun0t2v4pn9s = { assets: Anonymize<I2pdjq1umlp617>; beneficiary: Anonymize<Ie897ubj3a1vaq> };

type I4gomd50gf1sdo = {
  assets: Anonymize<I2pdjq1umlp617>;
  dest: Anonymize<Ie897ubj3a1vaq>;
  xcm: Anonymize<I50ghg3dhe8sh3>;
};

type I8iu73oulmbcl6 = {
  destination: Anonymize<Ie897ubj3a1vaq>;
  query_id: bigint;
  max_weight: Anonymize<I4q39t5hn830vp>;
};

type I68v077ao044c4 = {
  assets: XcmV3MultiassetMultiAssetFilter;
  beneficiary: Anonymize<Ie897ubj3a1vaq>;
};

export type XcmV3MultiassetMultiAssetFilter = Enum<
  | { type: "Definite"; value: Anonymize<I2pdjq1umlp617> }
  | { type: "Wild"; value: Anonymize<XcmV3MultiassetWildMultiAsset> }
>;
export const XcmV3MultiassetMultiAssetFilter =
  _Enum as unknown as GetEnum<XcmV3MultiassetMultiAssetFilter>;

export type XcmV3MultiassetWildMultiAsset = Enum<
  | { type: "All"; value: undefined }
  | { type: "AllOf"; value: Anonymize<I4ihu8nnggag7m> }
  | { type: "AllCounted"; value: Anonymize<number> }
  | { type: "AllOfCounted"; value: Anonymize<I8t2ghbj5822uc> }
>;
export const XcmV3MultiassetWildMultiAsset =
  _Enum as unknown as GetEnum<XcmV3MultiassetWildMultiAsset>;

type I4ihu8nnggag7m = { id: XcmV3MultiassetAssetId; fun: XcmV2MultiassetWildFungibility };

type I8t2ghbj5822uc = {
  id: XcmV3MultiassetAssetId;
  fun: XcmV2MultiassetWildFungibility;
  count: number;
};

type Iehlmrpch57np8 = {
  assets: XcmV3MultiassetMultiAssetFilter;
  dest: Anonymize<Ie897ubj3a1vaq>;
  xcm: Anonymize<I50ghg3dhe8sh3>;
};

type Ic6p876kf5qu6l = {
  give: XcmV3MultiassetMultiAssetFilter;
  want: Anonymize<I2pdjq1umlp617>;
  maximal: boolean;
};

type I6njvicgem6gam = {
  assets: XcmV3MultiassetMultiAssetFilter;
  reserve: Anonymize<Ie897ubj3a1vaq>;
  xcm: Anonymize<I50ghg3dhe8sh3>;
};

type Ictq7qpggrhev0 = {
  response_info: Anonymize<I8iu73oulmbcl6>;
  assets: XcmV3MultiassetMultiAssetFilter;
};

type I5a4kvfk1c5e9 = { fees: Anonymize<Isj6qus1lv5t9>; weight_limit: XcmV3WeightLimit };

type Iatoh41hlqpeff = { assets: Anonymize<I2pdjq1umlp617>; ticket: Anonymize<Ie897ubj3a1vaq> };

type I9o6j30dnhmlg9 = { module_name: Binary; response_info: Anonymize<I8iu73oulmbcl6> };

type Iatj898em490l6 = {
  network: XcmV4JunctionNetworkId;
  destination: XcmV3Junctions;
  xcm: Anonymize<I50ghg3dhe8sh3>;
};

type Ifgane16e7gi0u = { asset: Anonymize<Isj6qus1lv5t9>; unlocker: Anonymize<Ie897ubj3a1vaq> };

type Ibs9ci5muat0jn = { asset: Anonymize<Isj6qus1lv5t9>; target: Anonymize<Ie897ubj3a1vaq> };

type I9pln3upoovp5l = { asset: Anonymize<Isj6qus1lv5t9>; owner: Anonymize<Ie897ubj3a1vaq> };

type Ibqteslvkvmmol = { asset: Anonymize<Isj6qus1lv5t9>; locker: Anonymize<Ie897ubj3a1vaq> };

type I8b0u1467piq8o = { weight_limit: XcmV3WeightLimit; check_origin: Anonymize<I189rbbmttkf8v> };

type Iqg2kc7rjuace = {
  dest: XcmVersionedLocation;
  beneficiary: XcmVersionedLocation;
  assets: XcmVersionedAssets;
  fee_asset_item: number;
};

type I94f2uf779bu0k = { message: XcmVersionedXcm1; max_weight: Anonymize<I4q39t5hn830vp> };

export type XcmVersionedXcm1 = Enum<
  | { type: "V2"; value: Anonymize<I6gdh0i5feh6sm> }
  | { type: "V3"; value: Anonymize<Ie2lqpvbcq3vl6> }
  | { type: "V4"; value: Anonymize<I2mfda7terfjrv> }
>;
export const XcmVersionedXcm1 = _Enum as unknown as GetEnum<XcmVersionedXcm1>;

type I6gdh0i5feh6sm = Array<XcmV2Instruction1>;

export type XcmV2Instruction1 = Enum<
  | { type: "WithdrawAsset"; value: Anonymize<Ia3ggl9eghkufh> }
  | { type: "ReserveAssetDeposited"; value: Anonymize<Ia3ggl9eghkufh> }
  | { type: "ReceiveTeleportedAsset"; value: Anonymize<Ia3ggl9eghkufh> }
  | { type: "QueryResponse"; value: Anonymize<I7adp6ofrfskbq> }
  | { type: "TransferAsset"; value: Anonymize<I55b7rvmacg132> }
  | { type: "TransferReserveAsset"; value: Anonymize<I87p6gu1rs00b9> }
  | { type: "Transact"; value: Anonymize<I61kq38r93nm9u> }
  | { type: "HrmpNewChannelOpenRequest"; value: Anonymize<I5uhhrjqfuo4e5> }
  | { type: "HrmpChannelAccepted"; value: Anonymize<Ifij4jam0o7sub> }
  | { type: "HrmpChannelClosing"; value: Anonymize<Ieeb4svd9i8fji> }
  | { type: "ClearOrigin"; value: undefined }
  | { type: "DescendOrigin"; value: Anonymize<XcmV2MultilocationJunctions> }
  | { type: "ReportError"; value: Anonymize<I99o59cf77uo81> }
  | { type: "DepositAsset"; value: Anonymize<I2fdiqplld7l4b> }
  | { type: "DepositReserveAsset"; value: Anonymize<I4e86ltq2coupq> }
  | { type: "ExchangeAsset"; value: Anonymize<I8i9t5akp4s2qr> }
  | { type: "InitiateReserveWithdraw"; value: Anonymize<I3rvvq2i351pp4> }
  | { type: "InitiateTeleport"; value: Anonymize<I2eh04tsbsec6v> }
  | { type: "QueryHolding"; value: Anonymize<Iih6kp60v9gan> }
  | { type: "BuyExecution"; value: Anonymize<I2u6ut68eoldqa> }
  | { type: "RefundSurplus"; value: undefined }
  | { type: "SetErrorHandler"; value: Anonymize<I6gdh0i5feh6sm> }
  | { type: "SetAppendix"; value: Anonymize<I6gdh0i5feh6sm> }
  | { type: "ClearError"; value: undefined }
  | { type: "ClaimAsset"; value: Anonymize<I60l7lelr2s5kd> }
  | { type: "Trap"; value: Anonymize<bigint> }
  | { type: "SubscribeVersion"; value: Anonymize<Ido2s48ntevurj> }
  | { type: "UnsubscribeVersion"; value: undefined }
>;
export const XcmV2Instruction1 = _Enum as unknown as GetEnum<XcmV2Instruction1>;

type Ie2lqpvbcq3vl6 = Array<XcmV3Instruction1>;

export type XcmV3Instruction1 = Enum<
  | { type: "WithdrawAsset"; value: Anonymize<I2pdjq1umlp617> }
  | { type: "ReserveAssetDeposited"; value: Anonymize<I2pdjq1umlp617> }
  | { type: "ReceiveTeleportedAsset"; value: Anonymize<I2pdjq1umlp617> }
  | { type: "QueryResponse"; value: Anonymize<Ifcbfhsum5pdt8> }
  | { type: "TransferAsset"; value: Anonymize<Iciun0t2v4pn9s> }
  | { type: "TransferReserveAsset"; value: Anonymize<I4gomd50gf1sdo> }
  | { type: "Transact"; value: Anonymize<I4sfmje1omkmem> }
  | { type: "HrmpNewChannelOpenRequest"; value: Anonymize<I5uhhrjqfuo4e5> }
  | { type: "HrmpChannelAccepted"; value: Anonymize<Ifij4jam0o7sub> }
  | { type: "HrmpChannelClosing"; value: Anonymize<Ieeb4svd9i8fji> }
  | { type: "ClearOrigin"; value: undefined }
  | { type: "DescendOrigin"; value: Anonymize<XcmV3Junctions> }
  | { type: "ReportError"; value: Anonymize<I8iu73oulmbcl6> }
  | { type: "DepositAsset"; value: Anonymize<I68v077ao044c4> }
  | { type: "DepositReserveAsset"; value: Anonymize<Iehlmrpch57np8> }
  | { type: "ExchangeAsset"; value: Anonymize<Ic6p876kf5qu6l> }
  | { type: "InitiateReserveWithdraw"; value: Anonymize<I6njvicgem6gam> }
  | { type: "InitiateTeleport"; value: Anonymize<Iehlmrpch57np8> }
  | { type: "ReportHolding"; value: Anonymize<Ictq7qpggrhev0> }
  | { type: "BuyExecution"; value: Anonymize<I5a4kvfk1c5e9> }
  | { type: "RefundSurplus"; value: undefined }
  | { type: "SetErrorHandler"; value: Anonymize<Ie2lqpvbcq3vl6> }
  | { type: "SetAppendix"; value: Anonymize<Ie2lqpvbcq3vl6> }
  | { type: "ClearError"; value: undefined }
  | { type: "ClaimAsset"; value: Anonymize<Iatoh41hlqpeff> }
  | { type: "Trap"; value: Anonymize<bigint> }
  | { type: "SubscribeVersion"; value: Anonymize<Ieprdqqu7ildvr> }
  | { type: "UnsubscribeVersion"; value: undefined }
  | { type: "BurnAsset"; value: Anonymize<I2pdjq1umlp617> }
  | { type: "ExpectAsset"; value: Anonymize<I2pdjq1umlp617> }
  | { type: "ExpectOrigin"; value: Anonymize<I189rbbmttkf8v> }
  | { type: "ExpectError"; value: Anonymize<I8j770n2arfq59> }
  | { type: "ExpectTransactStatus"; value: Anonymize<XcmV3MaybeErrorCode> }
  | { type: "QueryPallet"; value: Anonymize<I9o6j30dnhmlg9> }
  | { type: "ExpectPallet"; value: Anonymize<Id7mf37dkpgfjs> }
  | { type: "ReportTransactStatus"; value: Anonymize<I8iu73oulmbcl6> }
  | { type: "ClearTransactStatus"; value: undefined }
  | { type: "UniversalOrigin"; value: Anonymize<XcmV4Junction> }
  | { type: "ExportMessage"; value: Anonymize<Iatj898em490l6> }
  | { type: "LockAsset"; value: Anonymize<Ifgane16e7gi0u> }
  | { type: "UnlockAsset"; value: Anonymize<Ibs9ci5muat0jn> }
  | { type: "NoteUnlockable"; value: Anonymize<I9pln3upoovp5l> }
  | { type: "RequestUnlock"; value: Anonymize<Ibqteslvkvmmol> }
  | { type: "SetFeesMode"; value: Anonymize<I4nae9rsql8fa7> }
  | { type: "SetTopic"; value: Anonymize<Binary> }
  | { type: "ClearTopic"; value: undefined }
  | { type: "AliasOrigin"; value: Anonymize<Ie897ubj3a1vaq> }
  | { type: "UnpaidExecution"; value: Anonymize<I8b0u1467piq8o> }
>;
export const XcmV3Instruction1 = _Enum as unknown as GetEnum<XcmV3Instruction1>;

type I2mfda7terfjrv = Array<XcmV4Instruction1>;

export type XcmV4Instruction1 = Enum<
  | { type: "WithdrawAsset"; value: Anonymize<Ia04r3ent3ib9l> }
  | { type: "ReserveAssetDeposited"; value: Anonymize<Ia04r3ent3ib9l> }
  | { type: "ReceiveTeleportedAsset"; value: Anonymize<Ia04r3ent3ib9l> }
  | { type: "QueryResponse"; value: Anonymize<If5f25il1ge6a5> }
  | { type: "TransferAsset"; value: Anonymize<I9o6pmhgf6qb2n> }
  | { type: "TransferReserveAsset"; value: Anonymize<Ia3b5a2vfbfo8f> }
  | { type: "Transact"; value: Anonymize<I4sfmje1omkmem> }
  | { type: "HrmpNewChannelOpenRequest"; value: Anonymize<I5uhhrjqfuo4e5> }
  | { type: "HrmpChannelAccepted"; value: Anonymize<Ifij4jam0o7sub> }
  | { type: "HrmpChannelClosing"; value: Anonymize<Ieeb4svd9i8fji> }
  | { type: "ClearOrigin"; value: undefined }
  | { type: "DescendOrigin"; value: Anonymize<XcmV4Junctions> }
  | { type: "ReportError"; value: Anonymize<I2t1argld0e8ha> }
  | { type: "DepositAsset"; value: Anonymize<I1jvdebe5g7h7j> }
  | { type: "DepositReserveAsset"; value: Anonymize<Ip007io6ag12n> }
  | { type: "ExchangeAsset"; value: Anonymize<Ifblc1cck9cd37> }
  | { type: "InitiateReserveWithdraw"; value: Anonymize<I6p8gvucdlt5k7> }
  | { type: "InitiateTeleport"; value: Anonymize<Ip007io6ag12n> }
  | { type: "ReportHolding"; value: Anonymize<I674ulrvvjnhs0> }
  | { type: "BuyExecution"; value: Anonymize<Idbkkieivqk1gi> }
  | { type: "RefundSurplus"; value: undefined }
  | { type: "SetErrorHandler"; value: Anonymize<I2mfda7terfjrv> }
  | { type: "SetAppendix"; value: Anonymize<I2mfda7terfjrv> }
  | { type: "ClearError"; value: undefined }
  | { type: "ClaimAsset"; value: Anonymize<I66uqjrc5dmf2e> }
  | { type: "Trap"; value: Anonymize<bigint> }
  | { type: "SubscribeVersion"; value: Anonymize<Ieprdqqu7ildvr> }
  | { type: "UnsubscribeVersion"; value: undefined }
  | { type: "BurnAsset"; value: Anonymize<Ia04r3ent3ib9l> }
  | { type: "ExpectAsset"; value: Anonymize<Ia04r3ent3ib9l> }
  | { type: "ExpectOrigin"; value: Anonymize<Iehbh3m350i09b> }
  | { type: "ExpectError"; value: Anonymize<I8j770n2arfq59> }
  | { type: "ExpectTransactStatus"; value: Anonymize<XcmV3MaybeErrorCode> }
  | { type: "QueryPallet"; value: Anonymize<Ibltfjhsfj1s9i> }
  | { type: "ExpectPallet"; value: Anonymize<Id7mf37dkpgfjs> }
  | { type: "ReportTransactStatus"; value: Anonymize<I2t1argld0e8ha> }
  | { type: "ClearTransactStatus"; value: undefined }
  | { type: "UniversalOrigin"; value: Anonymize<XcmV4Junction> }
  | { type: "ExportMessage"; value: Anonymize<Ibrkdv8bf9uaq7> }
  | { type: "LockAsset"; value: Anonymize<Id49srnpojvmi6> }
  | { type: "UnlockAsset"; value: Anonymize<I3lc2qil81b3dt> }
  | { type: "NoteUnlockable"; value: Anonymize<Ib2aloe249nr2> }
  | { type: "RequestUnlock"; value: Anonymize<I2ida362kv5gol> }
  | { type: "SetFeesMode"; value: Anonymize<I4nae9rsql8fa7> }
  | { type: "SetTopic"; value: Anonymize<Binary> }
  | { type: "ClearTopic"; value: undefined }
  | { type: "AliasOrigin"; value: Anonymize<Ieq428hdmibf6m> }
  | { type: "UnpaidExecution"; value: Anonymize<I943mf7c2b9qmm> }
>;
export const XcmV4Instruction1 = _Enum as unknown as GetEnum<XcmV4Instruction1>;

type Ic76kfh5ebqkpl = { maybe_xcm_version: Anonymize<I4arjljr6dpflb> };

type Ib67j9jdqr7ekj = { location: XcmVersionedLocation };

type Ibk7a73iljhv98 = {
  dest: XcmVersionedLocation;
  beneficiary: XcmVersionedLocation;
  assets: XcmVersionedAssets;
  fee_asset_item: number;
  weight_limit: XcmV3WeightLimit;
};

type Ibgm4rnf22lal1 = { suspended: boolean };

export type BeefyPalletCall = Enum<
  | { type: "report_equivocation"; value: Anonymize<I894urueu7skf3> }
  | { type: "report_equivocation_unsigned"; value: Anonymize<I894urueu7skf3> }
  | { type: "set_new_genesis"; value: Anonymize<Iemqna2uucuei9> }
>;
export const BeefyPalletCall = _Enum as unknown as GetEnum<BeefyPalletCall>;

type I894urueu7skf3 = {
  equivocation_proof: Anonymize<I6lgkrki6lhal>;
  key_owner_proof: Anonymize<I3ia7aufsoj0l1>;
};

type I6lgkrki6lhal = { first: Anonymize<Ifukb31gb9emjd>; second: Anonymize<Ifukb31gb9emjd> };

type Ifukb31gb9emjd = { commitment: Anonymize<Ie28rq7o91tbl9>; id: Binary; signature: Binary };

type Ie28rq7o91tbl9 = {
  payload: Anonymize<I2eg8oaov5ur5b>;
  block_number: number;
  validator_set_id: bigint;
};

type I2eg8oaov5ur5b = Array<Anonymize<Id6fv7d28ees54>>;

type Id6fv7d28ees54 = [Binary, Binary];

type Iemqna2uucuei9 = { delay_in_blocks: number };

export type PolkadotRuntimeCommonIdentityMigratorPalletCall = Enum<
  | { type: "reap_identity"; value: Anonymize<I4cbvqmqadhrea> }
  | { type: "poke_deposit"; value: Anonymize<I4cbvqmqadhrea> }
>;
export const PolkadotRuntimeCommonIdentityMigratorPalletCall =
  _Enum as unknown as GetEnum<PolkadotRuntimeCommonIdentityMigratorPalletCall>;

export type PolkadotRuntimeCommonParasSudoWrapperPalletCall = Enum<
  | { type: "sudo_schedule_para_initialize"; value: Anonymize<I9geq5evbpu4im> }
  | { type: "sudo_schedule_para_cleanup"; value: Anonymize<Ic5b47dj4coa3r> }
  | { type: "sudo_schedule_parathread_upgrade"; value: Anonymize<Ic5b47dj4coa3r> }
  | { type: "sudo_schedule_parachain_downgrade"; value: Anonymize<Ic5b47dj4coa3r> }
  | { type: "sudo_queue_downward_xcm"; value: Anonymize<Ie3vanb7qoq2qr> }
  | { type: "sudo_establish_hrmp_channel"; value: Anonymize<Ic3430470j4mbv> }
>;
export const PolkadotRuntimeCommonParasSudoWrapperPalletCall =
  _Enum as unknown as GetEnum<PolkadotRuntimeCommonParasSudoWrapperPalletCall>;

type I9geq5evbpu4im = { id: number; genesis: Anonymize<I2duhnt686rv0q> };

type Ie3vanb7qoq2qr = { id: number; xcm: XcmVersionedXcm };

export type PolkadotRuntimeCommonAssignedSlotsPalletCall = Enum<
  | { type: "assign_perm_parachain_slot"; value: Anonymize<Ic5b47dj4coa3r> }
  | { type: "assign_temp_parachain_slot"; value: Anonymize<Ib4b26jo7ljjg3> }
  | { type: "unassign_parachain_slot"; value: Anonymize<Ic5b47dj4coa3r> }
  | { type: "set_max_permanent_slots"; value: Anonymize<I9d5h5irbki7mm> }
  | { type: "set_max_temporary_slots"; value: Anonymize<I9d5h5irbki7mm> }
>;
export const PolkadotRuntimeCommonAssignedSlotsPalletCall =
  _Enum as unknown as GetEnum<PolkadotRuntimeCommonAssignedSlotsPalletCall>;

type Ib4b26jo7ljjg3 = {
  id: number;
  lease_period_start: PolkadotRuntimeCommonAssignedSlotsSlotLeasePeriodStart;
};

export type PolkadotRuntimeCommonAssignedSlotsSlotLeasePeriodStart = Enum<
  { type: "Current"; value: undefined } | { type: "Next"; value: undefined }
>;
export const PolkadotRuntimeCommonAssignedSlotsSlotLeasePeriodStart =
  _Enum as unknown as GetEnum<PolkadotRuntimeCommonAssignedSlotsSlotLeasePeriodStart>;

type I8rur6c856tfev = AnonymousEnum<{
  register_validators: Anonymize<I97hfovkaaqb7h>;
  deregister_validators: Anonymize<I97hfovkaaqb7h>;
}>;

type I97hfovkaaqb7h = { validators: Anonymize<Ia2lhg7l2hilo3> };

type Idu3no1672nmsk = AnonymousEnum<{
  control_auto_migration: Anonymize<I7psec5e6ghc64>;
  continue_migrate: Anonymize<Ibnsai0l0rfp2>;
  migrate_custom_top: Anonymize<I585tk8khua0gk>;
  migrate_custom_child: Anonymize<I3ut99di214ru2>;
  set_signed_max_limits: Anonymize<Iemkp87d26vsbh>;
  force_set_progress: Anonymize<I4guudoaeoq2e9>;
}>;

type I7psec5e6ghc64 = { maybe_config: Anonymize<Ib17t3992hb64n> };

type Ibnsai0l0rfp2 = {
  limits: Anonymize<I215mkl885p4da>;
  real_size_upper: number;
  witness_task: Anonymize<I2sqgqj7ib3ine>;
};

type I585tk8khua0gk = { keys: Anonymize<Itom7fk49o0c9>; witness_size: number };

type I3ut99di214ru2 = { root: Binary; child_keys: Anonymize<Itom7fk49o0c9>; total_size: number };

type Iemkp87d26vsbh = { limits: Anonymize<I215mkl885p4da> };

type I4guudoaeoq2e9 = {
  progress_top: Anonymize<I2jvfaenrpg1ai>;
  progress_child: Anonymize<I2jvfaenrpg1ai>;
};

export type RootTestingPalletCall = Enum<
  | { type: "fill_block"; value: Anonymize<Ienjibnb78vnl0> }
  | { type: "trigger_defensive"; value: undefined }
>;
export const RootTestingPalletCall = _Enum as unknown as GetEnum<RootTestingPalletCall>;

type Ienjibnb78vnl0 = { ratio: number };

type I8aedjs0th135a = AnonymousEnum<{
  sudo: Anonymize<Ib6n6j9tk6ut3b>;
  sudo_unchecked_weight: Anonymize<Ie1uvn9e6ki8dd>;
  set_key: Anonymize<Icnonnse26sae7>;
  sudo_as: Anonymize<I94s8ivufjp2ik>;
  remove_key: undefined;
}>;

type Icnonnse26sae7 = { new: MultiAddress };

type I94s8ivufjp2ik = { who: MultiAddress; call: Anonymize<I5v7s5dm78b0mh> };

export type PalletError = Enum<
  | { type: "InvalidSpecName"; value: undefined }
  | { type: "SpecVersionNeedsToIncrease"; value: undefined }
  | { type: "FailedToExtractRuntimeVersion"; value: undefined }
  | { type: "NonDefaultComposite"; value: undefined }
  | { type: "NonZeroRefCount"; value: undefined }
  | { type: "CallFiltered"; value: undefined }
  | { type: "NothingAuthorized"; value: undefined }
  | { type: "Unauthorized"; value: undefined }
>;
export const PalletError = _Enum as unknown as GetEnum<PalletError>;

export type BabePalletError = Enum<
  | { type: "InvalidEquivocationProof"; value: undefined }
  | { type: "InvalidKeyOwnershipProof"; value: undefined }
  | { type: "DuplicateOffenceReport"; value: undefined }
  | { type: "InvalidConfiguration"; value: undefined }
>;
export const BabePalletError = _Enum as unknown as GetEnum<BabePalletError>;

export type IndicesPalletError = Enum<
  | { type: "NotAssigned"; value: undefined }
  | { type: "NotOwner"; value: undefined }
  | { type: "InUse"; value: undefined }
  | { type: "NotTransfer"; value: undefined }
  | { type: "Permanent"; value: undefined }
>;
export const IndicesPalletError = _Enum as unknown as GetEnum<IndicesPalletError>;

export type BalancesPalletError = Enum<
  | { type: "VestingBalance"; value: undefined }
  | { type: "LiquidityRestrictions"; value: undefined }
  | { type: "InsufficientBalance"; value: undefined }
  | { type: "ExistentialDeposit"; value: undefined }
  | { type: "Expendability"; value: undefined }
  | { type: "ExistingVestingSchedule"; value: undefined }
  | { type: "DeadAccount"; value: undefined }
  | { type: "TooManyReserves"; value: undefined }
  | { type: "TooManyHolds"; value: undefined }
  | { type: "TooManyFreezes"; value: undefined }
  | { type: "IssuanceDeactivated"; value: undefined }
  | { type: "DeltaZero"; value: undefined }
>;
export const BalancesPalletError = _Enum as unknown as GetEnum<BalancesPalletError>;

export type SessionPalletError = Enum<
  | { type: "InvalidProof"; value: undefined }
  | { type: "NoAssociatedValidatorId"; value: undefined }
  | { type: "DuplicatedKey"; value: undefined }
  | { type: "NoKeys"; value: undefined }
  | { type: "NoAccount"; value: undefined }
>;
export const SessionPalletError = _Enum as unknown as GetEnum<SessionPalletError>;

export type GrandpaPalletError = Enum<
  | { type: "PauseFailed"; value: undefined }
  | { type: "ResumeFailed"; value: undefined }
  | { type: "ChangePending"; value: undefined }
  | { type: "TooSoon"; value: undefined }
  | { type: "InvalidKeyOwnershipProof"; value: undefined }
  | { type: "InvalidEquivocationProof"; value: undefined }
  | { type: "DuplicateOffenceReport"; value: undefined }
>;
export const GrandpaPalletError = _Enum as unknown as GetEnum<GrandpaPalletError>;

export type TreasuryPalletError = Enum<
  | { type: "InsufficientProposersBalance"; value: undefined }
  | { type: "InvalidIndex"; value: undefined }
  | { type: "TooManyApprovals"; value: undefined }
  | { type: "InsufficientPermission"; value: undefined }
  | { type: "ProposalNotApproved"; value: undefined }
  | { type: "FailedToConvertBalance"; value: undefined }
  | { type: "SpendExpired"; value: undefined }
  | { type: "EarlyPayout"; value: undefined }
  | { type: "AlreadyAttempted"; value: undefined }
  | { type: "PayoutError"; value: undefined }
  | { type: "NotAttempted"; value: undefined }
  | { type: "Inconclusive"; value: undefined }
>;
export const TreasuryPalletError = _Enum as unknown as GetEnum<TreasuryPalletError>;

export type ConvictionVotingPalletError = Enum<
  | { type: "NotOngoing"; value: undefined }
  | { type: "NotVoter"; value: undefined }
  | { type: "NoPermission"; value: undefined }
  | { type: "NoPermissionYet"; value: undefined }
  | { type: "AlreadyDelegating"; value: undefined }
  | { type: "AlreadyVoting"; value: undefined }
  | { type: "InsufficientFunds"; value: undefined }
  | { type: "NotDelegating"; value: undefined }
  | { type: "Nonsense"; value: undefined }
  | { type: "MaxVotesReached"; value: undefined }
  | { type: "ClassNeeded"; value: undefined }
  | { type: "BadClass"; value: undefined }
>;
export const ConvictionVotingPalletError = _Enum as unknown as GetEnum<ConvictionVotingPalletError>;

export type ReferendaPalletError = Enum<
  | { type: "NotOngoing"; value: undefined }
  | { type: "HasDeposit"; value: undefined }
  | { type: "BadTrack"; value: undefined }
  | { type: "Full"; value: undefined }
  | { type: "QueueEmpty"; value: undefined }
  | { type: "BadReferendum"; value: undefined }
  | { type: "NothingToDo"; value: undefined }
  | { type: "NoTrack"; value: undefined }
  | { type: "Unfinished"; value: undefined }
  | { type: "NoPermission"; value: undefined }
  | { type: "NoDeposit"; value: undefined }
  | { type: "BadStatus"; value: undefined }
  | { type: "PreimageNotExist"; value: undefined }
>;
export const ReferendaPalletError = _Enum as unknown as GetEnum<ReferendaPalletError>;

type Ict11ieg4bvcav = AnonymousEnum<{
  AlreadyMember: undefined;
  NotMember: undefined;
  NotPolling: undefined;
  Ongoing: undefined;
  NoneRemaining: undefined;
  Corruption: undefined;
  RankTooLow: undefined;
  InvalidWitness: undefined;
  NoPermission: undefined;
  SameMember: undefined;
}>;

export type WhitelistPalletError = Enum<
  | { type: "UnavailablePreImage"; value: undefined }
  | { type: "UndecodableCall"; value: undefined }
  | { type: "InvalidCallWeightWitness"; value: undefined }
  | { type: "CallIsNotWhitelisted"; value: undefined }
  | { type: "CallAlreadyWhitelisted"; value: undefined }
>;
export const WhitelistPalletError = _Enum as unknown as GetEnum<WhitelistPalletError>;

export type CommonClaimsPalletError = Enum<
  | { type: "InvalidEthereumSignature"; value: undefined }
  | { type: "SignerHasNoClaim"; value: undefined }
  | { type: "SenderHasNoClaim"; value: undefined }
  | { type: "PotUnderflow"; value: undefined }
  | { type: "InvalidStatement"; value: undefined }
  | { type: "VestedBalanceExists"; value: undefined }
>;
export const CommonClaimsPalletError = _Enum as unknown as GetEnum<CommonClaimsPalletError>;

export type UtilityPalletError = Enum<{ type: "TooManyCalls"; value: undefined }>;
export const UtilityPalletError = _Enum as unknown as GetEnum<UtilityPalletError>;

export type IdentityPalletError = Enum<
  | { type: "TooManySubAccounts"; value: undefined }
  | { type: "NotFound"; value: undefined }
  | { type: "NotNamed"; value: undefined }
  | { type: "EmptyIndex"; value: undefined }
  | { type: "FeeChanged"; value: undefined }
  | { type: "NoIdentity"; value: undefined }
  | { type: "StickyJudgement"; value: undefined }
  | { type: "JudgementGiven"; value: undefined }
  | { type: "InvalidJudgement"; value: undefined }
  | { type: "InvalidIndex"; value: undefined }
  | { type: "InvalidTarget"; value: undefined }
  | { type: "TooManyRegistrars"; value: undefined }
  | { type: "AlreadyClaimed"; value: undefined }
  | { type: "NotSub"; value: undefined }
  | { type: "NotOwned"; value: undefined }
  | { type: "JudgementForDifferentIdentity"; value: undefined }
  | { type: "JudgementPaymentFailed"; value: undefined }
  | { type: "InvalidSuffix"; value: undefined }
  | { type: "NotUsernameAuthority"; value: undefined }
  | { type: "NoAllocation"; value: undefined }
  | { type: "InvalidSignature"; value: undefined }
  | { type: "RequiresSignature"; value: undefined }
  | { type: "InvalidUsername"; value: undefined }
  | { type: "UsernameTaken"; value: undefined }
  | { type: "NoUsername"; value: undefined }
  | { type: "NotExpired"; value: undefined }
>;
export const IdentityPalletError = _Enum as unknown as GetEnum<IdentityPalletError>;

type Infmqt45b3map = AnonymousEnum<{
  NotMember: undefined;
  AlreadyMember: undefined;
  Suspended: undefined;
  NotSuspended: undefined;
  NoPayout: undefined;
  AlreadyFounded: undefined;
  InsufficientPot: undefined;
  AlreadyVouching: undefined;
  NotVouchingOnBidder: undefined;
  Head: undefined;
  Founder: undefined;
  AlreadyBid: undefined;
  AlreadyCandidate: undefined;
  NotCandidate: undefined;
  MaxMembers: undefined;
  NotFounder: undefined;
  NotHead: undefined;
  NotApproved: undefined;
  NotRejected: undefined;
  Approved: undefined;
  Rejected: undefined;
  InProgress: undefined;
  TooEarly: undefined;
  Voted: undefined;
  Expired: undefined;
  NotBidder: undefined;
  NoDefender: undefined;
  NotGroup: undefined;
  AlreadyElevated: undefined;
  AlreadyPunished: undefined;
  InsufficientFunds: undefined;
  NoVotes: undefined;
}>;

export type RecoveryPalletError = Enum<
  | { type: "NotAllowed"; value: undefined }
  | { type: "ZeroThreshold"; value: undefined }
  | { type: "NotEnoughFriends"; value: undefined }
  | { type: "MaxFriends"; value: undefined }
  | { type: "NotSorted"; value: undefined }
  | { type: "NotRecoverable"; value: undefined }
  | { type: "AlreadyRecoverable"; value: undefined }
  | { type: "AlreadyStarted"; value: undefined }
  | { type: "NotStarted"; value: undefined }
  | { type: "NotFriend"; value: undefined }
  | { type: "DelayPeriod"; value: undefined }
  | { type: "AlreadyVouched"; value: undefined }
  | { type: "Threshold"; value: undefined }
  | { type: "StillActive"; value: undefined }
  | { type: "AlreadyProxy"; value: undefined }
  | { type: "BadState"; value: undefined }
>;
export const RecoveryPalletError = _Enum as unknown as GetEnum<RecoveryPalletError>;

export type VestingPalletError = Enum<
  | { type: "NotVesting"; value: undefined }
  | { type: "AtMaxVestingSchedules"; value: undefined }
  | { type: "AmountLow"; value: undefined }
  | { type: "ScheduleIndexOutOfBounds"; value: undefined }
  | { type: "InvalidScheduleParams"; value: undefined }
>;
export const VestingPalletError = _Enum as unknown as GetEnum<VestingPalletError>;

export type SchedulerPalletError = Enum<
  | { type: "FailedToSchedule"; value: undefined }
  | { type: "NotFound"; value: undefined }
  | { type: "TargetBlockNumberInPast"; value: undefined }
  | { type: "RescheduleNoChange"; value: undefined }
  | { type: "Named"; value: undefined }
>;
export const SchedulerPalletError = _Enum as unknown as GetEnum<SchedulerPalletError>;

export type ProxyPalletError = Enum<
  | { type: "TooMany"; value: undefined }
  | { type: "NotFound"; value: undefined }
  | { type: "NotProxy"; value: undefined }
  | { type: "Unproxyable"; value: undefined }
  | { type: "Duplicate"; value: undefined }
  | { type: "NoPermission"; value: undefined }
  | { type: "Unannounced"; value: undefined }
  | { type: "NoSelfProxy"; value: undefined }
>;
export const ProxyPalletError = _Enum as unknown as GetEnum<ProxyPalletError>;

export type MultisigPalletError = Enum<
  | { type: "MinimumThreshold"; value: undefined }
  | { type: "AlreadyApproved"; value: undefined }
  | { type: "NoApprovalsNeeded"; value: undefined }
  | { type: "TooFewSignatories"; value: undefined }
  | { type: "TooManySignatories"; value: undefined }
  | { type: "SignatoriesOutOfOrder"; value: undefined }
  | { type: "SenderInSignatories"; value: undefined }
  | { type: "NotFound"; value: undefined }
  | { type: "NotOwner"; value: undefined }
  | { type: "NoTimepoint"; value: undefined }
  | { type: "WrongTimepoint"; value: undefined }
  | { type: "UnexpectedTimepoint"; value: undefined }
  | { type: "MaxWeightTooLow"; value: undefined }
  | { type: "AlreadyStored"; value: undefined }
>;
export const MultisigPalletError = _Enum as unknown as GetEnum<MultisigPalletError>;

export type PreimagePalletError = Enum<
  | { type: "TooBig"; value: undefined }
  | { type: "AlreadyNoted"; value: undefined }
  | { type: "NotAuthorized"; value: undefined }
  | { type: "NotNoted"; value: undefined }
  | { type: "Requested"; value: undefined }
  | { type: "NotRequested"; value: undefined }
  | { type: "TooMany"; value: undefined }
  | { type: "TooFew"; value: undefined }
>;
export const PreimagePalletError = _Enum as unknown as GetEnum<PreimagePalletError>;

export type AssetRatePalletError = Enum<
  | { type: "UnknownAssetKind"; value: undefined }
  | { type: "AlreadyExists"; value: undefined }
  | { type: "Overflow"; value: undefined }
>;
export const AssetRatePalletError = _Enum as unknown as GetEnum<AssetRatePalletError>;

export type BountiesPalletError = Enum<
  | { type: "InsufficientProposersBalance"; value: undefined }
  | { type: "InvalidIndex"; value: undefined }
  | { type: "ReasonTooBig"; value: undefined }
  | { type: "UnexpectedStatus"; value: undefined }
  | { type: "RequireCurator"; value: undefined }
  | { type: "InvalidValue"; value: undefined }
  | { type: "InvalidFee"; value: undefined }
  | { type: "PendingPayout"; value: undefined }
  | { type: "Premature"; value: undefined }
  | { type: "HasActiveChildBounty"; value: undefined }
  | { type: "TooManyQueued"; value: undefined }
>;
export const BountiesPalletError = _Enum as unknown as GetEnum<BountiesPalletError>;

export type ChildBountiesPalletError = Enum<
  | { type: "ParentBountyNotActive"; value: undefined }
  | { type: "InsufficientBountyBalance"; value: undefined }
  | { type: "TooManyChildBounties"; value: undefined }
>;
export const ChildBountiesPalletError = _Enum as unknown as GetEnum<ChildBountiesPalletError>;

type I5j6imevs2stc = AnonymousEnum<{
  DurationTooSmall: undefined;
  DurationTooBig: undefined;
  AmountTooSmall: undefined;
  BidTooLow: undefined;
  UnknownReceipt: undefined;
  NotOwner: undefined;
  NotExpired: undefined;
  UnknownBid: undefined;
  PortionTooBig: undefined;
  Unfunded: undefined;
  AlreadyFunded: undefined;
  Throttled: undefined;
  MakesDust: undefined;
  AlreadyCommunal: undefined;
  AlreadyPrivate: undefined;
}>;

export type ParachainsConfigurationPalletError = Enum<{
  type: "InvalidNewValue";
  value: undefined;
}>;
export const ParachainsConfigurationPalletError =
  _Enum as unknown as GetEnum<ParachainsConfigurationPalletError>;

export type ParachainsInclusionPalletError = Enum<
  | { type: "UnsortedOrDuplicateValidatorIndices"; value: undefined }
  | { type: "UnsortedOrDuplicateDisputeStatementSet"; value: undefined }
  | { type: "UnsortedOrDuplicateBackedCandidates"; value: undefined }
  | { type: "UnexpectedRelayParent"; value: undefined }
  | { type: "WrongBitfieldSize"; value: undefined }
  | { type: "BitfieldAllZeros"; value: undefined }
  | { type: "BitfieldDuplicateOrUnordered"; value: undefined }
  | { type: "ValidatorIndexOutOfBounds"; value: undefined }
  | { type: "InvalidBitfieldSignature"; value: undefined }
  | { type: "UnscheduledCandidate"; value: undefined }
  | { type: "CandidateScheduledBeforeParaFree"; value: undefined }
  | { type: "ScheduledOutOfOrder"; value: undefined }
  | { type: "HeadDataTooLarge"; value: undefined }
  | { type: "PrematureCodeUpgrade"; value: undefined }
  | { type: "NewCodeTooLarge"; value: undefined }
  | { type: "DisallowedRelayParent"; value: undefined }
  | { type: "InvalidAssignment"; value: undefined }
  | { type: "InvalidGroupIndex"; value: undefined }
  | { type: "InsufficientBacking"; value: undefined }
  | { type: "InvalidBacking"; value: undefined }
  | { type: "NotCollatorSigned"; value: undefined }
  | { type: "ValidationDataHashMismatch"; value: undefined }
  | { type: "IncorrectDownwardMessageHandling"; value: undefined }
  | { type: "InvalidUpwardMessages"; value: undefined }
  | { type: "HrmpWatermarkMishandling"; value: undefined }
  | { type: "InvalidOutboundHrmp"; value: undefined }
  | { type: "InvalidValidationCodeHash"; value: undefined }
  | { type: "ParaHeadMismatch"; value: undefined }
  | { type: "BitfieldReferencesFreedCore"; value: undefined }
>;
export const ParachainsInclusionPalletError =
  _Enum as unknown as GetEnum<ParachainsInclusionPalletError>;

export type PolkadotRuntimeParachainsParasInherentPalletError = Enum<
  | { type: "TooManyInclusionInherents"; value: undefined }
  | { type: "InvalidParentHeader"; value: undefined }
  | { type: "CandidateConcludedInvalid"; value: undefined }
  | { type: "InherentOverweight"; value: undefined }
  | { type: "DisputeStatementsUnsortedOrDuplicates"; value: undefined }
  | { type: "DisputeInvalid"; value: undefined }
  | { type: "BackedByDisabled"; value: undefined }
  | { type: "BackedOnUnscheduledCore"; value: undefined }
  | { type: "UnscheduledCandidate"; value: undefined }
>;
export const PolkadotRuntimeParachainsParasInherentPalletError =
  _Enum as unknown as GetEnum<PolkadotRuntimeParachainsParasInherentPalletError>;

export type PolkadotRuntimeParachainsParasPalletError = Enum<
  | { type: "NotRegistered"; value: undefined }
  | { type: "CannotOnboard"; value: undefined }
  | { type: "CannotOffboard"; value: undefined }
  | { type: "CannotUpgrade"; value: undefined }
  | { type: "CannotDowngrade"; value: undefined }
  | { type: "PvfCheckStatementStale"; value: undefined }
  | { type: "PvfCheckStatementFuture"; value: undefined }
  | { type: "PvfCheckValidatorIndexOutOfBounds"; value: undefined }
  | { type: "PvfCheckInvalidSignature"; value: undefined }
  | { type: "PvfCheckDoubleVote"; value: undefined }
  | { type: "PvfCheckSubjectInvalid"; value: undefined }
  | { type: "CannotUpgradeCode"; value: undefined }
  | { type: "InvalidCode"; value: undefined }
>;
export const PolkadotRuntimeParachainsParasPalletError =
  _Enum as unknown as GetEnum<PolkadotRuntimeParachainsParasPalletError>;

export type PolkadotRuntimeParachainsHrmpPalletError = Enum<
  | { type: "OpenHrmpChannelToSelf"; value: undefined }
  | { type: "OpenHrmpChannelInvalidRecipient"; value: undefined }
  | { type: "OpenHrmpChannelZeroCapacity"; value: undefined }
  | { type: "OpenHrmpChannelCapacityExceedsLimit"; value: undefined }
  | { type: "OpenHrmpChannelZeroMessageSize"; value: undefined }
  | { type: "OpenHrmpChannelMessageSizeExceedsLimit"; value: undefined }
  | { type: "OpenHrmpChannelAlreadyExists"; value: undefined }
  | { type: "OpenHrmpChannelAlreadyRequested"; value: undefined }
  | { type: "OpenHrmpChannelLimitExceeded"; value: undefined }
  | { type: "AcceptHrmpChannelDoesntExist"; value: undefined }
  | { type: "AcceptHrmpChannelAlreadyConfirmed"; value: undefined }
  | { type: "AcceptHrmpChannelLimitExceeded"; value: undefined }
  | { type: "CloseHrmpChannelUnauthorized"; value: undefined }
  | { type: "CloseHrmpChannelDoesntExist"; value: undefined }
  | { type: "CloseHrmpChannelAlreadyUnderway"; value: undefined }
  | { type: "CancelHrmpOpenChannelUnauthorized"; value: undefined }
  | { type: "OpenHrmpChannelDoesntExist"; value: undefined }
  | { type: "OpenHrmpChannelAlreadyConfirmed"; value: undefined }
  | { type: "WrongWitness"; value: undefined }
  | { type: "ChannelCreationNotAuthorized"; value: undefined }
>;
export const PolkadotRuntimeParachainsHrmpPalletError =
  _Enum as unknown as GetEnum<PolkadotRuntimeParachainsHrmpPalletError>;

export type ParachainsDisputesPalletError = Enum<
  | { type: "DuplicateDisputeStatementSets"; value: undefined }
  | { type: "AncientDisputeStatement"; value: undefined }
  | { type: "ValidatorIndexOutOfBounds"; value: undefined }
  | { type: "InvalidSignature"; value: undefined }
  | { type: "DuplicateStatement"; value: undefined }
  | { type: "SingleSidedDispute"; value: undefined }
  | { type: "MaliciousBacker"; value: undefined }
  | { type: "MissingBackingVotes"; value: undefined }
  | { type: "UnconfirmedDispute"; value: undefined }
>;
export const ParachainsDisputesPalletError =
  _Enum as unknown as GetEnum<ParachainsDisputesPalletError>;

export type ParachainsDisputesSlashingPalletError = Enum<
  | { type: "InvalidKeyOwnershipProof"; value: undefined }
  | { type: "InvalidSessionIndex"; value: undefined }
  | { type: "InvalidCandidateHash"; value: undefined }
  | { type: "InvalidValidatorIndex"; value: undefined }
  | { type: "ValidatorIndexIdMismatch"; value: undefined }
  | { type: "DuplicateSlashingReport"; value: undefined }
>;
export const ParachainsDisputesSlashingPalletError =
  _Enum as unknown as GetEnum<ParachainsDisputesSlashingPalletError>;

export type MessageQueuePalletError = Enum<
  | { type: "NotReapable"; value: undefined }
  | { type: "NoPage"; value: undefined }
  | { type: "NoMessage"; value: undefined }
  | { type: "AlreadyProcessed"; value: undefined }
  | { type: "Queued"; value: undefined }
  | { type: "InsufficientWeight"; value: undefined }
  | { type: "TemporarilyUnprocessable"; value: undefined }
  | { type: "QueuePaused"; value: undefined }
  | { type: "RecursiveDisallowed"; value: undefined }
>;
export const MessageQueuePalletError = _Enum as unknown as GetEnum<MessageQueuePalletError>;

export type PolkadotRuntimeParachainsAssignerOnDemandPalletError = Enum<
  | { type: "InvalidParaId"; value: undefined }
  | { type: "QueueFull"; value: undefined }
  | { type: "SpotPriceHigherThanMaxAmount"; value: undefined }
>;
export const PolkadotRuntimeParachainsAssignerOnDemandPalletError =
  _Enum as unknown as GetEnum<PolkadotRuntimeParachainsAssignerOnDemandPalletError>;

export type PolkadotRuntimeParachainsAssignerCoretimePalletError = Enum<
  | { type: "AssignmentsEmpty"; value: undefined }
  | { type: "OverScheduled"; value: undefined }
  | { type: "UnderScheduled"; value: undefined }
  | { type: "DisallowedInsert"; value: undefined }
  | { type: "DuplicateInsert"; value: undefined }
  | { type: "AssignmentsNotSorted"; value: undefined }
>;
export const PolkadotRuntimeParachainsAssignerCoretimePalletError =
  _Enum as unknown as GetEnum<PolkadotRuntimeParachainsAssignerCoretimePalletError>;

export type PolkadotRuntimeCommonParasRegistrarPalletError = Enum<
  | { type: "NotRegistered"; value: undefined }
  | { type: "AlreadyRegistered"; value: undefined }
  | { type: "NotOwner"; value: undefined }
  | { type: "CodeTooLarge"; value: undefined }
  | { type: "HeadDataTooLarge"; value: undefined }
  | { type: "NotParachain"; value: undefined }
  | { type: "NotParathread"; value: undefined }
  | { type: "CannotDeregister"; value: undefined }
  | { type: "CannotDowngrade"; value: undefined }
  | { type: "CannotUpgrade"; value: undefined }
  | { type: "ParaLocked"; value: undefined }
  | { type: "NotReserved"; value: undefined }
  | { type: "InvalidCode"; value: undefined }
  | { type: "CannotSwap"; value: undefined }
>;
export const PolkadotRuntimeCommonParasRegistrarPalletError =
  _Enum as unknown as GetEnum<PolkadotRuntimeCommonParasRegistrarPalletError>;

export type CommonSlotsPalletError = Enum<
  { type: "ParaNotOnboarding"; value: undefined } | { type: "LeaseError"; value: undefined }
>;
export const CommonSlotsPalletError = _Enum as unknown as GetEnum<CommonSlotsPalletError>;

export type CommonAuctionsPalletError = Enum<
  | { type: "AuctionInProgress"; value: undefined }
  | { type: "LeasePeriodInPast"; value: undefined }
  | { type: "ParaNotRegistered"; value: undefined }
  | { type: "NotCurrentAuction"; value: undefined }
  | { type: "NotAuction"; value: undefined }
  | { type: "AuctionEnded"; value: undefined }
  | { type: "AlreadyLeasedOut"; value: undefined }
>;
export const CommonAuctionsPalletError = _Enum as unknown as GetEnum<CommonAuctionsPalletError>;

export type CommonCrowdloanPalletError = Enum<
  | { type: "FirstPeriodInPast"; value: undefined }
  | { type: "FirstPeriodTooFarInFuture"; value: undefined }
  | { type: "LastPeriodBeforeFirstPeriod"; value: undefined }
  | { type: "LastPeriodTooFarInFuture"; value: undefined }
  | { type: "CannotEndInPast"; value: undefined }
  | { type: "EndTooFarInFuture"; value: undefined }
  | { type: "Overflow"; value: undefined }
  | { type: "ContributionTooSmall"; value: undefined }
  | { type: "InvalidParaId"; value: undefined }
  | { type: "CapExceeded"; value: undefined }
  | { type: "ContributionPeriodOver"; value: undefined }
  | { type: "InvalidOrigin"; value: undefined }
  | { type: "NotParachain"; value: undefined }
  | { type: "LeaseActive"; value: undefined }
  | { type: "BidOrLeaseActive"; value: undefined }
  | { type: "FundNotEnded"; value: undefined }
  | { type: "NoContributions"; value: undefined }
  | { type: "NotReadyToDissolve"; value: undefined }
  | { type: "InvalidSignature"; value: undefined }
  | { type: "MemoTooLarge"; value: undefined }
  | { type: "AlreadyInNewRaise"; value: undefined }
  | { type: "VrfDelayInProgress"; value: undefined }
  | { type: "NoLeasePeriod"; value: undefined }
>;
export const CommonCrowdloanPalletError = _Enum as unknown as GetEnum<CommonCrowdloanPalletError>;

export type PolkadotRuntimeParachainsCoretimePalletError = Enum<{
  type: "NotBroker";
  value: undefined;
}>;
export const PolkadotRuntimeParachainsCoretimePalletError =
  _Enum as unknown as GetEnum<PolkadotRuntimeParachainsCoretimePalletError>;

export type XcmPalletError = Enum<
  | { type: "Unreachable"; value: undefined }
  | { type: "SendFailure"; value: undefined }
  | { type: "Filtered"; value: undefined }
  | { type: "UnweighableMessage"; value: undefined }
  | { type: "DestinationNotInvertible"; value: undefined }
  | { type: "Empty"; value: undefined }
  | { type: "CannotReanchor"; value: undefined }
  | { type: "TooManyAssets"; value: undefined }
  | { type: "InvalidOrigin"; value: undefined }
  | { type: "BadVersion"; value: undefined }
  | { type: "BadLocation"; value: undefined }
  | { type: "NoSubscription"; value: undefined }
  | { type: "AlreadySubscribed"; value: undefined }
  | { type: "CannotCheckOutTeleport"; value: undefined }
  | { type: "LowBalance"; value: undefined }
  | { type: "TooManyLocks"; value: undefined }
  | { type: "AccountNotSovereign"; value: undefined }
  | { type: "FeesNotMet"; value: undefined }
  | { type: "LockNotFound"; value: undefined }
  | { type: "InUse"; value: undefined }
  | { type: "InvalidAssetNotConcrete"; value: undefined }
  | { type: "InvalidAssetUnknownReserve"; value: undefined }
  | { type: "InvalidAssetUnsupportedReserve"; value: undefined }
  | { type: "TooManyReserves"; value: undefined }
  | { type: "LocalExecutionIncomplete"; value: undefined }
>;
export const XcmPalletError = _Enum as unknown as GetEnum<XcmPalletError>;

export type PolkadotRuntimeCommonParasSudoWrapperPalletError = Enum<
  | { type: "ParaDoesntExist"; value: undefined }
  | { type: "ParaAlreadyExists"; value: undefined }
  | { type: "ExceedsMaxMessageSize"; value: undefined }
  | { type: "CouldntCleanup"; value: undefined }
  | { type: "NotParathread"; value: undefined }
  | { type: "NotParachain"; value: undefined }
  | { type: "CannotUpgrade"; value: undefined }
  | { type: "CannotDowngrade"; value: undefined }
  | { type: "TooManyCores"; value: undefined }
>;
export const PolkadotRuntimeCommonParasSudoWrapperPalletError =
  _Enum as unknown as GetEnum<PolkadotRuntimeCommonParasSudoWrapperPalletError>;

export type PolkadotRuntimeCommonAssignedSlotsPalletError = Enum<
  | { type: "ParaDoesntExist"; value: undefined }
  | { type: "NotParathread"; value: undefined }
  | { type: "CannotUpgrade"; value: undefined }
  | { type: "CannotDowngrade"; value: undefined }
  | { type: "SlotAlreadyAssigned"; value: undefined }
  | { type: "SlotNotAssigned"; value: undefined }
  | { type: "OngoingLeaseExists"; value: undefined }
  | { type: "MaxPermanentSlotsExceeded"; value: undefined }
  | { type: "MaxTemporarySlotsExceeded"; value: undefined }
>;
export const PolkadotRuntimeCommonAssignedSlotsPalletError =
  _Enum as unknown as GetEnum<PolkadotRuntimeCommonAssignedSlotsPalletError>;

export type SudoPalletError = Enum<{ type: "RequireSudo"; value: undefined }>;
export const SudoPalletError = _Enum as unknown as GetEnum<SudoPalletError>;

type Ia4jqf401148a4 = { header: Anonymize<I6t1nedlt7mobn>; extrinsics: Anonymize<Itom7fk49o0c9> };

type I132412t2eshc6 = ResultPayload<Anonymize<Idtdr91jmq5g4i>, TransactionValidityError>;

export type TransactionValidityError = Enum<
  | { type: "Invalid"; value: Anonymize<TransactionValidityInvalidTransaction> }
  | { type: "Unknown"; value: Anonymize<TransactionValidityUnknownTransaction> }
>;
export const TransactionValidityError = _Enum as unknown as GetEnum<TransactionValidityError>;

export type TransactionValidityInvalidTransaction = Enum<
  | { type: "Call"; value: undefined }
  | { type: "Payment"; value: undefined }
  | { type: "Future"; value: undefined }
  | { type: "Stale"; value: undefined }
  | { type: "BadProof"; value: undefined }
  | { type: "AncientBirthBlock"; value: undefined }
  | { type: "ExhaustsResources"; value: undefined }
  | { type: "Custom"; value: Anonymize<number> }
  | { type: "BadMandatory"; value: undefined }
  | { type: "MandatoryValidation"; value: undefined }
  | { type: "BadSigner"; value: undefined }
>;
export const TransactionValidityInvalidTransaction =
  _Enum as unknown as GetEnum<TransactionValidityInvalidTransaction>;

export type TransactionValidityUnknownTransaction = Enum<
  | { type: "CannotLookup"; value: undefined }
  | { type: "NoUnsignedValidator"; value: undefined }
  | { type: "Custom"; value: Anonymize<number> }
>;
export const TransactionValidityUnknownTransaction =
  _Enum as unknown as GetEnum<TransactionValidityUnknownTransaction>;

type If39abi8floaaf = Array<Anonymize<I1kbn2golmm2dm>>;

type I1kbn2golmm2dm = [Binary, Binary];

type Ibcdj0j79gq1uo = { okay: boolean; fatal_error: boolean; errors: Anonymize<If39abi8floaaf> };

export type TransactionValidityTransactionSource = Enum<
  | { type: "InBlock"; value: undefined }
  | { type: "Local"; value: undefined }
  | { type: "External"; value: undefined }
>;
export const TransactionValidityTransactionSource =
  _Enum as unknown as GetEnum<TransactionValidityTransactionSource>;

type I6d9lhhdri071p = ResultPayload<Anonymize<I6g5lcd9vf2cr0>, TransactionValidityError>;

type I6g5lcd9vf2cr0 = {
  priority: bigint;
  requires: Anonymize<Itom7fk49o0c9>;
  provides: Anonymize<Itom7fk49o0c9>;
  longevity: bigint;
  propagate: boolean;
};

type I5985kfq7sspta = [Anonymize<Iarlj3qd8u1v13>, Anonymize<I94uslvmqboam8>];

type I94uslvmqboam8 = {
  session_start_block: number;
  group_rotation_frequency: number;
  now: number;
};

type I8a7gsn4nh0gne = Array<PolkadotPrimitivesV5CoreState>;

export type PolkadotPrimitivesV5CoreState = Enum<
  | { type: "Occupied"; value: Anonymize<Iedrr54lmrujd0> }
  | { type: "Scheduled"; value: Anonymize<Ij0oq61lvrdfj> }
  | { type: "Free"; value: undefined }
>;
export const PolkadotPrimitivesV5CoreState =
  _Enum as unknown as GetEnum<PolkadotPrimitivesV5CoreState>;

type Iedrr54lmrujd0 = {
  next_up_on_available: Anonymize<I4j5il5p7i6los>;
  occupied_since: number;
  time_out_at: number;
  next_up_on_time_out: Anonymize<I4j5il5p7i6los>;
  availability: { bytes: Uint8Array; bitsLen: number };
  group_responsible: number;
  candidate_hash: Binary;
  candidate_descriptor: Anonymize<Ib2u20s6roco9i>;
};

type I4j5il5p7i6los = Anonymize<Ij0oq61lvrdfj> | undefined;

type Ij0oq61lvrdfj = { para_id: number; collator: Anonymize<I17k3ujudqd5df> };

export type PolkadotPrimitivesV5OccupiedCoreAssumption = Enum<
  | { type: "Included"; value: undefined }
  | { type: "TimedOut"; value: undefined }
  | { type: "Free"; value: undefined }
>;
export const PolkadotPrimitivesV5OccupiedCoreAssumption =
  _Enum as unknown as GetEnum<PolkadotPrimitivesV5OccupiedCoreAssumption>;

type I3gq2gui038qt = Anonymize<I5r8ef6aie125l> | undefined;

type I5r8ef6aie125l = {
  parent_head: Binary;
  relay_parent_number: number;
  relay_parent_storage_root: Binary;
  max_pov_size: number;
};

type Ic7as9gfl5utn9 = Anonymize<I9rov8gdjkv3b9> | undefined;

type I9rov8gdjkv3b9 = [Anonymize<I5r8ef6aie125l>, Binary];

type I2914et6njplkd = Anonymize<Iedmhjqij0hr8g> | undefined;

type Ianqtalutublq5 = Array<PolkadotPrimitivesV5CandidateEvent>;

export type PolkadotPrimitivesV5CandidateEvent = Enum<
  | { type: "CandidateBacked"; value: Anonymize<Ieno5vn1m65ng2> }
  | { type: "CandidateIncluded"; value: Anonymize<Ieno5vn1m65ng2> }
  | { type: "CandidateTimedOut"; value: Anonymize<Ievbvtucck5gnq> }
>;
export const PolkadotPrimitivesV5CandidateEvent =
  _Enum as unknown as GetEnum<PolkadotPrimitivesV5CandidateEvent>;

type I2pf0b05mc7sdr = Array<Anonymize<I9hvej6h53dqj0>>;

type I9hvej6h53dqj0 = [number, Anonymize<Iev3u09i2vqn93>];

type I6v0rmhf2dhasf = Anonymize<Ie7c40q4va8u7r> | undefined;

type I5537sfe0t0708 = Anonymize<I7k9oi9p83j43l> | undefined;

type I8bqhj6psoqsin = Array<Anonymize<Irsmd7gp7po60>>;

type Irsmd7gp7po60 = [number, Binary, Anonymize<I87u7jalc0lhah>];

type Ibt0rqpsbsimp1 = Anonymize<I79n5nka727sg9> | undefined;

type Icqsglphsrpv5 = Array<Anonymize<I5ork0l271qluj>>;

type I5ork0l271qluj = [number, Binary, Anonymize<I3g2jv3qmtkrbe>];

type I4gc64ora0pbbf = Anonymize<I53uro0ns8ba5b> | undefined;

type I53uro0ns8ba5b = {
  constraints: Anonymize<I269u13ficnsd5>;
  pending_availability: Anonymize<Ic05b0cuggt8lp>;
};

type I269u13ficnsd5 = {
  min_relay_parent_number: number;
  max_pov_size: number;
  max_code_size: number;
  ump_remaining: number;
  ump_remaining_bytes: number;
  max_ump_num_per_candidate: number;
  dmp_remaining_messages: Anonymize<Icgljjb6j82uhn>;
  hrmp_inbound: Anonymize<Icgljjb6j82uhn>;
  hrmp_channels_out: Anonymize<I1ilbeu6195gbh>;
  max_hrmp_num_per_candidate: number;
  required_parent: Binary;
  validation_code_hash: Binary;
  upgrade_restriction: Anonymize<Id34bi0o1gnln9>;
  future_validation_code: Anonymize<I7r2laa851pa4v>;
};

type I1ilbeu6195gbh = Array<Anonymize<If6i47cerum785>>;

type If6i47cerum785 = [number, Anonymize<Ifq5eqaefrc6it>];

type Ifq5eqaefrc6it = { bytes_remaining: number; messages_remaining: number };

type Id34bi0o1gnln9 = PolkadotPrimitivesV5UpgradeRestriction | undefined;

type I7r2laa851pa4v = Anonymize<If89923vhoiaim> | undefined;

type Ic05b0cuggt8lp = Array<Anonymize<Ids4u62ll2l1p4>>;

type Ids4u62ll2l1p4 = {
  candidate_hash: Binary;
  descriptor: Anonymize<Ib2u20s6roco9i>;
  commitments: Anonymize<Ic1d4u2opv3fst>;
  relay_parent_number: number;
  max_pov_size: number;
};

type I71rr2d0kb0iu3 = Anonymize<I17bb7o70l1ke3> | undefined;

type I17bb7o70l1ke3 = { validators: Anonymize<Ie5mvl0hn85mkc>; id: bigint };

type I7m8g2583l7nbe = ResultPayload<Binary, MmrPrimitivesError>;

export type MmrPrimitivesError = Enum<
  | { type: "InvalidNumericOp"; value: undefined }
  | { type: "Push"; value: undefined }
  | { type: "GetRoot"; value: undefined }
  | { type: "Commit"; value: undefined }
  | { type: "GenerateProof"; value: undefined }
  | { type: "Verify"; value: undefined }
  | { type: "LeafNotFound"; value: undefined }
  | { type: "PalletNotIncluded"; value: undefined }
  | { type: "InvalidLeafIndex"; value: undefined }
  | { type: "InvalidBestKnownBlock"; value: undefined }
>;
export const MmrPrimitivesError = _Enum as unknown as GetEnum<MmrPrimitivesError>;

type I256gkugrgtccq = ResultPayload<bigint, MmrPrimitivesError>;

type I32e5ntud5fl8i = ResultPayload<Anonymize<If93480lp8rssc>, MmrPrimitivesError>;

type If93480lp8rssc = [Anonymize<Itom7fk49o0c9>, Anonymize<I2ij509mgq3dve>];

type I2ij509mgq3dve = {
  leaf_indices: Anonymize<Iafqnechp3omqg>;
  leaf_count: bigint;
  items: Anonymize<Idhnf6rtqoslea>;
};

type Iafqnechp3omqg = Array<bigint>;

type I9lnenfklcdre = ResultPayload<undefined, MmrPrimitivesError>;

type Iap4lrotpgmqpv = {
  slot_duration: bigint;
  epoch_length: bigint;
  c: Anonymize<I2j729bmgsdiuo>;
  authorities: Anonymize<I2qinct8jq4bqe>;
  randomness: Binary;
  allowed_slots: BabeAllowedSlots;
};

type I6sbl80o3jmgfd = {
  epoch_index: bigint;
  start_slot: bigint;
  duration: bigint;
  authorities: Anonymize<I2qinct8jq4bqe>;
  randomness: Binary;
  config: Anonymize<Idkva8q2m9meg0>;
};

type I96ok5o1k21bpi = Anonymize<I4gkfq1hbsjrle> | undefined;

type I4gkfq1hbsjrle = Array<Anonymize<I3dmbm7ul207u0>>;

type I3dmbm7ul207u0 = [Binary, Binary];

type I5r4utvvlnltpd = {
  weight: Anonymize<I4q39t5hn830vp>;
  class: DispatchClass;
  partial_fee: bigint;
};

type Iei2mvq0mjvt81 = { inclusion_fee: Anonymize<Id37fum600qfau>; tip: bigint };

type Id37fum600qfau = Anonymize<I246faqtjrsnee> | undefined;

type I246faqtjrsnee = { base_fee: bigint; len_fee: bigint; adjusted_weight_fee: bigint };

type I9q9lvv0h98nhk = ResultPayload<undefined, string>;

const cf3ncnq6stc567 = "f3ncnq6stc567";
const cdhc577m5btfvc = "dhc577m5btfvc";
const cfmam6uono5akb = "fmam6uono5akb";
const c7i24fvhamijd = "7i24fvhamijd";
const cegurtje61sbu9 = "egurtje61sbu9";
const cb3qt1mgvgmbgi = "b3qt1mgvgmbgi";
const c7d2tqf029vcod = "7d2tqf029vcod";
const c2qinct8jq4bqe = "2qinct8jq4bqe";
const c5g2vv0ckl2m8b = "5g2vv0ckl2m8b";
const cdkva8q2m9meg0 = "dkva8q2m9meg0";
const cbtum7aot707m6 = "btum7aot707m6";
const c1e2385hi92o2f = "1e2385hi92o2f";
const cbe3psbb7upqsv = "be3psbb7upqsv";
const cbf1alv83g5fj0 = "bf1alv83g5fj0";
const cbfv6n704ucltm = "bfv6n704ucltm";
const c86esvcpejimq9 = "86esvcpejimq9";
const ca2lhg7l2hilo3 = "a2lhg7l2hilo3";
const ccgljjb6j82uhn = "cgljjb6j82uhn";
const c3u1nuj99f15hf = "3u1nuj99f15hf";
const cdhnf6rtqoslea = "dhnf6rtqoslea";
const c2snmki6ntodt = "2snmki6ntodt";
const c9hkvi4k3kh916 = "9hkvi4k3kh916";
const ca9db3r6lab3b6 = "a9db3r6lab3b6";
const cau4oj9ee8cokk = "au4oj9ee8cokk";
const cau2fn2kb28v1i = "au2fn2kb28v1i";
const cep4uo61810hfs = "ep4uo61810hfs";
const c5e37esnd8etqt = "5e37esnd8etqt";
const ccsbuok7olbg29 = "csbuok7olbg29";
const cd43g4eveajpkl = "d43g4eveajpkl";
const c4arjljr6dpflb = "4arjljr6dpflb";
const ce5mvl0hn85mkc = "e5mvl0hn85mkc";
const c3qm5u05tioq05 = "3qm5u05tioq05";
const c35p85j063s0il = "35p85j063s0il";
const c7l9d2ilde057d = "7l9d2ilde057d";
const cbafpkl9hhno69 = "bafpkl9hhno69";
const c4q39t5hn830vp = "4q39t5hn830vp";
const c8ofcg5rbj0g2c = "8ofcg5rbj0g2c";
const c6pjjpfvhvcfru = "6pjjpfvhvcfru";
const c6a5n5ij3gomuo = "6a5n5ij3gomuo";
const c7mmbgd20nut80 = "7mmbgd20nut80";
const c666bl2fqjkejo = "666bl2fqjkejo";
const cdge7gk9m5car0 = "dge7gk9m5car0";
const cen6q0lasi0m7i = "en6q0lasi0m7i";
const ccacgruoo9j3r2 = "cacgruoo9j3r2";
const c7dgmo7im9hljo = "7dgmo7im9hljo";
const cargojp1sv9icj = "argojp1sv9icj";
const cbmr18suc9ikh9 = "bmr18suc9ikh9";
const ce0io91hk7pejj = "e0io91hk7pejj";
const c3u7ci02rtsn3o = "3u7ci02rtsn3o";
const c39ibapn5dm6tr = "39ibapn5dm6tr";
const c4kjek1q6rj24q = "4kjek1q6rj24q";
const ccm9m0qeemu66d = "cm9m0qeemu66d";
const c8lsplvtf2fhfc = "8lsplvtf2fhfc";
const ccbio0e1f0034b = "cbio0e1f0034b";
const cfml0k0sf0mu2g = "fml0k0sf0mu2g";
const c2nmm2fprtp4kl = "2nmm2fprtp4kl";
const c8413rb6im3iko = "8413rb6im3iko";
const cb6n6j9tk6ut3b = "b6n6j9tk6ut3b";
const c1u2b1ro792h6p = "1u2b1ro792h6p";
const ce1uvn9e6ki8dd = "e1uvn9e6ki8dd";
const cbsu2pfvipmui6 = "bsu2pfvipmui6";
const c29er5j74l8bu = "29er5j74l8bu";
const cclf5v4qsadc12 = "clf5v4qsadc12";
const ce5l999tf7t2te = "e5l999tf7t2te";
const ce5v6njpckr05b = "e5v6njpckr05b";
const c3qt1hgg4djhgb = "3qt1hgg4djhgb";
const c4b66js88p45m8 = "4b66js88p45m8";
const cfddpcgiumovp6 = "fddpcgiumovp6";
const c5n4sebgkfr760 = "5n4sebgkfr760";
const cdsdstalforb09 = "dsdstalforb09";
const c7mmcnc01jgbim = "7mmcnc01jgbim";
const cd3bpmvju2iqi5 = "d3bpmvju2iqi5";
const cd9d48vaes3c53 = "d9d48vaes3c53";
const c4digl36cac7si = "4digl36cac7si";
const cdi8jsg8hkprub = "di8jsg8hkprub";
const ca9p5bg6p18r0i = "a9p5bg6p18r0i";
const c2gr10p66od9ch = "2gr10p66od9ch";
const ccckjru8f8flpj = "cckjru8f8flpj";
const c3vh014cqgmrfd = "3vh014cqgmrfd";
const cd581arok0b1nj = "d581arok0b1nj";
const c9jsikd1ghmc7l = "9jsikd1ghmc7l";
const c1k3urvkqqshbc = "1k3urvkqqshbc";
const c2ff0ffsh15vej = "2ff0ffsh15vej";
const caus4cb3drhu9q = "aus4cb3drhu9q";
const cd1baei7m8gkhk = "d1baei7m8gkhk";
const cc3430470j4mbv = "c3430470j4mbv";
const c50mrcbubp554e = "50mrcbubp554e";
const caa7g3f5tlv3gf = "aa7g3f5tlv3gf";
const cc5b47dj4coa3r = "c5b47dj4coa3r";
const c3js6c9fubdele = "3js6c9fubdele";
const ca1u3jll6a06ae = "a1u3jll6a06ae";
const cqg2kc7rjuace = "qg2kc7rjuace";
const cergfrmj3c9dor = "ergfrmj3c9dor";
const cb67j9jdqr7ekj = "b67j9jdqr7ekj";
const cbk7a73iljhv98 = "bk7a73iljhv98";
const c894urueu7skf3 = "894urueu7skf3";
const c4cbvqmqadhrea = "4cbvqmqadhrea";
const c9d5h5irbki7mm = "9d5h5irbki7mm";
const c97hfovkaaqb7h = "97hfovkaaqb7h";
const ccbccs0ug47ilf = "cbccs0ug47ilf";
const ccv68aq8841478 = "cv68aq8841478";
const cc262ibdoec56a = "c262ibdoec56a";
const cflcfm9b6nlmdd = "flcfm9b6nlmdd";
const cjrsf4mnp3eka = "jrsf4mnp3eka";
const cd5fm4p8lj5qgi = "d5fm4p8lj5qgi";
const cdm5rqp3duosod = "dm5rqp3duosod";
const c4fooe9dun9o0t = "4fooe9dun9o0t";
const cek7v4hrgnq6iv = "ek7v4hrgnq6iv";
const cdhr9v8mlnjej = "dhr9v8mlnjej";
const c62nte77gksm0f = "62nte77gksm0f";
const clhp45uime5tp = "lhp45uime5tp";
const c50aq0q2l1cdkr = "50aq0q2l1cdkr";
const cm1pm2vf6llcn = "m1pm2vf6llcn";
const c27notaksll8qt = "27notaksll8qt";
const ce5i8qqljk3tjb = "e5i8qqljk3tjb";
const cep1lmt6q3s6r3 = "ep1lmt6q3s6r3";
const c1fac16213rie2 = "1fac16213rie2";
const cck3mveut33f44 = "ck3mveut33f44";
const c2rg5btjrsqec0 = "2rg5btjrsqec0";
const cbdqerrooruuq9 = "bdqerrooruuq9";
const c8ircspqpe79k7 = "8ircspqpe79k7";
const ce3gphha4ejh40 = "e3gphha4ejh40";
const c9vkkue6cq74et = "9vkkue6cq74et";
const cbkv7dijodoblp = "bkv7dijodoblp";
const c3ce0ol160n343 = "3ce0ol160n343";
const c70sc1pdo8vtos = "70sc1pdo8vtos";
const c60p8l86a8cm59 = "60p8l86a8cm59";
const ce2vmgag6qiue4 = "e2vmgag6qiue4";
const ceno5vn1m65ng2 = "eno5vn1m65ng2";
const c34ha3jf7jm288 = "34ha3jf7jm288";
const c64gm4hrq7urum = "64gm4hrq7urum";
const cd2bej717ckub0 = "d2bej717ckub0";
const c545vo2e86o5i4 = "545vo2e86o5i4";
const c37r4bdai8o9mp = "37r4bdai8o9mp";
const c8ve4g3egaln6a = "8ve4g3egaln6a";
const cc8gqph940s142 = "c8gqph940s142";
const c2uqmls7kcdnii = "2uqmls7kcdnii";
const c20naetn2lovo9 = "20naetn2lovo9";
const c4fi6oaheho56l = "4fi6oaheho56l";
const cccvjqfmga2rn = "ccvjqfmga2rn";
const c331o7t2g0ooi9 = "331o7t2g0ooi9";

const stg_System_Account: StorageDescriptor<
  [Key: SS58String],
  {
    nonce: number;
    consumers: number;
    providers: number;
    sufficients: number;
    data: Anonymize<I1q8tnt1cluu5j>;
  },
  false
> = "4u7j8i23ciccr" as StorageDescriptor<
  [Key: SS58String],
  {
    nonce: number;
    consumers: number;
    providers: number;
    sufficients: number;
    data: Anonymize<I1q8tnt1cluu5j>;
  },
  false
>;
const stg_System_ExtrinsicCount: StorageDescriptor<[], number, true> =
  cf3ncnq6stc567 as StorageDescriptor<[], number, true>;
const stg_System_BlockWeight: StorageDescriptor<
  [],
  {
    normal: Anonymize<I4q39t5hn830vp>;
    operational: Anonymize<I4q39t5hn830vp>;
    mandatory: Anonymize<I4q39t5hn830vp>;
  },
  false
> = "ffmde3ekjedi9" as StorageDescriptor<
  [],
  {
    normal: Anonymize<I4q39t5hn830vp>;
    operational: Anonymize<I4q39t5hn830vp>;
    mandatory: Anonymize<I4q39t5hn830vp>;
  },
  false
>;
const stg_System_AllExtrinsicsLen: StorageDescriptor<[], number, true> =
  cf3ncnq6stc567 as StorageDescriptor<[], number, true>;
const stg_System_BlockHash: StorageDescriptor<[Key: number], Binary, false> =
  cdhc577m5btfvc as StorageDescriptor<[Key: number], Binary, false>;
const stg_System_ExtrinsicData: StorageDescriptor<[Key: number], Binary, false> =
  cfmam6uono5akb as StorageDescriptor<[Key: number], Binary, false>;
const stg_System_Number: StorageDescriptor<[], number, false> = cf3ncnq6stc567 as StorageDescriptor<
  [],
  number,
  false
>;
const stg_System_ParentHash: StorageDescriptor<[], Binary, false> =
  c7i24fvhamijd as StorageDescriptor<[], Binary, false>;
const stg_System_Digest: StorageDescriptor<
  [],
  Array<DigestItem>,
  false
> = "din6nhq46lvdj" as StorageDescriptor<[], Array<DigestItem>, false>;
const stg_System_Events: StorageDescriptor<
  [],
  Array<Anonymize<I9pt9i86qkqfgf>>,
  false
> = "2oomvrncvb8jh" as StorageDescriptor<[], Array<Anonymize<I9pt9i86qkqfgf>>, false>;
const stg_System_EventCount: StorageDescriptor<[], number, false> =
  cf3ncnq6stc567 as StorageDescriptor<[], number, false>;
const stg_System_EventTopics: StorageDescriptor<
  [Key: Binary],
  Array<Anonymize<I5g2vv0ckl2m8b>>,
  false
> = "b9d1sol3oue21" as StorageDescriptor<[Key: Binary], Array<Anonymize<I5g2vv0ckl2m8b>>, false>;
const stg_System_LastRuntimeUpgrade: StorageDescriptor<
  [],
  { spec_version: number; spec_name: string },
  true
> = "eniouoqkq4icf" as StorageDescriptor<[], { spec_version: number; spec_name: string }, true>;
const stg_System_UpgradedToU32RefCount: StorageDescriptor<[], boolean, false> =
  cegurtje61sbu9 as StorageDescriptor<[], boolean, false>;
const stg_System_UpgradedToTripleRefCount: StorageDescriptor<[], boolean, false> =
  cegurtje61sbu9 as StorageDescriptor<[], boolean, false>;
const stg_System_ExecutionPhase: StorageDescriptor<[], Phase, true> =
  "a3md08567vo9e" as StorageDescriptor<[], Phase, true>;
const stg_System_AuthorizedUpgrade: StorageDescriptor<
  [],
  { code_hash: Binary; check_version: boolean },
  true
> = cb3qt1mgvgmbgi as StorageDescriptor<[], { code_hash: Binary; check_version: boolean }, true>;
const stg_Babe_EpochIndex: StorageDescriptor<[], bigint, false> =
  c7d2tqf029vcod as StorageDescriptor<[], bigint, false>;
const stg_Babe_Authorities: StorageDescriptor<
  [],
  Array<Anonymize<I3iuggguvi9njj>>,
  false
> = c2qinct8jq4bqe as StorageDescriptor<[], Array<Anonymize<I3iuggguvi9njj>>, false>;
const stg_Babe_GenesisSlot: StorageDescriptor<[], bigint, false> =
  c7d2tqf029vcod as StorageDescriptor<[], bigint, false>;
const stg_Babe_CurrentSlot: StorageDescriptor<[], bigint, false> =
  c7d2tqf029vcod as StorageDescriptor<[], bigint, false>;
const stg_Babe_Randomness: StorageDescriptor<[], Binary, false> =
  c7i24fvhamijd as StorageDescriptor<[], Binary, false>;
const stg_Babe_PendingEpochConfigChange: StorageDescriptor<
  [],
  BabeDigestsNextConfigDescriptor,
  true
> = "dbrkktcap6cr5" as StorageDescriptor<[], BabeDigestsNextConfigDescriptor, true>;
const stg_Babe_NextRandomness: StorageDescriptor<[], Binary, false> =
  c7i24fvhamijd as StorageDescriptor<[], Binary, false>;
const stg_Babe_NextAuthorities: StorageDescriptor<
  [],
  Array<Anonymize<I3iuggguvi9njj>>,
  false
> = c2qinct8jq4bqe as StorageDescriptor<[], Array<Anonymize<I3iuggguvi9njj>>, false>;
const stg_Babe_SegmentIndex: StorageDescriptor<[], number, false> =
  cf3ncnq6stc567 as StorageDescriptor<[], number, false>;
const stg_Babe_UnderConstruction: StorageDescriptor<
  [Key: number],
  Array<Binary>,
  false
> = "dqa2fau59jv7f" as StorageDescriptor<[Key: number], Array<Binary>, false>;
const stg_Babe_Initialized: StorageDescriptor<[], BabeDigestsPreDigest | undefined, true> =
  "7eh0hrps1dlgp" as StorageDescriptor<[], BabeDigestsPreDigest | undefined, true>;
const stg_Babe_AuthorVrfRandomness: StorageDescriptor<[], Binary | undefined, false> =
  "17k3ujudqd5df" as StorageDescriptor<[], Binary | undefined, false>;
const stg_Babe_EpochStart: StorageDescriptor<[], [number, number], false> =
  c5g2vv0ckl2m8b as StorageDescriptor<[], [number, number], false>;
const stg_Babe_Lateness: StorageDescriptor<[], number, false> = cf3ncnq6stc567 as StorageDescriptor<
  [],
  number,
  false
>;
const stg_Babe_EpochConfig: StorageDescriptor<
  [],
  { c: Anonymize<I2j729bmgsdiuo>; allowed_slots: BabeAllowedSlots },
  true
> = cdkva8q2m9meg0 as StorageDescriptor<
  [],
  { c: Anonymize<I2j729bmgsdiuo>; allowed_slots: BabeAllowedSlots },
  true
>;
const stg_Babe_NextEpochConfig: StorageDescriptor<
  [],
  { c: Anonymize<I2j729bmgsdiuo>; allowed_slots: BabeAllowedSlots },
  true
> = cdkva8q2m9meg0 as StorageDescriptor<
  [],
  { c: Anonymize<I2j729bmgsdiuo>; allowed_slots: BabeAllowedSlots },
  true
>;
const stg_Babe_SkippedEpochs: StorageDescriptor<
  [],
  Array<Anonymize<I6cs1itejju2vv>>,
  false
> = "fip05kcrl65am" as StorageDescriptor<[], Array<Anonymize<I6cs1itejju2vv>>, false>;
const stg_Timestamp_Now: StorageDescriptor<[], bigint, false> = c7d2tqf029vcod as StorageDescriptor<
  [],
  bigint,
  false
>;
const stg_Timestamp_DidUpdate: StorageDescriptor<[], boolean, false> =
  cegurtje61sbu9 as StorageDescriptor<[], boolean, false>;
const stg_Indices_Accounts: StorageDescriptor<[Key: number], [SS58String, bigint, boolean], true> =
  "eie54ehffvlk9" as StorageDescriptor<[Key: number], [SS58String, bigint, boolean], true>;
const stg_Balances_TotalIssuance: StorageDescriptor<[], bigint, false> =
  c7d2tqf029vcod as StorageDescriptor<[], bigint, false>;
const stg_Balances_InactiveIssuance: StorageDescriptor<[], bigint, false> =
  c7d2tqf029vcod as StorageDescriptor<[], bigint, false>;
const stg_Balances_Account: StorageDescriptor<
  [Key: SS58String],
  { free: bigint; reserved: bigint; frozen: bigint; flags: bigint },
  false
> = cbtum7aot707m6 as StorageDescriptor<
  [Key: SS58String],
  { free: bigint; reserved: bigint; frozen: bigint; flags: bigint },
  false
>;
const stg_Balances_Locks: StorageDescriptor<
  [Key: SS58String],
  Array<Anonymize<I5b29v4qfq4tu7>>,
  false
> = c1e2385hi92o2f as StorageDescriptor<[Key: SS58String], Array<Anonymize<I5b29v4qfq4tu7>>, false>;
const stg_Balances_Reserves: StorageDescriptor<
  [Key: SS58String],
  Array<Anonymize<I32btm6htd9bck>>,
  false
> = cbe3psbb7upqsv as StorageDescriptor<[Key: SS58String], Array<Anonymize<I32btm6htd9bck>>, false>;
const stg_Balances_Holds: StorageDescriptor<
  [Key: SS58String],
  Array<Anonymize<Iemrlqc7oc4k5g>>,
  false
> = cbf1alv83g5fj0 as StorageDescriptor<[Key: SS58String], Array<Anonymize<Iemrlqc7oc4k5g>>, false>;
const stg_Balances_Freezes: StorageDescriptor<
  [Key: SS58String],
  Array<Anonymize<I7qdm60946h5u9>>,
  false
> = cbfv6n704ucltm as StorageDescriptor<[Key: SS58String], Array<Anonymize<I7qdm60946h5u9>>, false>;
const stg_TransactionPayment_NextFeeMultiplier: StorageDescriptor<[], bigint, false> =
  c7d2tqf029vcod as StorageDescriptor<[], bigint, false>;
const stg_TransactionPayment_StorageVersion: StorageDescriptor<
  [],
  TransactionPaymentReleases,
  false
> = "944994o3qcrkq" as StorageDescriptor<[], TransactionPaymentReleases, false>;
const stg_Authorship_Author: StorageDescriptor<[], SS58String, true> =
  c86esvcpejimq9 as StorageDescriptor<[], SS58String, true>;
const stg_Offences_Reports: StorageDescriptor<
  [Key: Binary],
  { offender: Anonymize<Iidji8iemeki4>; reporters: Anonymize<Ia2lhg7l2hilo3> },
  true
> = "7rict23hfjelv" as StorageDescriptor<
  [Key: Binary],
  { offender: Anonymize<Iidji8iemeki4>; reporters: Anonymize<Ia2lhg7l2hilo3> },
  true
>;
const stg_Offences_ConcurrentReportsIndex: StorageDescriptor<
  [Binary, Binary],
  Array<Binary>,
  false
> = "5jnsgk9uqigfd" as StorageDescriptor<[Binary, Binary], Array<Binary>, false>;
const stg_Historical_HistoricalSessions: StorageDescriptor<[Key: number], [Binary, number], true> =
  "d34dbh29cfct1" as StorageDescriptor<[Key: number], [Binary, number], true>;
const stg_Historical_StoredRange: StorageDescriptor<[], [number, number], true> =
  c5g2vv0ckl2m8b as StorageDescriptor<[], [number, number], true>;
const stg_Session_Validators: StorageDescriptor<
  [],
  Array<SS58String>,
  false
> = ca2lhg7l2hilo3 as StorageDescriptor<[], Array<SS58String>, false>;
const stg_Session_CurrentIndex: StorageDescriptor<[], number, false> =
  cf3ncnq6stc567 as StorageDescriptor<[], number, false>;
const stg_Session_QueuedChanged: StorageDescriptor<[], boolean, false> =
  cegurtje61sbu9 as StorageDescriptor<[], boolean, false>;
const stg_Session_QueuedKeys: StorageDescriptor<
  [],
  Array<Anonymize<I9mnddod4k7nfj>>,
  false
> = "avru0osmavfj6" as StorageDescriptor<[], Array<Anonymize<I9mnddod4k7nfj>>, false>;
const stg_Session_DisabledValidators: StorageDescriptor<
  [],
  Array<number>,
  false
> = ccgljjb6j82uhn as StorageDescriptor<[], Array<number>, false>;
const stg_Session_NextKeys: StorageDescriptor<
  [Key: SS58String],
  {
    grandpa: Binary;
    babe: Binary;
    para_validator: Binary;
    para_assignment: Binary;
    authority_discovery: Binary;
    beefy: Binary;
  },
  true
> = "a1pvuqg3jk4ip" as StorageDescriptor<
  [Key: SS58String],
  {
    grandpa: Binary;
    babe: Binary;
    para_validator: Binary;
    para_assignment: Binary;
    authority_discovery: Binary;
    beefy: Binary;
  },
  true
>;
const stg_Session_KeyOwner: StorageDescriptor<[Key: [Binary, Binary]], SS58String, true> =
  "8drjr6qpp26i0" as StorageDescriptor<[Key: [Binary, Binary]], SS58String, true>;
const stg_Grandpa_State: StorageDescriptor<[], GrandpaStoredState, false> =
  "9atcbaq1i12p8" as StorageDescriptor<[], GrandpaStoredState, false>;
const stg_Grandpa_PendingChange: StorageDescriptor<
  [],
  {
    scheduled_at: number;
    delay: number;
    next_authorities: Anonymize<I2qinct8jq4bqe>;
    forced: Anonymize<I4arjljr6dpflb>;
  },
  true
> = "bu50q5vhsfq5e" as StorageDescriptor<
  [],
  {
    scheduled_at: number;
    delay: number;
    next_authorities: Anonymize<I2qinct8jq4bqe>;
    forced: Anonymize<I4arjljr6dpflb>;
  },
  true
>;
const stg_Grandpa_NextForced: StorageDescriptor<[], number, true> =
  cf3ncnq6stc567 as StorageDescriptor<[], number, true>;
const stg_Grandpa_Stalled: StorageDescriptor<[], [number, number], true> =
  c5g2vv0ckl2m8b as StorageDescriptor<[], [number, number], true>;
const stg_Grandpa_CurrentSetId: StorageDescriptor<[], bigint, false> =
  c7d2tqf029vcod as StorageDescriptor<[], bigint, false>;
const stg_Grandpa_SetIdSession: StorageDescriptor<[Key: bigint], number, true> =
  c3u1nuj99f15hf as StorageDescriptor<[Key: bigint], number, true>;
const stg_Grandpa_Authorities: StorageDescriptor<
  [],
  Array<Anonymize<I3iuggguvi9njj>>,
  false
> = c2qinct8jq4bqe as StorageDescriptor<[], Array<Anonymize<I3iuggguvi9njj>>, false>;
const stg_AuthorityDiscovery_Keys: StorageDescriptor<
  [],
  Array<Binary>,
  false
> = cdhnf6rtqoslea as StorageDescriptor<[], Array<Binary>, false>;
const stg_AuthorityDiscovery_NextKeys: StorageDescriptor<
  [],
  Array<Binary>,
  false
> = cdhnf6rtqoslea as StorageDescriptor<[], Array<Binary>, false>;
const stg_Treasury_ProposalCount: StorageDescriptor<[], number, false> =
  cf3ncnq6stc567 as StorageDescriptor<[], number, false>;
const stg_Treasury_Proposals: StorageDescriptor<
  [Key: number],
  { proposer: SS58String; value: bigint; beneficiary: SS58String; bond: bigint },
  true
> = "91okf2u22en9d" as StorageDescriptor<
  [Key: number],
  { proposer: SS58String; value: bigint; beneficiary: SS58String; bond: bigint },
  true
>;
const stg_Treasury_Deactivated: StorageDescriptor<[], bigint, false> =
  c7d2tqf029vcod as StorageDescriptor<[], bigint, false>;
const stg_Treasury_Approvals: StorageDescriptor<
  [],
  Array<number>,
  false
> = ccgljjb6j82uhn as StorageDescriptor<[], Array<number>, false>;
const stg_Treasury_SpendCount: StorageDescriptor<[], number, false> =
  cf3ncnq6stc567 as StorageDescriptor<[], number, false>;
const stg_Treasury_Spends: StorageDescriptor<
  [Key: number],
  {
    asset_kind: PolkadotRuntimeCommonImplsVersionedLocatableAsset;
    amount: bigint;
    beneficiary: XcmVersionedLocation;
    valid_from: number;
    expire_at: number;
    status: TreasuryPaymentState;
  },
  true
> = "c68htk26j7ji3" as StorageDescriptor<
  [Key: number],
  {
    asset_kind: PolkadotRuntimeCommonImplsVersionedLocatableAsset;
    amount: bigint;
    beneficiary: XcmVersionedLocation;
    valid_from: number;
    expire_at: number;
    status: TreasuryPaymentState;
  },
  true
>;
const stg_ConvictionVoting_VotingFor: StorageDescriptor<
  [SS58String, number],
  ConvictionVotingVoteVoting,
  false
> = "d2utt7cpsnpeg" as StorageDescriptor<[SS58String, number], ConvictionVotingVoteVoting, false>;
const stg_ConvictionVoting_ClassLocksFor: StorageDescriptor<
  [Key: SS58String],
  Array<Anonymize<I4ojmnsk1dchql>>,
  false
> = "c5g150o6tnujs" as StorageDescriptor<
  [Key: SS58String],
  Array<Anonymize<I4ojmnsk1dchql>>,
  false
>;
const stg_Referenda_ReferendumCount: StorageDescriptor<[], number, false> =
  cf3ncnq6stc567 as StorageDescriptor<[], number, false>;
const stg_Referenda_ReferendumInfoFor: StorageDescriptor<
  [Key: number],
  ReferendaTypesReferendumInfo,
  true
> = "am93e8v3cifij" as StorageDescriptor<[Key: number], ReferendaTypesReferendumInfo, true>;
const stg_Referenda_TrackQueue: StorageDescriptor<
  [Key: number],
  Array<Anonymize<I4ojmnsk1dchql>>,
  false
> = "3aojtj9s5c6p0" as StorageDescriptor<[Key: number], Array<Anonymize<I4ojmnsk1dchql>>, false>;
const stg_Referenda_DecidingCount: StorageDescriptor<[Key: number], number, false> =
  c2snmki6ntodt as StorageDescriptor<[Key: number], number, false>;
const stg_Referenda_MetadataOf: StorageDescriptor<[Key: number], Binary, true> =
  cdhc577m5btfvc as StorageDescriptor<[Key: number], Binary, true>;
const stg_FellowshipCollective_MemberCount: StorageDescriptor<[Key: number], number, false> =
  c2snmki6ntodt as StorageDescriptor<[Key: number], number, false>;
const stg_FellowshipCollective_Members: StorageDescriptor<[Key: SS58String], number, true> =
  "44uqc9gktfck1" as StorageDescriptor<[Key: SS58String], number, true>;
const stg_FellowshipCollective_IdToIndex: StorageDescriptor<[number, SS58String], number, true> =
  "79l89n8ard6m" as StorageDescriptor<[number, SS58String], number, true>;
const stg_FellowshipCollective_IndexToId: StorageDescriptor<[number, number], SS58String, true> =
  "55521bp75hkst" as StorageDescriptor<[number, number], SS58String, true>;
const stg_FellowshipCollective_Voting: StorageDescriptor<
  [number, SS58String],
  Anonymize<AnonymousEnum<{ Aye: Anonymize<number>; Nay: Anonymize<number> }>>,
  true
> = "90au5nba8i6ue" as StorageDescriptor<
  [number, SS58String],
  Anonymize<AnonymousEnum<{ Aye: Anonymize<number>; Nay: Anonymize<number> }>>,
  true
>;
const stg_FellowshipCollective_VotingCleanup: StorageDescriptor<[Key: number], Binary, true> =
  cfmam6uono5akb as StorageDescriptor<[Key: number], Binary, true>;
const stg_FellowshipReferenda_ReferendumCount: StorageDescriptor<[], number, false> =
  cf3ncnq6stc567 as StorageDescriptor<[], number, false>;
const stg_FellowshipReferenda_ReferendumInfoFor: StorageDescriptor<
  [Key: number],
  Anonymize<
    AnonymousEnum<{
      Ongoing: Anonymize<Ieeijqsusb4a5r>;
      Approved: Anonymize<Ini94eljn5lj8>;
      Rejected: Anonymize<Ini94eljn5lj8>;
      Cancelled: Anonymize<Ini94eljn5lj8>;
      TimedOut: Anonymize<Ini94eljn5lj8>;
      Killed: Anonymize<number>;
    }>
  >,
  true
> = "1c9vcgqasifog" as StorageDescriptor<
  [Key: number],
  Anonymize<
    AnonymousEnum<{
      Ongoing: Anonymize<Ieeijqsusb4a5r>;
      Approved: Anonymize<Ini94eljn5lj8>;
      Rejected: Anonymize<Ini94eljn5lj8>;
      Cancelled: Anonymize<Ini94eljn5lj8>;
      TimedOut: Anonymize<Ini94eljn5lj8>;
      Killed: Anonymize<number>;
    }>
  >,
  true
>;
const stg_FellowshipReferenda_TrackQueue: StorageDescriptor<
  [Key: number],
  Array<Anonymize<I5g2vv0ckl2m8b>>,
  false
> = "ar2fosddkt418" as StorageDescriptor<[Key: number], Array<Anonymize<I5g2vv0ckl2m8b>>, false>;
const stg_FellowshipReferenda_DecidingCount: StorageDescriptor<[Key: number], number, false> =
  c2snmki6ntodt as StorageDescriptor<[Key: number], number, false>;
const stg_FellowshipReferenda_MetadataOf: StorageDescriptor<[Key: number], Binary, true> =
  cdhc577m5btfvc as StorageDescriptor<[Key: number], Binary, true>;
const stg_Whitelist_WhitelistedCall: StorageDescriptor<[Key: Binary], undefined, true> =
  "2pgep7i8dce2o" as StorageDescriptor<[Key: Binary], undefined, true>;
const stg_Claims_Claims: StorageDescriptor<[Key: Binary], bigint, true> =
  "de707245g30i" as StorageDescriptor<[Key: Binary], bigint, true>;
const stg_Claims_Total: StorageDescriptor<[], bigint, false> = c7d2tqf029vcod as StorageDescriptor<
  [],
  bigint,
  false
>;
const stg_Claims_Vesting: StorageDescriptor<[Key: Binary], [bigint, bigint, number], true> =
  "ds6mu9uf8odml" as StorageDescriptor<[Key: Binary], [bigint, bigint, number], true>;
const stg_Claims_Signing: StorageDescriptor<[Key: Binary], ClaimsStatementKind, true> =
  "cpugkad2ei7ko" as StorageDescriptor<[Key: Binary], ClaimsStatementKind, true>;
const stg_Claims_Preclaims: StorageDescriptor<[Key: SS58String], Binary, true> =
  "cmettfsqoveue" as StorageDescriptor<[Key: SS58String], Binary, true>;
const stg_Identity_IdentityOf: StorageDescriptor<
  [Key: SS58String],
  [Anonymize<I60cr97f9of5s5>, Anonymize<Iabpgqcjikia83>],
  true
> = "4177e23me4ui1" as StorageDescriptor<
  [Key: SS58String],
  [Anonymize<I60cr97f9of5s5>, Anonymize<Iabpgqcjikia83>],
  true
>;
const stg_Identity_SuperOf: StorageDescriptor<
  [Key: SS58String],
  [SS58String, IdentityTypesData],
  true
> = "culsi9ibos67i" as StorageDescriptor<[Key: SS58String], [SS58String, IdentityTypesData], true>;
const stg_Identity_SubsOf: StorageDescriptor<
  [Key: SS58String],
  [bigint, Anonymize<Ia2lhg7l2hilo3>],
  false
> = "8b0o8nk1fee6p" as StorageDescriptor<
  [Key: SS58String],
  [bigint, Anonymize<Ia2lhg7l2hilo3>],
  false
>;
const stg_Identity_Registrars: StorageDescriptor<
  [],
  Array<Anonymize<I48v3sekdprq30>>,
  false
> = "74af64m08r6as" as StorageDescriptor<[], Array<Anonymize<I48v3sekdprq30>>, false>;
const stg_Identity_UsernameAuthorities: StorageDescriptor<
  [Key: SS58String],
  { suffix: Binary; allocation: number },
  true
> = "4ijo1cmmcgasr" as StorageDescriptor<
  [Key: SS58String],
  { suffix: Binary; allocation: number },
  true
>;
const stg_Identity_AccountOfUsername: StorageDescriptor<[Key: Binary], SS58String, true> =
  "e8n69tjjl9afb" as StorageDescriptor<[Key: Binary], SS58String, true>;
const stg_Identity_PendingUsernames: StorageDescriptor<[Key: Binary], [SS58String, number], true> =
  "ck4cg2931n5m9" as StorageDescriptor<[Key: Binary], [SS58String, number], true>;
const stg_Society_Parameters: StorageDescriptor<
  [],
  { max_members: number; max_intake: number; max_strikes: number; candidate_deposit: bigint },
  true
> = c9hkvi4k3kh916 as StorageDescriptor<
  [],
  { max_members: number; max_intake: number; max_strikes: number; candidate_deposit: bigint },
  true
>;
const stg_Society_Pot: StorageDescriptor<[], bigint, false> = c7d2tqf029vcod as StorageDescriptor<
  [],
  bigint,
  false
>;
const stg_Society_Founder: StorageDescriptor<[], SS58String, true> =
  c86esvcpejimq9 as StorageDescriptor<[], SS58String, true>;
const stg_Society_Head: StorageDescriptor<[], SS58String, true> =
  c86esvcpejimq9 as StorageDescriptor<[], SS58String, true>;
const stg_Society_Rules: StorageDescriptor<[], Binary, true> = c7i24fvhamijd as StorageDescriptor<
  [],
  Binary,
  true
>;
const stg_Society_Members: StorageDescriptor<
  [Key: SS58String],
  { rank: number; strikes: number; vouching: Anonymize<Iceunvs0fkhdlv>; index: number },
  true
> = ca9db3r6lab3b6 as StorageDescriptor<
  [Key: SS58String],
  { rank: number; strikes: number; vouching: Anonymize<Iceunvs0fkhdlv>; index: number },
  true
>;
const stg_Society_Payouts: StorageDescriptor<
  [Key: SS58String],
  { paid: bigint; payouts: Anonymize<If9jidduiuq7vv> },
  false
> = "7pshtr9c3meps" as StorageDescriptor<
  [Key: SS58String],
  { paid: bigint; payouts: Anonymize<If9jidduiuq7vv> },
  false
>;
const stg_Society_MemberCount: StorageDescriptor<[], number, false> =
  cf3ncnq6stc567 as StorageDescriptor<[], number, false>;
const stg_Society_MemberByIndex: StorageDescriptor<[Key: number], SS58String, true> =
  "7ld384v28l458" as StorageDescriptor<[Key: number], SS58String, true>;
const stg_Society_SuspendedMembers: StorageDescriptor<
  [Key: SS58String],
  { rank: number; strikes: number; vouching: Anonymize<Iceunvs0fkhdlv>; index: number },
  true
> = ca9db3r6lab3b6 as StorageDescriptor<
  [Key: SS58String],
  { rank: number; strikes: number; vouching: Anonymize<Iceunvs0fkhdlv>; index: number },
  true
>;
const stg_Society_RoundCount: StorageDescriptor<[], number, false> =
  cf3ncnq6stc567 as StorageDescriptor<[], number, false>;
const stg_Society_Bids: StorageDescriptor<
  [],
  Array<Anonymize<I5cap79lpfupvc>>,
  false
> = "aru90csm3chpv" as StorageDescriptor<[], Array<Anonymize<I5cap79lpfupvc>>, false>;
const stg_Society_Candidates: StorageDescriptor<
  [Key: SS58String],
  {
    round: number;
    kind: Anonymize<Idvcmh0j8tecg6>;
    bid: bigint;
    tally: Anonymize<Ic4n54m09tc5na>;
    skeptic_struck: boolean;
  },
  true
> = "2fo79bpe7456o" as StorageDescriptor<
  [Key: SS58String],
  {
    round: number;
    kind: Anonymize<Idvcmh0j8tecg6>;
    bid: bigint;
    tally: Anonymize<Ic4n54m09tc5na>;
    skeptic_struck: boolean;
  },
  true
>;
const stg_Society_Skeptic: StorageDescriptor<[], SS58String, true> =
  c86esvcpejimq9 as StorageDescriptor<[], SS58String, true>;
const stg_Society_Votes: StorageDescriptor<
  [SS58String, SS58String],
  { approve: boolean; weight: number },
  true
> = "e0p3f321rs5j1" as StorageDescriptor<
  [SS58String, SS58String],
  { approve: boolean; weight: number },
  true
>;
const stg_Society_VoteClearCursor: StorageDescriptor<[Key: SS58String], Binary, true> =
  "ugv2v6jos0e6" as StorageDescriptor<[Key: SS58String], Binary, true>;
const stg_Society_NextHead: StorageDescriptor<
  [],
  { who: SS58String; bid: bigint; round: number },
  true
> = "7mgf69gvksvu2" as StorageDescriptor<[], { who: SS58String; bid: bigint; round: number }, true>;
const stg_Society_ChallengeRoundCount: StorageDescriptor<[], number, false> =
  cf3ncnq6stc567 as StorageDescriptor<[], number, false>;
const stg_Society_Defending: StorageDescriptor<
  [],
  [SS58String, SS58String, Anonymize<Ic4n54m09tc5na>],
  true
> = "3sfg6klg56vmk" as StorageDescriptor<
  [],
  [SS58String, SS58String, Anonymize<Ic4n54m09tc5na>],
  true
>;
const stg_Society_DefenderVotes: StorageDescriptor<
  [number, SS58String],
  { approve: boolean; weight: number },
  true
> = "eml6bo7gdm781" as StorageDescriptor<
  [number, SS58String],
  { approve: boolean; weight: number },
  true
>;
const stg_Recovery_Recoverable: StorageDescriptor<
  [Key: SS58String],
  { delay_period: number; deposit: bigint; friends: Anonymize<Ia2lhg7l2hilo3>; threshold: number },
  true
> = "9635ncec3aq65" as StorageDescriptor<
  [Key: SS58String],
  { delay_period: number; deposit: bigint; friends: Anonymize<Ia2lhg7l2hilo3>; threshold: number },
  true
>;
const stg_Recovery_ActiveRecoveries: StorageDescriptor<
  [SS58String, SS58String],
  { created: number; deposit: bigint; friends: Anonymize<Ia2lhg7l2hilo3> },
  true
> = "82s4jq8q5st24" as StorageDescriptor<
  [SS58String, SS58String],
  { created: number; deposit: bigint; friends: Anonymize<Ia2lhg7l2hilo3> },
  true
>;
const stg_Recovery_Proxy: StorageDescriptor<[Key: SS58String], SS58String, true> =
  "4j0n8mcbuvkci" as StorageDescriptor<[Key: SS58String], SS58String, true>;
const stg_Vesting_Vesting: StorageDescriptor<
  [Key: SS58String],
  Array<Anonymize<I4aro1m78pdrtt>>,
  true
> = "13g9ahdidi9g6" as StorageDescriptor<[Key: SS58String], Array<Anonymize<I4aro1m78pdrtt>>, true>;
const stg_Vesting_StorageVersion: StorageDescriptor<[], VestingReleases, false> =
  "f6v8gsdlcpu8j" as StorageDescriptor<[], VestingReleases, false>;
const stg_Scheduler_IncompleteSince: StorageDescriptor<[], number, true> =
  cf3ncnq6stc567 as StorageDescriptor<[], number, true>;
const stg_Scheduler_Agenda: StorageDescriptor<
  [Key: number],
  Array<Anonymize<Ife7f8jq02avq2>>,
  false
> = "40id79lk2n8u8" as StorageDescriptor<[Key: number], Array<Anonymize<Ife7f8jq02avq2>>, false>;
const stg_Scheduler_Retries: StorageDescriptor<
  [Key: [number, number]],
  { total_retries: number; remaining: number; period: number },
  true
> = "1vo83pc7u3r0s" as StorageDescriptor<
  [Key: [number, number]],
  { total_retries: number; remaining: number; period: number },
  true
>;
const stg_Scheduler_Lookup: StorageDescriptor<[Key: Binary], [number, number], true> =
  "1lvn5l8onnkmp" as StorageDescriptor<[Key: Binary], [number, number], true>;
const stg_Proxy_Proxies: StorageDescriptor<
  [Key: SS58String],
  [Anonymize<I4n424tm4s3ajv>, bigint],
  false
> = "3lp2sk5b0rq6" as StorageDescriptor<
  [Key: SS58String],
  [Anonymize<I4n424tm4s3ajv>, bigint],
  false
>;
const stg_Proxy_Announcements: StorageDescriptor<
  [Key: SS58String],
  [Anonymize<I99svmvk5amkcq>, bigint],
  false
> = "bkn8jef9a7jau" as StorageDescriptor<
  [Key: SS58String],
  [Anonymize<I99svmvk5amkcq>, bigint],
  false
>;
const stg_Multisig_Multisigs: StorageDescriptor<
  [SS58String, Binary],
  {
    when: Anonymize<Itvprrpb0nm3o>;
    deposit: bigint;
    depositor: SS58String;
    approvals: Anonymize<Ia2lhg7l2hilo3>;
  },
  true
> = "4hs7g508a8hnc" as StorageDescriptor<
  [SS58String, Binary],
  {
    when: Anonymize<Itvprrpb0nm3o>;
    deposit: bigint;
    depositor: SS58String;
    approvals: Anonymize<Ia2lhg7l2hilo3>;
  },
  true
>;
const stg_Preimage_StatusFor: StorageDescriptor<[Key: Binary], PreimageRequestStatus, true> =
  "efdvjds2mvqk7" as StorageDescriptor<[Key: Binary], PreimageRequestStatus, true>;
const stg_Preimage_RequestStatusFor: StorageDescriptor<
  [Key: Binary],
  PreimageRequestStatus1,
  true
> = "20nddnciorh2q" as StorageDescriptor<[Key: Binary], PreimageRequestStatus1, true>;
const stg_Preimage_PreimageFor: StorageDescriptor<[Key: [Binary, number]], Binary, true> =
  "6q1snsvkrbatm" as StorageDescriptor<[Key: [Binary, number]], Binary, true>;
const stg_AssetRate_ConversionRateToNative: StorageDescriptor<
  [Key: PolkadotRuntimeCommonImplsVersionedLocatableAsset],
  bigint,
  true
> = "4kqs2s14dfpg6" as StorageDescriptor<
  [Key: PolkadotRuntimeCommonImplsVersionedLocatableAsset],
  bigint,
  true
>;
const stg_Bounties_BountyCount: StorageDescriptor<[], number, false> =
  cf3ncnq6stc567 as StorageDescriptor<[], number, false>;
const stg_Bounties_Bounties: StorageDescriptor<
  [Key: number],
  {
    proposer: SS58String;
    value: bigint;
    fee: bigint;
    curator_deposit: bigint;
    bond: bigint;
    status: BountiesBountyStatus;
  },
  true
> = "3d8o4cndintp6" as StorageDescriptor<
  [Key: number],
  {
    proposer: SS58String;
    value: bigint;
    fee: bigint;
    curator_deposit: bigint;
    bond: bigint;
    status: BountiesBountyStatus;
  },
  true
>;
const stg_Bounties_BountyDescriptions: StorageDescriptor<[Key: number], Binary, true> =
  cfmam6uono5akb as StorageDescriptor<[Key: number], Binary, true>;
const stg_Bounties_BountyApprovals: StorageDescriptor<
  [],
  Array<number>,
  false
> = ccgljjb6j82uhn as StorageDescriptor<[], Array<number>, false>;
const stg_ChildBounties_ChildBountyCount: StorageDescriptor<[], number, false> =
  cf3ncnq6stc567 as StorageDescriptor<[], number, false>;
const stg_ChildBounties_ParentChildBounties: StorageDescriptor<[Key: number], number, false> =
  c2snmki6ntodt as StorageDescriptor<[Key: number], number, false>;
const stg_ChildBounties_ChildBounties: StorageDescriptor<
  [number, number],
  {
    parent_bounty: number;
    value: bigint;
    fee: bigint;
    curator_deposit: bigint;
    status: ChildBountyStatus;
  },
  true
> = "ehpnsumg33leb" as StorageDescriptor<
  [number, number],
  {
    parent_bounty: number;
    value: bigint;
    fee: bigint;
    curator_deposit: bigint;
    status: ChildBountyStatus;
  },
  true
>;
const stg_ChildBounties_ChildBountyDescriptions: StorageDescriptor<[Key: number], Binary, true> =
  cfmam6uono5akb as StorageDescriptor<[Key: number], Binary, true>;
const stg_ChildBounties_ChildrenCuratorFees: StorageDescriptor<[Key: number], bigint, false> =
  cau4oj9ee8cokk as StorageDescriptor<[Key: number], bigint, false>;
const stg_Nis_QueueTotals: StorageDescriptor<
  [],
  Array<Anonymize<I4ojmnsk1dchql>>,
  false
> = "f9jidduiuq7vv" as StorageDescriptor<[], Array<Anonymize<I4ojmnsk1dchql>>, false>;
const stg_Nis_Queues: StorageDescriptor<
  [Key: number],
  Array<Anonymize<Id5fm4p8lj5qgi>>,
  false
> = "9n44uc9spgev7" as StorageDescriptor<[Key: number], Array<Anonymize<Id5fm4p8lj5qgi>>, false>;
const stg_Nis_Summary: StorageDescriptor<
  [],
  {
    proportion_owed: bigint;
    index: number;
    thawed: bigint;
    last_period: number;
    receipts_on_hold: bigint;
  },
  false
> = "7btpfdeli5ea8" as StorageDescriptor<
  [],
  {
    proportion_owed: bigint;
    index: number;
    thawed: bigint;
    last_period: number;
    receipts_on_hold: bigint;
  },
  false
>;
const stg_Nis_Receipts: StorageDescriptor<
  [Key: number],
  { proportion: bigint; owner: Anonymize<I92hdo1clkbp4g>; expiry: number },
  true
> = "7hs7d51ua7f1e" as StorageDescriptor<
  [Key: number],
  { proportion: bigint; owner: Anonymize<I92hdo1clkbp4g>; expiry: number },
  true
>;
const stg_NisCounterpartBalances_TotalIssuance: StorageDescriptor<[], bigint, false> =
  c7d2tqf029vcod as StorageDescriptor<[], bigint, false>;
const stg_NisCounterpartBalances_InactiveIssuance: StorageDescriptor<[], bigint, false> =
  c7d2tqf029vcod as StorageDescriptor<[], bigint, false>;
const stg_NisCounterpartBalances_Account: StorageDescriptor<
  [Key: SS58String],
  { free: bigint; reserved: bigint; frozen: bigint; flags: bigint },
  false
> = cbtum7aot707m6 as StorageDescriptor<
  [Key: SS58String],
  { free: bigint; reserved: bigint; frozen: bigint; flags: bigint },
  false
>;
const stg_NisCounterpartBalances_Locks: StorageDescriptor<
  [Key: SS58String],
  Array<Anonymize<I5b29v4qfq4tu7>>,
  false
> = c1e2385hi92o2f as StorageDescriptor<[Key: SS58String], Array<Anonymize<I5b29v4qfq4tu7>>, false>;
const stg_NisCounterpartBalances_Reserves: StorageDescriptor<
  [Key: SS58String],
  Array<Anonymize<I32btm6htd9bck>>,
  false
> = cbe3psbb7upqsv as StorageDescriptor<[Key: SS58String], Array<Anonymize<I32btm6htd9bck>>, false>;
const stg_NisCounterpartBalances_Holds: StorageDescriptor<
  [Key: SS58String],
  Array<Anonymize<Iemrlqc7oc4k5g>>,
  false
> = cbf1alv83g5fj0 as StorageDescriptor<[Key: SS58String], Array<Anonymize<Iemrlqc7oc4k5g>>, false>;
const stg_NisCounterpartBalances_Freezes: StorageDescriptor<
  [Key: SS58String],
  Array<Anonymize<I7qdm60946h5u9>>,
  false
> = cbfv6n704ucltm as StorageDescriptor<[Key: SS58String], Array<Anonymize<I7qdm60946h5u9>>, false>;
const stg_Configuration_ActiveConfig: StorageDescriptor<
  [],
  {
    max_code_size: number;
    max_head_data_size: number;
    max_upward_queue_count: number;
    max_upward_queue_size: number;
    max_upward_message_size: number;
    max_upward_message_num_per_candidate: number;
    hrmp_max_message_num_per_candidate: number;
    validation_upgrade_cooldown: number;
    validation_upgrade_delay: number;
    async_backing_params: Anonymize<Iavuvfkop6318c>;
    max_pov_size: number;
    max_downward_message_size: number;
    hrmp_max_parachain_outbound_channels: number;
    hrmp_sender_deposit: bigint;
    hrmp_recipient_deposit: bigint;
    hrmp_channel_max_capacity: number;
    hrmp_channel_max_total_size: number;
    hrmp_max_parachain_inbound_channels: number;
    hrmp_channel_max_message_size: number;
    executor_params: Anonymize<I79n5nka727sg9>;
    code_retention_period: number;
    coretime_cores: number;
    on_demand_retries: number;
    on_demand_queue_max_size: number;
    on_demand_target_queue_utilization: number;
    on_demand_fee_variability: number;
    on_demand_base_fee: bigint;
    on_demand_ttl: number;
    group_rotation_frequency: number;
    paras_availability_period: number;
    scheduling_lookahead: number;
    max_validators_per_core: Anonymize<I4arjljr6dpflb>;
    max_validators: Anonymize<I4arjljr6dpflb>;
    dispute_period: number;
    dispute_post_conclusion_acceptance_period: number;
    no_show_slots: number;
    n_delay_tranches: number;
    zeroth_delay_tranche_width: number;
    needed_approvals: number;
    relay_vrf_modulo_samples: number;
    pvf_voting_ttl: number;
    minimum_validation_upgrade_delay: number;
    minimum_backing_votes: number;
    node_features: { bytes: Uint8Array; bitsLen: number };
    approval_voting_params: number;
  },
  false
> = "b8600fv9p32lu" as StorageDescriptor<
  [],
  {
    max_code_size: number;
    max_head_data_size: number;
    max_upward_queue_count: number;
    max_upward_queue_size: number;
    max_upward_message_size: number;
    max_upward_message_num_per_candidate: number;
    hrmp_max_message_num_per_candidate: number;
    validation_upgrade_cooldown: number;
    validation_upgrade_delay: number;
    async_backing_params: Anonymize<Iavuvfkop6318c>;
    max_pov_size: number;
    max_downward_message_size: number;
    hrmp_max_parachain_outbound_channels: number;
    hrmp_sender_deposit: bigint;
    hrmp_recipient_deposit: bigint;
    hrmp_channel_max_capacity: number;
    hrmp_channel_max_total_size: number;
    hrmp_max_parachain_inbound_channels: number;
    hrmp_channel_max_message_size: number;
    executor_params: Anonymize<I79n5nka727sg9>;
    code_retention_period: number;
    coretime_cores: number;
    on_demand_retries: number;
    on_demand_queue_max_size: number;
    on_demand_target_queue_utilization: number;
    on_demand_fee_variability: number;
    on_demand_base_fee: bigint;
    on_demand_ttl: number;
    group_rotation_frequency: number;
    paras_availability_period: number;
    scheduling_lookahead: number;
    max_validators_per_core: Anonymize<I4arjljr6dpflb>;
    max_validators: Anonymize<I4arjljr6dpflb>;
    dispute_period: number;
    dispute_post_conclusion_acceptance_period: number;
    no_show_slots: number;
    n_delay_tranches: number;
    zeroth_delay_tranche_width: number;
    needed_approvals: number;
    relay_vrf_modulo_samples: number;
    pvf_voting_ttl: number;
    minimum_validation_upgrade_delay: number;
    minimum_backing_votes: number;
    node_features: { bytes: Uint8Array; bitsLen: number };
    approval_voting_params: number;
  },
  false
>;
const stg_Configuration_PendingConfigs: StorageDescriptor<
  [],
  Array<Anonymize<If59lb4asqkhca>>,
  false
> = "2rsa7njg1k6v1" as StorageDescriptor<[], Array<Anonymize<If59lb4asqkhca>>, false>;
const stg_Configuration_BypassConsistencyCheck: StorageDescriptor<[], boolean, false> =
  cegurtje61sbu9 as StorageDescriptor<[], boolean, false>;
const stg_ParasShared_CurrentSessionIndex: StorageDescriptor<[], number, false> =
  cf3ncnq6stc567 as StorageDescriptor<[], number, false>;
const stg_ParasShared_ActiveValidatorIndices: StorageDescriptor<
  [],
  Array<number>,
  false
> = ccgljjb6j82uhn as StorageDescriptor<[], Array<number>, false>;
const stg_ParasShared_ActiveValidatorKeys: StorageDescriptor<
  [],
  Array<Binary>,
  false
> = cdhnf6rtqoslea as StorageDescriptor<[], Array<Binary>, false>;
const stg_ParasShared_AllowedRelayParents: StorageDescriptor<
  [],
  { buffer: Anonymize<Idm6djpsp7gtcm>; latest_number: number },
  false
> = "663ddds93n8hc" as StorageDescriptor<
  [],
  { buffer: Anonymize<Idm6djpsp7gtcm>; latest_number: number },
  false
>;
const stg_ParaInclusion_AvailabilityBitfields: StorageDescriptor<
  [Key: number],
  { bitfield: { bytes: Uint8Array; bitsLen: number }; submitted_at: number },
  true
> = "5pj0vo2r2ljrv" as StorageDescriptor<
  [Key: number],
  { bitfield: { bytes: Uint8Array; bitsLen: number }; submitted_at: number },
  true
>;
const stg_ParaInclusion_PendingAvailability: StorageDescriptor<
  [Key: number],
  {
    core: number;
    hash: Binary;
    descriptor: Anonymize<Ib2u20s6roco9i>;
    availability_votes: { bytes: Uint8Array; bitsLen: number };
    backers: { bytes: Uint8Array; bitsLen: number };
    relay_parent_number: number;
    backed_in_number: number;
    backing_group: number;
  },
  true
> = "fc80em05kj1cq" as StorageDescriptor<
  [Key: number],
  {
    core: number;
    hash: Binary;
    descriptor: Anonymize<Ib2u20s6roco9i>;
    availability_votes: { bytes: Uint8Array; bitsLen: number };
    backers: { bytes: Uint8Array; bitsLen: number };
    relay_parent_number: number;
    backed_in_number: number;
    backing_group: number;
  },
  true
>;
const stg_ParaInclusion_PendingAvailabilityCommitments: StorageDescriptor<
  [Key: number],
  {
    upward_messages: Anonymize<Itom7fk49o0c9>;
    horizontal_messages: Anonymize<I6r5cbv8ttrb09>;
    new_validation_code: Anonymize<Iabpgqcjikia83>;
    head_data: Binary;
    processed_downward_messages: number;
    hrmp_watermark: number;
  },
  true
> = "b23qnlqop1kue" as StorageDescriptor<
  [Key: number],
  {
    upward_messages: Anonymize<Itom7fk49o0c9>;
    horizontal_messages: Anonymize<I6r5cbv8ttrb09>;
    new_validation_code: Anonymize<Iabpgqcjikia83>;
    head_data: Binary;
    processed_downward_messages: number;
    hrmp_watermark: number;
  },
  true
>;
const stg_ParaInherent_Included: StorageDescriptor<[], undefined, true> =
  cau2fn2kb28v1i as StorageDescriptor<[], undefined, true>;
const stg_ParaInherent_OnChainVotes: StorageDescriptor<
  [],
  {
    session: number;
    backing_validators_per_candidate: Anonymize<Ibabtlc0psj69a>;
    disputes: Anonymize<Ib08u599nb52js>;
  },
  true
> = "e7c40q4va8u7r" as StorageDescriptor<
  [],
  {
    session: number;
    backing_validators_per_candidate: Anonymize<Ibabtlc0psj69a>;
    disputes: Anonymize<Ib08u599nb52js>;
  },
  true
>;
const stg_ParaScheduler_ValidatorGroups: StorageDescriptor<
  [],
  Array<Anonymize<Icgljjb6j82uhn>>,
  false
> = "arlj3qd8u1v13" as StorageDescriptor<[], Array<Anonymize<Icgljjb6j82uhn>>, false>;
const stg_ParaScheduler_AvailabilityCores: StorageDescriptor<
  [],
  Array<PolkadotRuntimeParachainsSchedulerPalletCoreOccupied>,
  false
> = "9gvbjb5fb4hhm" as StorageDescriptor<
  [],
  Array<PolkadotRuntimeParachainsSchedulerPalletCoreOccupied>,
  false
>;
const stg_ParaScheduler_SessionStartBlock: StorageDescriptor<[], number, false> =
  cf3ncnq6stc567 as StorageDescriptor<[], number, false>;
const stg_ParaScheduler_ClaimQueue: StorageDescriptor<
  [],
  Array<Anonymize<Iekknq5gev7jf7>>,
  false
> = "c136fqousb794" as StorageDescriptor<[], Array<Anonymize<Iekknq5gev7jf7>>, false>;
const stg_Paras_PvfActiveVoteMap: StorageDescriptor<
  [Key: Binary],
  {
    votes_accept: { bytes: Uint8Array; bitsLen: number };
    votes_reject: { bytes: Uint8Array; bitsLen: number };
    age: number;
    created_at: number;
    causes: Anonymize<Ia0a9586d8d811>;
  },
  true
> = "8vn9b4il050fq" as StorageDescriptor<
  [Key: Binary],
  {
    votes_accept: { bytes: Uint8Array; bitsLen: number };
    votes_reject: { bytes: Uint8Array; bitsLen: number };
    age: number;
    created_at: number;
    causes: Anonymize<Ia0a9586d8d811>;
  },
  true
>;
const stg_Paras_PvfActiveVoteList: StorageDescriptor<
  [],
  Array<Binary>,
  false
> = cdhnf6rtqoslea as StorageDescriptor<[], Array<Binary>, false>;
const stg_Paras_Parachains: StorageDescriptor<
  [],
  Array<number>,
  false
> = ccgljjb6j82uhn as StorageDescriptor<[], Array<number>, false>;
const stg_Paras_ParaLifecycles: StorageDescriptor<
  [Key: number],
  ParachainsParasParaLifecycle,
  true
> = "c7oj6ac9ipmaj" as StorageDescriptor<[Key: number], ParachainsParasParaLifecycle, true>;
const stg_Paras_Heads: StorageDescriptor<[Key: number], Binary, true> =
  cfmam6uono5akb as StorageDescriptor<[Key: number], Binary, true>;
const stg_Paras_MostRecentContext: StorageDescriptor<[Key: number], number, true> =
  c2snmki6ntodt as StorageDescriptor<[Key: number], number, true>;
const stg_Paras_CurrentCodeHash: StorageDescriptor<[Key: number], Binary, true> =
  cdhc577m5btfvc as StorageDescriptor<[Key: number], Binary, true>;
const stg_Paras_PastCodeHash: StorageDescriptor<[Key: [number, number]], Binary, true> =
  "7qshf3nk8rev8" as StorageDescriptor<[Key: [number, number]], Binary, true>;
const stg_Paras_PastCodeMeta: StorageDescriptor<
  [Key: number],
  { upgrade_times: Anonymize<I2v6n2k262gqsq>; last_pruned: Anonymize<I4arjljr6dpflb> },
  false
> = "e2bu7rd8isrpv" as StorageDescriptor<
  [Key: number],
  { upgrade_times: Anonymize<I2v6n2k262gqsq>; last_pruned: Anonymize<I4arjljr6dpflb> },
  false
>;
const stg_Paras_PastCodePruning: StorageDescriptor<
  [],
  Array<Anonymize<I5g2vv0ckl2m8b>>,
  false
> = cep4uo61810hfs as StorageDescriptor<[], Array<Anonymize<I5g2vv0ckl2m8b>>, false>;
const stg_Paras_FutureCodeUpgrades: StorageDescriptor<[Key: number], number, true> =
  c2snmki6ntodt as StorageDescriptor<[Key: number], number, true>;
const stg_Paras_FutureCodeHash: StorageDescriptor<[Key: number], Binary, true> =
  cdhc577m5btfvc as StorageDescriptor<[Key: number], Binary, true>;
const stg_Paras_UpgradeGoAheadSignal: StorageDescriptor<
  [Key: number],
  PolkadotPrimitivesV5UpgradeGoAhead,
  true
> = "8herb61smtp3l" as StorageDescriptor<[Key: number], PolkadotPrimitivesV5UpgradeGoAhead, true>;
const stg_Paras_UpgradeRestrictionSignal: StorageDescriptor<
  [Key: number],
  PolkadotPrimitivesV5UpgradeRestriction,
  true
> = "50ao57qq5bmnu" as StorageDescriptor<
  [Key: number],
  PolkadotPrimitivesV5UpgradeRestriction,
  true
>;
const stg_Paras_UpgradeCooldowns: StorageDescriptor<
  [],
  Array<Anonymize<I5g2vv0ckl2m8b>>,
  false
> = cep4uo61810hfs as StorageDescriptor<[], Array<Anonymize<I5g2vv0ckl2m8b>>, false>;
const stg_Paras_UpcomingUpgrades: StorageDescriptor<
  [],
  Array<Anonymize<I5g2vv0ckl2m8b>>,
  false
> = cep4uo61810hfs as StorageDescriptor<[], Array<Anonymize<I5g2vv0ckl2m8b>>, false>;
const stg_Paras_ActionsQueue: StorageDescriptor<
  [Key: number],
  Array<number>,
  false
> = c5e37esnd8etqt as StorageDescriptor<[Key: number], Array<number>, false>;
const stg_Paras_UpcomingParasGenesis: StorageDescriptor<
  [Key: number],
  { genesis_head: Binary; validation_code: Binary; para_kind: boolean },
  true
> = "ga3hcc415ve2" as StorageDescriptor<
  [Key: number],
  { genesis_head: Binary; validation_code: Binary; para_kind: boolean },
  true
>;
const stg_Paras_CodeByHashRefs: StorageDescriptor<[Key: Binary], number, false> =
  ccsbuok7olbg29 as StorageDescriptor<[Key: Binary], number, false>;
const stg_Paras_CodeByHash: StorageDescriptor<[Key: Binary], Binary, true> =
  "49bulag1bvk1v" as StorageDescriptor<[Key: Binary], Binary, true>;
const stg_Initializer_HasInitialized: StorageDescriptor<[], undefined, true> =
  cau2fn2kb28v1i as StorageDescriptor<[], undefined, true>;
const stg_Initializer_BufferedSessionChanges: StorageDescriptor<
  [],
  Array<Anonymize<I36mfku1ea0i8t>>,
  false
> = "14hoekog5s49k" as StorageDescriptor<[], Array<Anonymize<I36mfku1ea0i8t>>, false>;
const stg_Dmp_DownwardMessageQueues: StorageDescriptor<
  [Key: number],
  Array<Anonymize<I60847k37jfcc6>>,
  false
> = "9bipd38l817ra" as StorageDescriptor<[Key: number], Array<Anonymize<I60847k37jfcc6>>, false>;
const stg_Dmp_DownwardMessageQueueHeads: StorageDescriptor<[Key: number], Binary, false> =
  cdhc577m5btfvc as StorageDescriptor<[Key: number], Binary, false>;
const stg_Dmp_DeliveryFeeFactor: StorageDescriptor<[Key: number], bigint, false> =
  cau4oj9ee8cokk as StorageDescriptor<[Key: number], bigint, false>;
const stg_Hrmp_HrmpOpenChannelRequests: StorageDescriptor<
  [Key: { sender: number; recipient: number }],
  {
    confirmed: boolean;
    _age: number;
    sender_deposit: bigint;
    max_message_size: number;
    max_capacity: number;
    max_total_size: number;
  },
  true
> = "9a2ssh2u357ft" as StorageDescriptor<
  [Key: { sender: number; recipient: number }],
  {
    confirmed: boolean;
    _age: number;
    sender_deposit: bigint;
    max_message_size: number;
    max_capacity: number;
    max_total_size: number;
  },
  true
>;
const stg_Hrmp_HrmpOpenChannelRequestsList: StorageDescriptor<
  [],
  Array<Anonymize<I50mrcbubp554e>>,
  false
> = cd43g4eveajpkl as StorageDescriptor<[], Array<Anonymize<I50mrcbubp554e>>, false>;
const stg_Hrmp_HrmpOpenChannelRequestCount: StorageDescriptor<[Key: number], number, false> =
  c2snmki6ntodt as StorageDescriptor<[Key: number], number, false>;
const stg_Hrmp_HrmpAcceptedChannelRequestCount: StorageDescriptor<[Key: number], number, false> =
  c2snmki6ntodt as StorageDescriptor<[Key: number], number, false>;
const stg_Hrmp_HrmpCloseChannelRequests: StorageDescriptor<
  [Key: { sender: number; recipient: number }],
  undefined,
  true
> = "cbol1ehll269p" as StorageDescriptor<
  [Key: { sender: number; recipient: number }],
  undefined,
  true
>;
const stg_Hrmp_HrmpCloseChannelRequestsList: StorageDescriptor<
  [],
  Array<Anonymize<I50mrcbubp554e>>,
  false
> = cd43g4eveajpkl as StorageDescriptor<[], Array<Anonymize<I50mrcbubp554e>>, false>;
const stg_Hrmp_HrmpWatermarks: StorageDescriptor<[Key: number], number, true> =
  c2snmki6ntodt as StorageDescriptor<[Key: number], number, true>;
const stg_Hrmp_HrmpChannels: StorageDescriptor<
  [Key: { sender: number; recipient: number }],
  {
    max_capacity: number;
    max_total_size: number;
    max_message_size: number;
    msg_count: number;
    total_size: number;
    mqc_head: Anonymize<I17k3ujudqd5df>;
    sender_deposit: bigint;
    recipient_deposit: bigint;
  },
  true
> = "ei0nlfv9nobu1" as StorageDescriptor<
  [Key: { sender: number; recipient: number }],
  {
    max_capacity: number;
    max_total_size: number;
    max_message_size: number;
    msg_count: number;
    total_size: number;
    mqc_head: Anonymize<I17k3ujudqd5df>;
    sender_deposit: bigint;
    recipient_deposit: bigint;
  },
  true
>;
const stg_Hrmp_HrmpIngressChannelsIndex: StorageDescriptor<
  [Key: number],
  Array<number>,
  false
> = c5e37esnd8etqt as StorageDescriptor<[Key: number], Array<number>, false>;
const stg_Hrmp_HrmpEgressChannelsIndex: StorageDescriptor<
  [Key: number],
  Array<number>,
  false
> = c5e37esnd8etqt as StorageDescriptor<[Key: number], Array<number>, false>;
const stg_Hrmp_HrmpChannelContents: StorageDescriptor<
  [Key: { sender: number; recipient: number }],
  Array<Anonymize<I409qo0sfkbh16>>,
  false
> = "8e3fv1tdp89pc" as StorageDescriptor<
  [Key: { sender: number; recipient: number }],
  Array<Anonymize<I409qo0sfkbh16>>,
  false
>;
const stg_Hrmp_HrmpChannelDigests: StorageDescriptor<
  [Key: number],
  Array<Anonymize<I8pg2rpr4ldgp9>>,
  false
> = "70nlu29lgdbpl" as StorageDescriptor<[Key: number], Array<Anonymize<I8pg2rpr4ldgp9>>, false>;
const stg_ParaSessionInfo_AssignmentKeysUnsafe: StorageDescriptor<
  [],
  Array<Binary>,
  false
> = cdhnf6rtqoslea as StorageDescriptor<[], Array<Binary>, false>;
const stg_ParaSessionInfo_EarliestStoredSession: StorageDescriptor<[], number, false> =
  cf3ncnq6stc567 as StorageDescriptor<[], number, false>;
const stg_ParaSessionInfo_Sessions: StorageDescriptor<
  [Key: number],
  {
    active_validator_indices: Anonymize<Icgljjb6j82uhn>;
    random_seed: Binary;
    dispute_period: number;
    validators: Anonymize<Idhnf6rtqoslea>;
    discovery_keys: Anonymize<Idhnf6rtqoslea>;
    assignment_keys: Anonymize<Idhnf6rtqoslea>;
    validator_groups: Anonymize<Iarlj3qd8u1v13>;
    n_cores: number;
    zeroth_delay_tranche_width: number;
    relay_vrf_modulo_samples: number;
    n_delay_tranches: number;
    no_show_slots: number;
    needed_approvals: number;
  },
  true
> = "8o83l0ja8v2oc" as StorageDescriptor<
  [Key: number],
  {
    active_validator_indices: Anonymize<Icgljjb6j82uhn>;
    random_seed: Binary;
    dispute_period: number;
    validators: Anonymize<Idhnf6rtqoslea>;
    discovery_keys: Anonymize<Idhnf6rtqoslea>;
    assignment_keys: Anonymize<Idhnf6rtqoslea>;
    validator_groups: Anonymize<Iarlj3qd8u1v13>;
    n_cores: number;
    zeroth_delay_tranche_width: number;
    relay_vrf_modulo_samples: number;
    n_delay_tranches: number;
    no_show_slots: number;
    needed_approvals: number;
  },
  true
>;
const stg_ParaSessionInfo_AccountKeys: StorageDescriptor<
  [Key: number],
  Array<SS58String>,
  true
> = "f71t9q3462n6o" as StorageDescriptor<[Key: number], Array<SS58String>, true>;
const stg_ParaSessionInfo_SessionExecutorParams: StorageDescriptor<
  [Key: number],
  Array<PolkadotPrimitivesV6ExecutorParamsExecutorParam>,
  true
> = "6s4mpdophsc5o" as StorageDescriptor<
  [Key: number],
  Array<PolkadotPrimitivesV6ExecutorParamsExecutorParam>,
  true
>;
const stg_ParasDisputes_LastPrunedSession: StorageDescriptor<[], number, true> =
  cf3ncnq6stc567 as StorageDescriptor<[], number, true>;
const stg_ParasDisputes_Disputes: StorageDescriptor<
  [number, Binary],
  {
    validators_for: { bytes: Uint8Array; bitsLen: number };
    validators_against: { bytes: Uint8Array; bitsLen: number };
    start: number;
    concluded_at: Anonymize<I4arjljr6dpflb>;
  },
  true
> = "eh9amj3lnllpd" as StorageDescriptor<
  [number, Binary],
  {
    validators_for: { bytes: Uint8Array; bitsLen: number };
    validators_against: { bytes: Uint8Array; bitsLen: number };
    start: number;
    concluded_at: Anonymize<I4arjljr6dpflb>;
  },
  true
>;
const stg_ParasDisputes_BackersOnDisputes: StorageDescriptor<
  [number, Binary],
  Array<number>,
  true
> = "b3dj3c9mhg0i9" as StorageDescriptor<[number, Binary], Array<number>, true>;
const stg_ParasDisputes_Included: StorageDescriptor<[number, Binary], number, true> =
  "d30vab5tnappv" as StorageDescriptor<[number, Binary], number, true>;
const stg_ParasDisputes_Frozen: StorageDescriptor<[], number | undefined, false> =
  c4arjljr6dpflb as StorageDescriptor<[], number | undefined, false>;
const stg_ParasSlashing_UnappliedSlashes: StorageDescriptor<
  [number, Binary],
  { keys: Anonymize<I93ssha9egqq23>; kind: PolkadotPrimitivesV5SlashingSlashingOffenceKind },
  true
> = "c9o14aqplk52r" as StorageDescriptor<
  [number, Binary],
  { keys: Anonymize<I93ssha9egqq23>; kind: PolkadotPrimitivesV5SlashingSlashingOffenceKind },
  true
>;
const stg_ParasSlashing_ValidatorSetCounts: StorageDescriptor<[Key: number], number, true> =
  c2snmki6ntodt as StorageDescriptor<[Key: number], number, true>;
const stg_MessageQueue_BookStateFor: StorageDescriptor<
  [Key: ParachainsInclusionAggregateMessageOrigin],
  {
    begin: number;
    end: number;
    count: number;
    ready_neighbours: Anonymize<Iav3cdf9g9n9fp>;
    message_count: bigint;
    size: bigint;
  },
  false
> = "34el3ro8dldbd" as StorageDescriptor<
  [Key: ParachainsInclusionAggregateMessageOrigin],
  {
    begin: number;
    end: number;
    count: number;
    ready_neighbours: Anonymize<Iav3cdf9g9n9fp>;
    message_count: bigint;
    size: bigint;
  },
  false
>;
const stg_MessageQueue_ServiceHead: StorageDescriptor<
  [],
  ParachainsInclusionAggregateMessageOrigin,
  true
> = "6liql5oh190bt" as StorageDescriptor<[], ParachainsInclusionAggregateMessageOrigin, true>;
const stg_MessageQueue_Pages: StorageDescriptor<
  [ParachainsInclusionAggregateMessageOrigin, number],
  {
    remaining: number;
    remaining_size: number;
    first_index: number;
    first: number;
    last: number;
    heap: Binary;
  },
  true
> = "2dosj31r13pfm" as StorageDescriptor<
  [ParachainsInclusionAggregateMessageOrigin, number],
  {
    remaining: number;
    remaining_size: number;
    first_index: number;
    first: number;
    last: number;
    heap: Binary;
  },
  true
>;
const stg_OnDemandAssignmentProvider_SpotTraffic: StorageDescriptor<[], bigint, false> =
  c7d2tqf029vcod as StorageDescriptor<[], bigint, false>;
const stg_OnDemandAssignmentProvider_OnDemandQueue: StorageDescriptor<
  [],
  Array<number>,
  false
> = ccgljjb6j82uhn as StorageDescriptor<[], Array<number>, false>;
const stg_OnDemandAssignmentProvider_ParaIdAffinity: StorageDescriptor<
  [Key: number],
  { core_idx: number; count: number },
  true
> = "ckbkan6diuu14" as StorageDescriptor<[Key: number], { core_idx: number; count: number }, true>;
const stg_CoretimeAssignmentProvider_CoreSchedules: StorageDescriptor<
  [Key: [number, number]],
  {
    assignments: Anonymize<Id1dkuivtmc10j>;
    end_hint: Anonymize<I4arjljr6dpflb>;
    next_schedule: Anonymize<I4arjljr6dpflb>;
  },
  true
> = "6tql60t79tq82" as StorageDescriptor<
  [Key: [number, number]],
  {
    assignments: Anonymize<Id1dkuivtmc10j>;
    end_hint: Anonymize<I4arjljr6dpflb>;
    next_schedule: Anonymize<I4arjljr6dpflb>;
  },
  true
>;
const stg_CoretimeAssignmentProvider_CoreDescriptors: StorageDescriptor<
  [Key: number],
  { queue: Anonymize<If4sm31aq7v0i8>; current_work: Anonymize<I154iog8qmouro> },
  false
> = "f2oigse5fj7os" as StorageDescriptor<
  [Key: number],
  { queue: Anonymize<If4sm31aq7v0i8>; current_work: Anonymize<I154iog8qmouro> },
  false
>;
const stg_Registrar_PendingSwap: StorageDescriptor<[Key: number], number, true> =
  c2snmki6ntodt as StorageDescriptor<[Key: number], number, true>;
const stg_Registrar_Paras: StorageDescriptor<
  [Key: number],
  { manager: SS58String; deposit: bigint; locked: Anonymize<I8ie0dco0kcuq5> },
  true
> = "bb6vpfpdsttrn" as StorageDescriptor<
  [Key: number],
  { manager: SS58String; deposit: bigint; locked: Anonymize<I8ie0dco0kcuq5> },
  true
>;
const stg_Registrar_NextFreeParaId: StorageDescriptor<[], number, false> =
  cf3ncnq6stc567 as StorageDescriptor<[], number, false>;
const stg_Slots_Leases: StorageDescriptor<
  [Key: number],
  Array<Anonymize<I92hdo1clkbp4g>>,
  false
> = "3jn2ljvqgpmg4" as StorageDescriptor<[Key: number], Array<Anonymize<I92hdo1clkbp4g>>, false>;
const stg_Auctions_AuctionCounter: StorageDescriptor<[], number, false> =
  cf3ncnq6stc567 as StorageDescriptor<[], number, false>;
const stg_Auctions_AuctionInfo: StorageDescriptor<[], [number, number], true> =
  c5g2vv0ckl2m8b as StorageDescriptor<[], [number, number], true>;
const stg_Auctions_ReservedAmounts: StorageDescriptor<[Key: [SS58String, number]], bigint, true> =
  "cckspamfm7bph" as StorageDescriptor<[Key: [SS58String, number]], bigint, true>;
const stg_Auctions_Winning: StorageDescriptor<
  [Key: number],
  Array<Anonymize<I1qlf98109qt29>>,
  true
> = "631551otptb43" as StorageDescriptor<[Key: number], Array<Anonymize<I1qlf98109qt29>>, true>;
const stg_Crowdloan_Funds: StorageDescriptor<
  [Key: number],
  {
    depositor: SS58String;
    verifier: Anonymize<I8t18p6mokc3s4>;
    deposit: bigint;
    raised: bigint;
    end: number;
    cap: bigint;
    last_contribution: CommonCrowdloanLastContribution;
    first_period: number;
    last_period: number;
    fund_index: number;
  },
  true
> = "72idmjf73mhcr" as StorageDescriptor<
  [Key: number],
  {
    depositor: SS58String;
    verifier: Anonymize<I8t18p6mokc3s4>;
    deposit: bigint;
    raised: bigint;
    end: number;
    cap: bigint;
    last_contribution: CommonCrowdloanLastContribution;
    first_period: number;
    last_period: number;
    fund_index: number;
  },
  true
>;
const stg_Crowdloan_NewRaise: StorageDescriptor<
  [],
  Array<number>,
  false
> = ccgljjb6j82uhn as StorageDescriptor<[], Array<number>, false>;
const stg_Crowdloan_EndingsCount: StorageDescriptor<[], number, false> =
  cf3ncnq6stc567 as StorageDescriptor<[], number, false>;
const stg_Crowdloan_NextFundIndex: StorageDescriptor<[], number, false> =
  cf3ncnq6stc567 as StorageDescriptor<[], number, false>;
const stg_XcmPallet_QueryCounter: StorageDescriptor<[], bigint, false> =
  c7d2tqf029vcod as StorageDescriptor<[], bigint, false>;
const stg_XcmPallet_Queries: StorageDescriptor<[Key: bigint], XcmPalletQueryStatus, true> =
  "a07bm8fugo20n" as StorageDescriptor<[Key: bigint], XcmPalletQueryStatus, true>;
const stg_XcmPallet_AssetTraps: StorageDescriptor<[Key: Binary], number, false> =
  ccsbuok7olbg29 as StorageDescriptor<[Key: Binary], number, false>;
const stg_XcmPallet_SafeXcmVersion: StorageDescriptor<[], number, true> =
  cf3ncnq6stc567 as StorageDescriptor<[], number, true>;
const stg_XcmPallet_SupportedVersion: StorageDescriptor<
  [number, XcmVersionedLocation],
  number,
  true
> = "8qfjkhifu8a7p" as StorageDescriptor<[number, XcmVersionedLocation], number, true>;
const stg_XcmPallet_VersionNotifiers: StorageDescriptor<
  [number, XcmVersionedLocation],
  bigint,
  true
> = "6bu4evhvoel3" as StorageDescriptor<[number, XcmVersionedLocation], bigint, true>;
const stg_XcmPallet_VersionNotifyTargets: StorageDescriptor<
  [number, XcmVersionedLocation],
  [bigint, Anonymize<I4q39t5hn830vp>, number],
  true
> = "3il9uuiloil79" as StorageDescriptor<
  [number, XcmVersionedLocation],
  [bigint, Anonymize<I4q39t5hn830vp>, number],
  true
>;
const stg_XcmPallet_VersionDiscoveryQueue: StorageDescriptor<
  [],
  Array<Anonymize<Iak31q8r4mqtse>>,
  false
> = "6g82vlc185ggp" as StorageDescriptor<[], Array<Anonymize<Iak31q8r4mqtse>>, false>;
const stg_XcmPallet_CurrentMigration: StorageDescriptor<[], XcmPalletVersionMigrationStage, true> =
  "1s1utqquuaa2b" as StorageDescriptor<[], XcmPalletVersionMigrationStage, true>;
const stg_XcmPallet_RemoteLockedFungibles: StorageDescriptor<
  [number, SS58String, XcmVersionedAssetId],
  {
    amount: bigint;
    owner: XcmVersionedLocation;
    locker: XcmVersionedLocation;
    consumers: Anonymize<I48jka0f0ufl6q>;
  },
  true
> = "b468l8c8kbjr2" as StorageDescriptor<
  [number, SS58String, XcmVersionedAssetId],
  {
    amount: bigint;
    owner: XcmVersionedLocation;
    locker: XcmVersionedLocation;
    consumers: Anonymize<I48jka0f0ufl6q>;
  },
  true
>;
const stg_XcmPallet_LockedFungibles: StorageDescriptor<
  [Key: SS58String],
  Array<Anonymize<I5pu2dsc6d3iv6>>,
  true
> = "1phb478brkiu7" as StorageDescriptor<[Key: SS58String], Array<Anonymize<I5pu2dsc6d3iv6>>, true>;
const stg_XcmPallet_XcmExecutionSuspended: StorageDescriptor<[], boolean, false> =
  cegurtje61sbu9 as StorageDescriptor<[], boolean, false>;
const stg_Beefy_Authorities: StorageDescriptor<
  [],
  Array<Binary>,
  false
> = ce5mvl0hn85mkc as StorageDescriptor<[], Array<Binary>, false>;
const stg_Beefy_ValidatorSetId: StorageDescriptor<[], bigint, false> =
  c7d2tqf029vcod as StorageDescriptor<[], bigint, false>;
const stg_Beefy_NextAuthorities: StorageDescriptor<
  [],
  Array<Binary>,
  false
> = ce5mvl0hn85mkc as StorageDescriptor<[], Array<Binary>, false>;
const stg_Beefy_SetIdSession: StorageDescriptor<[Key: bigint], number, true> =
  c3u1nuj99f15hf as StorageDescriptor<[Key: bigint], number, true>;
const stg_Beefy_GenesisBlock: StorageDescriptor<[], number | undefined, false> =
  c4arjljr6dpflb as StorageDescriptor<[], number | undefined, false>;
const stg_Mmr_RootHash: StorageDescriptor<[], Binary, false> = c7i24fvhamijd as StorageDescriptor<
  [],
  Binary,
  false
>;
const stg_Mmr_NumberOfLeaves: StorageDescriptor<[], bigint, false> =
  c7d2tqf029vcod as StorageDescriptor<[], bigint, false>;
const stg_Mmr_Nodes: StorageDescriptor<[Key: bigint], Binary, true> =
  "d7pvv0lg5qu1i" as StorageDescriptor<[Key: bigint], Binary, true>;
const stg_MmrLeaf_BeefyAuthorities: StorageDescriptor<
  [],
  { id: bigint; len: number; keyset_commitment: Binary },
  false
> = c3qm5u05tioq05 as StorageDescriptor<
  [],
  { id: bigint; len: number; keyset_commitment: Binary },
  false
>;
const stg_MmrLeaf_BeefyNextAuthorities: StorageDescriptor<
  [],
  { id: bigint; len: number; keyset_commitment: Binary },
  false
> = c3qm5u05tioq05 as StorageDescriptor<
  [],
  { id: bigint; len: number; keyset_commitment: Binary },
  false
>;
const stg_AssignedSlots_PermanentSlots: StorageDescriptor<[Key: number], [number, number], true> =
  "dmjfd364lbpn6" as StorageDescriptor<[Key: number], [number, number], true>;
const stg_AssignedSlots_PermanentSlotCount: StorageDescriptor<[], number, false> =
  cf3ncnq6stc567 as StorageDescriptor<[], number, false>;
const stg_AssignedSlots_TemporarySlots: StorageDescriptor<
  [Key: number],
  {
    manager: SS58String;
    period_begin: number;
    period_count: number;
    last_lease: Anonymize<I4arjljr6dpflb>;
    lease_count: number;
  },
  true
> = "c03ung7kn5baa" as StorageDescriptor<
  [Key: number],
  {
    manager: SS58String;
    period_begin: number;
    period_count: number;
    last_lease: Anonymize<I4arjljr6dpflb>;
    lease_count: number;
  },
  true
>;
const stg_AssignedSlots_TemporarySlotCount: StorageDescriptor<[], number, false> =
  cf3ncnq6stc567 as StorageDescriptor<[], number, false>;
const stg_AssignedSlots_ActiveTemporarySlotCount: StorageDescriptor<[], number, false> =
  cf3ncnq6stc567 as StorageDescriptor<[], number, false>;
const stg_AssignedSlots_MaxTemporarySlots: StorageDescriptor<[], number, false> =
  cf3ncnq6stc567 as StorageDescriptor<[], number, false>;
const stg_AssignedSlots_MaxPermanentSlots: StorageDescriptor<[], number, false> =
  cf3ncnq6stc567 as StorageDescriptor<[], number, false>;
const stg_ValidatorManager_ValidatorsToRetire: StorageDescriptor<
  [],
  Array<SS58String>,
  false
> = ca2lhg7l2hilo3 as StorageDescriptor<[], Array<SS58String>, false>;
const stg_ValidatorManager_ValidatorsToAdd: StorageDescriptor<
  [],
  Array<SS58String>,
  false
> = ca2lhg7l2hilo3 as StorageDescriptor<[], Array<SS58String>, false>;
const stg_StateTrieMigration_MigrationProcess: StorageDescriptor<
  [],
  {
    progress_top: Anonymize<I2jvfaenrpg1ai>;
    progress_child: Anonymize<I2jvfaenrpg1ai>;
    size: number;
    top_items: number;
    child_items: number;
  },
  false
> = "2sqgqj7ib3ine" as StorageDescriptor<
  [],
  {
    progress_top: Anonymize<I2jvfaenrpg1ai>;
    progress_child: Anonymize<I2jvfaenrpg1ai>;
    size: number;
    top_items: number;
    child_items: number;
  },
  false
>;
const stg_StateTrieMigration_AutoLimits: StorageDescriptor<
  [],
  Anonymize<I215mkl885p4da> | undefined,
  false
> = "b17t3992hb64n" as StorageDescriptor<[], Anonymize<I215mkl885p4da> | undefined, false>;
const stg_StateTrieMigration_SignedMigrationMaxLimits: StorageDescriptor<
  [],
  { size: number; item: number },
  true
> = "215mkl885p4da" as StorageDescriptor<[], { size: number; item: number }, true>;
const stg_Sudo_Key: StorageDescriptor<[], SS58String, true> = c86esvcpejimq9 as StorageDescriptor<
  [],
  SS58String,
  true
>;
const call_System_remark: TxDescriptor<{ remark: Binary }> = c8ofcg5rbj0g2c as TxDescriptor<{
  remark: Binary;
}>;
const call_System_set_heap_pages: TxDescriptor<{ pages: bigint }> =
  "4adgbll7gku4i" as TxDescriptor<{ pages: bigint }>;
const call_System_set_code: TxDescriptor<{ code: Binary }> = c6pjjpfvhvcfru as TxDescriptor<{
  code: Binary;
}>;
const call_System_set_code_without_checks: TxDescriptor<{ code: Binary }> =
  c6pjjpfvhvcfru as TxDescriptor<{ code: Binary }>;
const call_System_set_storage: TxDescriptor<{ items: Anonymize<I5g1ftt6bt65bl> }> =
  "8qrhskdehbu57" as TxDescriptor<{ items: Anonymize<I5g1ftt6bt65bl> }>;
const call_System_kill_storage: TxDescriptor<{ keys: Anonymize<Itom7fk49o0c9> }> =
  "39uah9nss64h9" as TxDescriptor<{ keys: Anonymize<Itom7fk49o0c9> }>;
const call_System_kill_prefix: TxDescriptor<{ prefix: Binary; subkeys: number }> =
  "k64dknsq7k08" as TxDescriptor<{ prefix: Binary; subkeys: number }>;
const call_System_remark_with_event: TxDescriptor<{ remark: Binary }> =
  c8ofcg5rbj0g2c as TxDescriptor<{ remark: Binary }>;
const call_System_authorize_upgrade: TxDescriptor<{ code_hash: Binary }> =
  c6a5n5ij3gomuo as TxDescriptor<{ code_hash: Binary }>;
const call_System_authorize_upgrade_without_checks: TxDescriptor<{ code_hash: Binary }> =
  c6a5n5ij3gomuo as TxDescriptor<{ code_hash: Binary }>;
const call_System_apply_authorized_upgrade: TxDescriptor<{ code: Binary }> =
  c6pjjpfvhvcfru as TxDescriptor<{ code: Binary }>;
const call_Babe_report_equivocation: TxDescriptor<{
  equivocation_proof: Anonymize<I7bek4s9acs8nl>;
  key_owner_proof: Anonymize<I3ia7aufsoj0l1>;
}> = c7mmbgd20nut80 as TxDescriptor<{
  equivocation_proof: Anonymize<I7bek4s9acs8nl>;
  key_owner_proof: Anonymize<I3ia7aufsoj0l1>;
}>;
const call_Babe_report_equivocation_unsigned: TxDescriptor<{
  equivocation_proof: Anonymize<I7bek4s9acs8nl>;
  key_owner_proof: Anonymize<I3ia7aufsoj0l1>;
}> = c7mmbgd20nut80 as TxDescriptor<{
  equivocation_proof: Anonymize<I7bek4s9acs8nl>;
  key_owner_proof: Anonymize<I3ia7aufsoj0l1>;
}>;
const call_Babe_plan_config_change: TxDescriptor<{ config: BabeDigestsNextConfigDescriptor }> =
  "2dcpbss9027dl" as TxDescriptor<{ config: BabeDigestsNextConfigDescriptor }>;
const call_Timestamp_set: TxDescriptor<{ now: bigint }> = "dcr6u6361oad9" as TxDescriptor<{
  now: bigint;
}>;
const call_Indices_claim: TxDescriptor<{ index: number }> = c666bl2fqjkejo as TxDescriptor<{
  index: number;
}>;
const call_Indices_transfer: TxDescriptor<{ new: MultiAddress; index: number }> =
  cdge7gk9m5car0 as TxDescriptor<{ new: MultiAddress; index: number }>;
const call_Indices_free: TxDescriptor<{ index: number }> = c666bl2fqjkejo as TxDescriptor<{
  index: number;
}>;
const call_Indices_force_transfer: TxDescriptor<{
  new: MultiAddress;
  index: number;
  freeze: boolean;
}> = "34pbimt2i69v7" as TxDescriptor<{ new: MultiAddress; index: number; freeze: boolean }>;
const call_Indices_freeze: TxDescriptor<{ index: number }> = c666bl2fqjkejo as TxDescriptor<{
  index: number;
}>;
const call_Balances_transfer_allow_death: TxDescriptor<{ dest: MultiAddress; value: bigint }> =
  cen6q0lasi0m7i as TxDescriptor<{ dest: MultiAddress; value: bigint }>;
const call_Balances_force_transfer: TxDescriptor<{
  source: MultiAddress;
  dest: MultiAddress;
  value: bigint;
}> = ccacgruoo9j3r2 as TxDescriptor<{ source: MultiAddress; dest: MultiAddress; value: bigint }>;
const call_Balances_transfer_keep_alive: TxDescriptor<{ dest: MultiAddress; value: bigint }> =
  cen6q0lasi0m7i as TxDescriptor<{ dest: MultiAddress; value: bigint }>;
const call_Balances_transfer_all: TxDescriptor<{ dest: MultiAddress; keep_alive: boolean }> =
  c7dgmo7im9hljo as TxDescriptor<{ dest: MultiAddress; keep_alive: boolean }>;
const call_Balances_force_unreserve: TxDescriptor<{ who: MultiAddress; amount: bigint }> =
  cargojp1sv9icj as TxDescriptor<{ who: MultiAddress; amount: bigint }>;
const call_Balances_upgrade_accounts: TxDescriptor<{ who: Anonymize<Ia2lhg7l2hilo3> }> =
  cbmr18suc9ikh9 as TxDescriptor<{ who: Anonymize<Ia2lhg7l2hilo3> }>;
const call_Balances_force_set_balance: TxDescriptor<{ who: MultiAddress; new_free: bigint }> =
  ce0io91hk7pejj as TxDescriptor<{ who: MultiAddress; new_free: bigint }>;
const call_Balances_force_adjust_total_issuance: TxDescriptor<{
  direction: BalancesTypesAdjustmentDirection;
  delta: bigint;
}> = c3u7ci02rtsn3o as TxDescriptor<{ direction: BalancesTypesAdjustmentDirection; delta: bigint }>;
const call_Session_set_keys: TxDescriptor<{ keys: Anonymize<Ieup5s9ka3brpi>; proof: Binary }> =
  "dl8sn1uiguh1s" as TxDescriptor<{ keys: Anonymize<Ieup5s9ka3brpi>; proof: Binary }>;
const call_Session_purge_keys: TxDescriptor<undefined> = c39ibapn5dm6tr as TxDescriptor<undefined>;
const call_Grandpa_report_equivocation: TxDescriptor<{
  equivocation_proof: Anonymize<I95fr9lj1pb8v8>;
  key_owner_proof: Anonymize<I3ia7aufsoj0l1>;
}> = c4kjek1q6rj24q as TxDescriptor<{
  equivocation_proof: Anonymize<I95fr9lj1pb8v8>;
  key_owner_proof: Anonymize<I3ia7aufsoj0l1>;
}>;
const call_Grandpa_report_equivocation_unsigned: TxDescriptor<{
  equivocation_proof: Anonymize<I95fr9lj1pb8v8>;
  key_owner_proof: Anonymize<I3ia7aufsoj0l1>;
}> = c4kjek1q6rj24q as TxDescriptor<{
  equivocation_proof: Anonymize<I95fr9lj1pb8v8>;
  key_owner_proof: Anonymize<I3ia7aufsoj0l1>;
}>;
const call_Grandpa_note_stalled: TxDescriptor<{
  delay: number;
  best_finalized_block_number: number;
}> = "2hviml3snvhhn" as TxDescriptor<{ delay: number; best_finalized_block_number: number }>;
const call_Treasury_propose_spend: TxDescriptor<{ value: bigint; beneficiary: MultiAddress }> =
  "5c883qnnqciv8" as TxDescriptor<{ value: bigint; beneficiary: MultiAddress }>;
const call_Treasury_reject_proposal: TxDescriptor<{ proposal_id: number }> =
  ccm9m0qeemu66d as TxDescriptor<{ proposal_id: number }>;
const call_Treasury_approve_proposal: TxDescriptor<{ proposal_id: number }> =
  ccm9m0qeemu66d as TxDescriptor<{ proposal_id: number }>;
const call_Treasury_spend_local: TxDescriptor<{ amount: bigint; beneficiary: MultiAddress }> =
  "dpn74s0i9cdvp" as TxDescriptor<{ amount: bigint; beneficiary: MultiAddress }>;
const call_Treasury_remove_approval: TxDescriptor<{ proposal_id: number }> =
  ccm9m0qeemu66d as TxDescriptor<{ proposal_id: number }>;
const call_Treasury_spend: TxDescriptor<{
  asset_kind: PolkadotRuntimeCommonImplsVersionedLocatableAsset;
  amount: bigint;
  beneficiary: XcmVersionedLocation;
  valid_from: Anonymize<I4arjljr6dpflb>;
}> = "7vlce2m9b88vv" as TxDescriptor<{
  asset_kind: PolkadotRuntimeCommonImplsVersionedLocatableAsset;
  amount: bigint;
  beneficiary: XcmVersionedLocation;
  valid_from: Anonymize<I4arjljr6dpflb>;
}>;
const call_Treasury_payout: TxDescriptor<{ index: number }> = c666bl2fqjkejo as TxDescriptor<{
  index: number;
}>;
const call_Treasury_check_status: TxDescriptor<{ index: number }> = c666bl2fqjkejo as TxDescriptor<{
  index: number;
}>;
const call_Treasury_void_spend: TxDescriptor<{ index: number }> = c666bl2fqjkejo as TxDescriptor<{
  index: number;
}>;
const call_ConvictionVoting_vote: TxDescriptor<{
  poll_index: number;
  vote: ConvictionVotingVoteAccountVote;
}> = "dnsr2pndm36h0" as TxDescriptor<{ poll_index: number; vote: ConvictionVotingVoteAccountVote }>;
const call_ConvictionVoting_delegate: TxDescriptor<{
  class: number;
  to: MultiAddress;
  conviction: VotingConviction;
  balance: bigint;
}> = "d7ut33dljf52c" as TxDescriptor<{
  class: number;
  to: MultiAddress;
  conviction: VotingConviction;
  balance: bigint;
}>;
const call_ConvictionVoting_undelegate: TxDescriptor<{ class: number }> =
  "8steo882k7qns" as TxDescriptor<{ class: number }>;
const call_ConvictionVoting_unlock: TxDescriptor<{ class: number; target: MultiAddress }> =
  "1vc8h4t228bot" as TxDescriptor<{ class: number; target: MultiAddress }>;
const call_ConvictionVoting_remove_vote: TxDescriptor<{
  class: Anonymize<I4arjljr6dpflb>;
  index: number;
}> = "5f178ab6b89t3" as TxDescriptor<{ class: Anonymize<I4arjljr6dpflb>; index: number }>;
const call_ConvictionVoting_remove_other_vote: TxDescriptor<{
  target: MultiAddress;
  class: number;
  index: number;
}> = "5fak1u82ohqtm" as TxDescriptor<{ target: MultiAddress; class: number; index: number }>;
const call_Referenda_submit: TxDescriptor<{
  proposal_origin: WestendRuntimeOriginCaller;
  proposal: PreimagesBounded;
  enactment_moment: TraitsScheduleDispatchTime;
}> = c8lsplvtf2fhfc as TxDescriptor<{
  proposal_origin: WestendRuntimeOriginCaller;
  proposal: PreimagesBounded;
  enactment_moment: TraitsScheduleDispatchTime;
}>;
const call_Referenda_place_decision_deposit: TxDescriptor<{ index: number }> =
  c666bl2fqjkejo as TxDescriptor<{ index: number }>;
const call_Referenda_refund_decision_deposit: TxDescriptor<{ index: number }> =
  c666bl2fqjkejo as TxDescriptor<{ index: number }>;
const call_Referenda_cancel: TxDescriptor<{ index: number }> = c666bl2fqjkejo as TxDescriptor<{
  index: number;
}>;
const call_Referenda_kill: TxDescriptor<{ index: number }> = c666bl2fqjkejo as TxDescriptor<{
  index: number;
}>;
const call_Referenda_nudge_referendum: TxDescriptor<{ index: number }> =
  c666bl2fqjkejo as TxDescriptor<{ index: number }>;
const call_Referenda_one_fewer_deciding: TxDescriptor<{ track: number }> =
  ccbio0e1f0034b as TxDescriptor<{ track: number }>;
const call_Referenda_refund_submission_deposit: TxDescriptor<{ index: number }> =
  c666bl2fqjkejo as TxDescriptor<{ index: number }>;
const call_Referenda_set_metadata: TxDescriptor<{
  index: number;
  maybe_hash: Anonymize<I17k3ujudqd5df>;
}> = cfml0k0sf0mu2g as TxDescriptor<{ index: number; maybe_hash: Anonymize<I17k3ujudqd5df> }>;
const call_FellowshipCollective_add_member: TxDescriptor<{ who: MultiAddress }> =
  c2nmm2fprtp4kl as TxDescriptor<{ who: MultiAddress }>;
const call_FellowshipCollective_promote_member: TxDescriptor<{ who: MultiAddress }> =
  c2nmm2fprtp4kl as TxDescriptor<{ who: MultiAddress }>;
const call_FellowshipCollective_demote_member: TxDescriptor<{ who: MultiAddress }> =
  c2nmm2fprtp4kl as TxDescriptor<{ who: MultiAddress }>;
const call_FellowshipCollective_remove_member: TxDescriptor<{
  who: MultiAddress;
  min_rank: number;
}> = "8c23ih8fpmjma" as TxDescriptor<{ who: MultiAddress; min_rank: number }>;
const call_FellowshipCollective_vote: TxDescriptor<{ poll: number; aye: boolean }> =
  "8bvk21lpmah75" as TxDescriptor<{ poll: number; aye: boolean }>;
const call_FellowshipCollective_cleanup_poll: TxDescriptor<{ poll_index: number; max: number }> =
  "449n3riv6jbum" as TxDescriptor<{ poll_index: number; max: number }>;
const call_FellowshipCollective_exchange_member: TxDescriptor<{
  who: MultiAddress;
  new_who: MultiAddress;
}> = "6nasb41qm5s5m" as TxDescriptor<{ who: MultiAddress; new_who: MultiAddress }>;
const call_FellowshipReferenda_submit: TxDescriptor<{
  proposal_origin: WestendRuntimeOriginCaller;
  proposal: PreimagesBounded;
  enactment_moment: TraitsScheduleDispatchTime;
}> = c8lsplvtf2fhfc as TxDescriptor<{
  proposal_origin: WestendRuntimeOriginCaller;
  proposal: PreimagesBounded;
  enactment_moment: TraitsScheduleDispatchTime;
}>;
const call_FellowshipReferenda_place_decision_deposit: TxDescriptor<{ index: number }> =
  c666bl2fqjkejo as TxDescriptor<{ index: number }>;
const call_FellowshipReferenda_refund_decision_deposit: TxDescriptor<{ index: number }> =
  c666bl2fqjkejo as TxDescriptor<{ index: number }>;
const call_FellowshipReferenda_cancel: TxDescriptor<{ index: number }> =
  c666bl2fqjkejo as TxDescriptor<{ index: number }>;
const call_FellowshipReferenda_kill: TxDescriptor<{ index: number }> =
  c666bl2fqjkejo as TxDescriptor<{ index: number }>;
const call_FellowshipReferenda_nudge_referendum: TxDescriptor<{ index: number }> =
  c666bl2fqjkejo as TxDescriptor<{ index: number }>;
const call_FellowshipReferenda_one_fewer_deciding: TxDescriptor<{ track: number }> =
  ccbio0e1f0034b as TxDescriptor<{ track: number }>;
const call_FellowshipReferenda_refund_submission_deposit: TxDescriptor<{ index: number }> =
  c666bl2fqjkejo as TxDescriptor<{ index: number }>;
const call_FellowshipReferenda_set_metadata: TxDescriptor<{
  index: number;
  maybe_hash: Anonymize<I17k3ujudqd5df>;
}> = cfml0k0sf0mu2g as TxDescriptor<{ index: number; maybe_hash: Anonymize<I17k3ujudqd5df> }>;
const call_Whitelist_whitelist_call: TxDescriptor<{ call_hash: Binary }> =
  c8413rb6im3iko as TxDescriptor<{ call_hash: Binary }>;
const call_Whitelist_remove_whitelisted_call: TxDescriptor<{ call_hash: Binary }> =
  c8413rb6im3iko as TxDescriptor<{ call_hash: Binary }>;
const call_Whitelist_dispatch_whitelisted_call: TxDescriptor<{
  call_hash: Binary;
  call_encoded_len: number;
  call_weight_witness: Anonymize<I4q39t5hn830vp>;
}> = "d3s9pakjjc472" as TxDescriptor<{
  call_hash: Binary;
  call_encoded_len: number;
  call_weight_witness: Anonymize<I4q39t5hn830vp>;
}>;
const call_Whitelist_dispatch_whitelisted_call_with_preimage: TxDescriptor<{
  call: Anonymize<I5v7s5dm78b0mh>;
}> = cb6n6j9tk6ut3b as TxDescriptor<{ call: Anonymize<I5v7s5dm78b0mh> }>;
const call_Claims_claim: TxDescriptor<{ dest: SS58String; ethereum_signature: Binary }> =
  "1u3s4gbjnre15" as TxDescriptor<{ dest: SS58String; ethereum_signature: Binary }>;
const call_Claims_mint_claim: TxDescriptor<{
  who: Binary;
  value: bigint;
  vesting_schedule: Anonymize<I70kqehrkegc98>;
  statement: Anonymize<I6rcr4im2g3gv9>;
}> = "20qiajmn4c5d4" as TxDescriptor<{
  who: Binary;
  value: bigint;
  vesting_schedule: Anonymize<I70kqehrkegc98>;
  statement: Anonymize<I6rcr4im2g3gv9>;
}>;
const call_Claims_claim_attest: TxDescriptor<{
  dest: SS58String;
  ethereum_signature: Binary;
  statement: Binary;
}> = "e3aplba76d794" as TxDescriptor<{
  dest: SS58String;
  ethereum_signature: Binary;
  statement: Binary;
}>;
const call_Claims_attest: TxDescriptor<{ statement: Binary }> = "1ntko0oih7v1a" as TxDescriptor<{
  statement: Binary;
}>;
const call_Claims_move_claim: TxDescriptor<{
  old: Binary;
  new: Binary;
  maybe_preclaim: Anonymize<Ihfphjolmsqq1>;
}> = "193pigt6gtjff" as TxDescriptor<{
  old: Binary;
  new: Binary;
  maybe_preclaim: Anonymize<Ihfphjolmsqq1>;
}>;
const call_Utility_batch: TxDescriptor<{ calls: Anonymize<I52mcb5hepi70g> }> =
  c1u2b1ro792h6p as TxDescriptor<{ calls: Anonymize<I52mcb5hepi70g> }>;
const call_Utility_as_derivative: TxDescriptor<{ index: number; call: Anonymize<I5v7s5dm78b0mh> }> =
  "cc61753r53l80" as TxDescriptor<{ index: number; call: Anonymize<I5v7s5dm78b0mh> }>;
const call_Utility_batch_all: TxDescriptor<{ calls: Anonymize<I52mcb5hepi70g> }> =
  c1u2b1ro792h6p as TxDescriptor<{ calls: Anonymize<I52mcb5hepi70g> }>;
const call_Utility_dispatch_as: TxDescriptor<{
  as_origin: WestendRuntimeOriginCaller;
  call: Anonymize<I5v7s5dm78b0mh>;
}> = "fe2vmvsk5r0gl" as TxDescriptor<{
  as_origin: WestendRuntimeOriginCaller;
  call: Anonymize<I5v7s5dm78b0mh>;
}>;
const call_Utility_force_batch: TxDescriptor<{ calls: Anonymize<I52mcb5hepi70g> }> =
  c1u2b1ro792h6p as TxDescriptor<{ calls: Anonymize<I52mcb5hepi70g> }>;
const call_Utility_with_weight: TxDescriptor<{
  call: Anonymize<I5v7s5dm78b0mh>;
  weight: Anonymize<I4q39t5hn830vp>;
}> = ce1uvn9e6ki8dd as TxDescriptor<{
  call: Anonymize<I5v7s5dm78b0mh>;
  weight: Anonymize<I4q39t5hn830vp>;
}>;
const call_Identity_add_registrar: TxDescriptor<{ account: MultiAddress }> =
  cbsu2pfvipmui6 as TxDescriptor<{ account: MultiAddress }>;
const call_Identity_set_identity: TxDescriptor<{ info: Anonymize<I939m6d6t3dsgm> }> =
  "621gpns74tp1f" as TxDescriptor<{ info: Anonymize<I939m6d6t3dsgm> }>;
const call_Identity_set_subs: TxDescriptor<{ subs: Anonymize<I47e5e4dh41s5v> }> =
  "5100vdjbepcoj" as TxDescriptor<{ subs: Anonymize<I47e5e4dh41s5v> }>;
const call_Identity_clear_identity: TxDescriptor<undefined> =
  c39ibapn5dm6tr as TxDescriptor<undefined>;
const call_Identity_request_judgement: TxDescriptor<{ reg_index: number; max_fee: bigint }> =
  "9l2s4klu0831o" as TxDescriptor<{ reg_index: number; max_fee: bigint }>;
const call_Identity_cancel_request: TxDescriptor<{ reg_index: number }> =
  "2ctrt5nqb8o7c" as TxDescriptor<{ reg_index: number }>;
const call_Identity_set_fee: TxDescriptor<{ index: number; fee: bigint }> =
  "711qahikocb1c" as TxDescriptor<{ index: number; fee: bigint }>;
const call_Identity_set_account_id: TxDescriptor<{ index: number; new: MultiAddress }> =
  cdge7gk9m5car0 as TxDescriptor<{ index: number; new: MultiAddress }>;
const call_Identity_set_fields: TxDescriptor<{ index: number; fields: bigint }> =
  "d6gojh30v9ib2" as TxDescriptor<{ index: number; fields: bigint }>;
const call_Identity_provide_judgement: TxDescriptor<{
  reg_index: number;
  target: MultiAddress;
  judgement: IdentityJudgement;
  identity: Binary;
}> = "2g5s5rvm0mfuf" as TxDescriptor<{
  reg_index: number;
  target: MultiAddress;
  judgement: IdentityJudgement;
  identity: Binary;
}>;
const call_Identity_kill_identity: TxDescriptor<{ target: MultiAddress }> =
  c29er5j74l8bu as TxDescriptor<{ target: MultiAddress }>;
const call_Identity_add_sub: TxDescriptor<{ sub: MultiAddress; data: IdentityTypesData }> =
  cclf5v4qsadc12 as TxDescriptor<{ sub: MultiAddress; data: IdentityTypesData }>;
const call_Identity_rename_sub: TxDescriptor<{ sub: MultiAddress; data: IdentityTypesData }> =
  cclf5v4qsadc12 as TxDescriptor<{ sub: MultiAddress; data: IdentityTypesData }>;
const call_Identity_remove_sub: TxDescriptor<{ sub: MultiAddress }> =
  "fcc5t6ed1elfd" as TxDescriptor<{ sub: MultiAddress }>;
const call_Identity_quit_sub: TxDescriptor<undefined> = c39ibapn5dm6tr as TxDescriptor<undefined>;
const call_Identity_add_username_authority: TxDescriptor<{
  authority: MultiAddress;
  suffix: Binary;
  allocation: number;
}> = "bb4j9hgamcrbc" as TxDescriptor<{
  authority: MultiAddress;
  suffix: Binary;
  allocation: number;
}>;
const call_Identity_remove_username_authority: TxDescriptor<{ authority: MultiAddress }> =
  "8b1tfbsa65tdb" as TxDescriptor<{ authority: MultiAddress }>;
const call_Identity_set_username_for: TxDescriptor<{
  who: MultiAddress;
  username: Binary;
  signature: Anonymize<I7us28h09nc5sv>;
}> = "419rt0n0pbfjf" as TxDescriptor<{
  who: MultiAddress;
  username: Binary;
  signature: Anonymize<I7us28h09nc5sv>;
}>;
const call_Identity_accept_username: TxDescriptor<{ username: Binary }> =
  ce5l999tf7t2te as TxDescriptor<{ username: Binary }>;
const call_Identity_remove_expired_approval: TxDescriptor<{ username: Binary }> =
  ce5l999tf7t2te as TxDescriptor<{ username: Binary }>;
const call_Identity_set_primary_username: TxDescriptor<{ username: Binary }> =
  ce5l999tf7t2te as TxDescriptor<{ username: Binary }>;
const call_Identity_remove_dangling_username: TxDescriptor<{ username: Binary }> =
  ce5l999tf7t2te as TxDescriptor<{ username: Binary }>;
const call_Society_bid: TxDescriptor<{ value: bigint }> = ce5v6njpckr05b as TxDescriptor<{
  value: bigint;
}>;
const call_Society_unbid: TxDescriptor<undefined> = c39ibapn5dm6tr as TxDescriptor<undefined>;
const call_Society_vouch: TxDescriptor<{ who: MultiAddress; value: bigint; tip: bigint }> =
  "7tjn67p2i76ft" as TxDescriptor<{ who: MultiAddress; value: bigint; tip: bigint }>;
const call_Society_unvouch: TxDescriptor<undefined> = c39ibapn5dm6tr as TxDescriptor<undefined>;
const call_Society_vote: TxDescriptor<{ candidate: MultiAddress; approve: boolean }> =
  "8263q56dd34vq" as TxDescriptor<{ candidate: MultiAddress; approve: boolean }>;
const call_Society_defender_vote: TxDescriptor<{ approve: boolean }> =
  "26anifrfnljoi" as TxDescriptor<{ approve: boolean }>;
const call_Society_payout: TxDescriptor<undefined> = c39ibapn5dm6tr as TxDescriptor<undefined>;
const call_Society_waive_repay: TxDescriptor<{ amount: bigint }> = c3qt1hgg4djhgb as TxDescriptor<{
  amount: bigint;
}>;
const call_Society_found_society: TxDescriptor<{
  founder: MultiAddress;
  max_members: number;
  max_intake: number;
  max_strikes: number;
  candidate_deposit: bigint;
  rules: Binary;
}> = "g0li41oi4nn6" as TxDescriptor<{
  founder: MultiAddress;
  max_members: number;
  max_intake: number;
  max_strikes: number;
  candidate_deposit: bigint;
  rules: Binary;
}>;
const call_Society_dissolve: TxDescriptor<undefined> = c39ibapn5dm6tr as TxDescriptor<undefined>;
const call_Society_judge_suspended_member: TxDescriptor<{ who: MultiAddress; forgive: boolean }> =
  "6jkhgb7bmmdqp" as TxDescriptor<{ who: MultiAddress; forgive: boolean }>;
const call_Society_set_parameters: TxDescriptor<{
  max_members: number;
  max_intake: number;
  max_strikes: number;
  candidate_deposit: bigint;
}> = c9hkvi4k3kh916 as TxDescriptor<{
  max_members: number;
  max_intake: number;
  max_strikes: number;
  candidate_deposit: bigint;
}>;
const call_Society_punish_skeptic: TxDescriptor<undefined> =
  c39ibapn5dm6tr as TxDescriptor<undefined>;
const call_Society_claim_membership: TxDescriptor<undefined> =
  c39ibapn5dm6tr as TxDescriptor<undefined>;
const call_Society_bestow_membership: TxDescriptor<{ candidate: SS58String }> =
  c4b66js88p45m8 as TxDescriptor<{ candidate: SS58String }>;
const call_Society_kick_candidate: TxDescriptor<{ candidate: SS58String }> =
  c4b66js88p45m8 as TxDescriptor<{ candidate: SS58String }>;
const call_Society_resign_candidacy: TxDescriptor<undefined> =
  c39ibapn5dm6tr as TxDescriptor<undefined>;
const call_Society_drop_candidate: TxDescriptor<{ candidate: SS58String }> =
  c4b66js88p45m8 as TxDescriptor<{ candidate: SS58String }>;
const call_Society_cleanup_candidacy: TxDescriptor<{ candidate: SS58String; max: number }> =
  "eh40op1lafir9" as TxDescriptor<{ candidate: SS58String; max: number }>;
const call_Society_cleanup_challenge: TxDescriptor<{ challenge_round: number; max: number }> =
  "cjp7f9u9fno72" as TxDescriptor<{ challenge_round: number; max: number }>;
const call_Recovery_as_recovered: TxDescriptor<{
  account: MultiAddress;
  call: Anonymize<I5v7s5dm78b0mh>;
}> = "bkasvcbhle02a" as TxDescriptor<{ account: MultiAddress; call: Anonymize<I5v7s5dm78b0mh> }>;
const call_Recovery_set_recovered: TxDescriptor<{ lost: MultiAddress; rescuer: MultiAddress }> =
  cfddpcgiumovp6 as TxDescriptor<{ lost: MultiAddress; rescuer: MultiAddress }>;
const call_Recovery_create_recovery: TxDescriptor<{
  friends: Anonymize<Ia2lhg7l2hilo3>;
  threshold: number;
  delay_period: number;
}> = "6s6ihmfj6j5qq" as TxDescriptor<{
  friends: Anonymize<Ia2lhg7l2hilo3>;
  threshold: number;
  delay_period: number;
}>;
const call_Recovery_initiate_recovery: TxDescriptor<{ account: MultiAddress }> =
  cbsu2pfvipmui6 as TxDescriptor<{ account: MultiAddress }>;
const call_Recovery_vouch_recovery: TxDescriptor<{ lost: MultiAddress; rescuer: MultiAddress }> =
  cfddpcgiumovp6 as TxDescriptor<{ lost: MultiAddress; rescuer: MultiAddress }>;
const call_Recovery_claim_recovery: TxDescriptor<{ account: MultiAddress }> =
  cbsu2pfvipmui6 as TxDescriptor<{ account: MultiAddress }>;
const call_Recovery_close_recovery: TxDescriptor<{ rescuer: MultiAddress }> =
  "8p14f9e765cn8" as TxDescriptor<{ rescuer: MultiAddress }>;
const call_Recovery_remove_recovery: TxDescriptor<undefined> =
  c39ibapn5dm6tr as TxDescriptor<undefined>;
const call_Recovery_cancel_recovered: TxDescriptor<{ account: MultiAddress }> =
  cbsu2pfvipmui6 as TxDescriptor<{ account: MultiAddress }>;
const call_Vesting_vest: TxDescriptor<undefined> = c39ibapn5dm6tr as TxDescriptor<undefined>;
const call_Vesting_vest_other: TxDescriptor<{ target: MultiAddress }> =
  c29er5j74l8bu as TxDescriptor<{ target: MultiAddress }>;
const call_Vesting_vested_transfer: TxDescriptor<{
  target: MultiAddress;
  schedule: Anonymize<I4aro1m78pdrtt>;
}> = "9l9kkok4o3ekh" as TxDescriptor<{ target: MultiAddress; schedule: Anonymize<I4aro1m78pdrtt> }>;
const call_Vesting_force_vested_transfer: TxDescriptor<{
  source: MultiAddress;
  target: MultiAddress;
  schedule: Anonymize<I4aro1m78pdrtt>;
}> = "50ve0bbda0j1r" as TxDescriptor<{
  source: MultiAddress;
  target: MultiAddress;
  schedule: Anonymize<I4aro1m78pdrtt>;
}>;
const call_Vesting_merge_schedules: TxDescriptor<{
  schedule1_index: number;
  schedule2_index: number;
}> = "ct9ivhr2c5hv0" as TxDescriptor<{ schedule1_index: number; schedule2_index: number }>;
const call_Vesting_force_remove_vesting_schedule: TxDescriptor<{
  target: MultiAddress;
  schedule_index: number;
}> = "eihemrjk2piff" as TxDescriptor<{ target: MultiAddress; schedule_index: number }>;
const call_Scheduler_schedule: TxDescriptor<{
  when: number;
  maybe_periodic: Anonymize<I34gtdjipdmjpt>;
  priority: number;
  call: Anonymize<I5v7s5dm78b0mh>;
}> = "ihs4sg3g01bu" as TxDescriptor<{
  when: number;
  maybe_periodic: Anonymize<I34gtdjipdmjpt>;
  priority: number;
  call: Anonymize<I5v7s5dm78b0mh>;
}>;
const call_Scheduler_cancel: TxDescriptor<{ when: number; index: number }> =
  c5n4sebgkfr760 as TxDescriptor<{ when: number; index: number }>;
const call_Scheduler_schedule_named: TxDescriptor<{
  id: Binary;
  when: number;
  maybe_periodic: Anonymize<I34gtdjipdmjpt>;
  priority: number;
  call: Anonymize<I5v7s5dm78b0mh>;
}> = "72fgprrpc6paj" as TxDescriptor<{
  id: Binary;
  when: number;
  maybe_periodic: Anonymize<I34gtdjipdmjpt>;
  priority: number;
  call: Anonymize<I5v7s5dm78b0mh>;
}>;
const call_Scheduler_cancel_named: TxDescriptor<{ id: Binary }> = cdsdstalforb09 as TxDescriptor<{
  id: Binary;
}>;
const call_Scheduler_schedule_after: TxDescriptor<{
  after: number;
  maybe_periodic: Anonymize<I34gtdjipdmjpt>;
  priority: number;
  call: Anonymize<I5v7s5dm78b0mh>;
}> = "2erle9fi4utli" as TxDescriptor<{
  after: number;
  maybe_periodic: Anonymize<I34gtdjipdmjpt>;
  priority: number;
  call: Anonymize<I5v7s5dm78b0mh>;
}>;
const call_Scheduler_schedule_named_after: TxDescriptor<{
  id: Binary;
  after: number;
  maybe_periodic: Anonymize<I34gtdjipdmjpt>;
  priority: number;
  call: Anonymize<I5v7s5dm78b0mh>;
}> = "9u9iqnbk7726p" as TxDescriptor<{
  id: Binary;
  after: number;
  maybe_periodic: Anonymize<I34gtdjipdmjpt>;
  priority: number;
  call: Anonymize<I5v7s5dm78b0mh>;
}>;
const call_Scheduler_set_retry: TxDescriptor<{
  task: Anonymize<I5g2vv0ckl2m8b>;
  retries: number;
  period: number;
}> = "4f96q1hbj65r4" as TxDescriptor<{
  task: Anonymize<I5g2vv0ckl2m8b>;
  retries: number;
  period: number;
}>;
const call_Scheduler_set_retry_named: TxDescriptor<{
  id: Binary;
  retries: number;
  period: number;
}> = "c9l30ktlnnckb" as TxDescriptor<{ id: Binary; retries: number; period: number }>;
const call_Scheduler_cancel_retry: TxDescriptor<{ task: Anonymize<I5g2vv0ckl2m8b> }> =
  "7dqo5l1v1ae92" as TxDescriptor<{ task: Anonymize<I5g2vv0ckl2m8b> }>;
const call_Scheduler_cancel_retry_named: TxDescriptor<{ id: Binary }> =
  cdsdstalforb09 as TxDescriptor<{ id: Binary }>;
const call_Proxy_proxy: TxDescriptor<{
  real: MultiAddress;
  force_proxy_type: Anonymize<I6adql68ljr6ip>;
  call: Anonymize<I5v7s5dm78b0mh>;
}> = "5a5u61se9rl1f" as TxDescriptor<{
  real: MultiAddress;
  force_proxy_type: Anonymize<I6adql68ljr6ip>;
  call: Anonymize<I5v7s5dm78b0mh>;
}>;
const call_Proxy_add_proxy: TxDescriptor<{
  delegate: MultiAddress;
  proxy_type: Anonymize<I90b4nim1kup1v>;
  delay: number;
}> = c7mmcnc01jgbim as TxDescriptor<{
  delegate: MultiAddress;
  proxy_type: Anonymize<I90b4nim1kup1v>;
  delay: number;
}>;
const call_Proxy_remove_proxy: TxDescriptor<{
  delegate: MultiAddress;
  proxy_type: Anonymize<I90b4nim1kup1v>;
  delay: number;
}> = c7mmcnc01jgbim as TxDescriptor<{
  delegate: MultiAddress;
  proxy_type: Anonymize<I90b4nim1kup1v>;
  delay: number;
}>;
const call_Proxy_remove_proxies: TxDescriptor<undefined> =
  c39ibapn5dm6tr as TxDescriptor<undefined>;
const call_Proxy_create_pure: TxDescriptor<{
  proxy_type: Anonymize<I90b4nim1kup1v>;
  delay: number;
  index: number;
}> = "porp0oj935c" as TxDescriptor<{
  proxy_type: Anonymize<I90b4nim1kup1v>;
  delay: number;
  index: number;
}>;
const call_Proxy_kill_pure: TxDescriptor<{
  spawner: MultiAddress;
  proxy_type: Anonymize<I90b4nim1kup1v>;
  index: number;
  height: number;
  ext_index: number;
}> = "f2c1ulh9jgo8t" as TxDescriptor<{
  spawner: MultiAddress;
  proxy_type: Anonymize<I90b4nim1kup1v>;
  index: number;
  height: number;
  ext_index: number;
}>;
const call_Proxy_announce: TxDescriptor<{ real: MultiAddress; call_hash: Binary }> =
  cd3bpmvju2iqi5 as TxDescriptor<{ real: MultiAddress; call_hash: Binary }>;
const call_Proxy_remove_announcement: TxDescriptor<{ real: MultiAddress; call_hash: Binary }> =
  cd3bpmvju2iqi5 as TxDescriptor<{ real: MultiAddress; call_hash: Binary }>;
const call_Proxy_reject_announcement: TxDescriptor<{ delegate: MultiAddress; call_hash: Binary }> =
  "etdab69eu3c4e" as TxDescriptor<{ delegate: MultiAddress; call_hash: Binary }>;
const call_Proxy_proxy_announced: TxDescriptor<{
  delegate: MultiAddress;
  real: MultiAddress;
  force_proxy_type: Anonymize<I6adql68ljr6ip>;
  call: Anonymize<I5v7s5dm78b0mh>;
}> = "ffuoq5s44a6st" as TxDescriptor<{
  delegate: MultiAddress;
  real: MultiAddress;
  force_proxy_type: Anonymize<I6adql68ljr6ip>;
  call: Anonymize<I5v7s5dm78b0mh>;
}>;
const call_Multisig_as_multi_threshold_1: TxDescriptor<{
  other_signatories: Anonymize<Ia2lhg7l2hilo3>;
  call: Anonymize<I5v7s5dm78b0mh>;
}> = "9o3rcvcaeatto" as TxDescriptor<{
  other_signatories: Anonymize<Ia2lhg7l2hilo3>;
  call: Anonymize<I5v7s5dm78b0mh>;
}>;
const call_Multisig_as_multi: TxDescriptor<{
  threshold: number;
  other_signatories: Anonymize<Ia2lhg7l2hilo3>;
  maybe_timepoint: Anonymize<I95jfd8j5cr5eh>;
  call: Anonymize<I5v7s5dm78b0mh>;
  max_weight: Anonymize<I4q39t5hn830vp>;
}> = "av5h5sjlg3ds7" as TxDescriptor<{
  threshold: number;
  other_signatories: Anonymize<Ia2lhg7l2hilo3>;
  maybe_timepoint: Anonymize<I95jfd8j5cr5eh>;
  call: Anonymize<I5v7s5dm78b0mh>;
  max_weight: Anonymize<I4q39t5hn830vp>;
}>;
const call_Multisig_approve_as_multi: TxDescriptor<{
  threshold: number;
  other_signatories: Anonymize<Ia2lhg7l2hilo3>;
  maybe_timepoint: Anonymize<I95jfd8j5cr5eh>;
  call_hash: Binary;
  max_weight: Anonymize<I4q39t5hn830vp>;
}> = "349bg0i7n8ohu" as TxDescriptor<{
  threshold: number;
  other_signatories: Anonymize<Ia2lhg7l2hilo3>;
  maybe_timepoint: Anonymize<I95jfd8j5cr5eh>;
  call_hash: Binary;
  max_weight: Anonymize<I4q39t5hn830vp>;
}>;
const call_Multisig_cancel_as_multi: TxDescriptor<{
  threshold: number;
  other_signatories: Anonymize<Ia2lhg7l2hilo3>;
  timepoint: Anonymize<Itvprrpb0nm3o>;
  call_hash: Binary;
}> = "8plicv234mqe5" as TxDescriptor<{
  threshold: number;
  other_signatories: Anonymize<Ia2lhg7l2hilo3>;
  timepoint: Anonymize<Itvprrpb0nm3o>;
  call_hash: Binary;
}>;
const call_Preimage_note_preimage: TxDescriptor<{ bytes: Binary }> =
  "82nfqfkd48n10" as TxDescriptor<{ bytes: Binary }>;
const call_Preimage_unnote_preimage: TxDescriptor<{ hash: Binary }> =
  cd9d48vaes3c53 as TxDescriptor<{ hash: Binary }>;
const call_Preimage_request_preimage: TxDescriptor<{ hash: Binary }> =
  cd9d48vaes3c53 as TxDescriptor<{ hash: Binary }>;
const call_Preimage_unrequest_preimage: TxDescriptor<{ hash: Binary }> =
  cd9d48vaes3c53 as TxDescriptor<{ hash: Binary }>;
const call_Preimage_ensure_updated: TxDescriptor<{ hashes: Anonymize<Idhnf6rtqoslea> }> =
  "daor7tajt0l3k" as TxDescriptor<{ hashes: Anonymize<Idhnf6rtqoslea> }>;
const call_AssetRate_create: TxDescriptor<{
  asset_kind: PolkadotRuntimeCommonImplsVersionedLocatableAsset;
  rate: bigint;
}> = c4digl36cac7si as TxDescriptor<{
  asset_kind: PolkadotRuntimeCommonImplsVersionedLocatableAsset;
  rate: bigint;
}>;
const call_AssetRate_update: TxDescriptor<{
  asset_kind: PolkadotRuntimeCommonImplsVersionedLocatableAsset;
  rate: bigint;
}> = c4digl36cac7si as TxDescriptor<{
  asset_kind: PolkadotRuntimeCommonImplsVersionedLocatableAsset;
  rate: bigint;
}>;
const call_AssetRate_remove: TxDescriptor<{
  asset_kind: PolkadotRuntimeCommonImplsVersionedLocatableAsset;
}> = cdi8jsg8hkprub as TxDescriptor<{
  asset_kind: PolkadotRuntimeCommonImplsVersionedLocatableAsset;
}>;
const call_Bounties_propose_bounty: TxDescriptor<{ value: bigint; description: Binary }> =
  "2a839vbf5817q" as TxDescriptor<{ value: bigint; description: Binary }>;
const call_Bounties_approve_bounty: TxDescriptor<{ bounty_id: number }> =
  ca9p5bg6p18r0i as TxDescriptor<{ bounty_id: number }>;
const call_Bounties_propose_curator: TxDescriptor<{
  bounty_id: number;
  curator: MultiAddress;
  fee: bigint;
}> = "86gbm3avnuhcj" as TxDescriptor<{ bounty_id: number; curator: MultiAddress; fee: bigint }>;
const call_Bounties_unassign_curator: TxDescriptor<{ bounty_id: number }> =
  ca9p5bg6p18r0i as TxDescriptor<{ bounty_id: number }>;
const call_Bounties_accept_curator: TxDescriptor<{ bounty_id: number }> =
  ca9p5bg6p18r0i as TxDescriptor<{ bounty_id: number }>;
const call_Bounties_award_bounty: TxDescriptor<{ bounty_id: number; beneficiary: MultiAddress }> =
  "9khudebied2et" as TxDescriptor<{ bounty_id: number; beneficiary: MultiAddress }>;
const call_Bounties_claim_bounty: TxDescriptor<{ bounty_id: number }> =
  ca9p5bg6p18r0i as TxDescriptor<{ bounty_id: number }>;
const call_Bounties_close_bounty: TxDescriptor<{ bounty_id: number }> =
  ca9p5bg6p18r0i as TxDescriptor<{ bounty_id: number }>;
const call_Bounties_extend_bounty_expiry: TxDescriptor<{ bounty_id: number; remark: Binary }> =
  "90n6nnkpdahrh" as TxDescriptor<{ bounty_id: number; remark: Binary }>;
const call_ChildBounties_add_child_bounty: TxDescriptor<{
  parent_bounty_id: number;
  value: bigint;
  description: Binary;
}> = "8mk5kjgn02hi8" as TxDescriptor<{
  parent_bounty_id: number;
  value: bigint;
  description: Binary;
}>;
const call_ChildBounties_propose_curator: TxDescriptor<{
  parent_bounty_id: number;
  child_bounty_id: number;
  curator: MultiAddress;
  fee: bigint;
}> = "113qogfj9ii7a" as TxDescriptor<{
  parent_bounty_id: number;
  child_bounty_id: number;
  curator: MultiAddress;
  fee: bigint;
}>;
const call_ChildBounties_accept_curator: TxDescriptor<{
  parent_bounty_id: number;
  child_bounty_id: number;
}> = c2gr10p66od9ch as TxDescriptor<{ parent_bounty_id: number; child_bounty_id: number }>;
const call_ChildBounties_unassign_curator: TxDescriptor<{
  parent_bounty_id: number;
  child_bounty_id: number;
}> = c2gr10p66od9ch as TxDescriptor<{ parent_bounty_id: number; child_bounty_id: number }>;
const call_ChildBounties_award_child_bounty: TxDescriptor<{
  parent_bounty_id: number;
  child_bounty_id: number;
  beneficiary: MultiAddress;
}> = "6okbrc1o6b331" as TxDescriptor<{
  parent_bounty_id: number;
  child_bounty_id: number;
  beneficiary: MultiAddress;
}>;
const call_ChildBounties_claim_child_bounty: TxDescriptor<{
  parent_bounty_id: number;
  child_bounty_id: number;
}> = c2gr10p66od9ch as TxDescriptor<{ parent_bounty_id: number; child_bounty_id: number }>;
const call_ChildBounties_close_child_bounty: TxDescriptor<{
  parent_bounty_id: number;
  child_bounty_id: number;
}> = c2gr10p66od9ch as TxDescriptor<{ parent_bounty_id: number; child_bounty_id: number }>;
const call_Nis_place_bid: TxDescriptor<{ amount: bigint; duration: number }> =
  ccckjru8f8flpj as TxDescriptor<{ amount: bigint; duration: number }>;
const call_Nis_retract_bid: TxDescriptor<{ amount: bigint; duration: number }> =
  ccckjru8f8flpj as TxDescriptor<{ amount: bigint; duration: number }>;
const call_Nis_fund_deficit: TxDescriptor<undefined> = c39ibapn5dm6tr as TxDescriptor<undefined>;
const call_Nis_thaw_private: TxDescriptor<{
  index: number;
  maybe_proportion: Anonymize<I35p85j063s0il>;
}> = "ba38lgbjrs7je" as TxDescriptor<{
  index: number;
  maybe_proportion: Anonymize<I35p85j063s0il>;
}>;
const call_Nis_thaw_communal: TxDescriptor<{ index: number }> = c666bl2fqjkejo as TxDescriptor<{
  index: number;
}>;
const call_Nis_communify: TxDescriptor<{ index: number }> = c666bl2fqjkejo as TxDescriptor<{
  index: number;
}>;
const call_Nis_privatize: TxDescriptor<{ index: number }> = c666bl2fqjkejo as TxDescriptor<{
  index: number;
}>;
const call_NisCounterpartBalances_transfer_allow_death: TxDescriptor<{
  dest: MultiAddress;
  value: bigint;
}> = cen6q0lasi0m7i as TxDescriptor<{ dest: MultiAddress; value: bigint }>;
const call_NisCounterpartBalances_force_transfer: TxDescriptor<{
  source: MultiAddress;
  dest: MultiAddress;
  value: bigint;
}> = ccacgruoo9j3r2 as TxDescriptor<{ source: MultiAddress; dest: MultiAddress; value: bigint }>;
const call_NisCounterpartBalances_transfer_keep_alive: TxDescriptor<{
  dest: MultiAddress;
  value: bigint;
}> = cen6q0lasi0m7i as TxDescriptor<{ dest: MultiAddress; value: bigint }>;
const call_NisCounterpartBalances_transfer_all: TxDescriptor<{
  dest: MultiAddress;
  keep_alive: boolean;
}> = c7dgmo7im9hljo as TxDescriptor<{ dest: MultiAddress; keep_alive: boolean }>;
const call_NisCounterpartBalances_force_unreserve: TxDescriptor<{
  who: MultiAddress;
  amount: bigint;
}> = cargojp1sv9icj as TxDescriptor<{ who: MultiAddress; amount: bigint }>;
const call_NisCounterpartBalances_upgrade_accounts: TxDescriptor<{
  who: Anonymize<Ia2lhg7l2hilo3>;
}> = cbmr18suc9ikh9 as TxDescriptor<{ who: Anonymize<Ia2lhg7l2hilo3> }>;
const call_NisCounterpartBalances_force_set_balance: TxDescriptor<{
  who: MultiAddress;
  new_free: bigint;
}> = ce0io91hk7pejj as TxDescriptor<{ who: MultiAddress; new_free: bigint }>;
const call_NisCounterpartBalances_force_adjust_total_issuance: TxDescriptor<{
  direction: BalancesTypesAdjustmentDirection;
  delta: bigint;
}> = c3u7ci02rtsn3o as TxDescriptor<{ direction: BalancesTypesAdjustmentDirection; delta: bigint }>;
const call_Configuration_set_validation_upgrade_cooldown: TxDescriptor<{ new: number }> =
  c3vh014cqgmrfd as TxDescriptor<{ new: number }>;
const call_Configuration_set_validation_upgrade_delay: TxDescriptor<{ new: number }> =
  c3vh014cqgmrfd as TxDescriptor<{ new: number }>;
const call_Configuration_set_code_retention_period: TxDescriptor<{ new: number }> =
  c3vh014cqgmrfd as TxDescriptor<{ new: number }>;
const call_Configuration_set_max_code_size: TxDescriptor<{ new: number }> =
  c3vh014cqgmrfd as TxDescriptor<{ new: number }>;
const call_Configuration_set_max_pov_size: TxDescriptor<{ new: number }> =
  c3vh014cqgmrfd as TxDescriptor<{ new: number }>;
const call_Configuration_set_max_head_data_size: TxDescriptor<{ new: number }> =
  c3vh014cqgmrfd as TxDescriptor<{ new: number }>;
const call_Configuration_set_coretime_cores: TxDescriptor<{ new: number }> =
  c3vh014cqgmrfd as TxDescriptor<{ new: number }>;
const call_Configuration_set_on_demand_retries: TxDescriptor<{ new: number }> =
  c3vh014cqgmrfd as TxDescriptor<{ new: number }>;
const call_Configuration_set_group_rotation_frequency: TxDescriptor<{ new: number }> =
  c3vh014cqgmrfd as TxDescriptor<{ new: number }>;
const call_Configuration_set_paras_availability_period: TxDescriptor<{ new: number }> =
  c3vh014cqgmrfd as TxDescriptor<{ new: number }>;
const call_Configuration_set_scheduling_lookahead: TxDescriptor<{ new: number }> =
  c3vh014cqgmrfd as TxDescriptor<{ new: number }>;
const call_Configuration_set_max_validators_per_core: TxDescriptor<{
  new: Anonymize<I4arjljr6dpflb>;
}> = cd581arok0b1nj as TxDescriptor<{ new: Anonymize<I4arjljr6dpflb> }>;
const call_Configuration_set_max_validators: TxDescriptor<{ new: Anonymize<I4arjljr6dpflb> }> =
  cd581arok0b1nj as TxDescriptor<{ new: Anonymize<I4arjljr6dpflb> }>;
const call_Configuration_set_dispute_period: TxDescriptor<{ new: number }> =
  c3vh014cqgmrfd as TxDescriptor<{ new: number }>;
const call_Configuration_set_dispute_post_conclusion_acceptance_period: TxDescriptor<{
  new: number;
}> = c3vh014cqgmrfd as TxDescriptor<{ new: number }>;
const call_Configuration_set_no_show_slots: TxDescriptor<{ new: number }> =
  c3vh014cqgmrfd as TxDescriptor<{ new: number }>;
const call_Configuration_set_n_delay_tranches: TxDescriptor<{ new: number }> =
  c3vh014cqgmrfd as TxDescriptor<{ new: number }>;
const call_Configuration_set_zeroth_delay_tranche_width: TxDescriptor<{ new: number }> =
  c3vh014cqgmrfd as TxDescriptor<{ new: number }>;
const call_Configuration_set_needed_approvals: TxDescriptor<{ new: number }> =
  c3vh014cqgmrfd as TxDescriptor<{ new: number }>;
const call_Configuration_set_relay_vrf_modulo_samples: TxDescriptor<{ new: number }> =
  c3vh014cqgmrfd as TxDescriptor<{ new: number }>;
const call_Configuration_set_max_upward_queue_count: TxDescriptor<{ new: number }> =
  c3vh014cqgmrfd as TxDescriptor<{ new: number }>;
const call_Configuration_set_max_upward_queue_size: TxDescriptor<{ new: number }> =
  c3vh014cqgmrfd as TxDescriptor<{ new: number }>;
const call_Configuration_set_max_downward_message_size: TxDescriptor<{ new: number }> =
  c3vh014cqgmrfd as TxDescriptor<{ new: number }>;
const call_Configuration_set_max_upward_message_size: TxDescriptor<{ new: number }> =
  c3vh014cqgmrfd as TxDescriptor<{ new: number }>;
const call_Configuration_set_max_upward_message_num_per_candidate: TxDescriptor<{ new: number }> =
  c3vh014cqgmrfd as TxDescriptor<{ new: number }>;
const call_Configuration_set_hrmp_open_request_ttl: TxDescriptor<{ new: number }> =
  c3vh014cqgmrfd as TxDescriptor<{ new: number }>;
const call_Configuration_set_hrmp_sender_deposit: TxDescriptor<{ new: bigint }> =
  c9jsikd1ghmc7l as TxDescriptor<{ new: bigint }>;
const call_Configuration_set_hrmp_recipient_deposit: TxDescriptor<{ new: bigint }> =
  c9jsikd1ghmc7l as TxDescriptor<{ new: bigint }>;
const call_Configuration_set_hrmp_channel_max_capacity: TxDescriptor<{ new: number }> =
  c3vh014cqgmrfd as TxDescriptor<{ new: number }>;
const call_Configuration_set_hrmp_channel_max_total_size: TxDescriptor<{ new: number }> =
  c3vh014cqgmrfd as TxDescriptor<{ new: number }>;
const call_Configuration_set_hrmp_max_parachain_inbound_channels: TxDescriptor<{ new: number }> =
  c3vh014cqgmrfd as TxDescriptor<{ new: number }>;
const call_Configuration_set_hrmp_channel_max_message_size: TxDescriptor<{ new: number }> =
  c3vh014cqgmrfd as TxDescriptor<{ new: number }>;
const call_Configuration_set_hrmp_max_parachain_outbound_channels: TxDescriptor<{ new: number }> =
  c3vh014cqgmrfd as TxDescriptor<{ new: number }>;
const call_Configuration_set_hrmp_max_message_num_per_candidate: TxDescriptor<{ new: number }> =
  c3vh014cqgmrfd as TxDescriptor<{ new: number }>;
const call_Configuration_set_pvf_voting_ttl: TxDescriptor<{ new: number }> =
  c3vh014cqgmrfd as TxDescriptor<{ new: number }>;
const call_Configuration_set_minimum_validation_upgrade_delay: TxDescriptor<{ new: number }> =
  c3vh014cqgmrfd as TxDescriptor<{ new: number }>;
const call_Configuration_set_bypass_consistency_check: TxDescriptor<{ new: boolean }> =
  "2f6mha3v4ooda" as TxDescriptor<{ new: boolean }>;
const call_Configuration_set_async_backing_params: TxDescriptor<{
  new: Anonymize<Iavuvfkop6318c>;
}> = "asqjdhasi408s" as TxDescriptor<{ new: Anonymize<Iavuvfkop6318c> }>;
const call_Configuration_set_executor_params: TxDescriptor<{ new: Anonymize<I79n5nka727sg9> }> =
  "bgp9od4kojpki" as TxDescriptor<{ new: Anonymize<I79n5nka727sg9> }>;
const call_Configuration_set_on_demand_base_fee: TxDescriptor<{ new: bigint }> =
  c9jsikd1ghmc7l as TxDescriptor<{ new: bigint }>;
const call_Configuration_set_on_demand_fee_variability: TxDescriptor<{ new: number }> =
  c3vh014cqgmrfd as TxDescriptor<{ new: number }>;
const call_Configuration_set_on_demand_queue_max_size: TxDescriptor<{ new: number }> =
  c3vh014cqgmrfd as TxDescriptor<{ new: number }>;
const call_Configuration_set_on_demand_target_queue_utilization: TxDescriptor<{ new: number }> =
  c3vh014cqgmrfd as TxDescriptor<{ new: number }>;
const call_Configuration_set_on_demand_ttl: TxDescriptor<{ new: number }> =
  c3vh014cqgmrfd as TxDescriptor<{ new: number }>;
const call_Configuration_set_minimum_backing_votes: TxDescriptor<{ new: number }> =
  c3vh014cqgmrfd as TxDescriptor<{ new: number }>;
const call_Configuration_set_node_feature: TxDescriptor<{ index: number; value: boolean }> =
  "aid4btmkr5thp" as TxDescriptor<{ index: number; value: boolean }>;
const call_Configuration_set_approval_voting_params: TxDescriptor<{ new: number }> =
  c3vh014cqgmrfd as TxDescriptor<{ new: number }>;
const call_ParaInherent_enter: TxDescriptor<{ data: Anonymize<I9hpj128t7mg6a> }> =
  "4ej35gb01m5ai" as TxDescriptor<{ data: Anonymize<I9hpj128t7mg6a> }>;
const call_Paras_force_set_current_code: TxDescriptor<{ para: number; new_code: Binary }> =
  c1k3urvkqqshbc as TxDescriptor<{ para: number; new_code: Binary }>;
const call_Paras_force_set_current_head: TxDescriptor<{ para: number; new_head: Binary }> =
  c2ff0ffsh15vej as TxDescriptor<{ para: number; new_head: Binary }>;
const call_Paras_force_schedule_code_upgrade: TxDescriptor<{
  para: number;
  new_code: Binary;
  relay_parent_number: number;
}> = "1orfg86bkg123" as TxDescriptor<{
  para: number;
  new_code: Binary;
  relay_parent_number: number;
}>;
const call_Paras_force_note_new_head: TxDescriptor<{ para: number; new_head: Binary }> =
  c2ff0ffsh15vej as TxDescriptor<{ para: number; new_head: Binary }>;
const call_Paras_force_queue_action: TxDescriptor<{ para: number }> =
  caus4cb3drhu9q as TxDescriptor<{ para: number }>;
const call_Paras_add_trusted_validation_code: TxDescriptor<{ validation_code: Binary }> =
  "vnsat10lv9d6" as TxDescriptor<{ validation_code: Binary }>;
const call_Paras_poke_unused_validation_code: TxDescriptor<{ validation_code_hash: Binary }> =
  "fqm1da2k7es4d" as TxDescriptor<{ validation_code_hash: Binary }>;
const call_Paras_include_pvf_check_statement: TxDescriptor<{
  stmt: Anonymize<I3h1ccufdk38ej>;
  signature: Binary;
}> = "4aouqiv2fh67c" as TxDescriptor<{ stmt: Anonymize<I3h1ccufdk38ej>; signature: Binary }>;
const call_Paras_force_set_most_recent_context: TxDescriptor<{ para: number; context: number }> =
  "9tmok5kceg2bg" as TxDescriptor<{ para: number; context: number }>;
const call_Initializer_force_approve: TxDescriptor<{ up_to: number }> =
  "85icj2qbjeqbe" as TxDescriptor<{ up_to: number }>;
const call_Hrmp_hrmp_init_open_channel: TxDescriptor<{
  recipient: number;
  proposed_max_capacity: number;
  proposed_max_message_size: number;
}> = "buhbp68e6tkct" as TxDescriptor<{
  recipient: number;
  proposed_max_capacity: number;
  proposed_max_message_size: number;
}>;
const call_Hrmp_hrmp_accept_open_channel: TxDescriptor<{ sender: number }> =
  "drevppfiubhve" as TxDescriptor<{ sender: number }>;
const call_Hrmp_hrmp_close_channel: TxDescriptor<{ channel_id: Anonymize<I50mrcbubp554e> }> =
  "9s2h36kr71vk9" as TxDescriptor<{ channel_id: Anonymize<I50mrcbubp554e> }>;
const call_Hrmp_force_clean_hrmp: TxDescriptor<{
  para: number;
  num_inbound: number;
  num_outbound: number;
}> = "4lkbiubo9ogq9" as TxDescriptor<{ para: number; num_inbound: number; num_outbound: number }>;
const call_Hrmp_force_process_hrmp_open: TxDescriptor<{ channels: number }> =
  cd1baei7m8gkhk as TxDescriptor<{ channels: number }>;
const call_Hrmp_force_process_hrmp_close: TxDescriptor<{ channels: number }> =
  cd1baei7m8gkhk as TxDescriptor<{ channels: number }>;
const call_Hrmp_hrmp_cancel_open_request: TxDescriptor<{
  channel_id: Anonymize<I50mrcbubp554e>;
  open_requests: number;
}> = "96ftepqm4vs7m" as TxDescriptor<{
  channel_id: Anonymize<I50mrcbubp554e>;
  open_requests: number;
}>;
const call_Hrmp_force_open_hrmp_channel: TxDescriptor<{
  sender: number;
  recipient: number;
  max_capacity: number;
  max_message_size: number;
}> = cc3430470j4mbv as TxDescriptor<{
  sender: number;
  recipient: number;
  max_capacity: number;
  max_message_size: number;
}>;
const call_Hrmp_establish_system_channel: TxDescriptor<{ sender: number; recipient: number }> =
  c50mrcbubp554e as TxDescriptor<{ sender: number; recipient: number }>;
const call_Hrmp_poke_channel_deposits: TxDescriptor<{ sender: number; recipient: number }> =
  c50mrcbubp554e as TxDescriptor<{ sender: number; recipient: number }>;
const call_ParasDisputes_force_unfreeze: TxDescriptor<undefined> =
  c39ibapn5dm6tr as TxDescriptor<undefined>;
const call_ParasSlashing_report_dispute_lost_unsigned: TxDescriptor<{
  dispute_proof: Anonymize<Iag14tqe65tvpf>;
  key_owner_proof: Anonymize<I3ia7aufsoj0l1>;
}> = "1ur1874hp9ar5" as TxDescriptor<{
  dispute_proof: Anonymize<Iag14tqe65tvpf>;
  key_owner_proof: Anonymize<I3ia7aufsoj0l1>;
}>;
const call_MessageQueue_reap_page: TxDescriptor<{
  message_origin: ParachainsInclusionAggregateMessageOrigin;
  page_index: number;
}> = "3f1tq7m3kurig" as TxDescriptor<{
  message_origin: ParachainsInclusionAggregateMessageOrigin;
  page_index: number;
}>;
const call_MessageQueue_execute_overweight: TxDescriptor<{
  message_origin: ParachainsInclusionAggregateMessageOrigin;
  page: number;
  index: number;
  weight_limit: Anonymize<I4q39t5hn830vp>;
}> = "fhnflnkf9f278" as TxDescriptor<{
  message_origin: ParachainsInclusionAggregateMessageOrigin;
  page: number;
  index: number;
  weight_limit: Anonymize<I4q39t5hn830vp>;
}>;
const call_OnDemandAssignmentProvider_place_order_allow_death: TxDescriptor<{
  max_amount: bigint;
  para_id: number;
}> = caa7g3f5tlv3gf as TxDescriptor<{ max_amount: bigint; para_id: number }>;
const call_OnDemandAssignmentProvider_place_order_keep_alive: TxDescriptor<{
  max_amount: bigint;
  para_id: number;
}> = caa7g3f5tlv3gf as TxDescriptor<{ max_amount: bigint; para_id: number }>;
const call_Registrar_register: TxDescriptor<{
  id: number;
  genesis_head: Binary;
  validation_code: Binary;
}> = "7mf0sij342109" as TxDescriptor<{ id: number; genesis_head: Binary; validation_code: Binary }>;
const call_Registrar_force_register: TxDescriptor<{
  who: SS58String;
  deposit: bigint;
  id: number;
  genesis_head: Binary;
  validation_code: Binary;
}> = "bvirp862qkkup" as TxDescriptor<{
  who: SS58String;
  deposit: bigint;
  id: number;
  genesis_head: Binary;
  validation_code: Binary;
}>;
const call_Registrar_deregister: TxDescriptor<{ id: number }> = cc5b47dj4coa3r as TxDescriptor<{
  id: number;
}>;
const call_Registrar_swap: TxDescriptor<{ id: number; other: number }> =
  "dehabrqi23sc0" as TxDescriptor<{ id: number; other: number }>;
const call_Registrar_remove_lock: TxDescriptor<{ para: number }> = caus4cb3drhu9q as TxDescriptor<{
  para: number;
}>;
const call_Registrar_reserve: TxDescriptor<undefined> = c39ibapn5dm6tr as TxDescriptor<undefined>;
const call_Registrar_add_lock: TxDescriptor<{ para: number }> = caus4cb3drhu9q as TxDescriptor<{
  para: number;
}>;
const call_Registrar_schedule_code_upgrade: TxDescriptor<{ para: number; new_code: Binary }> =
  c1k3urvkqqshbc as TxDescriptor<{ para: number; new_code: Binary }>;
const call_Registrar_set_current_head: TxDescriptor<{ para: number; new_head: Binary }> =
  c2ff0ffsh15vej as TxDescriptor<{ para: number; new_head: Binary }>;
const call_Slots_force_lease: TxDescriptor<{
  para: number;
  leaser: SS58String;
  amount: bigint;
  period_begin: number;
  period_count: number;
}> = "dfpo6162k0hq" as TxDescriptor<{
  para: number;
  leaser: SS58String;
  amount: bigint;
  period_begin: number;
  period_count: number;
}>;
const call_Slots_clear_all_leases: TxDescriptor<{ para: number }> = caus4cb3drhu9q as TxDescriptor<{
  para: number;
}>;
const call_Slots_trigger_onboard: TxDescriptor<{ para: number }> = caus4cb3drhu9q as TxDescriptor<{
  para: number;
}>;
const call_Auctions_new_auction: TxDescriptor<{ duration: number; lease_period_index: number }> =
  "19hvnphoaj44l" as TxDescriptor<{ duration: number; lease_period_index: number }>;
const call_Auctions_bid: TxDescriptor<{
  para: number;
  auction_index: number;
  first_slot: number;
  last_slot: number;
  amount: bigint;
}> = "1ng31ej27mh4k" as TxDescriptor<{
  para: number;
  auction_index: number;
  first_slot: number;
  last_slot: number;
  amount: bigint;
}>;
const call_Auctions_cancel_auction: TxDescriptor<undefined> =
  c39ibapn5dm6tr as TxDescriptor<undefined>;
const call_Crowdloan_create: TxDescriptor<{
  index: number;
  cap: bigint;
  first_period: number;
  last_period: number;
  end: number;
  verifier: Anonymize<I8t18p6mokc3s4>;
}> = c3js6c9fubdele as TxDescriptor<{
  index: number;
  cap: bigint;
  first_period: number;
  last_period: number;
  end: number;
  verifier: Anonymize<I8t18p6mokc3s4>;
}>;
const call_Crowdloan_contribute: TxDescriptor<{
  index: number;
  value: bigint;
  signature: Anonymize<I7us28h09nc5sv>;
}> = "6n5jj22t7mm7i" as TxDescriptor<{
  index: number;
  value: bigint;
  signature: Anonymize<I7us28h09nc5sv>;
}>;
const call_Crowdloan_withdraw: TxDescriptor<{ who: SS58String; index: number }> =
  ca1u3jll6a06ae as TxDescriptor<{ who: SS58String; index: number }>;
const call_Crowdloan_refund: TxDescriptor<{ index: number }> = c666bl2fqjkejo as TxDescriptor<{
  index: number;
}>;
const call_Crowdloan_dissolve: TxDescriptor<{ index: number }> = c666bl2fqjkejo as TxDescriptor<{
  index: number;
}>;
const call_Crowdloan_edit: TxDescriptor<{
  index: number;
  cap: bigint;
  first_period: number;
  last_period: number;
  end: number;
  verifier: Anonymize<I8t18p6mokc3s4>;
}> = c3js6c9fubdele as TxDescriptor<{
  index: number;
  cap: bigint;
  first_period: number;
  last_period: number;
  end: number;
  verifier: Anonymize<I8t18p6mokc3s4>;
}>;
const call_Crowdloan_add_memo: TxDescriptor<{ index: number; memo: Binary }> =
  "7cl9esn1l72m7" as TxDescriptor<{ index: number; memo: Binary }>;
const call_Crowdloan_poke: TxDescriptor<{ index: number }> = c666bl2fqjkejo as TxDescriptor<{
  index: number;
}>;
const call_Crowdloan_contribute_all: TxDescriptor<{
  index: number;
  signature: Anonymize<I7us28h09nc5sv>;
}> = "3k27o64k49es2" as TxDescriptor<{ index: number; signature: Anonymize<I7us28h09nc5sv> }>;
const call_Coretime_request_core_count: TxDescriptor<{ count: number }> =
  "afscmv8tjf0ou" as TxDescriptor<{ count: number }>;
const call_Coretime_assign_core: TxDescriptor<{
  core: number;
  begin: number;
  assignment: Anonymize<Id1dkuivtmc10j>;
  end_hint: Anonymize<I4arjljr6dpflb>;
}> = "6ls6t1gggjtv6" as TxDescriptor<{
  core: number;
  begin: number;
  assignment: Anonymize<Id1dkuivtmc10j>;
  end_hint: Anonymize<I4arjljr6dpflb>;
}>;
const call_XcmPallet_send: TxDescriptor<{ dest: XcmVersionedLocation; message: XcmVersionedXcm }> =
  "3d5n20t9iopr1" as TxDescriptor<{ dest: XcmVersionedLocation; message: XcmVersionedXcm }>;
const call_XcmPallet_teleport_assets: TxDescriptor<{
  dest: XcmVersionedLocation;
  beneficiary: XcmVersionedLocation;
  assets: XcmVersionedAssets;
  fee_asset_item: number;
}> = cqg2kc7rjuace as TxDescriptor<{
  dest: XcmVersionedLocation;
  beneficiary: XcmVersionedLocation;
  assets: XcmVersionedAssets;
  fee_asset_item: number;
}>;
const call_XcmPallet_reserve_transfer_assets: TxDescriptor<{
  dest: XcmVersionedLocation;
  beneficiary: XcmVersionedLocation;
  assets: XcmVersionedAssets;
  fee_asset_item: number;
}> = cqg2kc7rjuace as TxDescriptor<{
  dest: XcmVersionedLocation;
  beneficiary: XcmVersionedLocation;
  assets: XcmVersionedAssets;
  fee_asset_item: number;
}>;
const call_XcmPallet_execute: TxDescriptor<{
  message: XcmVersionedXcm1;
  max_weight: Anonymize<I4q39t5hn830vp>;
}> = "94f2uf779bu0k" as TxDescriptor<{
  message: XcmVersionedXcm1;
  max_weight: Anonymize<I4q39t5hn830vp>;
}>;
const call_XcmPallet_force_xcm_version: TxDescriptor<{
  location: Anonymize<Ieq428hdmibf6m>;
  version: number;
}> = cergfrmj3c9dor as TxDescriptor<{ location: Anonymize<Ieq428hdmibf6m>; version: number }>;
const call_XcmPallet_force_default_xcm_version: TxDescriptor<{
  maybe_xcm_version: Anonymize<I4arjljr6dpflb>;
}> = "c76kfh5ebqkpl" as TxDescriptor<{ maybe_xcm_version: Anonymize<I4arjljr6dpflb> }>;
const call_XcmPallet_force_subscribe_version_notify: TxDescriptor<{
  location: XcmVersionedLocation;
}> = cb67j9jdqr7ekj as TxDescriptor<{ location: XcmVersionedLocation }>;
const call_XcmPallet_force_unsubscribe_version_notify: TxDescriptor<{
  location: XcmVersionedLocation;
}> = cb67j9jdqr7ekj as TxDescriptor<{ location: XcmVersionedLocation }>;
const call_XcmPallet_limited_reserve_transfer_assets: TxDescriptor<{
  dest: XcmVersionedLocation;
  beneficiary: XcmVersionedLocation;
  assets: XcmVersionedAssets;
  fee_asset_item: number;
  weight_limit: XcmV3WeightLimit;
}> = cbk7a73iljhv98 as TxDescriptor<{
  dest: XcmVersionedLocation;
  beneficiary: XcmVersionedLocation;
  assets: XcmVersionedAssets;
  fee_asset_item: number;
  weight_limit: XcmV3WeightLimit;
}>;
const call_XcmPallet_limited_teleport_assets: TxDescriptor<{
  dest: XcmVersionedLocation;
  beneficiary: XcmVersionedLocation;
  assets: XcmVersionedAssets;
  fee_asset_item: number;
  weight_limit: XcmV3WeightLimit;
}> = cbk7a73iljhv98 as TxDescriptor<{
  dest: XcmVersionedLocation;
  beneficiary: XcmVersionedLocation;
  assets: XcmVersionedAssets;
  fee_asset_item: number;
  weight_limit: XcmV3WeightLimit;
}>;
const call_XcmPallet_force_suspension: TxDescriptor<{ suspended: boolean }> =
  "bgm4rnf22lal1" as TxDescriptor<{ suspended: boolean }>;
const call_XcmPallet_transfer_assets: TxDescriptor<{
  dest: XcmVersionedLocation;
  beneficiary: XcmVersionedLocation;
  assets: XcmVersionedAssets;
  fee_asset_item: number;
  weight_limit: XcmV3WeightLimit;
}> = cbk7a73iljhv98 as TxDescriptor<{
  dest: XcmVersionedLocation;
  beneficiary: XcmVersionedLocation;
  assets: XcmVersionedAssets;
  fee_asset_item: number;
  weight_limit: XcmV3WeightLimit;
}>;
const call_Beefy_report_equivocation: TxDescriptor<{
  equivocation_proof: Anonymize<I6lgkrki6lhal>;
  key_owner_proof: Anonymize<I3ia7aufsoj0l1>;
}> = c894urueu7skf3 as TxDescriptor<{
  equivocation_proof: Anonymize<I6lgkrki6lhal>;
  key_owner_proof: Anonymize<I3ia7aufsoj0l1>;
}>;
const call_Beefy_report_equivocation_unsigned: TxDescriptor<{
  equivocation_proof: Anonymize<I6lgkrki6lhal>;
  key_owner_proof: Anonymize<I3ia7aufsoj0l1>;
}> = c894urueu7skf3 as TxDescriptor<{
  equivocation_proof: Anonymize<I6lgkrki6lhal>;
  key_owner_proof: Anonymize<I3ia7aufsoj0l1>;
}>;
const call_Beefy_set_new_genesis: TxDescriptor<{ delay_in_blocks: number }> =
  "emqna2uucuei9" as TxDescriptor<{ delay_in_blocks: number }>;
const call_IdentityMigrator_reap_identity: TxDescriptor<{ who: SS58String }> =
  c4cbvqmqadhrea as TxDescriptor<{ who: SS58String }>;
const call_IdentityMigrator_poke_deposit: TxDescriptor<{ who: SS58String }> =
  c4cbvqmqadhrea as TxDescriptor<{ who: SS58String }>;
const call_ParasSudoWrapper_sudo_schedule_para_initialize: TxDescriptor<{
  id: number;
  genesis: Anonymize<I2duhnt686rv0q>;
}> = "9geq5evbpu4im" as TxDescriptor<{ id: number; genesis: Anonymize<I2duhnt686rv0q> }>;
const call_ParasSudoWrapper_sudo_schedule_para_cleanup: TxDescriptor<{ id: number }> =
  cc5b47dj4coa3r as TxDescriptor<{ id: number }>;
const call_ParasSudoWrapper_sudo_schedule_parathread_upgrade: TxDescriptor<{ id: number }> =
  cc5b47dj4coa3r as TxDescriptor<{ id: number }>;
const call_ParasSudoWrapper_sudo_schedule_parachain_downgrade: TxDescriptor<{ id: number }> =
  cc5b47dj4coa3r as TxDescriptor<{ id: number }>;
const call_ParasSudoWrapper_sudo_queue_downward_xcm: TxDescriptor<{
  id: number;
  xcm: XcmVersionedXcm;
}> = "e3vanb7qoq2qr" as TxDescriptor<{ id: number; xcm: XcmVersionedXcm }>;
const call_ParasSudoWrapper_sudo_establish_hrmp_channel: TxDescriptor<{
  sender: number;
  recipient: number;
  max_capacity: number;
  max_message_size: number;
}> = cc3430470j4mbv as TxDescriptor<{
  sender: number;
  recipient: number;
  max_capacity: number;
  max_message_size: number;
}>;
const call_AssignedSlots_assign_perm_parachain_slot: TxDescriptor<{ id: number }> =
  cc5b47dj4coa3r as TxDescriptor<{ id: number }>;
const call_AssignedSlots_assign_temp_parachain_slot: TxDescriptor<{
  id: number;
  lease_period_start: PolkadotRuntimeCommonAssignedSlotsSlotLeasePeriodStart;
}> = "b4b26jo7ljjg3" as TxDescriptor<{
  id: number;
  lease_period_start: PolkadotRuntimeCommonAssignedSlotsSlotLeasePeriodStart;
}>;
const call_AssignedSlots_unassign_parachain_slot: TxDescriptor<{ id: number }> =
  cc5b47dj4coa3r as TxDescriptor<{ id: number }>;
const call_AssignedSlots_set_max_permanent_slots: TxDescriptor<{ slots: number }> =
  c9d5h5irbki7mm as TxDescriptor<{ slots: number }>;
const call_AssignedSlots_set_max_temporary_slots: TxDescriptor<{ slots: number }> =
  c9d5h5irbki7mm as TxDescriptor<{ slots: number }>;
const call_ValidatorManager_register_validators: TxDescriptor<{
  validators: Anonymize<Ia2lhg7l2hilo3>;
}> = c97hfovkaaqb7h as TxDescriptor<{ validators: Anonymize<Ia2lhg7l2hilo3> }>;
const call_ValidatorManager_deregister_validators: TxDescriptor<{
  validators: Anonymize<Ia2lhg7l2hilo3>;
}> = c97hfovkaaqb7h as TxDescriptor<{ validators: Anonymize<Ia2lhg7l2hilo3> }>;
const call_StateTrieMigration_control_auto_migration: TxDescriptor<{
  maybe_config: Anonymize<Ib17t3992hb64n>;
}> = "7psec5e6ghc64" as TxDescriptor<{ maybe_config: Anonymize<Ib17t3992hb64n> }>;
const call_StateTrieMigration_continue_migrate: TxDescriptor<{
  limits: Anonymize<I215mkl885p4da>;
  real_size_upper: number;
  witness_task: Anonymize<I2sqgqj7ib3ine>;
}> = "bnsai0l0rfp2" as TxDescriptor<{
  limits: Anonymize<I215mkl885p4da>;
  real_size_upper: number;
  witness_task: Anonymize<I2sqgqj7ib3ine>;
}>;
const call_StateTrieMigration_migrate_custom_top: TxDescriptor<{
  keys: Anonymize<Itom7fk49o0c9>;
  witness_size: number;
}> = "585tk8khua0gk" as TxDescriptor<{ keys: Anonymize<Itom7fk49o0c9>; witness_size: number }>;
const call_StateTrieMigration_migrate_custom_child: TxDescriptor<{
  root: Binary;
  child_keys: Anonymize<Itom7fk49o0c9>;
  total_size: number;
}> = "3ut99di214ru2" as TxDescriptor<{
  root: Binary;
  child_keys: Anonymize<Itom7fk49o0c9>;
  total_size: number;
}>;
const call_StateTrieMigration_set_signed_max_limits: TxDescriptor<{
  limits: Anonymize<I215mkl885p4da>;
}> = "emkp87d26vsbh" as TxDescriptor<{ limits: Anonymize<I215mkl885p4da> }>;
const call_StateTrieMigration_force_set_progress: TxDescriptor<{
  progress_top: Anonymize<I2jvfaenrpg1ai>;
  progress_child: Anonymize<I2jvfaenrpg1ai>;
}> = "4guudoaeoq2e9" as TxDescriptor<{
  progress_top: Anonymize<I2jvfaenrpg1ai>;
  progress_child: Anonymize<I2jvfaenrpg1ai>;
}>;
const call_RootTesting_fill_block: TxDescriptor<{ ratio: number }> =
  "enjibnb78vnl0" as TxDescriptor<{ ratio: number }>;
const call_RootTesting_trigger_defensive: TxDescriptor<undefined> =
  c39ibapn5dm6tr as TxDescriptor<undefined>;
const call_Sudo_sudo: TxDescriptor<{ call: Anonymize<I5v7s5dm78b0mh> }> =
  cb6n6j9tk6ut3b as TxDescriptor<{ call: Anonymize<I5v7s5dm78b0mh> }>;
const call_Sudo_sudo_unchecked_weight: TxDescriptor<{
  call: Anonymize<I5v7s5dm78b0mh>;
  weight: Anonymize<I4q39t5hn830vp>;
}> = ce1uvn9e6ki8dd as TxDescriptor<{
  call: Anonymize<I5v7s5dm78b0mh>;
  weight: Anonymize<I4q39t5hn830vp>;
}>;
const call_Sudo_set_key: TxDescriptor<{ new: MultiAddress }> = "cnonnse26sae7" as TxDescriptor<{
  new: MultiAddress;
}>;
const call_Sudo_sudo_as: TxDescriptor<{ who: MultiAddress; call: Anonymize<I5v7s5dm78b0mh> }> =
  "94s8ivufjp2ik" as TxDescriptor<{ who: MultiAddress; call: Anonymize<I5v7s5dm78b0mh> }>;
const call_Sudo_remove_key: TxDescriptor<undefined> = c39ibapn5dm6tr as TxDescriptor<undefined>;
const evt_System_ExtrinsicSuccess: PlainDescriptor<{ dispatch_info: Anonymize<Ia2iiohca2et6f> }> =
  "ede1ukavoderd" as PlainDescriptor<{ dispatch_info: Anonymize<Ia2iiohca2et6f> }>;
const evt_System_ExtrinsicFailed: PlainDescriptor<{
  dispatch_error: DispatchError;
  dispatch_info: Anonymize<Ia2iiohca2et6f>;
}> = "ennefu6o2bgdm" as PlainDescriptor<{
  dispatch_error: DispatchError;
  dispatch_info: Anonymize<Ia2iiohca2et6f>;
}>;
const evt_System_CodeUpdated: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const evt_System_NewAccount: PlainDescriptor<{ account: SS58String }> =
  ccbccs0ug47ilf as PlainDescriptor<{ account: SS58String }>;
const evt_System_KilledAccount: PlainDescriptor<{ account: SS58String }> =
  ccbccs0ug47ilf as PlainDescriptor<{ account: SS58String }>;
const evt_System_Remarked: PlainDescriptor<{ sender: SS58String; hash: Binary }> =
  "eob37pbjnvmkj" as PlainDescriptor<{ sender: SS58String; hash: Binary }>;
const evt_System_UpgradeAuthorized: PlainDescriptor<{ code_hash: Binary; check_version: boolean }> =
  cb3qt1mgvgmbgi as PlainDescriptor<{ code_hash: Binary; check_version: boolean }>;
const evt_Indices_IndexAssigned: PlainDescriptor<{ who: SS58String; index: number }> =
  ca1u3jll6a06ae as PlainDescriptor<{ who: SS58String; index: number }>;
const evt_Indices_IndexFreed: PlainDescriptor<{ index: number }> =
  c666bl2fqjkejo as PlainDescriptor<{ index: number }>;
const evt_Indices_IndexFrozen: PlainDescriptor<{ who: SS58String; index: number }> =
  ca1u3jll6a06ae as PlainDescriptor<{ who: SS58String; index: number }>;
const evt_Balances_Endowed: PlainDescriptor<{ account: SS58String; free_balance: bigint }> =
  ccv68aq8841478 as PlainDescriptor<{ account: SS58String; free_balance: bigint }>;
const evt_Balances_DustLost: PlainDescriptor<{ account: SS58String; amount: bigint }> =
  cc262ibdoec56a as PlainDescriptor<{ account: SS58String; amount: bigint }>;
const evt_Balances_Transfer: PlainDescriptor<{ from: SS58String; to: SS58String; amount: bigint }> =
  cflcfm9b6nlmdd as PlainDescriptor<{ from: SS58String; to: SS58String; amount: bigint }>;
const evt_Balances_BalanceSet: PlainDescriptor<{ who: SS58String; free: bigint }> =
  cjrsf4mnp3eka as PlainDescriptor<{ who: SS58String; free: bigint }>;
const evt_Balances_Reserved: PlainDescriptor<{ who: SS58String; amount: bigint }> =
  cd5fm4p8lj5qgi as PlainDescriptor<{ who: SS58String; amount: bigint }>;
const evt_Balances_Unreserved: PlainDescriptor<{ who: SS58String; amount: bigint }> =
  cd5fm4p8lj5qgi as PlainDescriptor<{ who: SS58String; amount: bigint }>;
const evt_Balances_ReserveRepatriated: PlainDescriptor<{
  from: SS58String;
  to: SS58String;
  amount: bigint;
  destination_status: BalanceStatus;
}> = cdm5rqp3duosod as PlainDescriptor<{
  from: SS58String;
  to: SS58String;
  amount: bigint;
  destination_status: BalanceStatus;
}>;
const evt_Balances_Deposit: PlainDescriptor<{ who: SS58String; amount: bigint }> =
  cd5fm4p8lj5qgi as PlainDescriptor<{ who: SS58String; amount: bigint }>;
const evt_Balances_Withdraw: PlainDescriptor<{ who: SS58String; amount: bigint }> =
  cd5fm4p8lj5qgi as PlainDescriptor<{ who: SS58String; amount: bigint }>;
const evt_Balances_Slashed: PlainDescriptor<{ who: SS58String; amount: bigint }> =
  cd5fm4p8lj5qgi as PlainDescriptor<{ who: SS58String; amount: bigint }>;
const evt_Balances_Minted: PlainDescriptor<{ who: SS58String; amount: bigint }> =
  cd5fm4p8lj5qgi as PlainDescriptor<{ who: SS58String; amount: bigint }>;
const evt_Balances_Burned: PlainDescriptor<{ who: SS58String; amount: bigint }> =
  cd5fm4p8lj5qgi as PlainDescriptor<{ who: SS58String; amount: bigint }>;
const evt_Balances_Suspended: PlainDescriptor<{ who: SS58String; amount: bigint }> =
  cd5fm4p8lj5qgi as PlainDescriptor<{ who: SS58String; amount: bigint }>;
const evt_Balances_Restored: PlainDescriptor<{ who: SS58String; amount: bigint }> =
  cd5fm4p8lj5qgi as PlainDescriptor<{ who: SS58String; amount: bigint }>;
const evt_Balances_Upgraded: PlainDescriptor<{ who: SS58String }> =
  c4cbvqmqadhrea as PlainDescriptor<{ who: SS58String }>;
const evt_Balances_Issued: PlainDescriptor<{ amount: bigint }> = c3qt1hgg4djhgb as PlainDescriptor<{
  amount: bigint;
}>;
const evt_Balances_Rescinded: PlainDescriptor<{ amount: bigint }> =
  c3qt1hgg4djhgb as PlainDescriptor<{ amount: bigint }>;
const evt_Balances_Locked: PlainDescriptor<{ who: SS58String; amount: bigint }> =
  cd5fm4p8lj5qgi as PlainDescriptor<{ who: SS58String; amount: bigint }>;
const evt_Balances_Unlocked: PlainDescriptor<{ who: SS58String; amount: bigint }> =
  cd5fm4p8lj5qgi as PlainDescriptor<{ who: SS58String; amount: bigint }>;
const evt_Balances_Frozen: PlainDescriptor<{ who: SS58String; amount: bigint }> =
  cd5fm4p8lj5qgi as PlainDescriptor<{ who: SS58String; amount: bigint }>;
const evt_Balances_Thawed: PlainDescriptor<{ who: SS58String; amount: bigint }> =
  cd5fm4p8lj5qgi as PlainDescriptor<{ who: SS58String; amount: bigint }>;
const evt_Balances_TotalIssuanceForced: PlainDescriptor<{ old: bigint; new: bigint }> =
  c4fooe9dun9o0t as PlainDescriptor<{ old: bigint; new: bigint }>;
const evt_TransactionPayment_TransactionFeePaid: PlainDescriptor<{
  who: SS58String;
  actual_fee: bigint;
  tip: bigint;
}> = "er2cke86dqbr2" as PlainDescriptor<{ who: SS58String; actual_fee: bigint; tip: bigint }>;
const evt_Offences_Offence: PlainDescriptor<{ kind: Binary; timeslot: Binary }> =
  "41n4hddrgegvb" as PlainDescriptor<{ kind: Binary; timeslot: Binary }>;
const evt_Session_NewSession: PlainDescriptor<{ session_index: number }> =
  "2hq50pu2kdjpo" as PlainDescriptor<{ session_index: number }>;
const evt_Grandpa_NewAuthorities: PlainDescriptor<{ authority_set: Anonymize<I2qinct8jq4bqe> }> =
  "b31jedabim0q7" as PlainDescriptor<{ authority_set: Anonymize<I2qinct8jq4bqe> }>;
const evt_Grandpa_Paused: PlainDescriptor<undefined> = c39ibapn5dm6tr as PlainDescriptor<undefined>;
const evt_Grandpa_Resumed: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const evt_Treasury_Proposed: PlainDescriptor<{ proposal_index: number }> =
  "44hc4lgsn4o1j" as PlainDescriptor<{ proposal_index: number }>;
const evt_Treasury_Spending: PlainDescriptor<{ budget_remaining: bigint }> =
  "8iksqi3eani0a" as PlainDescriptor<{ budget_remaining: bigint }>;
const evt_Treasury_Awarded: PlainDescriptor<{
  proposal_index: number;
  award: bigint;
  account: SS58String;
}> = "16enopmju1p0q" as PlainDescriptor<{
  proposal_index: number;
  award: bigint;
  account: SS58String;
}>;
const evt_Treasury_Rejected: PlainDescriptor<{ proposal_index: number; slashed: bigint }> =
  "fgqhle2413de7" as PlainDescriptor<{ proposal_index: number; slashed: bigint }>;
const evt_Treasury_Burnt: PlainDescriptor<{ burnt_funds: bigint }> =
  "43kq8qudg7pq9" as PlainDescriptor<{ burnt_funds: bigint }>;
const evt_Treasury_Rollover: PlainDescriptor<{ rollover_balance: bigint }> =
  "76riseemre533" as PlainDescriptor<{ rollover_balance: bigint }>;
const evt_Treasury_Deposit: PlainDescriptor<{ value: bigint }> = ce5v6njpckr05b as PlainDescriptor<{
  value: bigint;
}>;
const evt_Treasury_SpendApproved: PlainDescriptor<{
  proposal_index: number;
  amount: bigint;
  beneficiary: SS58String;
}> = "38bmcrmh852rk" as PlainDescriptor<{
  proposal_index: number;
  amount: bigint;
  beneficiary: SS58String;
}>;
const evt_Treasury_UpdatedInactive: PlainDescriptor<{ reactivated: bigint; deactivated: bigint }> =
  "4hcillge8de5f" as PlainDescriptor<{ reactivated: bigint; deactivated: bigint }>;
const evt_Treasury_AssetSpendApproved: PlainDescriptor<{
  index: number;
  asset_kind: PolkadotRuntimeCommonImplsVersionedLocatableAsset;
  amount: bigint;
  beneficiary: XcmVersionedLocation;
  valid_from: number;
  expire_at: number;
}> = "5nk68406dirvd" as PlainDescriptor<{
  index: number;
  asset_kind: PolkadotRuntimeCommonImplsVersionedLocatableAsset;
  amount: bigint;
  beneficiary: XcmVersionedLocation;
  valid_from: number;
  expire_at: number;
}>;
const evt_Treasury_AssetSpendVoided: PlainDescriptor<{ index: number }> =
  c666bl2fqjkejo as PlainDescriptor<{ index: number }>;
const evt_Treasury_Paid: PlainDescriptor<{ index: number; payment_id: bigint }> =
  cek7v4hrgnq6iv as PlainDescriptor<{ index: number; payment_id: bigint }>;
const evt_Treasury_PaymentFailed: PlainDescriptor<{ index: number; payment_id: bigint }> =
  cek7v4hrgnq6iv as PlainDescriptor<{ index: number; payment_id: bigint }>;
const evt_Treasury_SpendProcessed: PlainDescriptor<{ index: number }> =
  c666bl2fqjkejo as PlainDescriptor<{ index: number }>;
const evt_ConvictionVoting_Delegated: PlainDescriptor<[SS58String, SS58String]> =
  "c5oktqtdlvdvq" as PlainDescriptor<[SS58String, SS58String]>;
const evt_ConvictionVoting_Undelegated: PlainDescriptor<SS58String> =
  "5ud5q8n726j4" as PlainDescriptor<SS58String>;
const evt_Referenda_Submitted: PlainDescriptor<{
  index: number;
  track: number;
  proposal: PreimagesBounded;
}> = cdhr9v8mlnjej as PlainDescriptor<{ index: number; track: number; proposal: PreimagesBounded }>;
const evt_Referenda_DecisionDepositPlaced: PlainDescriptor<{
  index: number;
  who: SS58String;
  amount: bigint;
}> = c62nte77gksm0f as PlainDescriptor<{ index: number; who: SS58String; amount: bigint }>;
const evt_Referenda_DecisionDepositRefunded: PlainDescriptor<{
  index: number;
  who: SS58String;
  amount: bigint;
}> = c62nte77gksm0f as PlainDescriptor<{ index: number; who: SS58String; amount: bigint }>;
const evt_Referenda_DepositSlashed: PlainDescriptor<{ who: SS58String; amount: bigint }> =
  cd5fm4p8lj5qgi as PlainDescriptor<{ who: SS58String; amount: bigint }>;
const evt_Referenda_DecisionStarted: PlainDescriptor<{
  index: number;
  track: number;
  proposal: PreimagesBounded;
  tally: Anonymize<Ifsk7cbmtit1jd>;
}> = "932allgc83a4a" as PlainDescriptor<{
  index: number;
  track: number;
  proposal: PreimagesBounded;
  tally: Anonymize<Ifsk7cbmtit1jd>;
}>;
const evt_Referenda_ConfirmStarted: PlainDescriptor<{ index: number }> =
  c666bl2fqjkejo as PlainDescriptor<{ index: number }>;
const evt_Referenda_ConfirmAborted: PlainDescriptor<{ index: number }> =
  c666bl2fqjkejo as PlainDescriptor<{ index: number }>;
const evt_Referenda_Confirmed: PlainDescriptor<{
  index: number;
  tally: Anonymize<Ifsk7cbmtit1jd>;
}> = clhp45uime5tp as PlainDescriptor<{ index: number; tally: Anonymize<Ifsk7cbmtit1jd> }>;
const evt_Referenda_Approved: PlainDescriptor<{ index: number }> =
  c666bl2fqjkejo as PlainDescriptor<{ index: number }>;
const evt_Referenda_Rejected: PlainDescriptor<{ index: number; tally: Anonymize<Ifsk7cbmtit1jd> }> =
  clhp45uime5tp as PlainDescriptor<{ index: number; tally: Anonymize<Ifsk7cbmtit1jd> }>;
const evt_Referenda_TimedOut: PlainDescriptor<{ index: number; tally: Anonymize<Ifsk7cbmtit1jd> }> =
  clhp45uime5tp as PlainDescriptor<{ index: number; tally: Anonymize<Ifsk7cbmtit1jd> }>;
const evt_Referenda_Cancelled: PlainDescriptor<{
  index: number;
  tally: Anonymize<Ifsk7cbmtit1jd>;
}> = clhp45uime5tp as PlainDescriptor<{ index: number; tally: Anonymize<Ifsk7cbmtit1jd> }>;
const evt_Referenda_Killed: PlainDescriptor<{ index: number; tally: Anonymize<Ifsk7cbmtit1jd> }> =
  clhp45uime5tp as PlainDescriptor<{ index: number; tally: Anonymize<Ifsk7cbmtit1jd> }>;
const evt_Referenda_SubmissionDepositRefunded: PlainDescriptor<{
  index: number;
  who: SS58String;
  amount: bigint;
}> = c62nte77gksm0f as PlainDescriptor<{ index: number; who: SS58String; amount: bigint }>;
const evt_Referenda_MetadataSet: PlainDescriptor<{ index: number; hash: Binary }> =
  c50aq0q2l1cdkr as PlainDescriptor<{ index: number; hash: Binary }>;
const evt_Referenda_MetadataCleared: PlainDescriptor<{ index: number; hash: Binary }> =
  c50aq0q2l1cdkr as PlainDescriptor<{ index: number; hash: Binary }>;
const evt_FellowshipCollective_MemberAdded: PlainDescriptor<{ who: SS58String }> =
  c4cbvqmqadhrea as PlainDescriptor<{ who: SS58String }>;
const evt_FellowshipCollective_RankChanged: PlainDescriptor<{ who: SS58String; rank: number }> =
  cm1pm2vf6llcn as PlainDescriptor<{ who: SS58String; rank: number }>;
const evt_FellowshipCollective_MemberRemoved: PlainDescriptor<{ who: SS58String; rank: number }> =
  cm1pm2vf6llcn as PlainDescriptor<{ who: SS58String; rank: number }>;
const evt_FellowshipCollective_Voted: PlainDescriptor<{
  who: SS58String;
  poll: number;
  vote: Anonymize<I996080trg6urn>;
  tally: Anonymize<I6d634btv7misu>;
}> = "31vg1u4v55v90" as PlainDescriptor<{
  who: SS58String;
  poll: number;
  vote: Anonymize<I996080trg6urn>;
  tally: Anonymize<I6d634btv7misu>;
}>;
const evt_FellowshipCollective_MemberExchanged: PlainDescriptor<{
  who: SS58String;
  new_who: SS58String;
}> = "er6ck0tpfo7" as PlainDescriptor<{ who: SS58String; new_who: SS58String }>;
const evt_FellowshipReferenda_Submitted: PlainDescriptor<{
  index: number;
  track: number;
  proposal: PreimagesBounded;
}> = cdhr9v8mlnjej as PlainDescriptor<{ index: number; track: number; proposal: PreimagesBounded }>;
const evt_FellowshipReferenda_DecisionDepositPlaced: PlainDescriptor<{
  index: number;
  who: SS58String;
  amount: bigint;
}> = c62nte77gksm0f as PlainDescriptor<{ index: number; who: SS58String; amount: bigint }>;
const evt_FellowshipReferenda_DecisionDepositRefunded: PlainDescriptor<{
  index: number;
  who: SS58String;
  amount: bigint;
}> = c62nte77gksm0f as PlainDescriptor<{ index: number; who: SS58String; amount: bigint }>;
const evt_FellowshipReferenda_DepositSlashed: PlainDescriptor<{ who: SS58String; amount: bigint }> =
  cd5fm4p8lj5qgi as PlainDescriptor<{ who: SS58String; amount: bigint }>;
const evt_FellowshipReferenda_DecisionStarted: PlainDescriptor<{
  index: number;
  track: number;
  proposal: PreimagesBounded;
  tally: Anonymize<I6d634btv7misu>;
}> = "uu6tledtt64n" as PlainDescriptor<{
  index: number;
  track: number;
  proposal: PreimagesBounded;
  tally: Anonymize<I6d634btv7misu>;
}>;
const evt_FellowshipReferenda_ConfirmStarted: PlainDescriptor<{ index: number }> =
  c666bl2fqjkejo as PlainDescriptor<{ index: number }>;
const evt_FellowshipReferenda_ConfirmAborted: PlainDescriptor<{ index: number }> =
  c666bl2fqjkejo as PlainDescriptor<{ index: number }>;
const evt_FellowshipReferenda_Confirmed: PlainDescriptor<{
  index: number;
  tally: Anonymize<I6d634btv7misu>;
}> = c27notaksll8qt as PlainDescriptor<{ index: number; tally: Anonymize<I6d634btv7misu> }>;
const evt_FellowshipReferenda_Approved: PlainDescriptor<{ index: number }> =
  c666bl2fqjkejo as PlainDescriptor<{ index: number }>;
const evt_FellowshipReferenda_Rejected: PlainDescriptor<{
  index: number;
  tally: Anonymize<I6d634btv7misu>;
}> = c27notaksll8qt as PlainDescriptor<{ index: number; tally: Anonymize<I6d634btv7misu> }>;
const evt_FellowshipReferenda_TimedOut: PlainDescriptor<{
  index: number;
  tally: Anonymize<I6d634btv7misu>;
}> = c27notaksll8qt as PlainDescriptor<{ index: number; tally: Anonymize<I6d634btv7misu> }>;
const evt_FellowshipReferenda_Cancelled: PlainDescriptor<{
  index: number;
  tally: Anonymize<I6d634btv7misu>;
}> = c27notaksll8qt as PlainDescriptor<{ index: number; tally: Anonymize<I6d634btv7misu> }>;
const evt_FellowshipReferenda_Killed: PlainDescriptor<{
  index: number;
  tally: Anonymize<I6d634btv7misu>;
}> = c27notaksll8qt as PlainDescriptor<{ index: number; tally: Anonymize<I6d634btv7misu> }>;
const evt_FellowshipReferenda_SubmissionDepositRefunded: PlainDescriptor<{
  index: number;
  who: SS58String;
  amount: bigint;
}> = c62nte77gksm0f as PlainDescriptor<{ index: number; who: SS58String; amount: bigint }>;
const evt_FellowshipReferenda_MetadataSet: PlainDescriptor<{ index: number; hash: Binary }> =
  c50aq0q2l1cdkr as PlainDescriptor<{ index: number; hash: Binary }>;
const evt_FellowshipReferenda_MetadataCleared: PlainDescriptor<{ index: number; hash: Binary }> =
  c50aq0q2l1cdkr as PlainDescriptor<{ index: number; hash: Binary }>;
const evt_Whitelist_CallWhitelisted: PlainDescriptor<{ call_hash: Binary }> =
  c8413rb6im3iko as PlainDescriptor<{ call_hash: Binary }>;
const evt_Whitelist_WhitelistedCallRemoved: PlainDescriptor<{ call_hash: Binary }> =
  c8413rb6im3iko as PlainDescriptor<{ call_hash: Binary }>;
const evt_Whitelist_WhitelistedCallDispatched: PlainDescriptor<{
  call_hash: Binary;
  result: Anonymize<Idurpak8qagatr>;
}> = "7b8pe56shlide" as PlainDescriptor<{ call_hash: Binary; result: Anonymize<Idurpak8qagatr> }>;
const evt_Claims_Claimed: PlainDescriptor<{
  who: SS58String;
  ethereum_address: Binary;
  amount: bigint;
}> = "dhjiuhlaei3db" as PlainDescriptor<{
  who: SS58String;
  ethereum_address: Binary;
  amount: bigint;
}>;
const evt_Utility_BatchInterrupted: PlainDescriptor<{ index: number; error: DispatchError }> =
  "6tn8e5lqr339o" as PlainDescriptor<{ index: number; error: DispatchError }>;
const evt_Utility_BatchCompleted: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const evt_Utility_BatchCompletedWithErrors: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const evt_Utility_ItemCompleted: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const evt_Utility_ItemFailed: PlainDescriptor<{ error: DispatchError }> =
  "11lb9o37qkk4f" as PlainDescriptor<{ error: DispatchError }>;
const evt_Utility_DispatchedAs: PlainDescriptor<{ result: Anonymize<Idtdr91jmq5g4i> }> =
  ce5i8qqljk3tjb as PlainDescriptor<{ result: Anonymize<Idtdr91jmq5g4i> }>;
const evt_Identity_IdentitySet: PlainDescriptor<{ who: SS58String }> =
  c4cbvqmqadhrea as PlainDescriptor<{ who: SS58String }>;
const evt_Identity_IdentityCleared: PlainDescriptor<{ who: SS58String; deposit: bigint }> =
  cep1lmt6q3s6r3 as PlainDescriptor<{ who: SS58String; deposit: bigint }>;
const evt_Identity_IdentityKilled: PlainDescriptor<{ who: SS58String; deposit: bigint }> =
  cep1lmt6q3s6r3 as PlainDescriptor<{ who: SS58String; deposit: bigint }>;
const evt_Identity_JudgementRequested: PlainDescriptor<{
  who: SS58String;
  registrar_index: number;
}> = c1fac16213rie2 as PlainDescriptor<{ who: SS58String; registrar_index: number }>;
const evt_Identity_JudgementUnrequested: PlainDescriptor<{
  who: SS58String;
  registrar_index: number;
}> = c1fac16213rie2 as PlainDescriptor<{ who: SS58String; registrar_index: number }>;
const evt_Identity_JudgementGiven: PlainDescriptor<{
  target: SS58String;
  registrar_index: number;
}> = "fjt77oc391o43" as PlainDescriptor<{ target: SS58String; registrar_index: number }>;
const evt_Identity_RegistrarAdded: PlainDescriptor<{ registrar_index: number }> =
  "tvt1jsipv0lc" as PlainDescriptor<{ registrar_index: number }>;
const evt_Identity_SubIdentityAdded: PlainDescriptor<{
  sub: SS58String;
  main: SS58String;
  deposit: bigint;
}> = cck3mveut33f44 as PlainDescriptor<{ sub: SS58String; main: SS58String; deposit: bigint }>;
const evt_Identity_SubIdentityRemoved: PlainDescriptor<{
  sub: SS58String;
  main: SS58String;
  deposit: bigint;
}> = cck3mveut33f44 as PlainDescriptor<{ sub: SS58String; main: SS58String; deposit: bigint }>;
const evt_Identity_SubIdentityRevoked: PlainDescriptor<{
  sub: SS58String;
  main: SS58String;
  deposit: bigint;
}> = cck3mveut33f44 as PlainDescriptor<{ sub: SS58String; main: SS58String; deposit: bigint }>;
const evt_Identity_AuthorityAdded: PlainDescriptor<{ authority: SS58String }> =
  c2rg5btjrsqec0 as PlainDescriptor<{ authority: SS58String }>;
const evt_Identity_AuthorityRemoved: PlainDescriptor<{ authority: SS58String }> =
  c2rg5btjrsqec0 as PlainDescriptor<{ authority: SS58String }>;
const evt_Identity_UsernameSet: PlainDescriptor<{ who: SS58String; username: Binary }> =
  cbdqerrooruuq9 as PlainDescriptor<{ who: SS58String; username: Binary }>;
const evt_Identity_UsernameQueued: PlainDescriptor<{
  who: SS58String;
  username: Binary;
  expiration: number;
}> = "8u2ba9jeiu6q0" as PlainDescriptor<{ who: SS58String; username: Binary; expiration: number }>;
const evt_Identity_PreapprovalExpired: PlainDescriptor<{ whose: SS58String }> =
  "7ieadb293k6b4" as PlainDescriptor<{ whose: SS58String }>;
const evt_Identity_PrimaryUsernameSet: PlainDescriptor<{ who: SS58String; username: Binary }> =
  cbdqerrooruuq9 as PlainDescriptor<{ who: SS58String; username: Binary }>;
const evt_Identity_DanglingUsernameRemoved: PlainDescriptor<{ who: SS58String; username: Binary }> =
  cbdqerrooruuq9 as PlainDescriptor<{ who: SS58String; username: Binary }>;
const evt_Society_Founded: PlainDescriptor<{ founder: SS58String }> =
  c8ircspqpe79k7 as PlainDescriptor<{ founder: SS58String }>;
const evt_Society_Bid: PlainDescriptor<{ candidate_id: SS58String; offer: bigint }> =
  "1vlt8l6nvnen6" as PlainDescriptor<{ candidate_id: SS58String; offer: bigint }>;
const evt_Society_Vouch: PlainDescriptor<{
  candidate_id: SS58String;
  offer: bigint;
  vouching: SS58String;
}> = "72vf0j9juc1df" as PlainDescriptor<{
  candidate_id: SS58String;
  offer: bigint;
  vouching: SS58String;
}>;
const evt_Society_AutoUnbid: PlainDescriptor<{ candidate: SS58String }> =
  c4b66js88p45m8 as PlainDescriptor<{ candidate: SS58String }>;
const evt_Society_Unbid: PlainDescriptor<{ candidate: SS58String }> =
  c4b66js88p45m8 as PlainDescriptor<{ candidate: SS58String }>;
const evt_Society_Unvouch: PlainDescriptor<{ candidate: SS58String }> =
  c4b66js88p45m8 as PlainDescriptor<{ candidate: SS58String }>;
const evt_Society_Inducted: PlainDescriptor<{
  primary: SS58String;
  candidates: Anonymize<Ia2lhg7l2hilo3>;
}> = "e0phfcvllr4od" as PlainDescriptor<{
  primary: SS58String;
  candidates: Anonymize<Ia2lhg7l2hilo3>;
}>;
const evt_Society_SuspendedMemberJudgement: PlainDescriptor<{ who: SS58String; judged: boolean }> =
  "476ep66p6ddg1" as PlainDescriptor<{ who: SS58String; judged: boolean }>;
const evt_Society_CandidateSuspended: PlainDescriptor<{ candidate: SS58String }> =
  c4b66js88p45m8 as PlainDescriptor<{ candidate: SS58String }>;
const evt_Society_MemberSuspended: PlainDescriptor<{ member: SS58String }> =
  ce3gphha4ejh40 as PlainDescriptor<{ member: SS58String }>;
const evt_Society_Challenged: PlainDescriptor<{ member: SS58String }> =
  ce3gphha4ejh40 as PlainDescriptor<{ member: SS58String }>;
const evt_Society_Vote: PlainDescriptor<{
  candidate: SS58String;
  voter: SS58String;
  vote: boolean;
}> = "f0btdsid69rrm" as PlainDescriptor<{
  candidate: SS58String;
  voter: SS58String;
  vote: boolean;
}>;
const evt_Society_DefenderVote: PlainDescriptor<{ voter: SS58String; vote: boolean }> =
  "ctusacdcqj61u" as PlainDescriptor<{ voter: SS58String; vote: boolean }>;
const evt_Society_NewParams: PlainDescriptor<{ params: Anonymize<I9hkvi4k3kh916> }> =
  "bsolu3c3q1lb4" as PlainDescriptor<{ params: Anonymize<I9hkvi4k3kh916> }>;
const evt_Society_Unfounded: PlainDescriptor<{ founder: SS58String }> =
  c8ircspqpe79k7 as PlainDescriptor<{ founder: SS58String }>;
const evt_Society_Deposit: PlainDescriptor<{ value: bigint }> = ce5v6njpckr05b as PlainDescriptor<{
  value: bigint;
}>;
const evt_Society_Elevated: PlainDescriptor<{ member: SS58String; rank: number }> =
  "b116djgo9u4uo" as PlainDescriptor<{ member: SS58String; rank: number }>;
const evt_Recovery_RecoveryCreated: PlainDescriptor<{ account: SS58String }> =
  ccbccs0ug47ilf as PlainDescriptor<{ account: SS58String }>;
const evt_Recovery_RecoveryInitiated: PlainDescriptor<{
  lost_account: SS58String;
  rescuer_account: SS58String;
}> = c9vkkue6cq74et as PlainDescriptor<{ lost_account: SS58String; rescuer_account: SS58String }>;
const evt_Recovery_RecoveryVouched: PlainDescriptor<{
  lost_account: SS58String;
  rescuer_account: SS58String;
  sender: SS58String;
}> = "bu56t5h1q49i4" as PlainDescriptor<{
  lost_account: SS58String;
  rescuer_account: SS58String;
  sender: SS58String;
}>;
const evt_Recovery_RecoveryClosed: PlainDescriptor<{
  lost_account: SS58String;
  rescuer_account: SS58String;
}> = c9vkkue6cq74et as PlainDescriptor<{ lost_account: SS58String; rescuer_account: SS58String }>;
const evt_Recovery_AccountRecovered: PlainDescriptor<{
  lost_account: SS58String;
  rescuer_account: SS58String;
}> = c9vkkue6cq74et as PlainDescriptor<{ lost_account: SS58String; rescuer_account: SS58String }>;
const evt_Recovery_RecoveryRemoved: PlainDescriptor<{ lost_account: SS58String }> =
  "1dmtl5t34b9g" as PlainDescriptor<{ lost_account: SS58String }>;
const evt_Vesting_VestingUpdated: PlainDescriptor<{ account: SS58String; unvested: bigint }> =
  "evr89968437gm" as PlainDescriptor<{ account: SS58String; unvested: bigint }>;
const evt_Vesting_VestingCompleted: PlainDescriptor<{ account: SS58String }> =
  ccbccs0ug47ilf as PlainDescriptor<{ account: SS58String }>;
const evt_Scheduler_Scheduled: PlainDescriptor<{ when: number; index: number }> =
  c5n4sebgkfr760 as PlainDescriptor<{ when: number; index: number }>;
const evt_Scheduler_Canceled: PlainDescriptor<{ when: number; index: number }> =
  c5n4sebgkfr760 as PlainDescriptor<{ when: number; index: number }>;
const evt_Scheduler_Dispatched: PlainDescriptor<{
  task: Anonymize<I5g2vv0ckl2m8b>;
  id: Anonymize<I17k3ujudqd5df>;
  result: Anonymize<Idtdr91jmq5g4i>;
}> = "dv8erd9m7jvse" as PlainDescriptor<{
  task: Anonymize<I5g2vv0ckl2m8b>;
  id: Anonymize<I17k3ujudqd5df>;
  result: Anonymize<Idtdr91jmq5g4i>;
}>;
const evt_Scheduler_RetrySet: PlainDescriptor<{
  task: Anonymize<I5g2vv0ckl2m8b>;
  id: Anonymize<I17k3ujudqd5df>;
  period: number;
  retries: number;
}> = "1fj2d8s3noh0v" as PlainDescriptor<{
  task: Anonymize<I5g2vv0ckl2m8b>;
  id: Anonymize<I17k3ujudqd5df>;
  period: number;
  retries: number;
}>;
const evt_Scheduler_RetryCancelled: PlainDescriptor<{
  task: Anonymize<I5g2vv0ckl2m8b>;
  id: Anonymize<I17k3ujudqd5df>;
}> = cbkv7dijodoblp as PlainDescriptor<{
  task: Anonymize<I5g2vv0ckl2m8b>;
  id: Anonymize<I17k3ujudqd5df>;
}>;
const evt_Scheduler_CallUnavailable: PlainDescriptor<{
  task: Anonymize<I5g2vv0ckl2m8b>;
  id: Anonymize<I17k3ujudqd5df>;
}> = cbkv7dijodoblp as PlainDescriptor<{
  task: Anonymize<I5g2vv0ckl2m8b>;
  id: Anonymize<I17k3ujudqd5df>;
}>;
const evt_Scheduler_PeriodicFailed: PlainDescriptor<{
  task: Anonymize<I5g2vv0ckl2m8b>;
  id: Anonymize<I17k3ujudqd5df>;
}> = cbkv7dijodoblp as PlainDescriptor<{
  task: Anonymize<I5g2vv0ckl2m8b>;
  id: Anonymize<I17k3ujudqd5df>;
}>;
const evt_Scheduler_RetryFailed: PlainDescriptor<{
  task: Anonymize<I5g2vv0ckl2m8b>;
  id: Anonymize<I17k3ujudqd5df>;
}> = cbkv7dijodoblp as PlainDescriptor<{
  task: Anonymize<I5g2vv0ckl2m8b>;
  id: Anonymize<I17k3ujudqd5df>;
}>;
const evt_Scheduler_PermanentlyOverweight: PlainDescriptor<{
  task: Anonymize<I5g2vv0ckl2m8b>;
  id: Anonymize<I17k3ujudqd5df>;
}> = cbkv7dijodoblp as PlainDescriptor<{
  task: Anonymize<I5g2vv0ckl2m8b>;
  id: Anonymize<I17k3ujudqd5df>;
}>;
const evt_Proxy_ProxyExecuted: PlainDescriptor<{ result: Anonymize<Idtdr91jmq5g4i> }> =
  ce5i8qqljk3tjb as PlainDescriptor<{ result: Anonymize<Idtdr91jmq5g4i> }>;
const evt_Proxy_PureCreated: PlainDescriptor<{
  pure: SS58String;
  who: SS58String;
  proxy_type: Anonymize<I90b4nim1kup1v>;
  disambiguation_index: number;
}> = "2bf4tmdcees31" as PlainDescriptor<{
  pure: SS58String;
  who: SS58String;
  proxy_type: Anonymize<I90b4nim1kup1v>;
  disambiguation_index: number;
}>;
const evt_Proxy_Announced: PlainDescriptor<{
  real: SS58String;
  proxy: SS58String;
  call_hash: Binary;
}> = "dbjbboh0q507r" as PlainDescriptor<{ real: SS58String; proxy: SS58String; call_hash: Binary }>;
const evt_Proxy_ProxyAdded: PlainDescriptor<{
  delegator: SS58String;
  delegatee: SS58String;
  proxy_type: Anonymize<I90b4nim1kup1v>;
  delay: number;
}> = c3ce0ol160n343 as PlainDescriptor<{
  delegator: SS58String;
  delegatee: SS58String;
  proxy_type: Anonymize<I90b4nim1kup1v>;
  delay: number;
}>;
const evt_Proxy_ProxyRemoved: PlainDescriptor<{
  delegator: SS58String;
  delegatee: SS58String;
  proxy_type: Anonymize<I90b4nim1kup1v>;
  delay: number;
}> = c3ce0ol160n343 as PlainDescriptor<{
  delegator: SS58String;
  delegatee: SS58String;
  proxy_type: Anonymize<I90b4nim1kup1v>;
  delay: number;
}>;
const evt_Multisig_NewMultisig: PlainDescriptor<{
  approving: SS58String;
  multisig: SS58String;
  call_hash: Binary;
}> = "bvv58de7m7rsi" as PlainDescriptor<{
  approving: SS58String;
  multisig: SS58String;
  call_hash: Binary;
}>;
const evt_Multisig_MultisigApproval: PlainDescriptor<{
  approving: SS58String;
  timepoint: Anonymize<Itvprrpb0nm3o>;
  multisig: SS58String;
  call_hash: Binary;
}> = "4uo2dg1jvbdtg" as PlainDescriptor<{
  approving: SS58String;
  timepoint: Anonymize<Itvprrpb0nm3o>;
  multisig: SS58String;
  call_hash: Binary;
}>;
const evt_Multisig_MultisigExecuted: PlainDescriptor<{
  approving: SS58String;
  timepoint: Anonymize<Itvprrpb0nm3o>;
  multisig: SS58String;
  call_hash: Binary;
  result: Anonymize<Idtdr91jmq5g4i>;
}> = "fbo6gts4g8u33" as PlainDescriptor<{
  approving: SS58String;
  timepoint: Anonymize<Itvprrpb0nm3o>;
  multisig: SS58String;
  call_hash: Binary;
  result: Anonymize<Idtdr91jmq5g4i>;
}>;
const evt_Multisig_MultisigCancelled: PlainDescriptor<{
  cancelling: SS58String;
  timepoint: Anonymize<Itvprrpb0nm3o>;
  multisig: SS58String;
  call_hash: Binary;
}> = "82jp3a00f0f8k" as PlainDescriptor<{
  cancelling: SS58String;
  timepoint: Anonymize<Itvprrpb0nm3o>;
  multisig: SS58String;
  call_hash: Binary;
}>;
const evt_Preimage_Noted: PlainDescriptor<{ hash: Binary }> = cd9d48vaes3c53 as PlainDescriptor<{
  hash: Binary;
}>;
const evt_Preimage_Requested: PlainDescriptor<{ hash: Binary }> =
  cd9d48vaes3c53 as PlainDescriptor<{ hash: Binary }>;
const evt_Preimage_Cleared: PlainDescriptor<{ hash: Binary }> = cd9d48vaes3c53 as PlainDescriptor<{
  hash: Binary;
}>;
const evt_AssetRate_AssetRateCreated: PlainDescriptor<{
  asset_kind: PolkadotRuntimeCommonImplsVersionedLocatableAsset;
  rate: bigint;
}> = c4digl36cac7si as PlainDescriptor<{
  asset_kind: PolkadotRuntimeCommonImplsVersionedLocatableAsset;
  rate: bigint;
}>;
const evt_AssetRate_AssetRateRemoved: PlainDescriptor<{
  asset_kind: PolkadotRuntimeCommonImplsVersionedLocatableAsset;
}> = cdi8jsg8hkprub as PlainDescriptor<{
  asset_kind: PolkadotRuntimeCommonImplsVersionedLocatableAsset;
}>;
const evt_AssetRate_AssetRateUpdated: PlainDescriptor<{
  asset_kind: PolkadotRuntimeCommonImplsVersionedLocatableAsset;
  old: bigint;
  new: bigint;
}> = "asmn1g877fblj" as PlainDescriptor<{
  asset_kind: PolkadotRuntimeCommonImplsVersionedLocatableAsset;
  old: bigint;
  new: bigint;
}>;
const evt_Bounties_BountyProposed: PlainDescriptor<{ index: number }> =
  c666bl2fqjkejo as PlainDescriptor<{ index: number }>;
const evt_Bounties_BountyRejected: PlainDescriptor<{ index: number; bond: bigint }> =
  "d9idaj83175f9" as PlainDescriptor<{ index: number; bond: bigint }>;
const evt_Bounties_BountyBecameActive: PlainDescriptor<{ index: number }> =
  c666bl2fqjkejo as PlainDescriptor<{ index: number }>;
const evt_Bounties_BountyAwarded: PlainDescriptor<{ index: number; beneficiary: SS58String }> =
  "e1semicfuv5uu" as PlainDescriptor<{ index: number; beneficiary: SS58String }>;
const evt_Bounties_BountyClaimed: PlainDescriptor<{
  index: number;
  payout: bigint;
  beneficiary: SS58String;
}> = "f25fjs9o37co1" as PlainDescriptor<{ index: number; payout: bigint; beneficiary: SS58String }>;
const evt_Bounties_BountyCanceled: PlainDescriptor<{ index: number }> =
  c666bl2fqjkejo as PlainDescriptor<{ index: number }>;
const evt_Bounties_BountyExtended: PlainDescriptor<{ index: number }> =
  c666bl2fqjkejo as PlainDescriptor<{ index: number }>;
const evt_Bounties_BountyApproved: PlainDescriptor<{ index: number }> =
  c666bl2fqjkejo as PlainDescriptor<{ index: number }>;
const evt_Bounties_CuratorProposed: PlainDescriptor<{ bounty_id: number; curator: SS58String }> =
  c70sc1pdo8vtos as PlainDescriptor<{ bounty_id: number; curator: SS58String }>;
const evt_Bounties_CuratorUnassigned: PlainDescriptor<{ bounty_id: number }> =
  ca9p5bg6p18r0i as PlainDescriptor<{ bounty_id: number }>;
const evt_Bounties_CuratorAccepted: PlainDescriptor<{ bounty_id: number; curator: SS58String }> =
  c70sc1pdo8vtos as PlainDescriptor<{ bounty_id: number; curator: SS58String }>;
const evt_ChildBounties_Added: PlainDescriptor<{ index: number; child_index: number }> =
  c60p8l86a8cm59 as PlainDescriptor<{ index: number; child_index: number }>;
const evt_ChildBounties_Awarded: PlainDescriptor<{
  index: number;
  child_index: number;
  beneficiary: SS58String;
}> = "3m3sk2lgcabvp" as PlainDescriptor<{
  index: number;
  child_index: number;
  beneficiary: SS58String;
}>;
const evt_ChildBounties_Claimed: PlainDescriptor<{
  index: number;
  child_index: number;
  payout: bigint;
  beneficiary: SS58String;
}> = "5pf572duh4oeg" as PlainDescriptor<{
  index: number;
  child_index: number;
  payout: bigint;
  beneficiary: SS58String;
}>;
const evt_ChildBounties_Canceled: PlainDescriptor<{ index: number; child_index: number }> =
  c60p8l86a8cm59 as PlainDescriptor<{ index: number; child_index: number }>;
const evt_Nis_BidPlaced: PlainDescriptor<{ who: SS58String; amount: bigint; duration: number }> =
  ce2vmgag6qiue4 as PlainDescriptor<{ who: SS58String; amount: bigint; duration: number }>;
const evt_Nis_BidRetracted: PlainDescriptor<{ who: SS58String; amount: bigint; duration: number }> =
  ce2vmgag6qiue4 as PlainDescriptor<{ who: SS58String; amount: bigint; duration: number }>;
const evt_Nis_BidDropped: PlainDescriptor<{ who: SS58String; amount: bigint; duration: number }> =
  ce2vmgag6qiue4 as PlainDescriptor<{ who: SS58String; amount: bigint; duration: number }>;
const evt_Nis_Issued: PlainDescriptor<{
  index: number;
  expiry: number;
  who: SS58String;
  proportion: bigint;
  amount: bigint;
}> = "1kpuet4lkcdpf" as PlainDescriptor<{
  index: number;
  expiry: number;
  who: SS58String;
  proportion: bigint;
  amount: bigint;
}>;
const evt_Nis_Thawed: PlainDescriptor<{
  index: number;
  who: SS58String;
  proportion: bigint;
  amount: bigint;
  dropped: boolean;
}> = "5tr1gp2t9i6oh" as PlainDescriptor<{
  index: number;
  who: SS58String;
  proportion: bigint;
  amount: bigint;
  dropped: boolean;
}>;
const evt_Nis_Funded: PlainDescriptor<{ deficit: bigint }> = "7ls1r4u61csif" as PlainDescriptor<{
  deficit: bigint;
}>;
const evt_Nis_Transferred: PlainDescriptor<{ from: SS58String; to: SS58String; index: number }> =
  "3gjgtso69eopr" as PlainDescriptor<{ from: SS58String; to: SS58String; index: number }>;
const evt_NisCounterpartBalances_Endowed: PlainDescriptor<{
  account: SS58String;
  free_balance: bigint;
}> = ccv68aq8841478 as PlainDescriptor<{ account: SS58String; free_balance: bigint }>;
const evt_NisCounterpartBalances_DustLost: PlainDescriptor<{
  account: SS58String;
  amount: bigint;
}> = cc262ibdoec56a as PlainDescriptor<{ account: SS58String; amount: bigint }>;
const evt_NisCounterpartBalances_Transfer: PlainDescriptor<{
  from: SS58String;
  to: SS58String;
  amount: bigint;
}> = cflcfm9b6nlmdd as PlainDescriptor<{ from: SS58String; to: SS58String; amount: bigint }>;
const evt_NisCounterpartBalances_BalanceSet: PlainDescriptor<{ who: SS58String; free: bigint }> =
  cjrsf4mnp3eka as PlainDescriptor<{ who: SS58String; free: bigint }>;
const evt_NisCounterpartBalances_Reserved: PlainDescriptor<{ who: SS58String; amount: bigint }> =
  cd5fm4p8lj5qgi as PlainDescriptor<{ who: SS58String; amount: bigint }>;
const evt_NisCounterpartBalances_Unreserved: PlainDescriptor<{ who: SS58String; amount: bigint }> =
  cd5fm4p8lj5qgi as PlainDescriptor<{ who: SS58String; amount: bigint }>;
const evt_NisCounterpartBalances_ReserveRepatriated: PlainDescriptor<{
  from: SS58String;
  to: SS58String;
  amount: bigint;
  destination_status: BalanceStatus;
}> = cdm5rqp3duosod as PlainDescriptor<{
  from: SS58String;
  to: SS58String;
  amount: bigint;
  destination_status: BalanceStatus;
}>;
const evt_NisCounterpartBalances_Deposit: PlainDescriptor<{ who: SS58String; amount: bigint }> =
  cd5fm4p8lj5qgi as PlainDescriptor<{ who: SS58String; amount: bigint }>;
const evt_NisCounterpartBalances_Withdraw: PlainDescriptor<{ who: SS58String; amount: bigint }> =
  cd5fm4p8lj5qgi as PlainDescriptor<{ who: SS58String; amount: bigint }>;
const evt_NisCounterpartBalances_Slashed: PlainDescriptor<{ who: SS58String; amount: bigint }> =
  cd5fm4p8lj5qgi as PlainDescriptor<{ who: SS58String; amount: bigint }>;
const evt_NisCounterpartBalances_Minted: PlainDescriptor<{ who: SS58String; amount: bigint }> =
  cd5fm4p8lj5qgi as PlainDescriptor<{ who: SS58String; amount: bigint }>;
const evt_NisCounterpartBalances_Burned: PlainDescriptor<{ who: SS58String; amount: bigint }> =
  cd5fm4p8lj5qgi as PlainDescriptor<{ who: SS58String; amount: bigint }>;
const evt_NisCounterpartBalances_Suspended: PlainDescriptor<{ who: SS58String; amount: bigint }> =
  cd5fm4p8lj5qgi as PlainDescriptor<{ who: SS58String; amount: bigint }>;
const evt_NisCounterpartBalances_Restored: PlainDescriptor<{ who: SS58String; amount: bigint }> =
  cd5fm4p8lj5qgi as PlainDescriptor<{ who: SS58String; amount: bigint }>;
const evt_NisCounterpartBalances_Upgraded: PlainDescriptor<{ who: SS58String }> =
  c4cbvqmqadhrea as PlainDescriptor<{ who: SS58String }>;
const evt_NisCounterpartBalances_Issued: PlainDescriptor<{ amount: bigint }> =
  c3qt1hgg4djhgb as PlainDescriptor<{ amount: bigint }>;
const evt_NisCounterpartBalances_Rescinded: PlainDescriptor<{ amount: bigint }> =
  c3qt1hgg4djhgb as PlainDescriptor<{ amount: bigint }>;
const evt_NisCounterpartBalances_Locked: PlainDescriptor<{ who: SS58String; amount: bigint }> =
  cd5fm4p8lj5qgi as PlainDescriptor<{ who: SS58String; amount: bigint }>;
const evt_NisCounterpartBalances_Unlocked: PlainDescriptor<{ who: SS58String; amount: bigint }> =
  cd5fm4p8lj5qgi as PlainDescriptor<{ who: SS58String; amount: bigint }>;
const evt_NisCounterpartBalances_Frozen: PlainDescriptor<{ who: SS58String; amount: bigint }> =
  cd5fm4p8lj5qgi as PlainDescriptor<{ who: SS58String; amount: bigint }>;
const evt_NisCounterpartBalances_Thawed: PlainDescriptor<{ who: SS58String; amount: bigint }> =
  cd5fm4p8lj5qgi as PlainDescriptor<{ who: SS58String; amount: bigint }>;
const evt_NisCounterpartBalances_TotalIssuanceForced: PlainDescriptor<{
  old: bigint;
  new: bigint;
}> = c4fooe9dun9o0t as PlainDescriptor<{ old: bigint; new: bigint }>;
const evt_ParaInclusion_CandidateBacked: PlainDescriptor<
  [Anonymize<I4vjld3472quct>, Binary, number, number]
> = ceno5vn1m65ng2 as PlainDescriptor<[Anonymize<I4vjld3472quct>, Binary, number, number]>;
const evt_ParaInclusion_CandidateIncluded: PlainDescriptor<
  [Anonymize<I4vjld3472quct>, Binary, number, number]
> = ceno5vn1m65ng2 as PlainDescriptor<[Anonymize<I4vjld3472quct>, Binary, number, number]>;
const evt_ParaInclusion_CandidateTimedOut: PlainDescriptor<
  [Anonymize<I4vjld3472quct>, Binary, number]
> = "evbvtucck5gnq" as PlainDescriptor<[Anonymize<I4vjld3472quct>, Binary, number]>;
const evt_ParaInclusion_UpwardMessagesReceived: PlainDescriptor<{ from: number; count: number }> =
  "c8i89mfkmn3n7" as PlainDescriptor<{ from: number; count: number }>;
const evt_Paras_CurrentCodeUpdated: PlainDescriptor<number> =
  c34ha3jf7jm288 as PlainDescriptor<number>;
const evt_Paras_CurrentHeadUpdated: PlainDescriptor<number> =
  c34ha3jf7jm288 as PlainDescriptor<number>;
const evt_Paras_CodeUpgradeScheduled: PlainDescriptor<number> =
  c34ha3jf7jm288 as PlainDescriptor<number>;
const evt_Paras_NewHeadNoted: PlainDescriptor<number> = c34ha3jf7jm288 as PlainDescriptor<number>;
const evt_Paras_ActionQueued: PlainDescriptor<[number, number]> = c5g2vv0ckl2m8b as PlainDescriptor<
  [number, number]
>;
const evt_Paras_PvfCheckStarted: PlainDescriptor<[Binary, number]> =
  c64gm4hrq7urum as PlainDescriptor<[Binary, number]>;
const evt_Paras_PvfCheckAccepted: PlainDescriptor<[Binary, number]> =
  c64gm4hrq7urum as PlainDescriptor<[Binary, number]>;
const evt_Paras_PvfCheckRejected: PlainDescriptor<[Binary, number]> =
  c64gm4hrq7urum as PlainDescriptor<[Binary, number]>;
const evt_Hrmp_OpenChannelRequested: PlainDescriptor<{
  sender: number;
  recipient: number;
  proposed_max_capacity: number;
  proposed_max_message_size: number;
}> = cd2bej717ckub0 as PlainDescriptor<{
  sender: number;
  recipient: number;
  proposed_max_capacity: number;
  proposed_max_message_size: number;
}>;
const evt_Hrmp_OpenChannelCanceled: PlainDescriptor<{
  by_parachain: number;
  channel_id: Anonymize<I50mrcbubp554e>;
}> = c545vo2e86o5i4 as PlainDescriptor<{
  by_parachain: number;
  channel_id: Anonymize<I50mrcbubp554e>;
}>;
const evt_Hrmp_OpenChannelAccepted: PlainDescriptor<{ sender: number; recipient: number }> =
  c50mrcbubp554e as PlainDescriptor<{ sender: number; recipient: number }>;
const evt_Hrmp_ChannelClosed: PlainDescriptor<{
  by_parachain: number;
  channel_id: Anonymize<I50mrcbubp554e>;
}> = c545vo2e86o5i4 as PlainDescriptor<{
  by_parachain: number;
  channel_id: Anonymize<I50mrcbubp554e>;
}>;
const evt_Hrmp_HrmpChannelForceOpened: PlainDescriptor<{
  sender: number;
  recipient: number;
  proposed_max_capacity: number;
  proposed_max_message_size: number;
}> = cd2bej717ckub0 as PlainDescriptor<{
  sender: number;
  recipient: number;
  proposed_max_capacity: number;
  proposed_max_message_size: number;
}>;
const evt_Hrmp_HrmpSystemChannelOpened: PlainDescriptor<{
  sender: number;
  recipient: number;
  proposed_max_capacity: number;
  proposed_max_message_size: number;
}> = cd2bej717ckub0 as PlainDescriptor<{
  sender: number;
  recipient: number;
  proposed_max_capacity: number;
  proposed_max_message_size: number;
}>;
const evt_Hrmp_OpenChannelDepositsUpdated: PlainDescriptor<{ sender: number; recipient: number }> =
  c50mrcbubp554e as PlainDescriptor<{ sender: number; recipient: number }>;
const evt_ParasDisputes_DisputeInitiated: PlainDescriptor<
  [Binary, ParachainsDisputesDisputeLocation]
> = "3o099fcusuh31" as PlainDescriptor<[Binary, ParachainsDisputesDisputeLocation]>;
const evt_ParasDisputes_DisputeConcluded: PlainDescriptor<
  [Binary, ParachainsDisputesDisputeResult]
> = "fr2e7vm3bun8k" as PlainDescriptor<[Binary, ParachainsDisputesDisputeResult]>;
const evt_ParasDisputes_Revert: PlainDescriptor<number> = c34ha3jf7jm288 as PlainDescriptor<number>;
const evt_MessageQueue_ProcessingFailed: PlainDescriptor<{
  id: Binary;
  origin: ParachainsInclusionAggregateMessageOrigin;
  error: ProcessMessageError;
}> = "6ian27okrbc15" as PlainDescriptor<{
  id: Binary;
  origin: ParachainsInclusionAggregateMessageOrigin;
  error: ProcessMessageError;
}>;
const evt_MessageQueue_Processed: PlainDescriptor<{
  id: Binary;
  origin: ParachainsInclusionAggregateMessageOrigin;
  weight_used: Anonymize<I4q39t5hn830vp>;
  success: boolean;
}> = "74b8cu68dfbfr" as PlainDescriptor<{
  id: Binary;
  origin: ParachainsInclusionAggregateMessageOrigin;
  weight_used: Anonymize<I4q39t5hn830vp>;
  success: boolean;
}>;
const evt_MessageQueue_OverweightEnqueued: PlainDescriptor<{
  id: Binary;
  origin: ParachainsInclusionAggregateMessageOrigin;
  page_index: number;
  message_index: number;
}> = "acc6dee8ffsrh" as PlainDescriptor<{
  id: Binary;
  origin: ParachainsInclusionAggregateMessageOrigin;
  page_index: number;
  message_index: number;
}>;
const evt_MessageQueue_PageReaped: PlainDescriptor<{
  origin: ParachainsInclusionAggregateMessageOrigin;
  index: number;
}> = "elsom2b1kkdm6" as PlainDescriptor<{
  origin: ParachainsInclusionAggregateMessageOrigin;
  index: number;
}>;
const evt_OnDemandAssignmentProvider_OnDemandOrderPlaced: PlainDescriptor<{
  para_id: number;
  spot_price: bigint;
}> = "89dbspj67f63r" as PlainDescriptor<{ para_id: number; spot_price: bigint }>;
const evt_OnDemandAssignmentProvider_SpotTrafficSet: PlainDescriptor<{ traffic: bigint }> =
  "a7efn5eqruldn" as PlainDescriptor<{ traffic: bigint }>;
const evt_Registrar_Registered: PlainDescriptor<{ para_id: number; manager: SS58String }> =
  "bs22tt76qp5bi" as PlainDescriptor<{ para_id: number; manager: SS58String }>;
const evt_Registrar_Deregistered: PlainDescriptor<{ para_id: number }> =
  c37r4bdai8o9mp as PlainDescriptor<{ para_id: number }>;
const evt_Registrar_Reserved: PlainDescriptor<{ para_id: number; who: SS58String }> =
  "dn2ghub1o4i40" as PlainDescriptor<{ para_id: number; who: SS58String }>;
const evt_Registrar_Swapped: PlainDescriptor<{ para_id: number; other_id: number }> =
  "48u78djt89dod" as PlainDescriptor<{ para_id: number; other_id: number }>;
const evt_Slots_NewLeasePeriod: PlainDescriptor<{ lease_period: number }> =
  "b85m5kfbepu2t" as PlainDescriptor<{ lease_period: number }>;
const evt_Slots_Leased: PlainDescriptor<{
  para_id: number;
  leaser: SS58String;
  period_begin: number;
  period_count: number;
  extra_reserved: bigint;
  total_amount: bigint;
}> = "daml5bdhsfcsl" as PlainDescriptor<{
  para_id: number;
  leaser: SS58String;
  period_begin: number;
  period_count: number;
  extra_reserved: bigint;
  total_amount: bigint;
}>;
const evt_Auctions_AuctionStarted: PlainDescriptor<{
  auction_index: number;
  lease_period: number;
  ending: number;
}> = "eec0cu336gteb" as PlainDescriptor<{
  auction_index: number;
  lease_period: number;
  ending: number;
}>;
const evt_Auctions_AuctionClosed: PlainDescriptor<{ auction_index: number }> =
  "815d5k4ij85nv" as PlainDescriptor<{ auction_index: number }>;
const evt_Auctions_Reserved: PlainDescriptor<{
  bidder: SS58String;
  extra_reserved: bigint;
  total_amount: bigint;
}> = "fi98fgi9o46v7" as PlainDescriptor<{
  bidder: SS58String;
  extra_reserved: bigint;
  total_amount: bigint;
}>;
const evt_Auctions_Unreserved: PlainDescriptor<{ bidder: SS58String; amount: bigint }> =
  "c0oj9tok33uap" as PlainDescriptor<{ bidder: SS58String; amount: bigint }>;
const evt_Auctions_ReserveConfiscated: PlainDescriptor<{
  para_id: number;
  leaser: SS58String;
  amount: bigint;
}> = "3tdutpfjuk32j" as PlainDescriptor<{ para_id: number; leaser: SS58String; amount: bigint }>;
const evt_Auctions_BidAccepted: PlainDescriptor<{
  bidder: SS58String;
  para_id: number;
  amount: bigint;
  first_slot: number;
  last_slot: number;
}> = "1esdujrkdacpb" as PlainDescriptor<{
  bidder: SS58String;
  para_id: number;
  amount: bigint;
  first_slot: number;
  last_slot: number;
}>;
const evt_Auctions_WinningOffset: PlainDescriptor<{ auction_index: number; block_number: number }> =
  "9g1d820jf9m2s" as PlainDescriptor<{ auction_index: number; block_number: number }>;
const evt_Crowdloan_Created: PlainDescriptor<{ para_id: number }> =
  c37r4bdai8o9mp as PlainDescriptor<{ para_id: number }>;
const evt_Crowdloan_Contributed: PlainDescriptor<{
  who: SS58String;
  fund_index: number;
  amount: bigint;
}> = c8ve4g3egaln6a as PlainDescriptor<{ who: SS58String; fund_index: number; amount: bigint }>;
const evt_Crowdloan_Withdrew: PlainDescriptor<{
  who: SS58String;
  fund_index: number;
  amount: bigint;
}> = c8ve4g3egaln6a as PlainDescriptor<{ who: SS58String; fund_index: number; amount: bigint }>;
const evt_Crowdloan_PartiallyRefunded: PlainDescriptor<{ para_id: number }> =
  c37r4bdai8o9mp as PlainDescriptor<{ para_id: number }>;
const evt_Crowdloan_AllRefunded: PlainDescriptor<{ para_id: number }> =
  c37r4bdai8o9mp as PlainDescriptor<{ para_id: number }>;
const evt_Crowdloan_Dissolved: PlainDescriptor<{ para_id: number }> =
  c37r4bdai8o9mp as PlainDescriptor<{ para_id: number }>;
const evt_Crowdloan_HandleBidResult: PlainDescriptor<{
  para_id: number;
  result: Anonymize<Idtdr91jmq5g4i>;
}> = "f9e3ujpsfl4g7" as PlainDescriptor<{ para_id: number; result: Anonymize<Idtdr91jmq5g4i> }>;
const evt_Crowdloan_Edited: PlainDescriptor<{ para_id: number }> =
  c37r4bdai8o9mp as PlainDescriptor<{ para_id: number }>;
const evt_Crowdloan_MemoUpdated: PlainDescriptor<{
  who: SS58String;
  para_id: number;
  memo: Binary;
}> = "f4hvqaeoqq5us" as PlainDescriptor<{ who: SS58String; para_id: number; memo: Binary }>;
const evt_Crowdloan_AddedToNewRaise: PlainDescriptor<{ para_id: number }> =
  c37r4bdai8o9mp as PlainDescriptor<{ para_id: number }>;
const evt_Coretime_RevenueInfoRequested: PlainDescriptor<{ when: number }> =
  "btsa3docbr9el" as PlainDescriptor<{ when: number }>;
const evt_Coretime_CoreAssigned: PlainDescriptor<{ core: number }> =
  "aiqv5prlisjkg" as PlainDescriptor<{ core: number }>;
const evt_XcmPallet_Attempted: PlainDescriptor<{ outcome: XcmV4TraitsOutcome }> =
  "39j4hmqmtbefm" as PlainDescriptor<{ outcome: XcmV4TraitsOutcome }>;
const evt_XcmPallet_Sent: PlainDescriptor<{
  origin: Anonymize<Ieq428hdmibf6m>;
  destination: Anonymize<Ieq428hdmibf6m>;
  message: Anonymize<I9uem0hkif9mb7>;
  message_id: Binary;
}> = "3gb27uk4ul5am" as PlainDescriptor<{
  origin: Anonymize<Ieq428hdmibf6m>;
  destination: Anonymize<Ieq428hdmibf6m>;
  message: Anonymize<I9uem0hkif9mb7>;
  message_id: Binary;
}>;
const evt_XcmPallet_UnexpectedResponse: PlainDescriptor<{
  origin: Anonymize<Ieq428hdmibf6m>;
  query_id: bigint;
}> = cc8gqph940s142 as PlainDescriptor<{ origin: Anonymize<Ieq428hdmibf6m>; query_id: bigint }>;
const evt_XcmPallet_ResponseReady: PlainDescriptor<{ query_id: bigint; response: XcmV4Response }> =
  "9k3jmtgmoiqqf" as PlainDescriptor<{ query_id: bigint; response: XcmV4Response }>;
const evt_XcmPallet_Notified: PlainDescriptor<{
  query_id: bigint;
  pallet_index: number;
  call_index: number;
}> = c2uqmls7kcdnii as PlainDescriptor<{
  query_id: bigint;
  pallet_index: number;
  call_index: number;
}>;
const evt_XcmPallet_NotifyOverweight: PlainDescriptor<{
  query_id: bigint;
  pallet_index: number;
  call_index: number;
  actual_weight: Anonymize<I4q39t5hn830vp>;
  max_budgeted_weight: Anonymize<I4q39t5hn830vp>;
}> = "dg69klialbkb8" as PlainDescriptor<{
  query_id: bigint;
  pallet_index: number;
  call_index: number;
  actual_weight: Anonymize<I4q39t5hn830vp>;
  max_budgeted_weight: Anonymize<I4q39t5hn830vp>;
}>;
const evt_XcmPallet_NotifyDispatchError: PlainDescriptor<{
  query_id: bigint;
  pallet_index: number;
  call_index: number;
}> = c2uqmls7kcdnii as PlainDescriptor<{
  query_id: bigint;
  pallet_index: number;
  call_index: number;
}>;
const evt_XcmPallet_NotifyDecodeFailed: PlainDescriptor<{
  query_id: bigint;
  pallet_index: number;
  call_index: number;
}> = c2uqmls7kcdnii as PlainDescriptor<{
  query_id: bigint;
  pallet_index: number;
  call_index: number;
}>;
const evt_XcmPallet_InvalidResponder: PlainDescriptor<{
  origin: Anonymize<Ieq428hdmibf6m>;
  query_id: bigint;
  expected_location: Anonymize<Iehbh3m350i09b>;
}> = "24et57o2f2i2m" as PlainDescriptor<{
  origin: Anonymize<Ieq428hdmibf6m>;
  query_id: bigint;
  expected_location: Anonymize<Iehbh3m350i09b>;
}>;
const evt_XcmPallet_InvalidResponderVersion: PlainDescriptor<{
  origin: Anonymize<Ieq428hdmibf6m>;
  query_id: bigint;
}> = cc8gqph940s142 as PlainDescriptor<{ origin: Anonymize<Ieq428hdmibf6m>; query_id: bigint }>;
const evt_XcmPallet_ResponseTaken: PlainDescriptor<{ query_id: bigint }> =
  "30pg328m00nr3" as PlainDescriptor<{ query_id: bigint }>;
const evt_XcmPallet_AssetsTrapped: PlainDescriptor<{
  hash: Binary;
  origin: Anonymize<Ieq428hdmibf6m>;
  assets: XcmVersionedAssets;
}> = c20naetn2lovo9 as PlainDescriptor<{
  hash: Binary;
  origin: Anonymize<Ieq428hdmibf6m>;
  assets: XcmVersionedAssets;
}>;
const evt_XcmPallet_VersionChangeNotified: PlainDescriptor<{
  destination: Anonymize<Ieq428hdmibf6m>;
  result: number;
  cost: Anonymize<Ia04r3ent3ib9l>;
  message_id: Binary;
}> = "4ks28k7d44l1p" as PlainDescriptor<{
  destination: Anonymize<Ieq428hdmibf6m>;
  result: number;
  cost: Anonymize<Ia04r3ent3ib9l>;
  message_id: Binary;
}>;
const evt_XcmPallet_SupportedVersionChanged: PlainDescriptor<{
  location: Anonymize<Ieq428hdmibf6m>;
  version: number;
}> = cergfrmj3c9dor as PlainDescriptor<{ location: Anonymize<Ieq428hdmibf6m>; version: number }>;
const evt_XcmPallet_NotifyTargetSendFail: PlainDescriptor<{
  location: Anonymize<Ieq428hdmibf6m>;
  query_id: bigint;
  error: XcmV3TraitsError;
}> = "9jji4pfgcqc9b" as PlainDescriptor<{
  location: Anonymize<Ieq428hdmibf6m>;
  query_id: bigint;
  error: XcmV3TraitsError;
}>;
const evt_XcmPallet_NotifyTargetMigrationFail: PlainDescriptor<{
  location: XcmVersionedLocation;
  query_id: bigint;
}> = "6djqr04u42c2g" as PlainDescriptor<{ location: XcmVersionedLocation; query_id: bigint }>;
const evt_XcmPallet_InvalidQuerierVersion: PlainDescriptor<{
  origin: Anonymize<Ieq428hdmibf6m>;
  query_id: bigint;
}> = cc8gqph940s142 as PlainDescriptor<{ origin: Anonymize<Ieq428hdmibf6m>; query_id: bigint }>;
const evt_XcmPallet_InvalidQuerier: PlainDescriptor<{
  origin: Anonymize<Ieq428hdmibf6m>;
  query_id: bigint;
  expected_querier: Anonymize<Ieq428hdmibf6m>;
  maybe_actual_querier: Anonymize<Iehbh3m350i09b>;
}> = "6ctg8b4bcjdek" as PlainDescriptor<{
  origin: Anonymize<Ieq428hdmibf6m>;
  query_id: bigint;
  expected_querier: Anonymize<Ieq428hdmibf6m>;
  maybe_actual_querier: Anonymize<Iehbh3m350i09b>;
}>;
const evt_XcmPallet_VersionNotifyStarted: PlainDescriptor<{
  destination: Anonymize<Ieq428hdmibf6m>;
  cost: Anonymize<Ia04r3ent3ib9l>;
  message_id: Binary;
}> = c4fi6oaheho56l as PlainDescriptor<{
  destination: Anonymize<Ieq428hdmibf6m>;
  cost: Anonymize<Ia04r3ent3ib9l>;
  message_id: Binary;
}>;
const evt_XcmPallet_VersionNotifyRequested: PlainDescriptor<{
  destination: Anonymize<Ieq428hdmibf6m>;
  cost: Anonymize<Ia04r3ent3ib9l>;
  message_id: Binary;
}> = c4fi6oaheho56l as PlainDescriptor<{
  destination: Anonymize<Ieq428hdmibf6m>;
  cost: Anonymize<Ia04r3ent3ib9l>;
  message_id: Binary;
}>;
const evt_XcmPallet_VersionNotifyUnrequested: PlainDescriptor<{
  destination: Anonymize<Ieq428hdmibf6m>;
  cost: Anonymize<Ia04r3ent3ib9l>;
  message_id: Binary;
}> = c4fi6oaheho56l as PlainDescriptor<{
  destination: Anonymize<Ieq428hdmibf6m>;
  cost: Anonymize<Ia04r3ent3ib9l>;
  message_id: Binary;
}>;
const evt_XcmPallet_FeesPaid: PlainDescriptor<{
  paying: Anonymize<Ieq428hdmibf6m>;
  fees: Anonymize<Ia04r3ent3ib9l>;
}> = "89pfbi9d7lm5p" as PlainDescriptor<{
  paying: Anonymize<Ieq428hdmibf6m>;
  fees: Anonymize<Ia04r3ent3ib9l>;
}>;
const evt_XcmPallet_AssetsClaimed: PlainDescriptor<{
  hash: Binary;
  origin: Anonymize<Ieq428hdmibf6m>;
  assets: XcmVersionedAssets;
}> = c20naetn2lovo9 as PlainDescriptor<{
  hash: Binary;
  origin: Anonymize<Ieq428hdmibf6m>;
  assets: XcmVersionedAssets;
}>;
const evt_XcmPallet_VersionMigrationFinished: PlainDescriptor<{ version: number }> =
  "6s1nbislhk619" as PlainDescriptor<{ version: number }>;
const evt_IdentityMigrator_IdentityReaped: PlainDescriptor<{ who: SS58String }> =
  c4cbvqmqadhrea as PlainDescriptor<{ who: SS58String }>;
const evt_IdentityMigrator_DepositUpdated: PlainDescriptor<{
  who: SS58String;
  identity: bigint;
  subs: bigint;
}> = "4i3u9uui7ktsd" as PlainDescriptor<{ who: SS58String; identity: bigint; subs: bigint }>;
const evt_AssignedSlots_PermanentSlotAssigned: PlainDescriptor<number> =
  c34ha3jf7jm288 as PlainDescriptor<number>;
const evt_AssignedSlots_TemporarySlotAssigned: PlainDescriptor<number> =
  c34ha3jf7jm288 as PlainDescriptor<number>;
const evt_AssignedSlots_MaxPermanentSlotsChanged: PlainDescriptor<{ slots: number }> =
  c9d5h5irbki7mm as PlainDescriptor<{ slots: number }>;
const evt_AssignedSlots_MaxTemporarySlotsChanged: PlainDescriptor<{ slots: number }> =
  c9d5h5irbki7mm as PlainDescriptor<{ slots: number }>;
const evt_ValidatorManager_ValidatorsRegistered: PlainDescriptor<Array<SS58String>> =
  cccvjqfmga2rn as PlainDescriptor<Array<SS58String>>;
const evt_ValidatorManager_ValidatorsDeregistered: PlainDescriptor<Array<SS58String>> =
  cccvjqfmga2rn as PlainDescriptor<Array<SS58String>>;
const evt_StateTrieMigration_Migrated: PlainDescriptor<{
  top: number;
  child: number;
  compute: Anonymize<I2os86rbc98tr0>;
}> = "3m87ve6mm5k6n" as PlainDescriptor<{
  top: number;
  child: number;
  compute: Anonymize<I2os86rbc98tr0>;
}>;
const evt_StateTrieMigration_Slashed: PlainDescriptor<{ who: SS58String; amount: bigint }> =
  cd5fm4p8lj5qgi as PlainDescriptor<{ who: SS58String; amount: bigint }>;
const evt_StateTrieMigration_AutoMigrationFinished: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const evt_StateTrieMigration_Halted: PlainDescriptor<{ error: Anonymize<Ihok5kol9m3tk> }> =
  "73c6rq6fkseh2" as PlainDescriptor<{ error: Anonymize<Ihok5kol9m3tk> }>;
const evt_RootTesting_DefensiveTestCall: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const evt_Sudo_Sudid: PlainDescriptor<{ sudo_result: Anonymize<Idtdr91jmq5g4i> }> =
  c331o7t2g0ooi9 as PlainDescriptor<{ sudo_result: Anonymize<Idtdr91jmq5g4i> }>;
const evt_Sudo_KeyChanged: PlainDescriptor<{ old: Anonymize<Ihfphjolmsqq1>; new: SS58String }> =
  "5rtkmhm2dng4u" as PlainDescriptor<{ old: Anonymize<Ihfphjolmsqq1>; new: SS58String }>;
const evt_Sudo_KeyRemoved: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const evt_Sudo_SudoAsDone: PlainDescriptor<{ sudo_result: Anonymize<Idtdr91jmq5g4i> }> =
  c331o7t2g0ooi9 as PlainDescriptor<{ sudo_result: Anonymize<Idtdr91jmq5g4i> }>;
const err_System_InvalidSpecName: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_System_SpecVersionNeedsToIncrease: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_System_FailedToExtractRuntimeVersion: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_System_NonDefaultComposite: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_System_NonZeroRefCount: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_System_CallFiltered: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_System_NothingAuthorized: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_System_Unauthorized: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Babe_InvalidEquivocationProof: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Babe_InvalidKeyOwnershipProof: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Babe_DuplicateOffenceReport: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Babe_InvalidConfiguration: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Indices_NotAssigned: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Indices_NotOwner: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Indices_InUse: PlainDescriptor<undefined> = c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Indices_NotTransfer: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Indices_Permanent: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Balances_VestingBalance: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Balances_LiquidityRestrictions: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Balances_InsufficientBalance: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Balances_ExistentialDeposit: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Balances_Expendability: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Balances_ExistingVestingSchedule: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Balances_DeadAccount: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Balances_TooManyReserves: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Balances_TooManyHolds: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Balances_TooManyFreezes: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Balances_IssuanceDeactivated: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Balances_DeltaZero: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Session_InvalidProof: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Session_NoAssociatedValidatorId: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Session_DuplicatedKey: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Session_NoKeys: PlainDescriptor<undefined> = c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Session_NoAccount: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Grandpa_PauseFailed: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Grandpa_ResumeFailed: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Grandpa_ChangePending: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Grandpa_TooSoon: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Grandpa_InvalidKeyOwnershipProof: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Grandpa_InvalidEquivocationProof: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Grandpa_DuplicateOffenceReport: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Treasury_InsufficientProposersBalance: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Treasury_InvalidIndex: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Treasury_TooManyApprovals: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Treasury_InsufficientPermission: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Treasury_ProposalNotApproved: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Treasury_FailedToConvertBalance: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Treasury_SpendExpired: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Treasury_EarlyPayout: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Treasury_AlreadyAttempted: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Treasury_PayoutError: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Treasury_NotAttempted: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Treasury_Inconclusive: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ConvictionVoting_NotOngoing: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ConvictionVoting_NotVoter: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ConvictionVoting_NoPermission: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ConvictionVoting_NoPermissionYet: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ConvictionVoting_AlreadyDelegating: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ConvictionVoting_AlreadyVoting: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ConvictionVoting_InsufficientFunds: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ConvictionVoting_NotDelegating: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ConvictionVoting_Nonsense: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ConvictionVoting_MaxVotesReached: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ConvictionVoting_ClassNeeded: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ConvictionVoting_BadClass: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Referenda_NotOngoing: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Referenda_HasDeposit: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Referenda_BadTrack: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Referenda_Full: PlainDescriptor<undefined> = c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Referenda_QueueEmpty: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Referenda_BadReferendum: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Referenda_NothingToDo: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Referenda_NoTrack: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Referenda_Unfinished: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Referenda_NoPermission: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Referenda_NoDeposit: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Referenda_BadStatus: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Referenda_PreimageNotExist: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_FellowshipCollective_AlreadyMember: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_FellowshipCollective_NotMember: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_FellowshipCollective_NotPolling: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_FellowshipCollective_Ongoing: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_FellowshipCollective_NoneRemaining: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_FellowshipCollective_Corruption: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_FellowshipCollective_RankTooLow: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_FellowshipCollective_InvalidWitness: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_FellowshipCollective_NoPermission: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_FellowshipCollective_SameMember: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_FellowshipReferenda_NotOngoing: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_FellowshipReferenda_HasDeposit: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_FellowshipReferenda_BadTrack: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_FellowshipReferenda_Full: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_FellowshipReferenda_QueueEmpty: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_FellowshipReferenda_BadReferendum: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_FellowshipReferenda_NothingToDo: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_FellowshipReferenda_NoTrack: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_FellowshipReferenda_Unfinished: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_FellowshipReferenda_NoPermission: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_FellowshipReferenda_NoDeposit: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_FellowshipReferenda_BadStatus: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_FellowshipReferenda_PreimageNotExist: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Whitelist_UnavailablePreImage: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Whitelist_UndecodableCall: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Whitelist_InvalidCallWeightWitness: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Whitelist_CallIsNotWhitelisted: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Whitelist_CallAlreadyWhitelisted: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Claims_InvalidEthereumSignature: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Claims_SignerHasNoClaim: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Claims_SenderHasNoClaim: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Claims_PotUnderflow: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Claims_InvalidStatement: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Claims_VestedBalanceExists: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Utility_TooManyCalls: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Identity_TooManySubAccounts: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Identity_NotFound: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Identity_NotNamed: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Identity_EmptyIndex: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Identity_FeeChanged: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Identity_NoIdentity: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Identity_StickyJudgement: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Identity_JudgementGiven: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Identity_InvalidJudgement: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Identity_InvalidIndex: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Identity_InvalidTarget: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Identity_TooManyRegistrars: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Identity_AlreadyClaimed: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Identity_NotSub: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Identity_NotOwned: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Identity_JudgementForDifferentIdentity: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Identity_JudgementPaymentFailed: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Identity_InvalidSuffix: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Identity_NotUsernameAuthority: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Identity_NoAllocation: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Identity_InvalidSignature: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Identity_RequiresSignature: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Identity_InvalidUsername: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Identity_UsernameTaken: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Identity_NoUsername: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Identity_NotExpired: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Society_NotMember: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Society_AlreadyMember: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Society_Suspended: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Society_NotSuspended: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Society_NoPayout: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Society_AlreadyFounded: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Society_InsufficientPot: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Society_AlreadyVouching: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Society_NotVouchingOnBidder: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Society_Head: PlainDescriptor<undefined> = c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Society_Founder: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Society_AlreadyBid: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Society_AlreadyCandidate: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Society_NotCandidate: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Society_MaxMembers: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Society_NotFounder: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Society_NotHead: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Society_NotApproved: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Society_NotRejected: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Society_Approved: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Society_Rejected: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Society_InProgress: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Society_TooEarly: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Society_Voted: PlainDescriptor<undefined> = c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Society_Expired: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Society_NotBidder: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Society_NoDefender: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Society_NotGroup: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Society_AlreadyElevated: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Society_AlreadyPunished: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Society_InsufficientFunds: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Society_NoVotes: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Recovery_NotAllowed: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Recovery_ZeroThreshold: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Recovery_NotEnoughFriends: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Recovery_MaxFriends: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Recovery_NotSorted: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Recovery_NotRecoverable: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Recovery_AlreadyRecoverable: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Recovery_AlreadyStarted: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Recovery_NotStarted: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Recovery_NotFriend: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Recovery_DelayPeriod: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Recovery_AlreadyVouched: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Recovery_Threshold: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Recovery_StillActive: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Recovery_AlreadyProxy: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Recovery_BadState: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Vesting_NotVesting: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Vesting_AtMaxVestingSchedules: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Vesting_AmountLow: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Vesting_ScheduleIndexOutOfBounds: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Vesting_InvalidScheduleParams: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Scheduler_FailedToSchedule: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Scheduler_NotFound: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Scheduler_TargetBlockNumberInPast: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Scheduler_RescheduleNoChange: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Scheduler_Named: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Proxy_TooMany: PlainDescriptor<undefined> = c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Proxy_NotFound: PlainDescriptor<undefined> = c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Proxy_NotProxy: PlainDescriptor<undefined> = c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Proxy_Unproxyable: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Proxy_Duplicate: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Proxy_NoPermission: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Proxy_Unannounced: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Proxy_NoSelfProxy: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Multisig_MinimumThreshold: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Multisig_AlreadyApproved: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Multisig_NoApprovalsNeeded: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Multisig_TooFewSignatories: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Multisig_TooManySignatories: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Multisig_SignatoriesOutOfOrder: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Multisig_SenderInSignatories: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Multisig_NotFound: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Multisig_NotOwner: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Multisig_NoTimepoint: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Multisig_WrongTimepoint: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Multisig_UnexpectedTimepoint: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Multisig_MaxWeightTooLow: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Multisig_AlreadyStored: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Preimage_TooBig: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Preimage_AlreadyNoted: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Preimage_NotAuthorized: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Preimage_NotNoted: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Preimage_Requested: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Preimage_NotRequested: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Preimage_TooMany: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Preimage_TooFew: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_AssetRate_UnknownAssetKind: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_AssetRate_AlreadyExists: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_AssetRate_Overflow: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Bounties_InsufficientProposersBalance: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Bounties_InvalidIndex: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Bounties_ReasonTooBig: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Bounties_UnexpectedStatus: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Bounties_RequireCurator: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Bounties_InvalidValue: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Bounties_InvalidFee: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Bounties_PendingPayout: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Bounties_Premature: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Bounties_HasActiveChildBounty: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Bounties_TooManyQueued: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ChildBounties_ParentBountyNotActive: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ChildBounties_InsufficientBountyBalance: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ChildBounties_TooManyChildBounties: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Nis_DurationTooSmall: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Nis_DurationTooBig: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Nis_AmountTooSmall: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Nis_BidTooLow: PlainDescriptor<undefined> = c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Nis_UnknownReceipt: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Nis_NotOwner: PlainDescriptor<undefined> = c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Nis_NotExpired: PlainDescriptor<undefined> = c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Nis_UnknownBid: PlainDescriptor<undefined> = c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Nis_PortionTooBig: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Nis_Unfunded: PlainDescriptor<undefined> = c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Nis_AlreadyFunded: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Nis_Throttled: PlainDescriptor<undefined> = c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Nis_MakesDust: PlainDescriptor<undefined> = c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Nis_AlreadyCommunal: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Nis_AlreadyPrivate: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_NisCounterpartBalances_VestingBalance: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_NisCounterpartBalances_LiquidityRestrictions: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_NisCounterpartBalances_InsufficientBalance: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_NisCounterpartBalances_ExistentialDeposit: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_NisCounterpartBalances_Expendability: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_NisCounterpartBalances_ExistingVestingSchedule: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_NisCounterpartBalances_DeadAccount: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_NisCounterpartBalances_TooManyReserves: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_NisCounterpartBalances_TooManyHolds: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_NisCounterpartBalances_TooManyFreezes: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_NisCounterpartBalances_IssuanceDeactivated: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_NisCounterpartBalances_DeltaZero: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Configuration_InvalidNewValue: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ParaInclusion_UnsortedOrDuplicateValidatorIndices: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ParaInclusion_UnsortedOrDuplicateDisputeStatementSet: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ParaInclusion_UnsortedOrDuplicateBackedCandidates: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ParaInclusion_UnexpectedRelayParent: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ParaInclusion_WrongBitfieldSize: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ParaInclusion_BitfieldAllZeros: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ParaInclusion_BitfieldDuplicateOrUnordered: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ParaInclusion_ValidatorIndexOutOfBounds: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ParaInclusion_InvalidBitfieldSignature: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ParaInclusion_UnscheduledCandidate: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ParaInclusion_CandidateScheduledBeforeParaFree: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ParaInclusion_ScheduledOutOfOrder: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ParaInclusion_HeadDataTooLarge: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ParaInclusion_PrematureCodeUpgrade: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ParaInclusion_NewCodeTooLarge: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ParaInclusion_DisallowedRelayParent: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ParaInclusion_InvalidAssignment: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ParaInclusion_InvalidGroupIndex: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ParaInclusion_InsufficientBacking: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ParaInclusion_InvalidBacking: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ParaInclusion_NotCollatorSigned: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ParaInclusion_ValidationDataHashMismatch: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ParaInclusion_IncorrectDownwardMessageHandling: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ParaInclusion_InvalidUpwardMessages: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ParaInclusion_HrmpWatermarkMishandling: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ParaInclusion_InvalidOutboundHrmp: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ParaInclusion_InvalidValidationCodeHash: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ParaInclusion_ParaHeadMismatch: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ParaInclusion_BitfieldReferencesFreedCore: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ParaInherent_TooManyInclusionInherents: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ParaInherent_InvalidParentHeader: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ParaInherent_CandidateConcludedInvalid: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ParaInherent_InherentOverweight: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ParaInherent_DisputeStatementsUnsortedOrDuplicates: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ParaInherent_DisputeInvalid: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ParaInherent_BackedByDisabled: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ParaInherent_BackedOnUnscheduledCore: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ParaInherent_UnscheduledCandidate: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Paras_NotRegistered: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Paras_CannotOnboard: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Paras_CannotOffboard: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Paras_CannotUpgrade: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Paras_CannotDowngrade: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Paras_PvfCheckStatementStale: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Paras_PvfCheckStatementFuture: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Paras_PvfCheckValidatorIndexOutOfBounds: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Paras_PvfCheckInvalidSignature: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Paras_PvfCheckDoubleVote: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Paras_PvfCheckSubjectInvalid: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Paras_CannotUpgradeCode: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Paras_InvalidCode: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Hrmp_OpenHrmpChannelToSelf: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Hrmp_OpenHrmpChannelInvalidRecipient: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Hrmp_OpenHrmpChannelZeroCapacity: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Hrmp_OpenHrmpChannelCapacityExceedsLimit: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Hrmp_OpenHrmpChannelZeroMessageSize: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Hrmp_OpenHrmpChannelMessageSizeExceedsLimit: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Hrmp_OpenHrmpChannelAlreadyExists: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Hrmp_OpenHrmpChannelAlreadyRequested: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Hrmp_OpenHrmpChannelLimitExceeded: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Hrmp_AcceptHrmpChannelDoesntExist: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Hrmp_AcceptHrmpChannelAlreadyConfirmed: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Hrmp_AcceptHrmpChannelLimitExceeded: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Hrmp_CloseHrmpChannelUnauthorized: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Hrmp_CloseHrmpChannelDoesntExist: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Hrmp_CloseHrmpChannelAlreadyUnderway: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Hrmp_CancelHrmpOpenChannelUnauthorized: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Hrmp_OpenHrmpChannelDoesntExist: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Hrmp_OpenHrmpChannelAlreadyConfirmed: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Hrmp_WrongWitness: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Hrmp_ChannelCreationNotAuthorized: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ParasDisputes_DuplicateDisputeStatementSets: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ParasDisputes_AncientDisputeStatement: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ParasDisputes_ValidatorIndexOutOfBounds: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ParasDisputes_InvalidSignature: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ParasDisputes_DuplicateStatement: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ParasDisputes_SingleSidedDispute: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ParasDisputes_MaliciousBacker: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ParasDisputes_MissingBackingVotes: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ParasDisputes_UnconfirmedDispute: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ParasSlashing_InvalidKeyOwnershipProof: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ParasSlashing_InvalidSessionIndex: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ParasSlashing_InvalidCandidateHash: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ParasSlashing_InvalidValidatorIndex: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ParasSlashing_ValidatorIndexIdMismatch: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ParasSlashing_DuplicateSlashingReport: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_MessageQueue_NotReapable: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_MessageQueue_NoPage: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_MessageQueue_NoMessage: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_MessageQueue_AlreadyProcessed: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_MessageQueue_Queued: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_MessageQueue_InsufficientWeight: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_MessageQueue_TemporarilyUnprocessable: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_MessageQueue_QueuePaused: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_MessageQueue_RecursiveDisallowed: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_OnDemandAssignmentProvider_InvalidParaId: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_OnDemandAssignmentProvider_QueueFull: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_OnDemandAssignmentProvider_SpotPriceHigherThanMaxAmount: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_CoretimeAssignmentProvider_AssignmentsEmpty: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_CoretimeAssignmentProvider_OverScheduled: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_CoretimeAssignmentProvider_UnderScheduled: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_CoretimeAssignmentProvider_DisallowedInsert: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_CoretimeAssignmentProvider_DuplicateInsert: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_CoretimeAssignmentProvider_AssignmentsNotSorted: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Registrar_NotRegistered: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Registrar_AlreadyRegistered: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Registrar_NotOwner: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Registrar_CodeTooLarge: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Registrar_HeadDataTooLarge: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Registrar_NotParachain: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Registrar_NotParathread: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Registrar_CannotDeregister: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Registrar_CannotDowngrade: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Registrar_CannotUpgrade: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Registrar_ParaLocked: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Registrar_NotReserved: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Registrar_InvalidCode: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Registrar_CannotSwap: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Slots_ParaNotOnboarding: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Slots_LeaseError: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Auctions_AuctionInProgress: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Auctions_LeasePeriodInPast: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Auctions_ParaNotRegistered: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Auctions_NotCurrentAuction: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Auctions_NotAuction: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Auctions_AuctionEnded: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Auctions_AlreadyLeasedOut: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Crowdloan_FirstPeriodInPast: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Crowdloan_FirstPeriodTooFarInFuture: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Crowdloan_LastPeriodBeforeFirstPeriod: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Crowdloan_LastPeriodTooFarInFuture: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Crowdloan_CannotEndInPast: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Crowdloan_EndTooFarInFuture: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Crowdloan_Overflow: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Crowdloan_ContributionTooSmall: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Crowdloan_InvalidParaId: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Crowdloan_CapExceeded: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Crowdloan_ContributionPeriodOver: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Crowdloan_InvalidOrigin: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Crowdloan_NotParachain: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Crowdloan_LeaseActive: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Crowdloan_BidOrLeaseActive: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Crowdloan_FundNotEnded: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Crowdloan_NoContributions: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Crowdloan_NotReadyToDissolve: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Crowdloan_InvalidSignature: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Crowdloan_MemoTooLarge: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Crowdloan_AlreadyInNewRaise: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Crowdloan_VrfDelayInProgress: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Crowdloan_NoLeasePeriod: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Coretime_NotBroker: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_XcmPallet_Unreachable: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_XcmPallet_SendFailure: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_XcmPallet_Filtered: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_XcmPallet_UnweighableMessage: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_XcmPallet_DestinationNotInvertible: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_XcmPallet_Empty: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_XcmPallet_CannotReanchor: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_XcmPallet_TooManyAssets: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_XcmPallet_InvalidOrigin: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_XcmPallet_BadVersion: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_XcmPallet_BadLocation: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_XcmPallet_NoSubscription: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_XcmPallet_AlreadySubscribed: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_XcmPallet_CannotCheckOutTeleport: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_XcmPallet_LowBalance: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_XcmPallet_TooManyLocks: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_XcmPallet_AccountNotSovereign: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_XcmPallet_FeesNotMet: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_XcmPallet_LockNotFound: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_XcmPallet_InUse: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_XcmPallet_InvalidAssetNotConcrete: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_XcmPallet_InvalidAssetUnknownReserve: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_XcmPallet_InvalidAssetUnsupportedReserve: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_XcmPallet_TooManyReserves: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_XcmPallet_LocalExecutionIncomplete: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Beefy_InvalidKeyOwnershipProof: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Beefy_InvalidEquivocationProof: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Beefy_DuplicateOffenceReport: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Beefy_InvalidConfiguration: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ParasSudoWrapper_ParaDoesntExist: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ParasSudoWrapper_ParaAlreadyExists: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ParasSudoWrapper_ExceedsMaxMessageSize: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ParasSudoWrapper_CouldntCleanup: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ParasSudoWrapper_NotParathread: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ParasSudoWrapper_NotParachain: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ParasSudoWrapper_CannotUpgrade: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ParasSudoWrapper_CannotDowngrade: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ParasSudoWrapper_TooManyCores: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_AssignedSlots_ParaDoesntExist: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_AssignedSlots_NotParathread: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_AssignedSlots_CannotUpgrade: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_AssignedSlots_CannotDowngrade: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_AssignedSlots_SlotAlreadyAssigned: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_AssignedSlots_SlotNotAssigned: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_AssignedSlots_OngoingLeaseExists: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_AssignedSlots_MaxPermanentSlotsExceeded: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_AssignedSlots_MaxTemporarySlotsExceeded: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_StateTrieMigration_MaxSignedLimits: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_StateTrieMigration_KeyTooLong: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_StateTrieMigration_NotEnoughFunds: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_StateTrieMigration_BadWitness: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_StateTrieMigration_SignedMigrationNotAllowed: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_StateTrieMigration_BadChildRoot: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Sudo_RequireSudo: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const const_System_BlockWeights: PlainDescriptor<{
  base_block: Anonymize<I4q39t5hn830vp>;
  max_block: Anonymize<I4q39t5hn830vp>;
  per_class: Anonymize<I79te2qqsklnbd>;
}> = "n7a38730s6qs" as PlainDescriptor<{
  base_block: Anonymize<I4q39t5hn830vp>;
  max_block: Anonymize<I4q39t5hn830vp>;
  per_class: Anonymize<I79te2qqsklnbd>;
}>;
const const_System_BlockLength: PlainDescriptor<{
  normal: number;
  operational: number;
  mandatory: number;
}> = "f15el53dd76v9" as PlainDescriptor<{ normal: number; operational: number; mandatory: number }>;
const const_System_BlockHashCount: PlainDescriptor<number> =
  cf3ncnq6stc567 as PlainDescriptor<number>;
const const_System_DbWeight: PlainDescriptor<{ read: bigint; write: bigint }> =
  "9s0ave7t0vnrk" as PlainDescriptor<{ read: bigint; write: bigint }>;
const const_System_Version: PlainDescriptor<{
  spec_name: string;
  impl_name: string;
  authoring_version: number;
  spec_version: number;
  impl_version: number;
  apis: Anonymize<I1st1p92iu8h7e>;
  transaction_version: number;
  state_version: number;
}> = "b5fo0fakl8rog" as PlainDescriptor<{
  spec_name: string;
  impl_name: string;
  authoring_version: number;
  spec_version: number;
  impl_version: number;
  apis: Anonymize<I1st1p92iu8h7e>;
  transaction_version: number;
  state_version: number;
}>;
const const_System_SS58Prefix: PlainDescriptor<number> = cf3ncnq6stc567 as PlainDescriptor<number>;
const const_Babe_EpochDuration: PlainDescriptor<bigint> = c7d2tqf029vcod as PlainDescriptor<bigint>;
const const_Babe_ExpectedBlockTime: PlainDescriptor<bigint> =
  c7d2tqf029vcod as PlainDescriptor<bigint>;
const const_Babe_MaxAuthorities: PlainDescriptor<number> =
  cf3ncnq6stc567 as PlainDescriptor<number>;
const const_Babe_MaxNominators: PlainDescriptor<number> = cf3ncnq6stc567 as PlainDescriptor<number>;
const const_Timestamp_MinimumPeriod: PlainDescriptor<bigint> =
  c7d2tqf029vcod as PlainDescriptor<bigint>;
const const_Indices_Deposit: PlainDescriptor<bigint> = c7d2tqf029vcod as PlainDescriptor<bigint>;
const const_Balances_ExistentialDeposit: PlainDescriptor<bigint> =
  c7d2tqf029vcod as PlainDescriptor<bigint>;
const const_Balances_MaxLocks: PlainDescriptor<number> = cf3ncnq6stc567 as PlainDescriptor<number>;
const const_Balances_MaxReserves: PlainDescriptor<number> =
  cf3ncnq6stc567 as PlainDescriptor<number>;
const const_Balances_MaxFreezes: PlainDescriptor<number> =
  cf3ncnq6stc567 as PlainDescriptor<number>;
const const_TransactionPayment_OperationalFeeMultiplier: PlainDescriptor<number> =
  cf3ncnq6stc567 as PlainDescriptor<number>;
const const_Grandpa_MaxAuthorities: PlainDescriptor<number> =
  cf3ncnq6stc567 as PlainDescriptor<number>;
const const_Grandpa_MaxNominators: PlainDescriptor<number> =
  cf3ncnq6stc567 as PlainDescriptor<number>;
const const_Grandpa_MaxSetIdSessionEntries: PlainDescriptor<bigint> =
  c7d2tqf029vcod as PlainDescriptor<bigint>;
const const_Treasury_ProposalBond: PlainDescriptor<number> =
  cf3ncnq6stc567 as PlainDescriptor<number>;
const const_Treasury_ProposalBondMinimum: PlainDescriptor<bigint> =
  c7d2tqf029vcod as PlainDescriptor<bigint>;
const const_Treasury_ProposalBondMaximum: PlainDescriptor<bigint | undefined> =
  c35p85j063s0il as PlainDescriptor<bigint | undefined>;
const const_Treasury_SpendPeriod: PlainDescriptor<number> =
  cf3ncnq6stc567 as PlainDescriptor<number>;
const const_Treasury_Burn: PlainDescriptor<number> = cf3ncnq6stc567 as PlainDescriptor<number>;
const const_Treasury_PalletId: PlainDescriptor<Binary> = c7l9d2ilde057d as PlainDescriptor<Binary>;
const const_Treasury_MaxApprovals: PlainDescriptor<number> =
  cf3ncnq6stc567 as PlainDescriptor<number>;
const const_Treasury_PayoutPeriod: PlainDescriptor<number> =
  cf3ncnq6stc567 as PlainDescriptor<number>;
const const_ConvictionVoting_MaxVotes: PlainDescriptor<number> =
  cf3ncnq6stc567 as PlainDescriptor<number>;
const const_ConvictionVoting_VoteLockingPeriod: PlainDescriptor<number> =
  cf3ncnq6stc567 as PlainDescriptor<number>;
const const_Referenda_SubmissionDeposit: PlainDescriptor<bigint> =
  c7d2tqf029vcod as PlainDescriptor<bigint>;
const const_Referenda_MaxQueued: PlainDescriptor<number> =
  cf3ncnq6stc567 as PlainDescriptor<number>;
const const_Referenda_UndecidingTimeout: PlainDescriptor<number> =
  cf3ncnq6stc567 as PlainDescriptor<number>;
const const_Referenda_AlarmInterval: PlainDescriptor<number> =
  cf3ncnq6stc567 as PlainDescriptor<number>;
const const_Referenda_Tracks: PlainDescriptor<Array<Anonymize<Ida9vhl30l98p4>>> =
  cbafpkl9hhno69 as PlainDescriptor<Array<Anonymize<Ida9vhl30l98p4>>>;
const const_FellowshipReferenda_SubmissionDeposit: PlainDescriptor<bigint> =
  c7d2tqf029vcod as PlainDescriptor<bigint>;
const const_FellowshipReferenda_MaxQueued: PlainDescriptor<number> =
  cf3ncnq6stc567 as PlainDescriptor<number>;
const const_FellowshipReferenda_UndecidingTimeout: PlainDescriptor<number> =
  cf3ncnq6stc567 as PlainDescriptor<number>;
const const_FellowshipReferenda_AlarmInterval: PlainDescriptor<number> =
  cf3ncnq6stc567 as PlainDescriptor<number>;
const const_FellowshipReferenda_Tracks: PlainDescriptor<Array<Anonymize<Ida9vhl30l98p4>>> =
  cbafpkl9hhno69 as PlainDescriptor<Array<Anonymize<Ida9vhl30l98p4>>>;
const const_Claims_Prefix: PlainDescriptor<Binary> = "51jgofmopm9om" as PlainDescriptor<Binary>;
const const_Utility_batched_calls_limit: PlainDescriptor<number> =
  cf3ncnq6stc567 as PlainDescriptor<number>;
const const_Identity_BasicDeposit: PlainDescriptor<bigint> =
  c7d2tqf029vcod as PlainDescriptor<bigint>;
const const_Identity_ByteDeposit: PlainDescriptor<bigint> =
  c7d2tqf029vcod as PlainDescriptor<bigint>;
const const_Identity_SubAccountDeposit: PlainDescriptor<bigint> =
  c7d2tqf029vcod as PlainDescriptor<bigint>;
const const_Identity_MaxSubAccounts: PlainDescriptor<number> =
  cf3ncnq6stc567 as PlainDescriptor<number>;
const const_Identity_MaxRegistrars: PlainDescriptor<number> =
  cf3ncnq6stc567 as PlainDescriptor<number>;
const const_Identity_PendingUsernameExpiration: PlainDescriptor<number> =
  cf3ncnq6stc567 as PlainDescriptor<number>;
const const_Identity_MaxSuffixLength: PlainDescriptor<number> =
  cf3ncnq6stc567 as PlainDescriptor<number>;
const const_Identity_MaxUsernameLength: PlainDescriptor<number> =
  cf3ncnq6stc567 as PlainDescriptor<number>;
const const_Society_PalletId: PlainDescriptor<Binary> = c7l9d2ilde057d as PlainDescriptor<Binary>;
const const_Society_GraceStrikes: PlainDescriptor<number> =
  cf3ncnq6stc567 as PlainDescriptor<number>;
const const_Society_PeriodSpend: PlainDescriptor<bigint> =
  c7d2tqf029vcod as PlainDescriptor<bigint>;
const const_Society_VotingPeriod: PlainDescriptor<number> =
  cf3ncnq6stc567 as PlainDescriptor<number>;
const const_Society_ClaimPeriod: PlainDescriptor<number> =
  cf3ncnq6stc567 as PlainDescriptor<number>;
const const_Society_MaxLockDuration: PlainDescriptor<number> =
  cf3ncnq6stc567 as PlainDescriptor<number>;
const const_Society_ChallengePeriod: PlainDescriptor<number> =
  cf3ncnq6stc567 as PlainDescriptor<number>;
const const_Society_MaxPayouts: PlainDescriptor<number> = cf3ncnq6stc567 as PlainDescriptor<number>;
const const_Society_MaxBids: PlainDescriptor<number> = cf3ncnq6stc567 as PlainDescriptor<number>;
const const_Recovery_ConfigDepositBase: PlainDescriptor<bigint> =
  c7d2tqf029vcod as PlainDescriptor<bigint>;
const const_Recovery_FriendDepositFactor: PlainDescriptor<bigint> =
  c7d2tqf029vcod as PlainDescriptor<bigint>;
const const_Recovery_MaxFriends: PlainDescriptor<number> =
  cf3ncnq6stc567 as PlainDescriptor<number>;
const const_Recovery_RecoveryDeposit: PlainDescriptor<bigint> =
  c7d2tqf029vcod as PlainDescriptor<bigint>;
const const_Vesting_MinVestedTransfer: PlainDescriptor<bigint> =
  c7d2tqf029vcod as PlainDescriptor<bigint>;
const const_Vesting_MaxVestingSchedules: PlainDescriptor<number> =
  cf3ncnq6stc567 as PlainDescriptor<number>;
const const_Scheduler_MaximumWeight: PlainDescriptor<{ ref_time: bigint; proof_size: bigint }> =
  c4q39t5hn830vp as PlainDescriptor<{ ref_time: bigint; proof_size: bigint }>;
const const_Scheduler_MaxScheduledPerBlock: PlainDescriptor<number> =
  cf3ncnq6stc567 as PlainDescriptor<number>;
const const_Proxy_ProxyDepositBase: PlainDescriptor<bigint> =
  c7d2tqf029vcod as PlainDescriptor<bigint>;
const const_Proxy_ProxyDepositFactor: PlainDescriptor<bigint> =
  c7d2tqf029vcod as PlainDescriptor<bigint>;
const const_Proxy_MaxProxies: PlainDescriptor<number> = cf3ncnq6stc567 as PlainDescriptor<number>;
const const_Proxy_MaxPending: PlainDescriptor<number> = cf3ncnq6stc567 as PlainDescriptor<number>;
const const_Proxy_AnnouncementDepositBase: PlainDescriptor<bigint> =
  c7d2tqf029vcod as PlainDescriptor<bigint>;
const const_Proxy_AnnouncementDepositFactor: PlainDescriptor<bigint> =
  c7d2tqf029vcod as PlainDescriptor<bigint>;
const const_Multisig_DepositBase: PlainDescriptor<bigint> =
  c7d2tqf029vcod as PlainDescriptor<bigint>;
const const_Multisig_DepositFactor: PlainDescriptor<bigint> =
  c7d2tqf029vcod as PlainDescriptor<bigint>;
const const_Multisig_MaxSignatories: PlainDescriptor<number> =
  cf3ncnq6stc567 as PlainDescriptor<number>;
const const_Bounties_BountyDepositBase: PlainDescriptor<bigint> =
  c7d2tqf029vcod as PlainDescriptor<bigint>;
const const_Bounties_BountyDepositPayoutDelay: PlainDescriptor<number> =
  cf3ncnq6stc567 as PlainDescriptor<number>;
const const_Bounties_BountyUpdatePeriod: PlainDescriptor<number> =
  cf3ncnq6stc567 as PlainDescriptor<number>;
const const_Bounties_CuratorDepositMultiplier: PlainDescriptor<number> =
  cf3ncnq6stc567 as PlainDescriptor<number>;
const const_Bounties_CuratorDepositMax: PlainDescriptor<bigint | undefined> =
  c35p85j063s0il as PlainDescriptor<bigint | undefined>;
const const_Bounties_CuratorDepositMin: PlainDescriptor<bigint | undefined> =
  c35p85j063s0il as PlainDescriptor<bigint | undefined>;
const const_Bounties_BountyValueMinimum: PlainDescriptor<bigint> =
  c7d2tqf029vcod as PlainDescriptor<bigint>;
const const_Bounties_DataDepositPerByte: PlainDescriptor<bigint> =
  c7d2tqf029vcod as PlainDescriptor<bigint>;
const const_Bounties_MaximumReasonLength: PlainDescriptor<number> =
  cf3ncnq6stc567 as PlainDescriptor<number>;
const const_ChildBounties_MaxActiveChildBountyCount: PlainDescriptor<number> =
  cf3ncnq6stc567 as PlainDescriptor<number>;
const const_ChildBounties_ChildBountyValueMinimum: PlainDescriptor<bigint> =
  c7d2tqf029vcod as PlainDescriptor<bigint>;
const const_Nis_PalletId: PlainDescriptor<Binary> = c7l9d2ilde057d as PlainDescriptor<Binary>;
const const_Nis_QueueCount: PlainDescriptor<number> = cf3ncnq6stc567 as PlainDescriptor<number>;
const const_Nis_MaxQueueLen: PlainDescriptor<number> = cf3ncnq6stc567 as PlainDescriptor<number>;
const const_Nis_FifoQueueLen: PlainDescriptor<number> = cf3ncnq6stc567 as PlainDescriptor<number>;
const const_Nis_BasePeriod: PlainDescriptor<number> = cf3ncnq6stc567 as PlainDescriptor<number>;
const const_Nis_MinBid: PlainDescriptor<bigint> = c7d2tqf029vcod as PlainDescriptor<bigint>;
const const_Nis_MinReceipt: PlainDescriptor<bigint> = c7d2tqf029vcod as PlainDescriptor<bigint>;
const const_Nis_IntakePeriod: PlainDescriptor<number> = cf3ncnq6stc567 as PlainDescriptor<number>;
const const_Nis_MaxIntakeWeight: PlainDescriptor<{ ref_time: bigint; proof_size: bigint }> =
  c4q39t5hn830vp as PlainDescriptor<{ ref_time: bigint; proof_size: bigint }>;
const const_Nis_ThawThrottle: PlainDescriptor<[bigint, number]> =
  "6cs1itejju2vv" as PlainDescriptor<[bigint, number]>;
const const_NisCounterpartBalances_ExistentialDeposit: PlainDescriptor<bigint> =
  c7d2tqf029vcod as PlainDescriptor<bigint>;
const const_NisCounterpartBalances_MaxLocks: PlainDescriptor<number> =
  cf3ncnq6stc567 as PlainDescriptor<number>;
const const_NisCounterpartBalances_MaxReserves: PlainDescriptor<number> =
  cf3ncnq6stc567 as PlainDescriptor<number>;
const const_NisCounterpartBalances_MaxFreezes: PlainDescriptor<number> =
  cf3ncnq6stc567 as PlainDescriptor<number>;
const const_Paras_UnsignedPriority: PlainDescriptor<bigint> =
  c7d2tqf029vcod as PlainDescriptor<bigint>;
const const_MessageQueue_HeapSize: PlainDescriptor<number> =
  cf3ncnq6stc567 as PlainDescriptor<number>;
const const_MessageQueue_MaxStale: PlainDescriptor<number> =
  cf3ncnq6stc567 as PlainDescriptor<number>;
const const_MessageQueue_ServiceWeight: PlainDescriptor<Anonymize<I4q39t5hn830vp> | undefined> =
  "asb8k6ash5mjn" as PlainDescriptor<Anonymize<I4q39t5hn830vp> | undefined>;
const const_OnDemandAssignmentProvider_TrafficDefaultValue: PlainDescriptor<bigint> =
  c7d2tqf029vcod as PlainDescriptor<bigint>;
const const_Registrar_ParaDeposit: PlainDescriptor<bigint> =
  c7d2tqf029vcod as PlainDescriptor<bigint>;
const const_Registrar_DataDepositPerByte: PlainDescriptor<bigint> =
  c7d2tqf029vcod as PlainDescriptor<bigint>;
const const_Slots_LeasePeriod: PlainDescriptor<number> = cf3ncnq6stc567 as PlainDescriptor<number>;
const const_Slots_LeaseOffset: PlainDescriptor<number> = cf3ncnq6stc567 as PlainDescriptor<number>;
const const_Auctions_EndingPeriod: PlainDescriptor<number> =
  cf3ncnq6stc567 as PlainDescriptor<number>;
const const_Auctions_SampleLength: PlainDescriptor<number> =
  cf3ncnq6stc567 as PlainDescriptor<number>;
const const_Auctions_SlotRangeCount: PlainDescriptor<number> =
  cf3ncnq6stc567 as PlainDescriptor<number>;
const const_Auctions_LeasePeriodsPerSlot: PlainDescriptor<number> =
  cf3ncnq6stc567 as PlainDescriptor<number>;
const const_Crowdloan_PalletId: PlainDescriptor<Binary> = c7l9d2ilde057d as PlainDescriptor<Binary>;
const const_Crowdloan_MinContribution: PlainDescriptor<bigint> =
  c7d2tqf029vcod as PlainDescriptor<bigint>;
const const_Crowdloan_RemoveKeysLimit: PlainDescriptor<number> =
  cf3ncnq6stc567 as PlainDescriptor<number>;
const const_Coretime_BrokerId: PlainDescriptor<number> = cf3ncnq6stc567 as PlainDescriptor<number>;
const const_Beefy_MaxAuthorities: PlainDescriptor<number> =
  cf3ncnq6stc567 as PlainDescriptor<number>;
const const_Beefy_MaxNominators: PlainDescriptor<number> =
  cf3ncnq6stc567 as PlainDescriptor<number>;
const const_Beefy_MaxSetIdSessionEntries: PlainDescriptor<bigint> =
  c7d2tqf029vcod as PlainDescriptor<bigint>;
const const_AssignedSlots_PermanentSlotLeasePeriodLength: PlainDescriptor<number> =
  cf3ncnq6stc567 as PlainDescriptor<number>;
const const_AssignedSlots_TemporarySlotLeasePeriodLength: PlainDescriptor<number> =
  cf3ncnq6stc567 as PlainDescriptor<number>;
const const_AssignedSlots_MaxTemporarySlotPerLeasePeriod: PlainDescriptor<number> =
  cf3ncnq6stc567 as PlainDescriptor<number>;
const const_StateTrieMigration_MaxKeyLen: PlainDescriptor<number> =
  cf3ncnq6stc567 as PlainDescriptor<number>;
const runtime_Core_version: RuntimeDescriptor<
  [],
  {
    spec_name: string;
    impl_name: string;
    authoring_version: number;
    spec_version: number;
    impl_version: number;
    apis: Anonymize<I1st1p92iu8h7e>;
    transaction_version: number;
    state_version: number;
  }
> = "bckjab5tbgn0d" as RuntimeDescriptor<
  [],
  {
    spec_name: string;
    impl_name: string;
    authoring_version: number;
    spec_version: number;
    impl_version: number;
    apis: Anonymize<I1st1p92iu8h7e>;
    transaction_version: number;
    state_version: number;
  }
>;
const runtime_Core_execute_block: RuntimeDescriptor<
  [block: { header: Anonymize<I6t1nedlt7mobn>; extrinsics: Anonymize<Itom7fk49o0c9> }],
  undefined
> = "etd7e8evd7agp" as RuntimeDescriptor<
  [block: { header: Anonymize<I6t1nedlt7mobn>; extrinsics: Anonymize<Itom7fk49o0c9> }],
  undefined
>;
const runtime_Core_initialize_block: RuntimeDescriptor<
  [
    header: {
      parent_hash: Binary;
      number: number;
      state_root: Binary;
      extrinsics_root: Binary;
      digest: Anonymize<Idin6nhq46lvdj>;
    },
  ],
  undefined
> = "2lnjmv4e9dljk" as RuntimeDescriptor<
  [
    header: {
      parent_hash: Binary;
      number: number;
      state_root: Binary;
      extrinsics_root: Binary;
      digest: Anonymize<Idin6nhq46lvdj>;
    },
  ],
  undefined
>;
const runtime_Metadata_metadata: RuntimeDescriptor<[], Binary> =
  "57k61dni7tq1q" as RuntimeDescriptor<[], Binary>;
const runtime_Metadata_metadata_at_version: RuntimeDescriptor<
  [version: number],
  Binary | undefined
> = "6s9tg3j576u36" as RuntimeDescriptor<[version: number], Binary | undefined>;
const runtime_Metadata_metadata_versions: RuntimeDescriptor<
  [],
  Array<number>
> = "64k0v4qcjg4rj" as RuntimeDescriptor<[], Array<number>>;
const runtime_BlockBuilder_apply_extrinsic: RuntimeDescriptor<
  [extrinsic: Binary],
  ResultPayload<Anonymize<Idtdr91jmq5g4i>, TransactionValidityError>
> = "1pgfbek4i461t" as RuntimeDescriptor<
  [extrinsic: Binary],
  ResultPayload<Anonymize<Idtdr91jmq5g4i>, TransactionValidityError>
>;
const runtime_BlockBuilder_finalize_block: RuntimeDescriptor<
  [],
  {
    parent_hash: Binary;
    number: number;
    state_root: Binary;
    extrinsics_root: Binary;
    digest: Anonymize<Idin6nhq46lvdj>;
  }
> = "anacqk7csnask" as RuntimeDescriptor<
  [],
  {
    parent_hash: Binary;
    number: number;
    state_root: Binary;
    extrinsics_root: Binary;
    digest: Anonymize<Idin6nhq46lvdj>;
  }
>;
const runtime_BlockBuilder_inherent_extrinsics: RuntimeDescriptor<
  [inherent: Array<Anonymize<I1kbn2golmm2dm>>],
  Array<Binary>
> = "69ch9obc52eu6" as RuntimeDescriptor<
  [inherent: Array<Anonymize<I1kbn2golmm2dm>>],
  Array<Binary>
>;
const runtime_BlockBuilder_check_inherents: RuntimeDescriptor<
  [
    block: { header: Anonymize<I6t1nedlt7mobn>; extrinsics: Anonymize<Itom7fk49o0c9> },
    data: Array<Anonymize<I1kbn2golmm2dm>>,
  ],
  { okay: boolean; fatal_error: boolean; errors: Anonymize<If39abi8floaaf> }
> = "cnpijjh9g0g79" as RuntimeDescriptor<
  [
    block: { header: Anonymize<I6t1nedlt7mobn>; extrinsics: Anonymize<Itom7fk49o0c9> },
    data: Array<Anonymize<I1kbn2golmm2dm>>,
  ],
  { okay: boolean; fatal_error: boolean; errors: Anonymize<If39abi8floaaf> }
>;
const runtime_TaggedTransactionQueue_validate_transaction: RuntimeDescriptor<
  [source: TransactionValidityTransactionSource, tx: Binary, block_hash: Binary],
  ResultPayload<Anonymize<I6g5lcd9vf2cr0>, TransactionValidityError>
> = "7v3l8ub2dan4" as RuntimeDescriptor<
  [source: TransactionValidityTransactionSource, tx: Binary, block_hash: Binary],
  ResultPayload<Anonymize<I6g5lcd9vf2cr0>, TransactionValidityError>
>;
const runtime_OffchainWorkerApi_offchain_worker: RuntimeDescriptor<
  [
    header: {
      parent_hash: Binary;
      number: number;
      state_root: Binary;
      extrinsics_root: Binary;
      digest: Anonymize<Idin6nhq46lvdj>;
    },
  ],
  undefined
> = "2lnjmv4e9dljk" as RuntimeDescriptor<
  [
    header: {
      parent_hash: Binary;
      number: number;
      state_root: Binary;
      extrinsics_root: Binary;
      digest: Anonymize<Idin6nhq46lvdj>;
    },
  ],
  undefined
>;
const runtime_ParachainHost_validators: RuntimeDescriptor<
  [],
  Array<Binary>
> = "2tjq7ncksi1hj" as RuntimeDescriptor<[], Array<Binary>>;
const runtime_ParachainHost_validator_groups: RuntimeDescriptor<
  [],
  [Anonymize<Iarlj3qd8u1v13>, Anonymize<I94uslvmqboam8>]
> = "2l2qs25mbqi3l" as RuntimeDescriptor<
  [],
  [Anonymize<Iarlj3qd8u1v13>, Anonymize<I94uslvmqboam8>]
>;
const runtime_ParachainHost_availability_cores: RuntimeDescriptor<
  [],
  Array<PolkadotPrimitivesV5CoreState>
> = "34d26ic4dpmld" as RuntimeDescriptor<[], Array<PolkadotPrimitivesV5CoreState>>;
const runtime_ParachainHost_persisted_validation_data: RuntimeDescriptor<
  [para_id: number, assumption: PolkadotPrimitivesV5OccupiedCoreAssumption],
  Anonymize<I5r8ef6aie125l> | undefined
> = "ahdhorm1njpaj" as RuntimeDescriptor<
  [para_id: number, assumption: PolkadotPrimitivesV5OccupiedCoreAssumption],
  Anonymize<I5r8ef6aie125l> | undefined
>;
const runtime_ParachainHost_assumed_validation_data: RuntimeDescriptor<
  [para_id: number, expected_persisted_validation_data_hash: Binary],
  Anonymize<I9rov8gdjkv3b9> | undefined
> = "6dim566dra8pm" as RuntimeDescriptor<
  [para_id: number, expected_persisted_validation_data_hash: Binary],
  Anonymize<I9rov8gdjkv3b9> | undefined
>;
const runtime_ParachainHost_check_validation_outputs: RuntimeDescriptor<
  [
    para_id: number,
    outputs: {
      upward_messages: Anonymize<Itom7fk49o0c9>;
      horizontal_messages: Anonymize<I6r5cbv8ttrb09>;
      new_validation_code: Anonymize<Iabpgqcjikia83>;
      head_data: Binary;
      processed_downward_messages: number;
      hrmp_watermark: number;
    },
  ],
  boolean
> = "7ac5amfbp9nh6" as RuntimeDescriptor<
  [
    para_id: number,
    outputs: {
      upward_messages: Anonymize<Itom7fk49o0c9>;
      horizontal_messages: Anonymize<I6r5cbv8ttrb09>;
      new_validation_code: Anonymize<Iabpgqcjikia83>;
      head_data: Binary;
      processed_downward_messages: number;
      hrmp_watermark: number;
    },
  ],
  boolean
>;
const runtime_ParachainHost_session_index_for_child: RuntimeDescriptor<[], number> =
  "5kdm82a42hnhk" as RuntimeDescriptor<[], number>;
const runtime_ParachainHost_validation_code: RuntimeDescriptor<
  [para_id: number, assumption: PolkadotPrimitivesV5OccupiedCoreAssumption],
  Binary | undefined
> = "ckdan58a602bd" as RuntimeDescriptor<
  [para_id: number, assumption: PolkadotPrimitivesV5OccupiedCoreAssumption],
  Binary | undefined
>;
const runtime_ParachainHost_candidate_pending_availability: RuntimeDescriptor<
  [para_id: number],
  Anonymize<Iedmhjqij0hr8g> | undefined
> = "dmhi3h77v35n" as RuntimeDescriptor<[para_id: number], Anonymize<Iedmhjqij0hr8g> | undefined>;
const runtime_ParachainHost_candidate_events: RuntimeDescriptor<
  [],
  Array<PolkadotPrimitivesV5CandidateEvent>
> = "f51o56v7ot5sa" as RuntimeDescriptor<[], Array<PolkadotPrimitivesV5CandidateEvent>>;
const runtime_ParachainHost_dmq_contents: RuntimeDescriptor<
  [recipient: number],
  Array<Anonymize<I60847k37jfcc6>>
> = "4dcsp55rkhols" as RuntimeDescriptor<[recipient: number], Array<Anonymize<I60847k37jfcc6>>>;
const runtime_ParachainHost_inbound_hrmp_channels_contents: RuntimeDescriptor<
  [recipient: number],
  Array<Anonymize<I9hvej6h53dqj0>>
> = "4et4b0kbi7l1b" as RuntimeDescriptor<[recipient: number], Array<Anonymize<I9hvej6h53dqj0>>>;
const runtime_ParachainHost_validation_code_by_hash: RuntimeDescriptor<
  [hash: Binary],
  Binary | undefined
> = "bhlals2nhjo4" as RuntimeDescriptor<[hash: Binary], Binary | undefined>;
const runtime_ParachainHost_on_chain_votes: RuntimeDescriptor<
  [],
  Anonymize<Ie7c40q4va8u7r> | undefined
> = "ev7qmpuc7kj8l" as RuntimeDescriptor<[], Anonymize<Ie7c40q4va8u7r> | undefined>;
const runtime_ParachainHost_session_info: RuntimeDescriptor<
  [index: number],
  Anonymize<I7k9oi9p83j43l> | undefined
> = "f44r7k25km7rl" as RuntimeDescriptor<[index: number], Anonymize<I7k9oi9p83j43l> | undefined>;
const runtime_ParachainHost_submit_pvf_check_statement: RuntimeDescriptor<
  [
    stmt: { accept: boolean; subject: Binary; session_index: number; validator_index: number },
    signature: Binary,
  ],
  undefined
> = "156a4rikarf9m" as RuntimeDescriptor<
  [
    stmt: { accept: boolean; subject: Binary; session_index: number; validator_index: number },
    signature: Binary,
  ],
  undefined
>;
const runtime_ParachainHost_pvfs_require_precheck: RuntimeDescriptor<
  [],
  Array<Binary>
> = "2tjq7ncksi1hj" as RuntimeDescriptor<[], Array<Binary>>;
const runtime_ParachainHost_validation_code_hash: RuntimeDescriptor<
  [para_id: number, assumption: PolkadotPrimitivesV5OccupiedCoreAssumption],
  Binary | undefined
> = "86ckgu3oplhf6" as RuntimeDescriptor<
  [para_id: number, assumption: PolkadotPrimitivesV5OccupiedCoreAssumption],
  Binary | undefined
>;
const runtime_ParachainHost_disputes: RuntimeDescriptor<
  [],
  Array<Anonymize<Irsmd7gp7po60>>
> = "cne3bdfipu9lt" as RuntimeDescriptor<[], Array<Anonymize<Irsmd7gp7po60>>>;
const runtime_ParachainHost_session_executor_params: RuntimeDescriptor<
  [session_index: number],
  Anonymize<I79n5nka727sg9> | undefined
> = "d1cjubbjip757" as RuntimeDescriptor<
  [session_index: number],
  Anonymize<I79n5nka727sg9> | undefined
>;
const runtime_ParachainHost_unapplied_slashes: RuntimeDescriptor<
  [],
  Array<Anonymize<I5ork0l271qluj>>
> = "adhvdm4odvv70" as RuntimeDescriptor<[], Array<Anonymize<I5ork0l271qluj>>>;
const runtime_ParachainHost_key_ownership_proof: RuntimeDescriptor<
  [validator_id: Binary],
  Binary | undefined
> = "51krimg7evbqt" as RuntimeDescriptor<[validator_id: Binary], Binary | undefined>;
const runtime_ParachainHost_submit_report_dispute_lost: RuntimeDescriptor<
  [
    dispute_proof: {
      time_slot: Anonymize<Ib0p1u8q0nho37>;
      kind: PolkadotPrimitivesV5SlashingSlashingOffenceKind;
      validator_index: number;
      validator_id: Binary;
    },
    key_ownership_proof: Binary,
  ],
  boolean
> = "6beaf16n6ga3p" as RuntimeDescriptor<
  [
    dispute_proof: {
      time_slot: Anonymize<Ib0p1u8q0nho37>;
      kind: PolkadotPrimitivesV5SlashingSlashingOffenceKind;
      validator_index: number;
      validator_id: Binary;
    },
    key_ownership_proof: Binary,
  ],
  boolean
>;
const runtime_ParachainHost_minimum_backing_votes: RuntimeDescriptor<[], number> =
  "5kdm82a42hnhk" as RuntimeDescriptor<[], number>;
const runtime_ParachainHost_para_backing_state: RuntimeDescriptor<
  [_: number],
  Anonymize<I53uro0ns8ba5b> | undefined
> = "96f0io5cu28uq" as RuntimeDescriptor<[_: number], Anonymize<I53uro0ns8ba5b> | undefined>;
const runtime_ParachainHost_async_backing_params: RuntimeDescriptor<
  [],
  { max_candidate_depth: number; allowed_ancestry_len: number }
> = "3o7ksjk7qbal3" as RuntimeDescriptor<
  [],
  { max_candidate_depth: number; allowed_ancestry_len: number }
>;
const runtime_ParachainHost_disabled_validators: RuntimeDescriptor<
  [],
  Array<number>
> = "64k0v4qcjg4rj" as RuntimeDescriptor<[], Array<number>>;
const runtime_ParachainHost_node_features: RuntimeDescriptor<
  [],
  { bytes: Uint8Array; bitsLen: number }
> = "e5eof49hb35c6" as RuntimeDescriptor<[], { bytes: Uint8Array; bitsLen: number }>;
const runtime_ParachainHost_approval_voting_params: RuntimeDescriptor<[], number> =
  "5kdm82a42hnhk" as RuntimeDescriptor<[], number>;
const runtime_BeefyApi_beefy_genesis: RuntimeDescriptor<[], number | undefined> =
  "2lqngfidsjia" as RuntimeDescriptor<[], number | undefined>;
const runtime_BeefyApi_validator_set: RuntimeDescriptor<[], Anonymize<I17bb7o70l1ke3> | undefined> =
  "8t9ag0905fer5" as RuntimeDescriptor<[], Anonymize<I17bb7o70l1ke3> | undefined>;
const runtime_BeefyApi_submit_report_equivocation_unsigned_extrinsic: RuntimeDescriptor<
  [
    equivocation_proof: { first: Anonymize<Ifukb31gb9emjd>; second: Anonymize<Ifukb31gb9emjd> },
    key_owner_proof: Binary,
  ],
  boolean
> = "aevemnfkodffb" as RuntimeDescriptor<
  [
    equivocation_proof: { first: Anonymize<Ifukb31gb9emjd>; second: Anonymize<Ifukb31gb9emjd> },
    key_owner_proof: Binary,
  ],
  boolean
>;
const runtime_BeefyApi_generate_key_ownership_proof: RuntimeDescriptor<
  [set_id: bigint, authority_id: Binary],
  Binary | undefined
> = "bp5tmqibnocla" as RuntimeDescriptor<
  [set_id: bigint, authority_id: Binary],
  Binary | undefined
>;
const runtime_MmrApi_mmr_root: RuntimeDescriptor<
  [],
  ResultPayload<Binary, MmrPrimitivesError>
> = "d0t5emn06irpm" as RuntimeDescriptor<[], ResultPayload<Binary, MmrPrimitivesError>>;
const runtime_MmrApi_mmr_leaf_count: RuntimeDescriptor<
  [],
  ResultPayload<bigint, MmrPrimitivesError>
> = "f67nhstfojq8e" as RuntimeDescriptor<[], ResultPayload<bigint, MmrPrimitivesError>>;
const runtime_MmrApi_generate_proof: RuntimeDescriptor<
  [block_numbers: Array<number>, best_known_block_number: number | undefined],
  ResultPayload<Anonymize<If93480lp8rssc>, MmrPrimitivesError>
> = "flul6i3tbh308" as RuntimeDescriptor<
  [block_numbers: Array<number>, best_known_block_number: number | undefined],
  ResultPayload<Anonymize<If93480lp8rssc>, MmrPrimitivesError>
>;
const runtime_MmrApi_verify_proof: RuntimeDescriptor<
  [
    leaves: Array<Binary>,
    proof: {
      leaf_indices: Anonymize<Iafqnechp3omqg>;
      leaf_count: bigint;
      items: Anonymize<Idhnf6rtqoslea>;
    },
  ],
  ResultPayload<undefined, MmrPrimitivesError>
> = "filfm8m2rpcos" as RuntimeDescriptor<
  [
    leaves: Array<Binary>,
    proof: {
      leaf_indices: Anonymize<Iafqnechp3omqg>;
      leaf_count: bigint;
      items: Anonymize<Idhnf6rtqoslea>;
    },
  ],
  ResultPayload<undefined, MmrPrimitivesError>
>;
const runtime_MmrApi_verify_proof_stateless: RuntimeDescriptor<
  [
    root: Binary,
    leaves: Array<Binary>,
    proof: {
      leaf_indices: Anonymize<Iafqnechp3omqg>;
      leaf_count: bigint;
      items: Anonymize<Idhnf6rtqoslea>;
    },
  ],
  ResultPayload<undefined, MmrPrimitivesError>
> = "7l16q2gl8950m" as RuntimeDescriptor<
  [
    root: Binary,
    leaves: Array<Binary>,
    proof: {
      leaf_indices: Anonymize<Iafqnechp3omqg>;
      leaf_count: bigint;
      items: Anonymize<Idhnf6rtqoslea>;
    },
  ],
  ResultPayload<undefined, MmrPrimitivesError>
>;
const runtime_GrandpaApi_grandpa_authorities: RuntimeDescriptor<
  [],
  Array<Anonymize<I3iuggguvi9njj>>
> = "42oj3l6qgi9il" as RuntimeDescriptor<[], Array<Anonymize<I3iuggguvi9njj>>>;
const runtime_GrandpaApi_submit_report_equivocation_unsigned_extrinsic: RuntimeDescriptor<
  [
    equivocation_proof: { set_id: bigint; equivocation: GrandpaEquivocation },
    key_owner_proof: Binary,
  ],
  boolean
> = "brn6vo9bp0vkf" as RuntimeDescriptor<
  [
    equivocation_proof: { set_id: bigint; equivocation: GrandpaEquivocation },
    key_owner_proof: Binary,
  ],
  boolean
>;
const runtime_GrandpaApi_generate_key_ownership_proof: RuntimeDescriptor<
  [set_id: bigint, authority_id: Binary],
  Binary | undefined
> = "7pgcvj2lrvoco" as RuntimeDescriptor<
  [set_id: bigint, authority_id: Binary],
  Binary | undefined
>;
const runtime_GrandpaApi_current_set_id: RuntimeDescriptor<[], bigint> =
  "b54qab7tpjjsf" as RuntimeDescriptor<[], bigint>;
const runtime_BabeApi_configuration: RuntimeDescriptor<
  [],
  {
    slot_duration: bigint;
    epoch_length: bigint;
    c: Anonymize<I2j729bmgsdiuo>;
    authorities: Anonymize<I2qinct8jq4bqe>;
    randomness: Binary;
    allowed_slots: BabeAllowedSlots;
  }
> = "2u3ba9266gu5i" as RuntimeDescriptor<
  [],
  {
    slot_duration: bigint;
    epoch_length: bigint;
    c: Anonymize<I2j729bmgsdiuo>;
    authorities: Anonymize<I2qinct8jq4bqe>;
    randomness: Binary;
    allowed_slots: BabeAllowedSlots;
  }
>;
const runtime_BabeApi_current_epoch_start: RuntimeDescriptor<[], bigint> =
  "b54qab7tpjjsf" as RuntimeDescriptor<[], bigint>;
const runtime_BabeApi_current_epoch: RuntimeDescriptor<
  [],
  {
    epoch_index: bigint;
    start_slot: bigint;
    duration: bigint;
    authorities: Anonymize<I2qinct8jq4bqe>;
    randomness: Binary;
    config: Anonymize<Idkva8q2m9meg0>;
  }
> = "67iiqsn0vc7uk" as RuntimeDescriptor<
  [],
  {
    epoch_index: bigint;
    start_slot: bigint;
    duration: bigint;
    authorities: Anonymize<I2qinct8jq4bqe>;
    randomness: Binary;
    config: Anonymize<Idkva8q2m9meg0>;
  }
>;
const runtime_BabeApi_next_epoch: RuntimeDescriptor<
  [],
  {
    epoch_index: bigint;
    start_slot: bigint;
    duration: bigint;
    authorities: Anonymize<I2qinct8jq4bqe>;
    randomness: Binary;
    config: Anonymize<Idkva8q2m9meg0>;
  }
> = "67iiqsn0vc7uk" as RuntimeDescriptor<
  [],
  {
    epoch_index: bigint;
    start_slot: bigint;
    duration: bigint;
    authorities: Anonymize<I2qinct8jq4bqe>;
    randomness: Binary;
    config: Anonymize<Idkva8q2m9meg0>;
  }
>;
const runtime_BabeApi_generate_key_ownership_proof: RuntimeDescriptor<
  [slot: bigint, authority_id: Binary],
  Binary | undefined
> = "29qdvifd7f7h8" as RuntimeDescriptor<[slot: bigint, authority_id: Binary], Binary | undefined>;
const runtime_BabeApi_submit_report_equivocation_unsigned_extrinsic: RuntimeDescriptor<
  [
    equivocation_proof: {
      offender: Binary;
      slot: bigint;
      first_header: Anonymize<I6t1nedlt7mobn>;
      second_header: Anonymize<I6t1nedlt7mobn>;
    },
    key_owner_proof: Binary,
  ],
  boolean
> = "66g8om8g9vie3" as RuntimeDescriptor<
  [
    equivocation_proof: {
      offender: Binary;
      slot: bigint;
      first_header: Anonymize<I6t1nedlt7mobn>;
      second_header: Anonymize<I6t1nedlt7mobn>;
    },
    key_owner_proof: Binary,
  ],
  boolean
>;
const runtime_AuthorityDiscoveryApi_authorities: RuntimeDescriptor<
  [],
  Array<Binary>
> = "2tjq7ncksi1hj" as RuntimeDescriptor<[], Array<Binary>>;
const runtime_SessionKeys_generate_session_keys: RuntimeDescriptor<
  [seed: Binary | undefined],
  Binary
> = "66l59qralgdd4" as RuntimeDescriptor<[seed: Binary | undefined], Binary>;
const runtime_SessionKeys_decode_session_keys: RuntimeDescriptor<
  [encoded: Binary],
  Anonymize<I4gkfq1hbsjrle> | undefined
> = "5kfkb20lni12j" as RuntimeDescriptor<[encoded: Binary], Anonymize<I4gkfq1hbsjrle> | undefined>;
const runtime_AccountNonceApi_account_nonce: RuntimeDescriptor<[account: SS58String], number> =
  "6hhhkoebr077h" as RuntimeDescriptor<[account: SS58String], number>;
const runtime_TransactionPaymentApi_query_info: RuntimeDescriptor<
  [uxt: Binary, len: number],
  { weight: Anonymize<I4q39t5hn830vp>; class: DispatchClass; partial_fee: bigint }
> = "18btbc6sp470m" as RuntimeDescriptor<
  [uxt: Binary, len: number],
  { weight: Anonymize<I4q39t5hn830vp>; class: DispatchClass; partial_fee: bigint }
>;
const runtime_TransactionPaymentApi_query_fee_details: RuntimeDescriptor<
  [uxt: Binary, len: number],
  { inclusion_fee: Anonymize<Id37fum600qfau>; tip: bigint }
> = "c4cp9g5l2rts5" as RuntimeDescriptor<
  [uxt: Binary, len: number],
  { inclusion_fee: Anonymize<Id37fum600qfau>; tip: bigint }
>;
const runtime_TransactionPaymentApi_query_weight_to_fee: RuntimeDescriptor<
  [weight: { ref_time: bigint; proof_size: bigint }],
  bigint
> = "9fha9spid57qj" as RuntimeDescriptor<
  [weight: { ref_time: bigint; proof_size: bigint }],
  bigint
>;
const runtime_TransactionPaymentApi_query_length_to_fee: RuntimeDescriptor<
  [length: number],
  bigint
> = "5a7vbbavuk0jq" as RuntimeDescriptor<[length: number], bigint>;
const runtime_BeefyMmrApi_authority_set_proof: RuntimeDescriptor<
  [],
  { id: bigint; len: number; keyset_commitment: Binary }
> = "esom3oc644tt8" as RuntimeDescriptor<
  [],
  { id: bigint; len: number; keyset_commitment: Binary }
>;
const runtime_BeefyMmrApi_next_authority_set_proof: RuntimeDescriptor<
  [],
  { id: bigint; len: number; keyset_commitment: Binary }
> = "esom3oc644tt8" as RuntimeDescriptor<
  [],
  { id: bigint; len: number; keyset_commitment: Binary }
>;
const runtime_GenesisBuilder_create_default_config: RuntimeDescriptor<[], Binary> =
  "57k61dni7tq1q" as RuntimeDescriptor<[], Binary>;
const runtime_GenesisBuilder_build_config: RuntimeDescriptor<
  [json: Binary],
  ResultPayload<undefined, string>
> = "4mqfppbv6571j" as RuntimeDescriptor<[json: Binary], ResultPayload<undefined, string>>;

type IPallets = {
  System: [
    {
      /**
       * The full account information for a particular account ID.
       */
      Account: StorageDescriptor<
        [Key: SS58String],
        {
          nonce: number;
          consumers: number;
          providers: number;
          sufficients: number;
          data: Anonymize<I1q8tnt1cluu5j>;
        },
        false
      >;

      /**
       * Total extrinsics count for the current block.
       */
      ExtrinsicCount: StorageDescriptor<[], number, true>;

      /**
       * The current weight for the block.
       */
      BlockWeight: StorageDescriptor<
        [],
        {
          normal: Anonymize<I4q39t5hn830vp>;
          operational: Anonymize<I4q39t5hn830vp>;
          mandatory: Anonymize<I4q39t5hn830vp>;
        },
        false
      >;

      /**
       * Total length (in bytes) for all extrinsics put together, for the current block.
       */
      AllExtrinsicsLen: StorageDescriptor<[], number, true>;

      /**
       * Map of block numbers to block hashes.
       */
      BlockHash: StorageDescriptor<[Key: number], Binary, false>;

      /**
       * Extrinsics data for the current block (maps an extrinsic's index to its data).
       */
      ExtrinsicData: StorageDescriptor<[Key: number], Binary, false>;

      /**
       * The current block number being processed. Set by `execute_block`.
       */
      Number: StorageDescriptor<[], number, false>;

      /**
       * Hash of the previous block.
       */
      ParentHash: StorageDescriptor<[], Binary, false>;

      /**
       * Digest of the current block, also part of the block header.
       */
      Digest: StorageDescriptor<[], Array<DigestItem>, false>;

      /**
       * Events deposited for the current block.
       *
       * NOTE: The item is unbound and should therefore never be read on chain.
       * It could otherwise inflate the PoV size of a block.
       *
       * Events have a large in-memory size. Box the events to not go out-of-memory
       * just in case someone still reads them from within the runtime.
       */
      Events: StorageDescriptor<[], Array<Anonymize<I9pt9i86qkqfgf>>, false>;

      /**
       * The number of events in the `Events<T>` list.
       */
      EventCount: StorageDescriptor<[], number, false>;

      /**
       * Mapping between a topic (represented by T::Hash) and a vector of indexes
       * of events in the `<Events<T>>` list.
       *
       * All topic vectors have deterministic storage locations depending on the topic. This
       * allows light-clients to leverage the changes trie storage tracking mechanism and
       * in case of changes fetch the list of events of interest.
       *
       * The value has the type `(BlockNumberFor<T>, EventIndex)` because if we used only just
       * the `EventIndex` then in case if the topic has the same contents on the next block
       * no notification will be triggered thus the event might be lost.
       */
      EventTopics: StorageDescriptor<[Key: Binary], Array<Anonymize<I5g2vv0ckl2m8b>>, false>;

      /**
       * Stores the `spec_version` and `spec_name` of when the last runtime upgrade happened.
       */
      LastRuntimeUpgrade: StorageDescriptor<[], { spec_version: number; spec_name: string }, true>;

      /**
       * True if we have upgraded so that `type RefCount` is `u32`. False (default) if not.
       */
      UpgradedToU32RefCount: StorageDescriptor<[], boolean, false>;

      /**
       * True if we have upgraded so that AccountInfo contains three types of `RefCount`. False
       * (default) if not.
       */
      UpgradedToTripleRefCount: StorageDescriptor<[], boolean, false>;

      /**
       * The execution phase of the block.
       */
      ExecutionPhase: StorageDescriptor<[], Phase, true>;

      /**
       * `Some` if a code upgrade has been authorized.
       */
      AuthorizedUpgrade: StorageDescriptor<[], { code_hash: Binary; check_version: boolean }, true>;
    },
    {
      /**
       *See [`Pallet::remark`].
       */
      remark: TxDescriptor<{ remark: Binary }>;

      /**
       *See [`Pallet::set_heap_pages`].
       */
      set_heap_pages: TxDescriptor<{ pages: bigint }>;

      /**
       *See [`Pallet::set_code`].
       */
      set_code: TxDescriptor<{ code: Binary }>;

      /**
       *See [`Pallet::set_code_without_checks`].
       */
      set_code_without_checks: TxDescriptor<{ code: Binary }>;

      /**
       *See [`Pallet::set_storage`].
       */
      set_storage: TxDescriptor<{ items: Anonymize<I5g1ftt6bt65bl> }>;

      /**
       *See [`Pallet::kill_storage`].
       */
      kill_storage: TxDescriptor<{ keys: Anonymize<Itom7fk49o0c9> }>;

      /**
       *See [`Pallet::kill_prefix`].
       */
      kill_prefix: TxDescriptor<{ prefix: Binary; subkeys: number }>;

      /**
       *See [`Pallet::remark_with_event`].
       */
      remark_with_event: TxDescriptor<{ remark: Binary }>;

      /**
       *See [`Pallet::authorize_upgrade`].
       */
      authorize_upgrade: TxDescriptor<{ code_hash: Binary }>;

      /**
       *See [`Pallet::authorize_upgrade_without_checks`].
       */
      authorize_upgrade_without_checks: TxDescriptor<{ code_hash: Binary }>;

      /**
       *See [`Pallet::apply_authorized_upgrade`].
       */
      apply_authorized_upgrade: TxDescriptor<{ code: Binary }>;
    },
    {
      /**
       *An extrinsic completed successfully.
       */
      ExtrinsicSuccess: PlainDescriptor<{ dispatch_info: Anonymize<Ia2iiohca2et6f> }>;

      /**
       *An extrinsic failed.
       */
      ExtrinsicFailed: PlainDescriptor<{
        dispatch_error: DispatchError;
        dispatch_info: Anonymize<Ia2iiohca2et6f>;
      }>;

      /**
       *`:code` was updated.
       */
      CodeUpdated: PlainDescriptor<undefined>;

      /**
       *A new account was created.
       */
      NewAccount: PlainDescriptor<{ account: SS58String }>;

      /**
       *An account was reaped.
       */
      KilledAccount: PlainDescriptor<{ account: SS58String }>;

      /**
       *On on-chain remark happened.
       */
      Remarked: PlainDescriptor<{ sender: SS58String; hash: Binary }>;

      /**
       *An upgrade was authorized.
       */
      UpgradeAuthorized: PlainDescriptor<{ code_hash: Binary; check_version: boolean }>;
    },
    {
      /**
       *The name of specification does not match between the current runtime
       *and the new runtime.
       */
      InvalidSpecName: PlainDescriptor<undefined>;

      /**
       *The specification version is not allowed to decrease between the current runtime
       *and the new runtime.
       */
      SpecVersionNeedsToIncrease: PlainDescriptor<undefined>;

      /**
       *Failed to extract the runtime version from the new runtime.
       *
       *Either calling `Core_version` or decoding `RuntimeVersion` failed.
       */
      FailedToExtractRuntimeVersion: PlainDescriptor<undefined>;

      /**
       *Suicide called when the account has non-default composite data.
       */
      NonDefaultComposite: PlainDescriptor<undefined>;

      /**
       *There is a non-zero reference count preventing the account from being purged.
       */
      NonZeroRefCount: PlainDescriptor<undefined>;

      /**
       *The origin filter prevent the call to be dispatched.
       */
      CallFiltered: PlainDescriptor<undefined>;

      /**
       *No upgrade authorized.
       */
      NothingAuthorized: PlainDescriptor<undefined>;

      /**
       *The submitted code is not authorized.
       */
      Unauthorized: PlainDescriptor<undefined>;
    },
    {
      /**
       * Block & extrinsics weights: base values and limits.
       */
      BlockWeights: PlainDescriptor<{
        base_block: Anonymize<I4q39t5hn830vp>;
        max_block: Anonymize<I4q39t5hn830vp>;
        per_class: Anonymize<I79te2qqsklnbd>;
      }>;

      /**
       * The maximum length of a block (in bytes).
       */
      BlockLength: PlainDescriptor<{ normal: number; operational: number; mandatory: number }>;

      /**
       * Maximum number of block number to block hash mappings to keep (oldest pruned first).
       */
      BlockHashCount: PlainDescriptor<number>;

      /**
       * The weight of runtime database operations the runtime can invoke.
       */
      DbWeight: PlainDescriptor<{ read: bigint; write: bigint }>;

      /**
       * Get the chain's current version.
       */
      Version: PlainDescriptor<{
        spec_name: string;
        impl_name: string;
        authoring_version: number;
        spec_version: number;
        impl_version: number;
        apis: Anonymize<I1st1p92iu8h7e>;
        transaction_version: number;
        state_version: number;
      }>;

      /**
       * The designated SS58 prefix of this chain.
       *
       * This replaces the "ss58Format" property declared in the chain spec. Reason is
       * that the runtime should know about the prefix in order to make use of it as
       * an identifier of the chain.
       */
      SS58Prefix: PlainDescriptor<number>;
    },
  ];
  Babe: [
    {
      /**
       * Current epoch index.
       */
      EpochIndex: StorageDescriptor<[], bigint, false>;

      /**
       * Current epoch authorities.
       */
      Authorities: StorageDescriptor<[], Array<Anonymize<I3iuggguvi9njj>>, false>;

      /**
       * The slot at which the first epoch actually started. This is 0
       * until the first block of the chain.
       */
      GenesisSlot: StorageDescriptor<[], bigint, false>;

      /**
       * Current slot number.
       */
      CurrentSlot: StorageDescriptor<[], bigint, false>;

      /**
       * The epoch randomness for the *current* epoch.
       *
       * # Security
       *
       * This MUST NOT be used for gambling, as it can be influenced by a
       * malicious validator in the short term. It MAY be used in many
       * cryptographic protocols, however, so long as one remembers that this
       * (like everything else on-chain) it is public. For example, it can be
       * used where a number is needed that cannot have been chosen by an
       * adversary, for purposes such as public-coin zero-knowledge proofs.
       */
      Randomness: StorageDescriptor<[], Binary, false>;

      /**
       * Pending epoch configuration change that will be applied when the next epoch is enacted.
       */
      PendingEpochConfigChange: StorageDescriptor<[], BabeDigestsNextConfigDescriptor, true>;

      /**
       * Next epoch randomness.
       */
      NextRandomness: StorageDescriptor<[], Binary, false>;

      /**
       * Next epoch authorities.
       */
      NextAuthorities: StorageDescriptor<[], Array<Anonymize<I3iuggguvi9njj>>, false>;

      /**
       * Randomness under construction.
       *
       * We make a trade-off between storage accesses and list length.
       * We store the under-construction randomness in segments of up to
       * `UNDER_CONSTRUCTION_SEGMENT_LENGTH`.
       *
       * Once a segment reaches this length, we begin the next one.
       * We reset all segments and return to `0` at the beginning of every
       * epoch.
       */
      SegmentIndex: StorageDescriptor<[], number, false>;

      /**
       * TWOX-NOTE: `SegmentIndex` is an increasing integer, so this is okay.
       */
      UnderConstruction: StorageDescriptor<[Key: number], Array<Binary>, false>;

      /**
       * Temporary value (cleared at block finalization) which is `Some`
       * if per-block initialization has already been called for current block.
       */
      Initialized: StorageDescriptor<[], BabeDigestsPreDigest | undefined, true>;

      /**
       * This field should always be populated during block processing unless
       * secondary plain slots are enabled (which don't contain a VRF output).
       *
       * It is set in `on_finalize`, before it will contain the value from the last block.
       */
      AuthorVrfRandomness: StorageDescriptor<[], Binary | undefined, false>;

      /**
       * The block numbers when the last and current epoch have started, respectively `N-1` and
       * `N`.
       * NOTE: We track this is in order to annotate the block number when a given pool of
       * entropy was fixed (i.e. it was known to chain observers). Since epochs are defined in
       * slots, which may be skipped, the block numbers may not line up with the slot numbers.
       */
      EpochStart: StorageDescriptor<[], [number, number], false>;

      /**
       * How late the current block is compared to its parent.
       *
       * This entry is populated as part of block execution and is cleaned up
       * on block finalization. Querying this storage entry outside of block
       * execution context should always yield zero.
       */
      Lateness: StorageDescriptor<[], number, false>;

      /**
       * The configuration for the current epoch. Should never be `None` as it is initialized in
       * genesis.
       */
      EpochConfig: StorageDescriptor<
        [],
        { c: Anonymize<I2j729bmgsdiuo>; allowed_slots: BabeAllowedSlots },
        true
      >;

      /**
       * The configuration for the next epoch, `None` if the config will not change
       * (you can fallback to `EpochConfig` instead in that case).
       */
      NextEpochConfig: StorageDescriptor<
        [],
        { c: Anonymize<I2j729bmgsdiuo>; allowed_slots: BabeAllowedSlots },
        true
      >;

      /**
       * A list of the last 100 skipped epochs and the corresponding session index
       * when the epoch was skipped.
       *
       * This is only used for validating equivocation proofs. An equivocation proof
       * must contains a key-ownership proof for a given session, therefore we need a
       * way to tie together sessions and epoch indices, i.e. we need to validate that
       * a validator was the owner of a given key on a given session, and what the
       * active epoch index was during that session.
       */
      SkippedEpochs: StorageDescriptor<[], Array<Anonymize<I6cs1itejju2vv>>, false>;
    },
    {
      /**
       *See [`Pallet::report_equivocation`].
       */
      report_equivocation: TxDescriptor<{
        equivocation_proof: Anonymize<I7bek4s9acs8nl>;
        key_owner_proof: Anonymize<I3ia7aufsoj0l1>;
      }>;

      /**
       *See [`Pallet::report_equivocation_unsigned`].
       */
      report_equivocation_unsigned: TxDescriptor<{
        equivocation_proof: Anonymize<I7bek4s9acs8nl>;
        key_owner_proof: Anonymize<I3ia7aufsoj0l1>;
      }>;

      /**
       *See [`Pallet::plan_config_change`].
       */
      plan_config_change: TxDescriptor<{ config: BabeDigestsNextConfigDescriptor }>;
    },
    {},
    {
      /**
       *An equivocation proof provided as part of an equivocation report is invalid.
       */
      InvalidEquivocationProof: PlainDescriptor<undefined>;

      /**
       *A key ownership proof provided as part of an equivocation report is invalid.
       */
      InvalidKeyOwnershipProof: PlainDescriptor<undefined>;

      /**
       *A given equivocation report is valid but already previously reported.
       */
      DuplicateOffenceReport: PlainDescriptor<undefined>;

      /**
       *Submitted configuration is invalid.
       */
      InvalidConfiguration: PlainDescriptor<undefined>;
    },
    {
      /**
       * The amount of time, in slots, that each epoch should last.
       * NOTE: Currently it is not possible to change the epoch duration after
       * the chain has started. Attempting to do so will brick block production.
       */
      EpochDuration: PlainDescriptor<bigint>;

      /**
       * The expected average block time at which BABE should be creating
       * blocks. Since BABE is probabilistic it is not trivial to figure out
       * what the expected average block time should be based on the slot
       * duration and the security parameter `c` (where `1 - c` represents
       * the probability of a slot being empty).
       */
      ExpectedBlockTime: PlainDescriptor<bigint>;

      /**
       * Max number of authorities allowed
       */
      MaxAuthorities: PlainDescriptor<number>;

      /**
       * The maximum number of nominators for each validator.
       */
      MaxNominators: PlainDescriptor<number>;
    },
  ];
  Timestamp: [
    {
      /**
       * The current time for the current block.
       */
      Now: StorageDescriptor<[], bigint, false>;

      /**
       * Whether the timestamp has been updated in this block.
       *
       * This value is updated to `true` upon successful submission of a timestamp by a node.
       * It is then checked at the end of each block execution in the `on_finalize` hook.
       */
      DidUpdate: StorageDescriptor<[], boolean, false>;
    },
    {
      /**
       *See [`Pallet::set`].
       */
      set: TxDescriptor<{ now: bigint }>;
    },
    {},
    {},
    {
      /**
       * The minimum period between blocks.
       *
       * Be aware that this is different to the *expected* period that the block production
       * apparatus provides. Your chosen consensus system will generally work with this to
       * determine a sensible block time. For example, in the Aura pallet it will be double this
       * period on default settings.
       */
      MinimumPeriod: PlainDescriptor<bigint>;
    },
  ];
  Indices: [
    {
      /**
       * The lookup from index to account.
       */
      Accounts: StorageDescriptor<[Key: number], [SS58String, bigint, boolean], true>;
    },
    {
      /**
       *See [`Pallet::claim`].
       */
      claim: TxDescriptor<{ index: number }>;

      /**
       *See [`Pallet::transfer`].
       */
      transfer: TxDescriptor<{ new: MultiAddress; index: number }>;

      /**
       *See [`Pallet::free`].
       */
      free: TxDescriptor<{ index: number }>;

      /**
       *See [`Pallet::force_transfer`].
       */
      force_transfer: TxDescriptor<{ new: MultiAddress; index: number; freeze: boolean }>;

      /**
       *See [`Pallet::freeze`].
       */
      freeze: TxDescriptor<{ index: number }>;
    },
    {
      /**
       *A account index was assigned.
       */
      IndexAssigned: PlainDescriptor<{ who: SS58String; index: number }>;

      /**
       *A account index has been freed up (unassigned).
       */
      IndexFreed: PlainDescriptor<{ index: number }>;

      /**
       *A account index has been frozen to its current account ID.
       */
      IndexFrozen: PlainDescriptor<{ who: SS58String; index: number }>;
    },
    {
      /**
       *The index was not already assigned.
       */
      NotAssigned: PlainDescriptor<undefined>;

      /**
       *The index is assigned to another account.
       */
      NotOwner: PlainDescriptor<undefined>;

      /**
       *The index was not available.
       */
      InUse: PlainDescriptor<undefined>;

      /**
       *The source and destination accounts are identical.
       */
      NotTransfer: PlainDescriptor<undefined>;

      /**
       *The index is permanent and may not be freed/changed.
       */
      Permanent: PlainDescriptor<undefined>;
    },
    {
      /**
       * The deposit needed for reserving an index.
       */
      Deposit: PlainDescriptor<bigint>;
    },
  ];
  Balances: [
    {
      /**
       * The total units issued in the system.
       */
      TotalIssuance: StorageDescriptor<[], bigint, false>;

      /**
       * The total units of outstanding deactivated balance in the system.
       */
      InactiveIssuance: StorageDescriptor<[], bigint, false>;

      /**
       * The Balances pallet example of storing the balance of an account.
       *
       * # Example
       *
       * ```nocompile
       *  impl pallet_balances::Config for Runtime {
       *    type AccountStore = StorageMapShim<Self::Account<Runtime>, frame_system::Provider<Runtime>, AccountId, Self::AccountData<Balance>>
       *  }
       * ```
       *
       * You can also store the balance of an account in the `System` pallet.
       *
       * # Example
       *
       * ```nocompile
       *  impl pallet_balances::Config for Runtime {
       *   type AccountStore = System
       *  }
       * ```
       *
       * But this comes with tradeoffs, storing account balances in the system pallet stores
       * `frame_system` data alongside the account data contrary to storing account balances in the
       * `Balances` pallet, which uses a `StorageMap` to store balances data only.
       * NOTE: This is only used in the case that this pallet is used to store balances.
       */
      Account: StorageDescriptor<
        [Key: SS58String],
        { free: bigint; reserved: bigint; frozen: bigint; flags: bigint },
        false
      >;

      /**
       * Any liquidity locks on some account balances.
       * NOTE: Should only be accessed when setting, changing and freeing a lock.
       */
      Locks: StorageDescriptor<[Key: SS58String], Array<Anonymize<I5b29v4qfq4tu7>>, false>;

      /**
       * Named reserves on some account balances.
       */
      Reserves: StorageDescriptor<[Key: SS58String], Array<Anonymize<I32btm6htd9bck>>, false>;

      /**
       * Holds on account balances.
       */
      Holds: StorageDescriptor<[Key: SS58String], Array<Anonymize<Iemrlqc7oc4k5g>>, false>;

      /**
       * Freeze locks on account balances.
       */
      Freezes: StorageDescriptor<[Key: SS58String], Array<Anonymize<I7qdm60946h5u9>>, false>;
    },
    {
      /**
       *See [`Pallet::transfer_allow_death`].
       */
      transfer_allow_death: TxDescriptor<{ dest: MultiAddress; value: bigint }>;

      /**
       *See [`Pallet::force_transfer`].
       */
      force_transfer: TxDescriptor<{ source: MultiAddress; dest: MultiAddress; value: bigint }>;

      /**
       *See [`Pallet::transfer_keep_alive`].
       */
      transfer_keep_alive: TxDescriptor<{ dest: MultiAddress; value: bigint }>;

      /**
       *See [`Pallet::transfer_all`].
       */
      transfer_all: TxDescriptor<{ dest: MultiAddress; keep_alive: boolean }>;

      /**
       *See [`Pallet::force_unreserve`].
       */
      force_unreserve: TxDescriptor<{ who: MultiAddress; amount: bigint }>;

      /**
       *See [`Pallet::upgrade_accounts`].
       */
      upgrade_accounts: TxDescriptor<{ who: Anonymize<Ia2lhg7l2hilo3> }>;

      /**
       *See [`Pallet::force_set_balance`].
       */
      force_set_balance: TxDescriptor<{ who: MultiAddress; new_free: bigint }>;

      /**
       *See [`Pallet::force_adjust_total_issuance`].
       */
      force_adjust_total_issuance: TxDescriptor<{
        direction: BalancesTypesAdjustmentDirection;
        delta: bigint;
      }>;
    },
    {
      /**
       *An account was created with some free balance.
       */
      Endowed: PlainDescriptor<{ account: SS58String; free_balance: bigint }>;

      /**
       *An account was removed whose balance was non-zero but below ExistentialDeposit,
       *resulting in an outright loss.
       */
      DustLost: PlainDescriptor<{ account: SS58String; amount: bigint }>;

      /**
       *Transfer succeeded.
       */
      Transfer: PlainDescriptor<{ from: SS58String; to: SS58String; amount: bigint }>;

      /**
       *A balance was set by root.
       */
      BalanceSet: PlainDescriptor<{ who: SS58String; free: bigint }>;

      /**
       *Some balance was reserved (moved from free to reserved).
       */
      Reserved: PlainDescriptor<{ who: SS58String; amount: bigint }>;

      /**
       *Some balance was unreserved (moved from reserved to free).
       */
      Unreserved: PlainDescriptor<{ who: SS58String; amount: bigint }>;

      /**
       *Some balance was moved from the reserve of the first account to the second account.
       *Final argument indicates the destination balance type.
       */
      ReserveRepatriated: PlainDescriptor<{
        from: SS58String;
        to: SS58String;
        amount: bigint;
        destination_status: BalanceStatus;
      }>;

      /**
       *Some amount was deposited (e.g. for transaction fees).
       */
      Deposit: PlainDescriptor<{ who: SS58String; amount: bigint }>;

      /**
       *Some amount was withdrawn from the account (e.g. for transaction fees).
       */
      Withdraw: PlainDescriptor<{ who: SS58String; amount: bigint }>;

      /**
       *Some amount was removed from the account (e.g. for misbehavior).
       */
      Slashed: PlainDescriptor<{ who: SS58String; amount: bigint }>;

      /**
       *Some amount was minted into an account.
       */
      Minted: PlainDescriptor<{ who: SS58String; amount: bigint }>;

      /**
       *Some amount was burned from an account.
       */
      Burned: PlainDescriptor<{ who: SS58String; amount: bigint }>;

      /**
       *Some amount was suspended from an account (it can be restored later).
       */
      Suspended: PlainDescriptor<{ who: SS58String; amount: bigint }>;

      /**
       *Some amount was restored into an account.
       */
      Restored: PlainDescriptor<{ who: SS58String; amount: bigint }>;

      /**
       *An account was upgraded.
       */
      Upgraded: PlainDescriptor<{ who: SS58String }>;

      /**
       *Total issuance was increased by `amount`, creating a credit to be balanced.
       */
      Issued: PlainDescriptor<{ amount: bigint }>;

      /**
       *Total issuance was decreased by `amount`, creating a debt to be balanced.
       */
      Rescinded: PlainDescriptor<{ amount: bigint }>;

      /**
       *Some balance was locked.
       */
      Locked: PlainDescriptor<{ who: SS58String; amount: bigint }>;

      /**
       *Some balance was unlocked.
       */
      Unlocked: PlainDescriptor<{ who: SS58String; amount: bigint }>;

      /**
       *Some balance was frozen.
       */
      Frozen: PlainDescriptor<{ who: SS58String; amount: bigint }>;

      /**
       *Some balance was thawed.
       */
      Thawed: PlainDescriptor<{ who: SS58String; amount: bigint }>;

      /**
       *The `TotalIssuance` was forcefully changed.
       */
      TotalIssuanceForced: PlainDescriptor<{ old: bigint; new: bigint }>;
    },
    {
      /**
       *Vesting balance too high to send value.
       */
      VestingBalance: PlainDescriptor<undefined>;

      /**
       *Account liquidity restrictions prevent withdrawal.
       */
      LiquidityRestrictions: PlainDescriptor<undefined>;

      /**
       *Balance too low to send value.
       */
      InsufficientBalance: PlainDescriptor<undefined>;

      /**
       *Value too low to create account due to existential deposit.
       */
      ExistentialDeposit: PlainDescriptor<undefined>;

      /**
       *Transfer/payment would kill account.
       */
      Expendability: PlainDescriptor<undefined>;

      /**
       *A vesting schedule already exists for this account.
       */
      ExistingVestingSchedule: PlainDescriptor<undefined>;

      /**
       *Beneficiary account must pre-exist.
       */
      DeadAccount: PlainDescriptor<undefined>;

      /**
       *Number of named reserves exceed `MaxReserves`.
       */
      TooManyReserves: PlainDescriptor<undefined>;

      /**
       *Number of holds exceed `VariantCountOf<T::RuntimeHoldReason>`.
       */
      TooManyHolds: PlainDescriptor<undefined>;

      /**
       *Number of freezes exceed `MaxFreezes`.
       */
      TooManyFreezes: PlainDescriptor<undefined>;

      /**
       *The issuance cannot be modified since it is already deactivated.
       */
      IssuanceDeactivated: PlainDescriptor<undefined>;

      /**
       *The delta cannot be zero.
       */
      DeltaZero: PlainDescriptor<undefined>;
    },
    {
      /**
       * The minimum amount required to keep an account open. MUST BE GREATER THAN ZERO!
       *
       * If you *really* need it to be zero, you can enable the feature `insecure_zero_ed` for
       * this pallet. However, you do so at your own risk: this will open up a major DoS vector.
       * In case you have multiple sources of provider references, you may also get unexpected
       * behaviour if you set this to zero.
       *
       * Bottom line: Do yourself a favour and make it at least one!
       */
      ExistentialDeposit: PlainDescriptor<bigint>;

      /**
       * The maximum number of locks that should exist on an account.
       * Not strictly enforced, but used for weight estimation.
       */
      MaxLocks: PlainDescriptor<number>;

      /**
       * The maximum number of named reserves that can exist on an account.
       */
      MaxReserves: PlainDescriptor<number>;

      /**
       * The maximum number of individual freeze locks that can exist on an account at any time.
       */
      MaxFreezes: PlainDescriptor<number>;
    },
  ];
  TransactionPayment: [
    {
      /**

 */
      NextFeeMultiplier: StorageDescriptor<[], bigint, false>;

      /**

 */
      StorageVersion: StorageDescriptor<[], TransactionPaymentReleases, false>;
    },
    {},
    {
      /**
       *A transaction fee `actual_fee`, of which `tip` was added to the minimum inclusion fee,
       *has been paid by `who`.
       */
      TransactionFeePaid: PlainDescriptor<{ who: SS58String; actual_fee: bigint; tip: bigint }>;
    },
    {},
    {
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
       */
      OperationalFeeMultiplier: PlainDescriptor<number>;
    },
  ];
  Authorship: [
    {
      /**
       * Author of current block.
       */
      Author: StorageDescriptor<[], SS58String, true>;
    },
    {},
    {},
    {},
    {},
  ];
  Offences: [
    {
      /**
       * The primary structure that holds all offence records keyed by report identifiers.
       */
      Reports: StorageDescriptor<
        [Key: Binary],
        { offender: Anonymize<Iidji8iemeki4>; reporters: Anonymize<Ia2lhg7l2hilo3> },
        true
      >;

      /**
       * A vector of reports of the same kind that happened at the same time slot.
       */
      ConcurrentReportsIndex: StorageDescriptor<[Binary, Binary], Array<Binary>, false>;
    },
    {},
    {
      /**
       *There is an offence reported of the given `kind` happened at the `session_index` and
       *(kind-specific) time slot. This event is not deposited for duplicate slashes.
       *\[kind, timeslot\].
       */
      Offence: PlainDescriptor<{ kind: Binary; timeslot: Binary }>;
    },
    {},
    {},
  ];
  Historical: [
    {
      /**
       * Mapping from historical session indices to session-data root hash and validator count.
       */
      HistoricalSessions: StorageDescriptor<[Key: number], [Binary, number], true>;

      /**
       * The range of historical sessions we store. [first, last)
       */
      StoredRange: StorageDescriptor<[], [number, number], true>;
    },
    {},
    {},
    {},
    {},
  ];
  Session: [
    {
      /**
       * The current set of validators.
       */
      Validators: StorageDescriptor<[], Array<SS58String>, false>;

      /**
       * Current index of the session.
       */
      CurrentIndex: StorageDescriptor<[], number, false>;

      /**
       * True if the underlying economic identities or weighting behind the validators
       * has changed in the queued validator set.
       */
      QueuedChanged: StorageDescriptor<[], boolean, false>;

      /**
       * The queued keys for the next session. When the next session begins, these keys
       * will be used to determine the validator's session keys.
       */
      QueuedKeys: StorageDescriptor<[], Array<Anonymize<I9mnddod4k7nfj>>, false>;

      /**
       * Indices of disabled validators.
       *
       * The vec is always kept sorted so that we can find whether a given validator is
       * disabled using binary search. It gets cleared when `on_session_ending` returns
       * a new set of identities.
       */
      DisabledValidators: StorageDescriptor<[], Array<number>, false>;

      /**
       * The next session keys for a validator.
       */
      NextKeys: StorageDescriptor<
        [Key: SS58String],
        {
          grandpa: Binary;
          babe: Binary;
          para_validator: Binary;
          para_assignment: Binary;
          authority_discovery: Binary;
          beefy: Binary;
        },
        true
      >;

      /**
       * The owner of a key. The key is the `KeyTypeId` + the encoded key.
       */
      KeyOwner: StorageDescriptor<[Key: [Binary, Binary]], SS58String, true>;
    },
    {
      /**
       *See [`Pallet::set_keys`].
       */
      set_keys: TxDescriptor<{ keys: Anonymize<Ieup5s9ka3brpi>; proof: Binary }>;

      /**
       *See [`Pallet::purge_keys`].
       */
      purge_keys: TxDescriptor<undefined>;
    },
    {
      /**
       *New session has happened. Note that the argument is the session index, not the
       *block number as the type might suggest.
       */
      NewSession: PlainDescriptor<{ session_index: number }>;
    },
    {
      /**
       *Invalid ownership proof.
       */
      InvalidProof: PlainDescriptor<undefined>;

      /**
       *No associated validator ID for account.
       */
      NoAssociatedValidatorId: PlainDescriptor<undefined>;

      /**
       *Registered duplicate key.
       */
      DuplicatedKey: PlainDescriptor<undefined>;

      /**
       *No keys are associated with this account.
       */
      NoKeys: PlainDescriptor<undefined>;

      /**
       *Key setting account is not live, so it's impossible to associate keys.
       */
      NoAccount: PlainDescriptor<undefined>;
    },
    {},
  ];
  Grandpa: [
    {
      /**
       * State of the current authority set.
       */
      State: StorageDescriptor<[], GrandpaStoredState, false>;

      /**
       * Pending change: (signaled at, scheduled change).
       */
      PendingChange: StorageDescriptor<
        [],
        {
          scheduled_at: number;
          delay: number;
          next_authorities: Anonymize<I2qinct8jq4bqe>;
          forced: Anonymize<I4arjljr6dpflb>;
        },
        true
      >;

      /**
       * next block number where we can force a change.
       */
      NextForced: StorageDescriptor<[], number, true>;

      /**
       * `true` if we are currently stalled.
       */
      Stalled: StorageDescriptor<[], [number, number], true>;

      /**
       * The number of changes (both in terms of keys and underlying economic responsibilities)
       * in the "set" of Grandpa validators from genesis.
       */
      CurrentSetId: StorageDescriptor<[], bigint, false>;

      /**
       * A mapping from grandpa set ID to the index of the *most recent* session for which its
       * members were responsible.
       *
       * This is only used for validating equivocation proofs. An equivocation proof must
       * contains a key-ownership proof for a given session, therefore we need a way to tie
       * together sessions and GRANDPA set ids, i.e. we need to validate that a validator
       * was the owner of a given key on a given session, and what the active set ID was
       * during that session.
       *
       * TWOX-NOTE: `SetId` is not under user control.
       */
      SetIdSession: StorageDescriptor<[Key: bigint], number, true>;

      /**
       * The current list of authorities.
       */
      Authorities: StorageDescriptor<[], Array<Anonymize<I3iuggguvi9njj>>, false>;
    },
    {
      /**
       *See [`Pallet::report_equivocation`].
       */
      report_equivocation: TxDescriptor<{
        equivocation_proof: Anonymize<I95fr9lj1pb8v8>;
        key_owner_proof: Anonymize<I3ia7aufsoj0l1>;
      }>;

      /**
       *See [`Pallet::report_equivocation_unsigned`].
       */
      report_equivocation_unsigned: TxDescriptor<{
        equivocation_proof: Anonymize<I95fr9lj1pb8v8>;
        key_owner_proof: Anonymize<I3ia7aufsoj0l1>;
      }>;

      /**
       *See [`Pallet::note_stalled`].
       */
      note_stalled: TxDescriptor<{ delay: number; best_finalized_block_number: number }>;
    },
    {
      /**
       *New authority set has been applied.
       */
      NewAuthorities: PlainDescriptor<{ authority_set: Anonymize<I2qinct8jq4bqe> }>;

      /**
       *Current authority set has been paused.
       */
      Paused: PlainDescriptor<undefined>;

      /**
       *Current authority set has been resumed.
       */
      Resumed: PlainDescriptor<undefined>;
    },
    {
      /**
       *Attempt to signal GRANDPA pause when the authority set isn't live
       *(either paused or already pending pause).
       */
      PauseFailed: PlainDescriptor<undefined>;

      /**
       *Attempt to signal GRANDPA resume when the authority set isn't paused
       *(either live or already pending resume).
       */
      ResumeFailed: PlainDescriptor<undefined>;

      /**
       *Attempt to signal GRANDPA change with one already pending.
       */
      ChangePending: PlainDescriptor<undefined>;

      /**
       *Cannot signal forced change so soon after last.
       */
      TooSoon: PlainDescriptor<undefined>;

      /**
       *A key ownership proof provided as part of an equivocation report is invalid.
       */
      InvalidKeyOwnershipProof: PlainDescriptor<undefined>;

      /**
       *An equivocation proof provided as part of an equivocation report is invalid.
       */
      InvalidEquivocationProof: PlainDescriptor<undefined>;

      /**
       *A given equivocation report is valid but already previously reported.
       */
      DuplicateOffenceReport: PlainDescriptor<undefined>;
    },
    {
      /**
       * Max Authorities in use
       */
      MaxAuthorities: PlainDescriptor<number>;

      /**
       * The maximum number of nominators for each validator.
       */
      MaxNominators: PlainDescriptor<number>;

      /**
       * The maximum number of entries to keep in the set id to session index mapping.
       *
       * Since the `SetIdSession` map is only used for validating equivocations this
       * value should relate to the bonding duration of whatever staking system is
       * being used (if any). If equivocation handling is not enabled then this value
       * can be zero.
       */
      MaxSetIdSessionEntries: PlainDescriptor<bigint>;
    },
  ];
  AuthorityDiscovery: [
    {
      /**
       * Keys of the current authority set.
       */
      Keys: StorageDescriptor<[], Array<Binary>, false>;

      /**
       * Keys of the next authority set.
       */
      NextKeys: StorageDescriptor<[], Array<Binary>, false>;
    },
    {},
    {},
    {},
    {},
  ];
  Treasury: [
    {
      /**
       * Number of proposals that have been made.
       */
      ProposalCount: StorageDescriptor<[], number, false>;

      /**
       * Proposals that have been made.
       */
      Proposals: StorageDescriptor<
        [Key: number],
        { proposer: SS58String; value: bigint; beneficiary: SS58String; bond: bigint },
        true
      >;

      /**
       * The amount which has been reported as inactive to Currency.
       */
      Deactivated: StorageDescriptor<[], bigint, false>;

      /**
       * Proposal indices that have been approved but not yet awarded.
       */
      Approvals: StorageDescriptor<[], Array<number>, false>;

      /**
       * The count of spends that have been made.
       */
      SpendCount: StorageDescriptor<[], number, false>;

      /**
       * Spends that have been approved and being processed.
       */
      Spends: StorageDescriptor<
        [Key: number],
        {
          asset_kind: PolkadotRuntimeCommonImplsVersionedLocatableAsset;
          amount: bigint;
          beneficiary: XcmVersionedLocation;
          valid_from: number;
          expire_at: number;
          status: TreasuryPaymentState;
        },
        true
      >;
    },
    {
      /**
       *See [`Pallet::propose_spend`].
       */
      propose_spend: TxDescriptor<{ value: bigint; beneficiary: MultiAddress }>;

      /**
       *See [`Pallet::reject_proposal`].
       */
      reject_proposal: TxDescriptor<{ proposal_id: number }>;

      /**
       *See [`Pallet::approve_proposal`].
       */
      approve_proposal: TxDescriptor<{ proposal_id: number }>;

      /**
       *See [`Pallet::spend_local`].
       */
      spend_local: TxDescriptor<{ amount: bigint; beneficiary: MultiAddress }>;

      /**
       *See [`Pallet::remove_approval`].
       */
      remove_approval: TxDescriptor<{ proposal_id: number }>;

      /**
       *See [`Pallet::spend`].
       */
      spend: TxDescriptor<{
        asset_kind: PolkadotRuntimeCommonImplsVersionedLocatableAsset;
        amount: bigint;
        beneficiary: XcmVersionedLocation;
        valid_from: Anonymize<I4arjljr6dpflb>;
      }>;

      /**
       *See [`Pallet::payout`].
       */
      payout: TxDescriptor<{ index: number }>;

      /**
       *See [`Pallet::check_status`].
       */
      check_status: TxDescriptor<{ index: number }>;

      /**
       *See [`Pallet::void_spend`].
       */
      void_spend: TxDescriptor<{ index: number }>;
    },
    {
      /**
       *New proposal.
       */
      Proposed: PlainDescriptor<{ proposal_index: number }>;

      /**
       *We have ended a spend period and will now allocate funds.
       */
      Spending: PlainDescriptor<{ budget_remaining: bigint }>;

      /**
       *Some funds have been allocated.
       */
      Awarded: PlainDescriptor<{ proposal_index: number; award: bigint; account: SS58String }>;

      /**
       *A proposal was rejected; funds were slashed.
       */
      Rejected: PlainDescriptor<{ proposal_index: number; slashed: bigint }>;

      /**
       *Some of our funds have been burnt.
       */
      Burnt: PlainDescriptor<{ burnt_funds: bigint }>;

      /**
       *Spending has finished; this is the amount that rolls over until next spend.
       */
      Rollover: PlainDescriptor<{ rollover_balance: bigint }>;

      /**
       *Some funds have been deposited.
       */
      Deposit: PlainDescriptor<{ value: bigint }>;

      /**
       *A new spend proposal has been approved.
       */
      SpendApproved: PlainDescriptor<{
        proposal_index: number;
        amount: bigint;
        beneficiary: SS58String;
      }>;

      /**
       *The inactive funds of the pallet have been updated.
       */
      UpdatedInactive: PlainDescriptor<{ reactivated: bigint; deactivated: bigint }>;

      /**
       *A new asset spend proposal has been approved.
       */
      AssetSpendApproved: PlainDescriptor<{
        index: number;
        asset_kind: PolkadotRuntimeCommonImplsVersionedLocatableAsset;
        amount: bigint;
        beneficiary: XcmVersionedLocation;
        valid_from: number;
        expire_at: number;
      }>;

      /**
       *An approved spend was voided.
       */
      AssetSpendVoided: PlainDescriptor<{ index: number }>;

      /**
       *A payment happened.
       */
      Paid: PlainDescriptor<{ index: number; payment_id: bigint }>;

      /**
       *A payment failed and can be retried.
       */
      PaymentFailed: PlainDescriptor<{ index: number; payment_id: bigint }>;

      /**
       *A spend was processed and removed from the storage. It might have been successfully
       *paid or it may have expired.
       */
      SpendProcessed: PlainDescriptor<{ index: number }>;
    },
    {
      /**
       *Proposer's balance is too low.
       */
      InsufficientProposersBalance: PlainDescriptor<undefined>;

      /**
       *No proposal, bounty or spend at that index.
       */
      InvalidIndex: PlainDescriptor<undefined>;

      /**
       *Too many approvals in the queue.
       */
      TooManyApprovals: PlainDescriptor<undefined>;

      /**
       *The spend origin is valid but the amount it is allowed to spend is lower than the
       *amount to be spent.
       */
      InsufficientPermission: PlainDescriptor<undefined>;

      /**
       *Proposal has not been approved.
       */
      ProposalNotApproved: PlainDescriptor<undefined>;

      /**
       *The balance of the asset kind is not convertible to the balance of the native asset.
       */
      FailedToConvertBalance: PlainDescriptor<undefined>;

      /**
       *The spend has expired and cannot be claimed.
       */
      SpendExpired: PlainDescriptor<undefined>;

      /**
       *The spend is not yet eligible for payout.
       */
      EarlyPayout: PlainDescriptor<undefined>;

      /**
       *The payment has already been attempted.
       */
      AlreadyAttempted: PlainDescriptor<undefined>;

      /**
       *There was some issue with the mechanism of payment.
       */
      PayoutError: PlainDescriptor<undefined>;

      /**
       *The payout was not yet attempted/claimed.
       */
      NotAttempted: PlainDescriptor<undefined>;

      /**
       *The payment has neither failed nor succeeded yet.
       */
      Inconclusive: PlainDescriptor<undefined>;
    },
    {
      /**
       * Fraction of a proposal's value that should be bonded in order to place the proposal.
       * An accepted proposal gets these back. A rejected proposal does not.
       */
      ProposalBond: PlainDescriptor<number>;

      /**
       * Minimum amount of funds that should be placed in a deposit for making a proposal.
       */
      ProposalBondMinimum: PlainDescriptor<bigint>;

      /**
       * Maximum amount of funds that should be placed in a deposit for making a proposal.
       */
      ProposalBondMaximum: PlainDescriptor<bigint | undefined>;

      /**
       * Period between successive spends.
       */
      SpendPeriod: PlainDescriptor<number>;

      /**
       * Percentage of spare funds (if any) that are burnt per spend period.
       */
      Burn: PlainDescriptor<number>;

      /**
       * The treasury's pallet id, used for deriving its sovereign account ID.
       */
      PalletId: PlainDescriptor<Binary>;

      /**
       * The maximum number of approvals that can wait in the spending queue.
       *
       * NOTE: This parameter is also used within the Bounties Pallet extension if enabled.
       */
      MaxApprovals: PlainDescriptor<number>;

      /**
       * The period during which an approved treasury spend has to be claimed.
       */
      PayoutPeriod: PlainDescriptor<number>;
    },
  ];
  ConvictionVoting: [
    {
      /**
       * All voting for a particular voter in a particular voting class. We store the balance for the
       * number of votes that we have recorded.
       */
      VotingFor: StorageDescriptor<[SS58String, number], ConvictionVotingVoteVoting, false>;

      /**
       * The voting classes which have a non-zero lock requirement and the lock amounts which they
       * require. The actual amount locked on behalf of this pallet should always be the maximum of
       * this list.
       */
      ClassLocksFor: StorageDescriptor<[Key: SS58String], Array<Anonymize<I4ojmnsk1dchql>>, false>;
    },
    {
      /**
       *See [`Pallet::vote`].
       */
      vote: TxDescriptor<{ poll_index: number; vote: ConvictionVotingVoteAccountVote }>;

      /**
       *See [`Pallet::delegate`].
       */
      delegate: TxDescriptor<{
        class: number;
        to: MultiAddress;
        conviction: VotingConviction;
        balance: bigint;
      }>;

      /**
       *See [`Pallet::undelegate`].
       */
      undelegate: TxDescriptor<{ class: number }>;

      /**
       *See [`Pallet::unlock`].
       */
      unlock: TxDescriptor<{ class: number; target: MultiAddress }>;

      /**
       *See [`Pallet::remove_vote`].
       */
      remove_vote: TxDescriptor<{ class: Anonymize<I4arjljr6dpflb>; index: number }>;

      /**
       *See [`Pallet::remove_other_vote`].
       */
      remove_other_vote: TxDescriptor<{ target: MultiAddress; class: number; index: number }>;
    },
    {
      /**
       *An account has delegated their vote to another account. \[who, target\]
       */
      Delegated: PlainDescriptor<[SS58String, SS58String]>;

      /**
       *An \[account\] has cancelled a previous delegation operation.
       */
      Undelegated: PlainDescriptor<SS58String>;
    },
    {
      /**
       *Poll is not ongoing.
       */
      NotOngoing: PlainDescriptor<undefined>;

      /**
       *The given account did not vote on the poll.
       */
      NotVoter: PlainDescriptor<undefined>;

      /**
       *The actor has no permission to conduct the action.
       */
      NoPermission: PlainDescriptor<undefined>;

      /**
       *The actor has no permission to conduct the action right now but will do in the future.
       */
      NoPermissionYet: PlainDescriptor<undefined>;

      /**
       *The account is already delegating.
       */
      AlreadyDelegating: PlainDescriptor<undefined>;

      /**
       *The account currently has votes attached to it and the operation cannot succeed until
       *these are removed through `remove_vote`.
       */
      AlreadyVoting: PlainDescriptor<undefined>;

      /**
       *Too high a balance was provided that the account cannot afford.
       */
      InsufficientFunds: PlainDescriptor<undefined>;

      /**
       *The account is not currently delegating.
       */
      NotDelegating: PlainDescriptor<undefined>;

      /**
       *Delegation to oneself makes no sense.
       */
      Nonsense: PlainDescriptor<undefined>;

      /**
       *Maximum number of votes reached.
       */
      MaxVotesReached: PlainDescriptor<undefined>;

      /**
       *The class must be supplied since it is not easily determinable from the state.
       */
      ClassNeeded: PlainDescriptor<undefined>;

      /**
       *The class ID supplied is invalid.
       */
      BadClass: PlainDescriptor<undefined>;
    },
    {
      /**
       * The maximum number of concurrent votes an account may have.
       *
       * Also used to compute weight, an overly large value can lead to extrinsics with large
       * weight estimation: see `delegate` for instance.
       */
      MaxVotes: PlainDescriptor<number>;

      /**
       * The minimum period of vote locking.
       *
       * It should be no shorter than enactment period to ensure that in the case of an approval,
       * those successful voters are locked into the consequences that their votes entail.
       */
      VoteLockingPeriod: PlainDescriptor<number>;
    },
  ];
  Referenda: [
    {
      /**
       * The next free referendum index, aka the number of referenda started so far.
       */
      ReferendumCount: StorageDescriptor<[], number, false>;

      /**
       * Information concerning any given referendum.
       */
      ReferendumInfoFor: StorageDescriptor<[Key: number], ReferendaTypesReferendumInfo, true>;

      /**
       * The sorted list of referenda ready to be decided but not yet being decided, ordered by
       * conviction-weighted approvals.
       *
       * This should be empty if `DecidingCount` is less than `TrackInfo::max_deciding`.
       */
      TrackQueue: StorageDescriptor<[Key: number], Array<Anonymize<I4ojmnsk1dchql>>, false>;

      /**
       * The number of referenda being decided currently.
       */
      DecidingCount: StorageDescriptor<[Key: number], number, false>;

      /**
       * The metadata is a general information concerning the referendum.
       * The `Hash` refers to the preimage of the `Preimages` provider which can be a JSON
       * dump or IPFS hash of a JSON file.
       *
       * Consider a garbage collection for a metadata of finished referendums to `unrequest` (remove)
       * large preimages.
       */
      MetadataOf: StorageDescriptor<[Key: number], Binary, true>;
    },
    {
      /**
       *See [`Pallet::submit`].
       */
      submit: TxDescriptor<{
        proposal_origin: WestendRuntimeOriginCaller;
        proposal: PreimagesBounded;
        enactment_moment: TraitsScheduleDispatchTime;
      }>;

      /**
       *See [`Pallet::place_decision_deposit`].
       */
      place_decision_deposit: TxDescriptor<{ index: number }>;

      /**
       *See [`Pallet::refund_decision_deposit`].
       */
      refund_decision_deposit: TxDescriptor<{ index: number }>;

      /**
       *See [`Pallet::cancel`].
       */
      cancel: TxDescriptor<{ index: number }>;

      /**
       *See [`Pallet::kill`].
       */
      kill: TxDescriptor<{ index: number }>;

      /**
       *See [`Pallet::nudge_referendum`].
       */
      nudge_referendum: TxDescriptor<{ index: number }>;

      /**
       *See [`Pallet::one_fewer_deciding`].
       */
      one_fewer_deciding: TxDescriptor<{ track: number }>;

      /**
       *See [`Pallet::refund_submission_deposit`].
       */
      refund_submission_deposit: TxDescriptor<{ index: number }>;

      /**
       *See [`Pallet::set_metadata`].
       */
      set_metadata: TxDescriptor<{ index: number; maybe_hash: Anonymize<I17k3ujudqd5df> }>;
    },
    {
      /**
       *A referendum has been submitted.
       */
      Submitted: PlainDescriptor<{ index: number; track: number; proposal: PreimagesBounded }>;

      /**
       *The decision deposit has been placed.
       */
      DecisionDepositPlaced: PlainDescriptor<{ index: number; who: SS58String; amount: bigint }>;

      /**
       *The decision deposit has been refunded.
       */
      DecisionDepositRefunded: PlainDescriptor<{ index: number; who: SS58String; amount: bigint }>;

      /**
       *A deposit has been slashed.
       */
      DepositSlashed: PlainDescriptor<{ who: SS58String; amount: bigint }>;

      /**
       *A referendum has moved into the deciding phase.
       */
      DecisionStarted: PlainDescriptor<{
        index: number;
        track: number;
        proposal: PreimagesBounded;
        tally: Anonymize<Ifsk7cbmtit1jd>;
      }>;

      /**

 */
      ConfirmStarted: PlainDescriptor<{ index: number }>;

      /**

 */
      ConfirmAborted: PlainDescriptor<{ index: number }>;

      /**
       *A referendum has ended its confirmation phase and is ready for approval.
       */
      Confirmed: PlainDescriptor<{ index: number; tally: Anonymize<Ifsk7cbmtit1jd> }>;

      /**
       *A referendum has been approved and its proposal has been scheduled.
       */
      Approved: PlainDescriptor<{ index: number }>;

      /**
       *A proposal has been rejected by referendum.
       */
      Rejected: PlainDescriptor<{ index: number; tally: Anonymize<Ifsk7cbmtit1jd> }>;

      /**
       *A referendum has been timed out without being decided.
       */
      TimedOut: PlainDescriptor<{ index: number; tally: Anonymize<Ifsk7cbmtit1jd> }>;

      /**
       *A referendum has been cancelled.
       */
      Cancelled: PlainDescriptor<{ index: number; tally: Anonymize<Ifsk7cbmtit1jd> }>;

      /**
       *A referendum has been killed.
       */
      Killed: PlainDescriptor<{ index: number; tally: Anonymize<Ifsk7cbmtit1jd> }>;

      /**
       *The submission deposit has been refunded.
       */
      SubmissionDepositRefunded: PlainDescriptor<{
        index: number;
        who: SS58String;
        amount: bigint;
      }>;

      /**
       *Metadata for a referendum has been set.
       */
      MetadataSet: PlainDescriptor<{ index: number; hash: Binary }>;

      /**
       *Metadata for a referendum has been cleared.
       */
      MetadataCleared: PlainDescriptor<{ index: number; hash: Binary }>;
    },
    {
      /**
       *Referendum is not ongoing.
       */
      NotOngoing: PlainDescriptor<undefined>;

      /**
       *Referendum's decision deposit is already paid.
       */
      HasDeposit: PlainDescriptor<undefined>;

      /**
       *The track identifier given was invalid.
       */
      BadTrack: PlainDescriptor<undefined>;

      /**
       *There are already a full complement of referenda in progress for this track.
       */
      Full: PlainDescriptor<undefined>;

      /**
       *The queue of the track is empty.
       */
      QueueEmpty: PlainDescriptor<undefined>;

      /**
       *The referendum index provided is invalid in this context.
       */
      BadReferendum: PlainDescriptor<undefined>;

      /**
       *There was nothing to do in the advancement.
       */
      NothingToDo: PlainDescriptor<undefined>;

      /**
       *No track exists for the proposal origin.
       */
      NoTrack: PlainDescriptor<undefined>;

      /**
       *Any deposit cannot be refunded until after the decision is over.
       */
      Unfinished: PlainDescriptor<undefined>;

      /**
       *The deposit refunder is not the depositor.
       */
      NoPermission: PlainDescriptor<undefined>;

      /**
       *The deposit cannot be refunded since none was made.
       */
      NoDeposit: PlainDescriptor<undefined>;

      /**
       *The referendum status is invalid for this operation.
       */
      BadStatus: PlainDescriptor<undefined>;

      /**
       *The preimage does not exist.
       */
      PreimageNotExist: PlainDescriptor<undefined>;
    },
    {
      /**
       * The minimum amount to be used as a deposit for a public referendum proposal.
       */
      SubmissionDeposit: PlainDescriptor<bigint>;

      /**
       * Maximum size of the referendum queue for a single track.
       */
      MaxQueued: PlainDescriptor<number>;

      /**
       * The number of blocks after submission that a referendum must begin being decided by.
       * Once this passes, then anyone may cancel the referendum.
       */
      UndecidingTimeout: PlainDescriptor<number>;

      /**
       * Quantization level for the referendum wakeup scheduler. A higher number will result in
       * fewer storage reads/writes needed for smaller voters, but also result in delays to the
       * automatic referendum status changes. Explicit servicing instructions are unaffected.
       */
      AlarmInterval: PlainDescriptor<number>;

      /**
       * Information concerning the different referendum tracks.
       */
      Tracks: PlainDescriptor<Array<Anonymize<Ida9vhl30l98p4>>>;
    },
  ];
  FellowshipCollective: [
    {
      /**
       * The number of members in the collective who have at least the rank according to the index
       * of the vec.
       */
      MemberCount: StorageDescriptor<[Key: number], number, false>;

      /**
       * The current members of the collective.
       */
      Members: StorageDescriptor<[Key: SS58String], number, true>;

      /**
       * The index of each ranks's member into the group of members who have at least that rank.
       */
      IdToIndex: StorageDescriptor<[number, SS58String], number, true>;

      /**
       * The members in the collective by index. All indices in the range `0..MemberCount` will
       * return `Some`, however a member's index is not guaranteed to remain unchanged over time.
       */
      IndexToId: StorageDescriptor<[number, number], SS58String, true>;

      /**
       * Votes on a given proposal, if it is ongoing.
       */
      Voting: StorageDescriptor<
        [number, SS58String],
        Anonymize<AnonymousEnum<{ Aye: Anonymize<number>; Nay: Anonymize<number> }>>,
        true
      >;

      /**

 */
      VotingCleanup: StorageDescriptor<[Key: number], Binary, true>;
    },
    {
      /**
       *See [`Pallet::add_member`].
       */
      add_member: TxDescriptor<{ who: MultiAddress }>;

      /**
       *See [`Pallet::promote_member`].
       */
      promote_member: TxDescriptor<{ who: MultiAddress }>;

      /**
       *See [`Pallet::demote_member`].
       */
      demote_member: TxDescriptor<{ who: MultiAddress }>;

      /**
       *See [`Pallet::remove_member`].
       */
      remove_member: TxDescriptor<{ who: MultiAddress; min_rank: number }>;

      /**
       *See [`Pallet::vote`].
       */
      vote: TxDescriptor<{ poll: number; aye: boolean }>;

      /**
       *See [`Pallet::cleanup_poll`].
       */
      cleanup_poll: TxDescriptor<{ poll_index: number; max: number }>;

      /**
       *See [`Pallet::exchange_member`].
       */
      exchange_member: TxDescriptor<{ who: MultiAddress; new_who: MultiAddress }>;
    },
    {
      /**
       *A member `who` has been added.
       */
      MemberAdded: PlainDescriptor<{ who: SS58String }>;

      /**
       *The member `who`se rank has been changed to the given `rank`.
       */
      RankChanged: PlainDescriptor<{ who: SS58String; rank: number }>;

      /**
       *The member `who` of given `rank` has been removed from the collective.
       */
      MemberRemoved: PlainDescriptor<{ who: SS58String; rank: number }>;

      /**
       *The member `who` has voted for the `poll` with the given `vote` leading to an updated
       *`tally`.
       */
      Voted: PlainDescriptor<{
        who: SS58String;
        poll: number;
        vote: Anonymize<I996080trg6urn>;
        tally: Anonymize<I6d634btv7misu>;
      }>;

      /**
       *The member `who` had their `AccountId` changed to `new_who`.
       */
      MemberExchanged: PlainDescriptor<{ who: SS58String; new_who: SS58String }>;
    },
    {
      /**
       *Account is already a member.
       */
      AlreadyMember: PlainDescriptor<undefined>;

      /**
       *Account is not a member.
       */
      NotMember: PlainDescriptor<undefined>;

      /**
       *The given poll index is unknown or has closed.
       */
      NotPolling: PlainDescriptor<undefined>;

      /**
       *The given poll is still ongoing.
       */
      Ongoing: PlainDescriptor<undefined>;

      /**
       *There are no further records to be removed.
       */
      NoneRemaining: PlainDescriptor<undefined>;

      /**
       *Unexpected error in state.
       */
      Corruption: PlainDescriptor<undefined>;

      /**
       *The member's rank is too low to vote.
       */
      RankTooLow: PlainDescriptor<undefined>;

      /**
       *The information provided is incorrect.
       */
      InvalidWitness: PlainDescriptor<undefined>;

      /**
       *The origin is not sufficiently privileged to do the operation.
       */
      NoPermission: PlainDescriptor<undefined>;

      /**
       *The new member to exchange is the same as the old member
       */
      SameMember: PlainDescriptor<undefined>;
    },
    {},
  ];
  FellowshipReferenda: [
    {
      /**
       * The next free referendum index, aka the number of referenda started so far.
       */
      ReferendumCount: StorageDescriptor<[], number, false>;

      /**
       * Information concerning any given referendum.
       */
      ReferendumInfoFor: StorageDescriptor<
        [Key: number],
        Anonymize<
          AnonymousEnum<{
            Ongoing: Anonymize<Ieeijqsusb4a5r>;
            Approved: Anonymize<Ini94eljn5lj8>;
            Rejected: Anonymize<Ini94eljn5lj8>;
            Cancelled: Anonymize<Ini94eljn5lj8>;
            TimedOut: Anonymize<Ini94eljn5lj8>;
            Killed: Anonymize<number>;
          }>
        >,
        true
      >;

      /**
       * The sorted list of referenda ready to be decided but not yet being decided, ordered by
       * conviction-weighted approvals.
       *
       * This should be empty if `DecidingCount` is less than `TrackInfo::max_deciding`.
       */
      TrackQueue: StorageDescriptor<[Key: number], Array<Anonymize<I5g2vv0ckl2m8b>>, false>;

      /**
       * The number of referenda being decided currently.
       */
      DecidingCount: StorageDescriptor<[Key: number], number, false>;

      /**
       * The metadata is a general information concerning the referendum.
       * The `Hash` refers to the preimage of the `Preimages` provider which can be a JSON
       * dump or IPFS hash of a JSON file.
       *
       * Consider a garbage collection for a metadata of finished referendums to `unrequest` (remove)
       * large preimages.
       */
      MetadataOf: StorageDescriptor<[Key: number], Binary, true>;
    },
    {
      /**
       *See [`Pallet::submit`].
       */
      submit: TxDescriptor<{
        proposal_origin: WestendRuntimeOriginCaller;
        proposal: PreimagesBounded;
        enactment_moment: TraitsScheduleDispatchTime;
      }>;

      /**
       *See [`Pallet::place_decision_deposit`].
       */
      place_decision_deposit: TxDescriptor<{ index: number }>;

      /**
       *See [`Pallet::refund_decision_deposit`].
       */
      refund_decision_deposit: TxDescriptor<{ index: number }>;

      /**
       *See [`Pallet::cancel`].
       */
      cancel: TxDescriptor<{ index: number }>;

      /**
       *See [`Pallet::kill`].
       */
      kill: TxDescriptor<{ index: number }>;

      /**
       *See [`Pallet::nudge_referendum`].
       */
      nudge_referendum: TxDescriptor<{ index: number }>;

      /**
       *See [`Pallet::one_fewer_deciding`].
       */
      one_fewer_deciding: TxDescriptor<{ track: number }>;

      /**
       *See [`Pallet::refund_submission_deposit`].
       */
      refund_submission_deposit: TxDescriptor<{ index: number }>;

      /**
       *See [`Pallet::set_metadata`].
       */
      set_metadata: TxDescriptor<{ index: number; maybe_hash: Anonymize<I17k3ujudqd5df> }>;
    },
    {
      /**
       *A referendum has been submitted.
       */
      Submitted: PlainDescriptor<{ index: number; track: number; proposal: PreimagesBounded }>;

      /**
       *The decision deposit has been placed.
       */
      DecisionDepositPlaced: PlainDescriptor<{ index: number; who: SS58String; amount: bigint }>;

      /**
       *The decision deposit has been refunded.
       */
      DecisionDepositRefunded: PlainDescriptor<{ index: number; who: SS58String; amount: bigint }>;

      /**
       *A deposit has been slashed.
       */
      DepositSlashed: PlainDescriptor<{ who: SS58String; amount: bigint }>;

      /**
       *A referendum has moved into the deciding phase.
       */
      DecisionStarted: PlainDescriptor<{
        index: number;
        track: number;
        proposal: PreimagesBounded;
        tally: Anonymize<I6d634btv7misu>;
      }>;

      /**

 */
      ConfirmStarted: PlainDescriptor<{ index: number }>;

      /**

 */
      ConfirmAborted: PlainDescriptor<{ index: number }>;

      /**
       *A referendum has ended its confirmation phase and is ready for approval.
       */
      Confirmed: PlainDescriptor<{ index: number; tally: Anonymize<I6d634btv7misu> }>;

      /**
       *A referendum has been approved and its proposal has been scheduled.
       */
      Approved: PlainDescriptor<{ index: number }>;

      /**
       *A proposal has been rejected by referendum.
       */
      Rejected: PlainDescriptor<{ index: number; tally: Anonymize<I6d634btv7misu> }>;

      /**
       *A referendum has been timed out without being decided.
       */
      TimedOut: PlainDescriptor<{ index: number; tally: Anonymize<I6d634btv7misu> }>;

      /**
       *A referendum has been cancelled.
       */
      Cancelled: PlainDescriptor<{ index: number; tally: Anonymize<I6d634btv7misu> }>;

      /**
       *A referendum has been killed.
       */
      Killed: PlainDescriptor<{ index: number; tally: Anonymize<I6d634btv7misu> }>;

      /**
       *The submission deposit has been refunded.
       */
      SubmissionDepositRefunded: PlainDescriptor<{
        index: number;
        who: SS58String;
        amount: bigint;
      }>;

      /**
       *Metadata for a referendum has been set.
       */
      MetadataSet: PlainDescriptor<{ index: number; hash: Binary }>;

      /**
       *Metadata for a referendum has been cleared.
       */
      MetadataCleared: PlainDescriptor<{ index: number; hash: Binary }>;
    },
    {
      /**
       *Referendum is not ongoing.
       */
      NotOngoing: PlainDescriptor<undefined>;

      /**
       *Referendum's decision deposit is already paid.
       */
      HasDeposit: PlainDescriptor<undefined>;

      /**
       *The track identifier given was invalid.
       */
      BadTrack: PlainDescriptor<undefined>;

      /**
       *There are already a full complement of referenda in progress for this track.
       */
      Full: PlainDescriptor<undefined>;

      /**
       *The queue of the track is empty.
       */
      QueueEmpty: PlainDescriptor<undefined>;

      /**
       *The referendum index provided is invalid in this context.
       */
      BadReferendum: PlainDescriptor<undefined>;

      /**
       *There was nothing to do in the advancement.
       */
      NothingToDo: PlainDescriptor<undefined>;

      /**
       *No track exists for the proposal origin.
       */
      NoTrack: PlainDescriptor<undefined>;

      /**
       *Any deposit cannot be refunded until after the decision is over.
       */
      Unfinished: PlainDescriptor<undefined>;

      /**
       *The deposit refunder is not the depositor.
       */
      NoPermission: PlainDescriptor<undefined>;

      /**
       *The deposit cannot be refunded since none was made.
       */
      NoDeposit: PlainDescriptor<undefined>;

      /**
       *The referendum status is invalid for this operation.
       */
      BadStatus: PlainDescriptor<undefined>;

      /**
       *The preimage does not exist.
       */
      PreimageNotExist: PlainDescriptor<undefined>;
    },
    {
      /**
       * The minimum amount to be used as a deposit for a public referendum proposal.
       */
      SubmissionDeposit: PlainDescriptor<bigint>;

      /**
       * Maximum size of the referendum queue for a single track.
       */
      MaxQueued: PlainDescriptor<number>;

      /**
       * The number of blocks after submission that a referendum must begin being decided by.
       * Once this passes, then anyone may cancel the referendum.
       */
      UndecidingTimeout: PlainDescriptor<number>;

      /**
       * Quantization level for the referendum wakeup scheduler. A higher number will result in
       * fewer storage reads/writes needed for smaller voters, but also result in delays to the
       * automatic referendum status changes. Explicit servicing instructions are unaffected.
       */
      AlarmInterval: PlainDescriptor<number>;

      /**
       * Information concerning the different referendum tracks.
       */
      Tracks: PlainDescriptor<Array<Anonymize<Ida9vhl30l98p4>>>;
    },
  ];
  Origins: [{}, {}, {}, {}, {}];
  Whitelist: [
    {
      /**

 */
      WhitelistedCall: StorageDescriptor<[Key: Binary], undefined, true>;
    },
    {
      /**
       *See [`Pallet::whitelist_call`].
       */
      whitelist_call: TxDescriptor<{ call_hash: Binary }>;

      /**
       *See [`Pallet::remove_whitelisted_call`].
       */
      remove_whitelisted_call: TxDescriptor<{ call_hash: Binary }>;

      /**
       *See [`Pallet::dispatch_whitelisted_call`].
       */
      dispatch_whitelisted_call: TxDescriptor<{
        call_hash: Binary;
        call_encoded_len: number;
        call_weight_witness: Anonymize<I4q39t5hn830vp>;
      }>;

      /**
       *See [`Pallet::dispatch_whitelisted_call_with_preimage`].
       */
      dispatch_whitelisted_call_with_preimage: TxDescriptor<{ call: Anonymize<I5v7s5dm78b0mh> }>;
    },
    {
      /**

 */
      CallWhitelisted: PlainDescriptor<{ call_hash: Binary }>;

      /**

 */
      WhitelistedCallRemoved: PlainDescriptor<{ call_hash: Binary }>;

      /**

 */
      WhitelistedCallDispatched: PlainDescriptor<{
        call_hash: Binary;
        result: Anonymize<Idurpak8qagatr>;
      }>;
    },
    {
      /**
       *The preimage of the call hash could not be loaded.
       */
      UnavailablePreImage: PlainDescriptor<undefined>;

      /**
       *The call could not be decoded.
       */
      UndecodableCall: PlainDescriptor<undefined>;

      /**
       *The weight of the decoded call was higher than the witness.
       */
      InvalidCallWeightWitness: PlainDescriptor<undefined>;

      /**
       *The call was not whitelisted.
       */
      CallIsNotWhitelisted: PlainDescriptor<undefined>;

      /**
       *The call was already whitelisted; No-Op.
       */
      CallAlreadyWhitelisted: PlainDescriptor<undefined>;
    },
    {},
  ];
  Claims: [
    {
      /**

 */
      Claims: StorageDescriptor<[Key: Binary], bigint, true>;

      /**

 */
      Total: StorageDescriptor<[], bigint, false>;

      /**
       * Vesting schedule for a claim.
       * First balance is the total amount that should be held for vesting.
       * Second balance is how much should be unlocked per block.
       * The block number is when the vesting should start.
       */
      Vesting: StorageDescriptor<[Key: Binary], [bigint, bigint, number], true>;

      /**
       * The statement kind that must be signed, if any.
       */
      Signing: StorageDescriptor<[Key: Binary], ClaimsStatementKind, true>;

      /**
       * Pre-claimed Ethereum accounts, by the Account ID that they are claimed to.
       */
      Preclaims: StorageDescriptor<[Key: SS58String], Binary, true>;
    },
    {
      /**
       *See [`Pallet::claim`].
       */
      claim: TxDescriptor<{ dest: SS58String; ethereum_signature: Binary }>;

      /**
       *See [`Pallet::mint_claim`].
       */
      mint_claim: TxDescriptor<{
        who: Binary;
        value: bigint;
        vesting_schedule: Anonymize<I70kqehrkegc98>;
        statement: Anonymize<I6rcr4im2g3gv9>;
      }>;

      /**
       *See [`Pallet::claim_attest`].
       */
      claim_attest: TxDescriptor<{
        dest: SS58String;
        ethereum_signature: Binary;
        statement: Binary;
      }>;

      /**
       *See [`Pallet::attest`].
       */
      attest: TxDescriptor<{ statement: Binary }>;

      /**
       *See [`Pallet::move_claim`].
       */
      move_claim: TxDescriptor<{
        old: Binary;
        new: Binary;
        maybe_preclaim: Anonymize<Ihfphjolmsqq1>;
      }>;
    },
    {
      /**
       *Someone claimed some DOTs.
       */
      Claimed: PlainDescriptor<{ who: SS58String; ethereum_address: Binary; amount: bigint }>;
    },
    {
      /**
       *Invalid Ethereum signature.
       */
      InvalidEthereumSignature: PlainDescriptor<undefined>;

      /**
       *Ethereum address has no claim.
       */
      SignerHasNoClaim: PlainDescriptor<undefined>;

      /**
       *Account ID sending transaction has no claim.
       */
      SenderHasNoClaim: PlainDescriptor<undefined>;

      /**
       *There's not enough in the pot to pay out some unvested amount. Generally implies a
       *logic error.
       */
      PotUnderflow: PlainDescriptor<undefined>;

      /**
       *A needed statement was not included.
       */
      InvalidStatement: PlainDescriptor<undefined>;

      /**
       *The account already has a vested balance.
       */
      VestedBalanceExists: PlainDescriptor<undefined>;
    },
    {
      /**

 */
      Prefix: PlainDescriptor<Binary>;
    },
  ];
  Utility: [
    {},
    {
      /**
       *See [`Pallet::batch`].
       */
      batch: TxDescriptor<{ calls: Anonymize<I52mcb5hepi70g> }>;

      /**
       *See [`Pallet::as_derivative`].
       */
      as_derivative: TxDescriptor<{ index: number; call: Anonymize<I5v7s5dm78b0mh> }>;

      /**
       *See [`Pallet::batch_all`].
       */
      batch_all: TxDescriptor<{ calls: Anonymize<I52mcb5hepi70g> }>;

      /**
       *See [`Pallet::dispatch_as`].
       */
      dispatch_as: TxDescriptor<{
        as_origin: WestendRuntimeOriginCaller;
        call: Anonymize<I5v7s5dm78b0mh>;
      }>;

      /**
       *See [`Pallet::force_batch`].
       */
      force_batch: TxDescriptor<{ calls: Anonymize<I52mcb5hepi70g> }>;

      /**
       *See [`Pallet::with_weight`].
       */
      with_weight: TxDescriptor<{
        call: Anonymize<I5v7s5dm78b0mh>;
        weight: Anonymize<I4q39t5hn830vp>;
      }>;
    },
    {
      /**
       *Batch of dispatches did not complete fully. Index of first failing dispatch given, as
       *well as the error.
       */
      BatchInterrupted: PlainDescriptor<{ index: number; error: DispatchError }>;

      /**
       *Batch of dispatches completed fully with no error.
       */
      BatchCompleted: PlainDescriptor<undefined>;

      /**
       *Batch of dispatches completed but has errors.
       */
      BatchCompletedWithErrors: PlainDescriptor<undefined>;

      /**
       *A single item within a Batch of dispatches has completed with no error.
       */
      ItemCompleted: PlainDescriptor<undefined>;

      /**
       *A single item within a Batch of dispatches has completed with error.
       */
      ItemFailed: PlainDescriptor<{ error: DispatchError }>;

      /**
       *A call was dispatched.
       */
      DispatchedAs: PlainDescriptor<{ result: Anonymize<Idtdr91jmq5g4i> }>;
    },
    {
      /**
       *Too many calls batched.
       */
      TooManyCalls: PlainDescriptor<undefined>;
    },
    {
      /**
       * The limit on the number of batched calls.
       */
      batched_calls_limit: PlainDescriptor<number>;
    },
  ];
  Identity: [
    {
      /**
       * Information that is pertinent to identify the entity behind an account. First item is the
       * registration, second is the account's primary username.
       *
       * TWOX-NOTE: OK ― `AccountId` is a secure hash.
       */
      IdentityOf: StorageDescriptor<
        [Key: SS58String],
        [Anonymize<I60cr97f9of5s5>, Anonymize<Iabpgqcjikia83>],
        true
      >;

      /**
       * The super-identity of an alternative "sub" identity together with its name, within that
       * context. If the account is not some other account's sub-identity, then just `None`.
       */
      SuperOf: StorageDescriptor<[Key: SS58String], [SS58String, IdentityTypesData], true>;

      /**
       * Alternative "sub" identities of this account.
       *
       * The first item is the deposit, the second is a vector of the accounts.
       *
       * TWOX-NOTE: OK ― `AccountId` is a secure hash.
       */
      SubsOf: StorageDescriptor<[Key: SS58String], [bigint, Anonymize<Ia2lhg7l2hilo3>], false>;

      /**
       * The set of registrars. Not expected to get very big as can only be added through a
       * special origin (likely a council motion).
       *
       * The index into this can be cast to `RegistrarIndex` to get a valid value.
       */
      Registrars: StorageDescriptor<[], Array<Anonymize<I48v3sekdprq30>>, false>;

      /**
       * A map of the accounts who are authorized to grant usernames.
       */
      UsernameAuthorities: StorageDescriptor<
        [Key: SS58String],
        { suffix: Binary; allocation: number },
        true
      >;

      /**
       * Reverse lookup from `username` to the `AccountId` that has registered it. The value should
       * be a key in the `IdentityOf` map, but it may not if the user has cleared their identity.
       *
       * Multiple usernames may map to the same `AccountId`, but `IdentityOf` will only map to one
       * primary username.
       */
      AccountOfUsername: StorageDescriptor<[Key: Binary], SS58String, true>;

      /**
       * Usernames that an authority has granted, but that the account controller has not confirmed
       * that they want it. Used primarily in cases where the `AccountId` cannot provide a signature
       * because they are a pure proxy, multisig, etc. In order to confirm it, they should call
       * [`Call::accept_username`].
       *
       * First tuple item is the account and second is the acceptance deadline.
       */
      PendingUsernames: StorageDescriptor<[Key: Binary], [SS58String, number], true>;
    },
    {
      /**
       *See [`Pallet::add_registrar`].
       */
      add_registrar: TxDescriptor<{ account: MultiAddress }>;

      /**
       *See [`Pallet::set_identity`].
       */
      set_identity: TxDescriptor<{ info: Anonymize<I939m6d6t3dsgm> }>;

      /**
       *See [`Pallet::set_subs`].
       */
      set_subs: TxDescriptor<{ subs: Anonymize<I47e5e4dh41s5v> }>;

      /**
       *See [`Pallet::clear_identity`].
       */
      clear_identity: TxDescriptor<undefined>;

      /**
       *See [`Pallet::request_judgement`].
       */
      request_judgement: TxDescriptor<{ reg_index: number; max_fee: bigint }>;

      /**
       *See [`Pallet::cancel_request`].
       */
      cancel_request: TxDescriptor<{ reg_index: number }>;

      /**
       *See [`Pallet::set_fee`].
       */
      set_fee: TxDescriptor<{ index: number; fee: bigint }>;

      /**
       *See [`Pallet::set_account_id`].
       */
      set_account_id: TxDescriptor<{ index: number; new: MultiAddress }>;

      /**
       *See [`Pallet::set_fields`].
       */
      set_fields: TxDescriptor<{ index: number; fields: bigint }>;

      /**
       *See [`Pallet::provide_judgement`].
       */
      provide_judgement: TxDescriptor<{
        reg_index: number;
        target: MultiAddress;
        judgement: IdentityJudgement;
        identity: Binary;
      }>;

      /**
       *See [`Pallet::kill_identity`].
       */
      kill_identity: TxDescriptor<{ target: MultiAddress }>;

      /**
       *See [`Pallet::add_sub`].
       */
      add_sub: TxDescriptor<{ sub: MultiAddress; data: IdentityTypesData }>;

      /**
       *See [`Pallet::rename_sub`].
       */
      rename_sub: TxDescriptor<{ sub: MultiAddress; data: IdentityTypesData }>;

      /**
       *See [`Pallet::remove_sub`].
       */
      remove_sub: TxDescriptor<{ sub: MultiAddress }>;

      /**
       *See [`Pallet::quit_sub`].
       */
      quit_sub: TxDescriptor<undefined>;

      /**
       *See [`Pallet::add_username_authority`].
       */
      add_username_authority: TxDescriptor<{
        authority: MultiAddress;
        suffix: Binary;
        allocation: number;
      }>;

      /**
       *See [`Pallet::remove_username_authority`].
       */
      remove_username_authority: TxDescriptor<{ authority: MultiAddress }>;

      /**
       *See [`Pallet::set_username_for`].
       */
      set_username_for: TxDescriptor<{
        who: MultiAddress;
        username: Binary;
        signature: Anonymize<I7us28h09nc5sv>;
      }>;

      /**
       *See [`Pallet::accept_username`].
       */
      accept_username: TxDescriptor<{ username: Binary }>;

      /**
       *See [`Pallet::remove_expired_approval`].
       */
      remove_expired_approval: TxDescriptor<{ username: Binary }>;

      /**
       *See [`Pallet::set_primary_username`].
       */
      set_primary_username: TxDescriptor<{ username: Binary }>;

      /**
       *See [`Pallet::remove_dangling_username`].
       */
      remove_dangling_username: TxDescriptor<{ username: Binary }>;
    },
    {
      /**
       *A name was set or reset (which will remove all judgements).
       */
      IdentitySet: PlainDescriptor<{ who: SS58String }>;

      /**
       *A name was cleared, and the given balance returned.
       */
      IdentityCleared: PlainDescriptor<{ who: SS58String; deposit: bigint }>;

      /**
       *A name was removed and the given balance slashed.
       */
      IdentityKilled: PlainDescriptor<{ who: SS58String; deposit: bigint }>;

      /**
       *A judgement was asked from a registrar.
       */
      JudgementRequested: PlainDescriptor<{ who: SS58String; registrar_index: number }>;

      /**
       *A judgement request was retracted.
       */
      JudgementUnrequested: PlainDescriptor<{ who: SS58String; registrar_index: number }>;

      /**
       *A judgement was given by a registrar.
       */
      JudgementGiven: PlainDescriptor<{ target: SS58String; registrar_index: number }>;

      /**
       *A registrar was added.
       */
      RegistrarAdded: PlainDescriptor<{ registrar_index: number }>;

      /**
       *A sub-identity was added to an identity and the deposit paid.
       */
      SubIdentityAdded: PlainDescriptor<{ sub: SS58String; main: SS58String; deposit: bigint }>;

      /**
       *A sub-identity was removed from an identity and the deposit freed.
       */
      SubIdentityRemoved: PlainDescriptor<{ sub: SS58String; main: SS58String; deposit: bigint }>;

      /**
       *A sub-identity was cleared, and the given deposit repatriated from the
       *main identity account to the sub-identity account.
       */
      SubIdentityRevoked: PlainDescriptor<{ sub: SS58String; main: SS58String; deposit: bigint }>;

      /**
       *A username authority was added.
       */
      AuthorityAdded: PlainDescriptor<{ authority: SS58String }>;

      /**
       *A username authority was removed.
       */
      AuthorityRemoved: PlainDescriptor<{ authority: SS58String }>;

      /**
       *A username was set for `who`.
       */
      UsernameSet: PlainDescriptor<{ who: SS58String; username: Binary }>;

      /**
       *A username was queued, but `who` must accept it prior to `expiration`.
       */
      UsernameQueued: PlainDescriptor<{ who: SS58String; username: Binary; expiration: number }>;

      /**
       *A queued username passed its expiration without being claimed and was removed.
       */
      PreapprovalExpired: PlainDescriptor<{ whose: SS58String }>;

      /**
       *A username was set as a primary and can be looked up from `who`.
       */
      PrimaryUsernameSet: PlainDescriptor<{ who: SS58String; username: Binary }>;

      /**
       *A dangling username (as in, a username corresponding to an account that has removed its
       *identity) has been removed.
       */
      DanglingUsernameRemoved: PlainDescriptor<{ who: SS58String; username: Binary }>;
    },
    {
      /**
       *Too many subs-accounts.
       */
      TooManySubAccounts: PlainDescriptor<undefined>;

      /**
       *Account isn't found.
       */
      NotFound: PlainDescriptor<undefined>;

      /**
       *Account isn't named.
       */
      NotNamed: PlainDescriptor<undefined>;

      /**
       *Empty index.
       */
      EmptyIndex: PlainDescriptor<undefined>;

      /**
       *Fee is changed.
       */
      FeeChanged: PlainDescriptor<undefined>;

      /**
       *No identity found.
       */
      NoIdentity: PlainDescriptor<undefined>;

      /**
       *Sticky judgement.
       */
      StickyJudgement: PlainDescriptor<undefined>;

      /**
       *Judgement given.
       */
      JudgementGiven: PlainDescriptor<undefined>;

      /**
       *Invalid judgement.
       */
      InvalidJudgement: PlainDescriptor<undefined>;

      /**
       *The index is invalid.
       */
      InvalidIndex: PlainDescriptor<undefined>;

      /**
       *The target is invalid.
       */
      InvalidTarget: PlainDescriptor<undefined>;

      /**
       *Maximum amount of registrars reached. Cannot add any more.
       */
      TooManyRegistrars: PlainDescriptor<undefined>;

      /**
       *Account ID is already named.
       */
      AlreadyClaimed: PlainDescriptor<undefined>;

      /**
       *Sender is not a sub-account.
       */
      NotSub: PlainDescriptor<undefined>;

      /**
       *Sub-account isn't owned by sender.
       */
      NotOwned: PlainDescriptor<undefined>;

      /**
       *The provided judgement was for a different identity.
       */
      JudgementForDifferentIdentity: PlainDescriptor<undefined>;

      /**
       *Error that occurs when there is an issue paying for judgement.
       */
      JudgementPaymentFailed: PlainDescriptor<undefined>;

      /**
       *The provided suffix is too long.
       */
      InvalidSuffix: PlainDescriptor<undefined>;

      /**
       *The sender does not have permission to issue a username.
       */
      NotUsernameAuthority: PlainDescriptor<undefined>;

      /**
       *The authority cannot allocate any more usernames.
       */
      NoAllocation: PlainDescriptor<undefined>;

      /**
       *The signature on a username was not valid.
       */
      InvalidSignature: PlainDescriptor<undefined>;

      /**
       *Setting this username requires a signature, but none was provided.
       */
      RequiresSignature: PlainDescriptor<undefined>;

      /**
       *The username does not meet the requirements.
       */
      InvalidUsername: PlainDescriptor<undefined>;

      /**
       *The username is already taken.
       */
      UsernameTaken: PlainDescriptor<undefined>;

      /**
       *The requested username does not exist.
       */
      NoUsername: PlainDescriptor<undefined>;

      /**
       *The username cannot be forcefully removed because it can still be accepted.
       */
      NotExpired: PlainDescriptor<undefined>;
    },
    {
      /**
       * The amount held on deposit for a registered identity.
       */
      BasicDeposit: PlainDescriptor<bigint>;

      /**
       * The amount held on deposit per encoded byte for a registered identity.
       */
      ByteDeposit: PlainDescriptor<bigint>;

      /**
       * The amount held on deposit for a registered subaccount. This should account for the fact
       * that one storage item's value will increase by the size of an account ID, and there will
       * be another trie item whose value is the size of an account ID plus 32 bytes.
       */
      SubAccountDeposit: PlainDescriptor<bigint>;

      /**
       * The maximum number of sub-accounts allowed per identified account.
       */
      MaxSubAccounts: PlainDescriptor<number>;

      /**
       * Maxmimum number of registrars allowed in the system. Needed to bound the complexity
       * of, e.g., updating judgements.
       */
      MaxRegistrars: PlainDescriptor<number>;

      /**
       * The number of blocks within which a username grant must be accepted.
       */
      PendingUsernameExpiration: PlainDescriptor<number>;

      /**
       * The maximum length of a suffix.
       */
      MaxSuffixLength: PlainDescriptor<number>;

      /**
       * The maximum length of a username, including its suffix and any system-added delimiters.
       */
      MaxUsernameLength: PlainDescriptor<number>;
    },
  ];
  Society: [
    {
      /**
       * The max number of members for the society at one time.
       */
      Parameters: StorageDescriptor<
        [],
        { max_members: number; max_intake: number; max_strikes: number; candidate_deposit: bigint },
        true
      >;

      /**
       * Amount of our account balance that is specifically for the next round's bid(s).
       */
      Pot: StorageDescriptor<[], bigint, false>;

      /**
       * The first member.
       */
      Founder: StorageDescriptor<[], SS58String, true>;

      /**
       * The most primary from the most recently approved rank 0 members in the society.
       */
      Head: StorageDescriptor<[], SS58String, true>;

      /**
       * A hash of the rules of this society concerning membership. Can only be set once and
       * only by the founder.
       */
      Rules: StorageDescriptor<[], Binary, true>;

      /**
       * The current members and their rank. Doesn't include `SuspendedMembers`.
       */
      Members: StorageDescriptor<
        [Key: SS58String],
        { rank: number; strikes: number; vouching: Anonymize<Iceunvs0fkhdlv>; index: number },
        true
      >;

      /**
       * Information regarding rank-0 payouts, past and future.
       */
      Payouts: StorageDescriptor<
        [Key: SS58String],
        { paid: bigint; payouts: Anonymize<If9jidduiuq7vv> },
        false
      >;

      /**
       * The number of items in `Members` currently. (Doesn't include `SuspendedMembers`.)
       */
      MemberCount: StorageDescriptor<[], number, false>;

      /**
       * The current items in `Members` keyed by their unique index. Keys are densely populated
       * `0..MemberCount` (does not include `MemberCount`).
       */
      MemberByIndex: StorageDescriptor<[Key: number], SS58String, true>;

      /**
       * The set of suspended members, with their old membership record.
       */
      SuspendedMembers: StorageDescriptor<
        [Key: SS58String],
        { rank: number; strikes: number; vouching: Anonymize<Iceunvs0fkhdlv>; index: number },
        true
      >;

      /**
       * The number of rounds which have passed.
       */
      RoundCount: StorageDescriptor<[], number, false>;

      /**
       * The current bids, stored ordered by the value of the bid.
       */
      Bids: StorageDescriptor<[], Array<Anonymize<I5cap79lpfupvc>>, false>;

      /**

 */
      Candidates: StorageDescriptor<
        [Key: SS58String],
        {
          round: number;
          kind: Anonymize<Idvcmh0j8tecg6>;
          bid: bigint;
          tally: Anonymize<Ic4n54m09tc5na>;
          skeptic_struck: boolean;
        },
        true
      >;

      /**
       * The current skeptic.
       */
      Skeptic: StorageDescriptor<[], SS58String, true>;

      /**
       * Double map from Candidate -> Voter -> (Maybe) Vote.
       */
      Votes: StorageDescriptor<
        [SS58String, SS58String],
        { approve: boolean; weight: number },
        true
      >;

      /**
       * Clear-cursor for Vote, map from Candidate -> (Maybe) Cursor.
       */
      VoteClearCursor: StorageDescriptor<[Key: SS58String], Binary, true>;

      /**
       * At the end of the claim period, this contains the most recently approved members (along with
       * their bid and round ID) who is from the most recent round with the lowest bid. They will
       * become the new `Head`.
       */
      NextHead: StorageDescriptor<[], { who: SS58String; bid: bigint; round: number }, true>;

      /**
       * The number of challenge rounds there have been. Used to identify stale DefenderVotes.
       */
      ChallengeRoundCount: StorageDescriptor<[], number, false>;

      /**
       * The defending member currently being challenged, along with a running tally of votes.
       */
      Defending: StorageDescriptor<[], [SS58String, SS58String, Anonymize<Ic4n54m09tc5na>], true>;

      /**
       * Votes for the defender, keyed by challenge round.
       */
      DefenderVotes: StorageDescriptor<
        [number, SS58String],
        { approve: boolean; weight: number },
        true
      >;
    },
    {
      /**
       *See [`Pallet::bid`].
       */
      bid: TxDescriptor<{ value: bigint }>;

      /**
       *See [`Pallet::unbid`].
       */
      unbid: TxDescriptor<undefined>;

      /**
       *See [`Pallet::vouch`].
       */
      vouch: TxDescriptor<{ who: MultiAddress; value: bigint; tip: bigint }>;

      /**
       *See [`Pallet::unvouch`].
       */
      unvouch: TxDescriptor<undefined>;

      /**
       *See [`Pallet::vote`].
       */
      vote: TxDescriptor<{ candidate: MultiAddress; approve: boolean }>;

      /**
       *See [`Pallet::defender_vote`].
       */
      defender_vote: TxDescriptor<{ approve: boolean }>;

      /**
       *See [`Pallet::payout`].
       */
      payout: TxDescriptor<undefined>;

      /**
       *See [`Pallet::waive_repay`].
       */
      waive_repay: TxDescriptor<{ amount: bigint }>;

      /**
       *See [`Pallet::found_society`].
       */
      found_society: TxDescriptor<{
        founder: MultiAddress;
        max_members: number;
        max_intake: number;
        max_strikes: number;
        candidate_deposit: bigint;
        rules: Binary;
      }>;

      /**
       *See [`Pallet::dissolve`].
       */
      dissolve: TxDescriptor<undefined>;

      /**
       *See [`Pallet::judge_suspended_member`].
       */
      judge_suspended_member: TxDescriptor<{ who: MultiAddress; forgive: boolean }>;

      /**
       *See [`Pallet::set_parameters`].
       */
      set_parameters: TxDescriptor<{
        max_members: number;
        max_intake: number;
        max_strikes: number;
        candidate_deposit: bigint;
      }>;

      /**
       *See [`Pallet::punish_skeptic`].
       */
      punish_skeptic: TxDescriptor<undefined>;

      /**
       *See [`Pallet::claim_membership`].
       */
      claim_membership: TxDescriptor<undefined>;

      /**
       *See [`Pallet::bestow_membership`].
       */
      bestow_membership: TxDescriptor<{ candidate: SS58String }>;

      /**
       *See [`Pallet::kick_candidate`].
       */
      kick_candidate: TxDescriptor<{ candidate: SS58String }>;

      /**
       *See [`Pallet::resign_candidacy`].
       */
      resign_candidacy: TxDescriptor<undefined>;

      /**
       *See [`Pallet::drop_candidate`].
       */
      drop_candidate: TxDescriptor<{ candidate: SS58String }>;

      /**
       *See [`Pallet::cleanup_candidacy`].
       */
      cleanup_candidacy: TxDescriptor<{ candidate: SS58String; max: number }>;

      /**
       *See [`Pallet::cleanup_challenge`].
       */
      cleanup_challenge: TxDescriptor<{ challenge_round: number; max: number }>;
    },
    {
      /**
       *The society is founded by the given identity.
       */
      Founded: PlainDescriptor<{ founder: SS58String }>;

      /**
       *A membership bid just happened. The given account is the candidate's ID and their offer
       *is the second.
       */
      Bid: PlainDescriptor<{ candidate_id: SS58String; offer: bigint }>;

      /**
       *A membership bid just happened by vouching. The given account is the candidate's ID and
       *their offer is the second. The vouching party is the third.
       */
      Vouch: PlainDescriptor<{ candidate_id: SS58String; offer: bigint; vouching: SS58String }>;

      /**
       *A candidate was dropped (due to an excess of bids in the system).
       */
      AutoUnbid: PlainDescriptor<{ candidate: SS58String }>;

      /**
       *A candidate was dropped (by their request).
       */
      Unbid: PlainDescriptor<{ candidate: SS58String }>;

      /**
       *A candidate was dropped (by request of who vouched for them).
       */
      Unvouch: PlainDescriptor<{ candidate: SS58String }>;

      /**
       *A group of candidates have been inducted. The batch's primary is the first value, the
       *batch in full is the second.
       */
      Inducted: PlainDescriptor<{ primary: SS58String; candidates: Anonymize<Ia2lhg7l2hilo3> }>;

      /**
       *A suspended member has been judged.
       */
      SuspendedMemberJudgement: PlainDescriptor<{ who: SS58String; judged: boolean }>;

      /**
       *A candidate has been suspended
       */
      CandidateSuspended: PlainDescriptor<{ candidate: SS58String }>;

      /**
       *A member has been suspended
       */
      MemberSuspended: PlainDescriptor<{ member: SS58String }>;

      /**
       *A member has been challenged
       */
      Challenged: PlainDescriptor<{ member: SS58String }>;

      /**
       *A vote has been placed
       */
      Vote: PlainDescriptor<{ candidate: SS58String; voter: SS58String; vote: boolean }>;

      /**
       *A vote has been placed for a defending member
       */
      DefenderVote: PlainDescriptor<{ voter: SS58String; vote: boolean }>;

      /**
       *A new set of \[params\] has been set for the group.
       */
      NewParams: PlainDescriptor<{ params: Anonymize<I9hkvi4k3kh916> }>;

      /**
       *Society is unfounded.
       */
      Unfounded: PlainDescriptor<{ founder: SS58String }>;

      /**
       *Some funds were deposited into the society account.
       */
      Deposit: PlainDescriptor<{ value: bigint }>;

      /**
       *A \[member\] got elevated to \[rank\].
       */
      Elevated: PlainDescriptor<{ member: SS58String; rank: number }>;
    },
    {
      /**
       *User is not a member.
       */
      NotMember: PlainDescriptor<undefined>;

      /**
       *User is already a member.
       */
      AlreadyMember: PlainDescriptor<undefined>;

      /**
       *User is suspended.
       */
      Suspended: PlainDescriptor<undefined>;

      /**
       *User is not suspended.
       */
      NotSuspended: PlainDescriptor<undefined>;

      /**
       *Nothing to payout.
       */
      NoPayout: PlainDescriptor<undefined>;

      /**
       *Society already founded.
       */
      AlreadyFounded: PlainDescriptor<undefined>;

      /**
       *Not enough in pot to accept candidate.
       */
      InsufficientPot: PlainDescriptor<undefined>;

      /**
       *Member is already vouching or banned from vouching again.
       */
      AlreadyVouching: PlainDescriptor<undefined>;

      /**
       *Member is not vouching.
       */
      NotVouchingOnBidder: PlainDescriptor<undefined>;

      /**
       *Cannot remove the head of the chain.
       */
      Head: PlainDescriptor<undefined>;

      /**
       *Cannot remove the founder.
       */
      Founder: PlainDescriptor<undefined>;

      /**
       *User has already made a bid.
       */
      AlreadyBid: PlainDescriptor<undefined>;

      /**
       *User is already a candidate.
       */
      AlreadyCandidate: PlainDescriptor<undefined>;

      /**
       *User is not a candidate.
       */
      NotCandidate: PlainDescriptor<undefined>;

      /**
       *Too many members in the society.
       */
      MaxMembers: PlainDescriptor<undefined>;

      /**
       *The caller is not the founder.
       */
      NotFounder: PlainDescriptor<undefined>;

      /**
       *The caller is not the head.
       */
      NotHead: PlainDescriptor<undefined>;

      /**
       *The membership cannot be claimed as the candidate was not clearly approved.
       */
      NotApproved: PlainDescriptor<undefined>;

      /**
       *The candidate cannot be kicked as the candidate was not clearly rejected.
       */
      NotRejected: PlainDescriptor<undefined>;

      /**
       *The candidacy cannot be dropped as the candidate was clearly approved.
       */
      Approved: PlainDescriptor<undefined>;

      /**
       *The candidacy cannot be bestowed as the candidate was clearly rejected.
       */
      Rejected: PlainDescriptor<undefined>;

      /**
       *The candidacy cannot be concluded as the voting is still in progress.
       */
      InProgress: PlainDescriptor<undefined>;

      /**
       *The candidacy cannot be pruned until a full additional intake period has passed.
       */
      TooEarly: PlainDescriptor<undefined>;

      /**
       *The skeptic already voted.
       */
      Voted: PlainDescriptor<undefined>;

      /**
       *The skeptic need not vote on candidates from expired rounds.
       */
      Expired: PlainDescriptor<undefined>;

      /**
       *User is not a bidder.
       */
      NotBidder: PlainDescriptor<undefined>;

      /**
       *There is no defender currently.
       */
      NoDefender: PlainDescriptor<undefined>;

      /**
       *Group doesn't exist.
       */
      NotGroup: PlainDescriptor<undefined>;

      /**
       *The member is already elevated to this rank.
       */
      AlreadyElevated: PlainDescriptor<undefined>;

      /**
       *The skeptic has already been punished for this offence.
       */
      AlreadyPunished: PlainDescriptor<undefined>;

      /**
       *Funds are insufficient to pay off society debts.
       */
      InsufficientFunds: PlainDescriptor<undefined>;

      /**
       *The candidate/defender has no stale votes to remove.
       */
      NoVotes: PlainDescriptor<undefined>;
    },
    {
      /**
       * The societies's pallet id
       */
      PalletId: PlainDescriptor<Binary>;

      /**
       * The maximum number of strikes before a member gets funds slashed.
       */
      GraceStrikes: PlainDescriptor<number>;

      /**
       * The amount of incentive paid within each period. Doesn't include VoterTip.
       */
      PeriodSpend: PlainDescriptor<bigint>;

      /**
       * The number of blocks on which new candidates should be voted on. Together with
       * `ClaimPeriod`, this sums to the number of blocks between candidate intake periods.
       */
      VotingPeriod: PlainDescriptor<number>;

      /**
       * The number of blocks on which new candidates can claim their membership and be the
       * named head.
       */
      ClaimPeriod: PlainDescriptor<number>;

      /**
       * The maximum duration of the payout lock.
       */
      MaxLockDuration: PlainDescriptor<number>;

      /**
       * The number of blocks between membership challenges.
       */
      ChallengePeriod: PlainDescriptor<number>;

      /**
       * The maximum number of payouts a member may have waiting unclaimed.
       */
      MaxPayouts: PlainDescriptor<number>;

      /**
       * The maximum number of bids at once.
       */
      MaxBids: PlainDescriptor<number>;
    },
  ];
  Recovery: [
    {
      /**
       * The set of recoverable accounts and their recovery configuration.
       */
      Recoverable: StorageDescriptor<
        [Key: SS58String],
        {
          delay_period: number;
          deposit: bigint;
          friends: Anonymize<Ia2lhg7l2hilo3>;
          threshold: number;
        },
        true
      >;

      /**
       * Active recovery attempts.
       *
       * First account is the account to be recovered, and the second account
       * is the user trying to recover the account.
       */
      ActiveRecoveries: StorageDescriptor<
        [SS58String, SS58String],
        { created: number; deposit: bigint; friends: Anonymize<Ia2lhg7l2hilo3> },
        true
      >;

      /**
       * The list of allowed proxy accounts.
       *
       * Map from the user who can access it to the recovered account.
       */
      Proxy: StorageDescriptor<[Key: SS58String], SS58String, true>;
    },
    {
      /**
       *See [`Pallet::as_recovered`].
       */
      as_recovered: TxDescriptor<{ account: MultiAddress; call: Anonymize<I5v7s5dm78b0mh> }>;

      /**
       *See [`Pallet::set_recovered`].
       */
      set_recovered: TxDescriptor<{ lost: MultiAddress; rescuer: MultiAddress }>;

      /**
       *See [`Pallet::create_recovery`].
       */
      create_recovery: TxDescriptor<{
        friends: Anonymize<Ia2lhg7l2hilo3>;
        threshold: number;
        delay_period: number;
      }>;

      /**
       *See [`Pallet::initiate_recovery`].
       */
      initiate_recovery: TxDescriptor<{ account: MultiAddress }>;

      /**
       *See [`Pallet::vouch_recovery`].
       */
      vouch_recovery: TxDescriptor<{ lost: MultiAddress; rescuer: MultiAddress }>;

      /**
       *See [`Pallet::claim_recovery`].
       */
      claim_recovery: TxDescriptor<{ account: MultiAddress }>;

      /**
       *See [`Pallet::close_recovery`].
       */
      close_recovery: TxDescriptor<{ rescuer: MultiAddress }>;

      /**
       *See [`Pallet::remove_recovery`].
       */
      remove_recovery: TxDescriptor<undefined>;

      /**
       *See [`Pallet::cancel_recovered`].
       */
      cancel_recovered: TxDescriptor<{ account: MultiAddress }>;
    },
    {
      /**
       *A recovery process has been set up for an account.
       */
      RecoveryCreated: PlainDescriptor<{ account: SS58String }>;

      /**
       *A recovery process has been initiated for lost account by rescuer account.
       */
      RecoveryInitiated: PlainDescriptor<{ lost_account: SS58String; rescuer_account: SS58String }>;

      /**
       *A recovery process for lost account by rescuer account has been vouched for by sender.
       */
      RecoveryVouched: PlainDescriptor<{
        lost_account: SS58String;
        rescuer_account: SS58String;
        sender: SS58String;
      }>;

      /**
       *A recovery process for lost account by rescuer account has been closed.
       */
      RecoveryClosed: PlainDescriptor<{ lost_account: SS58String; rescuer_account: SS58String }>;

      /**
       *Lost account has been successfully recovered by rescuer account.
       */
      AccountRecovered: PlainDescriptor<{ lost_account: SS58String; rescuer_account: SS58String }>;

      /**
       *A recovery process has been removed for an account.
       */
      RecoveryRemoved: PlainDescriptor<{ lost_account: SS58String }>;
    },
    {
      /**
       *User is not allowed to make a call on behalf of this account
       */
      NotAllowed: PlainDescriptor<undefined>;

      /**
       *Threshold must be greater than zero
       */
      ZeroThreshold: PlainDescriptor<undefined>;

      /**
       *Friends list must be greater than zero and threshold
       */
      NotEnoughFriends: PlainDescriptor<undefined>;

      /**
       *Friends list must be less than max friends
       */
      MaxFriends: PlainDescriptor<undefined>;

      /**
       *Friends list must be sorted and free of duplicates
       */
      NotSorted: PlainDescriptor<undefined>;

      /**
       *This account is not set up for recovery
       */
      NotRecoverable: PlainDescriptor<undefined>;

      /**
       *This account is already set up for recovery
       */
      AlreadyRecoverable: PlainDescriptor<undefined>;

      /**
       *A recovery process has already started for this account
       */
      AlreadyStarted: PlainDescriptor<undefined>;

      /**
       *A recovery process has not started for this rescuer
       */
      NotStarted: PlainDescriptor<undefined>;

      /**
       *This account is not a friend who can vouch
       */
      NotFriend: PlainDescriptor<undefined>;

      /**
       *The friend must wait until the delay period to vouch for this recovery
       */
      DelayPeriod: PlainDescriptor<undefined>;

      /**
       *This user has already vouched for this recovery
       */
      AlreadyVouched: PlainDescriptor<undefined>;

      /**
       *The threshold for recovering this account has not been met
       */
      Threshold: PlainDescriptor<undefined>;

      /**
       *There are still active recovery attempts that need to be closed
       */
      StillActive: PlainDescriptor<undefined>;

      /**
       *This account is already set up for recovery
       */
      AlreadyProxy: PlainDescriptor<undefined>;

      /**
       *Some internal state is broken.
       */
      BadState: PlainDescriptor<undefined>;
    },
    {
      /**
       * The base amount of currency needed to reserve for creating a recovery configuration.
       *
       * This is held for an additional storage item whose value size is
       * `2 + sizeof(BlockNumber, Balance)` bytes.
       */
      ConfigDepositBase: PlainDescriptor<bigint>;

      /**
       * The amount of currency needed per additional user when creating a recovery
       * configuration.
       *
       * This is held for adding `sizeof(AccountId)` bytes more into a pre-existing storage
       * value.
       */
      FriendDepositFactor: PlainDescriptor<bigint>;

      /**
       * The maximum amount of friends allowed in a recovery configuration.
       *
       * NOTE: The threshold programmed in this Pallet uses u16, so it does
       * not really make sense to have a limit here greater than u16::MAX.
       * But also, that is a lot more than you should probably set this value
       * to anyway...
       */
      MaxFriends: PlainDescriptor<number>;

      /**
       * The base amount of currency needed to reserve for starting a recovery.
       *
       * This is primarily held for deterring malicious recovery attempts, and should
       * have a value large enough that a bad actor would choose not to place this
       * deposit. It also acts to fund additional storage item whose value size is
       * `sizeof(BlockNumber, Balance + T * AccountId)` bytes. Where T is a configurable
       * threshold.
       */
      RecoveryDeposit: PlainDescriptor<bigint>;
    },
  ];
  Vesting: [
    {
      /**
       * Information regarding the vesting of a given account.
       */
      Vesting: StorageDescriptor<[Key: SS58String], Array<Anonymize<I4aro1m78pdrtt>>, true>;

      /**
       * Storage version of the pallet.
       *
       * New networks start with latest version, as determined by the genesis build.
       */
      StorageVersion: StorageDescriptor<[], VestingReleases, false>;
    },
    {
      /**
       *See [`Pallet::vest`].
       */
      vest: TxDescriptor<undefined>;

      /**
       *See [`Pallet::vest_other`].
       */
      vest_other: TxDescriptor<{ target: MultiAddress }>;

      /**
       *See [`Pallet::vested_transfer`].
       */
      vested_transfer: TxDescriptor<{ target: MultiAddress; schedule: Anonymize<I4aro1m78pdrtt> }>;

      /**
       *See [`Pallet::force_vested_transfer`].
       */
      force_vested_transfer: TxDescriptor<{
        source: MultiAddress;
        target: MultiAddress;
        schedule: Anonymize<I4aro1m78pdrtt>;
      }>;

      /**
       *See [`Pallet::merge_schedules`].
       */
      merge_schedules: TxDescriptor<{ schedule1_index: number; schedule2_index: number }>;

      /**
       *See [`Pallet::force_remove_vesting_schedule`].
       */
      force_remove_vesting_schedule: TxDescriptor<{ target: MultiAddress; schedule_index: number }>;
    },
    {
      /**
       *The amount vested has been updated. This could indicate a change in funds available.
       *The balance given is the amount which is left unvested (and thus locked).
       */
      VestingUpdated: PlainDescriptor<{ account: SS58String; unvested: bigint }>;

      /**
       *An \[account\] has become fully vested.
       */
      VestingCompleted: PlainDescriptor<{ account: SS58String }>;
    },
    {
      /**
       *The account given is not vesting.
       */
      NotVesting: PlainDescriptor<undefined>;

      /**
       *The account already has `MaxVestingSchedules` count of schedules and thus
       *cannot add another one. Consider merging existing schedules in order to add another.
       */
      AtMaxVestingSchedules: PlainDescriptor<undefined>;

      /**
       *Amount being transferred is too low to create a vesting schedule.
       */
      AmountLow: PlainDescriptor<undefined>;

      /**
       *An index was out of bounds of the vesting schedules.
       */
      ScheduleIndexOutOfBounds: PlainDescriptor<undefined>;

      /**
       *Failed to create a new schedule because some parameter was invalid.
       */
      InvalidScheduleParams: PlainDescriptor<undefined>;
    },
    {
      /**
       * The minimum amount transferred to call `vested_transfer`.
       */
      MinVestedTransfer: PlainDescriptor<bigint>;

      /**

 */
      MaxVestingSchedules: PlainDescriptor<number>;
    },
  ];
  Scheduler: [
    {
      /**

 */
      IncompleteSince: StorageDescriptor<[], number, true>;

      /**
       * Items to be executed, indexed by the block number that they should be executed on.
       */
      Agenda: StorageDescriptor<[Key: number], Array<Anonymize<Ife7f8jq02avq2>>, false>;

      /**
       * Retry configurations for items to be executed, indexed by task address.
       */
      Retries: StorageDescriptor<
        [Key: [number, number]],
        { total_retries: number; remaining: number; period: number },
        true
      >;

      /**
       * Lookup from a name to the block number and index of the task.
       *
       * For v3 -> v4 the previously unbounded identities are Blake2-256 hashed to form the v4
       * identities.
       */
      Lookup: StorageDescriptor<[Key: Binary], [number, number], true>;
    },
    {
      /**
       *See [`Pallet::schedule`].
       */
      schedule: TxDescriptor<{
        when: number;
        maybe_periodic: Anonymize<I34gtdjipdmjpt>;
        priority: number;
        call: Anonymize<I5v7s5dm78b0mh>;
      }>;

      /**
       *See [`Pallet::cancel`].
       */
      cancel: TxDescriptor<{ when: number; index: number }>;

      /**
       *See [`Pallet::schedule_named`].
       */
      schedule_named: TxDescriptor<{
        id: Binary;
        when: number;
        maybe_periodic: Anonymize<I34gtdjipdmjpt>;
        priority: number;
        call: Anonymize<I5v7s5dm78b0mh>;
      }>;

      /**
       *See [`Pallet::cancel_named`].
       */
      cancel_named: TxDescriptor<{ id: Binary }>;

      /**
       *See [`Pallet::schedule_after`].
       */
      schedule_after: TxDescriptor<{
        after: number;
        maybe_periodic: Anonymize<I34gtdjipdmjpt>;
        priority: number;
        call: Anonymize<I5v7s5dm78b0mh>;
      }>;

      /**
       *See [`Pallet::schedule_named_after`].
       */
      schedule_named_after: TxDescriptor<{
        id: Binary;
        after: number;
        maybe_periodic: Anonymize<I34gtdjipdmjpt>;
        priority: number;
        call: Anonymize<I5v7s5dm78b0mh>;
      }>;

      /**
       *See [`Pallet::set_retry`].
       */
      set_retry: TxDescriptor<{ task: Anonymize<I5g2vv0ckl2m8b>; retries: number; period: number }>;

      /**
       *See [`Pallet::set_retry_named`].
       */
      set_retry_named: TxDescriptor<{ id: Binary; retries: number; period: number }>;

      /**
       *See [`Pallet::cancel_retry`].
       */
      cancel_retry: TxDescriptor<{ task: Anonymize<I5g2vv0ckl2m8b> }>;

      /**
       *See [`Pallet::cancel_retry_named`].
       */
      cancel_retry_named: TxDescriptor<{ id: Binary }>;
    },
    {
      /**
       *Scheduled some task.
       */
      Scheduled: PlainDescriptor<{ when: number; index: number }>;

      /**
       *Canceled some task.
       */
      Canceled: PlainDescriptor<{ when: number; index: number }>;

      /**
       *Dispatched some task.
       */
      Dispatched: PlainDescriptor<{
        task: Anonymize<I5g2vv0ckl2m8b>;
        id: Anonymize<I17k3ujudqd5df>;
        result: Anonymize<Idtdr91jmq5g4i>;
      }>;

      /**
       *Set a retry configuration for some task.
       */
      RetrySet: PlainDescriptor<{
        task: Anonymize<I5g2vv0ckl2m8b>;
        id: Anonymize<I17k3ujudqd5df>;
        period: number;
        retries: number;
      }>;

      /**
       *Cancel a retry configuration for some task.
       */
      RetryCancelled: PlainDescriptor<{
        task: Anonymize<I5g2vv0ckl2m8b>;
        id: Anonymize<I17k3ujudqd5df>;
      }>;

      /**
       *The call for the provided hash was not found so the task has been aborted.
       */
      CallUnavailable: PlainDescriptor<{
        task: Anonymize<I5g2vv0ckl2m8b>;
        id: Anonymize<I17k3ujudqd5df>;
      }>;

      /**
       *The given task was unable to be renewed since the agenda is full at that block.
       */
      PeriodicFailed: PlainDescriptor<{
        task: Anonymize<I5g2vv0ckl2m8b>;
        id: Anonymize<I17k3ujudqd5df>;
      }>;

      /**
       *The given task was unable to be retried since the agenda is full at that block or there
       *was not enough weight to reschedule it.
       */
      RetryFailed: PlainDescriptor<{
        task: Anonymize<I5g2vv0ckl2m8b>;
        id: Anonymize<I17k3ujudqd5df>;
      }>;

      /**
       *The given task can never be executed since it is overweight.
       */
      PermanentlyOverweight: PlainDescriptor<{
        task: Anonymize<I5g2vv0ckl2m8b>;
        id: Anonymize<I17k3ujudqd5df>;
      }>;
    },
    {
      /**
       *Failed to schedule a call
       */
      FailedToSchedule: PlainDescriptor<undefined>;

      /**
       *Cannot find the scheduled call.
       */
      NotFound: PlainDescriptor<undefined>;

      /**
       *Given target block number is in the past.
       */
      TargetBlockNumberInPast: PlainDescriptor<undefined>;

      /**
       *Reschedule failed because it does not change scheduled time.
       */
      RescheduleNoChange: PlainDescriptor<undefined>;

      /**
       *Attempt to use a non-named function on a named task.
       */
      Named: PlainDescriptor<undefined>;
    },
    {
      /**
       * The maximum weight that may be scheduled per block for any dispatchables.
       */
      MaximumWeight: PlainDescriptor<{ ref_time: bigint; proof_size: bigint }>;

      /**
       * The maximum number of scheduled calls in the queue for a single block.
       *
       * NOTE:
       * + Dependent pallets' benchmarks might require a higher limit for the setting. Set a
       * higher limit under `runtime-benchmarks` feature.
       */
      MaxScheduledPerBlock: PlainDescriptor<number>;
    },
  ];
  Proxy: [
    {
      /**
       * The set of account proxies. Maps the account which has delegated to the accounts
       * which are being delegated to, together with the amount held on deposit.
       */
      Proxies: StorageDescriptor<[Key: SS58String], [Anonymize<I4n424tm4s3ajv>, bigint], false>;

      /**
       * The announcements made by the proxy (key).
       */
      Announcements: StorageDescriptor<
        [Key: SS58String],
        [Anonymize<I99svmvk5amkcq>, bigint],
        false
      >;
    },
    {
      /**
       *See [`Pallet::proxy`].
       */
      proxy: TxDescriptor<{
        real: MultiAddress;
        force_proxy_type: Anonymize<I6adql68ljr6ip>;
        call: Anonymize<I5v7s5dm78b0mh>;
      }>;

      /**
       *See [`Pallet::add_proxy`].
       */
      add_proxy: TxDescriptor<{
        delegate: MultiAddress;
        proxy_type: Anonymize<I90b4nim1kup1v>;
        delay: number;
      }>;

      /**
       *See [`Pallet::remove_proxy`].
       */
      remove_proxy: TxDescriptor<{
        delegate: MultiAddress;
        proxy_type: Anonymize<I90b4nim1kup1v>;
        delay: number;
      }>;

      /**
       *See [`Pallet::remove_proxies`].
       */
      remove_proxies: TxDescriptor<undefined>;

      /**
       *See [`Pallet::create_pure`].
       */
      create_pure: TxDescriptor<{
        proxy_type: Anonymize<I90b4nim1kup1v>;
        delay: number;
        index: number;
      }>;

      /**
       *See [`Pallet::kill_pure`].
       */
      kill_pure: TxDescriptor<{
        spawner: MultiAddress;
        proxy_type: Anonymize<I90b4nim1kup1v>;
        index: number;
        height: number;
        ext_index: number;
      }>;

      /**
       *See [`Pallet::announce`].
       */
      announce: TxDescriptor<{ real: MultiAddress; call_hash: Binary }>;

      /**
       *See [`Pallet::remove_announcement`].
       */
      remove_announcement: TxDescriptor<{ real: MultiAddress; call_hash: Binary }>;

      /**
       *See [`Pallet::reject_announcement`].
       */
      reject_announcement: TxDescriptor<{ delegate: MultiAddress; call_hash: Binary }>;

      /**
       *See [`Pallet::proxy_announced`].
       */
      proxy_announced: TxDescriptor<{
        delegate: MultiAddress;
        real: MultiAddress;
        force_proxy_type: Anonymize<I6adql68ljr6ip>;
        call: Anonymize<I5v7s5dm78b0mh>;
      }>;
    },
    {
      /**
       *A proxy was executed correctly, with the given.
       */
      ProxyExecuted: PlainDescriptor<{ result: Anonymize<Idtdr91jmq5g4i> }>;

      /**
       *A pure account has been created by new proxy with given
       *disambiguation index and proxy type.
       */
      PureCreated: PlainDescriptor<{
        pure: SS58String;
        who: SS58String;
        proxy_type: Anonymize<I90b4nim1kup1v>;
        disambiguation_index: number;
      }>;

      /**
       *An announcement was placed to make a call in the future.
       */
      Announced: PlainDescriptor<{ real: SS58String; proxy: SS58String; call_hash: Binary }>;

      /**
       *A proxy was added.
       */
      ProxyAdded: PlainDescriptor<{
        delegator: SS58String;
        delegatee: SS58String;
        proxy_type: Anonymize<I90b4nim1kup1v>;
        delay: number;
      }>;

      /**
       *A proxy was removed.
       */
      ProxyRemoved: PlainDescriptor<{
        delegator: SS58String;
        delegatee: SS58String;
        proxy_type: Anonymize<I90b4nim1kup1v>;
        delay: number;
      }>;
    },
    {
      /**
       *There are too many proxies registered or too many announcements pending.
       */
      TooMany: PlainDescriptor<undefined>;

      /**
       *Proxy registration not found.
       */
      NotFound: PlainDescriptor<undefined>;

      /**
       *Sender is not a proxy of the account to be proxied.
       */
      NotProxy: PlainDescriptor<undefined>;

      /**
       *A call which is incompatible with the proxy type's filter was attempted.
       */
      Unproxyable: PlainDescriptor<undefined>;

      /**
       *Account is already a proxy.
       */
      Duplicate: PlainDescriptor<undefined>;

      /**
       *Call may not be made by proxy because it may escalate its privileges.
       */
      NoPermission: PlainDescriptor<undefined>;

      /**
       *Announcement, if made at all, was made too recently.
       */
      Unannounced: PlainDescriptor<undefined>;

      /**
       *Cannot add self as proxy.
       */
      NoSelfProxy: PlainDescriptor<undefined>;
    },
    {
      /**
       * The base amount of currency needed to reserve for creating a proxy.
       *
       * This is held for an additional storage item whose value size is
       * `sizeof(Balance)` bytes and whose key size is `sizeof(AccountId)` bytes.
       */
      ProxyDepositBase: PlainDescriptor<bigint>;

      /**
       * The amount of currency needed per proxy added.
       *
       * This is held for adding 32 bytes plus an instance of `ProxyType` more into a
       * pre-existing storage value. Thus, when configuring `ProxyDepositFactor` one should take
       * into account `32 + proxy_type.encode().len()` bytes of data.
       */
      ProxyDepositFactor: PlainDescriptor<bigint>;

      /**
       * The maximum amount of proxies allowed for a single account.
       */
      MaxProxies: PlainDescriptor<number>;

      /**
       * The maximum amount of time-delayed announcements that are allowed to be pending.
       */
      MaxPending: PlainDescriptor<number>;

      /**
       * The base amount of currency needed to reserve for creating an announcement.
       *
       * This is held when a new storage item holding a `Balance` is created (typically 16
       * bytes).
       */
      AnnouncementDepositBase: PlainDescriptor<bigint>;

      /**
       * The amount of currency needed per announcement made.
       *
       * This is held for adding an `AccountId`, `Hash` and `BlockNumber` (typically 68 bytes)
       * into a pre-existing storage value.
       */
      AnnouncementDepositFactor: PlainDescriptor<bigint>;
    },
  ];
  Multisig: [
    {
      /**
       * The set of open multisig operations.
       */
      Multisigs: StorageDescriptor<
        [SS58String, Binary],
        {
          when: Anonymize<Itvprrpb0nm3o>;
          deposit: bigint;
          depositor: SS58String;
          approvals: Anonymize<Ia2lhg7l2hilo3>;
        },
        true
      >;
    },
    {
      /**
       *See [`Pallet::as_multi_threshold_1`].
       */
      as_multi_threshold_1: TxDescriptor<{
        other_signatories: Anonymize<Ia2lhg7l2hilo3>;
        call: Anonymize<I5v7s5dm78b0mh>;
      }>;

      /**
       *See [`Pallet::as_multi`].
       */
      as_multi: TxDescriptor<{
        threshold: number;
        other_signatories: Anonymize<Ia2lhg7l2hilo3>;
        maybe_timepoint: Anonymize<I95jfd8j5cr5eh>;
        call: Anonymize<I5v7s5dm78b0mh>;
        max_weight: Anonymize<I4q39t5hn830vp>;
      }>;

      /**
       *See [`Pallet::approve_as_multi`].
       */
      approve_as_multi: TxDescriptor<{
        threshold: number;
        other_signatories: Anonymize<Ia2lhg7l2hilo3>;
        maybe_timepoint: Anonymize<I95jfd8j5cr5eh>;
        call_hash: Binary;
        max_weight: Anonymize<I4q39t5hn830vp>;
      }>;

      /**
       *See [`Pallet::cancel_as_multi`].
       */
      cancel_as_multi: TxDescriptor<{
        threshold: number;
        other_signatories: Anonymize<Ia2lhg7l2hilo3>;
        timepoint: Anonymize<Itvprrpb0nm3o>;
        call_hash: Binary;
      }>;
    },
    {
      /**
       *A new multisig operation has begun.
       */
      NewMultisig: PlainDescriptor<{
        approving: SS58String;
        multisig: SS58String;
        call_hash: Binary;
      }>;

      /**
       *A multisig operation has been approved by someone.
       */
      MultisigApproval: PlainDescriptor<{
        approving: SS58String;
        timepoint: Anonymize<Itvprrpb0nm3o>;
        multisig: SS58String;
        call_hash: Binary;
      }>;

      /**
       *A multisig operation has been executed.
       */
      MultisigExecuted: PlainDescriptor<{
        approving: SS58String;
        timepoint: Anonymize<Itvprrpb0nm3o>;
        multisig: SS58String;
        call_hash: Binary;
        result: Anonymize<Idtdr91jmq5g4i>;
      }>;

      /**
       *A multisig operation has been cancelled.
       */
      MultisigCancelled: PlainDescriptor<{
        cancelling: SS58String;
        timepoint: Anonymize<Itvprrpb0nm3o>;
        multisig: SS58String;
        call_hash: Binary;
      }>;
    },
    {
      /**
       *Threshold must be 2 or greater.
       */
      MinimumThreshold: PlainDescriptor<undefined>;

      /**
       *Call is already approved by this signatory.
       */
      AlreadyApproved: PlainDescriptor<undefined>;

      /**
       *Call doesn't need any (more) approvals.
       */
      NoApprovalsNeeded: PlainDescriptor<undefined>;

      /**
       *There are too few signatories in the list.
       */
      TooFewSignatories: PlainDescriptor<undefined>;

      /**
       *There are too many signatories in the list.
       */
      TooManySignatories: PlainDescriptor<undefined>;

      /**
       *The signatories were provided out of order; they should be ordered.
       */
      SignatoriesOutOfOrder: PlainDescriptor<undefined>;

      /**
       *The sender was contained in the other signatories; it shouldn't be.
       */
      SenderInSignatories: PlainDescriptor<undefined>;

      /**
       *Multisig operation not found when attempting to cancel.
       */
      NotFound: PlainDescriptor<undefined>;

      /**
       *Only the account that originally created the multisig is able to cancel it.
       */
      NotOwner: PlainDescriptor<undefined>;

      /**
       *No timepoint was given, yet the multisig operation is already underway.
       */
      NoTimepoint: PlainDescriptor<undefined>;

      /**
       *A different timepoint was given to the multisig operation that is underway.
       */
      WrongTimepoint: PlainDescriptor<undefined>;

      /**
       *A timepoint was given, yet no multisig operation is underway.
       */
      UnexpectedTimepoint: PlainDescriptor<undefined>;

      /**
       *The maximum weight information provided was too low.
       */
      MaxWeightTooLow: PlainDescriptor<undefined>;

      /**
       *The data to be stored is already stored.
       */
      AlreadyStored: PlainDescriptor<undefined>;
    },
    {
      /**
       * The base amount of currency needed to reserve for creating a multisig execution or to
       * store a dispatch call for later.
       *
       * This is held for an additional storage item whose value size is
       * `4 + sizeof((BlockNumber, Balance, AccountId))` bytes and whose key size is
       * `32 + sizeof(AccountId)` bytes.
       */
      DepositBase: PlainDescriptor<bigint>;

      /**
       * The amount of currency needed per unit threshold when creating a multisig execution.
       *
       * This is held for adding 32 bytes more into a pre-existing storage value.
       */
      DepositFactor: PlainDescriptor<bigint>;

      /**
       * The maximum amount of signatories allowed in the multisig.
       */
      MaxSignatories: PlainDescriptor<number>;
    },
  ];
  Preimage: [
    {
      /**
       * The request status of a given hash.
       */
      StatusFor: StorageDescriptor<[Key: Binary], PreimageRequestStatus, true>;

      /**
       * The request status of a given hash.
       */
      RequestStatusFor: StorageDescriptor<[Key: Binary], PreimageRequestStatus1, true>;

      /**

 */
      PreimageFor: StorageDescriptor<[Key: [Binary, number]], Binary, true>;
    },
    {
      /**
       *See [`Pallet::note_preimage`].
       */
      note_preimage: TxDescriptor<{ bytes: Binary }>;

      /**
       *See [`Pallet::unnote_preimage`].
       */
      unnote_preimage: TxDescriptor<{ hash: Binary }>;

      /**
       *See [`Pallet::request_preimage`].
       */
      request_preimage: TxDescriptor<{ hash: Binary }>;

      /**
       *See [`Pallet::unrequest_preimage`].
       */
      unrequest_preimage: TxDescriptor<{ hash: Binary }>;

      /**
       *See [`Pallet::ensure_updated`].
       */
      ensure_updated: TxDescriptor<{ hashes: Anonymize<Idhnf6rtqoslea> }>;
    },
    {
      /**
       *A preimage has been noted.
       */
      Noted: PlainDescriptor<{ hash: Binary }>;

      /**
       *A preimage has been requested.
       */
      Requested: PlainDescriptor<{ hash: Binary }>;

      /**
       *A preimage has ben cleared.
       */
      Cleared: PlainDescriptor<{ hash: Binary }>;
    },
    {
      /**
       *Preimage is too large to store on-chain.
       */
      TooBig: PlainDescriptor<undefined>;

      /**
       *Preimage has already been noted on-chain.
       */
      AlreadyNoted: PlainDescriptor<undefined>;

      /**
       *The user is not authorized to perform this action.
       */
      NotAuthorized: PlainDescriptor<undefined>;

      /**
       *The preimage cannot be removed since it has not yet been noted.
       */
      NotNoted: PlainDescriptor<undefined>;

      /**
       *A preimage may not be removed when there are outstanding requests.
       */
      Requested: PlainDescriptor<undefined>;

      /**
       *The preimage request cannot be removed since no outstanding requests exist.
       */
      NotRequested: PlainDescriptor<undefined>;

      /**
       *More than `MAX_HASH_UPGRADE_BULK_COUNT` hashes were requested to be upgraded at once.
       */
      TooMany: PlainDescriptor<undefined>;

      /**
       *Too few hashes were requested to be upgraded (i.e. zero).
       */
      TooFew: PlainDescriptor<undefined>;
    },
    {},
  ];
  AssetRate: [
    {
      /**
       * Maps an asset to its fixed point representation in the native balance.
       *
       * E.g. `native_amount = asset_amount * ConversionRateToNative::<T>::get(asset_kind)`
       */
      ConversionRateToNative: StorageDescriptor<
        [Key: PolkadotRuntimeCommonImplsVersionedLocatableAsset],
        bigint,
        true
      >;
    },
    {
      /**
       *See [`Pallet::create`].
       */
      create: TxDescriptor<{
        asset_kind: PolkadotRuntimeCommonImplsVersionedLocatableAsset;
        rate: bigint;
      }>;

      /**
       *See [`Pallet::update`].
       */
      update: TxDescriptor<{
        asset_kind: PolkadotRuntimeCommonImplsVersionedLocatableAsset;
        rate: bigint;
      }>;

      /**
       *See [`Pallet::remove`].
       */
      remove: TxDescriptor<{ asset_kind: PolkadotRuntimeCommonImplsVersionedLocatableAsset }>;
    },
    {
      /**

 */
      AssetRateCreated: PlainDescriptor<{
        asset_kind: PolkadotRuntimeCommonImplsVersionedLocatableAsset;
        rate: bigint;
      }>;

      /**

 */
      AssetRateRemoved: PlainDescriptor<{
        asset_kind: PolkadotRuntimeCommonImplsVersionedLocatableAsset;
      }>;

      /**

 */
      AssetRateUpdated: PlainDescriptor<{
        asset_kind: PolkadotRuntimeCommonImplsVersionedLocatableAsset;
        old: bigint;
        new: bigint;
      }>;
    },
    {
      /**
       *The given asset ID is unknown.
       */
      UnknownAssetKind: PlainDescriptor<undefined>;

      /**
       *The given asset ID already has an assigned conversion rate and cannot be re-created.
       */
      AlreadyExists: PlainDescriptor<undefined>;

      /**
       *Overflow ocurred when calculating the inverse rate.
       */
      Overflow: PlainDescriptor<undefined>;
    },
    {},
  ];
  Bounties: [
    {
      /**
       * Number of bounty proposals that have been made.
       */
      BountyCount: StorageDescriptor<[], number, false>;

      /**
       * Bounties that have been made.
       */
      Bounties: StorageDescriptor<
        [Key: number],
        {
          proposer: SS58String;
          value: bigint;
          fee: bigint;
          curator_deposit: bigint;
          bond: bigint;
          status: BountiesBountyStatus;
        },
        true
      >;

      /**
       * The description of each bounty.
       */
      BountyDescriptions: StorageDescriptor<[Key: number], Binary, true>;

      /**
       * Bounty indices that have been approved but not yet funded.
       */
      BountyApprovals: StorageDescriptor<[], Array<number>, false>;
    },
    {
      /**
       *See [`Pallet::propose_bounty`].
       */
      propose_bounty: TxDescriptor<{ value: bigint; description: Binary }>;

      /**
       *See [`Pallet::approve_bounty`].
       */
      approve_bounty: TxDescriptor<{ bounty_id: number }>;

      /**
       *See [`Pallet::propose_curator`].
       */
      propose_curator: TxDescriptor<{ bounty_id: number; curator: MultiAddress; fee: bigint }>;

      /**
       *See [`Pallet::unassign_curator`].
       */
      unassign_curator: TxDescriptor<{ bounty_id: number }>;

      /**
       *See [`Pallet::accept_curator`].
       */
      accept_curator: TxDescriptor<{ bounty_id: number }>;

      /**
       *See [`Pallet::award_bounty`].
       */
      award_bounty: TxDescriptor<{ bounty_id: number; beneficiary: MultiAddress }>;

      /**
       *See [`Pallet::claim_bounty`].
       */
      claim_bounty: TxDescriptor<{ bounty_id: number }>;

      /**
       *See [`Pallet::close_bounty`].
       */
      close_bounty: TxDescriptor<{ bounty_id: number }>;

      /**
       *See [`Pallet::extend_bounty_expiry`].
       */
      extend_bounty_expiry: TxDescriptor<{ bounty_id: number; remark: Binary }>;
    },
    {
      /**
       *New bounty proposal.
       */
      BountyProposed: PlainDescriptor<{ index: number }>;

      /**
       *A bounty proposal was rejected; funds were slashed.
       */
      BountyRejected: PlainDescriptor<{ index: number; bond: bigint }>;

      /**
       *A bounty proposal is funded and became active.
       */
      BountyBecameActive: PlainDescriptor<{ index: number }>;

      /**
       *A bounty is awarded to a beneficiary.
       */
      BountyAwarded: PlainDescriptor<{ index: number; beneficiary: SS58String }>;

      /**
       *A bounty is claimed by beneficiary.
       */
      BountyClaimed: PlainDescriptor<{ index: number; payout: bigint; beneficiary: SS58String }>;

      /**
       *A bounty is cancelled.
       */
      BountyCanceled: PlainDescriptor<{ index: number }>;

      /**
       *A bounty expiry is extended.
       */
      BountyExtended: PlainDescriptor<{ index: number }>;

      /**
       *A bounty is approved.
       */
      BountyApproved: PlainDescriptor<{ index: number }>;

      /**
       *A bounty curator is proposed.
       */
      CuratorProposed: PlainDescriptor<{ bounty_id: number; curator: SS58String }>;

      /**
       *A bounty curator is unassigned.
       */
      CuratorUnassigned: PlainDescriptor<{ bounty_id: number }>;

      /**
       *A bounty curator is accepted.
       */
      CuratorAccepted: PlainDescriptor<{ bounty_id: number; curator: SS58String }>;
    },
    {
      /**
       *Proposer's balance is too low.
       */
      InsufficientProposersBalance: PlainDescriptor<undefined>;

      /**
       *No proposal or bounty at that index.
       */
      InvalidIndex: PlainDescriptor<undefined>;

      /**
       *The reason given is just too big.
       */
      ReasonTooBig: PlainDescriptor<undefined>;

      /**
       *The bounty status is unexpected.
       */
      UnexpectedStatus: PlainDescriptor<undefined>;

      /**
       *Require bounty curator.
       */
      RequireCurator: PlainDescriptor<undefined>;

      /**
       *Invalid bounty value.
       */
      InvalidValue: PlainDescriptor<undefined>;

      /**
       *Invalid bounty fee.
       */
      InvalidFee: PlainDescriptor<undefined>;

      /**
       *A bounty payout is pending.
       *To cancel the bounty, you must unassign and slash the curator.
       */
      PendingPayout: PlainDescriptor<undefined>;

      /**
       *The bounties cannot be claimed/closed because it's still in the countdown period.
       */
      Premature: PlainDescriptor<undefined>;

      /**
       *The bounty cannot be closed because it has active child bounties.
       */
      HasActiveChildBounty: PlainDescriptor<undefined>;

      /**
       *Too many approvals are already queued.
       */
      TooManyQueued: PlainDescriptor<undefined>;
    },
    {
      /**
       * The amount held on deposit for placing a bounty proposal.
       */
      BountyDepositBase: PlainDescriptor<bigint>;

      /**
       * The delay period for which a bounty beneficiary need to wait before claim the payout.
       */
      BountyDepositPayoutDelay: PlainDescriptor<number>;

      /**
       * Bounty duration in blocks.
       */
      BountyUpdatePeriod: PlainDescriptor<number>;

      /**
       * The curator deposit is calculated as a percentage of the curator fee.
       *
       * This deposit has optional upper and lower bounds with `CuratorDepositMax` and
       * `CuratorDepositMin`.
       */
      CuratorDepositMultiplier: PlainDescriptor<number>;

      /**
       * Maximum amount of funds that should be placed in a deposit for making a proposal.
       */
      CuratorDepositMax: PlainDescriptor<bigint | undefined>;

      /**
       * Minimum amount of funds that should be placed in a deposit for making a proposal.
       */
      CuratorDepositMin: PlainDescriptor<bigint | undefined>;

      /**
       * Minimum value for a bounty.
       */
      BountyValueMinimum: PlainDescriptor<bigint>;

      /**
       * The amount held on deposit per byte within the tip report reason or bounty description.
       */
      DataDepositPerByte: PlainDescriptor<bigint>;

      /**
       * Maximum acceptable reason length.
       *
       * Benchmarks depend on this value, be sure to update weights file when changing this value
       */
      MaximumReasonLength: PlainDescriptor<number>;
    },
  ];
  ChildBounties: [
    {
      /**
       * Number of total child bounties.
       */
      ChildBountyCount: StorageDescriptor<[], number, false>;

      /**
       * Number of child bounties per parent bounty.
       * Map of parent bounty index to number of child bounties.
       */
      ParentChildBounties: StorageDescriptor<[Key: number], number, false>;

      /**
       * Child bounties that have been added.
       */
      ChildBounties: StorageDescriptor<
        [number, number],
        {
          parent_bounty: number;
          value: bigint;
          fee: bigint;
          curator_deposit: bigint;
          status: ChildBountyStatus;
        },
        true
      >;

      /**
       * The description of each child-bounty.
       */
      ChildBountyDescriptions: StorageDescriptor<[Key: number], Binary, true>;

      /**
       * The cumulative child-bounty curator fee for each parent bounty.
       */
      ChildrenCuratorFees: StorageDescriptor<[Key: number], bigint, false>;
    },
    {
      /**
       *See [`Pallet::add_child_bounty`].
       */
      add_child_bounty: TxDescriptor<{
        parent_bounty_id: number;
        value: bigint;
        description: Binary;
      }>;

      /**
       *See [`Pallet::propose_curator`].
       */
      propose_curator: TxDescriptor<{
        parent_bounty_id: number;
        child_bounty_id: number;
        curator: MultiAddress;
        fee: bigint;
      }>;

      /**
       *See [`Pallet::accept_curator`].
       */
      accept_curator: TxDescriptor<{ parent_bounty_id: number; child_bounty_id: number }>;

      /**
       *See [`Pallet::unassign_curator`].
       */
      unassign_curator: TxDescriptor<{ parent_bounty_id: number; child_bounty_id: number }>;

      /**
       *See [`Pallet::award_child_bounty`].
       */
      award_child_bounty: TxDescriptor<{
        parent_bounty_id: number;
        child_bounty_id: number;
        beneficiary: MultiAddress;
      }>;

      /**
       *See [`Pallet::claim_child_bounty`].
       */
      claim_child_bounty: TxDescriptor<{ parent_bounty_id: number; child_bounty_id: number }>;

      /**
       *See [`Pallet::close_child_bounty`].
       */
      close_child_bounty: TxDescriptor<{ parent_bounty_id: number; child_bounty_id: number }>;
    },
    {
      /**
       *A child-bounty is added.
       */
      Added: PlainDescriptor<{ index: number; child_index: number }>;

      /**
       *A child-bounty is awarded to a beneficiary.
       */
      Awarded: PlainDescriptor<{ index: number; child_index: number; beneficiary: SS58String }>;

      /**
       *A child-bounty is claimed by beneficiary.
       */
      Claimed: PlainDescriptor<{
        index: number;
        child_index: number;
        payout: bigint;
        beneficiary: SS58String;
      }>;

      /**
       *A child-bounty is cancelled.
       */
      Canceled: PlainDescriptor<{ index: number; child_index: number }>;
    },
    {
      /**
       *The parent bounty is not in active state.
       */
      ParentBountyNotActive: PlainDescriptor<undefined>;

      /**
       *The bounty balance is not enough to add new child-bounty.
       */
      InsufficientBountyBalance: PlainDescriptor<undefined>;

      /**
       *Number of child bounties exceeds limit `MaxActiveChildBountyCount`.
       */
      TooManyChildBounties: PlainDescriptor<undefined>;
    },
    {
      /**
       * Maximum number of child bounties that can be added to a parent bounty.
       */
      MaxActiveChildBountyCount: PlainDescriptor<number>;

      /**
       * Minimum value for a child-bounty.
       */
      ChildBountyValueMinimum: PlainDescriptor<bigint>;
    },
  ];
  Nis: [
    {
      /**
       * The totals of items and balances within each queue. Saves a lot of storage reads in the
       * case of sparsely packed queues.
       *
       * The vector is indexed by duration in `Period`s, offset by one, so information on the queue
       * whose duration is one `Period` would be storage `0`.
       */
      QueueTotals: StorageDescriptor<[], Array<Anonymize<I4ojmnsk1dchql>>, false>;

      /**
       * The queues of bids. Indexed by duration (in `Period`s).
       */
      Queues: StorageDescriptor<[Key: number], Array<Anonymize<Id5fm4p8lj5qgi>>, false>;

      /**
       * Summary information over the general state.
       */
      Summary: StorageDescriptor<
        [],
        {
          proportion_owed: bigint;
          index: number;
          thawed: bigint;
          last_period: number;
          receipts_on_hold: bigint;
        },
        false
      >;

      /**
       * The currently outstanding receipts, indexed according to the order of creation.
       */
      Receipts: StorageDescriptor<
        [Key: number],
        { proportion: bigint; owner: Anonymize<I92hdo1clkbp4g>; expiry: number },
        true
      >;
    },
    {
      /**
       *See [`Pallet::place_bid`].
       */
      place_bid: TxDescriptor<{ amount: bigint; duration: number }>;

      /**
       *See [`Pallet::retract_bid`].
       */
      retract_bid: TxDescriptor<{ amount: bigint; duration: number }>;

      /**
       *See [`Pallet::fund_deficit`].
       */
      fund_deficit: TxDescriptor<undefined>;

      /**
       *See [`Pallet::thaw_private`].
       */
      thaw_private: TxDescriptor<{ index: number; maybe_proportion: Anonymize<I35p85j063s0il> }>;

      /**
       *See [`Pallet::thaw_communal`].
       */
      thaw_communal: TxDescriptor<{ index: number }>;

      /**
       *See [`Pallet::communify`].
       */
      communify: TxDescriptor<{ index: number }>;

      /**
       *See [`Pallet::privatize`].
       */
      privatize: TxDescriptor<{ index: number }>;
    },
    {
      /**
       *A bid was successfully placed.
       */
      BidPlaced: PlainDescriptor<{ who: SS58String; amount: bigint; duration: number }>;

      /**
       *A bid was successfully removed (before being accepted).
       */
      BidRetracted: PlainDescriptor<{ who: SS58String; amount: bigint; duration: number }>;

      /**
       *A bid was dropped from a queue because of another, more substantial, bid was present.
       */
      BidDropped: PlainDescriptor<{ who: SS58String; amount: bigint; duration: number }>;

      /**
       *A bid was accepted. The balance may not be released until expiry.
       */
      Issued: PlainDescriptor<{
        index: number;
        expiry: number;
        who: SS58String;
        proportion: bigint;
        amount: bigint;
      }>;

      /**
       *An receipt has been (at least partially) thawed.
       */
      Thawed: PlainDescriptor<{
        index: number;
        who: SS58String;
        proportion: bigint;
        amount: bigint;
        dropped: boolean;
      }>;

      /**
       *An automatic funding of the deficit was made.
       */
      Funded: PlainDescriptor<{ deficit: bigint }>;

      /**
       *A receipt was transfered.
       */
      Transferred: PlainDescriptor<{ from: SS58String; to: SS58String; index: number }>;
    },
    {
      /**
       *The duration of the bid is less than one.
       */
      DurationTooSmall: PlainDescriptor<undefined>;

      /**
       *The duration is the bid is greater than the number of queues.
       */
      DurationTooBig: PlainDescriptor<undefined>;

      /**
       *The amount of the bid is less than the minimum allowed.
       */
      AmountTooSmall: PlainDescriptor<undefined>;

      /**
       *The queue for the bid's duration is full and the amount bid is too low to get in
       *through replacing an existing bid.
       */
      BidTooLow: PlainDescriptor<undefined>;

      /**
       *Receipt index is unknown.
       */
      UnknownReceipt: PlainDescriptor<undefined>;

      /**
       *Not the owner of the receipt.
       */
      NotOwner: PlainDescriptor<undefined>;

      /**
       *Bond not yet at expiry date.
       */
      NotExpired: PlainDescriptor<undefined>;

      /**
       *The given bid for retraction is not found.
       */
      UnknownBid: PlainDescriptor<undefined>;

      /**
       *The portion supplied is beyond the value of the receipt.
       */
      PortionTooBig: PlainDescriptor<undefined>;

      /**
       *Not enough funds are held to pay out.
       */
      Unfunded: PlainDescriptor<undefined>;

      /**
       *There are enough funds for what is required.
       */
      AlreadyFunded: PlainDescriptor<undefined>;

      /**
       *The thaw throttle has been reached for this period.
       */
      Throttled: PlainDescriptor<undefined>;

      /**
       *The operation would result in a receipt worth an insignficant value.
       */
      MakesDust: PlainDescriptor<undefined>;

      /**
       *The receipt is already communal.
       */
      AlreadyCommunal: PlainDescriptor<undefined>;

      /**
       *The receipt is already private.
       */
      AlreadyPrivate: PlainDescriptor<undefined>;
    },
    {
      /**
       * The treasury's pallet id, used for deriving its sovereign account ID.
       */
      PalletId: PlainDescriptor<Binary>;

      /**
       * Number of duration queues in total. This sets the maximum duration supported, which is
       * this value multiplied by `Period`.
       */
      QueueCount: PlainDescriptor<number>;

      /**
       * Maximum number of items that may be in each duration queue.
       *
       * Must be larger than zero.
       */
      MaxQueueLen: PlainDescriptor<number>;

      /**
       * Portion of the queue which is free from ordering and just a FIFO.
       *
       * Must be no greater than `MaxQueueLen`.
       */
      FifoQueueLen: PlainDescriptor<number>;

      /**
       * The base period for the duration queues. This is the common multiple across all
       * supported freezing durations that can be bid upon.
       */
      BasePeriod: PlainDescriptor<number>;

      /**
       * The minimum amount of funds that may be placed in a bid. Note that this
       * does not actually limit the amount which may be represented in a receipt since bids may
       * be split up by the system.
       *
       * It should be at least big enough to ensure that there is no possible storage spam attack
       * or queue-filling attack.
       */
      MinBid: PlainDescriptor<bigint>;

      /**
       * The minimum amount of funds which may intentionally be left remaining under a single
       * receipt.
       */
      MinReceipt: PlainDescriptor<bigint>;

      /**
       * The number of blocks between consecutive attempts to dequeue bids and create receipts.
       *
       * A larger value results in fewer storage hits each block, but a slower period to get to
       * the target.
       */
      IntakePeriod: PlainDescriptor<number>;

      /**
       * The maximum amount of bids that can consolidated into receipts in a single intake. A
       * larger value here means less of the block available for transactions should there be a
       * glut of bids.
       */
      MaxIntakeWeight: PlainDescriptor<{ ref_time: bigint; proof_size: bigint }>;

      /**
       * The maximum proportion which may be thawed and the period over which it is reset.
       */
      ThawThrottle: PlainDescriptor<[bigint, number]>;
    },
  ];
  NisCounterpartBalances: [
    {
      /**
       * The total units issued in the system.
       */
      TotalIssuance: StorageDescriptor<[], bigint, false>;

      /**
       * The total units of outstanding deactivated balance in the system.
       */
      InactiveIssuance: StorageDescriptor<[], bigint, false>;

      /**
       * The Balances pallet example of storing the balance of an account.
       *
       * # Example
       *
       * ```nocompile
       *  impl pallet_balances::Config for Runtime {
       *    type AccountStore = StorageMapShim<Self::Account<Runtime>, frame_system::Provider<Runtime>, AccountId, Self::AccountData<Balance>>
       *  }
       * ```
       *
       * You can also store the balance of an account in the `System` pallet.
       *
       * # Example
       *
       * ```nocompile
       *  impl pallet_balances::Config for Runtime {
       *   type AccountStore = System
       *  }
       * ```
       *
       * But this comes with tradeoffs, storing account balances in the system pallet stores
       * `frame_system` data alongside the account data contrary to storing account balances in the
       * `Balances` pallet, which uses a `StorageMap` to store balances data only.
       * NOTE: This is only used in the case that this pallet is used to store balances.
       */
      Account: StorageDescriptor<
        [Key: SS58String],
        { free: bigint; reserved: bigint; frozen: bigint; flags: bigint },
        false
      >;

      /**
       * Any liquidity locks on some account balances.
       * NOTE: Should only be accessed when setting, changing and freeing a lock.
       */
      Locks: StorageDescriptor<[Key: SS58String], Array<Anonymize<I5b29v4qfq4tu7>>, false>;

      /**
       * Named reserves on some account balances.
       */
      Reserves: StorageDescriptor<[Key: SS58String], Array<Anonymize<I32btm6htd9bck>>, false>;

      /**
       * Holds on account balances.
       */
      Holds: StorageDescriptor<[Key: SS58String], Array<Anonymize<Iemrlqc7oc4k5g>>, false>;

      /**
       * Freeze locks on account balances.
       */
      Freezes: StorageDescriptor<[Key: SS58String], Array<Anonymize<I7qdm60946h5u9>>, false>;
    },
    {
      /**
       *See [`Pallet::transfer_allow_death`].
       */
      transfer_allow_death: TxDescriptor<{ dest: MultiAddress; value: bigint }>;

      /**
       *See [`Pallet::force_transfer`].
       */
      force_transfer: TxDescriptor<{ source: MultiAddress; dest: MultiAddress; value: bigint }>;

      /**
       *See [`Pallet::transfer_keep_alive`].
       */
      transfer_keep_alive: TxDescriptor<{ dest: MultiAddress; value: bigint }>;

      /**
       *See [`Pallet::transfer_all`].
       */
      transfer_all: TxDescriptor<{ dest: MultiAddress; keep_alive: boolean }>;

      /**
       *See [`Pallet::force_unreserve`].
       */
      force_unreserve: TxDescriptor<{ who: MultiAddress; amount: bigint }>;

      /**
       *See [`Pallet::upgrade_accounts`].
       */
      upgrade_accounts: TxDescriptor<{ who: Anonymize<Ia2lhg7l2hilo3> }>;

      /**
       *See [`Pallet::force_set_balance`].
       */
      force_set_balance: TxDescriptor<{ who: MultiAddress; new_free: bigint }>;

      /**
       *See [`Pallet::force_adjust_total_issuance`].
       */
      force_adjust_total_issuance: TxDescriptor<{
        direction: BalancesTypesAdjustmentDirection;
        delta: bigint;
      }>;
    },
    {
      /**
       *An account was created with some free balance.
       */
      Endowed: PlainDescriptor<{ account: SS58String; free_balance: bigint }>;

      /**
       *An account was removed whose balance was non-zero but below ExistentialDeposit,
       *resulting in an outright loss.
       */
      DustLost: PlainDescriptor<{ account: SS58String; amount: bigint }>;

      /**
       *Transfer succeeded.
       */
      Transfer: PlainDescriptor<{ from: SS58String; to: SS58String; amount: bigint }>;

      /**
       *A balance was set by root.
       */
      BalanceSet: PlainDescriptor<{ who: SS58String; free: bigint }>;

      /**
       *Some balance was reserved (moved from free to reserved).
       */
      Reserved: PlainDescriptor<{ who: SS58String; amount: bigint }>;

      /**
       *Some balance was unreserved (moved from reserved to free).
       */
      Unreserved: PlainDescriptor<{ who: SS58String; amount: bigint }>;

      /**
       *Some balance was moved from the reserve of the first account to the second account.
       *Final argument indicates the destination balance type.
       */
      ReserveRepatriated: PlainDescriptor<{
        from: SS58String;
        to: SS58String;
        amount: bigint;
        destination_status: BalanceStatus;
      }>;

      /**
       *Some amount was deposited (e.g. for transaction fees).
       */
      Deposit: PlainDescriptor<{ who: SS58String; amount: bigint }>;

      /**
       *Some amount was withdrawn from the account (e.g. for transaction fees).
       */
      Withdraw: PlainDescriptor<{ who: SS58String; amount: bigint }>;

      /**
       *Some amount was removed from the account (e.g. for misbehavior).
       */
      Slashed: PlainDescriptor<{ who: SS58String; amount: bigint }>;

      /**
       *Some amount was minted into an account.
       */
      Minted: PlainDescriptor<{ who: SS58String; amount: bigint }>;

      /**
       *Some amount was burned from an account.
       */
      Burned: PlainDescriptor<{ who: SS58String; amount: bigint }>;

      /**
       *Some amount was suspended from an account (it can be restored later).
       */
      Suspended: PlainDescriptor<{ who: SS58String; amount: bigint }>;

      /**
       *Some amount was restored into an account.
       */
      Restored: PlainDescriptor<{ who: SS58String; amount: bigint }>;

      /**
       *An account was upgraded.
       */
      Upgraded: PlainDescriptor<{ who: SS58String }>;

      /**
       *Total issuance was increased by `amount`, creating a credit to be balanced.
       */
      Issued: PlainDescriptor<{ amount: bigint }>;

      /**
       *Total issuance was decreased by `amount`, creating a debt to be balanced.
       */
      Rescinded: PlainDescriptor<{ amount: bigint }>;

      /**
       *Some balance was locked.
       */
      Locked: PlainDescriptor<{ who: SS58String; amount: bigint }>;

      /**
       *Some balance was unlocked.
       */
      Unlocked: PlainDescriptor<{ who: SS58String; amount: bigint }>;

      /**
       *Some balance was frozen.
       */
      Frozen: PlainDescriptor<{ who: SS58String; amount: bigint }>;

      /**
       *Some balance was thawed.
       */
      Thawed: PlainDescriptor<{ who: SS58String; amount: bigint }>;

      /**
       *The `TotalIssuance` was forcefully changed.
       */
      TotalIssuanceForced: PlainDescriptor<{ old: bigint; new: bigint }>;
    },
    {
      /**
       *Vesting balance too high to send value.
       */
      VestingBalance: PlainDescriptor<undefined>;

      /**
       *Account liquidity restrictions prevent withdrawal.
       */
      LiquidityRestrictions: PlainDescriptor<undefined>;

      /**
       *Balance too low to send value.
       */
      InsufficientBalance: PlainDescriptor<undefined>;

      /**
       *Value too low to create account due to existential deposit.
       */
      ExistentialDeposit: PlainDescriptor<undefined>;

      /**
       *Transfer/payment would kill account.
       */
      Expendability: PlainDescriptor<undefined>;

      /**
       *A vesting schedule already exists for this account.
       */
      ExistingVestingSchedule: PlainDescriptor<undefined>;

      /**
       *Beneficiary account must pre-exist.
       */
      DeadAccount: PlainDescriptor<undefined>;

      /**
       *Number of named reserves exceed `MaxReserves`.
       */
      TooManyReserves: PlainDescriptor<undefined>;

      /**
       *Number of holds exceed `VariantCountOf<T::RuntimeHoldReason>`.
       */
      TooManyHolds: PlainDescriptor<undefined>;

      /**
       *Number of freezes exceed `MaxFreezes`.
       */
      TooManyFreezes: PlainDescriptor<undefined>;

      /**
       *The issuance cannot be modified since it is already deactivated.
       */
      IssuanceDeactivated: PlainDescriptor<undefined>;

      /**
       *The delta cannot be zero.
       */
      DeltaZero: PlainDescriptor<undefined>;
    },
    {
      /**
       * The minimum amount required to keep an account open. MUST BE GREATER THAN ZERO!
       *
       * If you *really* need it to be zero, you can enable the feature `insecure_zero_ed` for
       * this pallet. However, you do so at your own risk: this will open up a major DoS vector.
       * In case you have multiple sources of provider references, you may also get unexpected
       * behaviour if you set this to zero.
       *
       * Bottom line: Do yourself a favour and make it at least one!
       */
      ExistentialDeposit: PlainDescriptor<bigint>;

      /**
       * The maximum number of locks that should exist on an account.
       * Not strictly enforced, but used for weight estimation.
       */
      MaxLocks: PlainDescriptor<number>;

      /**
       * The maximum number of named reserves that can exist on an account.
       */
      MaxReserves: PlainDescriptor<number>;

      /**
       * The maximum number of individual freeze locks that can exist on an account at any time.
       */
      MaxFreezes: PlainDescriptor<number>;
    },
  ];
  ParachainsOrigin: [{}, {}, {}, {}, {}];
  Configuration: [
    {
      /**
       * The active configuration for the current session.
       */
      ActiveConfig: StorageDescriptor<
        [],
        {
          max_code_size: number;
          max_head_data_size: number;
          max_upward_queue_count: number;
          max_upward_queue_size: number;
          max_upward_message_size: number;
          max_upward_message_num_per_candidate: number;
          hrmp_max_message_num_per_candidate: number;
          validation_upgrade_cooldown: number;
          validation_upgrade_delay: number;
          async_backing_params: Anonymize<Iavuvfkop6318c>;
          max_pov_size: number;
          max_downward_message_size: number;
          hrmp_max_parachain_outbound_channels: number;
          hrmp_sender_deposit: bigint;
          hrmp_recipient_deposit: bigint;
          hrmp_channel_max_capacity: number;
          hrmp_channel_max_total_size: number;
          hrmp_max_parachain_inbound_channels: number;
          hrmp_channel_max_message_size: number;
          executor_params: Anonymize<I79n5nka727sg9>;
          code_retention_period: number;
          coretime_cores: number;
          on_demand_retries: number;
          on_demand_queue_max_size: number;
          on_demand_target_queue_utilization: number;
          on_demand_fee_variability: number;
          on_demand_base_fee: bigint;
          on_demand_ttl: number;
          group_rotation_frequency: number;
          paras_availability_period: number;
          scheduling_lookahead: number;
          max_validators_per_core: Anonymize<I4arjljr6dpflb>;
          max_validators: Anonymize<I4arjljr6dpflb>;
          dispute_period: number;
          dispute_post_conclusion_acceptance_period: number;
          no_show_slots: number;
          n_delay_tranches: number;
          zeroth_delay_tranche_width: number;
          needed_approvals: number;
          relay_vrf_modulo_samples: number;
          pvf_voting_ttl: number;
          minimum_validation_upgrade_delay: number;
          minimum_backing_votes: number;
          node_features: { bytes: Uint8Array; bitsLen: number };
          approval_voting_params: number;
        },
        false
      >;

      /**
       * Pending configuration changes.
       *
       * This is a list of configuration changes, each with a session index at which it should
       * be applied.
       *
       * The list is sorted ascending by session index. Also, this list can only contain at most
       * 2 items: for the next session and for the `scheduled_session`.
       */
      PendingConfigs: StorageDescriptor<[], Array<Anonymize<If59lb4asqkhca>>, false>;

      /**
       * If this is set, then the configuration setters will bypass the consistency checks. This
       * is meant to be used only as the last resort.
       */
      BypassConsistencyCheck: StorageDescriptor<[], boolean, false>;
    },
    {
      /**
       *See [`Pallet::set_validation_upgrade_cooldown`].
       */
      set_validation_upgrade_cooldown: TxDescriptor<{ new: number }>;

      /**
       *See [`Pallet::set_validation_upgrade_delay`].
       */
      set_validation_upgrade_delay: TxDescriptor<{ new: number }>;

      /**
       *See [`Pallet::set_code_retention_period`].
       */
      set_code_retention_period: TxDescriptor<{ new: number }>;

      /**
       *See [`Pallet::set_max_code_size`].
       */
      set_max_code_size: TxDescriptor<{ new: number }>;

      /**
       *See [`Pallet::set_max_pov_size`].
       */
      set_max_pov_size: TxDescriptor<{ new: number }>;

      /**
       *See [`Pallet::set_max_head_data_size`].
       */
      set_max_head_data_size: TxDescriptor<{ new: number }>;

      /**
       *See [`Pallet::set_coretime_cores`].
       */
      set_coretime_cores: TxDescriptor<{ new: number }>;

      /**
       *See [`Pallet::set_on_demand_retries`].
       */
      set_on_demand_retries: TxDescriptor<{ new: number }>;

      /**
       *See [`Pallet::set_group_rotation_frequency`].
       */
      set_group_rotation_frequency: TxDescriptor<{ new: number }>;

      /**
       *See [`Pallet::set_paras_availability_period`].
       */
      set_paras_availability_period: TxDescriptor<{ new: number }>;

      /**
       *See [`Pallet::set_scheduling_lookahead`].
       */
      set_scheduling_lookahead: TxDescriptor<{ new: number }>;

      /**
       *See [`Pallet::set_max_validators_per_core`].
       */
      set_max_validators_per_core: TxDescriptor<{ new: Anonymize<I4arjljr6dpflb> }>;

      /**
       *See [`Pallet::set_max_validators`].
       */
      set_max_validators: TxDescriptor<{ new: Anonymize<I4arjljr6dpflb> }>;

      /**
       *See [`Pallet::set_dispute_period`].
       */
      set_dispute_period: TxDescriptor<{ new: number }>;

      /**
       *See [`Pallet::set_dispute_post_conclusion_acceptance_period`].
       */
      set_dispute_post_conclusion_acceptance_period: TxDescriptor<{ new: number }>;

      /**
       *See [`Pallet::set_no_show_slots`].
       */
      set_no_show_slots: TxDescriptor<{ new: number }>;

      /**
       *See [`Pallet::set_n_delay_tranches`].
       */
      set_n_delay_tranches: TxDescriptor<{ new: number }>;

      /**
       *See [`Pallet::set_zeroth_delay_tranche_width`].
       */
      set_zeroth_delay_tranche_width: TxDescriptor<{ new: number }>;

      /**
       *See [`Pallet::set_needed_approvals`].
       */
      set_needed_approvals: TxDescriptor<{ new: number }>;

      /**
       *See [`Pallet::set_relay_vrf_modulo_samples`].
       */
      set_relay_vrf_modulo_samples: TxDescriptor<{ new: number }>;

      /**
       *See [`Pallet::set_max_upward_queue_count`].
       */
      set_max_upward_queue_count: TxDescriptor<{ new: number }>;

      /**
       *See [`Pallet::set_max_upward_queue_size`].
       */
      set_max_upward_queue_size: TxDescriptor<{ new: number }>;

      /**
       *See [`Pallet::set_max_downward_message_size`].
       */
      set_max_downward_message_size: TxDescriptor<{ new: number }>;

      /**
       *See [`Pallet::set_max_upward_message_size`].
       */
      set_max_upward_message_size: TxDescriptor<{ new: number }>;

      /**
       *See [`Pallet::set_max_upward_message_num_per_candidate`].
       */
      set_max_upward_message_num_per_candidate: TxDescriptor<{ new: number }>;

      /**
       *See [`Pallet::set_hrmp_open_request_ttl`].
       */
      set_hrmp_open_request_ttl: TxDescriptor<{ new: number }>;

      /**
       *See [`Pallet::set_hrmp_sender_deposit`].
       */
      set_hrmp_sender_deposit: TxDescriptor<{ new: bigint }>;

      /**
       *See [`Pallet::set_hrmp_recipient_deposit`].
       */
      set_hrmp_recipient_deposit: TxDescriptor<{ new: bigint }>;

      /**
       *See [`Pallet::set_hrmp_channel_max_capacity`].
       */
      set_hrmp_channel_max_capacity: TxDescriptor<{ new: number }>;

      /**
       *See [`Pallet::set_hrmp_channel_max_total_size`].
       */
      set_hrmp_channel_max_total_size: TxDescriptor<{ new: number }>;

      /**
       *See [`Pallet::set_hrmp_max_parachain_inbound_channels`].
       */
      set_hrmp_max_parachain_inbound_channels: TxDescriptor<{ new: number }>;

      /**
       *See [`Pallet::set_hrmp_channel_max_message_size`].
       */
      set_hrmp_channel_max_message_size: TxDescriptor<{ new: number }>;

      /**
       *See [`Pallet::set_hrmp_max_parachain_outbound_channels`].
       */
      set_hrmp_max_parachain_outbound_channels: TxDescriptor<{ new: number }>;

      /**
       *See [`Pallet::set_hrmp_max_message_num_per_candidate`].
       */
      set_hrmp_max_message_num_per_candidate: TxDescriptor<{ new: number }>;

      /**
       *See [`Pallet::set_pvf_voting_ttl`].
       */
      set_pvf_voting_ttl: TxDescriptor<{ new: number }>;

      /**
       *See [`Pallet::set_minimum_validation_upgrade_delay`].
       */
      set_minimum_validation_upgrade_delay: TxDescriptor<{ new: number }>;

      /**
       *See [`Pallet::set_bypass_consistency_check`].
       */
      set_bypass_consistency_check: TxDescriptor<{ new: boolean }>;

      /**
       *See [`Pallet::set_async_backing_params`].
       */
      set_async_backing_params: TxDescriptor<{ new: Anonymize<Iavuvfkop6318c> }>;

      /**
       *See [`Pallet::set_executor_params`].
       */
      set_executor_params: TxDescriptor<{ new: Anonymize<I79n5nka727sg9> }>;

      /**
       *See [`Pallet::set_on_demand_base_fee`].
       */
      set_on_demand_base_fee: TxDescriptor<{ new: bigint }>;

      /**
       *See [`Pallet::set_on_demand_fee_variability`].
       */
      set_on_demand_fee_variability: TxDescriptor<{ new: number }>;

      /**
       *See [`Pallet::set_on_demand_queue_max_size`].
       */
      set_on_demand_queue_max_size: TxDescriptor<{ new: number }>;

      /**
       *See [`Pallet::set_on_demand_target_queue_utilization`].
       */
      set_on_demand_target_queue_utilization: TxDescriptor<{ new: number }>;

      /**
       *See [`Pallet::set_on_demand_ttl`].
       */
      set_on_demand_ttl: TxDescriptor<{ new: number }>;

      /**
       *See [`Pallet::set_minimum_backing_votes`].
       */
      set_minimum_backing_votes: TxDescriptor<{ new: number }>;

      /**
       *See [`Pallet::set_node_feature`].
       */
      set_node_feature: TxDescriptor<{ index: number; value: boolean }>;

      /**
       *See [`Pallet::set_approval_voting_params`].
       */
      set_approval_voting_params: TxDescriptor<{ new: number }>;
    },
    {},
    {
      /**
       *The new value for a configuration parameter is invalid.
       */
      InvalidNewValue: PlainDescriptor<undefined>;
    },
    {},
  ];
  ParasShared: [
    {
      /**
       * The current session index.
       */
      CurrentSessionIndex: StorageDescriptor<[], number, false>;

      /**
       * All the validators actively participating in parachain consensus.
       * Indices are into the broader validator set.
       */
      ActiveValidatorIndices: StorageDescriptor<[], Array<number>, false>;

      /**
       * The parachain attestation keys of the validators actively participating in parachain
       * consensus. This should be the same length as `ActiveValidatorIndices`.
       */
      ActiveValidatorKeys: StorageDescriptor<[], Array<Binary>, false>;

      /**
       * All allowed relay-parents.
       */
      AllowedRelayParents: StorageDescriptor<
        [],
        { buffer: Anonymize<Idm6djpsp7gtcm>; latest_number: number },
        false
      >;
    },
    {},
    {},
    {},
    {},
  ];
  ParaInclusion: [
    {
      /**
       * The latest bitfield for each validator, referred to by their index in the validator set.
       */
      AvailabilityBitfields: StorageDescriptor<
        [Key: number],
        { bitfield: { bytes: Uint8Array; bitsLen: number }; submitted_at: number },
        true
      >;

      /**
       * Candidates pending availability by `ParaId`.
       */
      PendingAvailability: StorageDescriptor<
        [Key: number],
        {
          core: number;
          hash: Binary;
          descriptor: Anonymize<Ib2u20s6roco9i>;
          availability_votes: { bytes: Uint8Array; bitsLen: number };
          backers: { bytes: Uint8Array; bitsLen: number };
          relay_parent_number: number;
          backed_in_number: number;
          backing_group: number;
        },
        true
      >;

      /**
       * The commitments of candidates pending availability, by `ParaId`.
       */
      PendingAvailabilityCommitments: StorageDescriptor<
        [Key: number],
        {
          upward_messages: Anonymize<Itom7fk49o0c9>;
          horizontal_messages: Anonymize<I6r5cbv8ttrb09>;
          new_validation_code: Anonymize<Iabpgqcjikia83>;
          head_data: Binary;
          processed_downward_messages: number;
          hrmp_watermark: number;
        },
        true
      >;
    },
    {},
    {
      /**
       *A candidate was backed. `[candidate, head_data]`
       */
      CandidateBacked: PlainDescriptor<[Anonymize<I4vjld3472quct>, Binary, number, number]>;

      /**
       *A candidate was included. `[candidate, head_data]`
       */
      CandidateIncluded: PlainDescriptor<[Anonymize<I4vjld3472quct>, Binary, number, number]>;

      /**
       *A candidate timed out. `[candidate, head_data]`
       */
      CandidateTimedOut: PlainDescriptor<[Anonymize<I4vjld3472quct>, Binary, number]>;

      /**
       *Some upward messages have been received and will be processed.
       */
      UpwardMessagesReceived: PlainDescriptor<{ from: number; count: number }>;
    },
    {
      /**
       *Validator indices are out of order or contains duplicates.
       */
      UnsortedOrDuplicateValidatorIndices: PlainDescriptor<undefined>;

      /**
       *Dispute statement sets are out of order or contain duplicates.
       */
      UnsortedOrDuplicateDisputeStatementSet: PlainDescriptor<undefined>;

      /**
       *Backed candidates are out of order (core index) or contain duplicates.
       */
      UnsortedOrDuplicateBackedCandidates: PlainDescriptor<undefined>;

      /**
       *A different relay parent was provided compared to the on-chain stored one.
       */
      UnexpectedRelayParent: PlainDescriptor<undefined>;

      /**
       *Availability bitfield has unexpected size.
       */
      WrongBitfieldSize: PlainDescriptor<undefined>;

      /**
       *Bitfield consists of zeros only.
       */
      BitfieldAllZeros: PlainDescriptor<undefined>;

      /**
       *Multiple bitfields submitted by same validator or validators out of order by index.
       */
      BitfieldDuplicateOrUnordered: PlainDescriptor<undefined>;

      /**
       *Validator index out of bounds.
       */
      ValidatorIndexOutOfBounds: PlainDescriptor<undefined>;

      /**
       *Invalid signature
       */
      InvalidBitfieldSignature: PlainDescriptor<undefined>;

      /**
       *Candidate submitted but para not scheduled.
       */
      UnscheduledCandidate: PlainDescriptor<undefined>;

      /**
       *Candidate scheduled despite pending candidate already existing for the para.
       */
      CandidateScheduledBeforeParaFree: PlainDescriptor<undefined>;

      /**
       *Scheduled cores out of order.
       */
      ScheduledOutOfOrder: PlainDescriptor<undefined>;

      /**
       *Head data exceeds the configured maximum.
       */
      HeadDataTooLarge: PlainDescriptor<undefined>;

      /**
       *Code upgrade prematurely.
       */
      PrematureCodeUpgrade: PlainDescriptor<undefined>;

      /**
       *Output code is too large
       */
      NewCodeTooLarge: PlainDescriptor<undefined>;

      /**
       *The candidate's relay-parent was not allowed. Either it was
       *not recent enough or it didn't advance based on the last parachain block.
       */
      DisallowedRelayParent: PlainDescriptor<undefined>;

      /**
       *Failed to compute group index for the core: either it's out of bounds
       *or the relay parent doesn't belong to the current session.
       */
      InvalidAssignment: PlainDescriptor<undefined>;

      /**
       *Invalid group index in core assignment.
       */
      InvalidGroupIndex: PlainDescriptor<undefined>;

      /**
       *Insufficient (non-majority) backing.
       */
      InsufficientBacking: PlainDescriptor<undefined>;

      /**
       *Invalid (bad signature, unknown validator, etc.) backing.
       */
      InvalidBacking: PlainDescriptor<undefined>;

      /**
       *Collator did not sign PoV.
       */
      NotCollatorSigned: PlainDescriptor<undefined>;

      /**
       *The validation data hash does not match expected.
       */
      ValidationDataHashMismatch: PlainDescriptor<undefined>;

      /**
       *The downward message queue is not processed correctly.
       */
      IncorrectDownwardMessageHandling: PlainDescriptor<undefined>;

      /**
       *At least one upward message sent does not pass the acceptance criteria.
       */
      InvalidUpwardMessages: PlainDescriptor<undefined>;

      /**
       *The candidate didn't follow the rules of HRMP watermark advancement.
       */
      HrmpWatermarkMishandling: PlainDescriptor<undefined>;

      /**
       *The HRMP messages sent by the candidate is not valid.
       */
      InvalidOutboundHrmp: PlainDescriptor<undefined>;

      /**
       *The validation code hash of the candidate is not valid.
       */
      InvalidValidationCodeHash: PlainDescriptor<undefined>;

      /**
       *The `para_head` hash in the candidate descriptor doesn't match the hash of the actual
       *para head in the commitments.
       */
      ParaHeadMismatch: PlainDescriptor<undefined>;

      /**
       *A bitfield that references a freed core,
       *either intentionally or as part of a concluded
       *invalid dispute.
       */
      BitfieldReferencesFreedCore: PlainDescriptor<undefined>;
    },
    {},
  ];
  ParaInherent: [
    {
      /**
       * Whether the paras inherent was included within this block.
       *
       * The `Option<()>` is effectively a `bool`, but it never hits storage in the `None` variant
       * due to the guarantees of FRAME's storage APIs.
       *
       * If this is `None` at the end of the block, we panic and render the block invalid.
       */
      Included: StorageDescriptor<[], undefined, true>;

      /**
       * Scraped on chain data for extracting resolved disputes as well as backing votes.
       */
      OnChainVotes: StorageDescriptor<
        [],
        {
          session: number;
          backing_validators_per_candidate: Anonymize<Ibabtlc0psj69a>;
          disputes: Anonymize<Ib08u599nb52js>;
        },
        true
      >;
    },
    {
      /**
       *See [`Pallet::enter`].
       */
      enter: TxDescriptor<{ data: Anonymize<I9hpj128t7mg6a> }>;
    },
    {},
    {
      /**
       *Inclusion inherent called more than once per block.
       */
      TooManyInclusionInherents: PlainDescriptor<undefined>;

      /**
       *The hash of the submitted parent header doesn't correspond to the saved block hash of
       *the parent.
       */
      InvalidParentHeader: PlainDescriptor<undefined>;

      /**
       *Disputed candidate that was concluded invalid.
       */
      CandidateConcludedInvalid: PlainDescriptor<undefined>;

      /**
       *The data given to the inherent will result in an overweight block.
       */
      InherentOverweight: PlainDescriptor<undefined>;

      /**
       *The ordering of dispute statements was invalid.
       */
      DisputeStatementsUnsortedOrDuplicates: PlainDescriptor<undefined>;

      /**
       *A dispute statement was invalid.
       */
      DisputeInvalid: PlainDescriptor<undefined>;

      /**
       *A candidate was backed by a disabled validator
       */
      BackedByDisabled: PlainDescriptor<undefined>;

      /**
       *A candidate was backed even though the paraid was not scheduled.
       */
      BackedOnUnscheduledCore: PlainDescriptor<undefined>;

      /**
       *Too many candidates supplied.
       */
      UnscheduledCandidate: PlainDescriptor<undefined>;
    },
    {},
  ];
  ParaScheduler: [
    {
      /**
       * All the validator groups. One for each core. Indices are into `ActiveValidators` - not the
       * broader set of Polkadot validators, but instead just the subset used for parachains during
       * this session.
       *
       * Bound: The number of cores is the sum of the numbers of parachains and parathread
       * multiplexers. Reasonably, 100-1000. The dominant factor is the number of validators: safe
       * upper bound at 10k.
       */
      ValidatorGroups: StorageDescriptor<[], Array<Anonymize<Icgljjb6j82uhn>>, false>;

      /**
       * One entry for each availability core. Entries are `None` if the core is not currently
       * occupied. Can be temporarily `Some` if scheduled but not occupied.
       * The i'th parachain belongs to the i'th core, with the remaining cores all being
       * parathread-multiplexers.
       *
       * Bounded by the maximum of either of these two values:
       *   * The number of parachains and parathread multiplexers
       *   * The number of validators divided by `configuration.max_validators_per_core`.
       */
      AvailabilityCores: StorageDescriptor<
        [],
        Array<PolkadotRuntimeParachainsSchedulerPalletCoreOccupied>,
        false
      >;

      /**
       * The block number where the session start occurred. Used to track how many group rotations
       * have occurred.
       *
       * Note that in the context of parachains modules the session change is signaled during
       * the block and enacted at the end of the block (at the finalization stage, to be exact).
       * Thus for all intents and purposes the effect of the session change is observed at the
       * block following the session change, block number of which we save in this storage value.
       */
      SessionStartBlock: StorageDescriptor<[], number, false>;

      /**
       * One entry for each availability core. The `VecDeque` represents the assignments to be
       * scheduled on that core. The value contained here will not be valid after the end of
       * a block. Runtime APIs should be used to determine scheduled cores/ for the upcoming block.
       */
      ClaimQueue: StorageDescriptor<[], Array<Anonymize<Iekknq5gev7jf7>>, false>;
    },
    {},
    {},
    {},
    {},
  ];
  Paras: [
    {
      /**
       * All currently active PVF pre-checking votes.
       *
       * Invariant:
       * - There are no PVF pre-checking votes that exists in list but not in the set and vice versa.
       */
      PvfActiveVoteMap: StorageDescriptor<
        [Key: Binary],
        {
          votes_accept: { bytes: Uint8Array; bitsLen: number };
          votes_reject: { bytes: Uint8Array; bitsLen: number };
          age: number;
          created_at: number;
          causes: Anonymize<Ia0a9586d8d811>;
        },
        true
      >;

      /**
       * The list of all currently active PVF votes. Auxiliary to `PvfActiveVoteMap`.
       */
      PvfActiveVoteList: StorageDescriptor<[], Array<Binary>, false>;

      /**
       * All lease holding parachains. Ordered ascending by `ParaId`. On demand parachains are not
       * included.
       *
       * Consider using the [`ParachainsCache`] type of modifying.
       */
      Parachains: StorageDescriptor<[], Array<number>, false>;

      /**
       * The current lifecycle of a all known Para IDs.
       */
      ParaLifecycles: StorageDescriptor<[Key: number], ParachainsParasParaLifecycle, true>;

      /**
       * The head-data of every registered para.
       */
      Heads: StorageDescriptor<[Key: number], Binary, true>;

      /**
       * The context (relay-chain block number) of the most recent parachain head.
       */
      MostRecentContext: StorageDescriptor<[Key: number], number, true>;

      /**
       * The validation code hash of every live para.
       *
       * Corresponding code can be retrieved with [`CodeByHash`].
       */
      CurrentCodeHash: StorageDescriptor<[Key: number], Binary, true>;

      /**
       * Actual past code hash, indicated by the para id as well as the block number at which it
       * became outdated.
       *
       * Corresponding code can be retrieved with [`CodeByHash`].
       */
      PastCodeHash: StorageDescriptor<[Key: [number, number]], Binary, true>;

      /**
       * Past code of parachains. The parachains themselves may not be registered anymore,
       * but we also keep their code on-chain for the same amount of time as outdated code
       * to keep it available for approval checkers.
       */
      PastCodeMeta: StorageDescriptor<
        [Key: number],
        { upgrade_times: Anonymize<I2v6n2k262gqsq>; last_pruned: Anonymize<I4arjljr6dpflb> },
        false
      >;

      /**
       * Which paras have past code that needs pruning and the relay-chain block at which the code
       * was replaced. Note that this is the actual height of the included block, not the expected
       * height at which the code upgrade would be applied, although they may be equal.
       * This is to ensure the entire acceptance period is covered, not an offset acceptance period
       * starting from the time at which the parachain perceives a code upgrade as having occurred.
       * Multiple entries for a single para are permitted. Ordered ascending by block number.
       */
      PastCodePruning: StorageDescriptor<[], Array<Anonymize<I5g2vv0ckl2m8b>>, false>;

      /**
       * The block number at which the planned code change is expected for a para.
       * The change will be applied after the first parablock for this ID included which executes
       * in the context of a relay chain block with a number >= `expected_at`.
       */
      FutureCodeUpgrades: StorageDescriptor<[Key: number], number, true>;

      /**
       * The actual future code hash of a para.
       *
       * Corresponding code can be retrieved with [`CodeByHash`].
       */
      FutureCodeHash: StorageDescriptor<[Key: number], Binary, true>;

      /**
       * This is used by the relay-chain to communicate to a parachain a go-ahead with in the upgrade
       * procedure.
       *
       * This value is absent when there are no upgrades scheduled or during the time the relay chain
       * performs the checks. It is set at the first relay-chain block when the corresponding
       * parachain can switch its upgrade function. As soon as the parachain's block is included, the
       * value gets reset to `None`.
       *
       * NOTE that this field is used by parachains via merkle storage proofs, therefore changing
       * the format will require migration of parachains.
       */
      UpgradeGoAheadSignal: StorageDescriptor<
        [Key: number],
        PolkadotPrimitivesV5UpgradeGoAhead,
        true
      >;

      /**
       * This is used by the relay-chain to communicate that there are restrictions for performing
       * an upgrade for this parachain.
       *
       * This may be a because the parachain waits for the upgrade cooldown to expire. Another
       * potential use case is when we want to perform some maintenance (such as storage migration)
       * we could restrict upgrades to make the process simpler.
       *
       * NOTE that this field is used by parachains via merkle storage proofs, therefore changing
       * the format will require migration of parachains.
       */
      UpgradeRestrictionSignal: StorageDescriptor<
        [Key: number],
        PolkadotPrimitivesV5UpgradeRestriction,
        true
      >;

      /**
       * The list of parachains that are awaiting for their upgrade restriction to cooldown.
       *
       * Ordered ascending by block number.
       */
      UpgradeCooldowns: StorageDescriptor<[], Array<Anonymize<I5g2vv0ckl2m8b>>, false>;

      /**
       * The list of upcoming code upgrades. Each item is a pair of which para performs a code
       * upgrade and at which relay-chain block it is expected at.
       *
       * Ordered ascending by block number.
       */
      UpcomingUpgrades: StorageDescriptor<[], Array<Anonymize<I5g2vv0ckl2m8b>>, false>;

      /**
       * The actions to perform during the start of a specific session index.
       */
      ActionsQueue: StorageDescriptor<[Key: number], Array<number>, false>;

      /**
       * Upcoming paras instantiation arguments.
       *
       * NOTE that after PVF pre-checking is enabled the para genesis arg will have it's code set
       * to empty. Instead, the code will be saved into the storage right away via `CodeByHash`.
       */
      UpcomingParasGenesis: StorageDescriptor<
        [Key: number],
        { genesis_head: Binary; validation_code: Binary; para_kind: boolean },
        true
      >;

      /**
       * The number of reference on the validation code in [`CodeByHash`] storage.
       */
      CodeByHashRefs: StorageDescriptor<[Key: Binary], number, false>;

      /**
       * Validation code stored by its hash.
       *
       * This storage is consistent with [`FutureCodeHash`], [`CurrentCodeHash`] and
       * [`PastCodeHash`].
       */
      CodeByHash: StorageDescriptor<[Key: Binary], Binary, true>;
    },
    {
      /**
       *See [`Pallet::force_set_current_code`].
       */
      force_set_current_code: TxDescriptor<{ para: number; new_code: Binary }>;

      /**
       *See [`Pallet::force_set_current_head`].
       */
      force_set_current_head: TxDescriptor<{ para: number; new_head: Binary }>;

      /**
       *See [`Pallet::force_schedule_code_upgrade`].
       */
      force_schedule_code_upgrade: TxDescriptor<{
        para: number;
        new_code: Binary;
        relay_parent_number: number;
      }>;

      /**
       *See [`Pallet::force_note_new_head`].
       */
      force_note_new_head: TxDescriptor<{ para: number; new_head: Binary }>;

      /**
       *See [`Pallet::force_queue_action`].
       */
      force_queue_action: TxDescriptor<{ para: number }>;

      /**
       *See [`Pallet::add_trusted_validation_code`].
       */
      add_trusted_validation_code: TxDescriptor<{ validation_code: Binary }>;

      /**
       *See [`Pallet::poke_unused_validation_code`].
       */
      poke_unused_validation_code: TxDescriptor<{ validation_code_hash: Binary }>;

      /**
       *See [`Pallet::include_pvf_check_statement`].
       */
      include_pvf_check_statement: TxDescriptor<{
        stmt: Anonymize<I3h1ccufdk38ej>;
        signature: Binary;
      }>;

      /**
       *See [`Pallet::force_set_most_recent_context`].
       */
      force_set_most_recent_context: TxDescriptor<{ para: number; context: number }>;
    },
    {
      /**
       *Current code has been updated for a Para. `para_id`
       */
      CurrentCodeUpdated: PlainDescriptor<number>;

      /**
       *Current head has been updated for a Para. `para_id`
       */
      CurrentHeadUpdated: PlainDescriptor<number>;

      /**
       *A code upgrade has been scheduled for a Para. `para_id`
       */
      CodeUpgradeScheduled: PlainDescriptor<number>;

      /**
       *A new head has been noted for a Para. `para_id`
       */
      NewHeadNoted: PlainDescriptor<number>;

      /**
       *A para has been queued to execute pending actions. `para_id`
       */
      ActionQueued: PlainDescriptor<[number, number]>;

      /**
       *The given para either initiated or subscribed to a PVF check for the given validation
       *code. `code_hash` `para_id`
       */
      PvfCheckStarted: PlainDescriptor<[Binary, number]>;

      /**
       *The given validation code was accepted by the PVF pre-checking vote.
       *`code_hash` `para_id`
       */
      PvfCheckAccepted: PlainDescriptor<[Binary, number]>;

      /**
       *The given validation code was rejected by the PVF pre-checking vote.
       *`code_hash` `para_id`
       */
      PvfCheckRejected: PlainDescriptor<[Binary, number]>;
    },
    {
      /**
       *Para is not registered in our system.
       */
      NotRegistered: PlainDescriptor<undefined>;

      /**
       *Para cannot be onboarded because it is already tracked by our system.
       */
      CannotOnboard: PlainDescriptor<undefined>;

      /**
       *Para cannot be offboarded at this time.
       */
      CannotOffboard: PlainDescriptor<undefined>;

      /**
       *Para cannot be upgraded to a lease holding parachain.
       */
      CannotUpgrade: PlainDescriptor<undefined>;

      /**
       *Para cannot be downgraded to an on-demand parachain.
       */
      CannotDowngrade: PlainDescriptor<undefined>;

      /**
       *The statement for PVF pre-checking is stale.
       */
      PvfCheckStatementStale: PlainDescriptor<undefined>;

      /**
       *The statement for PVF pre-checking is for a future session.
       */
      PvfCheckStatementFuture: PlainDescriptor<undefined>;

      /**
       *Claimed validator index is out of bounds.
       */
      PvfCheckValidatorIndexOutOfBounds: PlainDescriptor<undefined>;

      /**
       *The signature for the PVF pre-checking is invalid.
       */
      PvfCheckInvalidSignature: PlainDescriptor<undefined>;

      /**
       *The given validator already has cast a vote.
       */
      PvfCheckDoubleVote: PlainDescriptor<undefined>;

      /**
       *The given PVF does not exist at the moment of process a vote.
       */
      PvfCheckSubjectInvalid: PlainDescriptor<undefined>;

      /**
       *Parachain cannot currently schedule a code upgrade.
       */
      CannotUpgradeCode: PlainDescriptor<undefined>;

      /**
       *Invalid validation code size.
       */
      InvalidCode: PlainDescriptor<undefined>;
    },
    {
      /**

 */
      UnsignedPriority: PlainDescriptor<bigint>;
    },
  ];
  Initializer: [
    {
      /**
       * Whether the parachains modules have been initialized within this block.
       *
       * Semantically a `bool`, but this guarantees it should never hit the trie,
       * as this is cleared in `on_finalize` and Frame optimizes `None` values to be empty values.
       *
       * As a `bool`, `set(false)` and `remove()` both lead to the next `get()` being false, but one
       * of them writes to the trie and one does not. This confusion makes `Option<()>` more suitable
       * for the semantics of this variable.
       */
      HasInitialized: StorageDescriptor<[], undefined, true>;

      /**
       * Buffered session changes along with the block number at which they should be applied.
       *
       * Typically this will be empty or one element long. Apart from that this item never hits
       * the storage.
       *
       * However this is a `Vec` regardless to handle various edge cases that may occur at runtime
       * upgrade boundaries or if governance intervenes.
       */
      BufferedSessionChanges: StorageDescriptor<[], Array<Anonymize<I36mfku1ea0i8t>>, false>;
    },
    {
      /**
       *See [`Pallet::force_approve`].
       */
      force_approve: TxDescriptor<{ up_to: number }>;
    },
    {},
    {},
    {},
  ];
  Dmp: [
    {
      /**
       * The downward messages addressed for a certain para.
       */
      DownwardMessageQueues: StorageDescriptor<
        [Key: number],
        Array<Anonymize<I60847k37jfcc6>>,
        false
      >;

      /**
       * A mapping that stores the downward message queue MQC head for each para.
       *
       * Each link in this chain has a form:
       * `(prev_head, B, H(M))`, where
       * - `prev_head`: is the previous head hash or zero if none.
       * - `B`: is the relay-chain block number in which a message was appended.
       * - `H(M)`: is the hash of the message being appended.
       */
      DownwardMessageQueueHeads: StorageDescriptor<[Key: number], Binary, false>;

      /**
       * The factor to multiply the base delivery fee by.
       */
      DeliveryFeeFactor: StorageDescriptor<[Key: number], bigint, false>;
    },
    {},
    {},
    {},
    {},
  ];
  Hrmp: [
    {
      /**
       * The set of pending HRMP open channel requests.
       *
       * The set is accompanied by a list for iteration.
       *
       * Invariant:
       * - There are no channels that exists in list but not in the set and vice versa.
       */
      HrmpOpenChannelRequests: StorageDescriptor<
        [Key: { sender: number; recipient: number }],
        {
          confirmed: boolean;
          _age: number;
          sender_deposit: bigint;
          max_message_size: number;
          max_capacity: number;
          max_total_size: number;
        },
        true
      >;

      /**

 */
      HrmpOpenChannelRequestsList: StorageDescriptor<[], Array<Anonymize<I50mrcbubp554e>>, false>;

      /**
       * This mapping tracks how many open channel requests are initiated by a given sender para.
       * Invariant: `HrmpOpenChannelRequests` should contain the same number of items that has
       * `(X, _)` as the number of `HrmpOpenChannelRequestCount` for `X`.
       */
      HrmpOpenChannelRequestCount: StorageDescriptor<[Key: number], number, false>;

      /**
       * This mapping tracks how many open channel requests were accepted by a given recipient para.
       * Invariant: `HrmpOpenChannelRequests` should contain the same number of items `(_, X)` with
       * `confirmed` set to true, as the number of `HrmpAcceptedChannelRequestCount` for `X`.
       */
      HrmpAcceptedChannelRequestCount: StorageDescriptor<[Key: number], number, false>;

      /**
       * A set of pending HRMP close channel requests that are going to be closed during the session
       * change. Used for checking if a given channel is registered for closure.
       *
       * The set is accompanied by a list for iteration.
       *
       * Invariant:
       * - There are no channels that exists in list but not in the set and vice versa.
       */
      HrmpCloseChannelRequests: StorageDescriptor<
        [Key: { sender: number; recipient: number }],
        undefined,
        true
      >;

      /**

 */
      HrmpCloseChannelRequestsList: StorageDescriptor<[], Array<Anonymize<I50mrcbubp554e>>, false>;

      /**
       * The HRMP watermark associated with each para.
       * Invariant:
       * - each para `P` used here as a key should satisfy `Paras::is_valid_para(P)` within a
       *   session.
       */
      HrmpWatermarks: StorageDescriptor<[Key: number], number, true>;

      /**
       * HRMP channel data associated with each para.
       * Invariant:
       * - each participant in the channel should satisfy `Paras::is_valid_para(P)` within a session.
       */
      HrmpChannels: StorageDescriptor<
        [Key: { sender: number; recipient: number }],
        {
          max_capacity: number;
          max_total_size: number;
          max_message_size: number;
          msg_count: number;
          total_size: number;
          mqc_head: Anonymize<I17k3ujudqd5df>;
          sender_deposit: bigint;
          recipient_deposit: bigint;
        },
        true
      >;

      /**
       * Ingress/egress indexes allow to find all the senders and receivers given the opposite side.
       * I.e.
       *
       * (a) ingress index allows to find all the senders for a given recipient.
       * (b) egress index allows to find all the recipients for a given sender.
       *
       * Invariants:
       * - for each ingress index entry for `P` each item `I` in the index should present in
       *   `HrmpChannels` as `(I, P)`.
       * - for each egress index entry for `P` each item `E` in the index should present in
       *   `HrmpChannels` as `(P, E)`.
       * - there should be no other dangling channels in `HrmpChannels`.
       * - the vectors are sorted.
       */
      HrmpIngressChannelsIndex: StorageDescriptor<[Key: number], Array<number>, false>;

      /**

 */
      HrmpEgressChannelsIndex: StorageDescriptor<[Key: number], Array<number>, false>;

      /**
       * Storage for the messages for each channel.
       * Invariant: cannot be non-empty if the corresponding channel in `HrmpChannels` is `None`.
       */
      HrmpChannelContents: StorageDescriptor<
        [Key: { sender: number; recipient: number }],
        Array<Anonymize<I409qo0sfkbh16>>,
        false
      >;

      /**
       * Maintains a mapping that can be used to answer the question: What paras sent a message at
       * the given block number for a given receiver. Invariants:
       * - The inner `Vec<ParaId>` is never empty.
       * - The inner `Vec<ParaId>` cannot store two same `ParaId`.
       * - The outer vector is sorted ascending by block number and cannot store two items with the
       *   same block number.
       */
      HrmpChannelDigests: StorageDescriptor<[Key: number], Array<Anonymize<I8pg2rpr4ldgp9>>, false>;
    },
    {
      /**
       *See [`Pallet::hrmp_init_open_channel`].
       */
      hrmp_init_open_channel: TxDescriptor<{
        recipient: number;
        proposed_max_capacity: number;
        proposed_max_message_size: number;
      }>;

      /**
       *See [`Pallet::hrmp_accept_open_channel`].
       */
      hrmp_accept_open_channel: TxDescriptor<{ sender: number }>;

      /**
       *See [`Pallet::hrmp_close_channel`].
       */
      hrmp_close_channel: TxDescriptor<{ channel_id: Anonymize<I50mrcbubp554e> }>;

      /**
       *See [`Pallet::force_clean_hrmp`].
       */
      force_clean_hrmp: TxDescriptor<{ para: number; num_inbound: number; num_outbound: number }>;

      /**
       *See [`Pallet::force_process_hrmp_open`].
       */
      force_process_hrmp_open: TxDescriptor<{ channels: number }>;

      /**
       *See [`Pallet::force_process_hrmp_close`].
       */
      force_process_hrmp_close: TxDescriptor<{ channels: number }>;

      /**
       *See [`Pallet::hrmp_cancel_open_request`].
       */
      hrmp_cancel_open_request: TxDescriptor<{
        channel_id: Anonymize<I50mrcbubp554e>;
        open_requests: number;
      }>;

      /**
       *See [`Pallet::force_open_hrmp_channel`].
       */
      force_open_hrmp_channel: TxDescriptor<{
        sender: number;
        recipient: number;
        max_capacity: number;
        max_message_size: number;
      }>;

      /**
       *See [`Pallet::establish_system_channel`].
       */
      establish_system_channel: TxDescriptor<{ sender: number; recipient: number }>;

      /**
       *See [`Pallet::poke_channel_deposits`].
       */
      poke_channel_deposits: TxDescriptor<{ sender: number; recipient: number }>;
    },
    {
      /**
       *Open HRMP channel requested.
       */
      OpenChannelRequested: PlainDescriptor<{
        sender: number;
        recipient: number;
        proposed_max_capacity: number;
        proposed_max_message_size: number;
      }>;

      /**
       *An HRMP channel request sent by the receiver was canceled by either party.
       */
      OpenChannelCanceled: PlainDescriptor<{
        by_parachain: number;
        channel_id: Anonymize<I50mrcbubp554e>;
      }>;

      /**
       *Open HRMP channel accepted.
       */
      OpenChannelAccepted: PlainDescriptor<{ sender: number; recipient: number }>;

      /**
       *HRMP channel closed.
       */
      ChannelClosed: PlainDescriptor<{
        by_parachain: number;
        channel_id: Anonymize<I50mrcbubp554e>;
      }>;

      /**
       *An HRMP channel was opened via Root origin.
       */
      HrmpChannelForceOpened: PlainDescriptor<{
        sender: number;
        recipient: number;
        proposed_max_capacity: number;
        proposed_max_message_size: number;
      }>;

      /**
       *An HRMP channel was opened between two system chains.
       */
      HrmpSystemChannelOpened: PlainDescriptor<{
        sender: number;
        recipient: number;
        proposed_max_capacity: number;
        proposed_max_message_size: number;
      }>;

      /**
       *An HRMP channel's deposits were updated.
       */
      OpenChannelDepositsUpdated: PlainDescriptor<{ sender: number; recipient: number }>;
    },
    {
      /**
       *The sender tried to open a channel to themselves.
       */
      OpenHrmpChannelToSelf: PlainDescriptor<undefined>;

      /**
       *The recipient is not a valid para.
       */
      OpenHrmpChannelInvalidRecipient: PlainDescriptor<undefined>;

      /**
       *The requested capacity is zero.
       */
      OpenHrmpChannelZeroCapacity: PlainDescriptor<undefined>;

      /**
       *The requested capacity exceeds the global limit.
       */
      OpenHrmpChannelCapacityExceedsLimit: PlainDescriptor<undefined>;

      /**
       *The requested maximum message size is 0.
       */
      OpenHrmpChannelZeroMessageSize: PlainDescriptor<undefined>;

      /**
       *The open request requested the message size that exceeds the global limit.
       */
      OpenHrmpChannelMessageSizeExceedsLimit: PlainDescriptor<undefined>;

      /**
       *The channel already exists
       */
      OpenHrmpChannelAlreadyExists: PlainDescriptor<undefined>;

      /**
       *There is already a request to open the same channel.
       */
      OpenHrmpChannelAlreadyRequested: PlainDescriptor<undefined>;

      /**
       *The sender already has the maximum number of allowed outbound channels.
       */
      OpenHrmpChannelLimitExceeded: PlainDescriptor<undefined>;

      /**
       *The channel from the sender to the origin doesn't exist.
       */
      AcceptHrmpChannelDoesntExist: PlainDescriptor<undefined>;

      /**
       *The channel is already confirmed.
       */
      AcceptHrmpChannelAlreadyConfirmed: PlainDescriptor<undefined>;

      /**
       *The recipient already has the maximum number of allowed inbound channels.
       */
      AcceptHrmpChannelLimitExceeded: PlainDescriptor<undefined>;

      /**
       *The origin tries to close a channel where it is neither the sender nor the recipient.
       */
      CloseHrmpChannelUnauthorized: PlainDescriptor<undefined>;

      /**
       *The channel to be closed doesn't exist.
       */
      CloseHrmpChannelDoesntExist: PlainDescriptor<undefined>;

      /**
       *The channel close request is already requested.
       */
      CloseHrmpChannelAlreadyUnderway: PlainDescriptor<undefined>;

      /**
       *Canceling is requested by neither the sender nor recipient of the open channel request.
       */
      CancelHrmpOpenChannelUnauthorized: PlainDescriptor<undefined>;

      /**
       *The open request doesn't exist.
       */
      OpenHrmpChannelDoesntExist: PlainDescriptor<undefined>;

      /**
       *Cannot cancel an HRMP open channel request because it is already confirmed.
       */
      OpenHrmpChannelAlreadyConfirmed: PlainDescriptor<undefined>;

      /**
       *The provided witness data is wrong.
       */
      WrongWitness: PlainDescriptor<undefined>;

      /**
       *The channel between these two chains cannot be authorized.
       */
      ChannelCreationNotAuthorized: PlainDescriptor<undefined>;
    },
    {},
  ];
  ParaSessionInfo: [
    {
      /**
       * Assignment keys for the current session.
       * Note that this API is private due to it being prone to 'off-by-one' at session boundaries.
       * When in doubt, use `Sessions` API instead.
       */
      AssignmentKeysUnsafe: StorageDescriptor<[], Array<Binary>, false>;

      /**
       * The earliest session for which previous session info is stored.
       */
      EarliestStoredSession: StorageDescriptor<[], number, false>;

      /**
       * Session information in a rolling window.
       * Should have an entry in range `EarliestStoredSession..=CurrentSessionIndex`.
       * Does not have any entries before the session index in the first session change notification.
       */
      Sessions: StorageDescriptor<
        [Key: number],
        {
          active_validator_indices: Anonymize<Icgljjb6j82uhn>;
          random_seed: Binary;
          dispute_period: number;
          validators: Anonymize<Idhnf6rtqoslea>;
          discovery_keys: Anonymize<Idhnf6rtqoslea>;
          assignment_keys: Anonymize<Idhnf6rtqoslea>;
          validator_groups: Anonymize<Iarlj3qd8u1v13>;
          n_cores: number;
          zeroth_delay_tranche_width: number;
          relay_vrf_modulo_samples: number;
          n_delay_tranches: number;
          no_show_slots: number;
          needed_approvals: number;
        },
        true
      >;

      /**
       * The validator account keys of the validators actively participating in parachain consensus.
       */
      AccountKeys: StorageDescriptor<[Key: number], Array<SS58String>, true>;

      /**
       * Executor parameter set for a given session index
       */
      SessionExecutorParams: StorageDescriptor<
        [Key: number],
        Array<PolkadotPrimitivesV6ExecutorParamsExecutorParam>,
        true
      >;
    },
    {},
    {},
    {},
    {},
  ];
  ParasDisputes: [
    {
      /**
       * The last pruned session, if any. All data stored by this module
       * references sessions.
       */
      LastPrunedSession: StorageDescriptor<[], number, true>;

      /**
       * All ongoing or concluded disputes for the last several sessions.
       */
      Disputes: StorageDescriptor<
        [number, Binary],
        {
          validators_for: { bytes: Uint8Array; bitsLen: number };
          validators_against: { bytes: Uint8Array; bitsLen: number };
          start: number;
          concluded_at: Anonymize<I4arjljr6dpflb>;
        },
        true
      >;

      /**
       * Backing votes stored for each dispute.
       * This storage is used for slashing.
       */
      BackersOnDisputes: StorageDescriptor<[number, Binary], Array<number>, true>;

      /**
       * All included blocks on the chain, as well as the block number in this chain that
       * should be reverted back to if the candidate is disputed and determined to be invalid.
       */
      Included: StorageDescriptor<[number, Binary], number, true>;

      /**
       * Whether the chain is frozen. Starts as `None`. When this is `Some`,
       * the chain will not accept any new parachain blocks for backing or inclusion,
       * and its value indicates the last valid block number in the chain.
       * It can only be set back to `None` by governance intervention.
       */
      Frozen: StorageDescriptor<[], number | undefined, false>;
    },
    {
      /**
       *See [`Pallet::force_unfreeze`].
       */
      force_unfreeze: TxDescriptor<undefined>;
    },
    {
      /**
       *A dispute has been initiated. \[candidate hash, dispute location\]
       */
      DisputeInitiated: PlainDescriptor<[Binary, ParachainsDisputesDisputeLocation]>;

      /**
       *A dispute has concluded for or against a candidate.
       *`\[para id, candidate hash, dispute result\]`
       */
      DisputeConcluded: PlainDescriptor<[Binary, ParachainsDisputesDisputeResult]>;

      /**
       *A dispute has concluded with supermajority against a candidate.
       *Block authors should no longer build on top of this head and should
       *instead revert the block at the given height. This should be the
       *number of the child of the last known valid block in the chain.
       */
      Revert: PlainDescriptor<number>;
    },
    {
      /**
       *Duplicate dispute statement sets provided.
       */
      DuplicateDisputeStatementSets: PlainDescriptor<undefined>;

      /**
       *Ancient dispute statement provided.
       */
      AncientDisputeStatement: PlainDescriptor<undefined>;

      /**
       *Validator index on statement is out of bounds for session.
       */
      ValidatorIndexOutOfBounds: PlainDescriptor<undefined>;

      /**
       *Invalid signature on statement.
       */
      InvalidSignature: PlainDescriptor<undefined>;

      /**
       *Validator vote submitted more than once to dispute.
       */
      DuplicateStatement: PlainDescriptor<undefined>;

      /**
       *A dispute where there are only votes on one side.
       */
      SingleSidedDispute: PlainDescriptor<undefined>;

      /**
       *A dispute vote from a malicious backer.
       */
      MaliciousBacker: PlainDescriptor<undefined>;

      /**
       *No backing votes were provides along dispute statements.
       */
      MissingBackingVotes: PlainDescriptor<undefined>;

      /**
       *Unconfirmed dispute statement sets provided.
       */
      UnconfirmedDispute: PlainDescriptor<undefined>;
    },
    {},
  ];
  ParasSlashing: [
    {
      /**
       * Validators pending dispute slashes.
       */
      UnappliedSlashes: StorageDescriptor<
        [number, Binary],
        { keys: Anonymize<I93ssha9egqq23>; kind: PolkadotPrimitivesV5SlashingSlashingOffenceKind },
        true
      >;

      /**
       * `ValidatorSetCount` per session.
       */
      ValidatorSetCounts: StorageDescriptor<[Key: number], number, true>;
    },
    {
      /**
       *See [`Pallet::report_dispute_lost_unsigned`].
       */
      report_dispute_lost_unsigned: TxDescriptor<{
        dispute_proof: Anonymize<Iag14tqe65tvpf>;
        key_owner_proof: Anonymize<I3ia7aufsoj0l1>;
      }>;
    },
    {},
    {
      /**
       *The key ownership proof is invalid.
       */
      InvalidKeyOwnershipProof: PlainDescriptor<undefined>;

      /**
       *The session index is too old or invalid.
       */
      InvalidSessionIndex: PlainDescriptor<undefined>;

      /**
       *The candidate hash is invalid.
       */
      InvalidCandidateHash: PlainDescriptor<undefined>;

      /**
       *There is no pending slash for the given validator index and time
       *slot.
       */
      InvalidValidatorIndex: PlainDescriptor<undefined>;

      /**
       *The validator index does not match the validator id.
       */
      ValidatorIndexIdMismatch: PlainDescriptor<undefined>;

      /**
       *The given slashing report is valid but already previously reported.
       */
      DuplicateSlashingReport: PlainDescriptor<undefined>;
    },
    {},
  ];
  MessageQueue: [
    {
      /**
       * The index of the first and last (non-empty) pages.
       */
      BookStateFor: StorageDescriptor<
        [Key: ParachainsInclusionAggregateMessageOrigin],
        {
          begin: number;
          end: number;
          count: number;
          ready_neighbours: Anonymize<Iav3cdf9g9n9fp>;
          message_count: bigint;
          size: bigint;
        },
        false
      >;

      /**
       * The origin at which we should begin servicing.
       */
      ServiceHead: StorageDescriptor<[], ParachainsInclusionAggregateMessageOrigin, true>;

      /**
       * The map of page indices to pages.
       */
      Pages: StorageDescriptor<
        [ParachainsInclusionAggregateMessageOrigin, number],
        {
          remaining: number;
          remaining_size: number;
          first_index: number;
          first: number;
          last: number;
          heap: Binary;
        },
        true
      >;
    },
    {
      /**
       *See [`Pallet::reap_page`].
       */
      reap_page: TxDescriptor<{
        message_origin: ParachainsInclusionAggregateMessageOrigin;
        page_index: number;
      }>;

      /**
       *See [`Pallet::execute_overweight`].
       */
      execute_overweight: TxDescriptor<{
        message_origin: ParachainsInclusionAggregateMessageOrigin;
        page: number;
        index: number;
        weight_limit: Anonymize<I4q39t5hn830vp>;
      }>;
    },
    {
      /**
       *Message discarded due to an error in the `MessageProcessor` (usually a format error).
       */
      ProcessingFailed: PlainDescriptor<{
        id: Binary;
        origin: ParachainsInclusionAggregateMessageOrigin;
        error: ProcessMessageError;
      }>;

      /**
       *Message is processed.
       */
      Processed: PlainDescriptor<{
        id: Binary;
        origin: ParachainsInclusionAggregateMessageOrigin;
        weight_used: Anonymize<I4q39t5hn830vp>;
        success: boolean;
      }>;

      /**
       *Message placed in overweight queue.
       */
      OverweightEnqueued: PlainDescriptor<{
        id: Binary;
        origin: ParachainsInclusionAggregateMessageOrigin;
        page_index: number;
        message_index: number;
      }>;

      /**
       *This page was reaped.
       */
      PageReaped: PlainDescriptor<{
        origin: ParachainsInclusionAggregateMessageOrigin;
        index: number;
      }>;
    },
    {
      /**
       *Page is not reapable because it has items remaining to be processed and is not old
       *enough.
       */
      NotReapable: PlainDescriptor<undefined>;

      /**
       *Page to be reaped does not exist.
       */
      NoPage: PlainDescriptor<undefined>;

      /**
       *The referenced message could not be found.
       */
      NoMessage: PlainDescriptor<undefined>;

      /**
       *The message was already processed and cannot be processed again.
       */
      AlreadyProcessed: PlainDescriptor<undefined>;

      /**
       *The message is queued for future execution.
       */
      Queued: PlainDescriptor<undefined>;

      /**
       *There is temporarily not enough weight to continue servicing messages.
       */
      InsufficientWeight: PlainDescriptor<undefined>;

      /**
       *This message is temporarily unprocessable.
       *
       *Such errors are expected, but not guaranteed, to resolve themselves eventually through
       *retrying.
       */
      TemporarilyUnprocessable: PlainDescriptor<undefined>;

      /**
       *The queue is paused and no message can be executed from it.
       *
       *This can change at any time and may resolve in the future by re-trying.
       */
      QueuePaused: PlainDescriptor<undefined>;

      /**
       *Another call is in progress and needs to finish before this call can happen.
       */
      RecursiveDisallowed: PlainDescriptor<undefined>;
    },
    {
      /**
       * The size of the page; this implies the maximum message size which can be sent.
       *
       * A good value depends on the expected message sizes, their weights, the weight that is
       * available for processing them and the maximal needed message size. The maximal message
       * size is slightly lower than this as defined by [`MaxMessageLenOf`].
       */
      HeapSize: PlainDescriptor<number>;

      /**
       * The maximum number of stale pages (i.e. of overweight messages) allowed before culling
       * can happen. Once there are more stale pages than this, then historical pages may be
       * dropped, even if they contain unprocessed overweight messages.
       */
      MaxStale: PlainDescriptor<number>;

      /**
       * The amount of weight (if any) which should be provided to the message queue for
       * servicing enqueued items.
       *
       * This may be legitimately `None` in the case that you will call
       * `ServiceQueues::service_queues` manually.
       */
      ServiceWeight: PlainDescriptor<Anonymize<I4q39t5hn830vp> | undefined>;
    },
  ];
  OnDemandAssignmentProvider: [
    {
      /**
       * Keeps track of the multiplier used to calculate the current spot price for the on demand
       * assigner.
       */
      SpotTraffic: StorageDescriptor<[], bigint, false>;

      /**
       * The order storage entry. Uses a VecDeque to be able to push to the front of the
       * queue from the scheduler on session boundaries.
       */
      OnDemandQueue: StorageDescriptor<[], Array<number>, false>;

      /**
       * Maps a `ParaId` to `CoreIndex` and keeps track of how many assignments the scheduler has in
       * it's lookahead. Keeping track of this affinity prevents parallel execution of the same
       * `ParaId` on two or more `CoreIndex`es.
       */
      ParaIdAffinity: StorageDescriptor<[Key: number], { core_idx: number; count: number }, true>;
    },
    {
      /**
       *See [`Pallet::place_order_allow_death`].
       */
      place_order_allow_death: TxDescriptor<{ max_amount: bigint; para_id: number }>;

      /**
       *See [`Pallet::place_order_keep_alive`].
       */
      place_order_keep_alive: TxDescriptor<{ max_amount: bigint; para_id: number }>;
    },
    {
      /**
       *An order was placed at some spot price amount.
       */
      OnDemandOrderPlaced: PlainDescriptor<{ para_id: number; spot_price: bigint }>;

      /**
       *The value of the spot traffic multiplier changed.
       */
      SpotTrafficSet: PlainDescriptor<{ traffic: bigint }>;
    },
    {
      /**
       *The `ParaId` supplied to the `place_order` call is not a valid `ParaThread`, making the
       *call is invalid.
       */
      InvalidParaId: PlainDescriptor<undefined>;

      /**
       *The order queue is full, `place_order` will not continue.
       */
      QueueFull: PlainDescriptor<undefined>;

      /**
       *The current spot price is higher than the max amount specified in the `place_order`
       *call, making it invalid.
       */
      SpotPriceHigherThanMaxAmount: PlainDescriptor<undefined>;
    },
    {
      /**
       * The default value for the spot traffic multiplier.
       */
      TrafficDefaultValue: PlainDescriptor<bigint>;
    },
  ];
  ParachainsAssignmentProvider: [{}, {}, {}, {}, {}];
  CoretimeAssignmentProvider: [
    {
      /**
       * Scheduled assignment sets.
       *
       * Assignments as of the given block number. They will go into state once the block number is
       * reached (and replace whatever was in there before).
       */
      CoreSchedules: StorageDescriptor<
        [Key: [number, number]],
        {
          assignments: Anonymize<Id1dkuivtmc10j>;
          end_hint: Anonymize<I4arjljr6dpflb>;
          next_schedule: Anonymize<I4arjljr6dpflb>;
        },
        true
      >;

      /**
       * Assignments which are currently active.
       *
       * They will be picked from `PendingAssignments` once we reach the scheduled block number in
       * `PendingAssignments`.
       */
      CoreDescriptors: StorageDescriptor<
        [Key: number],
        { queue: Anonymize<If4sm31aq7v0i8>; current_work: Anonymize<I154iog8qmouro> },
        false
      >;
    },
    {},
    {},
    {
      /**

 */
      AssignmentsEmpty: PlainDescriptor<undefined>;

      /**
       *Assignments together exceeded 57600.
       */
      OverScheduled: PlainDescriptor<undefined>;

      /**
       *Assignments together less than 57600
       */
      UnderScheduled: PlainDescriptor<undefined>;

      /**
       *assign_core is only allowed to append new assignments at the end of already existing
       *ones.
       */
      DisallowedInsert: PlainDescriptor<undefined>;

      /**
       *Tried to insert a schedule for the same core and block number as an existing schedule
       */
      DuplicateInsert: PlainDescriptor<undefined>;

      /**
       *Tried to add an unsorted set of assignments
       */
      AssignmentsNotSorted: PlainDescriptor<undefined>;
    },
    {},
  ];
  Registrar: [
    {
      /**
       * Pending swap operations.
       */
      PendingSwap: StorageDescriptor<[Key: number], number, true>;

      /**
       * Amount held on deposit for each para and the original depositor.
       *
       * The given account ID is responsible for registering the code and initial head data, but may
       * only do so if it isn't yet registered. (After that, it's up to governance to do so.)
       */
      Paras: StorageDescriptor<
        [Key: number],
        { manager: SS58String; deposit: bigint; locked: Anonymize<I8ie0dco0kcuq5> },
        true
      >;

      /**
       * The next free `ParaId`.
       */
      NextFreeParaId: StorageDescriptor<[], number, false>;
    },
    {
      /**
       *See [`Pallet::register`].
       */
      register: TxDescriptor<{ id: number; genesis_head: Binary; validation_code: Binary }>;

      /**
       *See [`Pallet::force_register`].
       */
      force_register: TxDescriptor<{
        who: SS58String;
        deposit: bigint;
        id: number;
        genesis_head: Binary;
        validation_code: Binary;
      }>;

      /**
       *See [`Pallet::deregister`].
       */
      deregister: TxDescriptor<{ id: number }>;

      /**
       *See [`Pallet::swap`].
       */
      swap: TxDescriptor<{ id: number; other: number }>;

      /**
       *See [`Pallet::remove_lock`].
       */
      remove_lock: TxDescriptor<{ para: number }>;

      /**
       *See [`Pallet::reserve`].
       */
      reserve: TxDescriptor<undefined>;

      /**
       *See [`Pallet::add_lock`].
       */
      add_lock: TxDescriptor<{ para: number }>;

      /**
       *See [`Pallet::schedule_code_upgrade`].
       */
      schedule_code_upgrade: TxDescriptor<{ para: number; new_code: Binary }>;

      /**
       *See [`Pallet::set_current_head`].
       */
      set_current_head: TxDescriptor<{ para: number; new_head: Binary }>;
    },
    {
      /**

 */
      Registered: PlainDescriptor<{ para_id: number; manager: SS58String }>;

      /**

 */
      Deregistered: PlainDescriptor<{ para_id: number }>;

      /**

 */
      Reserved: PlainDescriptor<{ para_id: number; who: SS58String }>;

      /**

 */
      Swapped: PlainDescriptor<{ para_id: number; other_id: number }>;
    },
    {
      /**
       *The ID is not registered.
       */
      NotRegistered: PlainDescriptor<undefined>;

      /**
       *The ID is already registered.
       */
      AlreadyRegistered: PlainDescriptor<undefined>;

      /**
       *The caller is not the owner of this Id.
       */
      NotOwner: PlainDescriptor<undefined>;

      /**
       *Invalid para code size.
       */
      CodeTooLarge: PlainDescriptor<undefined>;

      /**
       *Invalid para head data size.
       */
      HeadDataTooLarge: PlainDescriptor<undefined>;

      /**
       *Para is not a Parachain.
       */
      NotParachain: PlainDescriptor<undefined>;

      /**
       *Para is not a Parathread (on-demand parachain).
       */
      NotParathread: PlainDescriptor<undefined>;

      /**
       *Cannot deregister para
       */
      CannotDeregister: PlainDescriptor<undefined>;

      /**
       *Cannot schedule downgrade of lease holding parachain to on-demand parachain
       */
      CannotDowngrade: PlainDescriptor<undefined>;

      /**
       *Cannot schedule upgrade of on-demand parachain to lease holding parachain
       */
      CannotUpgrade: PlainDescriptor<undefined>;

      /**
       *Para is locked from manipulation by the manager. Must use parachain or relay chain
       *governance.
       */
      ParaLocked: PlainDescriptor<undefined>;

      /**
       *The ID given for registration has not been reserved.
       */
      NotReserved: PlainDescriptor<undefined>;

      /**
       *The validation code is invalid.
       */
      InvalidCode: PlainDescriptor<undefined>;

      /**
       *Cannot perform a parachain slot / lifecycle swap. Check that the state of both paras
       *are correct for the swap to work.
       */
      CannotSwap: PlainDescriptor<undefined>;
    },
    {
      /**
       * The deposit to be paid to run a on-demand parachain.
       * This should include the cost for storing the genesis head and validation code.
       */
      ParaDeposit: PlainDescriptor<bigint>;

      /**
       * The deposit to be paid per byte stored on chain.
       */
      DataDepositPerByte: PlainDescriptor<bigint>;
    },
  ];
  Slots: [
    {
      /**
       * Amounts held on deposit for each (possibly future) leased parachain.
       *
       * The actual amount locked on its behalf by any account at any time is the maximum of the
       * second values of the items in this list whose first value is the account.
       *
       * The first item in the list is the amount locked for the current Lease Period. Following
       * items are for the subsequent lease periods.
       *
       * The default value (an empty list) implies that the parachain no longer exists (or never
       * existed) as far as this pallet is concerned.
       *
       * If a parachain doesn't exist *yet* but is scheduled to exist in the future, then it
       * will be left-padded with one or more `None`s to denote the fact that nothing is held on
       * deposit for the non-existent chain currently, but is held at some point in the future.
       *
       * It is illegal for a `None` value to trail in the list.
       */
      Leases: StorageDescriptor<[Key: number], Array<Anonymize<I92hdo1clkbp4g>>, false>;
    },
    {
      /**
       *See [`Pallet::force_lease`].
       */
      force_lease: TxDescriptor<{
        para: number;
        leaser: SS58String;
        amount: bigint;
        period_begin: number;
        period_count: number;
      }>;

      /**
       *See [`Pallet::clear_all_leases`].
       */
      clear_all_leases: TxDescriptor<{ para: number }>;

      /**
       *See [`Pallet::trigger_onboard`].
       */
      trigger_onboard: TxDescriptor<{ para: number }>;
    },
    {
      /**
       *A new `[lease_period]` is beginning.
       */
      NewLeasePeriod: PlainDescriptor<{ lease_period: number }>;

      /**
       *A para has won the right to a continuous set of lease periods as a parachain.
       *First balance is any extra amount reserved on top of the para's existing deposit.
       *Second balance is the total amount reserved.
       */
      Leased: PlainDescriptor<{
        para_id: number;
        leaser: SS58String;
        period_begin: number;
        period_count: number;
        extra_reserved: bigint;
        total_amount: bigint;
      }>;
    },
    {
      /**
       *The parachain ID is not onboarding.
       */
      ParaNotOnboarding: PlainDescriptor<undefined>;

      /**
       *There was an error with the lease.
       */
      LeaseError: PlainDescriptor<undefined>;
    },
    {
      /**
       * The number of blocks over which a single period lasts.
       */
      LeasePeriod: PlainDescriptor<number>;

      /**
       * The number of blocks to offset each lease period by.
       */
      LeaseOffset: PlainDescriptor<number>;
    },
  ];
  Auctions: [
    {
      /**
       * Number of auctions started so far.
       */
      AuctionCounter: StorageDescriptor<[], number, false>;

      /**
       * Information relating to the current auction, if there is one.
       *
       * The first item in the tuple is the lease period index that the first of the four
       * contiguous lease periods on auction is for. The second is the block number when the
       * auction will "begin to end", i.e. the first block of the Ending Period of the auction.
       */
      AuctionInfo: StorageDescriptor<[], [number, number], true>;

      /**
       * Amounts currently reserved in the accounts of the bidders currently winning
       * (sub-)ranges.
       */
      ReservedAmounts: StorageDescriptor<[Key: [SS58String, number]], bigint, true>;

      /**
       * The winning bids for each of the 10 ranges at each sample in the final Ending Period of
       * the current auction. The map's key is the 0-based index into the Sample Size. The
       * first sample of the ending period is 0; the last is `Sample Size - 1`.
       */
      Winning: StorageDescriptor<[Key: number], Array<Anonymize<I1qlf98109qt29>>, true>;
    },
    {
      /**
       *See [`Pallet::new_auction`].
       */
      new_auction: TxDescriptor<{ duration: number; lease_period_index: number }>;

      /**
       *See [`Pallet::bid`].
       */
      bid: TxDescriptor<{
        para: number;
        auction_index: number;
        first_slot: number;
        last_slot: number;
        amount: bigint;
      }>;

      /**
       *See [`Pallet::cancel_auction`].
       */
      cancel_auction: TxDescriptor<undefined>;
    },
    {
      /**
       *An auction started. Provides its index and the block number where it will begin to
       *close and the first lease period of the quadruplet that is auctioned.
       */
      AuctionStarted: PlainDescriptor<{
        auction_index: number;
        lease_period: number;
        ending: number;
      }>;

      /**
       *An auction ended. All funds become unreserved.
       */
      AuctionClosed: PlainDescriptor<{ auction_index: number }>;

      /**
       *Funds were reserved for a winning bid. First balance is the extra amount reserved.
       *Second is the total.
       */
      Reserved: PlainDescriptor<{
        bidder: SS58String;
        extra_reserved: bigint;
        total_amount: bigint;
      }>;

      /**
       *Funds were unreserved since bidder is no longer active. `[bidder, amount]`
       */
      Unreserved: PlainDescriptor<{ bidder: SS58String; amount: bigint }>;

      /**
       *Someone attempted to lease the same slot twice for a parachain. The amount is held in
       *reserve but no parachain slot has been leased.
       */
      ReserveConfiscated: PlainDescriptor<{ para_id: number; leaser: SS58String; amount: bigint }>;

      /**
       *A new bid has been accepted as the current winner.
       */
      BidAccepted: PlainDescriptor<{
        bidder: SS58String;
        para_id: number;
        amount: bigint;
        first_slot: number;
        last_slot: number;
      }>;

      /**
       *The winning offset was chosen for an auction. This will map into the `Winning` storage
       *map.
       */
      WinningOffset: PlainDescriptor<{ auction_index: number; block_number: number }>;
    },
    {
      /**
       *This auction is already in progress.
       */
      AuctionInProgress: PlainDescriptor<undefined>;

      /**
       *The lease period is in the past.
       */
      LeasePeriodInPast: PlainDescriptor<undefined>;

      /**
       *Para is not registered
       */
      ParaNotRegistered: PlainDescriptor<undefined>;

      /**
       *Not a current auction.
       */
      NotCurrentAuction: PlainDescriptor<undefined>;

      /**
       *Not an auction.
       */
      NotAuction: PlainDescriptor<undefined>;

      /**
       *Auction has already ended.
       */
      AuctionEnded: PlainDescriptor<undefined>;

      /**
       *The para is already leased out for part of this range.
       */
      AlreadyLeasedOut: PlainDescriptor<undefined>;
    },
    {
      /**
       * The number of blocks over which an auction may be retroactively ended.
       */
      EndingPeriod: PlainDescriptor<number>;

      /**
       * The length of each sample to take during the ending period.
       *
       * `EndingPeriod` / `SampleLength` = Total # of Samples
       */
      SampleLength: PlainDescriptor<number>;

      /**

 */
      SlotRangeCount: PlainDescriptor<number>;

      /**

 */
      LeasePeriodsPerSlot: PlainDescriptor<number>;
    },
  ];
  Crowdloan: [
    {
      /**
       * Info on all of the funds.
       */
      Funds: StorageDescriptor<
        [Key: number],
        {
          depositor: SS58String;
          verifier: Anonymize<I8t18p6mokc3s4>;
          deposit: bigint;
          raised: bigint;
          end: number;
          cap: bigint;
          last_contribution: CommonCrowdloanLastContribution;
          first_period: number;
          last_period: number;
          fund_index: number;
        },
        true
      >;

      /**
       * The funds that have had additional contributions during the last block. This is used
       * in order to determine which funds should submit new or updated bids.
       */
      NewRaise: StorageDescriptor<[], Array<number>, false>;

      /**
       * The number of auctions that have entered into their ending period so far.
       */
      EndingsCount: StorageDescriptor<[], number, false>;

      /**
       * Tracker for the next available fund index
       */
      NextFundIndex: StorageDescriptor<[], number, false>;
    },
    {
      /**
       *See [`Pallet::create`].
       */
      create: TxDescriptor<{
        index: number;
        cap: bigint;
        first_period: number;
        last_period: number;
        end: number;
        verifier: Anonymize<I8t18p6mokc3s4>;
      }>;

      /**
       *See [`Pallet::contribute`].
       */
      contribute: TxDescriptor<{
        index: number;
        value: bigint;
        signature: Anonymize<I7us28h09nc5sv>;
      }>;

      /**
       *See [`Pallet::withdraw`].
       */
      withdraw: TxDescriptor<{ who: SS58String; index: number }>;

      /**
       *See [`Pallet::refund`].
       */
      refund: TxDescriptor<{ index: number }>;

      /**
       *See [`Pallet::dissolve`].
       */
      dissolve: TxDescriptor<{ index: number }>;

      /**
       *See [`Pallet::edit`].
       */
      edit: TxDescriptor<{
        index: number;
        cap: bigint;
        first_period: number;
        last_period: number;
        end: number;
        verifier: Anonymize<I8t18p6mokc3s4>;
      }>;

      /**
       *See [`Pallet::add_memo`].
       */
      add_memo: TxDescriptor<{ index: number; memo: Binary }>;

      /**
       *See [`Pallet::poke`].
       */
      poke: TxDescriptor<{ index: number }>;

      /**
       *See [`Pallet::contribute_all`].
       */
      contribute_all: TxDescriptor<{ index: number; signature: Anonymize<I7us28h09nc5sv> }>;
    },
    {
      /**
       *Create a new crowdloaning campaign.
       */
      Created: PlainDescriptor<{ para_id: number }>;

      /**
       *Contributed to a crowd sale.
       */
      Contributed: PlainDescriptor<{ who: SS58String; fund_index: number; amount: bigint }>;

      /**
       *Withdrew full balance of a contributor.
       */
      Withdrew: PlainDescriptor<{ who: SS58String; fund_index: number; amount: bigint }>;

      /**
       *The loans in a fund have been partially dissolved, i.e. there are some left
       *over child keys that still need to be killed.
       */
      PartiallyRefunded: PlainDescriptor<{ para_id: number }>;

      /**
       *All loans in a fund have been refunded.
       */
      AllRefunded: PlainDescriptor<{ para_id: number }>;

      /**
       *Fund is dissolved.
       */
      Dissolved: PlainDescriptor<{ para_id: number }>;

      /**
       *The result of trying to submit a new bid to the Slots pallet.
       */
      HandleBidResult: PlainDescriptor<{ para_id: number; result: Anonymize<Idtdr91jmq5g4i> }>;

      /**
       *The configuration to a crowdloan has been edited.
       */
      Edited: PlainDescriptor<{ para_id: number }>;

      /**
       *A memo has been updated.
       */
      MemoUpdated: PlainDescriptor<{ who: SS58String; para_id: number; memo: Binary }>;

      /**
       *A parachain has been moved to `NewRaise`
       */
      AddedToNewRaise: PlainDescriptor<{ para_id: number }>;
    },
    {
      /**
       *The current lease period is more than the first lease period.
       */
      FirstPeriodInPast: PlainDescriptor<undefined>;

      /**
       *The first lease period needs to at least be less than 3 `max_value`.
       */
      FirstPeriodTooFarInFuture: PlainDescriptor<undefined>;

      /**
       *Last lease period must be greater than first lease period.
       */
      LastPeriodBeforeFirstPeriod: PlainDescriptor<undefined>;

      /**
       *The last lease period cannot be more than 3 periods after the first period.
       */
      LastPeriodTooFarInFuture: PlainDescriptor<undefined>;

      /**
       *The campaign ends before the current block number. The end must be in the future.
       */
      CannotEndInPast: PlainDescriptor<undefined>;

      /**
       *The end date for this crowdloan is not sensible.
       */
      EndTooFarInFuture: PlainDescriptor<undefined>;

      /**
       *There was an overflow.
       */
      Overflow: PlainDescriptor<undefined>;

      /**
       *The contribution was below the minimum, `MinContribution`.
       */
      ContributionTooSmall: PlainDescriptor<undefined>;

      /**
       *Invalid fund index.
       */
      InvalidParaId: PlainDescriptor<undefined>;

      /**
       *Contributions exceed maximum amount.
       */
      CapExceeded: PlainDescriptor<undefined>;

      /**
       *The contribution period has already ended.
       */
      ContributionPeriodOver: PlainDescriptor<undefined>;

      /**
       *The origin of this call is invalid.
       */
      InvalidOrigin: PlainDescriptor<undefined>;

      /**
       *This crowdloan does not correspond to a parachain.
       */
      NotParachain: PlainDescriptor<undefined>;

      /**
       *This parachain lease is still active and retirement cannot yet begin.
       */
      LeaseActive: PlainDescriptor<undefined>;

      /**
       *This parachain's bid or lease is still active and withdraw cannot yet begin.
       */
      BidOrLeaseActive: PlainDescriptor<undefined>;

      /**
       *The crowdloan has not yet ended.
       */
      FundNotEnded: PlainDescriptor<undefined>;

      /**
       *There are no contributions stored in this crowdloan.
       */
      NoContributions: PlainDescriptor<undefined>;

      /**
       *The crowdloan is not ready to dissolve. Potentially still has a slot or in retirement
       *period.
       */
      NotReadyToDissolve: PlainDescriptor<undefined>;

      /**
       *Invalid signature.
       */
      InvalidSignature: PlainDescriptor<undefined>;

      /**
       *The provided memo is too large.
       */
      MemoTooLarge: PlainDescriptor<undefined>;

      /**
       *The fund is already in `NewRaise`
       */
      AlreadyInNewRaise: PlainDescriptor<undefined>;

      /**
       *No contributions allowed during the VRF delay
       */
      VrfDelayInProgress: PlainDescriptor<undefined>;

      /**
       *A lease period has not started yet, due to an offset in the starting block.
       */
      NoLeasePeriod: PlainDescriptor<undefined>;
    },
    {
      /**
       * `PalletId` for the crowdloan pallet. An appropriate value could be
       * `PalletId(*b"py/cfund")`
       */
      PalletId: PlainDescriptor<Binary>;

      /**
       * The minimum amount that may be contributed into a crowdloan. Should almost certainly be
       * at least `ExistentialDeposit`.
       */
      MinContribution: PlainDescriptor<bigint>;

      /**
       * Max number of storage keys to remove per extrinsic call.
       */
      RemoveKeysLimit: PlainDescriptor<number>;
    },
  ];
  Coretime: [
    {},
    {
      /**
       *See [`Pallet::request_core_count`].
       */
      request_core_count: TxDescriptor<{ count: number }>;

      /**
       *See [`Pallet::assign_core`].
       */
      assign_core: TxDescriptor<{
        core: number;
        begin: number;
        assignment: Anonymize<Id1dkuivtmc10j>;
        end_hint: Anonymize<I4arjljr6dpflb>;
      }>;
    },
    {
      /**
       *The broker chain has asked for revenue information for a specific block.
       */
      RevenueInfoRequested: PlainDescriptor<{ when: number }>;

      /**
       *A core has received a new assignment from the broker chain.
       */
      CoreAssigned: PlainDescriptor<{ core: number }>;
    },
    {
      /**
       *The paraid making the call is not the coretime brokerage system parachain.
       */
      NotBroker: PlainDescriptor<undefined>;
    },
    {
      /**
       * The ParaId of the broker system parachain.
       */
      BrokerId: PlainDescriptor<number>;
    },
  ];
  XcmPallet: [
    {
      /**
       * The latest available query index.
       */
      QueryCounter: StorageDescriptor<[], bigint, false>;

      /**
       * The ongoing queries.
       */
      Queries: StorageDescriptor<[Key: bigint], XcmPalletQueryStatus, true>;

      /**
       * The existing asset traps.
       *
       * Key is the blake2 256 hash of (origin, versioned `Assets`) pair. Value is the number of
       * times this pair has been trapped (usually just 1 if it exists at all).
       */
      AssetTraps: StorageDescriptor<[Key: Binary], number, false>;

      /**
       * Default version to encode XCM when latest version of destination is unknown. If `None`,
       * then the destinations whose XCM version is unknown are considered unreachable.
       */
      SafeXcmVersion: StorageDescriptor<[], number, true>;

      /**
       * The Latest versions that we know various locations support.
       */
      SupportedVersion: StorageDescriptor<[number, XcmVersionedLocation], number, true>;

      /**
       * All locations that we have requested version notifications from.
       */
      VersionNotifiers: StorageDescriptor<[number, XcmVersionedLocation], bigint, true>;

      /**
       * The target locations that are subscribed to our version changes, as well as the most recent
       * of our versions we informed them of.
       */
      VersionNotifyTargets: StorageDescriptor<
        [number, XcmVersionedLocation],
        [bigint, Anonymize<I4q39t5hn830vp>, number],
        true
      >;

      /**
       * Destinations whose latest XCM version we would like to know. Duplicates not allowed, and
       * the `u32` counter is the number of times that a send to the destination has been attempted,
       * which is used as a prioritization.
       */
      VersionDiscoveryQueue: StorageDescriptor<[], Array<Anonymize<Iak31q8r4mqtse>>, false>;

      /**
       * The current migration's stage, if any.
       */
      CurrentMigration: StorageDescriptor<[], XcmPalletVersionMigrationStage, true>;

      /**
       * Fungible assets which we know are locked on a remote chain.
       */
      RemoteLockedFungibles: StorageDescriptor<
        [number, SS58String, XcmVersionedAssetId],
        {
          amount: bigint;
          owner: XcmVersionedLocation;
          locker: XcmVersionedLocation;
          consumers: Anonymize<I48jka0f0ufl6q>;
        },
        true
      >;

      /**
       * Fungible assets which we know are locked on this chain.
       */
      LockedFungibles: StorageDescriptor<[Key: SS58String], Array<Anonymize<I5pu2dsc6d3iv6>>, true>;

      /**
       * Global suspension state of the XCM executor.
       */
      XcmExecutionSuspended: StorageDescriptor<[], boolean, false>;
    },
    {
      /**
       *See [`Pallet::send`].
       */
      send: TxDescriptor<{ dest: XcmVersionedLocation; message: XcmVersionedXcm }>;

      /**
       *See [`Pallet::teleport_assets`].
       */
      teleport_assets: TxDescriptor<{
        dest: XcmVersionedLocation;
        beneficiary: XcmVersionedLocation;
        assets: XcmVersionedAssets;
        fee_asset_item: number;
      }>;

      /**
       *See [`Pallet::reserve_transfer_assets`].
       */
      reserve_transfer_assets: TxDescriptor<{
        dest: XcmVersionedLocation;
        beneficiary: XcmVersionedLocation;
        assets: XcmVersionedAssets;
        fee_asset_item: number;
      }>;

      /**
       *See [`Pallet::execute`].
       */
      execute: TxDescriptor<{ message: XcmVersionedXcm1; max_weight: Anonymize<I4q39t5hn830vp> }>;

      /**
       *See [`Pallet::force_xcm_version`].
       */
      force_xcm_version: TxDescriptor<{ location: Anonymize<Ieq428hdmibf6m>; version: number }>;

      /**
       *See [`Pallet::force_default_xcm_version`].
       */
      force_default_xcm_version: TxDescriptor<{ maybe_xcm_version: Anonymize<I4arjljr6dpflb> }>;

      /**
       *See [`Pallet::force_subscribe_version_notify`].
       */
      force_subscribe_version_notify: TxDescriptor<{ location: XcmVersionedLocation }>;

      /**
       *See [`Pallet::force_unsubscribe_version_notify`].
       */
      force_unsubscribe_version_notify: TxDescriptor<{ location: XcmVersionedLocation }>;

      /**
       *See [`Pallet::limited_reserve_transfer_assets`].
       */
      limited_reserve_transfer_assets: TxDescriptor<{
        dest: XcmVersionedLocation;
        beneficiary: XcmVersionedLocation;
        assets: XcmVersionedAssets;
        fee_asset_item: number;
        weight_limit: XcmV3WeightLimit;
      }>;

      /**
       *See [`Pallet::limited_teleport_assets`].
       */
      limited_teleport_assets: TxDescriptor<{
        dest: XcmVersionedLocation;
        beneficiary: XcmVersionedLocation;
        assets: XcmVersionedAssets;
        fee_asset_item: number;
        weight_limit: XcmV3WeightLimit;
      }>;

      /**
       *See [`Pallet::force_suspension`].
       */
      force_suspension: TxDescriptor<{ suspended: boolean }>;

      /**
       *See [`Pallet::transfer_assets`].
       */
      transfer_assets: TxDescriptor<{
        dest: XcmVersionedLocation;
        beneficiary: XcmVersionedLocation;
        assets: XcmVersionedAssets;
        fee_asset_item: number;
        weight_limit: XcmV3WeightLimit;
      }>;
    },
    {
      /**
       *Execution of an XCM message was attempted.
       */
      Attempted: PlainDescriptor<{ outcome: XcmV4TraitsOutcome }>;

      /**
       *A XCM message was sent.
       */
      Sent: PlainDescriptor<{
        origin: Anonymize<Ieq428hdmibf6m>;
        destination: Anonymize<Ieq428hdmibf6m>;
        message: Anonymize<I9uem0hkif9mb7>;
        message_id: Binary;
      }>;

      /**
       *Query response received which does not match a registered query. This may be because a
       *matching query was never registered, it may be because it is a duplicate response, or
       *because the query timed out.
       */
      UnexpectedResponse: PlainDescriptor<{ origin: Anonymize<Ieq428hdmibf6m>; query_id: bigint }>;

      /**
       *Query response has been received and is ready for taking with `take_response`. There is
       *no registered notification call.
       */
      ResponseReady: PlainDescriptor<{ query_id: bigint; response: XcmV4Response }>;

      /**
       *Query response has been received and query is removed. The registered notification has
       *been dispatched and executed successfully.
       */
      Notified: PlainDescriptor<{ query_id: bigint; pallet_index: number; call_index: number }>;

      /**
       *Query response has been received and query is removed. The registered notification
       *could not be dispatched because the dispatch weight is greater than the maximum weight
       *originally budgeted by this runtime for the query result.
       */
      NotifyOverweight: PlainDescriptor<{
        query_id: bigint;
        pallet_index: number;
        call_index: number;
        actual_weight: Anonymize<I4q39t5hn830vp>;
        max_budgeted_weight: Anonymize<I4q39t5hn830vp>;
      }>;

      /**
       *Query response has been received and query is removed. There was a general error with
       *dispatching the notification call.
       */
      NotifyDispatchError: PlainDescriptor<{
        query_id: bigint;
        pallet_index: number;
        call_index: number;
      }>;

      /**
       *Query response has been received and query is removed. The dispatch was unable to be
       *decoded into a `Call`; this might be due to dispatch function having a signature which
       *is not `(origin, QueryId, Response)`.
       */
      NotifyDecodeFailed: PlainDescriptor<{
        query_id: bigint;
        pallet_index: number;
        call_index: number;
      }>;

      /**
       *Expected query response has been received but the origin location of the response does
       *not match that expected. The query remains registered for a later, valid, response to
       *be received and acted upon.
       */
      InvalidResponder: PlainDescriptor<{
        origin: Anonymize<Ieq428hdmibf6m>;
        query_id: bigint;
        expected_location: Anonymize<Iehbh3m350i09b>;
      }>;

      /**
       *Expected query response has been received but the expected origin location placed in
       *storage by this runtime previously cannot be decoded. The query remains registered.
       *
       *This is unexpected (since a location placed in storage in a previously executing
       *runtime should be readable prior to query timeout) and dangerous since the possibly
       *valid response will be dropped. Manual governance intervention is probably going to be
       *needed.
       */
      InvalidResponderVersion: PlainDescriptor<{
        origin: Anonymize<Ieq428hdmibf6m>;
        query_id: bigint;
      }>;

      /**
       *Received query response has been read and removed.
       */
      ResponseTaken: PlainDescriptor<{ query_id: bigint }>;

      /**
       *Some assets have been placed in an asset trap.
       */
      AssetsTrapped: PlainDescriptor<{
        hash: Binary;
        origin: Anonymize<Ieq428hdmibf6m>;
        assets: XcmVersionedAssets;
      }>;

      /**
       *An XCM version change notification message has been attempted to be sent.
       *
       *The cost of sending it (borne by the chain) is included.
       */
      VersionChangeNotified: PlainDescriptor<{
        destination: Anonymize<Ieq428hdmibf6m>;
        result: number;
        cost: Anonymize<Ia04r3ent3ib9l>;
        message_id: Binary;
      }>;

      /**
       *The supported version of a location has been changed. This might be through an
       *automatic notification or a manual intervention.
       */
      SupportedVersionChanged: PlainDescriptor<{
        location: Anonymize<Ieq428hdmibf6m>;
        version: number;
      }>;

      /**
       *A given location which had a version change subscription was dropped owing to an error
       *sending the notification to it.
       */
      NotifyTargetSendFail: PlainDescriptor<{
        location: Anonymize<Ieq428hdmibf6m>;
        query_id: bigint;
        error: XcmV3TraitsError;
      }>;

      /**
       *A given location which had a version change subscription was dropped owing to an error
       *migrating the location to our new XCM format.
       */
      NotifyTargetMigrationFail: PlainDescriptor<{
        location: XcmVersionedLocation;
        query_id: bigint;
      }>;

      /**
       *Expected query response has been received but the expected querier location placed in
       *storage by this runtime previously cannot be decoded. The query remains registered.
       *
       *This is unexpected (since a location placed in storage in a previously executing
       *runtime should be readable prior to query timeout) and dangerous since the possibly
       *valid response will be dropped. Manual governance intervention is probably going to be
       *needed.
       */
      InvalidQuerierVersion: PlainDescriptor<{
        origin: Anonymize<Ieq428hdmibf6m>;
        query_id: bigint;
      }>;

      /**
       *Expected query response has been received but the querier location of the response does
       *not match the expected. The query remains registered for a later, valid, response to
       *be received and acted upon.
       */
      InvalidQuerier: PlainDescriptor<{
        origin: Anonymize<Ieq428hdmibf6m>;
        query_id: bigint;
        expected_querier: Anonymize<Ieq428hdmibf6m>;
        maybe_actual_querier: Anonymize<Iehbh3m350i09b>;
      }>;

      /**
       *A remote has requested XCM version change notification from us and we have honored it.
       *A version information message is sent to them and its cost is included.
       */
      VersionNotifyStarted: PlainDescriptor<{
        destination: Anonymize<Ieq428hdmibf6m>;
        cost: Anonymize<Ia04r3ent3ib9l>;
        message_id: Binary;
      }>;

      /**
       *We have requested that a remote chain send us XCM version change notifications.
       */
      VersionNotifyRequested: PlainDescriptor<{
        destination: Anonymize<Ieq428hdmibf6m>;
        cost: Anonymize<Ia04r3ent3ib9l>;
        message_id: Binary;
      }>;

      /**
       *We have requested that a remote chain stops sending us XCM version change
       *notifications.
       */
      VersionNotifyUnrequested: PlainDescriptor<{
        destination: Anonymize<Ieq428hdmibf6m>;
        cost: Anonymize<Ia04r3ent3ib9l>;
        message_id: Binary;
      }>;

      /**
       *Fees were paid from a location for an operation (often for using `SendXcm`).
       */
      FeesPaid: PlainDescriptor<{
        paying: Anonymize<Ieq428hdmibf6m>;
        fees: Anonymize<Ia04r3ent3ib9l>;
      }>;

      /**
       *Some assets have been claimed from an asset trap
       */
      AssetsClaimed: PlainDescriptor<{
        hash: Binary;
        origin: Anonymize<Ieq428hdmibf6m>;
        assets: XcmVersionedAssets;
      }>;

      /**
       *A XCM version migration finished.
       */
      VersionMigrationFinished: PlainDescriptor<{ version: number }>;
    },
    {
      /**
       *The desired destination was unreachable, generally because there is a no way of routing
       *to it.
       */
      Unreachable: PlainDescriptor<undefined>;

      /**
       *There was some other issue (i.e. not to do with routing) in sending the message.
       *Perhaps a lack of space for buffering the message.
       */
      SendFailure: PlainDescriptor<undefined>;

      /**
       *The message execution fails the filter.
       */
      Filtered: PlainDescriptor<undefined>;

      /**
       *The message's weight could not be determined.
       */
      UnweighableMessage: PlainDescriptor<undefined>;

      /**
       *The destination `Location` provided cannot be inverted.
       */
      DestinationNotInvertible: PlainDescriptor<undefined>;

      /**
       *The assets to be sent are empty.
       */
      Empty: PlainDescriptor<undefined>;

      /**
       *Could not re-anchor the assets to declare the fees for the destination chain.
       */
      CannotReanchor: PlainDescriptor<undefined>;

      /**
       *Too many assets have been attempted for transfer.
       */
      TooManyAssets: PlainDescriptor<undefined>;

      /**
       *Origin is invalid for sending.
       */
      InvalidOrigin: PlainDescriptor<undefined>;

      /**
       *The version of the `Versioned` value used is not able to be interpreted.
       */
      BadVersion: PlainDescriptor<undefined>;

      /**
       *The given location could not be used (e.g. because it cannot be expressed in the
       *desired version of XCM).
       */
      BadLocation: PlainDescriptor<undefined>;

      /**
       *The referenced subscription could not be found.
       */
      NoSubscription: PlainDescriptor<undefined>;

      /**
       *The location is invalid since it already has a subscription from us.
       */
      AlreadySubscribed: PlainDescriptor<undefined>;

      /**
       *Could not check-out the assets for teleportation to the destination chain.
       */
      CannotCheckOutTeleport: PlainDescriptor<undefined>;

      /**
       *The owner does not own (all) of the asset that they wish to do the operation on.
       */
      LowBalance: PlainDescriptor<undefined>;

      /**
       *The asset owner has too many locks on the asset.
       */
      TooManyLocks: PlainDescriptor<undefined>;

      /**
       *The given account is not an identifiable sovereign account for any location.
       */
      AccountNotSovereign: PlainDescriptor<undefined>;

      /**
       *The operation required fees to be paid which the initiator could not meet.
       */
      FeesNotMet: PlainDescriptor<undefined>;

      /**
       *A remote lock with the corresponding data could not be found.
       */
      LockNotFound: PlainDescriptor<undefined>;

      /**
       *The unlock operation cannot succeed because there are still consumers of the lock.
       */
      InUse: PlainDescriptor<undefined>;

      /**
       *Invalid non-concrete asset.
       */
      InvalidAssetNotConcrete: PlainDescriptor<undefined>;

      /**
       *Invalid asset, reserve chain could not be determined for it.
       */
      InvalidAssetUnknownReserve: PlainDescriptor<undefined>;

      /**
       *Invalid asset, do not support remote asset reserves with different fees reserves.
       */
      InvalidAssetUnsupportedReserve: PlainDescriptor<undefined>;

      /**
       *Too many assets with different reserve locations have been attempted for transfer.
       */
      TooManyReserves: PlainDescriptor<undefined>;

      /**
       *Local XCM execution incomplete.
       */
      LocalExecutionIncomplete: PlainDescriptor<undefined>;
    },
    {},
  ];
  Beefy: [
    {
      /**
       * The current authorities set
       */
      Authorities: StorageDescriptor<[], Array<Binary>, false>;

      /**
       * The current validator set id
       */
      ValidatorSetId: StorageDescriptor<[], bigint, false>;

      /**
       * Authorities set scheduled to be used with the next session
       */
      NextAuthorities: StorageDescriptor<[], Array<Binary>, false>;

      /**
       * A mapping from BEEFY set ID to the index of the *most recent* session for which its
       * members were responsible.
       *
       * This is only used for validating equivocation proofs. An equivocation proof must
       * contains a key-ownership proof for a given session, therefore we need a way to tie
       * together sessions and BEEFY set ids, i.e. we need to validate that a validator
       * was the owner of a given key on a given session, and what the active set ID was
       * during that session.
       *
       * TWOX-NOTE: `ValidatorSetId` is not under user control.
       */
      SetIdSession: StorageDescriptor<[Key: bigint], number, true>;

      /**
       * Block number where BEEFY consensus is enabled/started.
       * By changing this (through privileged `set_new_genesis()`), BEEFY consensus is effectively
       * restarted from the newly set block number.
       */
      GenesisBlock: StorageDescriptor<[], number | undefined, false>;
    },
    {
      /**
       *See [`Pallet::report_equivocation`].
       */
      report_equivocation: TxDescriptor<{
        equivocation_proof: Anonymize<I6lgkrki6lhal>;
        key_owner_proof: Anonymize<I3ia7aufsoj0l1>;
      }>;

      /**
       *See [`Pallet::report_equivocation_unsigned`].
       */
      report_equivocation_unsigned: TxDescriptor<{
        equivocation_proof: Anonymize<I6lgkrki6lhal>;
        key_owner_proof: Anonymize<I3ia7aufsoj0l1>;
      }>;

      /**
       *See [`Pallet::set_new_genesis`].
       */
      set_new_genesis: TxDescriptor<{ delay_in_blocks: number }>;
    },
    {},
    {
      /**
       *A key ownership proof provided as part of an equivocation report is invalid.
       */
      InvalidKeyOwnershipProof: PlainDescriptor<undefined>;

      /**
       *An equivocation proof provided as part of an equivocation report is invalid.
       */
      InvalidEquivocationProof: PlainDescriptor<undefined>;

      /**
       *A given equivocation report is valid but already previously reported.
       */
      DuplicateOffenceReport: PlainDescriptor<undefined>;

      /**
       *Submitted configuration is invalid.
       */
      InvalidConfiguration: PlainDescriptor<undefined>;
    },
    {
      /**
       * The maximum number of authorities that can be added.
       */
      MaxAuthorities: PlainDescriptor<number>;

      /**
       * The maximum number of nominators for each validator.
       */
      MaxNominators: PlainDescriptor<number>;

      /**
       * The maximum number of entries to keep in the set id to session index mapping.
       *
       * Since the `SetIdSession` map is only used for validating equivocations this
       * value should relate to the bonding duration of whatever staking system is
       * being used (if any). If equivocation handling is not enabled then this value
       * can be zero.
       */
      MaxSetIdSessionEntries: PlainDescriptor<bigint>;
    },
  ];
  Mmr: [
    {
      /**
       * Latest MMR Root hash.
       */
      RootHash: StorageDescriptor<[], Binary, false>;

      /**
       * Current size of the MMR (number of leaves).
       */
      NumberOfLeaves: StorageDescriptor<[], bigint, false>;

      /**
       * Hashes of the nodes in the MMR.
       *
       * Note this collection only contains MMR peaks, the inner nodes (and leaves)
       * are pruned and only stored in the Offchain DB.
       */
      Nodes: StorageDescriptor<[Key: bigint], Binary, true>;
    },
    {},
    {},
    {},
    {},
  ];
  MmrLeaf: [
    {
      /**
       * Details of current BEEFY authority set.
       */
      BeefyAuthorities: StorageDescriptor<
        [],
        { id: bigint; len: number; keyset_commitment: Binary },
        false
      >;

      /**
       * Details of next BEEFY authority set.
       *
       * This storage entry is used as cache for calls to `update_beefy_next_authority_set`.
       */
      BeefyNextAuthorities: StorageDescriptor<
        [],
        { id: bigint; len: number; keyset_commitment: Binary },
        false
      >;
    },
    {},
    {},
    {},
    {},
  ];
  IdentityMigrator: [
    {},
    {
      /**
       *See [`Pallet::reap_identity`].
       */
      reap_identity: TxDescriptor<{ who: SS58String }>;

      /**
       *See [`Pallet::poke_deposit`].
       */
      poke_deposit: TxDescriptor<{ who: SS58String }>;
    },
    {
      /**
       *The identity and all sub accounts were reaped for `who`.
       */
      IdentityReaped: PlainDescriptor<{ who: SS58String }>;

      /**
       *The deposits held for `who` were updated. `identity` is the new deposit held for
       *identity info, and `subs` is the new deposit held for the sub-accounts.
       */
      DepositUpdated: PlainDescriptor<{ who: SS58String; identity: bigint; subs: bigint }>;
    },
    {},
    {},
  ];
  ParasSudoWrapper: [
    {},
    {
      /**
       *See [`Pallet::sudo_schedule_para_initialize`].
       */
      sudo_schedule_para_initialize: TxDescriptor<{
        id: number;
        genesis: Anonymize<I2duhnt686rv0q>;
      }>;

      /**
       *See [`Pallet::sudo_schedule_para_cleanup`].
       */
      sudo_schedule_para_cleanup: TxDescriptor<{ id: number }>;

      /**
       *See [`Pallet::sudo_schedule_parathread_upgrade`].
       */
      sudo_schedule_parathread_upgrade: TxDescriptor<{ id: number }>;

      /**
       *See [`Pallet::sudo_schedule_parachain_downgrade`].
       */
      sudo_schedule_parachain_downgrade: TxDescriptor<{ id: number }>;

      /**
       *See [`Pallet::sudo_queue_downward_xcm`].
       */
      sudo_queue_downward_xcm: TxDescriptor<{ id: number; xcm: XcmVersionedXcm }>;

      /**
       *See [`Pallet::sudo_establish_hrmp_channel`].
       */
      sudo_establish_hrmp_channel: TxDescriptor<{
        sender: number;
        recipient: number;
        max_capacity: number;
        max_message_size: number;
      }>;
    },
    {},
    {
      /**
       *The specified parachain is not registered.
       */
      ParaDoesntExist: PlainDescriptor<undefined>;

      /**
       *The specified parachain is already registered.
       */
      ParaAlreadyExists: PlainDescriptor<undefined>;

      /**
       *A DMP message couldn't be sent because it exceeds the maximum size allowed for a
       *downward message.
       */
      ExceedsMaxMessageSize: PlainDescriptor<undefined>;

      /**
       *Could not schedule para cleanup.
       */
      CouldntCleanup: PlainDescriptor<undefined>;

      /**
       *Not a parathread (on-demand parachain).
       */
      NotParathread: PlainDescriptor<undefined>;

      /**
       *Not a lease holding parachain.
       */
      NotParachain: PlainDescriptor<undefined>;

      /**
       *Cannot upgrade on-demand parachain to lease holding parachain.
       */
      CannotUpgrade: PlainDescriptor<undefined>;

      /**
       *Cannot downgrade lease holding parachain to on-demand.
       */
      CannotDowngrade: PlainDescriptor<undefined>;

      /**
       *There are more cores than supported by the runtime.
       */
      TooManyCores: PlainDescriptor<undefined>;
    },
    {},
  ];
  AssignedSlots: [
    {
      /**
       * Assigned permanent slots, with their start lease period, and duration.
       */
      PermanentSlots: StorageDescriptor<[Key: number], [number, number], true>;

      /**
       * Number of assigned (and active) permanent slots.
       */
      PermanentSlotCount: StorageDescriptor<[], number, false>;

      /**
       * Assigned temporary slots.
       */
      TemporarySlots: StorageDescriptor<
        [Key: number],
        {
          manager: SS58String;
          period_begin: number;
          period_count: number;
          last_lease: Anonymize<I4arjljr6dpflb>;
          lease_count: number;
        },
        true
      >;

      /**
       * Number of assigned temporary slots.
       */
      TemporarySlotCount: StorageDescriptor<[], number, false>;

      /**
       * Number of active temporary slots in current slot lease period.
       */
      ActiveTemporarySlotCount: StorageDescriptor<[], number, false>;

      /**
       *  The max number of temporary slots that can be assigned.
       */
      MaxTemporarySlots: StorageDescriptor<[], number, false>;

      /**
       * The max number of permanent slots that can be assigned.
       */
      MaxPermanentSlots: StorageDescriptor<[], number, false>;
    },
    {
      /**
       *See [`Pallet::assign_perm_parachain_slot`].
       */
      assign_perm_parachain_slot: TxDescriptor<{ id: number }>;

      /**
       *See [`Pallet::assign_temp_parachain_slot`].
       */
      assign_temp_parachain_slot: TxDescriptor<{
        id: number;
        lease_period_start: PolkadotRuntimeCommonAssignedSlotsSlotLeasePeriodStart;
      }>;

      /**
       *See [`Pallet::unassign_parachain_slot`].
       */
      unassign_parachain_slot: TxDescriptor<{ id: number }>;

      /**
       *See [`Pallet::set_max_permanent_slots`].
       */
      set_max_permanent_slots: TxDescriptor<{ slots: number }>;

      /**
       *See [`Pallet::set_max_temporary_slots`].
       */
      set_max_temporary_slots: TxDescriptor<{ slots: number }>;
    },
    {
      /**
       *A parachain was assigned a permanent parachain slot
       */
      PermanentSlotAssigned: PlainDescriptor<number>;

      /**
       *A parachain was assigned a temporary parachain slot
       */
      TemporarySlotAssigned: PlainDescriptor<number>;

      /**
       *The maximum number of permanent slots has been changed
       */
      MaxPermanentSlotsChanged: PlainDescriptor<{ slots: number }>;

      /**
       *The maximum number of temporary slots has been changed
       */
      MaxTemporarySlotsChanged: PlainDescriptor<{ slots: number }>;
    },
    {
      /**
       *The specified parachain is not registered.
       */
      ParaDoesntExist: PlainDescriptor<undefined>;

      /**
       *Not a parathread (on-demand parachain).
       */
      NotParathread: PlainDescriptor<undefined>;

      /**
       *Cannot upgrade on-demand parachain to lease holding
       *parachain.
       */
      CannotUpgrade: PlainDescriptor<undefined>;

      /**
       *Cannot downgrade lease holding parachain to
       *on-demand.
       */
      CannotDowngrade: PlainDescriptor<undefined>;

      /**
       *Permanent or Temporary slot already assigned.
       */
      SlotAlreadyAssigned: PlainDescriptor<undefined>;

      /**
       *Permanent or Temporary slot has not been assigned.
       */
      SlotNotAssigned: PlainDescriptor<undefined>;

      /**
       *An ongoing lease already exists.
       */
      OngoingLeaseExists: PlainDescriptor<undefined>;

      /**

 */
      MaxPermanentSlotsExceeded: PlainDescriptor<undefined>;

      /**

 */
      MaxTemporarySlotsExceeded: PlainDescriptor<undefined>;
    },
    {
      /**
       * The number of lease periods a permanent parachain slot lasts.
       */
      PermanentSlotLeasePeriodLength: PlainDescriptor<number>;

      /**
       * The number of lease periods a temporary parachain slot lasts.
       */
      TemporarySlotLeasePeriodLength: PlainDescriptor<number>;

      /**
       * The max number of temporary slots to be scheduled per lease periods.
       */
      MaxTemporarySlotPerLeasePeriod: PlainDescriptor<number>;
    },
  ];
  ValidatorManager: [
    {
      /**
       * Validators that should be retired, because their Parachain was deregistered.
       */
      ValidatorsToRetire: StorageDescriptor<[], Array<SS58String>, false>;

      /**
       * Validators that should be added.
       */
      ValidatorsToAdd: StorageDescriptor<[], Array<SS58String>, false>;
    },
    {
      /**
       *See [`Pallet::register_validators`].
       */
      register_validators: TxDescriptor<{ validators: Anonymize<Ia2lhg7l2hilo3> }>;

      /**
       *See [`Pallet::deregister_validators`].
       */
      deregister_validators: TxDescriptor<{ validators: Anonymize<Ia2lhg7l2hilo3> }>;
    },
    {
      /**
       *New validators were added to the set.
       */
      ValidatorsRegistered: PlainDescriptor<Array<SS58String>>;

      /**
       *Validators were removed from the set.
       */
      ValidatorsDeregistered: PlainDescriptor<Array<SS58String>>;
    },
    {},
    {},
  ];
  StateTrieMigration: [
    {
      /**
       * Migration progress.
       *
       * This stores the snapshot of the last migrated keys. It can be set into motion and move
       * forward by any of the means provided by this pallet.
       */
      MigrationProcess: StorageDescriptor<
        [],
        {
          progress_top: Anonymize<I2jvfaenrpg1ai>;
          progress_child: Anonymize<I2jvfaenrpg1ai>;
          size: number;
          top_items: number;
          child_items: number;
        },
        false
      >;

      /**
       * The limits that are imposed on automatic migrations.
       *
       * If set to None, then no automatic migration happens.
       */
      AutoLimits: StorageDescriptor<[], Anonymize<I215mkl885p4da> | undefined, false>;

      /**
       * The maximum limits that the signed migration could use.
       *
       * If not set, no signed submission is allowed.
       */
      SignedMigrationMaxLimits: StorageDescriptor<[], { size: number; item: number }, true>;
    },
    {
      /**
       *See [`Pallet::control_auto_migration`].
       */
      control_auto_migration: TxDescriptor<{ maybe_config: Anonymize<Ib17t3992hb64n> }>;

      /**
       *See [`Pallet::continue_migrate`].
       */
      continue_migrate: TxDescriptor<{
        limits: Anonymize<I215mkl885p4da>;
        real_size_upper: number;
        witness_task: Anonymize<I2sqgqj7ib3ine>;
      }>;

      /**
       *See [`Pallet::migrate_custom_top`].
       */
      migrate_custom_top: TxDescriptor<{ keys: Anonymize<Itom7fk49o0c9>; witness_size: number }>;

      /**
       *See [`Pallet::migrate_custom_child`].
       */
      migrate_custom_child: TxDescriptor<{
        root: Binary;
        child_keys: Anonymize<Itom7fk49o0c9>;
        total_size: number;
      }>;

      /**
       *See [`Pallet::set_signed_max_limits`].
       */
      set_signed_max_limits: TxDescriptor<{ limits: Anonymize<I215mkl885p4da> }>;

      /**
       *See [`Pallet::force_set_progress`].
       */
      force_set_progress: TxDescriptor<{
        progress_top: Anonymize<I2jvfaenrpg1ai>;
        progress_child: Anonymize<I2jvfaenrpg1ai>;
      }>;
    },
    {
      /**
       *Given number of `(top, child)` keys were migrated respectively, with the given
       *`compute`.
       */
      Migrated: PlainDescriptor<{ top: number; child: number; compute: Anonymize<I2os86rbc98tr0> }>;

      /**
       *Some account got slashed by the given amount.
       */
      Slashed: PlainDescriptor<{ who: SS58String; amount: bigint }>;

      /**
       *The auto migration task finished.
       */
      AutoMigrationFinished: PlainDescriptor<undefined>;

      /**
       *Migration got halted due to an error or miss-configuration.
       */
      Halted: PlainDescriptor<{ error: Anonymize<Ihok5kol9m3tk> }>;
    },
    {
      /**
       *Max signed limits not respected.
       */
      MaxSignedLimits: PlainDescriptor<undefined>;

      /**
       *A key was longer than the configured maximum.
       *
       *This means that the migration halted at the current [`Progress`] and
       *can be resumed with a larger [`crate::Config::MaxKeyLen`] value.
       *Retrying with the same [`crate::Config::MaxKeyLen`] value will not work.
       *The value should only be increased to avoid a storage migration for the currently
       *stored [`crate::Progress::LastKey`].
       */
      KeyTooLong: PlainDescriptor<undefined>;

      /**
       *submitter does not have enough funds.
       */
      NotEnoughFunds: PlainDescriptor<undefined>;

      /**
       *Bad witness data provided.
       */
      BadWitness: PlainDescriptor<undefined>;

      /**
       *Signed migration is not allowed because the maximum limit is not set yet.
       */
      SignedMigrationNotAllowed: PlainDescriptor<undefined>;

      /**
       *Bad child root provided.
       */
      BadChildRoot: PlainDescriptor<undefined>;
    },
    {
      /**
       * Maximal number of bytes that a key can have.
       *
       * FRAME itself does not limit the key length.
       * The concrete value must therefore depend on your storage usage.
       * A [`frame_support::storage::StorageNMap`] for example can have an arbitrary number of
       * keys which are then hashed and concatenated, resulting in arbitrarily long keys.
       *
       * Use the *state migration RPC* to retrieve the length of the longest key in your
       * storage: <https://github.com/paritytech/substrate/issues/11642>
       *
       * The migration will halt with a `Halted` event if this value is too small.
       * Since there is no real penalty from over-estimating, it is advised to use a large
       * value. The default is 512 byte.
       *
       * Some key lengths for reference:
       * - [`frame_support::storage::StorageValue`]: 32 byte
       * - [`frame_support::storage::StorageMap`]: 64 byte
       * - [`frame_support::storage::StorageDoubleMap`]: 96 byte
       *
       * For more info see
       * <https://www.shawntabrizi.com/blog/substrate/querying-substrate-storage-via-rpc/>
       */
      MaxKeyLen: PlainDescriptor<number>;
    },
  ];
  RootTesting: [
    {},
    {
      /**
       *See `Pallet::fill_block`.
       */
      fill_block: TxDescriptor<{ ratio: number }>;

      /**
       *See `Pallet::trigger_defensive`.
       */
      trigger_defensive: TxDescriptor<undefined>;
    },
    {
      /**
       *Event dispatched when the trigger_defensive extrinsic is called.
       */
      DefensiveTestCall: PlainDescriptor<undefined>;
    },
    {},
    {},
  ];
  Sudo: [
    {
      /**
       * The `AccountId` of the sudo key.
       */
      Key: StorageDescriptor<[], SS58String, true>;
    },
    {
      /**
       *See [`Pallet::sudo`].
       */
      sudo: TxDescriptor<{ call: Anonymize<I5v7s5dm78b0mh> }>;

      /**
       *See [`Pallet::sudo_unchecked_weight`].
       */
      sudo_unchecked_weight: TxDescriptor<{
        call: Anonymize<I5v7s5dm78b0mh>;
        weight: Anonymize<I4q39t5hn830vp>;
      }>;

      /**
       *See [`Pallet::set_key`].
       */
      set_key: TxDescriptor<{ new: MultiAddress }>;

      /**
       *See [`Pallet::sudo_as`].
       */
      sudo_as: TxDescriptor<{ who: MultiAddress; call: Anonymize<I5v7s5dm78b0mh> }>;

      /**
       *See [`Pallet::remove_key`].
       */
      remove_key: TxDescriptor<undefined>;
    },
    {
      /**
       *A sudo call just took place.
       */
      Sudid: PlainDescriptor<{ sudo_result: Anonymize<Idtdr91jmq5g4i> }>;

      /**
       *The sudo key has been updated.
       */
      KeyChanged: PlainDescriptor<{ old: Anonymize<Ihfphjolmsqq1>; new: SS58String }>;

      /**
       *The key was permanently removed.
       */
      KeyRemoved: PlainDescriptor<undefined>;

      /**
       *A [sudo_as](Pallet::sudo_as) call just took place.
       */
      SudoAsDone: PlainDescriptor<{ sudo_result: Anonymize<Idtdr91jmq5g4i> }>;
    },
    {
      /**
       *Sender must be the Sudo account.
       */
      RequireSudo: PlainDescriptor<undefined>;
    },
    {},
  ];
};
export const pallets: IPallets = {
  System: [
    {
      Account: stg_System_Account,
      ExtrinsicCount: stg_System_ExtrinsicCount,
      BlockWeight: stg_System_BlockWeight,
      AllExtrinsicsLen: stg_System_AllExtrinsicsLen,
      BlockHash: stg_System_BlockHash,
      ExtrinsicData: stg_System_ExtrinsicData,
      Number: stg_System_Number,
      ParentHash: stg_System_ParentHash,
      Digest: stg_System_Digest,
      Events: stg_System_Events,
      EventCount: stg_System_EventCount,
      EventTopics: stg_System_EventTopics,
      LastRuntimeUpgrade: stg_System_LastRuntimeUpgrade,
      UpgradedToU32RefCount: stg_System_UpgradedToU32RefCount,
      UpgradedToTripleRefCount: stg_System_UpgradedToTripleRefCount,
      ExecutionPhase: stg_System_ExecutionPhase,
      AuthorizedUpgrade: stg_System_AuthorizedUpgrade,
    },
    {
      remark: call_System_remark,
      set_heap_pages: call_System_set_heap_pages,
      set_code: call_System_set_code,
      set_code_without_checks: call_System_set_code_without_checks,
      set_storage: call_System_set_storage,
      kill_storage: call_System_kill_storage,
      kill_prefix: call_System_kill_prefix,
      remark_with_event: call_System_remark_with_event,
      authorize_upgrade: call_System_authorize_upgrade,
      authorize_upgrade_without_checks: call_System_authorize_upgrade_without_checks,
      apply_authorized_upgrade: call_System_apply_authorized_upgrade,
    },
    {
      ExtrinsicSuccess: evt_System_ExtrinsicSuccess,
      ExtrinsicFailed: evt_System_ExtrinsicFailed,
      CodeUpdated: evt_System_CodeUpdated,
      NewAccount: evt_System_NewAccount,
      KilledAccount: evt_System_KilledAccount,
      Remarked: evt_System_Remarked,
      UpgradeAuthorized: evt_System_UpgradeAuthorized,
    },
    {
      InvalidSpecName: err_System_InvalidSpecName,
      SpecVersionNeedsToIncrease: err_System_SpecVersionNeedsToIncrease,
      FailedToExtractRuntimeVersion: err_System_FailedToExtractRuntimeVersion,
      NonDefaultComposite: err_System_NonDefaultComposite,
      NonZeroRefCount: err_System_NonZeroRefCount,
      CallFiltered: err_System_CallFiltered,
      NothingAuthorized: err_System_NothingAuthorized,
      Unauthorized: err_System_Unauthorized,
    },
    {
      BlockWeights: const_System_BlockWeights,
      BlockLength: const_System_BlockLength,
      BlockHashCount: const_System_BlockHashCount,
      DbWeight: const_System_DbWeight,
      Version: const_System_Version,
      SS58Prefix: const_System_SS58Prefix,
    },
  ],
  Babe: [
    {
      EpochIndex: stg_Babe_EpochIndex,
      Authorities: stg_Babe_Authorities,
      GenesisSlot: stg_Babe_GenesisSlot,
      CurrentSlot: stg_Babe_CurrentSlot,
      Randomness: stg_Babe_Randomness,
      PendingEpochConfigChange: stg_Babe_PendingEpochConfigChange,
      NextRandomness: stg_Babe_NextRandomness,
      NextAuthorities: stg_Babe_NextAuthorities,
      SegmentIndex: stg_Babe_SegmentIndex,
      UnderConstruction: stg_Babe_UnderConstruction,
      Initialized: stg_Babe_Initialized,
      AuthorVrfRandomness: stg_Babe_AuthorVrfRandomness,
      EpochStart: stg_Babe_EpochStart,
      Lateness: stg_Babe_Lateness,
      EpochConfig: stg_Babe_EpochConfig,
      NextEpochConfig: stg_Babe_NextEpochConfig,
      SkippedEpochs: stg_Babe_SkippedEpochs,
    },
    {
      report_equivocation: call_Babe_report_equivocation,
      report_equivocation_unsigned: call_Babe_report_equivocation_unsigned,
      plan_config_change: call_Babe_plan_config_change,
    },
    {},
    {
      InvalidEquivocationProof: err_Babe_InvalidEquivocationProof,
      InvalidKeyOwnershipProof: err_Babe_InvalidKeyOwnershipProof,
      DuplicateOffenceReport: err_Babe_DuplicateOffenceReport,
      InvalidConfiguration: err_Babe_InvalidConfiguration,
    },
    {
      EpochDuration: const_Babe_EpochDuration,
      ExpectedBlockTime: const_Babe_ExpectedBlockTime,
      MaxAuthorities: const_Babe_MaxAuthorities,
      MaxNominators: const_Babe_MaxNominators,
    },
  ],
  Timestamp: [
    { Now: stg_Timestamp_Now, DidUpdate: stg_Timestamp_DidUpdate },
    { set: call_Timestamp_set },
    {},
    {},
    { MinimumPeriod: const_Timestamp_MinimumPeriod },
  ],
  Indices: [
    { Accounts: stg_Indices_Accounts },
    {
      claim: call_Indices_claim,
      transfer: call_Indices_transfer,
      free: call_Indices_free,
      force_transfer: call_Indices_force_transfer,
      freeze: call_Indices_freeze,
    },
    {
      IndexAssigned: evt_Indices_IndexAssigned,
      IndexFreed: evt_Indices_IndexFreed,
      IndexFrozen: evt_Indices_IndexFrozen,
    },
    {
      NotAssigned: err_Indices_NotAssigned,
      NotOwner: err_Indices_NotOwner,
      InUse: err_Indices_InUse,
      NotTransfer: err_Indices_NotTransfer,
      Permanent: err_Indices_Permanent,
    },
    { Deposit: const_Indices_Deposit },
  ],
  Balances: [
    {
      TotalIssuance: stg_Balances_TotalIssuance,
      InactiveIssuance: stg_Balances_InactiveIssuance,
      Account: stg_Balances_Account,
      Locks: stg_Balances_Locks,
      Reserves: stg_Balances_Reserves,
      Holds: stg_Balances_Holds,
      Freezes: stg_Balances_Freezes,
    },
    {
      transfer_allow_death: call_Balances_transfer_allow_death,
      force_transfer: call_Balances_force_transfer,
      transfer_keep_alive: call_Balances_transfer_keep_alive,
      transfer_all: call_Balances_transfer_all,
      force_unreserve: call_Balances_force_unreserve,
      upgrade_accounts: call_Balances_upgrade_accounts,
      force_set_balance: call_Balances_force_set_balance,
      force_adjust_total_issuance: call_Balances_force_adjust_total_issuance,
    },
    {
      Endowed: evt_Balances_Endowed,
      DustLost: evt_Balances_DustLost,
      Transfer: evt_Balances_Transfer,
      BalanceSet: evt_Balances_BalanceSet,
      Reserved: evt_Balances_Reserved,
      Unreserved: evt_Balances_Unreserved,
      ReserveRepatriated: evt_Balances_ReserveRepatriated,
      Deposit: evt_Balances_Deposit,
      Withdraw: evt_Balances_Withdraw,
      Slashed: evt_Balances_Slashed,
      Minted: evt_Balances_Minted,
      Burned: evt_Balances_Burned,
      Suspended: evt_Balances_Suspended,
      Restored: evt_Balances_Restored,
      Upgraded: evt_Balances_Upgraded,
      Issued: evt_Balances_Issued,
      Rescinded: evt_Balances_Rescinded,
      Locked: evt_Balances_Locked,
      Unlocked: evt_Balances_Unlocked,
      Frozen: evt_Balances_Frozen,
      Thawed: evt_Balances_Thawed,
      TotalIssuanceForced: evt_Balances_TotalIssuanceForced,
    },
    {
      VestingBalance: err_Balances_VestingBalance,
      LiquidityRestrictions: err_Balances_LiquidityRestrictions,
      InsufficientBalance: err_Balances_InsufficientBalance,
      ExistentialDeposit: err_Balances_ExistentialDeposit,
      Expendability: err_Balances_Expendability,
      ExistingVestingSchedule: err_Balances_ExistingVestingSchedule,
      DeadAccount: err_Balances_DeadAccount,
      TooManyReserves: err_Balances_TooManyReserves,
      TooManyHolds: err_Balances_TooManyHolds,
      TooManyFreezes: err_Balances_TooManyFreezes,
      IssuanceDeactivated: err_Balances_IssuanceDeactivated,
      DeltaZero: err_Balances_DeltaZero,
    },
    {
      ExistentialDeposit: const_Balances_ExistentialDeposit,
      MaxLocks: const_Balances_MaxLocks,
      MaxReserves: const_Balances_MaxReserves,
      MaxFreezes: const_Balances_MaxFreezes,
    },
  ],
  TransactionPayment: [
    {
      NextFeeMultiplier: stg_TransactionPayment_NextFeeMultiplier,
      StorageVersion: stg_TransactionPayment_StorageVersion,
    },
    {},
    { TransactionFeePaid: evt_TransactionPayment_TransactionFeePaid },
    {},
    { OperationalFeeMultiplier: const_TransactionPayment_OperationalFeeMultiplier },
  ],
  Authorship: [{ Author: stg_Authorship_Author }, {}, {}, {}, {}],
  Offences: [
    { Reports: stg_Offences_Reports, ConcurrentReportsIndex: stg_Offences_ConcurrentReportsIndex },
    {},
    { Offence: evt_Offences_Offence },
    {},
    {},
  ],
  Historical: [
    {
      HistoricalSessions: stg_Historical_HistoricalSessions,
      StoredRange: stg_Historical_StoredRange,
    },
    {},
    {},
    {},
    {},
  ],
  Session: [
    {
      Validators: stg_Session_Validators,
      CurrentIndex: stg_Session_CurrentIndex,
      QueuedChanged: stg_Session_QueuedChanged,
      QueuedKeys: stg_Session_QueuedKeys,
      DisabledValidators: stg_Session_DisabledValidators,
      NextKeys: stg_Session_NextKeys,
      KeyOwner: stg_Session_KeyOwner,
    },
    { set_keys: call_Session_set_keys, purge_keys: call_Session_purge_keys },
    { NewSession: evt_Session_NewSession },
    {
      InvalidProof: err_Session_InvalidProof,
      NoAssociatedValidatorId: err_Session_NoAssociatedValidatorId,
      DuplicatedKey: err_Session_DuplicatedKey,
      NoKeys: err_Session_NoKeys,
      NoAccount: err_Session_NoAccount,
    },
    {},
  ],
  Grandpa: [
    {
      State: stg_Grandpa_State,
      PendingChange: stg_Grandpa_PendingChange,
      NextForced: stg_Grandpa_NextForced,
      Stalled: stg_Grandpa_Stalled,
      CurrentSetId: stg_Grandpa_CurrentSetId,
      SetIdSession: stg_Grandpa_SetIdSession,
      Authorities: stg_Grandpa_Authorities,
    },
    {
      report_equivocation: call_Grandpa_report_equivocation,
      report_equivocation_unsigned: call_Grandpa_report_equivocation_unsigned,
      note_stalled: call_Grandpa_note_stalled,
    },
    {
      NewAuthorities: evt_Grandpa_NewAuthorities,
      Paused: evt_Grandpa_Paused,
      Resumed: evt_Grandpa_Resumed,
    },
    {
      PauseFailed: err_Grandpa_PauseFailed,
      ResumeFailed: err_Grandpa_ResumeFailed,
      ChangePending: err_Grandpa_ChangePending,
      TooSoon: err_Grandpa_TooSoon,
      InvalidKeyOwnershipProof: err_Grandpa_InvalidKeyOwnershipProof,
      InvalidEquivocationProof: err_Grandpa_InvalidEquivocationProof,
      DuplicateOffenceReport: err_Grandpa_DuplicateOffenceReport,
    },
    {
      MaxAuthorities: const_Grandpa_MaxAuthorities,
      MaxNominators: const_Grandpa_MaxNominators,
      MaxSetIdSessionEntries: const_Grandpa_MaxSetIdSessionEntries,
    },
  ],
  AuthorityDiscovery: [
    { Keys: stg_AuthorityDiscovery_Keys, NextKeys: stg_AuthorityDiscovery_NextKeys },
    {},
    {},
    {},
    {},
  ],
  Treasury: [
    {
      ProposalCount: stg_Treasury_ProposalCount,
      Proposals: stg_Treasury_Proposals,
      Deactivated: stg_Treasury_Deactivated,
      Approvals: stg_Treasury_Approvals,
      SpendCount: stg_Treasury_SpendCount,
      Spends: stg_Treasury_Spends,
    },
    {
      propose_spend: call_Treasury_propose_spend,
      reject_proposal: call_Treasury_reject_proposal,
      approve_proposal: call_Treasury_approve_proposal,
      spend_local: call_Treasury_spend_local,
      remove_approval: call_Treasury_remove_approval,
      spend: call_Treasury_spend,
      payout: call_Treasury_payout,
      check_status: call_Treasury_check_status,
      void_spend: call_Treasury_void_spend,
    },
    {
      Proposed: evt_Treasury_Proposed,
      Spending: evt_Treasury_Spending,
      Awarded: evt_Treasury_Awarded,
      Rejected: evt_Treasury_Rejected,
      Burnt: evt_Treasury_Burnt,
      Rollover: evt_Treasury_Rollover,
      Deposit: evt_Treasury_Deposit,
      SpendApproved: evt_Treasury_SpendApproved,
      UpdatedInactive: evt_Treasury_UpdatedInactive,
      AssetSpendApproved: evt_Treasury_AssetSpendApproved,
      AssetSpendVoided: evt_Treasury_AssetSpendVoided,
      Paid: evt_Treasury_Paid,
      PaymentFailed: evt_Treasury_PaymentFailed,
      SpendProcessed: evt_Treasury_SpendProcessed,
    },
    {
      InsufficientProposersBalance: err_Treasury_InsufficientProposersBalance,
      InvalidIndex: err_Treasury_InvalidIndex,
      TooManyApprovals: err_Treasury_TooManyApprovals,
      InsufficientPermission: err_Treasury_InsufficientPermission,
      ProposalNotApproved: err_Treasury_ProposalNotApproved,
      FailedToConvertBalance: err_Treasury_FailedToConvertBalance,
      SpendExpired: err_Treasury_SpendExpired,
      EarlyPayout: err_Treasury_EarlyPayout,
      AlreadyAttempted: err_Treasury_AlreadyAttempted,
      PayoutError: err_Treasury_PayoutError,
      NotAttempted: err_Treasury_NotAttempted,
      Inconclusive: err_Treasury_Inconclusive,
    },
    {
      ProposalBond: const_Treasury_ProposalBond,
      ProposalBondMinimum: const_Treasury_ProposalBondMinimum,
      ProposalBondMaximum: const_Treasury_ProposalBondMaximum,
      SpendPeriod: const_Treasury_SpendPeriod,
      Burn: const_Treasury_Burn,
      PalletId: const_Treasury_PalletId,
      MaxApprovals: const_Treasury_MaxApprovals,
      PayoutPeriod: const_Treasury_PayoutPeriod,
    },
  ],
  ConvictionVoting: [
    {
      VotingFor: stg_ConvictionVoting_VotingFor,
      ClassLocksFor: stg_ConvictionVoting_ClassLocksFor,
    },
    {
      vote: call_ConvictionVoting_vote,
      delegate: call_ConvictionVoting_delegate,
      undelegate: call_ConvictionVoting_undelegate,
      unlock: call_ConvictionVoting_unlock,
      remove_vote: call_ConvictionVoting_remove_vote,
      remove_other_vote: call_ConvictionVoting_remove_other_vote,
    },
    { Delegated: evt_ConvictionVoting_Delegated, Undelegated: evt_ConvictionVoting_Undelegated },
    {
      NotOngoing: err_ConvictionVoting_NotOngoing,
      NotVoter: err_ConvictionVoting_NotVoter,
      NoPermission: err_ConvictionVoting_NoPermission,
      NoPermissionYet: err_ConvictionVoting_NoPermissionYet,
      AlreadyDelegating: err_ConvictionVoting_AlreadyDelegating,
      AlreadyVoting: err_ConvictionVoting_AlreadyVoting,
      InsufficientFunds: err_ConvictionVoting_InsufficientFunds,
      NotDelegating: err_ConvictionVoting_NotDelegating,
      Nonsense: err_ConvictionVoting_Nonsense,
      MaxVotesReached: err_ConvictionVoting_MaxVotesReached,
      ClassNeeded: err_ConvictionVoting_ClassNeeded,
      BadClass: err_ConvictionVoting_BadClass,
    },
    {
      MaxVotes: const_ConvictionVoting_MaxVotes,
      VoteLockingPeriod: const_ConvictionVoting_VoteLockingPeriod,
    },
  ],
  Referenda: [
    {
      ReferendumCount: stg_Referenda_ReferendumCount,
      ReferendumInfoFor: stg_Referenda_ReferendumInfoFor,
      TrackQueue: stg_Referenda_TrackQueue,
      DecidingCount: stg_Referenda_DecidingCount,
      MetadataOf: stg_Referenda_MetadataOf,
    },
    {
      submit: call_Referenda_submit,
      place_decision_deposit: call_Referenda_place_decision_deposit,
      refund_decision_deposit: call_Referenda_refund_decision_deposit,
      cancel: call_Referenda_cancel,
      kill: call_Referenda_kill,
      nudge_referendum: call_Referenda_nudge_referendum,
      one_fewer_deciding: call_Referenda_one_fewer_deciding,
      refund_submission_deposit: call_Referenda_refund_submission_deposit,
      set_metadata: call_Referenda_set_metadata,
    },
    {
      Submitted: evt_Referenda_Submitted,
      DecisionDepositPlaced: evt_Referenda_DecisionDepositPlaced,
      DecisionDepositRefunded: evt_Referenda_DecisionDepositRefunded,
      DepositSlashed: evt_Referenda_DepositSlashed,
      DecisionStarted: evt_Referenda_DecisionStarted,
      ConfirmStarted: evt_Referenda_ConfirmStarted,
      ConfirmAborted: evt_Referenda_ConfirmAborted,
      Confirmed: evt_Referenda_Confirmed,
      Approved: evt_Referenda_Approved,
      Rejected: evt_Referenda_Rejected,
      TimedOut: evt_Referenda_TimedOut,
      Cancelled: evt_Referenda_Cancelled,
      Killed: evt_Referenda_Killed,
      SubmissionDepositRefunded: evt_Referenda_SubmissionDepositRefunded,
      MetadataSet: evt_Referenda_MetadataSet,
      MetadataCleared: evt_Referenda_MetadataCleared,
    },
    {
      NotOngoing: err_Referenda_NotOngoing,
      HasDeposit: err_Referenda_HasDeposit,
      BadTrack: err_Referenda_BadTrack,
      Full: err_Referenda_Full,
      QueueEmpty: err_Referenda_QueueEmpty,
      BadReferendum: err_Referenda_BadReferendum,
      NothingToDo: err_Referenda_NothingToDo,
      NoTrack: err_Referenda_NoTrack,
      Unfinished: err_Referenda_Unfinished,
      NoPermission: err_Referenda_NoPermission,
      NoDeposit: err_Referenda_NoDeposit,
      BadStatus: err_Referenda_BadStatus,
      PreimageNotExist: err_Referenda_PreimageNotExist,
    },
    {
      SubmissionDeposit: const_Referenda_SubmissionDeposit,
      MaxQueued: const_Referenda_MaxQueued,
      UndecidingTimeout: const_Referenda_UndecidingTimeout,
      AlarmInterval: const_Referenda_AlarmInterval,
      Tracks: const_Referenda_Tracks,
    },
  ],
  FellowshipCollective: [
    {
      MemberCount: stg_FellowshipCollective_MemberCount,
      Members: stg_FellowshipCollective_Members,
      IdToIndex: stg_FellowshipCollective_IdToIndex,
      IndexToId: stg_FellowshipCollective_IndexToId,
      Voting: stg_FellowshipCollective_Voting,
      VotingCleanup: stg_FellowshipCollective_VotingCleanup,
    },
    {
      add_member: call_FellowshipCollective_add_member,
      promote_member: call_FellowshipCollective_promote_member,
      demote_member: call_FellowshipCollective_demote_member,
      remove_member: call_FellowshipCollective_remove_member,
      vote: call_FellowshipCollective_vote,
      cleanup_poll: call_FellowshipCollective_cleanup_poll,
      exchange_member: call_FellowshipCollective_exchange_member,
    },
    {
      MemberAdded: evt_FellowshipCollective_MemberAdded,
      RankChanged: evt_FellowshipCollective_RankChanged,
      MemberRemoved: evt_FellowshipCollective_MemberRemoved,
      Voted: evt_FellowshipCollective_Voted,
      MemberExchanged: evt_FellowshipCollective_MemberExchanged,
    },
    {
      AlreadyMember: err_FellowshipCollective_AlreadyMember,
      NotMember: err_FellowshipCollective_NotMember,
      NotPolling: err_FellowshipCollective_NotPolling,
      Ongoing: err_FellowshipCollective_Ongoing,
      NoneRemaining: err_FellowshipCollective_NoneRemaining,
      Corruption: err_FellowshipCollective_Corruption,
      RankTooLow: err_FellowshipCollective_RankTooLow,
      InvalidWitness: err_FellowshipCollective_InvalidWitness,
      NoPermission: err_FellowshipCollective_NoPermission,
      SameMember: err_FellowshipCollective_SameMember,
    },
    {},
  ],
  FellowshipReferenda: [
    {
      ReferendumCount: stg_FellowshipReferenda_ReferendumCount,
      ReferendumInfoFor: stg_FellowshipReferenda_ReferendumInfoFor,
      TrackQueue: stg_FellowshipReferenda_TrackQueue,
      DecidingCount: stg_FellowshipReferenda_DecidingCount,
      MetadataOf: stg_FellowshipReferenda_MetadataOf,
    },
    {
      submit: call_FellowshipReferenda_submit,
      place_decision_deposit: call_FellowshipReferenda_place_decision_deposit,
      refund_decision_deposit: call_FellowshipReferenda_refund_decision_deposit,
      cancel: call_FellowshipReferenda_cancel,
      kill: call_FellowshipReferenda_kill,
      nudge_referendum: call_FellowshipReferenda_nudge_referendum,
      one_fewer_deciding: call_FellowshipReferenda_one_fewer_deciding,
      refund_submission_deposit: call_FellowshipReferenda_refund_submission_deposit,
      set_metadata: call_FellowshipReferenda_set_metadata,
    },
    {
      Submitted: evt_FellowshipReferenda_Submitted,
      DecisionDepositPlaced: evt_FellowshipReferenda_DecisionDepositPlaced,
      DecisionDepositRefunded: evt_FellowshipReferenda_DecisionDepositRefunded,
      DepositSlashed: evt_FellowshipReferenda_DepositSlashed,
      DecisionStarted: evt_FellowshipReferenda_DecisionStarted,
      ConfirmStarted: evt_FellowshipReferenda_ConfirmStarted,
      ConfirmAborted: evt_FellowshipReferenda_ConfirmAborted,
      Confirmed: evt_FellowshipReferenda_Confirmed,
      Approved: evt_FellowshipReferenda_Approved,
      Rejected: evt_FellowshipReferenda_Rejected,
      TimedOut: evt_FellowshipReferenda_TimedOut,
      Cancelled: evt_FellowshipReferenda_Cancelled,
      Killed: evt_FellowshipReferenda_Killed,
      SubmissionDepositRefunded: evt_FellowshipReferenda_SubmissionDepositRefunded,
      MetadataSet: evt_FellowshipReferenda_MetadataSet,
      MetadataCleared: evt_FellowshipReferenda_MetadataCleared,
    },
    {
      NotOngoing: err_FellowshipReferenda_NotOngoing,
      HasDeposit: err_FellowshipReferenda_HasDeposit,
      BadTrack: err_FellowshipReferenda_BadTrack,
      Full: err_FellowshipReferenda_Full,
      QueueEmpty: err_FellowshipReferenda_QueueEmpty,
      BadReferendum: err_FellowshipReferenda_BadReferendum,
      NothingToDo: err_FellowshipReferenda_NothingToDo,
      NoTrack: err_FellowshipReferenda_NoTrack,
      Unfinished: err_FellowshipReferenda_Unfinished,
      NoPermission: err_FellowshipReferenda_NoPermission,
      NoDeposit: err_FellowshipReferenda_NoDeposit,
      BadStatus: err_FellowshipReferenda_BadStatus,
      PreimageNotExist: err_FellowshipReferenda_PreimageNotExist,
    },
    {
      SubmissionDeposit: const_FellowshipReferenda_SubmissionDeposit,
      MaxQueued: const_FellowshipReferenda_MaxQueued,
      UndecidingTimeout: const_FellowshipReferenda_UndecidingTimeout,
      AlarmInterval: const_FellowshipReferenda_AlarmInterval,
      Tracks: const_FellowshipReferenda_Tracks,
    },
  ],
  Origins: [{}, {}, {}, {}, {}],
  Whitelist: [
    { WhitelistedCall: stg_Whitelist_WhitelistedCall },
    {
      whitelist_call: call_Whitelist_whitelist_call,
      remove_whitelisted_call: call_Whitelist_remove_whitelisted_call,
      dispatch_whitelisted_call: call_Whitelist_dispatch_whitelisted_call,
      dispatch_whitelisted_call_with_preimage:
        call_Whitelist_dispatch_whitelisted_call_with_preimage,
    },
    {
      CallWhitelisted: evt_Whitelist_CallWhitelisted,
      WhitelistedCallRemoved: evt_Whitelist_WhitelistedCallRemoved,
      WhitelistedCallDispatched: evt_Whitelist_WhitelistedCallDispatched,
    },
    {
      UnavailablePreImage: err_Whitelist_UnavailablePreImage,
      UndecodableCall: err_Whitelist_UndecodableCall,
      InvalidCallWeightWitness: err_Whitelist_InvalidCallWeightWitness,
      CallIsNotWhitelisted: err_Whitelist_CallIsNotWhitelisted,
      CallAlreadyWhitelisted: err_Whitelist_CallAlreadyWhitelisted,
    },
    {},
  ],
  Claims: [
    {
      Claims: stg_Claims_Claims,
      Total: stg_Claims_Total,
      Vesting: stg_Claims_Vesting,
      Signing: stg_Claims_Signing,
      Preclaims: stg_Claims_Preclaims,
    },
    {
      claim: call_Claims_claim,
      mint_claim: call_Claims_mint_claim,
      claim_attest: call_Claims_claim_attest,
      attest: call_Claims_attest,
      move_claim: call_Claims_move_claim,
    },
    { Claimed: evt_Claims_Claimed },
    {
      InvalidEthereumSignature: err_Claims_InvalidEthereumSignature,
      SignerHasNoClaim: err_Claims_SignerHasNoClaim,
      SenderHasNoClaim: err_Claims_SenderHasNoClaim,
      PotUnderflow: err_Claims_PotUnderflow,
      InvalidStatement: err_Claims_InvalidStatement,
      VestedBalanceExists: err_Claims_VestedBalanceExists,
    },
    { Prefix: const_Claims_Prefix },
  ],
  Utility: [
    {},
    {
      batch: call_Utility_batch,
      as_derivative: call_Utility_as_derivative,
      batch_all: call_Utility_batch_all,
      dispatch_as: call_Utility_dispatch_as,
      force_batch: call_Utility_force_batch,
      with_weight: call_Utility_with_weight,
    },
    {
      BatchInterrupted: evt_Utility_BatchInterrupted,
      BatchCompleted: evt_Utility_BatchCompleted,
      BatchCompletedWithErrors: evt_Utility_BatchCompletedWithErrors,
      ItemCompleted: evt_Utility_ItemCompleted,
      ItemFailed: evt_Utility_ItemFailed,
      DispatchedAs: evt_Utility_DispatchedAs,
    },
    { TooManyCalls: err_Utility_TooManyCalls },
    { batched_calls_limit: const_Utility_batched_calls_limit },
  ],
  Identity: [
    {
      IdentityOf: stg_Identity_IdentityOf,
      SuperOf: stg_Identity_SuperOf,
      SubsOf: stg_Identity_SubsOf,
      Registrars: stg_Identity_Registrars,
      UsernameAuthorities: stg_Identity_UsernameAuthorities,
      AccountOfUsername: stg_Identity_AccountOfUsername,
      PendingUsernames: stg_Identity_PendingUsernames,
    },
    {
      add_registrar: call_Identity_add_registrar,
      set_identity: call_Identity_set_identity,
      set_subs: call_Identity_set_subs,
      clear_identity: call_Identity_clear_identity,
      request_judgement: call_Identity_request_judgement,
      cancel_request: call_Identity_cancel_request,
      set_fee: call_Identity_set_fee,
      set_account_id: call_Identity_set_account_id,
      set_fields: call_Identity_set_fields,
      provide_judgement: call_Identity_provide_judgement,
      kill_identity: call_Identity_kill_identity,
      add_sub: call_Identity_add_sub,
      rename_sub: call_Identity_rename_sub,
      remove_sub: call_Identity_remove_sub,
      quit_sub: call_Identity_quit_sub,
      add_username_authority: call_Identity_add_username_authority,
      remove_username_authority: call_Identity_remove_username_authority,
      set_username_for: call_Identity_set_username_for,
      accept_username: call_Identity_accept_username,
      remove_expired_approval: call_Identity_remove_expired_approval,
      set_primary_username: call_Identity_set_primary_username,
      remove_dangling_username: call_Identity_remove_dangling_username,
    },
    {
      IdentitySet: evt_Identity_IdentitySet,
      IdentityCleared: evt_Identity_IdentityCleared,
      IdentityKilled: evt_Identity_IdentityKilled,
      JudgementRequested: evt_Identity_JudgementRequested,
      JudgementUnrequested: evt_Identity_JudgementUnrequested,
      JudgementGiven: evt_Identity_JudgementGiven,
      RegistrarAdded: evt_Identity_RegistrarAdded,
      SubIdentityAdded: evt_Identity_SubIdentityAdded,
      SubIdentityRemoved: evt_Identity_SubIdentityRemoved,
      SubIdentityRevoked: evt_Identity_SubIdentityRevoked,
      AuthorityAdded: evt_Identity_AuthorityAdded,
      AuthorityRemoved: evt_Identity_AuthorityRemoved,
      UsernameSet: evt_Identity_UsernameSet,
      UsernameQueued: evt_Identity_UsernameQueued,
      PreapprovalExpired: evt_Identity_PreapprovalExpired,
      PrimaryUsernameSet: evt_Identity_PrimaryUsernameSet,
      DanglingUsernameRemoved: evt_Identity_DanglingUsernameRemoved,
    },
    {
      TooManySubAccounts: err_Identity_TooManySubAccounts,
      NotFound: err_Identity_NotFound,
      NotNamed: err_Identity_NotNamed,
      EmptyIndex: err_Identity_EmptyIndex,
      FeeChanged: err_Identity_FeeChanged,
      NoIdentity: err_Identity_NoIdentity,
      StickyJudgement: err_Identity_StickyJudgement,
      JudgementGiven: err_Identity_JudgementGiven,
      InvalidJudgement: err_Identity_InvalidJudgement,
      InvalidIndex: err_Identity_InvalidIndex,
      InvalidTarget: err_Identity_InvalidTarget,
      TooManyRegistrars: err_Identity_TooManyRegistrars,
      AlreadyClaimed: err_Identity_AlreadyClaimed,
      NotSub: err_Identity_NotSub,
      NotOwned: err_Identity_NotOwned,
      JudgementForDifferentIdentity: err_Identity_JudgementForDifferentIdentity,
      JudgementPaymentFailed: err_Identity_JudgementPaymentFailed,
      InvalidSuffix: err_Identity_InvalidSuffix,
      NotUsernameAuthority: err_Identity_NotUsernameAuthority,
      NoAllocation: err_Identity_NoAllocation,
      InvalidSignature: err_Identity_InvalidSignature,
      RequiresSignature: err_Identity_RequiresSignature,
      InvalidUsername: err_Identity_InvalidUsername,
      UsernameTaken: err_Identity_UsernameTaken,
      NoUsername: err_Identity_NoUsername,
      NotExpired: err_Identity_NotExpired,
    },
    {
      BasicDeposit: const_Identity_BasicDeposit,
      ByteDeposit: const_Identity_ByteDeposit,
      SubAccountDeposit: const_Identity_SubAccountDeposit,
      MaxSubAccounts: const_Identity_MaxSubAccounts,
      MaxRegistrars: const_Identity_MaxRegistrars,
      PendingUsernameExpiration: const_Identity_PendingUsernameExpiration,
      MaxSuffixLength: const_Identity_MaxSuffixLength,
      MaxUsernameLength: const_Identity_MaxUsernameLength,
    },
  ],
  Society: [
    {
      Parameters: stg_Society_Parameters,
      Pot: stg_Society_Pot,
      Founder: stg_Society_Founder,
      Head: stg_Society_Head,
      Rules: stg_Society_Rules,
      Members: stg_Society_Members,
      Payouts: stg_Society_Payouts,
      MemberCount: stg_Society_MemberCount,
      MemberByIndex: stg_Society_MemberByIndex,
      SuspendedMembers: stg_Society_SuspendedMembers,
      RoundCount: stg_Society_RoundCount,
      Bids: stg_Society_Bids,
      Candidates: stg_Society_Candidates,
      Skeptic: stg_Society_Skeptic,
      Votes: stg_Society_Votes,
      VoteClearCursor: stg_Society_VoteClearCursor,
      NextHead: stg_Society_NextHead,
      ChallengeRoundCount: stg_Society_ChallengeRoundCount,
      Defending: stg_Society_Defending,
      DefenderVotes: stg_Society_DefenderVotes,
    },
    {
      bid: call_Society_bid,
      unbid: call_Society_unbid,
      vouch: call_Society_vouch,
      unvouch: call_Society_unvouch,
      vote: call_Society_vote,
      defender_vote: call_Society_defender_vote,
      payout: call_Society_payout,
      waive_repay: call_Society_waive_repay,
      found_society: call_Society_found_society,
      dissolve: call_Society_dissolve,
      judge_suspended_member: call_Society_judge_suspended_member,
      set_parameters: call_Society_set_parameters,
      punish_skeptic: call_Society_punish_skeptic,
      claim_membership: call_Society_claim_membership,
      bestow_membership: call_Society_bestow_membership,
      kick_candidate: call_Society_kick_candidate,
      resign_candidacy: call_Society_resign_candidacy,
      drop_candidate: call_Society_drop_candidate,
      cleanup_candidacy: call_Society_cleanup_candidacy,
      cleanup_challenge: call_Society_cleanup_challenge,
    },
    {
      Founded: evt_Society_Founded,
      Bid: evt_Society_Bid,
      Vouch: evt_Society_Vouch,
      AutoUnbid: evt_Society_AutoUnbid,
      Unbid: evt_Society_Unbid,
      Unvouch: evt_Society_Unvouch,
      Inducted: evt_Society_Inducted,
      SuspendedMemberJudgement: evt_Society_SuspendedMemberJudgement,
      CandidateSuspended: evt_Society_CandidateSuspended,
      MemberSuspended: evt_Society_MemberSuspended,
      Challenged: evt_Society_Challenged,
      Vote: evt_Society_Vote,
      DefenderVote: evt_Society_DefenderVote,
      NewParams: evt_Society_NewParams,
      Unfounded: evt_Society_Unfounded,
      Deposit: evt_Society_Deposit,
      Elevated: evt_Society_Elevated,
    },
    {
      NotMember: err_Society_NotMember,
      AlreadyMember: err_Society_AlreadyMember,
      Suspended: err_Society_Suspended,
      NotSuspended: err_Society_NotSuspended,
      NoPayout: err_Society_NoPayout,
      AlreadyFounded: err_Society_AlreadyFounded,
      InsufficientPot: err_Society_InsufficientPot,
      AlreadyVouching: err_Society_AlreadyVouching,
      NotVouchingOnBidder: err_Society_NotVouchingOnBidder,
      Head: err_Society_Head,
      Founder: err_Society_Founder,
      AlreadyBid: err_Society_AlreadyBid,
      AlreadyCandidate: err_Society_AlreadyCandidate,
      NotCandidate: err_Society_NotCandidate,
      MaxMembers: err_Society_MaxMembers,
      NotFounder: err_Society_NotFounder,
      NotHead: err_Society_NotHead,
      NotApproved: err_Society_NotApproved,
      NotRejected: err_Society_NotRejected,
      Approved: err_Society_Approved,
      Rejected: err_Society_Rejected,
      InProgress: err_Society_InProgress,
      TooEarly: err_Society_TooEarly,
      Voted: err_Society_Voted,
      Expired: err_Society_Expired,
      NotBidder: err_Society_NotBidder,
      NoDefender: err_Society_NoDefender,
      NotGroup: err_Society_NotGroup,
      AlreadyElevated: err_Society_AlreadyElevated,
      AlreadyPunished: err_Society_AlreadyPunished,
      InsufficientFunds: err_Society_InsufficientFunds,
      NoVotes: err_Society_NoVotes,
    },
    {
      PalletId: const_Society_PalletId,
      GraceStrikes: const_Society_GraceStrikes,
      PeriodSpend: const_Society_PeriodSpend,
      VotingPeriod: const_Society_VotingPeriod,
      ClaimPeriod: const_Society_ClaimPeriod,
      MaxLockDuration: const_Society_MaxLockDuration,
      ChallengePeriod: const_Society_ChallengePeriod,
      MaxPayouts: const_Society_MaxPayouts,
      MaxBids: const_Society_MaxBids,
    },
  ],
  Recovery: [
    {
      Recoverable: stg_Recovery_Recoverable,
      ActiveRecoveries: stg_Recovery_ActiveRecoveries,
      Proxy: stg_Recovery_Proxy,
    },
    {
      as_recovered: call_Recovery_as_recovered,
      set_recovered: call_Recovery_set_recovered,
      create_recovery: call_Recovery_create_recovery,
      initiate_recovery: call_Recovery_initiate_recovery,
      vouch_recovery: call_Recovery_vouch_recovery,
      claim_recovery: call_Recovery_claim_recovery,
      close_recovery: call_Recovery_close_recovery,
      remove_recovery: call_Recovery_remove_recovery,
      cancel_recovered: call_Recovery_cancel_recovered,
    },
    {
      RecoveryCreated: evt_Recovery_RecoveryCreated,
      RecoveryInitiated: evt_Recovery_RecoveryInitiated,
      RecoveryVouched: evt_Recovery_RecoveryVouched,
      RecoveryClosed: evt_Recovery_RecoveryClosed,
      AccountRecovered: evt_Recovery_AccountRecovered,
      RecoveryRemoved: evt_Recovery_RecoveryRemoved,
    },
    {
      NotAllowed: err_Recovery_NotAllowed,
      ZeroThreshold: err_Recovery_ZeroThreshold,
      NotEnoughFriends: err_Recovery_NotEnoughFriends,
      MaxFriends: err_Recovery_MaxFriends,
      NotSorted: err_Recovery_NotSorted,
      NotRecoverable: err_Recovery_NotRecoverable,
      AlreadyRecoverable: err_Recovery_AlreadyRecoverable,
      AlreadyStarted: err_Recovery_AlreadyStarted,
      NotStarted: err_Recovery_NotStarted,
      NotFriend: err_Recovery_NotFriend,
      DelayPeriod: err_Recovery_DelayPeriod,
      AlreadyVouched: err_Recovery_AlreadyVouched,
      Threshold: err_Recovery_Threshold,
      StillActive: err_Recovery_StillActive,
      AlreadyProxy: err_Recovery_AlreadyProxy,
      BadState: err_Recovery_BadState,
    },
    {
      ConfigDepositBase: const_Recovery_ConfigDepositBase,
      FriendDepositFactor: const_Recovery_FriendDepositFactor,
      MaxFriends: const_Recovery_MaxFriends,
      RecoveryDeposit: const_Recovery_RecoveryDeposit,
    },
  ],
  Vesting: [
    { Vesting: stg_Vesting_Vesting, StorageVersion: stg_Vesting_StorageVersion },
    {
      vest: call_Vesting_vest,
      vest_other: call_Vesting_vest_other,
      vested_transfer: call_Vesting_vested_transfer,
      force_vested_transfer: call_Vesting_force_vested_transfer,
      merge_schedules: call_Vesting_merge_schedules,
      force_remove_vesting_schedule: call_Vesting_force_remove_vesting_schedule,
    },
    { VestingUpdated: evt_Vesting_VestingUpdated, VestingCompleted: evt_Vesting_VestingCompleted },
    {
      NotVesting: err_Vesting_NotVesting,
      AtMaxVestingSchedules: err_Vesting_AtMaxVestingSchedules,
      AmountLow: err_Vesting_AmountLow,
      ScheduleIndexOutOfBounds: err_Vesting_ScheduleIndexOutOfBounds,
      InvalidScheduleParams: err_Vesting_InvalidScheduleParams,
    },
    {
      MinVestedTransfer: const_Vesting_MinVestedTransfer,
      MaxVestingSchedules: const_Vesting_MaxVestingSchedules,
    },
  ],
  Scheduler: [
    {
      IncompleteSince: stg_Scheduler_IncompleteSince,
      Agenda: stg_Scheduler_Agenda,
      Retries: stg_Scheduler_Retries,
      Lookup: stg_Scheduler_Lookup,
    },
    {
      schedule: call_Scheduler_schedule,
      cancel: call_Scheduler_cancel,
      schedule_named: call_Scheduler_schedule_named,
      cancel_named: call_Scheduler_cancel_named,
      schedule_after: call_Scheduler_schedule_after,
      schedule_named_after: call_Scheduler_schedule_named_after,
      set_retry: call_Scheduler_set_retry,
      set_retry_named: call_Scheduler_set_retry_named,
      cancel_retry: call_Scheduler_cancel_retry,
      cancel_retry_named: call_Scheduler_cancel_retry_named,
    },
    {
      Scheduled: evt_Scheduler_Scheduled,
      Canceled: evt_Scheduler_Canceled,
      Dispatched: evt_Scheduler_Dispatched,
      RetrySet: evt_Scheduler_RetrySet,
      RetryCancelled: evt_Scheduler_RetryCancelled,
      CallUnavailable: evt_Scheduler_CallUnavailable,
      PeriodicFailed: evt_Scheduler_PeriodicFailed,
      RetryFailed: evt_Scheduler_RetryFailed,
      PermanentlyOverweight: evt_Scheduler_PermanentlyOverweight,
    },
    {
      FailedToSchedule: err_Scheduler_FailedToSchedule,
      NotFound: err_Scheduler_NotFound,
      TargetBlockNumberInPast: err_Scheduler_TargetBlockNumberInPast,
      RescheduleNoChange: err_Scheduler_RescheduleNoChange,
      Named: err_Scheduler_Named,
    },
    {
      MaximumWeight: const_Scheduler_MaximumWeight,
      MaxScheduledPerBlock: const_Scheduler_MaxScheduledPerBlock,
    },
  ],
  Proxy: [
    { Proxies: stg_Proxy_Proxies, Announcements: stg_Proxy_Announcements },
    {
      proxy: call_Proxy_proxy,
      add_proxy: call_Proxy_add_proxy,
      remove_proxy: call_Proxy_remove_proxy,
      remove_proxies: call_Proxy_remove_proxies,
      create_pure: call_Proxy_create_pure,
      kill_pure: call_Proxy_kill_pure,
      announce: call_Proxy_announce,
      remove_announcement: call_Proxy_remove_announcement,
      reject_announcement: call_Proxy_reject_announcement,
      proxy_announced: call_Proxy_proxy_announced,
    },
    {
      ProxyExecuted: evt_Proxy_ProxyExecuted,
      PureCreated: evt_Proxy_PureCreated,
      Announced: evt_Proxy_Announced,
      ProxyAdded: evt_Proxy_ProxyAdded,
      ProxyRemoved: evt_Proxy_ProxyRemoved,
    },
    {
      TooMany: err_Proxy_TooMany,
      NotFound: err_Proxy_NotFound,
      NotProxy: err_Proxy_NotProxy,
      Unproxyable: err_Proxy_Unproxyable,
      Duplicate: err_Proxy_Duplicate,
      NoPermission: err_Proxy_NoPermission,
      Unannounced: err_Proxy_Unannounced,
      NoSelfProxy: err_Proxy_NoSelfProxy,
    },
    {
      ProxyDepositBase: const_Proxy_ProxyDepositBase,
      ProxyDepositFactor: const_Proxy_ProxyDepositFactor,
      MaxProxies: const_Proxy_MaxProxies,
      MaxPending: const_Proxy_MaxPending,
      AnnouncementDepositBase: const_Proxy_AnnouncementDepositBase,
      AnnouncementDepositFactor: const_Proxy_AnnouncementDepositFactor,
    },
  ],
  Multisig: [
    { Multisigs: stg_Multisig_Multisigs },
    {
      as_multi_threshold_1: call_Multisig_as_multi_threshold_1,
      as_multi: call_Multisig_as_multi,
      approve_as_multi: call_Multisig_approve_as_multi,
      cancel_as_multi: call_Multisig_cancel_as_multi,
    },
    {
      NewMultisig: evt_Multisig_NewMultisig,
      MultisigApproval: evt_Multisig_MultisigApproval,
      MultisigExecuted: evt_Multisig_MultisigExecuted,
      MultisigCancelled: evt_Multisig_MultisigCancelled,
    },
    {
      MinimumThreshold: err_Multisig_MinimumThreshold,
      AlreadyApproved: err_Multisig_AlreadyApproved,
      NoApprovalsNeeded: err_Multisig_NoApprovalsNeeded,
      TooFewSignatories: err_Multisig_TooFewSignatories,
      TooManySignatories: err_Multisig_TooManySignatories,
      SignatoriesOutOfOrder: err_Multisig_SignatoriesOutOfOrder,
      SenderInSignatories: err_Multisig_SenderInSignatories,
      NotFound: err_Multisig_NotFound,
      NotOwner: err_Multisig_NotOwner,
      NoTimepoint: err_Multisig_NoTimepoint,
      WrongTimepoint: err_Multisig_WrongTimepoint,
      UnexpectedTimepoint: err_Multisig_UnexpectedTimepoint,
      MaxWeightTooLow: err_Multisig_MaxWeightTooLow,
      AlreadyStored: err_Multisig_AlreadyStored,
    },
    {
      DepositBase: const_Multisig_DepositBase,
      DepositFactor: const_Multisig_DepositFactor,
      MaxSignatories: const_Multisig_MaxSignatories,
    },
  ],
  Preimage: [
    {
      StatusFor: stg_Preimage_StatusFor,
      RequestStatusFor: stg_Preimage_RequestStatusFor,
      PreimageFor: stg_Preimage_PreimageFor,
    },
    {
      note_preimage: call_Preimage_note_preimage,
      unnote_preimage: call_Preimage_unnote_preimage,
      request_preimage: call_Preimage_request_preimage,
      unrequest_preimage: call_Preimage_unrequest_preimage,
      ensure_updated: call_Preimage_ensure_updated,
    },
    { Noted: evt_Preimage_Noted, Requested: evt_Preimage_Requested, Cleared: evt_Preimage_Cleared },
    {
      TooBig: err_Preimage_TooBig,
      AlreadyNoted: err_Preimage_AlreadyNoted,
      NotAuthorized: err_Preimage_NotAuthorized,
      NotNoted: err_Preimage_NotNoted,
      Requested: err_Preimage_Requested,
      NotRequested: err_Preimage_NotRequested,
      TooMany: err_Preimage_TooMany,
      TooFew: err_Preimage_TooFew,
    },
    {},
  ],
  AssetRate: [
    { ConversionRateToNative: stg_AssetRate_ConversionRateToNative },
    { create: call_AssetRate_create, update: call_AssetRate_update, remove: call_AssetRate_remove },
    {
      AssetRateCreated: evt_AssetRate_AssetRateCreated,
      AssetRateRemoved: evt_AssetRate_AssetRateRemoved,
      AssetRateUpdated: evt_AssetRate_AssetRateUpdated,
    },
    {
      UnknownAssetKind: err_AssetRate_UnknownAssetKind,
      AlreadyExists: err_AssetRate_AlreadyExists,
      Overflow: err_AssetRate_Overflow,
    },
    {},
  ],
  Bounties: [
    {
      BountyCount: stg_Bounties_BountyCount,
      Bounties: stg_Bounties_Bounties,
      BountyDescriptions: stg_Bounties_BountyDescriptions,
      BountyApprovals: stg_Bounties_BountyApprovals,
    },
    {
      propose_bounty: call_Bounties_propose_bounty,
      approve_bounty: call_Bounties_approve_bounty,
      propose_curator: call_Bounties_propose_curator,
      unassign_curator: call_Bounties_unassign_curator,
      accept_curator: call_Bounties_accept_curator,
      award_bounty: call_Bounties_award_bounty,
      claim_bounty: call_Bounties_claim_bounty,
      close_bounty: call_Bounties_close_bounty,
      extend_bounty_expiry: call_Bounties_extend_bounty_expiry,
    },
    {
      BountyProposed: evt_Bounties_BountyProposed,
      BountyRejected: evt_Bounties_BountyRejected,
      BountyBecameActive: evt_Bounties_BountyBecameActive,
      BountyAwarded: evt_Bounties_BountyAwarded,
      BountyClaimed: evt_Bounties_BountyClaimed,
      BountyCanceled: evt_Bounties_BountyCanceled,
      BountyExtended: evt_Bounties_BountyExtended,
      BountyApproved: evt_Bounties_BountyApproved,
      CuratorProposed: evt_Bounties_CuratorProposed,
      CuratorUnassigned: evt_Bounties_CuratorUnassigned,
      CuratorAccepted: evt_Bounties_CuratorAccepted,
    },
    {
      InsufficientProposersBalance: err_Bounties_InsufficientProposersBalance,
      InvalidIndex: err_Bounties_InvalidIndex,
      ReasonTooBig: err_Bounties_ReasonTooBig,
      UnexpectedStatus: err_Bounties_UnexpectedStatus,
      RequireCurator: err_Bounties_RequireCurator,
      InvalidValue: err_Bounties_InvalidValue,
      InvalidFee: err_Bounties_InvalidFee,
      PendingPayout: err_Bounties_PendingPayout,
      Premature: err_Bounties_Premature,
      HasActiveChildBounty: err_Bounties_HasActiveChildBounty,
      TooManyQueued: err_Bounties_TooManyQueued,
    },
    {
      BountyDepositBase: const_Bounties_BountyDepositBase,
      BountyDepositPayoutDelay: const_Bounties_BountyDepositPayoutDelay,
      BountyUpdatePeriod: const_Bounties_BountyUpdatePeriod,
      CuratorDepositMultiplier: const_Bounties_CuratorDepositMultiplier,
      CuratorDepositMax: const_Bounties_CuratorDepositMax,
      CuratorDepositMin: const_Bounties_CuratorDepositMin,
      BountyValueMinimum: const_Bounties_BountyValueMinimum,
      DataDepositPerByte: const_Bounties_DataDepositPerByte,
      MaximumReasonLength: const_Bounties_MaximumReasonLength,
    },
  ],
  ChildBounties: [
    {
      ChildBountyCount: stg_ChildBounties_ChildBountyCount,
      ParentChildBounties: stg_ChildBounties_ParentChildBounties,
      ChildBounties: stg_ChildBounties_ChildBounties,
      ChildBountyDescriptions: stg_ChildBounties_ChildBountyDescriptions,
      ChildrenCuratorFees: stg_ChildBounties_ChildrenCuratorFees,
    },
    {
      add_child_bounty: call_ChildBounties_add_child_bounty,
      propose_curator: call_ChildBounties_propose_curator,
      accept_curator: call_ChildBounties_accept_curator,
      unassign_curator: call_ChildBounties_unassign_curator,
      award_child_bounty: call_ChildBounties_award_child_bounty,
      claim_child_bounty: call_ChildBounties_claim_child_bounty,
      close_child_bounty: call_ChildBounties_close_child_bounty,
    },
    {
      Added: evt_ChildBounties_Added,
      Awarded: evt_ChildBounties_Awarded,
      Claimed: evt_ChildBounties_Claimed,
      Canceled: evt_ChildBounties_Canceled,
    },
    {
      ParentBountyNotActive: err_ChildBounties_ParentBountyNotActive,
      InsufficientBountyBalance: err_ChildBounties_InsufficientBountyBalance,
      TooManyChildBounties: err_ChildBounties_TooManyChildBounties,
    },
    {
      MaxActiveChildBountyCount: const_ChildBounties_MaxActiveChildBountyCount,
      ChildBountyValueMinimum: const_ChildBounties_ChildBountyValueMinimum,
    },
  ],
  Nis: [
    {
      QueueTotals: stg_Nis_QueueTotals,
      Queues: stg_Nis_Queues,
      Summary: stg_Nis_Summary,
      Receipts: stg_Nis_Receipts,
    },
    {
      place_bid: call_Nis_place_bid,
      retract_bid: call_Nis_retract_bid,
      fund_deficit: call_Nis_fund_deficit,
      thaw_private: call_Nis_thaw_private,
      thaw_communal: call_Nis_thaw_communal,
      communify: call_Nis_communify,
      privatize: call_Nis_privatize,
    },
    {
      BidPlaced: evt_Nis_BidPlaced,
      BidRetracted: evt_Nis_BidRetracted,
      BidDropped: evt_Nis_BidDropped,
      Issued: evt_Nis_Issued,
      Thawed: evt_Nis_Thawed,
      Funded: evt_Nis_Funded,
      Transferred: evt_Nis_Transferred,
    },
    {
      DurationTooSmall: err_Nis_DurationTooSmall,
      DurationTooBig: err_Nis_DurationTooBig,
      AmountTooSmall: err_Nis_AmountTooSmall,
      BidTooLow: err_Nis_BidTooLow,
      UnknownReceipt: err_Nis_UnknownReceipt,
      NotOwner: err_Nis_NotOwner,
      NotExpired: err_Nis_NotExpired,
      UnknownBid: err_Nis_UnknownBid,
      PortionTooBig: err_Nis_PortionTooBig,
      Unfunded: err_Nis_Unfunded,
      AlreadyFunded: err_Nis_AlreadyFunded,
      Throttled: err_Nis_Throttled,
      MakesDust: err_Nis_MakesDust,
      AlreadyCommunal: err_Nis_AlreadyCommunal,
      AlreadyPrivate: err_Nis_AlreadyPrivate,
    },
    {
      PalletId: const_Nis_PalletId,
      QueueCount: const_Nis_QueueCount,
      MaxQueueLen: const_Nis_MaxQueueLen,
      FifoQueueLen: const_Nis_FifoQueueLen,
      BasePeriod: const_Nis_BasePeriod,
      MinBid: const_Nis_MinBid,
      MinReceipt: const_Nis_MinReceipt,
      IntakePeriod: const_Nis_IntakePeriod,
      MaxIntakeWeight: const_Nis_MaxIntakeWeight,
      ThawThrottle: const_Nis_ThawThrottle,
    },
  ],
  NisCounterpartBalances: [
    {
      TotalIssuance: stg_NisCounterpartBalances_TotalIssuance,
      InactiveIssuance: stg_NisCounterpartBalances_InactiveIssuance,
      Account: stg_NisCounterpartBalances_Account,
      Locks: stg_NisCounterpartBalances_Locks,
      Reserves: stg_NisCounterpartBalances_Reserves,
      Holds: stg_NisCounterpartBalances_Holds,
      Freezes: stg_NisCounterpartBalances_Freezes,
    },
    {
      transfer_allow_death: call_NisCounterpartBalances_transfer_allow_death,
      force_transfer: call_NisCounterpartBalances_force_transfer,
      transfer_keep_alive: call_NisCounterpartBalances_transfer_keep_alive,
      transfer_all: call_NisCounterpartBalances_transfer_all,
      force_unreserve: call_NisCounterpartBalances_force_unreserve,
      upgrade_accounts: call_NisCounterpartBalances_upgrade_accounts,
      force_set_balance: call_NisCounterpartBalances_force_set_balance,
      force_adjust_total_issuance: call_NisCounterpartBalances_force_adjust_total_issuance,
    },
    {
      Endowed: evt_NisCounterpartBalances_Endowed,
      DustLost: evt_NisCounterpartBalances_DustLost,
      Transfer: evt_NisCounterpartBalances_Transfer,
      BalanceSet: evt_NisCounterpartBalances_BalanceSet,
      Reserved: evt_NisCounterpartBalances_Reserved,
      Unreserved: evt_NisCounterpartBalances_Unreserved,
      ReserveRepatriated: evt_NisCounterpartBalances_ReserveRepatriated,
      Deposit: evt_NisCounterpartBalances_Deposit,
      Withdraw: evt_NisCounterpartBalances_Withdraw,
      Slashed: evt_NisCounterpartBalances_Slashed,
      Minted: evt_NisCounterpartBalances_Minted,
      Burned: evt_NisCounterpartBalances_Burned,
      Suspended: evt_NisCounterpartBalances_Suspended,
      Restored: evt_NisCounterpartBalances_Restored,
      Upgraded: evt_NisCounterpartBalances_Upgraded,
      Issued: evt_NisCounterpartBalances_Issued,
      Rescinded: evt_NisCounterpartBalances_Rescinded,
      Locked: evt_NisCounterpartBalances_Locked,
      Unlocked: evt_NisCounterpartBalances_Unlocked,
      Frozen: evt_NisCounterpartBalances_Frozen,
      Thawed: evt_NisCounterpartBalances_Thawed,
      TotalIssuanceForced: evt_NisCounterpartBalances_TotalIssuanceForced,
    },
    {
      VestingBalance: err_NisCounterpartBalances_VestingBalance,
      LiquidityRestrictions: err_NisCounterpartBalances_LiquidityRestrictions,
      InsufficientBalance: err_NisCounterpartBalances_InsufficientBalance,
      ExistentialDeposit: err_NisCounterpartBalances_ExistentialDeposit,
      Expendability: err_NisCounterpartBalances_Expendability,
      ExistingVestingSchedule: err_NisCounterpartBalances_ExistingVestingSchedule,
      DeadAccount: err_NisCounterpartBalances_DeadAccount,
      TooManyReserves: err_NisCounterpartBalances_TooManyReserves,
      TooManyHolds: err_NisCounterpartBalances_TooManyHolds,
      TooManyFreezes: err_NisCounterpartBalances_TooManyFreezes,
      IssuanceDeactivated: err_NisCounterpartBalances_IssuanceDeactivated,
      DeltaZero: err_NisCounterpartBalances_DeltaZero,
    },
    {
      ExistentialDeposit: const_NisCounterpartBalances_ExistentialDeposit,
      MaxLocks: const_NisCounterpartBalances_MaxLocks,
      MaxReserves: const_NisCounterpartBalances_MaxReserves,
      MaxFreezes: const_NisCounterpartBalances_MaxFreezes,
    },
  ],
  ParachainsOrigin: [{}, {}, {}, {}, {}],
  Configuration: [
    {
      ActiveConfig: stg_Configuration_ActiveConfig,
      PendingConfigs: stg_Configuration_PendingConfigs,
      BypassConsistencyCheck: stg_Configuration_BypassConsistencyCheck,
    },
    {
      set_validation_upgrade_cooldown: call_Configuration_set_validation_upgrade_cooldown,
      set_validation_upgrade_delay: call_Configuration_set_validation_upgrade_delay,
      set_code_retention_period: call_Configuration_set_code_retention_period,
      set_max_code_size: call_Configuration_set_max_code_size,
      set_max_pov_size: call_Configuration_set_max_pov_size,
      set_max_head_data_size: call_Configuration_set_max_head_data_size,
      set_coretime_cores: call_Configuration_set_coretime_cores,
      set_on_demand_retries: call_Configuration_set_on_demand_retries,
      set_group_rotation_frequency: call_Configuration_set_group_rotation_frequency,
      set_paras_availability_period: call_Configuration_set_paras_availability_period,
      set_scheduling_lookahead: call_Configuration_set_scheduling_lookahead,
      set_max_validators_per_core: call_Configuration_set_max_validators_per_core,
      set_max_validators: call_Configuration_set_max_validators,
      set_dispute_period: call_Configuration_set_dispute_period,
      set_dispute_post_conclusion_acceptance_period:
        call_Configuration_set_dispute_post_conclusion_acceptance_period,
      set_no_show_slots: call_Configuration_set_no_show_slots,
      set_n_delay_tranches: call_Configuration_set_n_delay_tranches,
      set_zeroth_delay_tranche_width: call_Configuration_set_zeroth_delay_tranche_width,
      set_needed_approvals: call_Configuration_set_needed_approvals,
      set_relay_vrf_modulo_samples: call_Configuration_set_relay_vrf_modulo_samples,
      set_max_upward_queue_count: call_Configuration_set_max_upward_queue_count,
      set_max_upward_queue_size: call_Configuration_set_max_upward_queue_size,
      set_max_downward_message_size: call_Configuration_set_max_downward_message_size,
      set_max_upward_message_size: call_Configuration_set_max_upward_message_size,
      set_max_upward_message_num_per_candidate:
        call_Configuration_set_max_upward_message_num_per_candidate,
      set_hrmp_open_request_ttl: call_Configuration_set_hrmp_open_request_ttl,
      set_hrmp_sender_deposit: call_Configuration_set_hrmp_sender_deposit,
      set_hrmp_recipient_deposit: call_Configuration_set_hrmp_recipient_deposit,
      set_hrmp_channel_max_capacity: call_Configuration_set_hrmp_channel_max_capacity,
      set_hrmp_channel_max_total_size: call_Configuration_set_hrmp_channel_max_total_size,
      set_hrmp_max_parachain_inbound_channels:
        call_Configuration_set_hrmp_max_parachain_inbound_channels,
      set_hrmp_channel_max_message_size: call_Configuration_set_hrmp_channel_max_message_size,
      set_hrmp_max_parachain_outbound_channels:
        call_Configuration_set_hrmp_max_parachain_outbound_channels,
      set_hrmp_max_message_num_per_candidate:
        call_Configuration_set_hrmp_max_message_num_per_candidate,
      set_pvf_voting_ttl: call_Configuration_set_pvf_voting_ttl,
      set_minimum_validation_upgrade_delay: call_Configuration_set_minimum_validation_upgrade_delay,
      set_bypass_consistency_check: call_Configuration_set_bypass_consistency_check,
      set_async_backing_params: call_Configuration_set_async_backing_params,
      set_executor_params: call_Configuration_set_executor_params,
      set_on_demand_base_fee: call_Configuration_set_on_demand_base_fee,
      set_on_demand_fee_variability: call_Configuration_set_on_demand_fee_variability,
      set_on_demand_queue_max_size: call_Configuration_set_on_demand_queue_max_size,
      set_on_demand_target_queue_utilization:
        call_Configuration_set_on_demand_target_queue_utilization,
      set_on_demand_ttl: call_Configuration_set_on_demand_ttl,
      set_minimum_backing_votes: call_Configuration_set_minimum_backing_votes,
      set_node_feature: call_Configuration_set_node_feature,
      set_approval_voting_params: call_Configuration_set_approval_voting_params,
    },
    {},
    { InvalidNewValue: err_Configuration_InvalidNewValue },
    {},
  ],
  ParasShared: [
    {
      CurrentSessionIndex: stg_ParasShared_CurrentSessionIndex,
      ActiveValidatorIndices: stg_ParasShared_ActiveValidatorIndices,
      ActiveValidatorKeys: stg_ParasShared_ActiveValidatorKeys,
      AllowedRelayParents: stg_ParasShared_AllowedRelayParents,
    },
    {},
    {},
    {},
    {},
  ],
  ParaInclusion: [
    {
      AvailabilityBitfields: stg_ParaInclusion_AvailabilityBitfields,
      PendingAvailability: stg_ParaInclusion_PendingAvailability,
      PendingAvailabilityCommitments: stg_ParaInclusion_PendingAvailabilityCommitments,
    },
    {},
    {
      CandidateBacked: evt_ParaInclusion_CandidateBacked,
      CandidateIncluded: evt_ParaInclusion_CandidateIncluded,
      CandidateTimedOut: evt_ParaInclusion_CandidateTimedOut,
      UpwardMessagesReceived: evt_ParaInclusion_UpwardMessagesReceived,
    },
    {
      UnsortedOrDuplicateValidatorIndices: err_ParaInclusion_UnsortedOrDuplicateValidatorIndices,
      UnsortedOrDuplicateDisputeStatementSet:
        err_ParaInclusion_UnsortedOrDuplicateDisputeStatementSet,
      UnsortedOrDuplicateBackedCandidates: err_ParaInclusion_UnsortedOrDuplicateBackedCandidates,
      UnexpectedRelayParent: err_ParaInclusion_UnexpectedRelayParent,
      WrongBitfieldSize: err_ParaInclusion_WrongBitfieldSize,
      BitfieldAllZeros: err_ParaInclusion_BitfieldAllZeros,
      BitfieldDuplicateOrUnordered: err_ParaInclusion_BitfieldDuplicateOrUnordered,
      ValidatorIndexOutOfBounds: err_ParaInclusion_ValidatorIndexOutOfBounds,
      InvalidBitfieldSignature: err_ParaInclusion_InvalidBitfieldSignature,
      UnscheduledCandidate: err_ParaInclusion_UnscheduledCandidate,
      CandidateScheduledBeforeParaFree: err_ParaInclusion_CandidateScheduledBeforeParaFree,
      ScheduledOutOfOrder: err_ParaInclusion_ScheduledOutOfOrder,
      HeadDataTooLarge: err_ParaInclusion_HeadDataTooLarge,
      PrematureCodeUpgrade: err_ParaInclusion_PrematureCodeUpgrade,
      NewCodeTooLarge: err_ParaInclusion_NewCodeTooLarge,
      DisallowedRelayParent: err_ParaInclusion_DisallowedRelayParent,
      InvalidAssignment: err_ParaInclusion_InvalidAssignment,
      InvalidGroupIndex: err_ParaInclusion_InvalidGroupIndex,
      InsufficientBacking: err_ParaInclusion_InsufficientBacking,
      InvalidBacking: err_ParaInclusion_InvalidBacking,
      NotCollatorSigned: err_ParaInclusion_NotCollatorSigned,
      ValidationDataHashMismatch: err_ParaInclusion_ValidationDataHashMismatch,
      IncorrectDownwardMessageHandling: err_ParaInclusion_IncorrectDownwardMessageHandling,
      InvalidUpwardMessages: err_ParaInclusion_InvalidUpwardMessages,
      HrmpWatermarkMishandling: err_ParaInclusion_HrmpWatermarkMishandling,
      InvalidOutboundHrmp: err_ParaInclusion_InvalidOutboundHrmp,
      InvalidValidationCodeHash: err_ParaInclusion_InvalidValidationCodeHash,
      ParaHeadMismatch: err_ParaInclusion_ParaHeadMismatch,
      BitfieldReferencesFreedCore: err_ParaInclusion_BitfieldReferencesFreedCore,
    },
    {},
  ],
  ParaInherent: [
    { Included: stg_ParaInherent_Included, OnChainVotes: stg_ParaInherent_OnChainVotes },
    { enter: call_ParaInherent_enter },
    {},
    {
      TooManyInclusionInherents: err_ParaInherent_TooManyInclusionInherents,
      InvalidParentHeader: err_ParaInherent_InvalidParentHeader,
      CandidateConcludedInvalid: err_ParaInherent_CandidateConcludedInvalid,
      InherentOverweight: err_ParaInherent_InherentOverweight,
      DisputeStatementsUnsortedOrDuplicates: err_ParaInherent_DisputeStatementsUnsortedOrDuplicates,
      DisputeInvalid: err_ParaInherent_DisputeInvalid,
      BackedByDisabled: err_ParaInherent_BackedByDisabled,
      BackedOnUnscheduledCore: err_ParaInherent_BackedOnUnscheduledCore,
      UnscheduledCandidate: err_ParaInherent_UnscheduledCandidate,
    },
    {},
  ],
  ParaScheduler: [
    {
      ValidatorGroups: stg_ParaScheduler_ValidatorGroups,
      AvailabilityCores: stg_ParaScheduler_AvailabilityCores,
      SessionStartBlock: stg_ParaScheduler_SessionStartBlock,
      ClaimQueue: stg_ParaScheduler_ClaimQueue,
    },
    {},
    {},
    {},
    {},
  ],
  Paras: [
    {
      PvfActiveVoteMap: stg_Paras_PvfActiveVoteMap,
      PvfActiveVoteList: stg_Paras_PvfActiveVoteList,
      Parachains: stg_Paras_Parachains,
      ParaLifecycles: stg_Paras_ParaLifecycles,
      Heads: stg_Paras_Heads,
      MostRecentContext: stg_Paras_MostRecentContext,
      CurrentCodeHash: stg_Paras_CurrentCodeHash,
      PastCodeHash: stg_Paras_PastCodeHash,
      PastCodeMeta: stg_Paras_PastCodeMeta,
      PastCodePruning: stg_Paras_PastCodePruning,
      FutureCodeUpgrades: stg_Paras_FutureCodeUpgrades,
      FutureCodeHash: stg_Paras_FutureCodeHash,
      UpgradeGoAheadSignal: stg_Paras_UpgradeGoAheadSignal,
      UpgradeRestrictionSignal: stg_Paras_UpgradeRestrictionSignal,
      UpgradeCooldowns: stg_Paras_UpgradeCooldowns,
      UpcomingUpgrades: stg_Paras_UpcomingUpgrades,
      ActionsQueue: stg_Paras_ActionsQueue,
      UpcomingParasGenesis: stg_Paras_UpcomingParasGenesis,
      CodeByHashRefs: stg_Paras_CodeByHashRefs,
      CodeByHash: stg_Paras_CodeByHash,
    },
    {
      force_set_current_code: call_Paras_force_set_current_code,
      force_set_current_head: call_Paras_force_set_current_head,
      force_schedule_code_upgrade: call_Paras_force_schedule_code_upgrade,
      force_note_new_head: call_Paras_force_note_new_head,
      force_queue_action: call_Paras_force_queue_action,
      add_trusted_validation_code: call_Paras_add_trusted_validation_code,
      poke_unused_validation_code: call_Paras_poke_unused_validation_code,
      include_pvf_check_statement: call_Paras_include_pvf_check_statement,
      force_set_most_recent_context: call_Paras_force_set_most_recent_context,
    },
    {
      CurrentCodeUpdated: evt_Paras_CurrentCodeUpdated,
      CurrentHeadUpdated: evt_Paras_CurrentHeadUpdated,
      CodeUpgradeScheduled: evt_Paras_CodeUpgradeScheduled,
      NewHeadNoted: evt_Paras_NewHeadNoted,
      ActionQueued: evt_Paras_ActionQueued,
      PvfCheckStarted: evt_Paras_PvfCheckStarted,
      PvfCheckAccepted: evt_Paras_PvfCheckAccepted,
      PvfCheckRejected: evt_Paras_PvfCheckRejected,
    },
    {
      NotRegistered: err_Paras_NotRegistered,
      CannotOnboard: err_Paras_CannotOnboard,
      CannotOffboard: err_Paras_CannotOffboard,
      CannotUpgrade: err_Paras_CannotUpgrade,
      CannotDowngrade: err_Paras_CannotDowngrade,
      PvfCheckStatementStale: err_Paras_PvfCheckStatementStale,
      PvfCheckStatementFuture: err_Paras_PvfCheckStatementFuture,
      PvfCheckValidatorIndexOutOfBounds: err_Paras_PvfCheckValidatorIndexOutOfBounds,
      PvfCheckInvalidSignature: err_Paras_PvfCheckInvalidSignature,
      PvfCheckDoubleVote: err_Paras_PvfCheckDoubleVote,
      PvfCheckSubjectInvalid: err_Paras_PvfCheckSubjectInvalid,
      CannotUpgradeCode: err_Paras_CannotUpgradeCode,
      InvalidCode: err_Paras_InvalidCode,
    },
    { UnsignedPriority: const_Paras_UnsignedPriority },
  ],
  Initializer: [
    {
      HasInitialized: stg_Initializer_HasInitialized,
      BufferedSessionChanges: stg_Initializer_BufferedSessionChanges,
    },
    { force_approve: call_Initializer_force_approve },
    {},
    {},
    {},
  ],
  Dmp: [
    {
      DownwardMessageQueues: stg_Dmp_DownwardMessageQueues,
      DownwardMessageQueueHeads: stg_Dmp_DownwardMessageQueueHeads,
      DeliveryFeeFactor: stg_Dmp_DeliveryFeeFactor,
    },
    {},
    {},
    {},
    {},
  ],
  Hrmp: [
    {
      HrmpOpenChannelRequests: stg_Hrmp_HrmpOpenChannelRequests,
      HrmpOpenChannelRequestsList: stg_Hrmp_HrmpOpenChannelRequestsList,
      HrmpOpenChannelRequestCount: stg_Hrmp_HrmpOpenChannelRequestCount,
      HrmpAcceptedChannelRequestCount: stg_Hrmp_HrmpAcceptedChannelRequestCount,
      HrmpCloseChannelRequests: stg_Hrmp_HrmpCloseChannelRequests,
      HrmpCloseChannelRequestsList: stg_Hrmp_HrmpCloseChannelRequestsList,
      HrmpWatermarks: stg_Hrmp_HrmpWatermarks,
      HrmpChannels: stg_Hrmp_HrmpChannels,
      HrmpIngressChannelsIndex: stg_Hrmp_HrmpIngressChannelsIndex,
      HrmpEgressChannelsIndex: stg_Hrmp_HrmpEgressChannelsIndex,
      HrmpChannelContents: stg_Hrmp_HrmpChannelContents,
      HrmpChannelDigests: stg_Hrmp_HrmpChannelDigests,
    },
    {
      hrmp_init_open_channel: call_Hrmp_hrmp_init_open_channel,
      hrmp_accept_open_channel: call_Hrmp_hrmp_accept_open_channel,
      hrmp_close_channel: call_Hrmp_hrmp_close_channel,
      force_clean_hrmp: call_Hrmp_force_clean_hrmp,
      force_process_hrmp_open: call_Hrmp_force_process_hrmp_open,
      force_process_hrmp_close: call_Hrmp_force_process_hrmp_close,
      hrmp_cancel_open_request: call_Hrmp_hrmp_cancel_open_request,
      force_open_hrmp_channel: call_Hrmp_force_open_hrmp_channel,
      establish_system_channel: call_Hrmp_establish_system_channel,
      poke_channel_deposits: call_Hrmp_poke_channel_deposits,
    },
    {
      OpenChannelRequested: evt_Hrmp_OpenChannelRequested,
      OpenChannelCanceled: evt_Hrmp_OpenChannelCanceled,
      OpenChannelAccepted: evt_Hrmp_OpenChannelAccepted,
      ChannelClosed: evt_Hrmp_ChannelClosed,
      HrmpChannelForceOpened: evt_Hrmp_HrmpChannelForceOpened,
      HrmpSystemChannelOpened: evt_Hrmp_HrmpSystemChannelOpened,
      OpenChannelDepositsUpdated: evt_Hrmp_OpenChannelDepositsUpdated,
    },
    {
      OpenHrmpChannelToSelf: err_Hrmp_OpenHrmpChannelToSelf,
      OpenHrmpChannelInvalidRecipient: err_Hrmp_OpenHrmpChannelInvalidRecipient,
      OpenHrmpChannelZeroCapacity: err_Hrmp_OpenHrmpChannelZeroCapacity,
      OpenHrmpChannelCapacityExceedsLimit: err_Hrmp_OpenHrmpChannelCapacityExceedsLimit,
      OpenHrmpChannelZeroMessageSize: err_Hrmp_OpenHrmpChannelZeroMessageSize,
      OpenHrmpChannelMessageSizeExceedsLimit: err_Hrmp_OpenHrmpChannelMessageSizeExceedsLimit,
      OpenHrmpChannelAlreadyExists: err_Hrmp_OpenHrmpChannelAlreadyExists,
      OpenHrmpChannelAlreadyRequested: err_Hrmp_OpenHrmpChannelAlreadyRequested,
      OpenHrmpChannelLimitExceeded: err_Hrmp_OpenHrmpChannelLimitExceeded,
      AcceptHrmpChannelDoesntExist: err_Hrmp_AcceptHrmpChannelDoesntExist,
      AcceptHrmpChannelAlreadyConfirmed: err_Hrmp_AcceptHrmpChannelAlreadyConfirmed,
      AcceptHrmpChannelLimitExceeded: err_Hrmp_AcceptHrmpChannelLimitExceeded,
      CloseHrmpChannelUnauthorized: err_Hrmp_CloseHrmpChannelUnauthorized,
      CloseHrmpChannelDoesntExist: err_Hrmp_CloseHrmpChannelDoesntExist,
      CloseHrmpChannelAlreadyUnderway: err_Hrmp_CloseHrmpChannelAlreadyUnderway,
      CancelHrmpOpenChannelUnauthorized: err_Hrmp_CancelHrmpOpenChannelUnauthorized,
      OpenHrmpChannelDoesntExist: err_Hrmp_OpenHrmpChannelDoesntExist,
      OpenHrmpChannelAlreadyConfirmed: err_Hrmp_OpenHrmpChannelAlreadyConfirmed,
      WrongWitness: err_Hrmp_WrongWitness,
      ChannelCreationNotAuthorized: err_Hrmp_ChannelCreationNotAuthorized,
    },
    {},
  ],
  ParaSessionInfo: [
    {
      AssignmentKeysUnsafe: stg_ParaSessionInfo_AssignmentKeysUnsafe,
      EarliestStoredSession: stg_ParaSessionInfo_EarliestStoredSession,
      Sessions: stg_ParaSessionInfo_Sessions,
      AccountKeys: stg_ParaSessionInfo_AccountKeys,
      SessionExecutorParams: stg_ParaSessionInfo_SessionExecutorParams,
    },
    {},
    {},
    {},
    {},
  ],
  ParasDisputes: [
    {
      LastPrunedSession: stg_ParasDisputes_LastPrunedSession,
      Disputes: stg_ParasDisputes_Disputes,
      BackersOnDisputes: stg_ParasDisputes_BackersOnDisputes,
      Included: stg_ParasDisputes_Included,
      Frozen: stg_ParasDisputes_Frozen,
    },
    { force_unfreeze: call_ParasDisputes_force_unfreeze },
    {
      DisputeInitiated: evt_ParasDisputes_DisputeInitiated,
      DisputeConcluded: evt_ParasDisputes_DisputeConcluded,
      Revert: evt_ParasDisputes_Revert,
    },
    {
      DuplicateDisputeStatementSets: err_ParasDisputes_DuplicateDisputeStatementSets,
      AncientDisputeStatement: err_ParasDisputes_AncientDisputeStatement,
      ValidatorIndexOutOfBounds: err_ParasDisputes_ValidatorIndexOutOfBounds,
      InvalidSignature: err_ParasDisputes_InvalidSignature,
      DuplicateStatement: err_ParasDisputes_DuplicateStatement,
      SingleSidedDispute: err_ParasDisputes_SingleSidedDispute,
      MaliciousBacker: err_ParasDisputes_MaliciousBacker,
      MissingBackingVotes: err_ParasDisputes_MissingBackingVotes,
      UnconfirmedDispute: err_ParasDisputes_UnconfirmedDispute,
    },
    {},
  ],
  ParasSlashing: [
    {
      UnappliedSlashes: stg_ParasSlashing_UnappliedSlashes,
      ValidatorSetCounts: stg_ParasSlashing_ValidatorSetCounts,
    },
    { report_dispute_lost_unsigned: call_ParasSlashing_report_dispute_lost_unsigned },
    {},
    {
      InvalidKeyOwnershipProof: err_ParasSlashing_InvalidKeyOwnershipProof,
      InvalidSessionIndex: err_ParasSlashing_InvalidSessionIndex,
      InvalidCandidateHash: err_ParasSlashing_InvalidCandidateHash,
      InvalidValidatorIndex: err_ParasSlashing_InvalidValidatorIndex,
      ValidatorIndexIdMismatch: err_ParasSlashing_ValidatorIndexIdMismatch,
      DuplicateSlashingReport: err_ParasSlashing_DuplicateSlashingReport,
    },
    {},
  ],
  MessageQueue: [
    {
      BookStateFor: stg_MessageQueue_BookStateFor,
      ServiceHead: stg_MessageQueue_ServiceHead,
      Pages: stg_MessageQueue_Pages,
    },
    {
      reap_page: call_MessageQueue_reap_page,
      execute_overweight: call_MessageQueue_execute_overweight,
    },
    {
      ProcessingFailed: evt_MessageQueue_ProcessingFailed,
      Processed: evt_MessageQueue_Processed,
      OverweightEnqueued: evt_MessageQueue_OverweightEnqueued,
      PageReaped: evt_MessageQueue_PageReaped,
    },
    {
      NotReapable: err_MessageQueue_NotReapable,
      NoPage: err_MessageQueue_NoPage,
      NoMessage: err_MessageQueue_NoMessage,
      AlreadyProcessed: err_MessageQueue_AlreadyProcessed,
      Queued: err_MessageQueue_Queued,
      InsufficientWeight: err_MessageQueue_InsufficientWeight,
      TemporarilyUnprocessable: err_MessageQueue_TemporarilyUnprocessable,
      QueuePaused: err_MessageQueue_QueuePaused,
      RecursiveDisallowed: err_MessageQueue_RecursiveDisallowed,
    },
    {
      HeapSize: const_MessageQueue_HeapSize,
      MaxStale: const_MessageQueue_MaxStale,
      ServiceWeight: const_MessageQueue_ServiceWeight,
    },
  ],
  OnDemandAssignmentProvider: [
    {
      SpotTraffic: stg_OnDemandAssignmentProvider_SpotTraffic,
      OnDemandQueue: stg_OnDemandAssignmentProvider_OnDemandQueue,
      ParaIdAffinity: stg_OnDemandAssignmentProvider_ParaIdAffinity,
    },
    {
      place_order_allow_death: call_OnDemandAssignmentProvider_place_order_allow_death,
      place_order_keep_alive: call_OnDemandAssignmentProvider_place_order_keep_alive,
    },
    {
      OnDemandOrderPlaced: evt_OnDemandAssignmentProvider_OnDemandOrderPlaced,
      SpotTrafficSet: evt_OnDemandAssignmentProvider_SpotTrafficSet,
    },
    {
      InvalidParaId: err_OnDemandAssignmentProvider_InvalidParaId,
      QueueFull: err_OnDemandAssignmentProvider_QueueFull,
      SpotPriceHigherThanMaxAmount: err_OnDemandAssignmentProvider_SpotPriceHigherThanMaxAmount,
    },
    { TrafficDefaultValue: const_OnDemandAssignmentProvider_TrafficDefaultValue },
  ],
  ParachainsAssignmentProvider: [{}, {}, {}, {}, {}],
  CoretimeAssignmentProvider: [
    {
      CoreSchedules: stg_CoretimeAssignmentProvider_CoreSchedules,
      CoreDescriptors: stg_CoretimeAssignmentProvider_CoreDescriptors,
    },
    {},
    {},
    {
      AssignmentsEmpty: err_CoretimeAssignmentProvider_AssignmentsEmpty,
      OverScheduled: err_CoretimeAssignmentProvider_OverScheduled,
      UnderScheduled: err_CoretimeAssignmentProvider_UnderScheduled,
      DisallowedInsert: err_CoretimeAssignmentProvider_DisallowedInsert,
      DuplicateInsert: err_CoretimeAssignmentProvider_DuplicateInsert,
      AssignmentsNotSorted: err_CoretimeAssignmentProvider_AssignmentsNotSorted,
    },
    {},
  ],
  Registrar: [
    {
      PendingSwap: stg_Registrar_PendingSwap,
      Paras: stg_Registrar_Paras,
      NextFreeParaId: stg_Registrar_NextFreeParaId,
    },
    {
      register: call_Registrar_register,
      force_register: call_Registrar_force_register,
      deregister: call_Registrar_deregister,
      swap: call_Registrar_swap,
      remove_lock: call_Registrar_remove_lock,
      reserve: call_Registrar_reserve,
      add_lock: call_Registrar_add_lock,
      schedule_code_upgrade: call_Registrar_schedule_code_upgrade,
      set_current_head: call_Registrar_set_current_head,
    },
    {
      Registered: evt_Registrar_Registered,
      Deregistered: evt_Registrar_Deregistered,
      Reserved: evt_Registrar_Reserved,
      Swapped: evt_Registrar_Swapped,
    },
    {
      NotRegistered: err_Registrar_NotRegistered,
      AlreadyRegistered: err_Registrar_AlreadyRegistered,
      NotOwner: err_Registrar_NotOwner,
      CodeTooLarge: err_Registrar_CodeTooLarge,
      HeadDataTooLarge: err_Registrar_HeadDataTooLarge,
      NotParachain: err_Registrar_NotParachain,
      NotParathread: err_Registrar_NotParathread,
      CannotDeregister: err_Registrar_CannotDeregister,
      CannotDowngrade: err_Registrar_CannotDowngrade,
      CannotUpgrade: err_Registrar_CannotUpgrade,
      ParaLocked: err_Registrar_ParaLocked,
      NotReserved: err_Registrar_NotReserved,
      InvalidCode: err_Registrar_InvalidCode,
      CannotSwap: err_Registrar_CannotSwap,
    },
    {
      ParaDeposit: const_Registrar_ParaDeposit,
      DataDepositPerByte: const_Registrar_DataDepositPerByte,
    },
  ],
  Slots: [
    { Leases: stg_Slots_Leases },
    {
      force_lease: call_Slots_force_lease,
      clear_all_leases: call_Slots_clear_all_leases,
      trigger_onboard: call_Slots_trigger_onboard,
    },
    { NewLeasePeriod: evt_Slots_NewLeasePeriod, Leased: evt_Slots_Leased },
    { ParaNotOnboarding: err_Slots_ParaNotOnboarding, LeaseError: err_Slots_LeaseError },
    { LeasePeriod: const_Slots_LeasePeriod, LeaseOffset: const_Slots_LeaseOffset },
  ],
  Auctions: [
    {
      AuctionCounter: stg_Auctions_AuctionCounter,
      AuctionInfo: stg_Auctions_AuctionInfo,
      ReservedAmounts: stg_Auctions_ReservedAmounts,
      Winning: stg_Auctions_Winning,
    },
    {
      new_auction: call_Auctions_new_auction,
      bid: call_Auctions_bid,
      cancel_auction: call_Auctions_cancel_auction,
    },
    {
      AuctionStarted: evt_Auctions_AuctionStarted,
      AuctionClosed: evt_Auctions_AuctionClosed,
      Reserved: evt_Auctions_Reserved,
      Unreserved: evt_Auctions_Unreserved,
      ReserveConfiscated: evt_Auctions_ReserveConfiscated,
      BidAccepted: evt_Auctions_BidAccepted,
      WinningOffset: evt_Auctions_WinningOffset,
    },
    {
      AuctionInProgress: err_Auctions_AuctionInProgress,
      LeasePeriodInPast: err_Auctions_LeasePeriodInPast,
      ParaNotRegistered: err_Auctions_ParaNotRegistered,
      NotCurrentAuction: err_Auctions_NotCurrentAuction,
      NotAuction: err_Auctions_NotAuction,
      AuctionEnded: err_Auctions_AuctionEnded,
      AlreadyLeasedOut: err_Auctions_AlreadyLeasedOut,
    },
    {
      EndingPeriod: const_Auctions_EndingPeriod,
      SampleLength: const_Auctions_SampleLength,
      SlotRangeCount: const_Auctions_SlotRangeCount,
      LeasePeriodsPerSlot: const_Auctions_LeasePeriodsPerSlot,
    },
  ],
  Crowdloan: [
    {
      Funds: stg_Crowdloan_Funds,
      NewRaise: stg_Crowdloan_NewRaise,
      EndingsCount: stg_Crowdloan_EndingsCount,
      NextFundIndex: stg_Crowdloan_NextFundIndex,
    },
    {
      create: call_Crowdloan_create,
      contribute: call_Crowdloan_contribute,
      withdraw: call_Crowdloan_withdraw,
      refund: call_Crowdloan_refund,
      dissolve: call_Crowdloan_dissolve,
      edit: call_Crowdloan_edit,
      add_memo: call_Crowdloan_add_memo,
      poke: call_Crowdloan_poke,
      contribute_all: call_Crowdloan_contribute_all,
    },
    {
      Created: evt_Crowdloan_Created,
      Contributed: evt_Crowdloan_Contributed,
      Withdrew: evt_Crowdloan_Withdrew,
      PartiallyRefunded: evt_Crowdloan_PartiallyRefunded,
      AllRefunded: evt_Crowdloan_AllRefunded,
      Dissolved: evt_Crowdloan_Dissolved,
      HandleBidResult: evt_Crowdloan_HandleBidResult,
      Edited: evt_Crowdloan_Edited,
      MemoUpdated: evt_Crowdloan_MemoUpdated,
      AddedToNewRaise: evt_Crowdloan_AddedToNewRaise,
    },
    {
      FirstPeriodInPast: err_Crowdloan_FirstPeriodInPast,
      FirstPeriodTooFarInFuture: err_Crowdloan_FirstPeriodTooFarInFuture,
      LastPeriodBeforeFirstPeriod: err_Crowdloan_LastPeriodBeforeFirstPeriod,
      LastPeriodTooFarInFuture: err_Crowdloan_LastPeriodTooFarInFuture,
      CannotEndInPast: err_Crowdloan_CannotEndInPast,
      EndTooFarInFuture: err_Crowdloan_EndTooFarInFuture,
      Overflow: err_Crowdloan_Overflow,
      ContributionTooSmall: err_Crowdloan_ContributionTooSmall,
      InvalidParaId: err_Crowdloan_InvalidParaId,
      CapExceeded: err_Crowdloan_CapExceeded,
      ContributionPeriodOver: err_Crowdloan_ContributionPeriodOver,
      InvalidOrigin: err_Crowdloan_InvalidOrigin,
      NotParachain: err_Crowdloan_NotParachain,
      LeaseActive: err_Crowdloan_LeaseActive,
      BidOrLeaseActive: err_Crowdloan_BidOrLeaseActive,
      FundNotEnded: err_Crowdloan_FundNotEnded,
      NoContributions: err_Crowdloan_NoContributions,
      NotReadyToDissolve: err_Crowdloan_NotReadyToDissolve,
      InvalidSignature: err_Crowdloan_InvalidSignature,
      MemoTooLarge: err_Crowdloan_MemoTooLarge,
      AlreadyInNewRaise: err_Crowdloan_AlreadyInNewRaise,
      VrfDelayInProgress: err_Crowdloan_VrfDelayInProgress,
      NoLeasePeriod: err_Crowdloan_NoLeasePeriod,
    },
    {
      PalletId: const_Crowdloan_PalletId,
      MinContribution: const_Crowdloan_MinContribution,
      RemoveKeysLimit: const_Crowdloan_RemoveKeysLimit,
    },
  ],
  Coretime: [
    {},
    {
      request_core_count: call_Coretime_request_core_count,
      assign_core: call_Coretime_assign_core,
    },
    {
      RevenueInfoRequested: evt_Coretime_RevenueInfoRequested,
      CoreAssigned: evt_Coretime_CoreAssigned,
    },
    { NotBroker: err_Coretime_NotBroker },
    { BrokerId: const_Coretime_BrokerId },
  ],
  XcmPallet: [
    {
      QueryCounter: stg_XcmPallet_QueryCounter,
      Queries: stg_XcmPallet_Queries,
      AssetTraps: stg_XcmPallet_AssetTraps,
      SafeXcmVersion: stg_XcmPallet_SafeXcmVersion,
      SupportedVersion: stg_XcmPallet_SupportedVersion,
      VersionNotifiers: stg_XcmPallet_VersionNotifiers,
      VersionNotifyTargets: stg_XcmPallet_VersionNotifyTargets,
      VersionDiscoveryQueue: stg_XcmPallet_VersionDiscoveryQueue,
      CurrentMigration: stg_XcmPallet_CurrentMigration,
      RemoteLockedFungibles: stg_XcmPallet_RemoteLockedFungibles,
      LockedFungibles: stg_XcmPallet_LockedFungibles,
      XcmExecutionSuspended: stg_XcmPallet_XcmExecutionSuspended,
    },
    {
      send: call_XcmPallet_send,
      teleport_assets: call_XcmPallet_teleport_assets,
      reserve_transfer_assets: call_XcmPallet_reserve_transfer_assets,
      execute: call_XcmPallet_execute,
      force_xcm_version: call_XcmPallet_force_xcm_version,
      force_default_xcm_version: call_XcmPallet_force_default_xcm_version,
      force_subscribe_version_notify: call_XcmPallet_force_subscribe_version_notify,
      force_unsubscribe_version_notify: call_XcmPallet_force_unsubscribe_version_notify,
      limited_reserve_transfer_assets: call_XcmPallet_limited_reserve_transfer_assets,
      limited_teleport_assets: call_XcmPallet_limited_teleport_assets,
      force_suspension: call_XcmPallet_force_suspension,
      transfer_assets: call_XcmPallet_transfer_assets,
    },
    {
      Attempted: evt_XcmPallet_Attempted,
      Sent: evt_XcmPallet_Sent,
      UnexpectedResponse: evt_XcmPallet_UnexpectedResponse,
      ResponseReady: evt_XcmPallet_ResponseReady,
      Notified: evt_XcmPallet_Notified,
      NotifyOverweight: evt_XcmPallet_NotifyOverweight,
      NotifyDispatchError: evt_XcmPallet_NotifyDispatchError,
      NotifyDecodeFailed: evt_XcmPallet_NotifyDecodeFailed,
      InvalidResponder: evt_XcmPallet_InvalidResponder,
      InvalidResponderVersion: evt_XcmPallet_InvalidResponderVersion,
      ResponseTaken: evt_XcmPallet_ResponseTaken,
      AssetsTrapped: evt_XcmPallet_AssetsTrapped,
      VersionChangeNotified: evt_XcmPallet_VersionChangeNotified,
      SupportedVersionChanged: evt_XcmPallet_SupportedVersionChanged,
      NotifyTargetSendFail: evt_XcmPallet_NotifyTargetSendFail,
      NotifyTargetMigrationFail: evt_XcmPallet_NotifyTargetMigrationFail,
      InvalidQuerierVersion: evt_XcmPallet_InvalidQuerierVersion,
      InvalidQuerier: evt_XcmPallet_InvalidQuerier,
      VersionNotifyStarted: evt_XcmPallet_VersionNotifyStarted,
      VersionNotifyRequested: evt_XcmPallet_VersionNotifyRequested,
      VersionNotifyUnrequested: evt_XcmPallet_VersionNotifyUnrequested,
      FeesPaid: evt_XcmPallet_FeesPaid,
      AssetsClaimed: evt_XcmPallet_AssetsClaimed,
      VersionMigrationFinished: evt_XcmPallet_VersionMigrationFinished,
    },
    {
      Unreachable: err_XcmPallet_Unreachable,
      SendFailure: err_XcmPallet_SendFailure,
      Filtered: err_XcmPallet_Filtered,
      UnweighableMessage: err_XcmPallet_UnweighableMessage,
      DestinationNotInvertible: err_XcmPallet_DestinationNotInvertible,
      Empty: err_XcmPallet_Empty,
      CannotReanchor: err_XcmPallet_CannotReanchor,
      TooManyAssets: err_XcmPallet_TooManyAssets,
      InvalidOrigin: err_XcmPallet_InvalidOrigin,
      BadVersion: err_XcmPallet_BadVersion,
      BadLocation: err_XcmPallet_BadLocation,
      NoSubscription: err_XcmPallet_NoSubscription,
      AlreadySubscribed: err_XcmPallet_AlreadySubscribed,
      CannotCheckOutTeleport: err_XcmPallet_CannotCheckOutTeleport,
      LowBalance: err_XcmPallet_LowBalance,
      TooManyLocks: err_XcmPallet_TooManyLocks,
      AccountNotSovereign: err_XcmPallet_AccountNotSovereign,
      FeesNotMet: err_XcmPallet_FeesNotMet,
      LockNotFound: err_XcmPallet_LockNotFound,
      InUse: err_XcmPallet_InUse,
      InvalidAssetNotConcrete: err_XcmPallet_InvalidAssetNotConcrete,
      InvalidAssetUnknownReserve: err_XcmPallet_InvalidAssetUnknownReserve,
      InvalidAssetUnsupportedReserve: err_XcmPallet_InvalidAssetUnsupportedReserve,
      TooManyReserves: err_XcmPallet_TooManyReserves,
      LocalExecutionIncomplete: err_XcmPallet_LocalExecutionIncomplete,
    },
    {},
  ],
  Beefy: [
    {
      Authorities: stg_Beefy_Authorities,
      ValidatorSetId: stg_Beefy_ValidatorSetId,
      NextAuthorities: stg_Beefy_NextAuthorities,
      SetIdSession: stg_Beefy_SetIdSession,
      GenesisBlock: stg_Beefy_GenesisBlock,
    },
    {
      report_equivocation: call_Beefy_report_equivocation,
      report_equivocation_unsigned: call_Beefy_report_equivocation_unsigned,
      set_new_genesis: call_Beefy_set_new_genesis,
    },
    {},
    {
      InvalidKeyOwnershipProof: err_Beefy_InvalidKeyOwnershipProof,
      InvalidEquivocationProof: err_Beefy_InvalidEquivocationProof,
      DuplicateOffenceReport: err_Beefy_DuplicateOffenceReport,
      InvalidConfiguration: err_Beefy_InvalidConfiguration,
    },
    {
      MaxAuthorities: const_Beefy_MaxAuthorities,
      MaxNominators: const_Beefy_MaxNominators,
      MaxSetIdSessionEntries: const_Beefy_MaxSetIdSessionEntries,
    },
  ],
  Mmr: [
    { RootHash: stg_Mmr_RootHash, NumberOfLeaves: stg_Mmr_NumberOfLeaves, Nodes: stg_Mmr_Nodes },
    {},
    {},
    {},
    {},
  ],
  MmrLeaf: [
    {
      BeefyAuthorities: stg_MmrLeaf_BeefyAuthorities,
      BeefyNextAuthorities: stg_MmrLeaf_BeefyNextAuthorities,
    },
    {},
    {},
    {},
    {},
  ],
  IdentityMigrator: [
    {},
    {
      reap_identity: call_IdentityMigrator_reap_identity,
      poke_deposit: call_IdentityMigrator_poke_deposit,
    },
    {
      IdentityReaped: evt_IdentityMigrator_IdentityReaped,
      DepositUpdated: evt_IdentityMigrator_DepositUpdated,
    },
    {},
    {},
  ],
  ParasSudoWrapper: [
    {},
    {
      sudo_schedule_para_initialize: call_ParasSudoWrapper_sudo_schedule_para_initialize,
      sudo_schedule_para_cleanup: call_ParasSudoWrapper_sudo_schedule_para_cleanup,
      sudo_schedule_parathread_upgrade: call_ParasSudoWrapper_sudo_schedule_parathread_upgrade,
      sudo_schedule_parachain_downgrade: call_ParasSudoWrapper_sudo_schedule_parachain_downgrade,
      sudo_queue_downward_xcm: call_ParasSudoWrapper_sudo_queue_downward_xcm,
      sudo_establish_hrmp_channel: call_ParasSudoWrapper_sudo_establish_hrmp_channel,
    },
    {},
    {
      ParaDoesntExist: err_ParasSudoWrapper_ParaDoesntExist,
      ParaAlreadyExists: err_ParasSudoWrapper_ParaAlreadyExists,
      ExceedsMaxMessageSize: err_ParasSudoWrapper_ExceedsMaxMessageSize,
      CouldntCleanup: err_ParasSudoWrapper_CouldntCleanup,
      NotParathread: err_ParasSudoWrapper_NotParathread,
      NotParachain: err_ParasSudoWrapper_NotParachain,
      CannotUpgrade: err_ParasSudoWrapper_CannotUpgrade,
      CannotDowngrade: err_ParasSudoWrapper_CannotDowngrade,
      TooManyCores: err_ParasSudoWrapper_TooManyCores,
    },
    {},
  ],
  AssignedSlots: [
    {
      PermanentSlots: stg_AssignedSlots_PermanentSlots,
      PermanentSlotCount: stg_AssignedSlots_PermanentSlotCount,
      TemporarySlots: stg_AssignedSlots_TemporarySlots,
      TemporarySlotCount: stg_AssignedSlots_TemporarySlotCount,
      ActiveTemporarySlotCount: stg_AssignedSlots_ActiveTemporarySlotCount,
      MaxTemporarySlots: stg_AssignedSlots_MaxTemporarySlots,
      MaxPermanentSlots: stg_AssignedSlots_MaxPermanentSlots,
    },
    {
      assign_perm_parachain_slot: call_AssignedSlots_assign_perm_parachain_slot,
      assign_temp_parachain_slot: call_AssignedSlots_assign_temp_parachain_slot,
      unassign_parachain_slot: call_AssignedSlots_unassign_parachain_slot,
      set_max_permanent_slots: call_AssignedSlots_set_max_permanent_slots,
      set_max_temporary_slots: call_AssignedSlots_set_max_temporary_slots,
    },
    {
      PermanentSlotAssigned: evt_AssignedSlots_PermanentSlotAssigned,
      TemporarySlotAssigned: evt_AssignedSlots_TemporarySlotAssigned,
      MaxPermanentSlotsChanged: evt_AssignedSlots_MaxPermanentSlotsChanged,
      MaxTemporarySlotsChanged: evt_AssignedSlots_MaxTemporarySlotsChanged,
    },
    {
      ParaDoesntExist: err_AssignedSlots_ParaDoesntExist,
      NotParathread: err_AssignedSlots_NotParathread,
      CannotUpgrade: err_AssignedSlots_CannotUpgrade,
      CannotDowngrade: err_AssignedSlots_CannotDowngrade,
      SlotAlreadyAssigned: err_AssignedSlots_SlotAlreadyAssigned,
      SlotNotAssigned: err_AssignedSlots_SlotNotAssigned,
      OngoingLeaseExists: err_AssignedSlots_OngoingLeaseExists,
      MaxPermanentSlotsExceeded: err_AssignedSlots_MaxPermanentSlotsExceeded,
      MaxTemporarySlotsExceeded: err_AssignedSlots_MaxTemporarySlotsExceeded,
    },
    {
      PermanentSlotLeasePeriodLength: const_AssignedSlots_PermanentSlotLeasePeriodLength,
      TemporarySlotLeasePeriodLength: const_AssignedSlots_TemporarySlotLeasePeriodLength,
      MaxTemporarySlotPerLeasePeriod: const_AssignedSlots_MaxTemporarySlotPerLeasePeriod,
    },
  ],
  ValidatorManager: [
    {
      ValidatorsToRetire: stg_ValidatorManager_ValidatorsToRetire,
      ValidatorsToAdd: stg_ValidatorManager_ValidatorsToAdd,
    },
    {
      register_validators: call_ValidatorManager_register_validators,
      deregister_validators: call_ValidatorManager_deregister_validators,
    },
    {
      ValidatorsRegistered: evt_ValidatorManager_ValidatorsRegistered,
      ValidatorsDeregistered: evt_ValidatorManager_ValidatorsDeregistered,
    },
    {},
    {},
  ],
  StateTrieMigration: [
    {
      MigrationProcess: stg_StateTrieMigration_MigrationProcess,
      AutoLimits: stg_StateTrieMigration_AutoLimits,
      SignedMigrationMaxLimits: stg_StateTrieMigration_SignedMigrationMaxLimits,
    },
    {
      control_auto_migration: call_StateTrieMigration_control_auto_migration,
      continue_migrate: call_StateTrieMigration_continue_migrate,
      migrate_custom_top: call_StateTrieMigration_migrate_custom_top,
      migrate_custom_child: call_StateTrieMigration_migrate_custom_child,
      set_signed_max_limits: call_StateTrieMigration_set_signed_max_limits,
      force_set_progress: call_StateTrieMigration_force_set_progress,
    },
    {
      Migrated: evt_StateTrieMigration_Migrated,
      Slashed: evt_StateTrieMigration_Slashed,
      AutoMigrationFinished: evt_StateTrieMigration_AutoMigrationFinished,
      Halted: evt_StateTrieMigration_Halted,
    },
    {
      MaxSignedLimits: err_StateTrieMigration_MaxSignedLimits,
      KeyTooLong: err_StateTrieMigration_KeyTooLong,
      NotEnoughFunds: err_StateTrieMigration_NotEnoughFunds,
      BadWitness: err_StateTrieMigration_BadWitness,
      SignedMigrationNotAllowed: err_StateTrieMigration_SignedMigrationNotAllowed,
      BadChildRoot: err_StateTrieMigration_BadChildRoot,
    },
    { MaxKeyLen: const_StateTrieMigration_MaxKeyLen },
  ],
  RootTesting: [
    {},
    {
      fill_block: call_RootTesting_fill_block,
      trigger_defensive: call_RootTesting_trigger_defensive,
    },
    { DefensiveTestCall: evt_RootTesting_DefensiveTestCall },
    {},
    {},
  ],
  Sudo: [
    { Key: stg_Sudo_Key },
    {
      sudo: call_Sudo_sudo,
      sudo_unchecked_weight: call_Sudo_sudo_unchecked_weight,
      set_key: call_Sudo_set_key,
      sudo_as: call_Sudo_sudo_as,
      remove_key: call_Sudo_remove_key,
    },
    {
      Sudid: evt_Sudo_Sudid,
      KeyChanged: evt_Sudo_KeyChanged,
      KeyRemoved: evt_Sudo_KeyRemoved,
      SudoAsDone: evt_Sudo_SudoAsDone,
    },
    { RequireSudo: err_Sudo_RequireSudo },
    {},
  ],
};

type IRuntimeCalls = {
  /**
   * The `Core` runtime api that every Substrate runtime needs to implement.
   */
  Core: {
    /**
     * Returns the version of the runtime.
     */
    version: RuntimeDescriptor<
      [],
      {
        spec_name: string;
        impl_name: string;
        authoring_version: number;
        spec_version: number;
        impl_version: number;
        apis: Anonymize<I1st1p92iu8h7e>;
        transaction_version: number;
        state_version: number;
      }
    >;

    /**
     * Execute the given block.
     */
    execute_block: RuntimeDescriptor<
      [block: { header: Anonymize<I6t1nedlt7mobn>; extrinsics: Anonymize<Itom7fk49o0c9> }],
      undefined
    >;

    /**
     * Initialize a block with the given header.
     */
    initialize_block: RuntimeDescriptor<
      [
        header: {
          parent_hash: Binary;
          number: number;
          state_root: Binary;
          extrinsics_root: Binary;
          digest: Anonymize<Idin6nhq46lvdj>;
        },
      ],
      undefined
    >;
  };

  /**
   * The `Metadata` api trait that returns metadata for the runtime.
   */
  Metadata: {
    /**
     * Returns the metadata of a runtime.
     */
    metadata: RuntimeDescriptor<[], Binary>;

    /**
     * Returns the metadata at a given version.
     *
     * If the given `version` isn't supported, this will return `None`.
     * Use [`Self::metadata_versions`] to find out about supported metadata version of the runtime.
     */
    metadata_at_version: RuntimeDescriptor<[version: number], Binary | undefined>;

    /**
     * Returns the supported metadata versions.
     *
     * This can be used to call `metadata_at_version`.
     */
    metadata_versions: RuntimeDescriptor<[], Array<number>>;
  };

  /**
   * The `BlockBuilder` api trait that provides the required functionality for building a block.
   */
  BlockBuilder: {
    /**
     * Apply the given extrinsic.
     *
     * Returns an inclusion outcome which specifies if this extrinsic is included in
     * this block or not.
     */
    apply_extrinsic: RuntimeDescriptor<
      [extrinsic: Binary],
      ResultPayload<Anonymize<Idtdr91jmq5g4i>, TransactionValidityError>
    >;

    /**
     * Finish the current block.
     */
    finalize_block: RuntimeDescriptor<
      [],
      {
        parent_hash: Binary;
        number: number;
        state_root: Binary;
        extrinsics_root: Binary;
        digest: Anonymize<Idin6nhq46lvdj>;
      }
    >;

    /**
     * Generate inherent extrinsics. The inherent data will vary from chain to chain.
     */
    inherent_extrinsics: RuntimeDescriptor<
      [inherent: Array<Anonymize<I1kbn2golmm2dm>>],
      Array<Binary>
    >;

    /**
     * Check that the inherents are valid. The inherent data will vary from chain to chain.
     */
    check_inherents: RuntimeDescriptor<
      [
        block: { header: Anonymize<I6t1nedlt7mobn>; extrinsics: Anonymize<Itom7fk49o0c9> },
        data: Array<Anonymize<I1kbn2golmm2dm>>,
      ],
      { okay: boolean; fatal_error: boolean; errors: Anonymize<If39abi8floaaf> }
    >;
  };

  /**
   * The `TaggedTransactionQueue` api trait for interfering with the transaction queue.
   */
  TaggedTransactionQueue: {
    /**
     * Validate the transaction.
     *
     * This method is invoked by the transaction pool to learn details about given transaction.
     * The implementation should make sure to verify the correctness of the transaction
     * against current state. The given `block_hash` corresponds to the hash of the block
     * that is used as current state.
     *
     * Note that this call may be performed by the pool multiple times and transactions
     * might be verified in any possible order.
     */
    validate_transaction: RuntimeDescriptor<
      [source: TransactionValidityTransactionSource, tx: Binary, block_hash: Binary],
      ResultPayload<Anonymize<I6g5lcd9vf2cr0>, TransactionValidityError>
    >;
  };

  /**
   * The offchain worker api.
   */
  OffchainWorkerApi: {
    /**
     * Starts the off-chain task for given block header.
     */
    offchain_worker: RuntimeDescriptor<
      [
        header: {
          parent_hash: Binary;
          number: number;
          state_root: Binary;
          extrinsics_root: Binary;
          digest: Anonymize<Idin6nhq46lvdj>;
        },
      ],
      undefined
    >;
  };

  /**
   * The API for querying the state of parachains on-chain.
   */
  ParachainHost: {
    /**
     * Get the current validators.
     */
    validators: RuntimeDescriptor<[], Array<Binary>>;

    /**
     * Returns the validator groups and rotation info localized based on the hypothetical child
     *  of a block whose state  this is invoked on. Note that `now` in the `GroupRotationInfo`
     * should be the successor of the number of the block.
     */
    validator_groups: RuntimeDescriptor<[], [Anonymize<Iarlj3qd8u1v13>, Anonymize<I94uslvmqboam8>]>;

    /**
     * Yields information on all availability cores as relevant to the child block.
     * Cores are either free or occupied. Free cores can have paras assigned to them.
     */
    availability_cores: RuntimeDescriptor<[], Array<PolkadotPrimitivesV5CoreState>>;

    /**
     * Yields the persisted validation data for the given `ParaId` along with an assumption that
     * should be used if the para currently occupies a core.
     *
     * Returns `None` if either the para is not registered or the assumption is `Freed`
     * and the para already occupies a core.
     */
    persisted_validation_data: RuntimeDescriptor<
      [para_id: number, assumption: PolkadotPrimitivesV5OccupiedCoreAssumption],
      Anonymize<I5r8ef6aie125l> | undefined
    >;

    /**
     * Returns the persisted validation data for the given `ParaId` along with the corresponding
     * validation code hash. Instead of accepting assumption about the para, matches the validation
     * data hash against an expected one and yields `None` if they're not equal.
     */
    assumed_validation_data: RuntimeDescriptor<
      [para_id: number, expected_persisted_validation_data_hash: Binary],
      Anonymize<I9rov8gdjkv3b9> | undefined
    >;

    /**
     * Checks if the given validation outputs pass the acceptance criteria.
     */
    check_validation_outputs: RuntimeDescriptor<
      [
        para_id: number,
        outputs: {
          upward_messages: Anonymize<Itom7fk49o0c9>;
          horizontal_messages: Anonymize<I6r5cbv8ttrb09>;
          new_validation_code: Anonymize<Iabpgqcjikia83>;
          head_data: Binary;
          processed_downward_messages: number;
          hrmp_watermark: number;
        },
      ],
      boolean
    >;

    /**
     * Returns the session index expected at a child of the block.
     *
     * This can be used to instantiate a `SigningContext`.
     */
    session_index_for_child: RuntimeDescriptor<[], number>;

    /**
     * Fetch the validation code used by a para, making the given `OccupiedCoreAssumption`.
     *
     * Returns `None` if either the para is not registered or the assumption is `Freed`
     * and the para already occupies a core.
     */
    validation_code: RuntimeDescriptor<
      [para_id: number, assumption: PolkadotPrimitivesV5OccupiedCoreAssumption],
      Binary | undefined
    >;

    /**
     * Get the receipt of a candidate pending availability. This returns `Some` for any paras
     * assigned to occupied cores in `availability_cores` and `None` otherwise.
     */
    candidate_pending_availability: RuntimeDescriptor<
      [para_id: number],
      Anonymize<Iedmhjqij0hr8g> | undefined
    >;

    /**
     * Get a vector of events concerning candidates that occurred within a block.
     */
    candidate_events: RuntimeDescriptor<[], Array<PolkadotPrimitivesV5CandidateEvent>>;

    /**
     * Get all the pending inbound messages in the downward message queue for a para.
     */
    dmq_contents: RuntimeDescriptor<[recipient: number], Array<Anonymize<I60847k37jfcc6>>>;

    /**
     * Get the contents of all channels addressed to the given recipient. Channels that have no
     * messages in them are also included.
     */
    inbound_hrmp_channels_contents: RuntimeDescriptor<
      [recipient: number],
      Array<Anonymize<I9hvej6h53dqj0>>
    >;

    /**
     * Get the validation code from its hash.
     */
    validation_code_by_hash: RuntimeDescriptor<[hash: Binary], Binary | undefined>;

    /**
     * Scrape dispute relevant from on-chain, backing votes and resolved disputes.
     */
    on_chain_votes: RuntimeDescriptor<[], Anonymize<Ie7c40q4va8u7r> | undefined>;

    /**
     * Get the session info for the given session, if stored.
     *
     * NOTE: This function is only available since parachain host version 2.
     */
    session_info: RuntimeDescriptor<[index: number], Anonymize<I7k9oi9p83j43l> | undefined>;

    /**
     * Submits a PVF pre-checking statement into the transaction pool.
     *
     * NOTE: This function is only available since parachain host version 2.
     */
    submit_pvf_check_statement: RuntimeDescriptor<
      [
        stmt: { accept: boolean; subject: Binary; session_index: number; validator_index: number },
        signature: Binary,
      ],
      undefined
    >;

    /**
     * Returns code hashes of PVFs that require pre-checking by validators in the active set.
     *
     * NOTE: This function is only available since parachain host version 2.
     */
    pvfs_require_precheck: RuntimeDescriptor<[], Array<Binary>>;

    /**
     * Fetch the hash of the validation code used by a para, making the given `OccupiedCoreAssumption`.
     *
     * NOTE: This function is only available since parachain host version 2.
     */
    validation_code_hash: RuntimeDescriptor<
      [para_id: number, assumption: PolkadotPrimitivesV5OccupiedCoreAssumption],
      Binary | undefined
    >;

    /**
     * Returns all onchain disputes.
     */
    disputes: RuntimeDescriptor<[], Array<Anonymize<Irsmd7gp7po60>>>;

    /**
     * Returns execution parameters for the session.
     */
    session_executor_params: RuntimeDescriptor<
      [session_index: number],
      Anonymize<I79n5nka727sg9> | undefined
    >;

    /**
     * Returns a list of validators that lost a past session dispute and need to be slashed.
     * NOTE: This function is only available since parachain host version 5.
     */
    unapplied_slashes: RuntimeDescriptor<[], Array<Anonymize<I5ork0l271qluj>>>;

    /**
     * Returns a merkle proof of a validator session key.
     * NOTE: This function is only available since parachain host version 5.
     */
    key_ownership_proof: RuntimeDescriptor<[validator_id: Binary], Binary | undefined>;

    /**
     * Submit an unsigned extrinsic to slash validators who lost a dispute about
     * a candidate of a past session.
     * NOTE: This function is only available since parachain host version 5.
     */
    submit_report_dispute_lost: RuntimeDescriptor<
      [
        dispute_proof: {
          time_slot: Anonymize<Ib0p1u8q0nho37>;
          kind: PolkadotPrimitivesV5SlashingSlashingOffenceKind;
          validator_index: number;
          validator_id: Binary;
        },
        key_ownership_proof: Binary,
      ],
      boolean
    >;

    /**
     * Get the minimum number of backing votes for a parachain candidate.
     * This is a staging method! Do not use on production runtimes!
     */
    minimum_backing_votes: RuntimeDescriptor<[], number>;

    /**
     * Returns the state of parachain backing for a given para.
     */
    para_backing_state: RuntimeDescriptor<[_: number], Anonymize<I53uro0ns8ba5b> | undefined>;

    /**
     * Returns candidate's acceptance limitations for asynchronous backing for a relay parent.
     */
    async_backing_params: RuntimeDescriptor<
      [],
      { max_candidate_depth: number; allowed_ancestry_len: number }
    >;

    /**
     * Returns a list of all disabled validators at the given block.
     */
    disabled_validators: RuntimeDescriptor<[], Array<number>>;

    /**
     * Get node features.
     * This is a staging method! Do not use on production runtimes!
     */
    node_features: RuntimeDescriptor<[], { bytes: Uint8Array; bitsLen: number }>;

    /**
     * Approval voting configuration parameters
     */
    approval_voting_params: RuntimeDescriptor<[], number>;
  };

  /**
   * API necessary for BEEFY voters.
   */
  BeefyApi: {
    /**
     * Return the block number where BEEFY consensus is enabled/started
     */
    beefy_genesis: RuntimeDescriptor<[], number | undefined>;

    /**
     * Return the current active BEEFY validator set
     */
    validator_set: RuntimeDescriptor<[], Anonymize<I17bb7o70l1ke3> | undefined>;

    /**
     * Submits an unsigned extrinsic to report an equivocation. The caller
     * must provide the equivocation proof and a key ownership proof
     * (should be obtained using `generate_key_ownership_proof`). The
     * extrinsic will be unsigned and should only be accepted for local
     * authorship (not to be broadcast to the network). This method returns
     * `None` when creation of the extrinsic fails, e.g. if equivocation
     * reporting is disabled for the given runtime (i.e. this method is
     * hardcoded to return `None`). Only useful in an offchain context.
     */
    submit_report_equivocation_unsigned_extrinsic: RuntimeDescriptor<
      [
        equivocation_proof: { first: Anonymize<Ifukb31gb9emjd>; second: Anonymize<Ifukb31gb9emjd> },
        key_owner_proof: Binary,
      ],
      boolean
    >;

    /**
     * Generates a proof of key ownership for the given authority in the
     * given set. An example usage of this module is coupled with the
     * session historical module to prove that a given authority key is
     * tied to a given staking identity during a specific session. Proofs
     * of key ownership are necessary for submitting equivocation reports.
     * NOTE: even though the API takes a `set_id` as parameter the current
     * implementations ignores this parameter and instead relies on this
     * method being called at the correct block height, i.e. any point at
     * which the given set id is live on-chain. Future implementations will
     * instead use indexed data through an offchain worker, not requiring
     * older states to be available.
     */
    generate_key_ownership_proof: RuntimeDescriptor<
      [set_id: bigint, authority_id: Binary],
      Binary | undefined
    >;
  };

  /**
   * API to interact with MMR pallet.
   */
  MmrApi: {
    /**
     * Return the on-chain MMR root hash.
     */
    mmr_root: RuntimeDescriptor<[], ResultPayload<Binary, MmrPrimitivesError>>;

    /**
     * Return the number of MMR blocks in the chain.
     */
    mmr_leaf_count: RuntimeDescriptor<[], ResultPayload<bigint, MmrPrimitivesError>>;

    /**
     * Generate MMR proof for a series of block numbers. If `best_known_block_number = Some(n)`,
     * use historical MMR state at given block height `n`. Else, use current MMR state.
     */
    generate_proof: RuntimeDescriptor<
      [block_numbers: Array<number>, best_known_block_number: number | undefined],
      ResultPayload<Anonymize<If93480lp8rssc>, MmrPrimitivesError>
    >;

    /**
     * Verify MMR proof against on-chain MMR for a batch of leaves.
     *
     * Note this function will use on-chain MMR root hash and check if the proof matches the hash.
     * Note, the leaves should be sorted such that corresponding leaves and leaf indices have the
     * same position in both the `leaves` vector and the `leaf_indices` vector contained in the [Proof]
     */
    verify_proof: RuntimeDescriptor<
      [
        leaves: Array<Binary>,
        proof: {
          leaf_indices: Anonymize<Iafqnechp3omqg>;
          leaf_count: bigint;
          items: Anonymize<Idhnf6rtqoslea>;
        },
      ],
      ResultPayload<undefined, MmrPrimitivesError>
    >;

    /**
     * Verify MMR proof against given root hash for a batch of leaves.
     *
     * Note this function does not require any on-chain storage - the
     * proof is verified against given MMR root hash.
     *
     * Note, the leaves should be sorted such that corresponding leaves and leaf indices have the
     * same position in both the `leaves` vector and the `leaf_indices` vector contained in the [Proof]
     */
    verify_proof_stateless: RuntimeDescriptor<
      [
        root: Binary,
        leaves: Array<Binary>,
        proof: {
          leaf_indices: Anonymize<Iafqnechp3omqg>;
          leaf_count: bigint;
          items: Anonymize<Idhnf6rtqoslea>;
        },
      ],
      ResultPayload<undefined, MmrPrimitivesError>
    >;
  };

  /**
   * APIs for integrating the GRANDPA finality gadget into runtimes.
   * This should be implemented on the runtime side.
   *
   * This is primarily used for negotiating authority-set changes for the
   * gadget. GRANDPA uses a signaling model of changing authority sets:
   * changes should be signaled with a delay of N blocks, and then automatically
   * applied in the runtime after those N blocks have passed.
   *
   * The consensus protocol will coordinate the handoff externally.
   */
  GrandpaApi: {
    /**
     * Get the current GRANDPA authorities and weights. This should not change except
     * for when changes are scheduled and the corresponding delay has passed.
     *
     * When called at block B, it will return the set of authorities that should be
     * used to finalize descendants of this block (B+1, B+2, ...). The block B itself
     * is finalized by the authorities from block B-1.
     */
    grandpa_authorities: RuntimeDescriptor<[], Array<Anonymize<I3iuggguvi9njj>>>;

    /**
     * Submits an unsigned extrinsic to report an equivocation. The caller
     * must provide the equivocation proof and a key ownership proof
     * (should be obtained using `generate_key_ownership_proof`). The
     * extrinsic will be unsigned and should only be accepted for local
     * authorship (not to be broadcast to the network). This method returns
     * `None` when creation of the extrinsic fails, e.g. if equivocation
     * reporting is disabled for the given runtime (i.e. this method is
     * hardcoded to return `None`). Only useful in an offchain context.
     */
    submit_report_equivocation_unsigned_extrinsic: RuntimeDescriptor<
      [
        equivocation_proof: { set_id: bigint; equivocation: GrandpaEquivocation },
        key_owner_proof: Binary,
      ],
      boolean
    >;

    /**
     * Generates a proof of key ownership for the given authority in the
     * given set. An example usage of this module is coupled with the
     * session historical module to prove that a given authority key is
     * tied to a given staking identity during a specific session. Proofs
     * of key ownership are necessary for submitting equivocation reports.
     * NOTE: even though the API takes a `set_id` as parameter the current
     * implementations ignore this parameter and instead rely on this
     * method being called at the correct block height, i.e. any point at
     * which the given set id is live on-chain. Future implementations will
     * instead use indexed data through an offchain worker, not requiring
     * older states to be available.
     */
    generate_key_ownership_proof: RuntimeDescriptor<
      [set_id: bigint, authority_id: Binary],
      Binary | undefined
    >;

    /**
     * Get current GRANDPA authority set id.
     */
    current_set_id: RuntimeDescriptor<[], bigint>;
  };

  /**
   * API necessary for block authorship with BABE.
   */
  BabeApi: {
    /**
     * Return the configuration for BABE.
     */
    configuration: RuntimeDescriptor<
      [],
      {
        slot_duration: bigint;
        epoch_length: bigint;
        c: Anonymize<I2j729bmgsdiuo>;
        authorities: Anonymize<I2qinct8jq4bqe>;
        randomness: Binary;
        allowed_slots: BabeAllowedSlots;
      }
    >;

    /**
     * Returns the slot that started the current epoch.
     */
    current_epoch_start: RuntimeDescriptor<[], bigint>;

    /**
     * Returns information regarding the current epoch.
     */
    current_epoch: RuntimeDescriptor<
      [],
      {
        epoch_index: bigint;
        start_slot: bigint;
        duration: bigint;
        authorities: Anonymize<I2qinct8jq4bqe>;
        randomness: Binary;
        config: Anonymize<Idkva8q2m9meg0>;
      }
    >;

    /**
     * Returns information regarding the next epoch (which was already
     * previously announced).
     */
    next_epoch: RuntimeDescriptor<
      [],
      {
        epoch_index: bigint;
        start_slot: bigint;
        duration: bigint;
        authorities: Anonymize<I2qinct8jq4bqe>;
        randomness: Binary;
        config: Anonymize<Idkva8q2m9meg0>;
      }
    >;

    /**
     * Generates a proof of key ownership for the given authority in the
     * current epoch. An example usage of this module is coupled with the
     * session historical module to prove that a given authority key is
     * tied to a given staking identity during a specific session. Proofs
     * of key ownership are necessary for submitting equivocation reports.
     * NOTE: even though the API takes a `slot` as parameter the current
     * implementations ignores this parameter and instead relies on this
     * method being called at the correct block height, i.e. any point at
     * which the epoch for the given slot is live on-chain. Future
     * implementations will instead use indexed data through an offchain
     * worker, not requiring older states to be available.
     */
    generate_key_ownership_proof: RuntimeDescriptor<
      [slot: bigint, authority_id: Binary],
      Binary | undefined
    >;

    /**
     * Submits an unsigned extrinsic to report an equivocation. The caller
     * must provide the equivocation proof and a key ownership proof
     * (should be obtained using `generate_key_ownership_proof`). The
     * extrinsic will be unsigned and should only be accepted for local
     * authorship (not to be broadcast to the network). This method returns
     * `None` when creation of the extrinsic fails, e.g. if equivocation
     * reporting is disabled for the given runtime (i.e. this method is
     * hardcoded to return `None`). Only useful in an offchain context.
     */
    submit_report_equivocation_unsigned_extrinsic: RuntimeDescriptor<
      [
        equivocation_proof: {
          offender: Binary;
          slot: bigint;
          first_header: Anonymize<I6t1nedlt7mobn>;
          second_header: Anonymize<I6t1nedlt7mobn>;
        },
        key_owner_proof: Binary,
      ],
      boolean
    >;
  };

  /**
   * The authority discovery api.
   *
   * This api is used by the `client/authority-discovery` module to retrieve identifiers
   * of the current and next authority set.
   */
  AuthorityDiscoveryApi: {
    /**
     * Retrieve authority identifiers of the current and next authority set.
     */
    authorities: RuntimeDescriptor<[], Array<Binary>>;
  };

  /**
   * Session keys runtime api.
   */
  SessionKeys: {
    /**
     * Generate a set of session keys with optionally using the given seed.
     * The keys should be stored within the keystore exposed via runtime
     * externalities.
     *
     * The seed needs to be a valid `utf8` string.
     *
     * Returns the concatenated SCALE encoded public keys.
     */
    generate_session_keys: RuntimeDescriptor<[seed: Binary | undefined], Binary>;

    /**
     * Decode the given public session keys.
     *
     * Returns the list of public raw public keys + key type.
     */
    decode_session_keys: RuntimeDescriptor<
      [encoded: Binary],
      Anonymize<I4gkfq1hbsjrle> | undefined
    >;
  };

  /**
   * The API to query account nonce.
   */
  AccountNonceApi: {
    /**
     * Get current account nonce of given `AccountId`.
     */
    account_nonce: RuntimeDescriptor<[account: SS58String], number>;
  };

  /**

 */
  TransactionPaymentApi: {
    /**

 */
    query_info: RuntimeDescriptor<
      [uxt: Binary, len: number],
      { weight: Anonymize<I4q39t5hn830vp>; class: DispatchClass; partial_fee: bigint }
    >;

    /**

 */
    query_fee_details: RuntimeDescriptor<
      [uxt: Binary, len: number],
      { inclusion_fee: Anonymize<Id37fum600qfau>; tip: bigint }
    >;

    /**

 */
    query_weight_to_fee: RuntimeDescriptor<
      [weight: { ref_time: bigint; proof_size: bigint }],
      bigint
    >;

    /**

 */
    query_length_to_fee: RuntimeDescriptor<[length: number], bigint>;
  };

  /**
   * API useful for BEEFY light clients.
   */
  BeefyMmrApi: {
    /**
     * Return the currently active BEEFY authority set proof.
     */
    authority_set_proof: RuntimeDescriptor<
      [],
      { id: bigint; len: number; keyset_commitment: Binary }
    >;

    /**
     * Return the next/queued BEEFY authority set proof.
     */
    next_authority_set_proof: RuntimeDescriptor<
      [],
      { id: bigint; len: number; keyset_commitment: Binary }
    >;
  };

  /**
   * API to interact with GenesisConfig for the runtime
   */
  GenesisBuilder: {
    /**
     * Creates the default `GenesisConfig` and returns it as a JSON blob.
     *
     * This function instantiates the default `GenesisConfig` struct for the runtime and serializes it into a JSON
     * blob. It returns a `Vec<u8>` containing the JSON representation of the default `GenesisConfig`.
     */
    create_default_config: RuntimeDescriptor<[], Binary>;

    /**
     * Build `GenesisConfig` from a JSON blob not using any defaults and store it in the storage.
     *
     * This function deserializes the full `GenesisConfig` from the given JSON blob and puts it into the storage.
     * If the provided JSON blob is incorrect or incomplete or the deserialization fails, an error is returned.
     * It is recommended to log any errors encountered during the process.
     *
     * Please note that provided json blob must contain all `GenesisConfig` fields, no defaults will be used.
     */
    build_config: RuntimeDescriptor<[json: Binary], ResultPayload<undefined, string>>;
  };
};
export const apis: IRuntimeCalls = {
  Core: {
    version: runtime_Core_version,
    execute_block: runtime_Core_execute_block,
    initialize_block: runtime_Core_initialize_block,
  },
  Metadata: {
    metadata: runtime_Metadata_metadata,
    metadata_at_version: runtime_Metadata_metadata_at_version,
    metadata_versions: runtime_Metadata_metadata_versions,
  },
  BlockBuilder: {
    apply_extrinsic: runtime_BlockBuilder_apply_extrinsic,
    finalize_block: runtime_BlockBuilder_finalize_block,
    inherent_extrinsics: runtime_BlockBuilder_inherent_extrinsics,
    check_inherents: runtime_BlockBuilder_check_inherents,
  },
  TaggedTransactionQueue: {
    validate_transaction: runtime_TaggedTransactionQueue_validate_transaction,
  },
  OffchainWorkerApi: { offchain_worker: runtime_OffchainWorkerApi_offchain_worker },
  ParachainHost: {
    validators: runtime_ParachainHost_validators,
    validator_groups: runtime_ParachainHost_validator_groups,
    availability_cores: runtime_ParachainHost_availability_cores,
    persisted_validation_data: runtime_ParachainHost_persisted_validation_data,
    assumed_validation_data: runtime_ParachainHost_assumed_validation_data,
    check_validation_outputs: runtime_ParachainHost_check_validation_outputs,
    session_index_for_child: runtime_ParachainHost_session_index_for_child,
    validation_code: runtime_ParachainHost_validation_code,
    candidate_pending_availability: runtime_ParachainHost_candidate_pending_availability,
    candidate_events: runtime_ParachainHost_candidate_events,
    dmq_contents: runtime_ParachainHost_dmq_contents,
    inbound_hrmp_channels_contents: runtime_ParachainHost_inbound_hrmp_channels_contents,
    validation_code_by_hash: runtime_ParachainHost_validation_code_by_hash,
    on_chain_votes: runtime_ParachainHost_on_chain_votes,
    session_info: runtime_ParachainHost_session_info,
    submit_pvf_check_statement: runtime_ParachainHost_submit_pvf_check_statement,
    pvfs_require_precheck: runtime_ParachainHost_pvfs_require_precheck,
    validation_code_hash: runtime_ParachainHost_validation_code_hash,
    disputes: runtime_ParachainHost_disputes,
    session_executor_params: runtime_ParachainHost_session_executor_params,
    unapplied_slashes: runtime_ParachainHost_unapplied_slashes,
    key_ownership_proof: runtime_ParachainHost_key_ownership_proof,
    submit_report_dispute_lost: runtime_ParachainHost_submit_report_dispute_lost,
    minimum_backing_votes: runtime_ParachainHost_minimum_backing_votes,
    para_backing_state: runtime_ParachainHost_para_backing_state,
    async_backing_params: runtime_ParachainHost_async_backing_params,
    disabled_validators: runtime_ParachainHost_disabled_validators,
    node_features: runtime_ParachainHost_node_features,
    approval_voting_params: runtime_ParachainHost_approval_voting_params,
  },
  BeefyApi: {
    beefy_genesis: runtime_BeefyApi_beefy_genesis,
    validator_set: runtime_BeefyApi_validator_set,
    submit_report_equivocation_unsigned_extrinsic:
      runtime_BeefyApi_submit_report_equivocation_unsigned_extrinsic,
    generate_key_ownership_proof: runtime_BeefyApi_generate_key_ownership_proof,
  },
  MmrApi: {
    mmr_root: runtime_MmrApi_mmr_root,
    mmr_leaf_count: runtime_MmrApi_mmr_leaf_count,
    generate_proof: runtime_MmrApi_generate_proof,
    verify_proof: runtime_MmrApi_verify_proof,
    verify_proof_stateless: runtime_MmrApi_verify_proof_stateless,
  },
  GrandpaApi: {
    grandpa_authorities: runtime_GrandpaApi_grandpa_authorities,
    submit_report_equivocation_unsigned_extrinsic:
      runtime_GrandpaApi_submit_report_equivocation_unsigned_extrinsic,
    generate_key_ownership_proof: runtime_GrandpaApi_generate_key_ownership_proof,
    current_set_id: runtime_GrandpaApi_current_set_id,
  },
  BabeApi: {
    configuration: runtime_BabeApi_configuration,
    current_epoch_start: runtime_BabeApi_current_epoch_start,
    current_epoch: runtime_BabeApi_current_epoch,
    next_epoch: runtime_BabeApi_next_epoch,
    generate_key_ownership_proof: runtime_BabeApi_generate_key_ownership_proof,
    submit_report_equivocation_unsigned_extrinsic:
      runtime_BabeApi_submit_report_equivocation_unsigned_extrinsic,
  },
  AuthorityDiscoveryApi: { authorities: runtime_AuthorityDiscoveryApi_authorities },
  SessionKeys: {
    generate_session_keys: runtime_SessionKeys_generate_session_keys,
    decode_session_keys: runtime_SessionKeys_decode_session_keys,
  },
  AccountNonceApi: { account_nonce: runtime_AccountNonceApi_account_nonce },
  TransactionPaymentApi: {
    query_info: runtime_TransactionPaymentApi_query_info,
    query_fee_details: runtime_TransactionPaymentApi_query_fee_details,
    query_weight_to_fee: runtime_TransactionPaymentApi_query_weight_to_fee,
    query_length_to_fee: runtime_TransactionPaymentApi_query_length_to_fee,
  },
  BeefyMmrApi: {
    authority_set_proof: runtime_BeefyMmrApi_authority_set_proof,
    next_authority_set_proof: runtime_BeefyMmrApi_next_authority_set_proof,
  },
  GenesisBuilder: {
    create_default_config: runtime_GenesisBuilder_create_default_config,
    build_config: runtime_GenesisBuilder_build_config,
  },
};

type IAsset = PlainDescriptor<void>;
const asset: IAsset = "" as IAsset;

type IDescriptors = { pallets: IPallets; apis: IRuntimeCalls; asset: IAsset };
const _allDescriptors: IDescriptors = { pallets, apis, asset };
export default _allDescriptors;

export type Queries = QueryFromDescriptors<IDescriptors>;
export type Calls = TxFromDescriptors<IDescriptors>;
export type Events = EventsFromDescriptors<IDescriptors>;
export type Errors = ErrorsFromDescriptors<IDescriptors>;
export type Constants = ConstFromDescriptors<IDescriptors>;
