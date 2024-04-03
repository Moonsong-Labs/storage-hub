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

type Idus5q78qvra6h = Array<Anonymize<Idvbs8vg3olusq>>;

type Idvbs8vg3olusq = {
  phase: Phase;
  event: Anonymize<I3fq1cs56r1k62>;
  topics: Anonymize<Idhnf6rtqoslea>;
};

export type Phase = Enum<
  | { type: "ApplyExtrinsic"; value: Anonymize<number> }
  | { type: "Finalization"; value: undefined }
  | { type: "Initialization"; value: undefined }
>;
export const Phase = _Enum as unknown as GetEnum<Phase>;

type I3fq1cs56r1k62 = AnonymousEnum<{
  System: Anonymize<PalletEvent>;
  ParachainSystem: Anonymize<Iav0g2u30ljnec>;
  Balances: Anonymize<BalancesEvent>;
  TransactionPayment: Anonymize<TransactionPaymentEvent>;
  Sudo: Anonymize<SudoEvent>;
  CollatorSelection: Anonymize<I4srakrmf0fspo>;
  Session: Anonymize<SessionEvent>;
  XcmpQueue: Anonymize<I2uos02bc7q3ed>;
  PolkadotXcm: Anonymize<I5f7pfeevv47ad>;
  CumulusXcm: Anonymize<I8l8o4l0arhl3h>;
  MessageQueue: Anonymize<I7mocdau0ca1md>;
  Providers: Anonymize<I6vnffms57hk75>;
  FileSystem: Anonymize<Idv54hgcrerpu2>;
  ProofsDealer: Anonymize<I93ejil83dpq82>;
}>;

export type PalletEvent = Enum<
  | { type: "ExtrinsicSuccess"; value: Anonymize<Iede1ukavoderd> }
  | { type: "ExtrinsicFailed"; value: Anonymize<Iennefu6o2bgdm> }
  | { type: "CodeUpdated"; value: undefined }
  | { type: "NewAccount"; value: Anonymize<Icbccs0ug47ilf> }
  | { type: "KilledAccount"; value: Anonymize<Icbccs0ug47ilf> }
  | { type: "Remarked"; value: Anonymize<Ieob37pbjnvmkj> }
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

type Iav0g2u30ljnec = AnonymousEnum<{
  ValidationFunctionStored: undefined;
  ValidationFunctionApplied: Anonymize<Idd7hd99u0ho0n>;
  ValidationFunctionDiscarded: undefined;
  UpgradeAuthorized: Anonymize<I6a5n5ij3gomuo>;
  DownwardMessagesReceived: Anonymize<Iafscmv8tjf0ou>;
  DownwardMessagesProcessed: Anonymize<I7a3a6ua4hud3s>;
  UpwardMessageSent: Anonymize<I4n7056p1k6c8b>;
}>;

type Idd7hd99u0ho0n = { relay_chain_block_num: number };

type I6a5n5ij3gomuo = { code_hash: Binary };

type Iafscmv8tjf0ou = { count: number };

type I7a3a6ua4hud3s = { weight_used: Anonymize<I4q39t5hn830vp>; dmq_head: Binary };

type I4n7056p1k6c8b = { message_hash: Anonymize<I17k3ujudqd5df> };

type I17k3ujudqd5df = Binary | undefined;

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

export type TransactionPaymentEvent = Enum<{
  type: "TransactionFeePaid";
  value: Anonymize<Ier2cke86dqbr2>;
}>;
export const TransactionPaymentEvent = _Enum as unknown as GetEnum<TransactionPaymentEvent>;

type Ier2cke86dqbr2 = { who: SS58String; actual_fee: bigint; tip: bigint };

export type SudoEvent = Enum<
  | { type: "Sudid"; value: Anonymize<I331o7t2g0ooi9> }
  | { type: "KeyChanged"; value: Anonymize<I5rtkmhm2dng4u> }
  | { type: "KeyRemoved"; value: undefined }
  | { type: "SudoAsDone"; value: Anonymize<I331o7t2g0ooi9> }
>;
export const SudoEvent = _Enum as unknown as GetEnum<SudoEvent>;

type I331o7t2g0ooi9 = { sudo_result: Anonymize<Idtdr91jmq5g4i> };

type Idtdr91jmq5g4i = ResultPayload<undefined, DispatchError>;

type I5rtkmhm2dng4u = { old: Anonymize<Ihfphjolmsqq1>; new: SS58String };

type Ihfphjolmsqq1 = SS58String | undefined;

type I4srakrmf0fspo = AnonymousEnum<{
  NewInvulnerables: Anonymize<I39t01nnod9109>;
  InvulnerableAdded: Anonymize<I6v8sm60vvkmk7>;
  InvulnerableRemoved: Anonymize<I6v8sm60vvkmk7>;
  NewDesiredCandidates: Anonymize<I1qmtmbe5so8r3>;
  NewCandidacyBond: Anonymize<Ih99m6ehpcar7>;
  CandidateAdded: Anonymize<Idgorhsbgdq2ap>;
  CandidateBondUpdated: Anonymize<Idgorhsbgdq2ap>;
  CandidateRemoved: Anonymize<I6v8sm60vvkmk7>;
  CandidateReplaced: Anonymize<I9ubb2kqevnu6t>;
  InvalidInvulnerableSkipped: Anonymize<I6v8sm60vvkmk7>;
}>;

type I39t01nnod9109 = { invulnerables: Anonymize<Ia2lhg7l2hilo3> };

type Ia2lhg7l2hilo3 = Array<SS58String>;

type I6v8sm60vvkmk7 = { account_id: SS58String };

type I1qmtmbe5so8r3 = { desired_candidates: number };

type Ih99m6ehpcar7 = { bond_amount: bigint };

type Idgorhsbgdq2ap = { account_id: SS58String; deposit: bigint };

type I9ubb2kqevnu6t = { old: SS58String; new: SS58String; deposit: bigint };

export type SessionEvent = Enum<{ type: "NewSession"; value: Anonymize<I2hq50pu2kdjpo> }>;
export const SessionEvent = _Enum as unknown as GetEnum<SessionEvent>;

type I2hq50pu2kdjpo = { session_index: number };

type I2uos02bc7q3ed = AnonymousEnum<{ XcmpMessageSent: Anonymize<I2vo9trn8nhllu> }>;

type I2vo9trn8nhllu = { message_hash: Binary };

type I5f7pfeevv47ad = AnonymousEnum<{
  Attempted: Anonymize<I4e7dkr4hrus3u>;
  Sent: Anonymize<Icr67tdr3h1l9n>;
  UnexpectedResponse: Anonymize<Idrsgrbh5b6rje>;
  ResponseReady: Anonymize<I5s81678scdptl>;
  Notified: Anonymize<I2uqmls7kcdnii>;
  NotifyOverweight: Anonymize<Idg69klialbkb8>;
  NotifyDispatchError: Anonymize<I2uqmls7kcdnii>;
  NotifyDecodeFailed: Anonymize<I2uqmls7kcdnii>;
  InvalidResponder: Anonymize<Idje8f9lv4sogt>;
  InvalidResponderVersion: Anonymize<Idrsgrbh5b6rje>;
  ResponseTaken: Anonymize<I30pg328m00nr3>;
  AssetsTrapped: Anonymize<I2pd6nni2u8392>;
  VersionChangeNotified: Anonymize<I6s4eucqd88i6a>;
  SupportedVersionChanged: Anonymize<Ie9it7tqcnjnfj>;
  NotifyTargetSendFail: Anonymize<I5lfvfuumat5pq>;
  NotifyTargetMigrationFail: Anonymize<Iqsl7ltbtjavb>;
  InvalidQuerierVersion: Anonymize<Idrsgrbh5b6rje>;
  InvalidQuerier: Anonymize<Iev28bbfu8eghg>;
  VersionNotifyStarted: Anonymize<I14amtmubrpgc8>;
  VersionNotifyRequested: Anonymize<I14amtmubrpgc8>;
  VersionNotifyUnrequested: Anonymize<I14amtmubrpgc8>;
  FeesPaid: Anonymize<I4tgpelgtlb6pi>;
  AssetsClaimed: Anonymize<I2pd6nni2u8392>;
}>;

type I4e7dkr4hrus3u = { outcome: XcmV3TraitsOutcome };

export type XcmV3TraitsOutcome = Enum<
  | { type: "Complete"; value: Anonymize<I4q39t5hn830vp> }
  | { type: "Incomplete"; value: Anonymize<Ilcvm3kc2hvtg> }
  | { type: "Error"; value: Anonymize<XcmV3TraitsError> }
>;
export const XcmV3TraitsOutcome = _Enum as unknown as GetEnum<XcmV3TraitsOutcome>;

type Ilcvm3kc2hvtg = [Anonymize<I4q39t5hn830vp>, XcmV3TraitsError];

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

type Icr67tdr3h1l9n = {
  origin: Anonymize<Ie897ubj3a1vaq>;
  destination: Anonymize<Ie897ubj3a1vaq>;
  message: Anonymize<I50ghg3dhe8sh3>;
  message_id: Binary;
};

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

type I2pdjq1umlp617 = Array<Anonymize<Isj6qus1lv5t9>>;

type Isj6qus1lv5t9 = { id: XcmV3MultiassetAssetId; fun: XcmV3MultiassetFungibility };

export type XcmV3MultiassetAssetId = Enum<
  | { type: "Concrete"; value: Anonymize<Ie897ubj3a1vaq> }
  | { type: "Abstract"; value: Anonymize<Binary> }
>;
export const XcmV3MultiassetAssetId = _Enum as unknown as GetEnum<XcmV3MultiassetAssetId>;

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

type Ifcbfhsum5pdt8 = {
  query_id: bigint;
  response: XcmV3Response;
  max_weight: Anonymize<I4q39t5hn830vp>;
  querier: Anonymize<I189rbbmttkf8v>;
};

export type XcmV3Response = Enum<
  | { type: "Null"; value: undefined }
  | { type: "Assets"; value: Anonymize<I2pdjq1umlp617> }
  | { type: "ExecutionResult"; value: Anonymize<I8j770n2arfq59> }
  | { type: "Version"; value: Anonymize<number> }
  | { type: "PalletsInfo"; value: Anonymize<I599u7h20b52at> }
  | { type: "DispatchResult"; value: Anonymize<XcmV3MaybeErrorCode> }
>;
export const XcmV3Response = _Enum as unknown as GetEnum<XcmV3Response>;

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

type I189rbbmttkf8v = Anonymize<Ie897ubj3a1vaq> | undefined;

type Iciun0t2v4pn9s = { assets: Anonymize<I2pdjq1umlp617>; beneficiary: Anonymize<Ie897ubj3a1vaq> };

type I4gomd50gf1sdo = {
  assets: Anonymize<I2pdjq1umlp617>;
  dest: Anonymize<Ie897ubj3a1vaq>;
  xcm: Anonymize<I50ghg3dhe8sh3>;
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

export type XcmV2MultiassetWildFungibility = Enum<
  { type: "Fungible"; value: undefined } | { type: "NonFungible"; value: undefined }
>;
export const XcmV2MultiassetWildFungibility =
  _Enum as unknown as GetEnum<XcmV2MultiassetWildFungibility>;

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

export type XcmV3WeightLimit = Enum<
  { type: "Unlimited"; value: undefined } | { type: "Limited"; value: Anonymize<I4q39t5hn830vp> }
>;
export const XcmV3WeightLimit = _Enum as unknown as GetEnum<XcmV3WeightLimit>;

type Iatoh41hlqpeff = { assets: Anonymize<I2pdjq1umlp617>; ticket: Anonymize<Ie897ubj3a1vaq> };

type Ieprdqqu7ildvr = { query_id: bigint; max_response_weight: Anonymize<I4q39t5hn830vp> };

type I9o6j30dnhmlg9 = { module_name: Binary; response_info: Anonymize<I8iu73oulmbcl6> };

type Id7mf37dkpgfjs = {
  index: number;
  name: Binary;
  module_name: Binary;
  crate_major: number;
  min_crate_minor: number;
};

type Iatj898em490l6 = {
  network: XcmV4JunctionNetworkId;
  destination: XcmV3Junctions;
  xcm: Anonymize<I50ghg3dhe8sh3>;
};

type Ifgane16e7gi0u = { asset: Anonymize<Isj6qus1lv5t9>; unlocker: Anonymize<Ie897ubj3a1vaq> };

type Ibs9ci5muat0jn = { asset: Anonymize<Isj6qus1lv5t9>; target: Anonymize<Ie897ubj3a1vaq> };

type I9pln3upoovp5l = { asset: Anonymize<Isj6qus1lv5t9>; owner: Anonymize<Ie897ubj3a1vaq> };

type Ibqteslvkvmmol = { asset: Anonymize<Isj6qus1lv5t9>; locker: Anonymize<Ie897ubj3a1vaq> };

type I4nae9rsql8fa7 = { jit_withdraw: boolean };

type I8b0u1467piq8o = { weight_limit: XcmV3WeightLimit; check_origin: Anonymize<I189rbbmttkf8v> };

type Idrsgrbh5b6rje = { origin: Anonymize<Ie897ubj3a1vaq>; query_id: bigint };

type I5s81678scdptl = { query_id: bigint; response: XcmV3Response };

type I2uqmls7kcdnii = { query_id: bigint; pallet_index: number; call_index: number };

type Idg69klialbkb8 = {
  query_id: bigint;
  pallet_index: number;
  call_index: number;
  actual_weight: Anonymize<I4q39t5hn830vp>;
  max_budgeted_weight: Anonymize<I4q39t5hn830vp>;
};

type Idje8f9lv4sogt = {
  origin: Anonymize<Ie897ubj3a1vaq>;
  query_id: bigint;
  expected_location: Anonymize<I189rbbmttkf8v>;
};

type I30pg328m00nr3 = { query_id: bigint };

type I2pd6nni2u8392 = {
  hash: Binary;
  origin: Anonymize<Ie897ubj3a1vaq>;
  assets: Anonymize<I2tnkj3t3en8tf>;
};

type I2tnkj3t3en8tf = AnonymousEnum<{
  V2: Anonymize<Ia3ggl9eghkufh>;
  V3: Anonymize<I2pdjq1umlp617>;
}>;

type Ia3ggl9eghkufh = Array<Anonymize<I16mc4mv5bb0qd>>;

type I16mc4mv5bb0qd = { id: XcmV2MultiassetAssetId; fun: XcmV2MultiassetFungibility };

export type XcmV2MultiassetAssetId = Enum<
  | { type: "Concrete"; value: Anonymize<Ibki0d249v3ojt> }
  | { type: "Abstract"; value: Anonymize<Binary> }
>;
export const XcmV2MultiassetAssetId = _Enum as unknown as GetEnum<XcmV2MultiassetAssetId>;

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

type I6s4eucqd88i6a = {
  destination: Anonymize<Ie897ubj3a1vaq>;
  result: number;
  cost: Anonymize<I2pdjq1umlp617>;
  message_id: Binary;
};

type Ie9it7tqcnjnfj = { location: Anonymize<Ie897ubj3a1vaq>; version: number };

type I5lfvfuumat5pq = {
  location: Anonymize<Ie897ubj3a1vaq>;
  query_id: bigint;
  error: XcmV3TraitsError;
};

type Iqsl7ltbtjavb = { location: Anonymize<Ib29ie59v4nmjq>; query_id: bigint };

type Ib29ie59v4nmjq = AnonymousEnum<{
  V2: Anonymize<Ibki0d249v3ojt>;
  V3: Anonymize<Ie897ubj3a1vaq>;
}>;

type Iev28bbfu8eghg = {
  origin: Anonymize<Ie897ubj3a1vaq>;
  query_id: bigint;
  expected_querier: Anonymize<Ie897ubj3a1vaq>;
  maybe_actual_querier: Anonymize<I189rbbmttkf8v>;
};

type I14amtmubrpgc8 = {
  destination: Anonymize<Ie897ubj3a1vaq>;
  cost: Anonymize<I2pdjq1umlp617>;
  message_id: Binary;
};

type I4tgpelgtlb6pi = { paying: Anonymize<Ie897ubj3a1vaq>; fees: Anonymize<I2pdjq1umlp617> };

type I8l8o4l0arhl3h = AnonymousEnum<{
  InvalidFormat: Anonymize<Binary>;
  UnsupportedVersion: Anonymize<Binary>;
  ExecutedDownward: Anonymize<Id0ii3t0e6fgob>;
}>;

type Id0ii3t0e6fgob = [Binary, XcmV3TraitsOutcome];

type I7mocdau0ca1md = AnonymousEnum<{
  ProcessingFailed: Anonymize<I82vnvii3s8i35>;
  Processed: Anonymize<Idgsr5mj02rcf9>;
  OverweightEnqueued: Anonymize<I9l2k151rfodj4>;
  PageReaped: Anonymize<I6947e8h0ume2q>;
}>;

type I82vnvii3s8i35 = { id: Binary; origin: Anonymize<Ifqm8uoikppunt>; error: ProcessMessageError };

type Ifqm8uoikppunt = AnonymousEnum<{
  Here: undefined;
  Parent: undefined;
  Sibling: Anonymize<number>;
}>;

export type ProcessMessageError = Enum<
  | { type: "BadFormat"; value: undefined }
  | { type: "Corrupt"; value: undefined }
  | { type: "Unsupported"; value: undefined }
  | { type: "Overweight"; value: Anonymize<I4q39t5hn830vp> }
  | { type: "Yield"; value: undefined }
>;
export const ProcessMessageError = _Enum as unknown as GetEnum<ProcessMessageError>;

type Idgsr5mj02rcf9 = {
  id: Binary;
  origin: Anonymize<Ifqm8uoikppunt>;
  weight_used: Anonymize<I4q39t5hn830vp>;
  success: boolean;
};

type I9l2k151rfodj4 = {
  id: Binary;
  origin: Anonymize<Ifqm8uoikppunt>;
  page_index: number;
  message_index: number;
};

type I6947e8h0ume2q = { origin: Anonymize<Ifqm8uoikppunt>; index: number };

type I6vnffms57hk75 = AnonymousEnum<{
  MspSignUpSuccess: Anonymize<I5k3ihioq7rv8s>;
  BspSignUpSuccess: Anonymize<I6vdjrn3qfv9c0>;
  MspSignOffSuccess: Anonymize<I4cbvqmqadhrea>;
  BspSignOffSuccess: Anonymize<I4cbvqmqadhrea>;
  TotalDataChanged: Anonymize<I1lnbp13mvbupl>;
}>;

type I5k3ihioq7rv8s = {
  who: SS58String;
  multiaddresses: Anonymize<Itom7fk49o0c9>;
  capacity: number;
  value_prop: Anonymize<Ienf50imfp828o>;
};

type Itom7fk49o0c9 = Array<Binary>;

type Ienf50imfp828o = {
  identifier: Binary;
  data_limit: number;
  protocols: Anonymize<Itom7fk49o0c9>;
};

type I6vdjrn3qfv9c0 = {
  who: SS58String;
  multiaddresses: Anonymize<Itom7fk49o0c9>;
  capacity: number;
};

type I1lnbp13mvbupl = { who: SS58String; old_capacity: number; new_capacity: number };

type Idv54hgcrerpu2 = AnonymousEnum<{
  NewStorageRequest: Anonymize<Icrmmcf16le1rr>;
  AcceptedBspVolunteer: Anonymize<I8c7cnrjg7sfkc>;
  StorageRequestExpired: Anonymize<Ieg5outd74d62b>;
  StorageRequestRevoked: Anonymize<Ieg5outd74d62b>;
  BspStoppedStoring: Anonymize<Iddserqjgpfbdm>;
}>;

type Icrmmcf16le1rr = {
  who: SS58String;
  location: Binary;
  fingerprint: Binary;
  size: bigint;
  multiaddresses: Anonymize<Itom7fk49o0c9>;
};

type I8c7cnrjg7sfkc = {
  who: SS58String;
  location: Binary;
  fingerprint: Binary;
  multiaddresses: Anonymize<Itom7fk49o0c9>;
};

type Ieg5outd74d62b = { location: Binary };

type Iddserqjgpfbdm = { bsp: SS58String; file_key: Binary; owner: SS58String; location: Binary };

type I93ejil83dpq82 = AnonymousEnum<{
  NewChallenge: Anonymize<I4q3amn12h8qc9>;
  ProofRejected: Anonymize<If5kdet8babji3>;
  ProofAccepted: Anonymize<I9mls6vu7id41c>;
}>;

type I4q3amn12h8qc9 = { who: SS58String; key_challenged: Binary };

type If5kdet8babji3 = {
  provider: Binary;
  proof: Anonymize<Itom7fk49o0c9>;
  reason: Anonymize<Ifhhbbpbpeqis>;
};

type Ifhhbbpbpeqis = AnonymousEnum<{ RootMismatch: undefined; NotConsecutiveLeaves: undefined }>;

type I9mls6vu7id41c = { provider: Binary; proof: Anonymize<Itom7fk49o0c9> };

type Idhnf6rtqoslea = Array<Binary>;

type Iep4uo61810hfs = Array<Anonymize<I5g2vv0ckl2m8b>>;

type I5g2vv0ckl2m8b = [number, number];

type Ieniouoqkq4icf = { spec_version: number; spec_name: string };

type I6f7juk1vj8fiq = Array<Anonymize<I8ajtuet8esesv>>;

type I8ajtuet8esesv = {
  used_bandwidth: Anonymize<Ieafp1gui1o4cl>;
  para_head_hash: Anonymize<I17k3ujudqd5df>;
  consumed_go_ahead_signal: Anonymize<Ie1vdku2j6ccvj>;
};

type Ieafp1gui1o4cl = {
  ump_msg_count: number;
  ump_total_bytes: number;
  hrmp_outgoing: Anonymize<I68brng9hc4b57>;
};

type I68brng9hc4b57 = Array<Anonymize<I2hfpgo4vigap7>>;

type I2hfpgo4vigap7 = [number, Anonymize<I37lfg356jmoof>];

type I37lfg356jmoof = { msg_count: number; total_bytes: number };

type Ie1vdku2j6ccvj = PolkadotPrimitivesV5UpgradeGoAhead | undefined;

export type PolkadotPrimitivesV5UpgradeGoAhead = Enum<
  { type: "Abort"; value: undefined } | { type: "GoAhead"; value: undefined }
>;
export const PolkadotPrimitivesV5UpgradeGoAhead =
  _Enum as unknown as GetEnum<PolkadotPrimitivesV5UpgradeGoAhead>;

type Iamnn769l033b4 = {
  used_bandwidth: Anonymize<Ieafp1gui1o4cl>;
  hrmp_watermark: Anonymize<I4arjljr6dpflb>;
  consumed_go_ahead_signal: Anonymize<Ie1vdku2j6ccvj>;
};

type I4arjljr6dpflb = number | undefined;

type I5r8ef6aie125l = {
  parent_head: Binary;
  relay_parent_number: number;
  relay_parent_storage_root: Binary;
  max_pov_size: number;
};

type Id34bi0o1gnln9 = PolkadotPrimitivesV5UpgradeRestriction | undefined;

export type PolkadotPrimitivesV5UpgradeRestriction = Enum<{ type: "Present"; value: undefined }>;
export const PolkadotPrimitivesV5UpgradeRestriction =
  _Enum as unknown as GetEnum<PolkadotPrimitivesV5UpgradeRestriction>;

type I8gicq33tfu7j1 = {
  dmq_mqc_head: Binary;
  relay_dispatch_queue_remaining_capacity: Anonymize<I3j1v1c2btq4bd>;
  ingress_channels: Anonymize<I90nfahji0n33j>;
  egress_channels: Anonymize<I90nfahji0n33j>;
};

type I3j1v1c2btq4bd = { remaining_count: number; remaining_size: number };

type I90nfahji0n33j = Array<Anonymize<Iemudar0nobhvs>>;

type Iemudar0nobhvs = [number, Anonymize<I5q7ff8kblv2cn>];

type I5q7ff8kblv2cn = {
  max_capacity: number;
  max_total_size: number;
  max_message_size: number;
  msg_count: number;
  total_size: number;
  mqc_head: Anonymize<I17k3ujudqd5df>;
};

type I4iumukclgj8ej = {
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
};

type Iavuvfkop6318c = { max_candidate_depth: number; allowed_ancestry_len: number };

type I93ssha9egqq23 = Array<Anonymize<If89923vhoiaim>>;

type If89923vhoiaim = [number, Binary];

type I6r5cbv8ttrb09 = Array<Anonymize<I958l48g4qg5rf>>;

type I958l48g4qg5rf = { recipient: number; data: Binary };

type Ib3qt1mgvgmbgi = { code_hash: Binary; check_version: boolean };

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

type Iee2mjl65qu5r6 = Array<Anonymize<I66c0bbqofu6gm>>;

type I66c0bbqofu6gm = { id: Anonymize<I7qfn0q0ihc1dq>; amount: bigint };

type I7qfn0q0ihc1dq = AnonymousEnum<{ Providers: Anonymize<I8lpabnnpbti8o> }>;

type I8lpabnnpbti8o = AnonymousEnum<{
  StorageProviderDeposit: undefined;
  AnotherUnrelatedHold: undefined;
}>;

type I3fe7nt2ne6qtf = Array<Anonymize<I7qdm60946h5u9>>;

type I7qdm60946h5u9 = { id: undefined; amount: bigint };

export type TransactionPaymentReleases = Enum<
  { type: "V1Ancient"; value: undefined } | { type: "V2"; value: undefined }
>;
export const TransactionPaymentReleases = _Enum as unknown as GetEnum<TransactionPaymentReleases>;

type Ifi4da1gej1fri = Array<Anonymize<Iep1lmt6q3s6r3>>;

type Iep1lmt6q3s6r3 = { who: SS58String; deposit: bigint };

type I9our4clsjkim0 = Array<Anonymize<I73gble6tmb52f>>;

type I73gble6tmb52f = [SS58String, Binary];

type Icgljjb6j82uhn = Array<number>;

type I6cs1itejju2vv = [bigint, number];

type I1mse8p8gvh143 = Array<Anonymize<Ittnsbm78tol1>>;

type Ittnsbm78tol1 = {
  recipient: number;
  state: Anonymize<Iafdd71v7fmmtg>;
  signals_exist: boolean;
  first_index: number;
  last_index: number;
};

type Iafdd71v7fmmtg = AnonymousEnum<{ Ok: undefined; Suspended: undefined }>;

type Ifup3lg9ro8a0f = {
  suspend_threshold: number;
  drop_threshold: number;
  resume_threshold: number;
};

type I6bgp6t7a12jh7 = AnonymousEnum<{
  Pending: Anonymize<I4n9ble5dnecdr>;
  VersionNotifier: Anonymize<Idc4lam0e7aiet>;
  Ready: Anonymize<I3239o3gbno6s5>;
}>;

type I4n9ble5dnecdr = {
  responder: Anonymize<Ib29ie59v4nmjq>;
  maybe_match_querier: Anonymize<I6l00lh1u9a347>;
  maybe_notify: Anonymize<I34gtdjipdmjpt>;
  timeout: number;
};

type I6l00lh1u9a347 = Anonymize<Ib29ie59v4nmjq> | undefined;

type I34gtdjipdmjpt = Anonymize<I5g2vv0ckl2m8b> | undefined;

type Idc4lam0e7aiet = { origin: Anonymize<Ib29ie59v4nmjq>; is_active: boolean };

type I3239o3gbno6s5 = { response: Anonymize<Ia44h320fv91e4>; at: number };

type Ia44h320fv91e4 = AnonymousEnum<{ V2: Anonymize<XcmV2Response>; V3: Anonymize<XcmV3Response> }>;

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

type I9cri6nt5h7m8e = [number, Anonymize<Ib29ie59v4nmjq>];

type I7vlvrrl2pnbgk = [bigint, Anonymize<I4q39t5hn830vp>, number];

type I9kuv6u16da6dg = Array<Anonymize<I82i8h7h2mvtd5>>;

type I82i8h7h2mvtd5 = [Anonymize<Ib29ie59v4nmjq>, number];

export type XcmPalletVersionMigrationStage = Enum<
  | { type: "MigrateSupportedVersion"; value: undefined }
  | { type: "MigrateVersionNotifiers"; value: undefined }
  | { type: "NotifyCurrentTargets"; value: Anonymize<Iabpgqcjikia83> }
  | { type: "MigrateAndNotifyOldTargets"; value: undefined }
>;
export const XcmPalletVersionMigrationStage =
  _Enum as unknown as GetEnum<XcmPalletVersionMigrationStage>;

type Iabpgqcjikia83 = Binary | undefined;

type Ie0vbv530ausfn = {
  amount: bigint;
  owner: Anonymize<Ib29ie59v4nmjq>;
  locker: Anonymize<Ib29ie59v4nmjq>;
  consumers: Anonymize<I48jka0f0ufl6q>;
};

type I48jka0f0ufl6q = Array<Anonymize<I2jndntq8n8661>>;

type I2jndntq8n8661 = [undefined, bigint];

type I7np5v1f7nq84i = [number, SS58String, Anonymize<I9hdbmmgal228m>];

type I9hdbmmgal228m = AnonymousEnum<{ V3: Anonymize<XcmV3MultiassetAssetId> }>;

type I60c4grt23726r = Array<Anonymize<Ifuuq590aavd5n>>;

type Ifuuq590aavd5n = [bigint, Anonymize<Ib29ie59v4nmjq>];

type I68vn2neu6fddv = {
  begin: number;
  end: number;
  count: number;
  ready_neighbours: Anonymize<If4d9hsuqsl01i>;
  message_count: bigint;
  size: bigint;
};

type If4d9hsuqsl01i = Anonymize<Icdc7rvj8e0og7> | undefined;

type Icdc7rvj8e0og7 = { prev: Anonymize<Ifqm8uoikppunt>; next: Anonymize<Ifqm8uoikppunt> };

type I53esa2ms463bk = {
  remaining: number;
  remaining_size: number;
  first_index: number;
  first: number;
  last: number;
  heap: Binary;
};

type I3h754n50bpufu = [Anonymize<Ifqm8uoikppunt>, number];

type Ie5fhu8bftngbn = {
  buckets: Anonymize<I45d79rdcadrnn>;
  capacity: number;
  data_used: number;
  multiaddresses: Anonymize<Itom7fk49o0c9>;
  value_prop: Anonymize<Ienf50imfp828o>;
};

type I45d79rdcadrnn = Array<Anonymize<Iabcds2c2si8d>>;

type Iabcds2c2si8d = { root: Binary; user_id: SS58String; msp_id: Binary };

type I352diutbubgrm = {
  capacity: number;
  data_used: number;
  multiaddresses: Anonymize<Itom7fk49o0c9>;
  root: Binary;
};

type Ida98omgna57r1 = {
  requested_at: number;
  owner: SS58String;
  fingerprint: Binary;
  size: bigint;
  user_multiaddresses: Anonymize<Itom7fk49o0c9>;
  data_server_sps: Anonymize<Ia2lhg7l2hilo3>;
  bsps_required: number;
  bsps_confirmed: number;
};

type I4p8l84tk038s = [Binary, SS58String];

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

type Iasb8k6ash5mjn = Anonymize<I4q39t5hn830vp> | undefined;

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

export type SystemPalletCall = Enum<
  | { type: "remark"; value: Anonymize<I8ofcg5rbj0g2c> }
  | { type: "set_heap_pages"; value: Anonymize<I4adgbll7gku4i> }
  | { type: "set_code"; value: Anonymize<I6pjjpfvhvcfru> }
  | { type: "set_code_without_checks"; value: Anonymize<I6pjjpfvhvcfru> }
  | { type: "set_storage"; value: Anonymize<I8qrhskdehbu57> }
  | { type: "kill_storage"; value: Anonymize<I39uah9nss64h9> }
  | { type: "kill_prefix"; value: Anonymize<Ik64dknsq7k08> }
  | { type: "remark_with_event"; value: Anonymize<I8ofcg5rbj0g2c> }
>;
export const SystemPalletCall = _Enum as unknown as GetEnum<SystemPalletCall>;

type I8ofcg5rbj0g2c = { remark: Binary };

type I4adgbll7gku4i = { pages: bigint };

type I6pjjpfvhvcfru = { code: Binary };

type I8qrhskdehbu57 = { items: Anonymize<I5g1ftt6bt65bl> };

type I5g1ftt6bt65bl = Array<Anonymize<Ief9tkec59fktv>>;

type Ief9tkec59fktv = [Binary, Binary];

type I39uah9nss64h9 = { keys: Anonymize<Itom7fk49o0c9> };

type Ik64dknsq7k08 = { prefix: Binary; subkeys: number };

type Ia0jlnena5ajog = AnonymousEnum<{
  set_validation_data: Anonymize<I68js79djhsbni>;
  sudo_send_upward_message: Anonymize<Ifpj261e8s63m3>;
  authorize_upgrade: Anonymize<Ib3qt1mgvgmbgi>;
  enact_authorized_upgrade: Anonymize<I6pjjpfvhvcfru>;
}>;

type I68js79djhsbni = { data: Anonymize<Icj9r7l64kc5ku> };

type Icj9r7l64kc5ku = {
  validation_data: Anonymize<I5r8ef6aie125l>;
  relay_chain_state: Anonymize<Itom7fk49o0c9>;
  downward_messages: Anonymize<I6ljjd4b5fa4ov>;
  horizontal_messages: Anonymize<I2pf0b05mc7sdr>;
};

type I6ljjd4b5fa4ov = Array<Anonymize<I60847k37jfcc6>>;

type I60847k37jfcc6 = { sent_at: number; msg: Binary };

type I2pf0b05mc7sdr = Array<Anonymize<I9hvej6h53dqj0>>;

type I9hvej6h53dqj0 = [number, Anonymize<Iev3u09i2vqn93>];

type Iev3u09i2vqn93 = Array<Anonymize<I409qo0sfkbh16>>;

type I409qo0sfkbh16 = { sent_at: number; data: Binary };

type Ifpj261e8s63m3 = { message: Binary };

export type TimestampPalletCall = Enum<{ type: "set"; value: Anonymize<Idcr6u6361oad9> }>;
export const TimestampPalletCall = _Enum as unknown as GetEnum<TimestampPalletCall>;

type Idcr6u6361oad9 = { now: bigint };

type Ibf8j84ii3a3kr = AnonymousEnum<{
  transfer_allow_death: Anonymize<Ien6q0lasi0m7i>;
  force_transfer: Anonymize<Icacgruoo9j3r2>;
  transfer_keep_alive: Anonymize<Ien6q0lasi0m7i>;
  transfer_all: Anonymize<I7dgmo7im9hljo>;
  force_unreserve: Anonymize<Iargojp1sv9icj>;
  upgrade_accounts: Anonymize<Ibmr18suc9ikh9>;
  force_set_balance: Anonymize<Ie0io91hk7pejj>;
}>;

type Ien6q0lasi0m7i = { dest: MultiAddress; value: bigint };

export type MultiAddress = Enum<
  | { type: "Id"; value: Anonymize<SS58String> }
  | { type: "Index"; value: Anonymize<number> }
  | { type: "Raw"; value: Anonymize<Binary> }
  | { type: "Address32"; value: Anonymize<Binary> }
  | { type: "Address20"; value: Anonymize<Binary> }
>;
export const MultiAddress = _Enum as unknown as GetEnum<MultiAddress>;

type Icacgruoo9j3r2 = { source: MultiAddress; dest: MultiAddress; value: bigint };

type I7dgmo7im9hljo = { dest: MultiAddress; keep_alive: boolean };

type Iargojp1sv9icj = { who: MultiAddress; amount: bigint };

type Ibmr18suc9ikh9 = { who: Anonymize<Ia2lhg7l2hilo3> };

type Ie0io91hk7pejj = { who: MultiAddress; new_free: bigint };

type Iam913892vifu6 = AnonymousEnum<{
  sudo: Anonymize<I95rtegihqfhrh>;
  sudo_unchecked_weight: Anonymize<Ifq9uub37mee7a>;
  set_key: Anonymize<Icnonnse26sae7>;
  sudo_as: Anonymize<I4h5fkjfra8jm3>;
  remove_key: undefined;
}>;

type I95rtegihqfhrh = { call: Anonymize<Iupi52pl09tgg> };

type Iupi52pl09tgg = AnonymousEnum<{
  System: Anonymize<SystemPalletCall>;
  ParachainSystem: Anonymize<Ia0jlnena5ajog>;
  Timestamp: Anonymize<TimestampPalletCall>;
  ParachainInfo: Anonymize<undefined>;
  Balances: Anonymize<Ibf8j84ii3a3kr>;
  Sudo: Anonymize<Iam913892vifu6>;
  CollatorSelection: Anonymize<I6ggjare8v1go5>;
  Session: Anonymize<I3v8vq7j9grsdj>;
  XcmpQueue: Anonymize<I286uete0pvcbe>;
  PolkadotXcm: Anonymize<I3br2bgla1bs2h>;
  CumulusXcm: Anonymize<undefined>;
  MessageQueue: Anonymize<I8lmlccfrohcqg>;
  Providers: Anonymize<I9jhevh1bis85g>;
  FileSystem: Anonymize<I8u4nbk1d32u7q>;
  ProofsDealer: Anonymize<Iaoc2q2c87hkb1>;
}>;

type I6ggjare8v1go5 = AnonymousEnum<{
  set_invulnerables: Anonymize<Ifccifqltb5obi>;
  set_desired_candidates: Anonymize<Iadtsfv699cq8b>;
  set_candidacy_bond: Anonymize<Ialpmgmhr3gk5r>;
  register_as_candidate: undefined;
  leave_intent: undefined;
  add_invulnerable: Anonymize<I4cbvqmqadhrea>;
  remove_invulnerable: Anonymize<I4cbvqmqadhrea>;
  update_bond: Anonymize<I3sdol54kg5jaq>;
  take_candidate_slot: Anonymize<I8fougodaj6di6>;
}>;

type Ifccifqltb5obi = { new: Anonymize<Ia2lhg7l2hilo3> };

type Iadtsfv699cq8b = { max: number };

type Ialpmgmhr3gk5r = { bond: bigint };

type I3sdol54kg5jaq = { new_deposit: bigint };

type I8fougodaj6di6 = { deposit: bigint; target: SS58String };

type I3v8vq7j9grsdj = AnonymousEnum<{ set_keys: Anonymize<Ivojoo8sbcs0m>; purge_keys: undefined }>;

type Ivojoo8sbcs0m = { keys: Binary; proof: Binary };

type I286uete0pvcbe = AnonymousEnum<{
  suspend_xcm_execution: undefined;
  resume_xcm_execution: undefined;
  update_suspend_threshold: Anonymize<I3vh014cqgmrfd>;
  update_drop_threshold: Anonymize<I3vh014cqgmrfd>;
  update_resume_threshold: Anonymize<I3vh014cqgmrfd>;
}>;

type I3vh014cqgmrfd = { new: number };

type I3br2bgla1bs2h = AnonymousEnum<{
  send: Anonymize<I6j18p941ujf3v>;
  teleport_assets: Anonymize<Idqjhq57s7jh4k>;
  reserve_transfer_assets: Anonymize<Idqjhq57s7jh4k>;
  execute: Anonymize<I8875l87smbh8d>;
  force_xcm_version: Anonymize<Ie9it7tqcnjnfj>;
  force_default_xcm_version: Anonymize<Ic76kfh5ebqkpl>;
  force_subscribe_version_notify: Anonymize<Idmol6ivsgrnjg>;
  force_unsubscribe_version_notify: Anonymize<Idmol6ivsgrnjg>;
  limited_reserve_transfer_assets: Anonymize<Iaia5o0jbgfjeu>;
  limited_teleport_assets: Anonymize<Iaia5o0jbgfjeu>;
  force_suspension: Anonymize<Ibgm4rnf22lal1>;
  transfer_assets: Anonymize<Iaia5o0jbgfjeu>;
}>;

type I6j18p941ujf3v = { dest: Anonymize<Ib29ie59v4nmjq>; message: Anonymize<Ieam757vsugkcv> };

type Ieam757vsugkcv = AnonymousEnum<{
  V2: Anonymize<I797ibmv93o8n9>;
  V3: Anonymize<I50ghg3dhe8sh3>;
}>;

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

type Idqjhq57s7jh4k = {
  dest: Anonymize<Ib29ie59v4nmjq>;
  beneficiary: Anonymize<Ib29ie59v4nmjq>;
  assets: Anonymize<I2tnkj3t3en8tf>;
  fee_asset_item: number;
};

type I8875l87smbh8d = { message: Anonymize<I2bgn21rdfqrr7>; max_weight: Anonymize<I4q39t5hn830vp> };

type I2bgn21rdfqrr7 = AnonymousEnum<{
  V2: Anonymize<I6gdh0i5feh6sm>;
  V3: Anonymize<Ie2lqpvbcq3vl6>;
}>;

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

type Ic76kfh5ebqkpl = { maybe_xcm_version: Anonymize<I4arjljr6dpflb> };

type Idmol6ivsgrnjg = { location: Anonymize<Ib29ie59v4nmjq> };

type Iaia5o0jbgfjeu = {
  dest: Anonymize<Ib29ie59v4nmjq>;
  beneficiary: Anonymize<Ib29ie59v4nmjq>;
  assets: Anonymize<I2tnkj3t3en8tf>;
  fee_asset_item: number;
  weight_limit: XcmV3WeightLimit;
};

type Ibgm4rnf22lal1 = { suspended: boolean };

type I8lmlccfrohcqg = AnonymousEnum<{
  reap_page: Anonymize<Ie7i4er98lat7a>;
  execute_overweight: Anonymize<Ia9qm3rtb7g8q2>;
}>;

type Ie7i4er98lat7a = { message_origin: Anonymize<Ifqm8uoikppunt>; page_index: number };

type Ia9qm3rtb7g8q2 = {
  message_origin: Anonymize<Ifqm8uoikppunt>;
  page: number;
  index: number;
  weight_limit: Anonymize<I4q39t5hn830vp>;
};

type I9jhevh1bis85g = AnonymousEnum<{
  msp_sign_up: Anonymize<I7f84sqlv1qo0o>;
  bsp_sign_up: Anonymize<Ia5ovv2se2388q>;
  msp_sign_off: undefined;
  bsp_sign_off: undefined;
  change_capacity: Anonymize<Idtgqk45hrbi8p>;
  add_value_prop: Anonymize<I14nnuk1kafge3>;
}>;

type I7f84sqlv1qo0o = {
  capacity: number;
  multiaddresses: Anonymize<Itom7fk49o0c9>;
  value_prop: Anonymize<Ienf50imfp828o>;
};

type Ia5ovv2se2388q = { capacity: number; multiaddresses: Anonymize<Itom7fk49o0c9> };

type Idtgqk45hrbi8p = { new_capacity: number };

type I14nnuk1kafge3 = { new_value_prop: Anonymize<Ienf50imfp828o> };

type I8u4nbk1d32u7q = AnonymousEnum<{
  create_bucket: undefined;
  issue_storage_request: Anonymize<I9qojolml35vd8>;
  revoke_storage_request: Anonymize<Ieg5outd74d62b>;
  bsp_volunteer: Anonymize<I9dkpda2lb6php>;
  bsp_stop_storing: Anonymize<Ial9ukhdv2nl6d>;
}>;

type I9qojolml35vd8 = {
  location: Binary;
  fingerprint: Binary;
  size: bigint;
  multiaddresses: Anonymize<Itom7fk49o0c9>;
};

type I9dkpda2lb6php = {
  location: Binary;
  fingerprint: Binary;
  multiaddresses: Anonymize<Itom7fk49o0c9>;
};

type Ial9ukhdv2nl6d = {
  file_key: Binary;
  location: Binary;
  owner: SS58String;
  fingerprint: Binary;
  size: bigint;
  can_serve: boolean;
};

type Iaoc2q2c87hkb1 = AnonymousEnum<{
  challenge: Anonymize<I7th9mlhkfgtvn>;
  submit_proof: Anonymize<I4rvc765f9unuv>;
  new_challenges_round: undefined;
}>;

type I7th9mlhkfgtvn = { key: Binary };

type I4rvc765f9unuv = {
  proof: Anonymize<Itom7fk49o0c9>;
  root: Binary;
  challenge_block: number;
  provider: Anonymize<I17k3ujudqd5df>;
};

type Ifq9uub37mee7a = { call: Anonymize<Iupi52pl09tgg>; weight: Anonymize<I4q39t5hn830vp> };

type Icnonnse26sae7 = { new: MultiAddress };

type I4h5fkjfra8jm3 = { who: MultiAddress; call: Anonymize<Iupi52pl09tgg> };

export type PalletError = Enum<
  | { type: "InvalidSpecName"; value: undefined }
  | { type: "SpecVersionNeedsToIncrease"; value: undefined }
  | { type: "FailedToExtractRuntimeVersion"; value: undefined }
  | { type: "NonDefaultComposite"; value: undefined }
  | { type: "NonZeroRefCount"; value: undefined }
  | { type: "CallFiltered"; value: undefined }
>;
export const PalletError = _Enum as unknown as GetEnum<PalletError>;

type I9vapnho0vb2oi = AnonymousEnum<{
  OverlappingUpgrades: undefined;
  ProhibitedByPolkadot: undefined;
  TooBig: undefined;
  ValidationDataNotAvailable: undefined;
  HostConfigurationNotAvailable: undefined;
  NotScheduled: undefined;
  NothingAuthorized: undefined;
  Unauthorized: undefined;
}>;

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
>;
export const BalancesPalletError = _Enum as unknown as GetEnum<BalancesPalletError>;

export type SudoPalletError = Enum<{ type: "RequireSudo"; value: undefined }>;
export const SudoPalletError = _Enum as unknown as GetEnum<SudoPalletError>;

type I5krkgn2iqf2ch = AnonymousEnum<{
  TooManyCandidates: undefined;
  TooFewEligibleCollators: undefined;
  AlreadyCandidate: undefined;
  NotCandidate: undefined;
  TooManyInvulnerables: undefined;
  AlreadyInvulnerable: undefined;
  NotInvulnerable: undefined;
  NoAssociatedValidatorId: undefined;
  ValidatorNotRegistered: undefined;
  InsertToCandidateListFailed: undefined;
  RemoveFromCandidateListFailed: undefined;
  DepositTooLow: undefined;
  UpdateCandidateListFailed: undefined;
  InsufficientBond: undefined;
  TargetIsNotCandidate: undefined;
  IdenticalDeposit: undefined;
  InvalidUnreserve: undefined;
}>;

export type SessionPalletError = Enum<
  | { type: "InvalidProof"; value: undefined }
  | { type: "NoAssociatedValidatorId"; value: undefined }
  | { type: "DuplicatedKey"; value: undefined }
  | { type: "NoKeys"; value: undefined }
  | { type: "NoAccount"; value: undefined }
>;
export const SessionPalletError = _Enum as unknown as GetEnum<SessionPalletError>;

type Ib5i5qplqm4io6 = AnonymousEnum<{
  BadQueueConfig: undefined;
  AlreadySuspended: undefined;
  AlreadyResumed: undefined;
}>;

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

export type MessageQueuePalletError = Enum<
  | { type: "NotReapable"; value: undefined }
  | { type: "NoPage"; value: undefined }
  | { type: "NoMessage"; value: undefined }
  | { type: "AlreadyProcessed"; value: undefined }
  | { type: "Queued"; value: undefined }
  | { type: "InsufficientWeight"; value: undefined }
  | { type: "TemporarilyUnprocessable"; value: undefined }
  | { type: "QueuePaused"; value: undefined }
>;
export const MessageQueuePalletError = _Enum as unknown as GetEnum<MessageQueuePalletError>;

type I3hjnk4podc7kt = AnonymousEnum<{
  AlreadyRegistered: undefined;
  StorageTooLow: undefined;
  NotEnoughBalance: undefined;
  CannotHoldDeposit: undefined;
  MaxBspsReached: undefined;
  MaxMspsReached: undefined;
  NotRegistered: undefined;
  SpRegisteredButDataNotFound: undefined;
  StorageStillInUse: undefined;
  NotEnoughTimePassed: undefined;
  NoUserId: undefined;
  NoBucketId: undefined;
  NoMultiAddress: undefined;
  InvalidMultiAddress: undefined;
  Overflow: undefined;
}>;

type Ibi6ldaq44tmah = AnonymousEnum<{
  StorageRequestAlreadyRegistered: undefined;
  StorageRequestNotFound: undefined;
  BspsRequiredCannotBeZero: undefined;
  BspsRequiredExceedsMax: undefined;
  BspVolunteerFailed: undefined;
  StorageRequestBspsRequiredFulfilled: undefined;
  BspAlreadyVolunteered: undefined;
  StorageRequestExpiredNoSlotAvailable: undefined;
  StorageRequestNotAuthorized: undefined;
  MaxBlockNumberReached: undefined;
}>;

type Ieuql58h232q35 = AnonymousEnum<{
  ChallengesQueueOverflow: undefined;
  PriorityChallengesQueueOverflow: undefined;
  NotProvider: undefined;
  FeeChargeFailed: undefined;
}>;

type Ia4jqf401148a4 = { header: Anonymize<I6t1nedlt7mobn>; extrinsics: Anonymize<Itom7fk49o0c9> };

type I6t1nedlt7mobn = {
  parent_hash: Binary;
  number: number;
  state_root: Binary;
  extrinsics_root: Binary;
  digest: Anonymize<Idin6nhq46lvdj>;
};

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

type Ic1d4u2opv3fst = {
  upward_messages: Anonymize<Itom7fk49o0c9>;
  horizontal_messages: Anonymize<I6r5cbv8ttrb09>;
  new_validation_code: Anonymize<Iabpgqcjikia83>;
  processed_downward_messages: number;
  hrmp_watermark: number;
  head_data: Binary;
};

type I9q9lvv0h98nhk = ResultPayload<undefined, string>;

const cf3ncnq6stc567 = "f3ncnq6stc567";
const cfmam6uono5akb = "fmam6uono5akb";
const c7i24fvhamijd = "7i24fvhamijd";
const cegurtje61sbu9 = "egurtje61sbu9";
const c51jgofmopm9om = "51jgofmopm9om";
const ctom7fk49o0c9 = "tom7fk49o0c9";
const c7d2tqf029vcod = "7d2tqf029vcod";
const c4q39t5hn830vp = "4q39t5hn830vp";
const cb3qt1mgvgmbgi = "b3qt1mgvgmbgi";
const c86esvcpejimq9 = "86esvcpejimq9";
const ca2lhg7l2hilo3 = "a2lhg7l2hilo3";
const ccgljjb6j82uhn = "cgljjb6j82uhn";
const cjf67g8lik487 = "jf67g8lik487";
const cdhnf6rtqoslea = "dhnf6rtqoslea";
const ccsbuok7olbg29 = "csbuok7olbg29";
const cdqa2fau59jv7f = "dqa2fau59jv7f";
const c8ofcg5rbj0g2c = "8ofcg5rbj0g2c";
const c6pjjpfvhvcfru = "6pjjpfvhvcfru";
const cen6q0lasi0m7i = "en6q0lasi0m7i";
const c39ibapn5dm6tr = "39ibapn5dm6tr";
const c4cbvqmqadhrea = "4cbvqmqadhrea";
const c3vh014cqgmrfd = "3vh014cqgmrfd";
const cdqjhq57s7jh4k = "dqjhq57s7jh4k";
const ce9it7tqcnjnfj = "e9it7tqcnjnfj";
const cdmol6ivsgrnjg = "dmol6ivsgrnjg";
const caia5o0jbgfjeu = "aia5o0jbgfjeu";
const ceg5outd74d62b = "eg5outd74d62b";
const ccbccs0ug47ilf = "cbccs0ug47ilf";
const cd5fm4p8lj5qgi = "d5fm4p8lj5qgi";
const c3qt1hgg4djhgb = "3qt1hgg4djhgb";
const c331o7t2g0ooi9 = "331o7t2g0ooi9";
const c6v8sm60vvkmk7 = "6v8sm60vvkmk7";
const cdgorhsbgdq2ap = "dgorhsbgdq2ap";
const cdrsgrbh5b6rje = "drsgrbh5b6rje";
const c2uqmls7kcdnii = "2uqmls7kcdnii";
const c2pd6nni2u8392 = "2pd6nni2u8392";
const c14amtmubrpgc8 = "14amtmubrpgc8";
const c3s707ha9gi47u = "3s707ha9gi47u";

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
  "dhc577m5btfvc" as StorageDescriptor<[Key: number], Binary, false>;
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
  Array<Anonymize<Idvbs8vg3olusq>>,
  false
> = "dus5q78qvra6h" as StorageDescriptor<[], Array<Anonymize<Idvbs8vg3olusq>>, false>;
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
const stg_ParachainSystem_UnincludedSegment: StorageDescriptor<
  [],
  Array<Anonymize<I8ajtuet8esesv>>,
  false
> = "6f7juk1vj8fiq" as StorageDescriptor<[], Array<Anonymize<I8ajtuet8esesv>>, false>;
const stg_ParachainSystem_AggregatedUnincludedSegment: StorageDescriptor<
  [],
  {
    used_bandwidth: Anonymize<Ieafp1gui1o4cl>;
    hrmp_watermark: Anonymize<I4arjljr6dpflb>;
    consumed_go_ahead_signal: Anonymize<Ie1vdku2j6ccvj>;
  },
  true
> = "amnn769l033b4" as StorageDescriptor<
  [],
  {
    used_bandwidth: Anonymize<Ieafp1gui1o4cl>;
    hrmp_watermark: Anonymize<I4arjljr6dpflb>;
    consumed_go_ahead_signal: Anonymize<Ie1vdku2j6ccvj>;
  },
  true
>;
const stg_ParachainSystem_PendingValidationCode: StorageDescriptor<[], Binary, false> =
  c51jgofmopm9om as StorageDescriptor<[], Binary, false>;
const stg_ParachainSystem_NewValidationCode: StorageDescriptor<[], Binary, true> =
  c51jgofmopm9om as StorageDescriptor<[], Binary, true>;
const stg_ParachainSystem_ValidationData: StorageDescriptor<
  [],
  {
    parent_head: Binary;
    relay_parent_number: number;
    relay_parent_storage_root: Binary;
    max_pov_size: number;
  },
  true
> = "5r8ef6aie125l" as StorageDescriptor<
  [],
  {
    parent_head: Binary;
    relay_parent_number: number;
    relay_parent_storage_root: Binary;
    max_pov_size: number;
  },
  true
>;
const stg_ParachainSystem_DidSetValidationCode: StorageDescriptor<[], boolean, false> =
  cegurtje61sbu9 as StorageDescriptor<[], boolean, false>;
const stg_ParachainSystem_LastRelayChainBlockNumber: StorageDescriptor<[], number, false> =
  cf3ncnq6stc567 as StorageDescriptor<[], number, false>;
const stg_ParachainSystem_UpgradeRestrictionSignal: StorageDescriptor<
  [],
  PolkadotPrimitivesV5UpgradeRestriction | undefined,
  false
> = "d34bi0o1gnln9" as StorageDescriptor<
  [],
  PolkadotPrimitivesV5UpgradeRestriction | undefined,
  false
>;
const stg_ParachainSystem_UpgradeGoAhead: StorageDescriptor<
  [],
  PolkadotPrimitivesV5UpgradeGoAhead | undefined,
  false
> = "e1vdku2j6ccvj" as StorageDescriptor<[], PolkadotPrimitivesV5UpgradeGoAhead | undefined, false>;
const stg_ParachainSystem_RelayStateProof: StorageDescriptor<
  [],
  Array<Binary>,
  true
> = ctom7fk49o0c9 as StorageDescriptor<[], Array<Binary>, true>;
const stg_ParachainSystem_RelevantMessagingState: StorageDescriptor<
  [],
  {
    dmq_mqc_head: Binary;
    relay_dispatch_queue_remaining_capacity: Anonymize<I3j1v1c2btq4bd>;
    ingress_channels: Anonymize<I90nfahji0n33j>;
    egress_channels: Anonymize<I90nfahji0n33j>;
  },
  true
> = "8gicq33tfu7j1" as StorageDescriptor<
  [],
  {
    dmq_mqc_head: Binary;
    relay_dispatch_queue_remaining_capacity: Anonymize<I3j1v1c2btq4bd>;
    ingress_channels: Anonymize<I90nfahji0n33j>;
    egress_channels: Anonymize<I90nfahji0n33j>;
  },
  true
>;
const stg_ParachainSystem_HostConfiguration: StorageDescriptor<
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
  },
  true
> = "4iumukclgj8ej" as StorageDescriptor<
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
  },
  true
>;
const stg_ParachainSystem_LastDmqMqcHead: StorageDescriptor<[], Binary, false> =
  c7i24fvhamijd as StorageDescriptor<[], Binary, false>;
const stg_ParachainSystem_LastHrmpMqcHeads: StorageDescriptor<
  [],
  Array<Anonymize<If89923vhoiaim>>,
  false
> = "93ssha9egqq23" as StorageDescriptor<[], Array<Anonymize<If89923vhoiaim>>, false>;
const stg_ParachainSystem_ProcessedDownwardMessages: StorageDescriptor<[], number, false> =
  cf3ncnq6stc567 as StorageDescriptor<[], number, false>;
const stg_ParachainSystem_HrmpWatermark: StorageDescriptor<[], number, false> =
  cf3ncnq6stc567 as StorageDescriptor<[], number, false>;
const stg_ParachainSystem_HrmpOutboundMessages: StorageDescriptor<
  [],
  Array<Anonymize<I958l48g4qg5rf>>,
  false
> = "6r5cbv8ttrb09" as StorageDescriptor<[], Array<Anonymize<I958l48g4qg5rf>>, false>;
const stg_ParachainSystem_UpwardMessages: StorageDescriptor<
  [],
  Array<Binary>,
  false
> = ctom7fk49o0c9 as StorageDescriptor<[], Array<Binary>, false>;
const stg_ParachainSystem_PendingUpwardMessages: StorageDescriptor<
  [],
  Array<Binary>,
  false
> = ctom7fk49o0c9 as StorageDescriptor<[], Array<Binary>, false>;
const stg_ParachainSystem_UpwardDeliveryFeeFactor: StorageDescriptor<[], bigint, false> =
  c7d2tqf029vcod as StorageDescriptor<[], bigint, false>;
const stg_ParachainSystem_AnnouncedHrmpMessagesPerCandidate: StorageDescriptor<[], number, false> =
  cf3ncnq6stc567 as StorageDescriptor<[], number, false>;
const stg_ParachainSystem_ReservedXcmpWeightOverride: StorageDescriptor<
  [],
  { ref_time: bigint; proof_size: bigint },
  true
> = c4q39t5hn830vp as StorageDescriptor<[], { ref_time: bigint; proof_size: bigint }, true>;
const stg_ParachainSystem_ReservedDmpWeightOverride: StorageDescriptor<
  [],
  { ref_time: bigint; proof_size: bigint },
  true
> = c4q39t5hn830vp as StorageDescriptor<[], { ref_time: bigint; proof_size: bigint }, true>;
const stg_ParachainSystem_AuthorizedUpgrade: StorageDescriptor<
  [],
  { code_hash: Binary; check_version: boolean },
  true
> = cb3qt1mgvgmbgi as StorageDescriptor<[], { code_hash: Binary; check_version: boolean }, true>;
const stg_ParachainSystem_CustomValidationHeadData: StorageDescriptor<[], Binary, true> =
  c51jgofmopm9om as StorageDescriptor<[], Binary, true>;
const stg_Timestamp_Now: StorageDescriptor<[], bigint, false> = c7d2tqf029vcod as StorageDescriptor<
  [],
  bigint,
  false
>;
const stg_Timestamp_DidUpdate: StorageDescriptor<[], boolean, false> =
  cegurtje61sbu9 as StorageDescriptor<[], boolean, false>;
const stg_ParachainInfo_ParachainId: StorageDescriptor<[], number, false> =
  cf3ncnq6stc567 as StorageDescriptor<[], number, false>;
const stg_Balances_TotalIssuance: StorageDescriptor<[], bigint, false> =
  c7d2tqf029vcod as StorageDescriptor<[], bigint, false>;
const stg_Balances_InactiveIssuance: StorageDescriptor<[], bigint, false> =
  c7d2tqf029vcod as StorageDescriptor<[], bigint, false>;
const stg_Balances_Account: StorageDescriptor<
  [Key: SS58String],
  { free: bigint; reserved: bigint; frozen: bigint; flags: bigint },
  false
> = "btum7aot707m6" as StorageDescriptor<
  [Key: SS58String],
  { free: bigint; reserved: bigint; frozen: bigint; flags: bigint },
  false
>;
const stg_Balances_Locks: StorageDescriptor<
  [Key: SS58String],
  Array<Anonymize<I5b29v4qfq4tu7>>,
  false
> = "1e2385hi92o2f" as StorageDescriptor<
  [Key: SS58String],
  Array<Anonymize<I5b29v4qfq4tu7>>,
  false
>;
const stg_Balances_Reserves: StorageDescriptor<
  [Key: SS58String],
  Array<Anonymize<I32btm6htd9bck>>,
  false
> = "be3psbb7upqsv" as StorageDescriptor<
  [Key: SS58String],
  Array<Anonymize<I32btm6htd9bck>>,
  false
>;
const stg_Balances_Holds: StorageDescriptor<
  [Key: SS58String],
  Array<Anonymize<I66c0bbqofu6gm>>,
  false
> = "46s043a5p4mgi" as StorageDescriptor<
  [Key: SS58String],
  Array<Anonymize<I66c0bbqofu6gm>>,
  false
>;
const stg_Balances_Freezes: StorageDescriptor<
  [Key: SS58String],
  Array<Anonymize<I7qdm60946h5u9>>,
  false
> = "bfv6n704ucltm" as StorageDescriptor<
  [Key: SS58String],
  Array<Anonymize<I7qdm60946h5u9>>,
  false
>;
const stg_TransactionPayment_NextFeeMultiplier: StorageDescriptor<[], bigint, false> =
  c7d2tqf029vcod as StorageDescriptor<[], bigint, false>;
const stg_TransactionPayment_StorageVersion: StorageDescriptor<
  [],
  TransactionPaymentReleases,
  false
> = "944994o3qcrkq" as StorageDescriptor<[], TransactionPaymentReleases, false>;
const stg_Sudo_Key: StorageDescriptor<[], SS58String, true> = c86esvcpejimq9 as StorageDescriptor<
  [],
  SS58String,
  true
>;
const stg_Authorship_Author: StorageDescriptor<[], SS58String, true> =
  c86esvcpejimq9 as StorageDescriptor<[], SS58String, true>;
const stg_CollatorSelection_Invulnerables: StorageDescriptor<
  [],
  Array<SS58String>,
  false
> = ca2lhg7l2hilo3 as StorageDescriptor<[], Array<SS58String>, false>;
const stg_CollatorSelection_CandidateList: StorageDescriptor<
  [],
  Array<Anonymize<Iep1lmt6q3s6r3>>,
  false
> = "fi4da1gej1fri" as StorageDescriptor<[], Array<Anonymize<Iep1lmt6q3s6r3>>, false>;
const stg_CollatorSelection_LastAuthoredBlock: StorageDescriptor<[Key: SS58String], number, false> =
  "44uqc9gktfck1" as StorageDescriptor<[Key: SS58String], number, false>;
const stg_CollatorSelection_DesiredCandidates: StorageDescriptor<[], number, false> =
  cf3ncnq6stc567 as StorageDescriptor<[], number, false>;
const stg_CollatorSelection_CandidacyBond: StorageDescriptor<[], bigint, false> =
  c7d2tqf029vcod as StorageDescriptor<[], bigint, false>;
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
  Array<Anonymize<I73gble6tmb52f>>,
  false
> = "9our4clsjkim0" as StorageDescriptor<[], Array<Anonymize<I73gble6tmb52f>>, false>;
const stg_Session_DisabledValidators: StorageDescriptor<
  [],
  Array<number>,
  false
> = ccgljjb6j82uhn as StorageDescriptor<[], Array<number>, false>;
const stg_Session_NextKeys: StorageDescriptor<[Key: SS58String], Binary, true> =
  cjf67g8lik487 as StorageDescriptor<[Key: SS58String], Binary, true>;
const stg_Session_KeyOwner: StorageDescriptor<[Key: [Binary, Binary]], SS58String, true> =
  "8drjr6qpp26i0" as StorageDescriptor<[Key: [Binary, Binary]], SS58String, true>;
const stg_Aura_Authorities: StorageDescriptor<
  [],
  Array<Binary>,
  false
> = cdhnf6rtqoslea as StorageDescriptor<[], Array<Binary>, false>;
const stg_Aura_CurrentSlot: StorageDescriptor<[], bigint, false> =
  c7d2tqf029vcod as StorageDescriptor<[], bigint, false>;
const stg_AuraExt_Authorities: StorageDescriptor<
  [],
  Array<Binary>,
  false
> = cdhnf6rtqoslea as StorageDescriptor<[], Array<Binary>, false>;
const stg_AuraExt_SlotInfo: StorageDescriptor<[], [bigint, number], true> =
  "6cs1itejju2vv" as StorageDescriptor<[], [bigint, number], true>;
const stg_XcmpQueue_InboundXcmpSuspended: StorageDescriptor<
  [],
  Array<number>,
  false
> = ccgljjb6j82uhn as StorageDescriptor<[], Array<number>, false>;
const stg_XcmpQueue_OutboundXcmpStatus: StorageDescriptor<
  [],
  Array<Anonymize<Ittnsbm78tol1>>,
  false
> = "1mse8p8gvh143" as StorageDescriptor<[], Array<Anonymize<Ittnsbm78tol1>>, false>;
const stg_XcmpQueue_OutboundXcmpMessages: StorageDescriptor<[number, number], Binary, false> =
  "8m3ph6rer8teq" as StorageDescriptor<[number, number], Binary, false>;
const stg_XcmpQueue_SignalMessages: StorageDescriptor<[Key: number], Binary, false> =
  cfmam6uono5akb as StorageDescriptor<[Key: number], Binary, false>;
const stg_XcmpQueue_QueueConfig: StorageDescriptor<
  [],
  { suspend_threshold: number; drop_threshold: number; resume_threshold: number },
  false
> = "fup3lg9ro8a0f" as StorageDescriptor<
  [],
  { suspend_threshold: number; drop_threshold: number; resume_threshold: number },
  false
>;
const stg_XcmpQueue_QueueSuspended: StorageDescriptor<[], boolean, false> =
  cegurtje61sbu9 as StorageDescriptor<[], boolean, false>;
const stg_XcmpQueue_DeliveryFeeFactor: StorageDescriptor<[Key: number], bigint, false> =
  "au4oj9ee8cokk" as StorageDescriptor<[Key: number], bigint, false>;
const stg_PolkadotXcm_QueryCounter: StorageDescriptor<[], bigint, false> =
  c7d2tqf029vcod as StorageDescriptor<[], bigint, false>;
const stg_PolkadotXcm_Queries: StorageDescriptor<
  [Key: bigint],
  Anonymize<
    AnonymousEnum<{
      Pending: Anonymize<I4n9ble5dnecdr>;
      VersionNotifier: Anonymize<Idc4lam0e7aiet>;
      Ready: Anonymize<I3239o3gbno6s5>;
    }>
  >,
  true
> = "bj05m6utni9tl" as StorageDescriptor<
  [Key: bigint],
  Anonymize<
    AnonymousEnum<{
      Pending: Anonymize<I4n9ble5dnecdr>;
      VersionNotifier: Anonymize<Idc4lam0e7aiet>;
      Ready: Anonymize<I3239o3gbno6s5>;
    }>
  >,
  true
>;
const stg_PolkadotXcm_AssetTraps: StorageDescriptor<[Key: Binary], number, false> =
  ccsbuok7olbg29 as StorageDescriptor<[Key: Binary], number, false>;
const stg_PolkadotXcm_SafeXcmVersion: StorageDescriptor<[], number, true> =
  cf3ncnq6stc567 as StorageDescriptor<[], number, true>;
const stg_PolkadotXcm_SupportedVersion: StorageDescriptor<
  [number, Anonymize<Ib29ie59v4nmjq>],
  number,
  true
> = "4o34ccjrejuen" as StorageDescriptor<[number, Anonymize<Ib29ie59v4nmjq>], number, true>;
const stg_PolkadotXcm_VersionNotifiers: StorageDescriptor<
  [number, Anonymize<Ib29ie59v4nmjq>],
  bigint,
  true
> = "6rpvtou9ar1o7" as StorageDescriptor<[number, Anonymize<Ib29ie59v4nmjq>], bigint, true>;
const stg_PolkadotXcm_VersionNotifyTargets: StorageDescriptor<
  [number, Anonymize<Ib29ie59v4nmjq>],
  [bigint, Anonymize<I4q39t5hn830vp>, number],
  true
> = "fd942arg9cnf0" as StorageDescriptor<
  [number, Anonymize<Ib29ie59v4nmjq>],
  [bigint, Anonymize<I4q39t5hn830vp>, number],
  true
>;
const stg_PolkadotXcm_VersionDiscoveryQueue: StorageDescriptor<
  [],
  Array<Anonymize<I82i8h7h2mvtd5>>,
  false
> = "9kuv6u16da6dg" as StorageDescriptor<[], Array<Anonymize<I82i8h7h2mvtd5>>, false>;
const stg_PolkadotXcm_CurrentMigration: StorageDescriptor<
  [],
  XcmPalletVersionMigrationStage,
  true
> = "1s1utqquuaa2b" as StorageDescriptor<[], XcmPalletVersionMigrationStage, true>;
const stg_PolkadotXcm_RemoteLockedFungibles: StorageDescriptor<
  [number, SS58String, Anonymize<I9hdbmmgal228m>],
  {
    amount: bigint;
    owner: Anonymize<Ib29ie59v4nmjq>;
    locker: Anonymize<Ib29ie59v4nmjq>;
    consumers: Anonymize<I48jka0f0ufl6q>;
  },
  true
> = "fmq7l72lbfn3h" as StorageDescriptor<
  [number, SS58String, Anonymize<I9hdbmmgal228m>],
  {
    amount: bigint;
    owner: Anonymize<Ib29ie59v4nmjq>;
    locker: Anonymize<Ib29ie59v4nmjq>;
    consumers: Anonymize<I48jka0f0ufl6q>;
  },
  true
>;
const stg_PolkadotXcm_LockedFungibles: StorageDescriptor<
  [Key: SS58String],
  Array<Anonymize<Ifuuq590aavd5n>>,
  true
> = "1297dab842br3" as StorageDescriptor<[Key: SS58String], Array<Anonymize<Ifuuq590aavd5n>>, true>;
const stg_PolkadotXcm_XcmExecutionSuspended: StorageDescriptor<[], boolean, false> =
  cegurtje61sbu9 as StorageDescriptor<[], boolean, false>;
const stg_MessageQueue_BookStateFor: StorageDescriptor<
  [
    Key: Anonymize<
      AnonymousEnum<{ Here: undefined; Parent: undefined; Sibling: Anonymize<number> }>
    >,
  ],
  {
    begin: number;
    end: number;
    count: number;
    ready_neighbours: Anonymize<If4d9hsuqsl01i>;
    message_count: bigint;
    size: bigint;
  },
  false
> = "9b0pbgc57qqtc" as StorageDescriptor<
  [
    Key: Anonymize<
      AnonymousEnum<{ Here: undefined; Parent: undefined; Sibling: Anonymize<number> }>
    >,
  ],
  {
    begin: number;
    end: number;
    count: number;
    ready_neighbours: Anonymize<If4d9hsuqsl01i>;
    message_count: bigint;
    size: bigint;
  },
  false
>;
const stg_MessageQueue_ServiceHead: StorageDescriptor<
  [],
  Anonymize<AnonymousEnum<{ Here: undefined; Parent: undefined; Sibling: Anonymize<number> }>>,
  true
> = "fqm8uoikppunt" as StorageDescriptor<
  [],
  Anonymize<AnonymousEnum<{ Here: undefined; Parent: undefined; Sibling: Anonymize<number> }>>,
  true
>;
const stg_MessageQueue_Pages: StorageDescriptor<
  [Anonymize<Ifqm8uoikppunt>, number],
  {
    remaining: number;
    remaining_size: number;
    first_index: number;
    first: number;
    last: number;
    heap: Binary;
  },
  true
> = "6425hqi2ga1ps" as StorageDescriptor<
  [Anonymize<Ifqm8uoikppunt>, number],
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
const stg_Providers_AccountIdToMainStorageProviderId: StorageDescriptor<
  [Key: SS58String],
  Binary,
  true
> = cjf67g8lik487 as StorageDescriptor<[Key: SS58String], Binary, true>;
const stg_Providers_MainStorageProviders: StorageDescriptor<
  [Key: Binary],
  {
    buckets: Anonymize<I45d79rdcadrnn>;
    capacity: number;
    data_used: number;
    multiaddresses: Anonymize<Itom7fk49o0c9>;
    value_prop: Anonymize<Ienf50imfp828o>;
  },
  true
> = "6jasqtdjjeqr9" as StorageDescriptor<
  [Key: Binary],
  {
    buckets: Anonymize<I45d79rdcadrnn>;
    capacity: number;
    data_used: number;
    multiaddresses: Anonymize<Itom7fk49o0c9>;
    value_prop: Anonymize<Ienf50imfp828o>;
  },
  true
>;
const stg_Providers_Buckets: StorageDescriptor<
  [Key: Binary],
  { root: Binary; user_id: SS58String; msp_id: Binary },
  true
> = "efnfq3tuv7tvj" as StorageDescriptor<
  [Key: Binary],
  { root: Binary; user_id: SS58String; msp_id: Binary },
  true
>;
const stg_Providers_AccountIdToBackupStorageProviderId: StorageDescriptor<
  [Key: SS58String],
  Binary,
  true
> = cjf67g8lik487 as StorageDescriptor<[Key: SS58String], Binary, true>;
const stg_Providers_BackupStorageProviders: StorageDescriptor<
  [Key: Binary],
  { capacity: number; data_used: number; multiaddresses: Anonymize<Itom7fk49o0c9>; root: Binary },
  true
> = "eu1m784fjn4qm" as StorageDescriptor<
  [Key: Binary],
  { capacity: number; data_used: number; multiaddresses: Anonymize<Itom7fk49o0c9>; root: Binary },
  true
>;
const stg_Providers_MspCount: StorageDescriptor<[], number, false> =
  cf3ncnq6stc567 as StorageDescriptor<[], number, false>;
const stg_Providers_BspCount: StorageDescriptor<[], number, false> =
  cf3ncnq6stc567 as StorageDescriptor<[], number, false>;
const stg_Providers_TotalBspsCapacity: StorageDescriptor<[], number, false> =
  cf3ncnq6stc567 as StorageDescriptor<[], number, false>;
const stg_FileSystem_StorageRequests: StorageDescriptor<
  [Key: Binary],
  {
    requested_at: number;
    owner: SS58String;
    fingerprint: Binary;
    size: bigint;
    user_multiaddresses: Anonymize<Itom7fk49o0c9>;
    data_server_sps: Anonymize<Ia2lhg7l2hilo3>;
    bsps_required: number;
    bsps_confirmed: number;
  },
  true
> = "a0bpikrpfb3uc" as StorageDescriptor<
  [Key: Binary],
  {
    requested_at: number;
    owner: SS58String;
    fingerprint: Binary;
    size: bigint;
    user_multiaddresses: Anonymize<Itom7fk49o0c9>;
    data_server_sps: Anonymize<Ia2lhg7l2hilo3>;
    bsps_required: number;
    bsps_confirmed: number;
  },
  true
>;
const stg_FileSystem_StorageRequestBsps: StorageDescriptor<[Binary, SS58String], boolean, true> =
  "bm58dgema804b" as StorageDescriptor<[Binary, SS58String], boolean, true>;
const stg_FileSystem_StorageRequestExpirations: StorageDescriptor<
  [Key: number],
  Array<Binary>,
  false
> = "2v3dnu8k55thc" as StorageDescriptor<[Key: number], Array<Binary>, false>;
const stg_FileSystem_NextAvailableExpirationInsertionBlock: StorageDescriptor<[], number, false> =
  cf3ncnq6stc567 as StorageDescriptor<[], number, false>;
const stg_FileSystem_NextStartingBlockToCleanUp: StorageDescriptor<[], number, false> =
  cf3ncnq6stc567 as StorageDescriptor<[], number, false>;
const stg_ProofsDealer_BlockToChallenges: StorageDescriptor<
  [Key: number],
  Array<Binary>,
  true
> = cdqa2fau59jv7f as StorageDescriptor<[Key: number], Array<Binary>, true>;
const stg_ProofsDealer_BlockToChallengedSps: StorageDescriptor<
  [Key: number],
  Array<Binary>,
  true
> = cdqa2fau59jv7f as StorageDescriptor<[Key: number], Array<Binary>, true>;
const stg_ProofsDealer_LastBlockSpSubmittedProofFor: StorageDescriptor<
  [Key: Binary],
  number,
  true
> = ccsbuok7olbg29 as StorageDescriptor<[Key: Binary], number, true>;
const stg_ProofsDealer_ChallengesQueue: StorageDescriptor<
  [],
  Array<Binary>,
  false
> = cdhnf6rtqoslea as StorageDescriptor<[], Array<Binary>, false>;
const stg_ProofsDealer_PriorityChallengesQueue: StorageDescriptor<
  [],
  Array<Binary>,
  false
> = cdhnf6rtqoslea as StorageDescriptor<[], Array<Binary>, false>;
const stg_ProofsDealer_LastCheckpointBlock: StorageDescriptor<[], number, false> =
  cf3ncnq6stc567 as StorageDescriptor<[], number, false>;
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
const call_ParachainSystem_set_validation_data: TxDescriptor<{ data: Anonymize<Icj9r7l64kc5ku> }> =
  "68js79djhsbni" as TxDescriptor<{ data: Anonymize<Icj9r7l64kc5ku> }>;
const call_ParachainSystem_sudo_send_upward_message: TxDescriptor<{ message: Binary }> =
  "fpj261e8s63m3" as TxDescriptor<{ message: Binary }>;
const call_ParachainSystem_authorize_upgrade: TxDescriptor<{
  code_hash: Binary;
  check_version: boolean;
}> = cb3qt1mgvgmbgi as TxDescriptor<{ code_hash: Binary; check_version: boolean }>;
const call_ParachainSystem_enact_authorized_upgrade: TxDescriptor<{ code: Binary }> =
  c6pjjpfvhvcfru as TxDescriptor<{ code: Binary }>;
const call_Timestamp_set: TxDescriptor<{ now: bigint }> = "dcr6u6361oad9" as TxDescriptor<{
  now: bigint;
}>;
const call_Balances_transfer_allow_death: TxDescriptor<{ dest: MultiAddress; value: bigint }> =
  cen6q0lasi0m7i as TxDescriptor<{ dest: MultiAddress; value: bigint }>;
const call_Balances_force_transfer: TxDescriptor<{
  source: MultiAddress;
  dest: MultiAddress;
  value: bigint;
}> = "cacgruoo9j3r2" as TxDescriptor<{ source: MultiAddress; dest: MultiAddress; value: bigint }>;
const call_Balances_transfer_keep_alive: TxDescriptor<{ dest: MultiAddress; value: bigint }> =
  cen6q0lasi0m7i as TxDescriptor<{ dest: MultiAddress; value: bigint }>;
const call_Balances_transfer_all: TxDescriptor<{ dest: MultiAddress; keep_alive: boolean }> =
  "7dgmo7im9hljo" as TxDescriptor<{ dest: MultiAddress; keep_alive: boolean }>;
const call_Balances_force_unreserve: TxDescriptor<{ who: MultiAddress; amount: bigint }> =
  "argojp1sv9icj" as TxDescriptor<{ who: MultiAddress; amount: bigint }>;
const call_Balances_upgrade_accounts: TxDescriptor<{ who: Anonymize<Ia2lhg7l2hilo3> }> =
  "bmr18suc9ikh9" as TxDescriptor<{ who: Anonymize<Ia2lhg7l2hilo3> }>;
const call_Balances_force_set_balance: TxDescriptor<{ who: MultiAddress; new_free: bigint }> =
  "e0io91hk7pejj" as TxDescriptor<{ who: MultiAddress; new_free: bigint }>;
const call_Sudo_sudo: TxDescriptor<{ call: Anonymize<Iupi52pl09tgg> }> =
  "95rtegihqfhrh" as TxDescriptor<{ call: Anonymize<Iupi52pl09tgg> }>;
const call_Sudo_sudo_unchecked_weight: TxDescriptor<{
  call: Anonymize<Iupi52pl09tgg>;
  weight: Anonymize<I4q39t5hn830vp>;
}> = "fq9uub37mee7a" as TxDescriptor<{
  call: Anonymize<Iupi52pl09tgg>;
  weight: Anonymize<I4q39t5hn830vp>;
}>;
const call_Sudo_set_key: TxDescriptor<{ new: MultiAddress }> = "cnonnse26sae7" as TxDescriptor<{
  new: MultiAddress;
}>;
const call_Sudo_sudo_as: TxDescriptor<{ who: MultiAddress; call: Anonymize<Iupi52pl09tgg> }> =
  "4h5fkjfra8jm3" as TxDescriptor<{ who: MultiAddress; call: Anonymize<Iupi52pl09tgg> }>;
const call_Sudo_remove_key: TxDescriptor<undefined> = c39ibapn5dm6tr as TxDescriptor<undefined>;
const call_CollatorSelection_set_invulnerables: TxDescriptor<{ new: Anonymize<Ia2lhg7l2hilo3> }> =
  "fccifqltb5obi" as TxDescriptor<{ new: Anonymize<Ia2lhg7l2hilo3> }>;
const call_CollatorSelection_set_desired_candidates: TxDescriptor<{ max: number }> =
  "adtsfv699cq8b" as TxDescriptor<{ max: number }>;
const call_CollatorSelection_set_candidacy_bond: TxDescriptor<{ bond: bigint }> =
  "alpmgmhr3gk5r" as TxDescriptor<{ bond: bigint }>;
const call_CollatorSelection_register_as_candidate: TxDescriptor<undefined> =
  c39ibapn5dm6tr as TxDescriptor<undefined>;
const call_CollatorSelection_leave_intent: TxDescriptor<undefined> =
  c39ibapn5dm6tr as TxDescriptor<undefined>;
const call_CollatorSelection_add_invulnerable: TxDescriptor<{ who: SS58String }> =
  c4cbvqmqadhrea as TxDescriptor<{ who: SS58String }>;
const call_CollatorSelection_remove_invulnerable: TxDescriptor<{ who: SS58String }> =
  c4cbvqmqadhrea as TxDescriptor<{ who: SS58String }>;
const call_CollatorSelection_update_bond: TxDescriptor<{ new_deposit: bigint }> =
  "3sdol54kg5jaq" as TxDescriptor<{ new_deposit: bigint }>;
const call_CollatorSelection_take_candidate_slot: TxDescriptor<{
  deposit: bigint;
  target: SS58String;
}> = "8fougodaj6di6" as TxDescriptor<{ deposit: bigint; target: SS58String }>;
const call_Session_set_keys: TxDescriptor<{ keys: Binary; proof: Binary }> =
  "vojoo8sbcs0m" as TxDescriptor<{ keys: Binary; proof: Binary }>;
const call_Session_purge_keys: TxDescriptor<undefined> = c39ibapn5dm6tr as TxDescriptor<undefined>;
const call_XcmpQueue_suspend_xcm_execution: TxDescriptor<undefined> =
  c39ibapn5dm6tr as TxDescriptor<undefined>;
const call_XcmpQueue_resume_xcm_execution: TxDescriptor<undefined> =
  c39ibapn5dm6tr as TxDescriptor<undefined>;
const call_XcmpQueue_update_suspend_threshold: TxDescriptor<{ new: number }> =
  c3vh014cqgmrfd as TxDescriptor<{ new: number }>;
const call_XcmpQueue_update_drop_threshold: TxDescriptor<{ new: number }> =
  c3vh014cqgmrfd as TxDescriptor<{ new: number }>;
const call_XcmpQueue_update_resume_threshold: TxDescriptor<{ new: number }> =
  c3vh014cqgmrfd as TxDescriptor<{ new: number }>;
const call_PolkadotXcm_send: TxDescriptor<{
  dest: Anonymize<Ib29ie59v4nmjq>;
  message: Anonymize<Ieam757vsugkcv>;
}> = "6j18p941ujf3v" as TxDescriptor<{
  dest: Anonymize<Ib29ie59v4nmjq>;
  message: Anonymize<Ieam757vsugkcv>;
}>;
const call_PolkadotXcm_teleport_assets: TxDescriptor<{
  dest: Anonymize<Ib29ie59v4nmjq>;
  beneficiary: Anonymize<Ib29ie59v4nmjq>;
  assets: Anonymize<I2tnkj3t3en8tf>;
  fee_asset_item: number;
}> = cdqjhq57s7jh4k as TxDescriptor<{
  dest: Anonymize<Ib29ie59v4nmjq>;
  beneficiary: Anonymize<Ib29ie59v4nmjq>;
  assets: Anonymize<I2tnkj3t3en8tf>;
  fee_asset_item: number;
}>;
const call_PolkadotXcm_reserve_transfer_assets: TxDescriptor<{
  dest: Anonymize<Ib29ie59v4nmjq>;
  beneficiary: Anonymize<Ib29ie59v4nmjq>;
  assets: Anonymize<I2tnkj3t3en8tf>;
  fee_asset_item: number;
}> = cdqjhq57s7jh4k as TxDescriptor<{
  dest: Anonymize<Ib29ie59v4nmjq>;
  beneficiary: Anonymize<Ib29ie59v4nmjq>;
  assets: Anonymize<I2tnkj3t3en8tf>;
  fee_asset_item: number;
}>;
const call_PolkadotXcm_execute: TxDescriptor<{
  message: Anonymize<I2bgn21rdfqrr7>;
  max_weight: Anonymize<I4q39t5hn830vp>;
}> = "8875l87smbh8d" as TxDescriptor<{
  message: Anonymize<I2bgn21rdfqrr7>;
  max_weight: Anonymize<I4q39t5hn830vp>;
}>;
const call_PolkadotXcm_force_xcm_version: TxDescriptor<{
  location: Anonymize<Ie897ubj3a1vaq>;
  version: number;
}> = ce9it7tqcnjnfj as TxDescriptor<{ location: Anonymize<Ie897ubj3a1vaq>; version: number }>;
const call_PolkadotXcm_force_default_xcm_version: TxDescriptor<{
  maybe_xcm_version: Anonymize<I4arjljr6dpflb>;
}> = "c76kfh5ebqkpl" as TxDescriptor<{ maybe_xcm_version: Anonymize<I4arjljr6dpflb> }>;
const call_PolkadotXcm_force_subscribe_version_notify: TxDescriptor<{
  location: Anonymize<Ib29ie59v4nmjq>;
}> = cdmol6ivsgrnjg as TxDescriptor<{ location: Anonymize<Ib29ie59v4nmjq> }>;
const call_PolkadotXcm_force_unsubscribe_version_notify: TxDescriptor<{
  location: Anonymize<Ib29ie59v4nmjq>;
}> = cdmol6ivsgrnjg as TxDescriptor<{ location: Anonymize<Ib29ie59v4nmjq> }>;
const call_PolkadotXcm_limited_reserve_transfer_assets: TxDescriptor<{
  dest: Anonymize<Ib29ie59v4nmjq>;
  beneficiary: Anonymize<Ib29ie59v4nmjq>;
  assets: Anonymize<I2tnkj3t3en8tf>;
  fee_asset_item: number;
  weight_limit: XcmV3WeightLimit;
}> = caia5o0jbgfjeu as TxDescriptor<{
  dest: Anonymize<Ib29ie59v4nmjq>;
  beneficiary: Anonymize<Ib29ie59v4nmjq>;
  assets: Anonymize<I2tnkj3t3en8tf>;
  fee_asset_item: number;
  weight_limit: XcmV3WeightLimit;
}>;
const call_PolkadotXcm_limited_teleport_assets: TxDescriptor<{
  dest: Anonymize<Ib29ie59v4nmjq>;
  beneficiary: Anonymize<Ib29ie59v4nmjq>;
  assets: Anonymize<I2tnkj3t3en8tf>;
  fee_asset_item: number;
  weight_limit: XcmV3WeightLimit;
}> = caia5o0jbgfjeu as TxDescriptor<{
  dest: Anonymize<Ib29ie59v4nmjq>;
  beneficiary: Anonymize<Ib29ie59v4nmjq>;
  assets: Anonymize<I2tnkj3t3en8tf>;
  fee_asset_item: number;
  weight_limit: XcmV3WeightLimit;
}>;
const call_PolkadotXcm_force_suspension: TxDescriptor<{ suspended: boolean }> =
  "bgm4rnf22lal1" as TxDescriptor<{ suspended: boolean }>;
const call_PolkadotXcm_transfer_assets: TxDescriptor<{
  dest: Anonymize<Ib29ie59v4nmjq>;
  beneficiary: Anonymize<Ib29ie59v4nmjq>;
  assets: Anonymize<I2tnkj3t3en8tf>;
  fee_asset_item: number;
  weight_limit: XcmV3WeightLimit;
}> = caia5o0jbgfjeu as TxDescriptor<{
  dest: Anonymize<Ib29ie59v4nmjq>;
  beneficiary: Anonymize<Ib29ie59v4nmjq>;
  assets: Anonymize<I2tnkj3t3en8tf>;
  fee_asset_item: number;
  weight_limit: XcmV3WeightLimit;
}>;
const call_MessageQueue_reap_page: TxDescriptor<{
  message_origin: Anonymize<Ifqm8uoikppunt>;
  page_index: number;
}> = "e7i4er98lat7a" as TxDescriptor<{
  message_origin: Anonymize<Ifqm8uoikppunt>;
  page_index: number;
}>;
const call_MessageQueue_execute_overweight: TxDescriptor<{
  message_origin: Anonymize<Ifqm8uoikppunt>;
  page: number;
  index: number;
  weight_limit: Anonymize<I4q39t5hn830vp>;
}> = "a9qm3rtb7g8q2" as TxDescriptor<{
  message_origin: Anonymize<Ifqm8uoikppunt>;
  page: number;
  index: number;
  weight_limit: Anonymize<I4q39t5hn830vp>;
}>;
const call_Providers_msp_sign_up: TxDescriptor<{
  capacity: number;
  multiaddresses: Anonymize<Itom7fk49o0c9>;
  value_prop: Anonymize<Ienf50imfp828o>;
}> = "7f84sqlv1qo0o" as TxDescriptor<{
  capacity: number;
  multiaddresses: Anonymize<Itom7fk49o0c9>;
  value_prop: Anonymize<Ienf50imfp828o>;
}>;
const call_Providers_bsp_sign_up: TxDescriptor<{
  capacity: number;
  multiaddresses: Anonymize<Itom7fk49o0c9>;
}> = "a5ovv2se2388q" as TxDescriptor<{
  capacity: number;
  multiaddresses: Anonymize<Itom7fk49o0c9>;
}>;
const call_Providers_msp_sign_off: TxDescriptor<undefined> =
  c39ibapn5dm6tr as TxDescriptor<undefined>;
const call_Providers_bsp_sign_off: TxDescriptor<undefined> =
  c39ibapn5dm6tr as TxDescriptor<undefined>;
const call_Providers_change_capacity: TxDescriptor<{ new_capacity: number }> =
  "dtgqk45hrbi8p" as TxDescriptor<{ new_capacity: number }>;
const call_Providers_add_value_prop: TxDescriptor<{ new_value_prop: Anonymize<Ienf50imfp828o> }> =
  "14nnuk1kafge3" as TxDescriptor<{ new_value_prop: Anonymize<Ienf50imfp828o> }>;
const call_FileSystem_create_bucket: TxDescriptor<undefined> =
  c39ibapn5dm6tr as TxDescriptor<undefined>;
const call_FileSystem_issue_storage_request: TxDescriptor<{
  location: Binary;
  fingerprint: Binary;
  size: bigint;
  multiaddresses: Anonymize<Itom7fk49o0c9>;
}> = "9qojolml35vd8" as TxDescriptor<{
  location: Binary;
  fingerprint: Binary;
  size: bigint;
  multiaddresses: Anonymize<Itom7fk49o0c9>;
}>;
const call_FileSystem_revoke_storage_request: TxDescriptor<{ location: Binary }> =
  ceg5outd74d62b as TxDescriptor<{ location: Binary }>;
const call_FileSystem_bsp_volunteer: TxDescriptor<{
  location: Binary;
  fingerprint: Binary;
  multiaddresses: Anonymize<Itom7fk49o0c9>;
}> = "9dkpda2lb6php" as TxDescriptor<{
  location: Binary;
  fingerprint: Binary;
  multiaddresses: Anonymize<Itom7fk49o0c9>;
}>;
const call_FileSystem_bsp_stop_storing: TxDescriptor<{
  file_key: Binary;
  location: Binary;
  owner: SS58String;
  fingerprint: Binary;
  size: bigint;
  can_serve: boolean;
}> = "al9ukhdv2nl6d" as TxDescriptor<{
  file_key: Binary;
  location: Binary;
  owner: SS58String;
  fingerprint: Binary;
  size: bigint;
  can_serve: boolean;
}>;
const call_ProofsDealer_challenge: TxDescriptor<{ key: Binary }> = "7th9mlhkfgtvn" as TxDescriptor<{
  key: Binary;
}>;
const call_ProofsDealer_submit_proof: TxDescriptor<{
  proof: Anonymize<Itom7fk49o0c9>;
  root: Binary;
  challenge_block: number;
  provider: Anonymize<I17k3ujudqd5df>;
}> = "4rvc765f9unuv" as TxDescriptor<{
  proof: Anonymize<Itom7fk49o0c9>;
  root: Binary;
  challenge_block: number;
  provider: Anonymize<I17k3ujudqd5df>;
}>;
const call_ProofsDealer_new_challenges_round: TxDescriptor<undefined> =
  c39ibapn5dm6tr as TxDescriptor<undefined>;
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
const evt_ParachainSystem_ValidationFunctionStored: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const evt_ParachainSystem_ValidationFunctionApplied: PlainDescriptor<{
  relay_chain_block_num: number;
}> = "dd7hd99u0ho0n" as PlainDescriptor<{ relay_chain_block_num: number }>;
const evt_ParachainSystem_ValidationFunctionDiscarded: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const evt_ParachainSystem_UpgradeAuthorized: PlainDescriptor<{ code_hash: Binary }> =
  "6a5n5ij3gomuo" as PlainDescriptor<{ code_hash: Binary }>;
const evt_ParachainSystem_DownwardMessagesReceived: PlainDescriptor<{ count: number }> =
  "afscmv8tjf0ou" as PlainDescriptor<{ count: number }>;
const evt_ParachainSystem_DownwardMessagesProcessed: PlainDescriptor<{
  weight_used: Anonymize<I4q39t5hn830vp>;
  dmq_head: Binary;
}> = "7a3a6ua4hud3s" as PlainDescriptor<{
  weight_used: Anonymize<I4q39t5hn830vp>;
  dmq_head: Binary;
}>;
const evt_ParachainSystem_UpwardMessageSent: PlainDescriptor<{
  message_hash: Anonymize<I17k3ujudqd5df>;
}> = "4n7056p1k6c8b" as PlainDescriptor<{ message_hash: Anonymize<I17k3ujudqd5df> }>;
const evt_Balances_Endowed: PlainDescriptor<{ account: SS58String; free_balance: bigint }> =
  "cv68aq8841478" as PlainDescriptor<{ account: SS58String; free_balance: bigint }>;
const evt_Balances_DustLost: PlainDescriptor<{ account: SS58String; amount: bigint }> =
  "c262ibdoec56a" as PlainDescriptor<{ account: SS58String; amount: bigint }>;
const evt_Balances_Transfer: PlainDescriptor<{ from: SS58String; to: SS58String; amount: bigint }> =
  "flcfm9b6nlmdd" as PlainDescriptor<{ from: SS58String; to: SS58String; amount: bigint }>;
const evt_Balances_BalanceSet: PlainDescriptor<{ who: SS58String; free: bigint }> =
  "jrsf4mnp3eka" as PlainDescriptor<{ who: SS58String; free: bigint }>;
const evt_Balances_Reserved: PlainDescriptor<{ who: SS58String; amount: bigint }> =
  cd5fm4p8lj5qgi as PlainDescriptor<{ who: SS58String; amount: bigint }>;
const evt_Balances_Unreserved: PlainDescriptor<{ who: SS58String; amount: bigint }> =
  cd5fm4p8lj5qgi as PlainDescriptor<{ who: SS58String; amount: bigint }>;
const evt_Balances_ReserveRepatriated: PlainDescriptor<{
  from: SS58String;
  to: SS58String;
  amount: bigint;
  destination_status: BalanceStatus;
}> = "dm5rqp3duosod" as PlainDescriptor<{
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
const evt_TransactionPayment_TransactionFeePaid: PlainDescriptor<{
  who: SS58String;
  actual_fee: bigint;
  tip: bigint;
}> = "er2cke86dqbr2" as PlainDescriptor<{ who: SS58String; actual_fee: bigint; tip: bigint }>;
const evt_Sudo_Sudid: PlainDescriptor<{ sudo_result: Anonymize<Idtdr91jmq5g4i> }> =
  c331o7t2g0ooi9 as PlainDescriptor<{ sudo_result: Anonymize<Idtdr91jmq5g4i> }>;
const evt_Sudo_KeyChanged: PlainDescriptor<{ old: Anonymize<Ihfphjolmsqq1>; new: SS58String }> =
  "5rtkmhm2dng4u" as PlainDescriptor<{ old: Anonymize<Ihfphjolmsqq1>; new: SS58String }>;
const evt_Sudo_KeyRemoved: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const evt_Sudo_SudoAsDone: PlainDescriptor<{ sudo_result: Anonymize<Idtdr91jmq5g4i> }> =
  c331o7t2g0ooi9 as PlainDescriptor<{ sudo_result: Anonymize<Idtdr91jmq5g4i> }>;
const evt_CollatorSelection_NewInvulnerables: PlainDescriptor<{
  invulnerables: Anonymize<Ia2lhg7l2hilo3>;
}> = "39t01nnod9109" as PlainDescriptor<{ invulnerables: Anonymize<Ia2lhg7l2hilo3> }>;
const evt_CollatorSelection_InvulnerableAdded: PlainDescriptor<{ account_id: SS58String }> =
  c6v8sm60vvkmk7 as PlainDescriptor<{ account_id: SS58String }>;
const evt_CollatorSelection_InvulnerableRemoved: PlainDescriptor<{ account_id: SS58String }> =
  c6v8sm60vvkmk7 as PlainDescriptor<{ account_id: SS58String }>;
const evt_CollatorSelection_NewDesiredCandidates: PlainDescriptor<{ desired_candidates: number }> =
  "1qmtmbe5so8r3" as PlainDescriptor<{ desired_candidates: number }>;
const evt_CollatorSelection_NewCandidacyBond: PlainDescriptor<{ bond_amount: bigint }> =
  "h99m6ehpcar7" as PlainDescriptor<{ bond_amount: bigint }>;
const evt_CollatorSelection_CandidateAdded: PlainDescriptor<{
  account_id: SS58String;
  deposit: bigint;
}> = cdgorhsbgdq2ap as PlainDescriptor<{ account_id: SS58String; deposit: bigint }>;
const evt_CollatorSelection_CandidateBondUpdated: PlainDescriptor<{
  account_id: SS58String;
  deposit: bigint;
}> = cdgorhsbgdq2ap as PlainDescriptor<{ account_id: SS58String; deposit: bigint }>;
const evt_CollatorSelection_CandidateRemoved: PlainDescriptor<{ account_id: SS58String }> =
  c6v8sm60vvkmk7 as PlainDescriptor<{ account_id: SS58String }>;
const evt_CollatorSelection_CandidateReplaced: PlainDescriptor<{
  old: SS58String;
  new: SS58String;
  deposit: bigint;
}> = "9ubb2kqevnu6t" as PlainDescriptor<{ old: SS58String; new: SS58String; deposit: bigint }>;
const evt_CollatorSelection_InvalidInvulnerableSkipped: PlainDescriptor<{
  account_id: SS58String;
}> = c6v8sm60vvkmk7 as PlainDescriptor<{ account_id: SS58String }>;
const evt_Session_NewSession: PlainDescriptor<{ session_index: number }> =
  "2hq50pu2kdjpo" as PlainDescriptor<{ session_index: number }>;
const evt_XcmpQueue_XcmpMessageSent: PlainDescriptor<{ message_hash: Binary }> =
  "2vo9trn8nhllu" as PlainDescriptor<{ message_hash: Binary }>;
const evt_PolkadotXcm_Attempted: PlainDescriptor<{ outcome: XcmV3TraitsOutcome }> =
  "4e7dkr4hrus3u" as PlainDescriptor<{ outcome: XcmV3TraitsOutcome }>;
const evt_PolkadotXcm_Sent: PlainDescriptor<{
  origin: Anonymize<Ie897ubj3a1vaq>;
  destination: Anonymize<Ie897ubj3a1vaq>;
  message: Anonymize<I50ghg3dhe8sh3>;
  message_id: Binary;
}> = "cr67tdr3h1l9n" as PlainDescriptor<{
  origin: Anonymize<Ie897ubj3a1vaq>;
  destination: Anonymize<Ie897ubj3a1vaq>;
  message: Anonymize<I50ghg3dhe8sh3>;
  message_id: Binary;
}>;
const evt_PolkadotXcm_UnexpectedResponse: PlainDescriptor<{
  origin: Anonymize<Ie897ubj3a1vaq>;
  query_id: bigint;
}> = cdrsgrbh5b6rje as PlainDescriptor<{ origin: Anonymize<Ie897ubj3a1vaq>; query_id: bigint }>;
const evt_PolkadotXcm_ResponseReady: PlainDescriptor<{
  query_id: bigint;
  response: XcmV3Response;
}> = "5s81678scdptl" as PlainDescriptor<{ query_id: bigint; response: XcmV3Response }>;
const evt_PolkadotXcm_Notified: PlainDescriptor<{
  query_id: bigint;
  pallet_index: number;
  call_index: number;
}> = c2uqmls7kcdnii as PlainDescriptor<{
  query_id: bigint;
  pallet_index: number;
  call_index: number;
}>;
const evt_PolkadotXcm_NotifyOverweight: PlainDescriptor<{
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
const evt_PolkadotXcm_NotifyDispatchError: PlainDescriptor<{
  query_id: bigint;
  pallet_index: number;
  call_index: number;
}> = c2uqmls7kcdnii as PlainDescriptor<{
  query_id: bigint;
  pallet_index: number;
  call_index: number;
}>;
const evt_PolkadotXcm_NotifyDecodeFailed: PlainDescriptor<{
  query_id: bigint;
  pallet_index: number;
  call_index: number;
}> = c2uqmls7kcdnii as PlainDescriptor<{
  query_id: bigint;
  pallet_index: number;
  call_index: number;
}>;
const evt_PolkadotXcm_InvalidResponder: PlainDescriptor<{
  origin: Anonymize<Ie897ubj3a1vaq>;
  query_id: bigint;
  expected_location: Anonymize<I189rbbmttkf8v>;
}> = "dje8f9lv4sogt" as PlainDescriptor<{
  origin: Anonymize<Ie897ubj3a1vaq>;
  query_id: bigint;
  expected_location: Anonymize<I189rbbmttkf8v>;
}>;
const evt_PolkadotXcm_InvalidResponderVersion: PlainDescriptor<{
  origin: Anonymize<Ie897ubj3a1vaq>;
  query_id: bigint;
}> = cdrsgrbh5b6rje as PlainDescriptor<{ origin: Anonymize<Ie897ubj3a1vaq>; query_id: bigint }>;
const evt_PolkadotXcm_ResponseTaken: PlainDescriptor<{ query_id: bigint }> =
  "30pg328m00nr3" as PlainDescriptor<{ query_id: bigint }>;
const evt_PolkadotXcm_AssetsTrapped: PlainDescriptor<{
  hash: Binary;
  origin: Anonymize<Ie897ubj3a1vaq>;
  assets: Anonymize<I2tnkj3t3en8tf>;
}> = c2pd6nni2u8392 as PlainDescriptor<{
  hash: Binary;
  origin: Anonymize<Ie897ubj3a1vaq>;
  assets: Anonymize<I2tnkj3t3en8tf>;
}>;
const evt_PolkadotXcm_VersionChangeNotified: PlainDescriptor<{
  destination: Anonymize<Ie897ubj3a1vaq>;
  result: number;
  cost: Anonymize<I2pdjq1umlp617>;
  message_id: Binary;
}> = "6s4eucqd88i6a" as PlainDescriptor<{
  destination: Anonymize<Ie897ubj3a1vaq>;
  result: number;
  cost: Anonymize<I2pdjq1umlp617>;
  message_id: Binary;
}>;
const evt_PolkadotXcm_SupportedVersionChanged: PlainDescriptor<{
  location: Anonymize<Ie897ubj3a1vaq>;
  version: number;
}> = ce9it7tqcnjnfj as PlainDescriptor<{ location: Anonymize<Ie897ubj3a1vaq>; version: number }>;
const evt_PolkadotXcm_NotifyTargetSendFail: PlainDescriptor<{
  location: Anonymize<Ie897ubj3a1vaq>;
  query_id: bigint;
  error: XcmV3TraitsError;
}> = "5lfvfuumat5pq" as PlainDescriptor<{
  location: Anonymize<Ie897ubj3a1vaq>;
  query_id: bigint;
  error: XcmV3TraitsError;
}>;
const evt_PolkadotXcm_NotifyTargetMigrationFail: PlainDescriptor<{
  location: Anonymize<Ib29ie59v4nmjq>;
  query_id: bigint;
}> = "qsl7ltbtjavb" as PlainDescriptor<{ location: Anonymize<Ib29ie59v4nmjq>; query_id: bigint }>;
const evt_PolkadotXcm_InvalidQuerierVersion: PlainDescriptor<{
  origin: Anonymize<Ie897ubj3a1vaq>;
  query_id: bigint;
}> = cdrsgrbh5b6rje as PlainDescriptor<{ origin: Anonymize<Ie897ubj3a1vaq>; query_id: bigint }>;
const evt_PolkadotXcm_InvalidQuerier: PlainDescriptor<{
  origin: Anonymize<Ie897ubj3a1vaq>;
  query_id: bigint;
  expected_querier: Anonymize<Ie897ubj3a1vaq>;
  maybe_actual_querier: Anonymize<I189rbbmttkf8v>;
}> = "ev28bbfu8eghg" as PlainDescriptor<{
  origin: Anonymize<Ie897ubj3a1vaq>;
  query_id: bigint;
  expected_querier: Anonymize<Ie897ubj3a1vaq>;
  maybe_actual_querier: Anonymize<I189rbbmttkf8v>;
}>;
const evt_PolkadotXcm_VersionNotifyStarted: PlainDescriptor<{
  destination: Anonymize<Ie897ubj3a1vaq>;
  cost: Anonymize<I2pdjq1umlp617>;
  message_id: Binary;
}> = c14amtmubrpgc8 as PlainDescriptor<{
  destination: Anonymize<Ie897ubj3a1vaq>;
  cost: Anonymize<I2pdjq1umlp617>;
  message_id: Binary;
}>;
const evt_PolkadotXcm_VersionNotifyRequested: PlainDescriptor<{
  destination: Anonymize<Ie897ubj3a1vaq>;
  cost: Anonymize<I2pdjq1umlp617>;
  message_id: Binary;
}> = c14amtmubrpgc8 as PlainDescriptor<{
  destination: Anonymize<Ie897ubj3a1vaq>;
  cost: Anonymize<I2pdjq1umlp617>;
  message_id: Binary;
}>;
const evt_PolkadotXcm_VersionNotifyUnrequested: PlainDescriptor<{
  destination: Anonymize<Ie897ubj3a1vaq>;
  cost: Anonymize<I2pdjq1umlp617>;
  message_id: Binary;
}> = c14amtmubrpgc8 as PlainDescriptor<{
  destination: Anonymize<Ie897ubj3a1vaq>;
  cost: Anonymize<I2pdjq1umlp617>;
  message_id: Binary;
}>;
const evt_PolkadotXcm_FeesPaid: PlainDescriptor<{
  paying: Anonymize<Ie897ubj3a1vaq>;
  fees: Anonymize<I2pdjq1umlp617>;
}> = "4tgpelgtlb6pi" as PlainDescriptor<{
  paying: Anonymize<Ie897ubj3a1vaq>;
  fees: Anonymize<I2pdjq1umlp617>;
}>;
const evt_PolkadotXcm_AssetsClaimed: PlainDescriptor<{
  hash: Binary;
  origin: Anonymize<Ie897ubj3a1vaq>;
  assets: Anonymize<I2tnkj3t3en8tf>;
}> = c2pd6nni2u8392 as PlainDescriptor<{
  hash: Binary;
  origin: Anonymize<Ie897ubj3a1vaq>;
  assets: Anonymize<I2tnkj3t3en8tf>;
}>;
const evt_CumulusXcm_InvalidFormat: PlainDescriptor<Binary> =
  c3s707ha9gi47u as PlainDescriptor<Binary>;
const evt_CumulusXcm_UnsupportedVersion: PlainDescriptor<Binary> =
  c3s707ha9gi47u as PlainDescriptor<Binary>;
const evt_CumulusXcm_ExecutedDownward: PlainDescriptor<[Binary, XcmV3TraitsOutcome]> =
  "d0ii3t0e6fgob" as PlainDescriptor<[Binary, XcmV3TraitsOutcome]>;
const evt_MessageQueue_ProcessingFailed: PlainDescriptor<{
  id: Binary;
  origin: Anonymize<Ifqm8uoikppunt>;
  error: ProcessMessageError;
}> = "82vnvii3s8i35" as PlainDescriptor<{
  id: Binary;
  origin: Anonymize<Ifqm8uoikppunt>;
  error: ProcessMessageError;
}>;
const evt_MessageQueue_Processed: PlainDescriptor<{
  id: Binary;
  origin: Anonymize<Ifqm8uoikppunt>;
  weight_used: Anonymize<I4q39t5hn830vp>;
  success: boolean;
}> = "dgsr5mj02rcf9" as PlainDescriptor<{
  id: Binary;
  origin: Anonymize<Ifqm8uoikppunt>;
  weight_used: Anonymize<I4q39t5hn830vp>;
  success: boolean;
}>;
const evt_MessageQueue_OverweightEnqueued: PlainDescriptor<{
  id: Binary;
  origin: Anonymize<Ifqm8uoikppunt>;
  page_index: number;
  message_index: number;
}> = "9l2k151rfodj4" as PlainDescriptor<{
  id: Binary;
  origin: Anonymize<Ifqm8uoikppunt>;
  page_index: number;
  message_index: number;
}>;
const evt_MessageQueue_PageReaped: PlainDescriptor<{
  origin: Anonymize<Ifqm8uoikppunt>;
  index: number;
}> = "6947e8h0ume2q" as PlainDescriptor<{ origin: Anonymize<Ifqm8uoikppunt>; index: number }>;
const evt_Providers_MspSignUpSuccess: PlainDescriptor<{
  who: SS58String;
  multiaddresses: Anonymize<Itom7fk49o0c9>;
  capacity: number;
  value_prop: Anonymize<Ienf50imfp828o>;
}> = "5k3ihioq7rv8s" as PlainDescriptor<{
  who: SS58String;
  multiaddresses: Anonymize<Itom7fk49o0c9>;
  capacity: number;
  value_prop: Anonymize<Ienf50imfp828o>;
}>;
const evt_Providers_BspSignUpSuccess: PlainDescriptor<{
  who: SS58String;
  multiaddresses: Anonymize<Itom7fk49o0c9>;
  capacity: number;
}> = "6vdjrn3qfv9c0" as PlainDescriptor<{
  who: SS58String;
  multiaddresses: Anonymize<Itom7fk49o0c9>;
  capacity: number;
}>;
const evt_Providers_MspSignOffSuccess: PlainDescriptor<{ who: SS58String }> =
  c4cbvqmqadhrea as PlainDescriptor<{ who: SS58String }>;
const evt_Providers_BspSignOffSuccess: PlainDescriptor<{ who: SS58String }> =
  c4cbvqmqadhrea as PlainDescriptor<{ who: SS58String }>;
const evt_Providers_TotalDataChanged: PlainDescriptor<{
  who: SS58String;
  old_capacity: number;
  new_capacity: number;
}> = "1lnbp13mvbupl" as PlainDescriptor<{
  who: SS58String;
  old_capacity: number;
  new_capacity: number;
}>;
const evt_FileSystem_NewStorageRequest: PlainDescriptor<{
  who: SS58String;
  location: Binary;
  fingerprint: Binary;
  size: bigint;
  multiaddresses: Anonymize<Itom7fk49o0c9>;
}> = "crmmcf16le1rr" as PlainDescriptor<{
  who: SS58String;
  location: Binary;
  fingerprint: Binary;
  size: bigint;
  multiaddresses: Anonymize<Itom7fk49o0c9>;
}>;
const evt_FileSystem_AcceptedBspVolunteer: PlainDescriptor<{
  who: SS58String;
  location: Binary;
  fingerprint: Binary;
  multiaddresses: Anonymize<Itom7fk49o0c9>;
}> = "8c7cnrjg7sfkc" as PlainDescriptor<{
  who: SS58String;
  location: Binary;
  fingerprint: Binary;
  multiaddresses: Anonymize<Itom7fk49o0c9>;
}>;
const evt_FileSystem_StorageRequestExpired: PlainDescriptor<{ location: Binary }> =
  ceg5outd74d62b as PlainDescriptor<{ location: Binary }>;
const evt_FileSystem_StorageRequestRevoked: PlainDescriptor<{ location: Binary }> =
  ceg5outd74d62b as PlainDescriptor<{ location: Binary }>;
const evt_FileSystem_BspStoppedStoring: PlainDescriptor<{
  bsp: SS58String;
  file_key: Binary;
  owner: SS58String;
  location: Binary;
}> = "ddserqjgpfbdm" as PlainDescriptor<{
  bsp: SS58String;
  file_key: Binary;
  owner: SS58String;
  location: Binary;
}>;
const evt_ProofsDealer_NewChallenge: PlainDescriptor<{ who: SS58String; key_challenged: Binary }> =
  "4q3amn12h8qc9" as PlainDescriptor<{ who: SS58String; key_challenged: Binary }>;
const evt_ProofsDealer_ProofRejected: PlainDescriptor<{
  provider: Binary;
  proof: Anonymize<Itom7fk49o0c9>;
  reason: Anonymize<Ifhhbbpbpeqis>;
}> = "f5kdet8babji3" as PlainDescriptor<{
  provider: Binary;
  proof: Anonymize<Itom7fk49o0c9>;
  reason: Anonymize<Ifhhbbpbpeqis>;
}>;
const evt_ProofsDealer_ProofAccepted: PlainDescriptor<{
  provider: Binary;
  proof: Anonymize<Itom7fk49o0c9>;
}> = "9mls6vu7id41c" as PlainDescriptor<{ provider: Binary; proof: Anonymize<Itom7fk49o0c9> }>;
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
const err_ParachainSystem_OverlappingUpgrades: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ParachainSystem_ProhibitedByPolkadot: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ParachainSystem_TooBig: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ParachainSystem_ValidationDataNotAvailable: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ParachainSystem_HostConfigurationNotAvailable: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ParachainSystem_NotScheduled: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ParachainSystem_NothingAuthorized: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ParachainSystem_Unauthorized: PlainDescriptor<undefined> =
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
const err_Sudo_RequireSudo: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_CollatorSelection_TooManyCandidates: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_CollatorSelection_TooFewEligibleCollators: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_CollatorSelection_AlreadyCandidate: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_CollatorSelection_NotCandidate: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_CollatorSelection_TooManyInvulnerables: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_CollatorSelection_AlreadyInvulnerable: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_CollatorSelection_NotInvulnerable: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_CollatorSelection_NoAssociatedValidatorId: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_CollatorSelection_ValidatorNotRegistered: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_CollatorSelection_InsertToCandidateListFailed: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_CollatorSelection_RemoveFromCandidateListFailed: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_CollatorSelection_DepositTooLow: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_CollatorSelection_UpdateCandidateListFailed: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_CollatorSelection_InsufficientBond: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_CollatorSelection_TargetIsNotCandidate: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_CollatorSelection_IdenticalDeposit: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_CollatorSelection_InvalidUnreserve: PlainDescriptor<undefined> =
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
const err_XcmpQueue_BadQueueConfig: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_XcmpQueue_AlreadySuspended: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_XcmpQueue_AlreadyResumed: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_PolkadotXcm_Unreachable: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_PolkadotXcm_SendFailure: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_PolkadotXcm_Filtered: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_PolkadotXcm_UnweighableMessage: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_PolkadotXcm_DestinationNotInvertible: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_PolkadotXcm_Empty: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_PolkadotXcm_CannotReanchor: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_PolkadotXcm_TooManyAssets: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_PolkadotXcm_InvalidOrigin: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_PolkadotXcm_BadVersion: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_PolkadotXcm_BadLocation: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_PolkadotXcm_NoSubscription: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_PolkadotXcm_AlreadySubscribed: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_PolkadotXcm_CannotCheckOutTeleport: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_PolkadotXcm_LowBalance: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_PolkadotXcm_TooManyLocks: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_PolkadotXcm_AccountNotSovereign: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_PolkadotXcm_FeesNotMet: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_PolkadotXcm_LockNotFound: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_PolkadotXcm_InUse: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_PolkadotXcm_InvalidAssetNotConcrete: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_PolkadotXcm_InvalidAssetUnknownReserve: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_PolkadotXcm_InvalidAssetUnsupportedReserve: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_PolkadotXcm_TooManyReserves: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_PolkadotXcm_LocalExecutionIncomplete: PlainDescriptor<undefined> =
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
const err_Providers_AlreadyRegistered: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Providers_StorageTooLow: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Providers_NotEnoughBalance: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Providers_CannotHoldDeposit: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Providers_MaxBspsReached: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Providers_MaxMspsReached: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Providers_NotRegistered: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Providers_SpRegisteredButDataNotFound: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Providers_StorageStillInUse: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Providers_NotEnoughTimePassed: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Providers_NoUserId: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Providers_NoBucketId: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Providers_NoMultiAddress: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Providers_InvalidMultiAddress: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_Providers_Overflow: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_FileSystem_StorageRequestAlreadyRegistered: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_FileSystem_StorageRequestNotFound: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_FileSystem_BspsRequiredCannotBeZero: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_FileSystem_BspsRequiredExceedsMax: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_FileSystem_BspVolunteerFailed: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_FileSystem_StorageRequestBspsRequiredFulfilled: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_FileSystem_BspAlreadyVolunteered: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_FileSystem_StorageRequestExpiredNoSlotAvailable: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_FileSystem_StorageRequestNotAuthorized: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_FileSystem_MaxBlockNumberReached: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ProofsDealer_ChallengesQueueOverflow: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ProofsDealer_PriorityChallengesQueueOverflow: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ProofsDealer_NotProvider: PlainDescriptor<undefined> =
  c39ibapn5dm6tr as PlainDescriptor<undefined>;
const err_ProofsDealer_FeeChargeFailed: PlainDescriptor<undefined> =
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
const const_Timestamp_MinimumPeriod: PlainDescriptor<bigint> =
  c7d2tqf029vcod as PlainDescriptor<bigint>;
const const_Balances_ExistentialDeposit: PlainDescriptor<bigint> =
  c7d2tqf029vcod as PlainDescriptor<bigint>;
const const_Balances_MaxLocks: PlainDescriptor<number> = cf3ncnq6stc567 as PlainDescriptor<number>;
const const_Balances_MaxReserves: PlainDescriptor<number> =
  cf3ncnq6stc567 as PlainDescriptor<number>;
const const_Balances_MaxHolds: PlainDescriptor<number> = cf3ncnq6stc567 as PlainDescriptor<number>;
const const_Balances_MaxFreezes: PlainDescriptor<number> =
  cf3ncnq6stc567 as PlainDescriptor<number>;
const const_TransactionPayment_OperationalFeeMultiplier: PlainDescriptor<number> =
  cf3ncnq6stc567 as PlainDescriptor<number>;
const const_XcmpQueue_MaxInboundSuspended: PlainDescriptor<number> =
  cf3ncnq6stc567 as PlainDescriptor<number>;
const const_MessageQueue_HeapSize: PlainDescriptor<number> =
  cf3ncnq6stc567 as PlainDescriptor<number>;
const const_MessageQueue_MaxStale: PlainDescriptor<number> =
  cf3ncnq6stc567 as PlainDescriptor<number>;
const const_MessageQueue_ServiceWeight: PlainDescriptor<Anonymize<I4q39t5hn830vp> | undefined> =
  "asb8k6ash5mjn" as PlainDescriptor<Anonymize<I4q39t5hn830vp> | undefined>;
const const_Providers_SpMinDeposit: PlainDescriptor<bigint> =
  c7d2tqf029vcod as PlainDescriptor<bigint>;
const const_Providers_SpMinCapacity: PlainDescriptor<number> =
  cf3ncnq6stc567 as PlainDescriptor<number>;
const const_Providers_DepositPerData: PlainDescriptor<bigint> =
  c7d2tqf029vcod as PlainDescriptor<bigint>;
const const_Providers_MaxBsps: PlainDescriptor<number> = cf3ncnq6stc567 as PlainDescriptor<number>;
const const_Providers_MaxMsps: PlainDescriptor<number> = cf3ncnq6stc567 as PlainDescriptor<number>;
const const_Providers_MaxMultiAddressSize: PlainDescriptor<number> =
  cf3ncnq6stc567 as PlainDescriptor<number>;
const const_Providers_MaxMultiAddressAmount: PlainDescriptor<number> =
  cf3ncnq6stc567 as PlainDescriptor<number>;
const const_Providers_MaxProtocols: PlainDescriptor<number> =
  cf3ncnq6stc567 as PlainDescriptor<number>;
const const_Providers_MaxBuckets: PlainDescriptor<number> =
  cf3ncnq6stc567 as PlainDescriptor<number>;
const const_FileSystem_TargetBspsRequired: PlainDescriptor<number> =
  cf3ncnq6stc567 as PlainDescriptor<number>;
const const_FileSystem_MaxBspsPerStorageRequest: PlainDescriptor<number> =
  cf3ncnq6stc567 as PlainDescriptor<number>;
const const_FileSystem_MaxFilePathSize: PlainDescriptor<number> =
  cf3ncnq6stc567 as PlainDescriptor<number>;
const const_FileSystem_MaxMultiAddressSize: PlainDescriptor<number> =
  cf3ncnq6stc567 as PlainDescriptor<number>;
const const_FileSystem_MaxMultiAddresses: PlainDescriptor<number> =
  cf3ncnq6stc567 as PlainDescriptor<number>;
const const_FileSystem_StorageRequestTtl: PlainDescriptor<number> =
  cf3ncnq6stc567 as PlainDescriptor<number>;
const const_FileSystem_MaxExpiredStorageRequests: PlainDescriptor<number> =
  cf3ncnq6stc567 as PlainDescriptor<number>;
const const_ProofsDealer_MaxChallengesPerBlock: PlainDescriptor<number> =
  cf3ncnq6stc567 as PlainDescriptor<number>;
const const_ProofsDealer_MaxProvidersChallengedPerBlock: PlainDescriptor<number> =
  cf3ncnq6stc567 as PlainDescriptor<number>;
const const_ProofsDealer_ChallengeHistoryLength: PlainDescriptor<number> =
  cf3ncnq6stc567 as PlainDescriptor<number>;
const const_ProofsDealer_ChallengesQueueLength: PlainDescriptor<number> =
  cf3ncnq6stc567 as PlainDescriptor<number>;
const const_ProofsDealer_CheckpointChallengePeriod: PlainDescriptor<number> =
  cf3ncnq6stc567 as PlainDescriptor<number>;
const const_ProofsDealer_ChallengesFee: PlainDescriptor<bigint> =
  c7d2tqf029vcod as PlainDescriptor<bigint>;
const const_ProofsDealer_Treasury: PlainDescriptor<SS58String> =
  c86esvcpejimq9 as PlainDescriptor<SS58String>;
const runtime_AuraApi_slot_duration: RuntimeDescriptor<[], bigint> =
  "b54qab7tpjjsf" as RuntimeDescriptor<[], bigint>;
const runtime_AuraApi_authorities: RuntimeDescriptor<
  [],
  Array<Binary>
> = "2tjq7ncksi1hj" as RuntimeDescriptor<[], Array<Binary>>;
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
const runtime_TransactionPaymentCallApi_query_call_info: RuntimeDescriptor<
  [
    call: Anonymize<
      AnonymousEnum<{
        System: Anonymize<SystemPalletCall>;
        ParachainSystem: Anonymize<Ia0jlnena5ajog>;
        Timestamp: Anonymize<TimestampPalletCall>;
        ParachainInfo: Anonymize<undefined>;
        Balances: Anonymize<Ibf8j84ii3a3kr>;
        Sudo: Anonymize<Iam913892vifu6>;
        CollatorSelection: Anonymize<I6ggjare8v1go5>;
        Session: Anonymize<I3v8vq7j9grsdj>;
        XcmpQueue: Anonymize<I286uete0pvcbe>;
        PolkadotXcm: Anonymize<I3br2bgla1bs2h>;
        CumulusXcm: Anonymize<undefined>;
        MessageQueue: Anonymize<I8lmlccfrohcqg>;
        Providers: Anonymize<I9jhevh1bis85g>;
        FileSystem: Anonymize<I8u4nbk1d32u7q>;
        ProofsDealer: Anonymize<Iaoc2q2c87hkb1>;
      }>
    >,
    len: number,
  ],
  { weight: Anonymize<I4q39t5hn830vp>; class: DispatchClass; partial_fee: bigint }
> = "7i8bq12jf6f7a" as RuntimeDescriptor<
  [
    call: Anonymize<
      AnonymousEnum<{
        System: Anonymize<SystemPalletCall>;
        ParachainSystem: Anonymize<Ia0jlnena5ajog>;
        Timestamp: Anonymize<TimestampPalletCall>;
        ParachainInfo: Anonymize<undefined>;
        Balances: Anonymize<Ibf8j84ii3a3kr>;
        Sudo: Anonymize<Iam913892vifu6>;
        CollatorSelection: Anonymize<I6ggjare8v1go5>;
        Session: Anonymize<I3v8vq7j9grsdj>;
        XcmpQueue: Anonymize<I286uete0pvcbe>;
        PolkadotXcm: Anonymize<I3br2bgla1bs2h>;
        CumulusXcm: Anonymize<undefined>;
        MessageQueue: Anonymize<I8lmlccfrohcqg>;
        Providers: Anonymize<I9jhevh1bis85g>;
        FileSystem: Anonymize<I8u4nbk1d32u7q>;
        ProofsDealer: Anonymize<Iaoc2q2c87hkb1>;
      }>
    >,
    len: number,
  ],
  { weight: Anonymize<I4q39t5hn830vp>; class: DispatchClass; partial_fee: bigint }
>;
const runtime_TransactionPaymentCallApi_query_call_fee_details: RuntimeDescriptor<
  [
    call: Anonymize<
      AnonymousEnum<{
        System: Anonymize<SystemPalletCall>;
        ParachainSystem: Anonymize<Ia0jlnena5ajog>;
        Timestamp: Anonymize<TimestampPalletCall>;
        ParachainInfo: Anonymize<undefined>;
        Balances: Anonymize<Ibf8j84ii3a3kr>;
        Sudo: Anonymize<Iam913892vifu6>;
        CollatorSelection: Anonymize<I6ggjare8v1go5>;
        Session: Anonymize<I3v8vq7j9grsdj>;
        XcmpQueue: Anonymize<I286uete0pvcbe>;
        PolkadotXcm: Anonymize<I3br2bgla1bs2h>;
        CumulusXcm: Anonymize<undefined>;
        MessageQueue: Anonymize<I8lmlccfrohcqg>;
        Providers: Anonymize<I9jhevh1bis85g>;
        FileSystem: Anonymize<I8u4nbk1d32u7q>;
        ProofsDealer: Anonymize<Iaoc2q2c87hkb1>;
      }>
    >,
    len: number,
  ],
  { inclusion_fee: Anonymize<Id37fum600qfau>; tip: bigint }
> = "aku79oal869n7" as RuntimeDescriptor<
  [
    call: Anonymize<
      AnonymousEnum<{
        System: Anonymize<SystemPalletCall>;
        ParachainSystem: Anonymize<Ia0jlnena5ajog>;
        Timestamp: Anonymize<TimestampPalletCall>;
        ParachainInfo: Anonymize<undefined>;
        Balances: Anonymize<Ibf8j84ii3a3kr>;
        Sudo: Anonymize<Iam913892vifu6>;
        CollatorSelection: Anonymize<I6ggjare8v1go5>;
        Session: Anonymize<I3v8vq7j9grsdj>;
        XcmpQueue: Anonymize<I286uete0pvcbe>;
        PolkadotXcm: Anonymize<I3br2bgla1bs2h>;
        CumulusXcm: Anonymize<undefined>;
        MessageQueue: Anonymize<I8lmlccfrohcqg>;
        Providers: Anonymize<I9jhevh1bis85g>;
        FileSystem: Anonymize<I8u4nbk1d32u7q>;
        ProofsDealer: Anonymize<Iaoc2q2c87hkb1>;
      }>
    >,
    len: number,
  ],
  { inclusion_fee: Anonymize<Id37fum600qfau>; tip: bigint }
>;
const runtime_TransactionPaymentCallApi_query_weight_to_fee: RuntimeDescriptor<
  [weight: { ref_time: bigint; proof_size: bigint }],
  bigint
> = "9fha9spid57qj" as RuntimeDescriptor<
  [weight: { ref_time: bigint; proof_size: bigint }],
  bigint
>;
const runtime_TransactionPaymentCallApi_query_length_to_fee: RuntimeDescriptor<
  [length: number],
  bigint
> = "5a7vbbavuk0jq" as RuntimeDescriptor<[length: number], bigint>;
const runtime_CollectCollationInfo_collect_collation_info: RuntimeDescriptor<
  [
    header: {
      parent_hash: Binary;
      number: number;
      state_root: Binary;
      extrinsics_root: Binary;
      digest: Anonymize<Idin6nhq46lvdj>;
    },
  ],
  {
    upward_messages: Anonymize<Itom7fk49o0c9>;
    horizontal_messages: Anonymize<I6r5cbv8ttrb09>;
    new_validation_code: Anonymize<Iabpgqcjikia83>;
    processed_downward_messages: number;
    hrmp_watermark: number;
    head_data: Binary;
  }
> = "9th1phn8v89r5" as RuntimeDescriptor<
  [
    header: {
      parent_hash: Binary;
      number: number;
      state_root: Binary;
      extrinsics_root: Binary;
      digest: Anonymize<Idin6nhq46lvdj>;
    },
  ],
  {
    upward_messages: Anonymize<Itom7fk49o0c9>;
    horizontal_messages: Anonymize<I6r5cbv8ttrb09>;
    new_validation_code: Anonymize<Iabpgqcjikia83>;
    processed_downward_messages: number;
    hrmp_watermark: number;
    head_data: Binary;
  }
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
      Events: StorageDescriptor<[], Array<Anonymize<Idvbs8vg3olusq>>, false>;

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
  ParachainSystem: [
    {
      /**
       * Latest included block descendants the runtime accepted. In other words, these are
       * ancestors of the currently executing block which have not been included in the observed
       * relay-chain state.
       *
       * The segment length is limited by the capacity returned from the [`ConsensusHook`] configured
       * in the pallet.
       */
      UnincludedSegment: StorageDescriptor<[], Array<Anonymize<I8ajtuet8esesv>>, false>;

      /**
       * Storage field that keeps track of bandwidth used by the unincluded segment along with the
       * latest HRMP watermark. Used for limiting the acceptance of new blocks with
       * respect to relay chain constraints.
       */
      AggregatedUnincludedSegment: StorageDescriptor<
        [],
        {
          used_bandwidth: Anonymize<Ieafp1gui1o4cl>;
          hrmp_watermark: Anonymize<I4arjljr6dpflb>;
          consumed_go_ahead_signal: Anonymize<Ie1vdku2j6ccvj>;
        },
        true
      >;

      /**
       * In case of a scheduled upgrade, this storage field contains the validation code to be
       * applied.
       *
       * As soon as the relay chain gives us the go-ahead signal, we will overwrite the
       * [`:code`][sp_core::storage::well_known_keys::CODE] which will result the next block process
       * with the new validation code. This concludes the upgrade process.
       */
      PendingValidationCode: StorageDescriptor<[], Binary, false>;

      /**
       * Validation code that is set by the parachain and is to be communicated to collator and
       * consequently the relay-chain.
       *
       * This will be cleared in `on_initialize` of each new block if no other pallet already set
       * the value.
       */
      NewValidationCode: StorageDescriptor<[], Binary, true>;

      /**
       * The [`PersistedValidationData`] set for this block.
       * This value is expected to be set only once per block and it's never stored
       * in the trie.
       */
      ValidationData: StorageDescriptor<
        [],
        {
          parent_head: Binary;
          relay_parent_number: number;
          relay_parent_storage_root: Binary;
          max_pov_size: number;
        },
        true
      >;

      /**
       * Were the validation data set to notify the relay chain?
       */
      DidSetValidationCode: StorageDescriptor<[], boolean, false>;

      /**
       * The relay chain block number associated with the last parachain block.
       *
       * This is updated in `on_finalize`.
       */
      LastRelayChainBlockNumber: StorageDescriptor<[], number, false>;

      /**
       * An option which indicates if the relay-chain restricts signalling a validation code upgrade.
       * In other words, if this is `Some` and [`NewValidationCode`] is `Some` then the produced
       * candidate will be invalid.
       *
       * This storage item is a mirror of the corresponding value for the current parachain from the
       * relay-chain. This value is ephemeral which means it doesn't hit the storage. This value is
       * set after the inherent.
       */
      UpgradeRestrictionSignal: StorageDescriptor<
        [],
        PolkadotPrimitivesV5UpgradeRestriction | undefined,
        false
      >;

      /**
       * Optional upgrade go-ahead signal from the relay-chain.
       *
       * This storage item is a mirror of the corresponding value for the current parachain from the
       * relay-chain. This value is ephemeral which means it doesn't hit the storage. This value is
       * set after the inherent.
       */
      UpgradeGoAhead: StorageDescriptor<[], PolkadotPrimitivesV5UpgradeGoAhead | undefined, false>;

      /**
       * The state proof for the last relay parent block.
       *
       * This field is meant to be updated each block with the validation data inherent. Therefore,
       * before processing of the inherent, e.g. in `on_initialize` this data may be stale.
       *
       * This data is also absent from the genesis.
       */
      RelayStateProof: StorageDescriptor<[], Array<Binary>, true>;

      /**
       * The snapshot of some state related to messaging relevant to the current parachain as per
       * the relay parent.
       *
       * This field is meant to be updated each block with the validation data inherent. Therefore,
       * before processing of the inherent, e.g. in `on_initialize` this data may be stale.
       *
       * This data is also absent from the genesis.
       */
      RelevantMessagingState: StorageDescriptor<
        [],
        {
          dmq_mqc_head: Binary;
          relay_dispatch_queue_remaining_capacity: Anonymize<I3j1v1c2btq4bd>;
          ingress_channels: Anonymize<I90nfahji0n33j>;
          egress_channels: Anonymize<I90nfahji0n33j>;
        },
        true
      >;

      /**
       * The parachain host configuration that was obtained from the relay parent.
       *
       * This field is meant to be updated each block with the validation data inherent. Therefore,
       * before processing of the inherent, e.g. in `on_initialize` this data may be stale.
       *
       * This data is also absent from the genesis.
       */
      HostConfiguration: StorageDescriptor<
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
        },
        true
      >;

      /**
       * The last downward message queue chain head we have observed.
       *
       * This value is loaded before and saved after processing inbound downward messages carried
       * by the system inherent.
       */
      LastDmqMqcHead: StorageDescriptor<[], Binary, false>;

      /**
       * The message queue chain heads we have observed per each channel incoming channel.
       *
       * This value is loaded before and saved after processing inbound downward messages carried
       * by the system inherent.
       */
      LastHrmpMqcHeads: StorageDescriptor<[], Array<Anonymize<If89923vhoiaim>>, false>;

      /**
       * Number of downward messages processed in a block.
       *
       * This will be cleared in `on_initialize` of each new block.
       */
      ProcessedDownwardMessages: StorageDescriptor<[], number, false>;

      /**
       * HRMP watermark that was set in a block.
       *
       * This will be cleared in `on_initialize` of each new block.
       */
      HrmpWatermark: StorageDescriptor<[], number, false>;

      /**
       * HRMP messages that were sent in a block.
       *
       * This will be cleared in `on_initialize` of each new block.
       */
      HrmpOutboundMessages: StorageDescriptor<[], Array<Anonymize<I958l48g4qg5rf>>, false>;

      /**
       * Upward messages that were sent in a block.
       *
       * This will be cleared in `on_initialize` of each new block.
       */
      UpwardMessages: StorageDescriptor<[], Array<Binary>, false>;

      /**
       * Upward messages that are still pending and not yet send to the relay chain.
       */
      PendingUpwardMessages: StorageDescriptor<[], Array<Binary>, false>;

      /**
       * The factor to multiply the base delivery fee by for UMP.
       */
      UpwardDeliveryFeeFactor: StorageDescriptor<[], bigint, false>;

      /**
       * The number of HRMP messages we observed in `on_initialize` and thus used that number for
       * announcing the weight of `on_initialize` and `on_finalize`.
       */
      AnnouncedHrmpMessagesPerCandidate: StorageDescriptor<[], number, false>;

      /**
       * The weight we reserve at the beginning of the block for processing XCMP messages. This
       * overrides the amount set in the Config trait.
       */
      ReservedXcmpWeightOverride: StorageDescriptor<
        [],
        { ref_time: bigint; proof_size: bigint },
        true
      >;

      /**
       * The weight we reserve at the beginning of the block for processing DMP messages. This
       * overrides the amount set in the Config trait.
       */
      ReservedDmpWeightOverride: StorageDescriptor<
        [],
        { ref_time: bigint; proof_size: bigint },
        true
      >;

      /**
       * The next authorized upgrade, if there is one.
       */
      AuthorizedUpgrade: StorageDescriptor<[], { code_hash: Binary; check_version: boolean }, true>;

      /**
       * A custom head data that should be returned as result of `validate_block`.
       *
       * See `Pallet::set_custom_validation_head_data` for more information.
       */
      CustomValidationHeadData: StorageDescriptor<[], Binary, true>;
    },
    {
      /**
       *See [`Pallet::set_validation_data`].
       */
      set_validation_data: TxDescriptor<{ data: Anonymize<Icj9r7l64kc5ku> }>;

      /**
       *See [`Pallet::sudo_send_upward_message`].
       */
      sudo_send_upward_message: TxDescriptor<{ message: Binary }>;

      /**
       *See [`Pallet::authorize_upgrade`].
       */
      authorize_upgrade: TxDescriptor<{ code_hash: Binary; check_version: boolean }>;

      /**
       *See [`Pallet::enact_authorized_upgrade`].
       */
      enact_authorized_upgrade: TxDescriptor<{ code: Binary }>;
    },
    {
      /**
       *The validation function has been scheduled to apply.
       */
      ValidationFunctionStored: PlainDescriptor<undefined>;

      /**
       *The validation function was applied as of the contained relay chain block number.
       */
      ValidationFunctionApplied: PlainDescriptor<{ relay_chain_block_num: number }>;

      /**
       *The relay-chain aborted the upgrade process.
       */
      ValidationFunctionDiscarded: PlainDescriptor<undefined>;

      /**
       *An upgrade has been authorized.
       */
      UpgradeAuthorized: PlainDescriptor<{ code_hash: Binary }>;

      /**
       *Some downward messages have been received and will be processed.
       */
      DownwardMessagesReceived: PlainDescriptor<{ count: number }>;

      /**
       *Downward messages were processed using the given weight.
       */
      DownwardMessagesProcessed: PlainDescriptor<{
        weight_used: Anonymize<I4q39t5hn830vp>;
        dmq_head: Binary;
      }>;

      /**
       *An upward message was sent to the relay chain.
       */
      UpwardMessageSent: PlainDescriptor<{ message_hash: Anonymize<I17k3ujudqd5df> }>;
    },
    {
      /**
       *Attempt to upgrade validation function while existing upgrade pending.
       */
      OverlappingUpgrades: PlainDescriptor<undefined>;

      /**
       *Polkadot currently prohibits this parachain from upgrading its validation function.
       */
      ProhibitedByPolkadot: PlainDescriptor<undefined>;

      /**
       *The supplied validation function has compiled into a blob larger than Polkadot is
       *willing to run.
       */
      TooBig: PlainDescriptor<undefined>;

      /**
       *The inherent which supplies the validation data did not run this block.
       */
      ValidationDataNotAvailable: PlainDescriptor<undefined>;

      /**
       *The inherent which supplies the host configuration did not run this block.
       */
      HostConfigurationNotAvailable: PlainDescriptor<undefined>;

      /**
       *No validation function upgrade is currently scheduled.
       */
      NotScheduled: PlainDescriptor<undefined>;

      /**
       *No code upgrade has been authorized.
       */
      NothingAuthorized: PlainDescriptor<undefined>;

      /**
       *The given code upgrade has not been authorized.
       */
      Unauthorized: PlainDescriptor<undefined>;
    },
    {},
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
  ParachainInfo: [
    {
      /**

 */
      ParachainId: StorageDescriptor<[], number, false>;
    },
    {},
    {},
    {},
    {},
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
      Holds: StorageDescriptor<[Key: SS58String], Array<Anonymize<I66c0bbqofu6gm>>, false>;

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
       *Number of holds exceed `MaxHolds`.
       */
      TooManyHolds: PlainDescriptor<undefined>;

      /**
       *Number of freezes exceed `MaxFreezes`.
       */
      TooManyFreezes: PlainDescriptor<undefined>;
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
       * The maximum number of holds that can exist on an account at any time.
       */
      MaxHolds: PlainDescriptor<number>;

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
      sudo: TxDescriptor<{ call: Anonymize<Iupi52pl09tgg> }>;

      /**
       *See [`Pallet::sudo_unchecked_weight`].
       */
      sudo_unchecked_weight: TxDescriptor<{
        call: Anonymize<Iupi52pl09tgg>;
        weight: Anonymize<I4q39t5hn830vp>;
      }>;

      /**
       *See [`Pallet::set_key`].
       */
      set_key: TxDescriptor<{ new: MultiAddress }>;

      /**
       *See [`Pallet::sudo_as`].
       */
      sudo_as: TxDescriptor<{ who: MultiAddress; call: Anonymize<Iupi52pl09tgg> }>;

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
  CollatorSelection: [
    {
      /**
       * The invulnerable, permissioned collators. This list must be sorted.
       */
      Invulnerables: StorageDescriptor<[], Array<SS58String>, false>;

      /**
       * The (community, limited) collation candidates. `Candidates` and `Invulnerables` should be
       * mutually exclusive.
       *
       * This list is sorted in ascending order by deposit and when the deposits are equal, the least
       * recently updated is considered greater.
       */
      CandidateList: StorageDescriptor<[], Array<Anonymize<Iep1lmt6q3s6r3>>, false>;

      /**
       * Last block authored by collator.
       */
      LastAuthoredBlock: StorageDescriptor<[Key: SS58String], number, false>;

      /**
       * Desired number of candidates.
       *
       * This should ideally always be less than [`Config::MaxCandidates`] for weights to be correct.
       */
      DesiredCandidates: StorageDescriptor<[], number, false>;

      /**
       * Fixed amount to deposit to become a collator.
       *
       * When a collator calls `leave_intent` they immediately receive the deposit back.
       */
      CandidacyBond: StorageDescriptor<[], bigint, false>;
    },
    {
      /**
       *See [`Pallet::set_invulnerables`].
       */
      set_invulnerables: TxDescriptor<{ new: Anonymize<Ia2lhg7l2hilo3> }>;

      /**
       *See [`Pallet::set_desired_candidates`].
       */
      set_desired_candidates: TxDescriptor<{ max: number }>;

      /**
       *See [`Pallet::set_candidacy_bond`].
       */
      set_candidacy_bond: TxDescriptor<{ bond: bigint }>;

      /**
       *See [`Pallet::register_as_candidate`].
       */
      register_as_candidate: TxDescriptor<undefined>;

      /**
       *See [`Pallet::leave_intent`].
       */
      leave_intent: TxDescriptor<undefined>;

      /**
       *See [`Pallet::add_invulnerable`].
       */
      add_invulnerable: TxDescriptor<{ who: SS58String }>;

      /**
       *See [`Pallet::remove_invulnerable`].
       */
      remove_invulnerable: TxDescriptor<{ who: SS58String }>;

      /**
       *See [`Pallet::update_bond`].
       */
      update_bond: TxDescriptor<{ new_deposit: bigint }>;

      /**
       *See [`Pallet::take_candidate_slot`].
       */
      take_candidate_slot: TxDescriptor<{ deposit: bigint; target: SS58String }>;
    },
    {
      /**
       *New Invulnerables were set.
       */
      NewInvulnerables: PlainDescriptor<{ invulnerables: Anonymize<Ia2lhg7l2hilo3> }>;

      /**
       *A new Invulnerable was added.
       */
      InvulnerableAdded: PlainDescriptor<{ account_id: SS58String }>;

      /**
       *An Invulnerable was removed.
       */
      InvulnerableRemoved: PlainDescriptor<{ account_id: SS58String }>;

      /**
       *The number of desired candidates was set.
       */
      NewDesiredCandidates: PlainDescriptor<{ desired_candidates: number }>;

      /**
       *The candidacy bond was set.
       */
      NewCandidacyBond: PlainDescriptor<{ bond_amount: bigint }>;

      /**
       *A new candidate joined.
       */
      CandidateAdded: PlainDescriptor<{ account_id: SS58String; deposit: bigint }>;

      /**
       *Bond of a candidate updated.
       */
      CandidateBondUpdated: PlainDescriptor<{ account_id: SS58String; deposit: bigint }>;

      /**
       *A candidate was removed.
       */
      CandidateRemoved: PlainDescriptor<{ account_id: SS58String }>;

      /**
       *An account was replaced in the candidate list by another one.
       */
      CandidateReplaced: PlainDescriptor<{ old: SS58String; new: SS58String; deposit: bigint }>;

      /**
       *An account was unable to be added to the Invulnerables because they did not have keys
       *registered. Other Invulnerables may have been set.
       */
      InvalidInvulnerableSkipped: PlainDescriptor<{ account_id: SS58String }>;
    },
    {
      /**
       *The pallet has too many candidates.
       */
      TooManyCandidates: PlainDescriptor<undefined>;

      /**
       *Leaving would result in too few candidates.
       */
      TooFewEligibleCollators: PlainDescriptor<undefined>;

      /**
       *Account is already a candidate.
       */
      AlreadyCandidate: PlainDescriptor<undefined>;

      /**
       *Account is not a candidate.
       */
      NotCandidate: PlainDescriptor<undefined>;

      /**
       *There are too many Invulnerables.
       */
      TooManyInvulnerables: PlainDescriptor<undefined>;

      /**
       *Account is already an Invulnerable.
       */
      AlreadyInvulnerable: PlainDescriptor<undefined>;

      /**
       *Account is not an Invulnerable.
       */
      NotInvulnerable: PlainDescriptor<undefined>;

      /**
       *Account has no associated validator ID.
       */
      NoAssociatedValidatorId: PlainDescriptor<undefined>;

      /**
       *Validator ID is not yet registered.
       */
      ValidatorNotRegistered: PlainDescriptor<undefined>;

      /**
       *Could not insert in the candidate list.
       */
      InsertToCandidateListFailed: PlainDescriptor<undefined>;

      /**
       *Could not remove from the candidate list.
       */
      RemoveFromCandidateListFailed: PlainDescriptor<undefined>;

      /**
       *New deposit amount would be below the minimum candidacy bond.
       */
      DepositTooLow: PlainDescriptor<undefined>;

      /**
       *Could not update the candidate list.
       */
      UpdateCandidateListFailed: PlainDescriptor<undefined>;

      /**
       *Deposit amount is too low to take the target's slot in the candidate list.
       */
      InsufficientBond: PlainDescriptor<undefined>;

      /**
       *The target account to be replaced in the candidate list is not a candidate.
       */
      TargetIsNotCandidate: PlainDescriptor<undefined>;

      /**
       *The updated deposit amount is equal to the amount already reserved.
       */
      IdenticalDeposit: PlainDescriptor<undefined>;

      /**
       *Cannot lower candidacy bond while occupying a future collator slot in the list.
       */
      InvalidUnreserve: PlainDescriptor<undefined>;
    },
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
      QueuedKeys: StorageDescriptor<[], Array<Anonymize<I73gble6tmb52f>>, false>;

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
      NextKeys: StorageDescriptor<[Key: SS58String], Binary, true>;

      /**
       * The owner of a key. The key is the `KeyTypeId` + the encoded key.
       */
      KeyOwner: StorageDescriptor<[Key: [Binary, Binary]], SS58String, true>;
    },
    {
      /**
       *See [`Pallet::set_keys`].
       */
      set_keys: TxDescriptor<{ keys: Binary; proof: Binary }>;

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
  Aura: [
    {
      /**
       * The current authority set.
       */
      Authorities: StorageDescriptor<[], Array<Binary>, false>;

      /**
       * The current slot of this block.
       *
       * This will be set in `on_initialize`.
       */
      CurrentSlot: StorageDescriptor<[], bigint, false>;
    },
    {},
    {},
    {},
    {},
  ];
  AuraExt: [
    {
      /**
       * Serves as cache for the authorities.
       *
       * The authorities in AuRa are overwritten in `on_initialize` when we switch to a new session,
       * but we require the old authorities to verify the seal when validating a PoV. This will
       * always be updated to the latest AuRa authorities in `on_finalize`.
       */
      Authorities: StorageDescriptor<[], Array<Binary>, false>;

      /**
       * Current slot paired with a number of authored blocks.
       *
       * Updated on each block initialization.
       */
      SlotInfo: StorageDescriptor<[], [bigint, number], true>;
    },
    {},
    {},
    {},
    {},
  ];
  XcmpQueue: [
    {
      /**
       * The suspended inbound XCMP channels. All others are not suspended.
       *
       * This is a `StorageValue` instead of a `StorageMap` since we expect multiple reads per block
       * to different keys with a one byte payload. The access to `BoundedBTreeSet` will be cached
       * within the block and therefore only included once in the proof size.
       *
       * NOTE: The PoV benchmarking cannot know this and will over-estimate, but the actual proof
       * will be smaller.
       */
      InboundXcmpSuspended: StorageDescriptor<[], Array<number>, false>;

      /**
       * The non-empty XCMP channels in order of becoming non-empty, and the index of the first
       * and last outbound message. If the two indices are equal, then it indicates an empty
       * queue and there must be a non-`Ok` `OutboundStatus`. We assume queues grow no greater
       * than 65535 items. Queue indices for normal messages begin at one; zero is reserved in
       * case of the need to send a high-priority signal message this block.
       * The bool is true if there is a signal message waiting to be sent.
       */
      OutboundXcmpStatus: StorageDescriptor<[], Array<Anonymize<Ittnsbm78tol1>>, false>;

      /**
       * The messages outbound in a given XCMP channel.
       */
      OutboundXcmpMessages: StorageDescriptor<[number, number], Binary, false>;

      /**
       * Any signal messages waiting to be sent.
       */
      SignalMessages: StorageDescriptor<[Key: number], Binary, false>;

      /**
       * The configuration which controls the dynamics of the outbound queue.
       */
      QueueConfig: StorageDescriptor<
        [],
        { suspend_threshold: number; drop_threshold: number; resume_threshold: number },
        false
      >;

      /**
       * Whether or not the XCMP queue is suspended from executing incoming XCMs or not.
       */
      QueueSuspended: StorageDescriptor<[], boolean, false>;

      /**
       * The factor to multiply the base delivery fee by.
       */
      DeliveryFeeFactor: StorageDescriptor<[Key: number], bigint, false>;
    },
    {
      /**
       *See [`Pallet::suspend_xcm_execution`].
       */
      suspend_xcm_execution: TxDescriptor<undefined>;

      /**
       *See [`Pallet::resume_xcm_execution`].
       */
      resume_xcm_execution: TxDescriptor<undefined>;

      /**
       *See [`Pallet::update_suspend_threshold`].
       */
      update_suspend_threshold: TxDescriptor<{ new: number }>;

      /**
       *See [`Pallet::update_drop_threshold`].
       */
      update_drop_threshold: TxDescriptor<{ new: number }>;

      /**
       *See [`Pallet::update_resume_threshold`].
       */
      update_resume_threshold: TxDescriptor<{ new: number }>;
    },
    {
      /**
       *An HRMP message was sent to a sibling parachain.
       */
      XcmpMessageSent: PlainDescriptor<{ message_hash: Binary }>;
    },
    {
      /**
       *Setting the queue config failed since one of its values was invalid.
       */
      BadQueueConfig: PlainDescriptor<undefined>;

      /**
       *The execution is already suspended.
       */
      AlreadySuspended: PlainDescriptor<undefined>;

      /**
       *The execution is already resumed.
       */
      AlreadyResumed: PlainDescriptor<undefined>;
    },
    {
      /**
       * The maximum number of inbound XCMP channels that can be suspended simultaneously.
       *
       * Any further channel suspensions will fail and messages may get dropped without further
       * notice. Choosing a high value (1000) is okay; the trade-off that is described in
       * [`InboundXcmpSuspended`] still applies at that scale.
       */
      MaxInboundSuspended: PlainDescriptor<number>;
    },
  ];
  PolkadotXcm: [
    {
      /**
       * The latest available query index.
       */
      QueryCounter: StorageDescriptor<[], bigint, false>;

      /**
       * The ongoing queries.
       */
      Queries: StorageDescriptor<
        [Key: bigint],
        Anonymize<
          AnonymousEnum<{
            Pending: Anonymize<I4n9ble5dnecdr>;
            VersionNotifier: Anonymize<Idc4lam0e7aiet>;
            Ready: Anonymize<I3239o3gbno6s5>;
          }>
        >,
        true
      >;

      /**
       * The existing asset traps.
       *
       * Key is the blake2 256 hash of (origin, versioned `MultiAssets`) pair. Value is the number of
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
      SupportedVersion: StorageDescriptor<[number, Anonymize<Ib29ie59v4nmjq>], number, true>;

      /**
       * All locations that we have requested version notifications from.
       */
      VersionNotifiers: StorageDescriptor<[number, Anonymize<Ib29ie59v4nmjq>], bigint, true>;

      /**
       * The target locations that are subscribed to our version changes, as well as the most recent
       * of our versions we informed them of.
       */
      VersionNotifyTargets: StorageDescriptor<
        [number, Anonymize<Ib29ie59v4nmjq>],
        [bigint, Anonymize<I4q39t5hn830vp>, number],
        true
      >;

      /**
       * Destinations whose latest XCM version we would like to know. Duplicates not allowed, and
       * the `u32` counter is the number of times that a send to the destination has been attempted,
       * which is used as a prioritization.
       */
      VersionDiscoveryQueue: StorageDescriptor<[], Array<Anonymize<I82i8h7h2mvtd5>>, false>;

      /**
       * The current migration's stage, if any.
       */
      CurrentMigration: StorageDescriptor<[], XcmPalletVersionMigrationStage, true>;

      /**
       * Fungible assets which we know are locked on a remote chain.
       */
      RemoteLockedFungibles: StorageDescriptor<
        [number, SS58String, Anonymize<I9hdbmmgal228m>],
        {
          amount: bigint;
          owner: Anonymize<Ib29ie59v4nmjq>;
          locker: Anonymize<Ib29ie59v4nmjq>;
          consumers: Anonymize<I48jka0f0ufl6q>;
        },
        true
      >;

      /**
       * Fungible assets which we know are locked on this chain.
       */
      LockedFungibles: StorageDescriptor<[Key: SS58String], Array<Anonymize<Ifuuq590aavd5n>>, true>;

      /**
       * Global suspension state of the XCM executor.
       */
      XcmExecutionSuspended: StorageDescriptor<[], boolean, false>;
    },
    {
      /**
       *See [`Pallet::send`].
       */
      send: TxDescriptor<{ dest: Anonymize<Ib29ie59v4nmjq>; message: Anonymize<Ieam757vsugkcv> }>;

      /**
       *See [`Pallet::teleport_assets`].
       */
      teleport_assets: TxDescriptor<{
        dest: Anonymize<Ib29ie59v4nmjq>;
        beneficiary: Anonymize<Ib29ie59v4nmjq>;
        assets: Anonymize<I2tnkj3t3en8tf>;
        fee_asset_item: number;
      }>;

      /**
       *See [`Pallet::reserve_transfer_assets`].
       */
      reserve_transfer_assets: TxDescriptor<{
        dest: Anonymize<Ib29ie59v4nmjq>;
        beneficiary: Anonymize<Ib29ie59v4nmjq>;
        assets: Anonymize<I2tnkj3t3en8tf>;
        fee_asset_item: number;
      }>;

      /**
       *See [`Pallet::execute`].
       */
      execute: TxDescriptor<{
        message: Anonymize<I2bgn21rdfqrr7>;
        max_weight: Anonymize<I4q39t5hn830vp>;
      }>;

      /**
       *See [`Pallet::force_xcm_version`].
       */
      force_xcm_version: TxDescriptor<{ location: Anonymize<Ie897ubj3a1vaq>; version: number }>;

      /**
       *See [`Pallet::force_default_xcm_version`].
       */
      force_default_xcm_version: TxDescriptor<{ maybe_xcm_version: Anonymize<I4arjljr6dpflb> }>;

      /**
       *See [`Pallet::force_subscribe_version_notify`].
       */
      force_subscribe_version_notify: TxDescriptor<{ location: Anonymize<Ib29ie59v4nmjq> }>;

      /**
       *See [`Pallet::force_unsubscribe_version_notify`].
       */
      force_unsubscribe_version_notify: TxDescriptor<{ location: Anonymize<Ib29ie59v4nmjq> }>;

      /**
       *See [`Pallet::limited_reserve_transfer_assets`].
       */
      limited_reserve_transfer_assets: TxDescriptor<{
        dest: Anonymize<Ib29ie59v4nmjq>;
        beneficiary: Anonymize<Ib29ie59v4nmjq>;
        assets: Anonymize<I2tnkj3t3en8tf>;
        fee_asset_item: number;
        weight_limit: XcmV3WeightLimit;
      }>;

      /**
       *See [`Pallet::limited_teleport_assets`].
       */
      limited_teleport_assets: TxDescriptor<{
        dest: Anonymize<Ib29ie59v4nmjq>;
        beneficiary: Anonymize<Ib29ie59v4nmjq>;
        assets: Anonymize<I2tnkj3t3en8tf>;
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
        dest: Anonymize<Ib29ie59v4nmjq>;
        beneficiary: Anonymize<Ib29ie59v4nmjq>;
        assets: Anonymize<I2tnkj3t3en8tf>;
        fee_asset_item: number;
        weight_limit: XcmV3WeightLimit;
      }>;
    },
    {
      /**
       *Execution of an XCM message was attempted.
       */
      Attempted: PlainDescriptor<{ outcome: XcmV3TraitsOutcome }>;

      /**
       *A XCM message was sent.
       */
      Sent: PlainDescriptor<{
        origin: Anonymize<Ie897ubj3a1vaq>;
        destination: Anonymize<Ie897ubj3a1vaq>;
        message: Anonymize<I50ghg3dhe8sh3>;
        message_id: Binary;
      }>;

      /**
       *Query response received which does not match a registered query. This may be because a
       *matching query was never registered, it may be because it is a duplicate response, or
       *because the query timed out.
       */
      UnexpectedResponse: PlainDescriptor<{ origin: Anonymize<Ie897ubj3a1vaq>; query_id: bigint }>;

      /**
       *Query response has been received and is ready for taking with `take_response`. There is
       *no registered notification call.
       */
      ResponseReady: PlainDescriptor<{ query_id: bigint; response: XcmV3Response }>;

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
        origin: Anonymize<Ie897ubj3a1vaq>;
        query_id: bigint;
        expected_location: Anonymize<I189rbbmttkf8v>;
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
        origin: Anonymize<Ie897ubj3a1vaq>;
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
        origin: Anonymize<Ie897ubj3a1vaq>;
        assets: Anonymize<I2tnkj3t3en8tf>;
      }>;

      /**
       *An XCM version change notification message has been attempted to be sent.
       *
       *The cost of sending it (borne by the chain) is included.
       */
      VersionChangeNotified: PlainDescriptor<{
        destination: Anonymize<Ie897ubj3a1vaq>;
        result: number;
        cost: Anonymize<I2pdjq1umlp617>;
        message_id: Binary;
      }>;

      /**
       *The supported version of a location has been changed. This might be through an
       *automatic notification or a manual intervention.
       */
      SupportedVersionChanged: PlainDescriptor<{
        location: Anonymize<Ie897ubj3a1vaq>;
        version: number;
      }>;

      /**
       *A given location which had a version change subscription was dropped owing to an error
       *sending the notification to it.
       */
      NotifyTargetSendFail: PlainDescriptor<{
        location: Anonymize<Ie897ubj3a1vaq>;
        query_id: bigint;
        error: XcmV3TraitsError;
      }>;

      /**
       *A given location which had a version change subscription was dropped owing to an error
       *migrating the location to our new XCM format.
       */
      NotifyTargetMigrationFail: PlainDescriptor<{
        location: Anonymize<Ib29ie59v4nmjq>;
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
        origin: Anonymize<Ie897ubj3a1vaq>;
        query_id: bigint;
      }>;

      /**
       *Expected query response has been received but the querier location of the response does
       *not match the expected. The query remains registered for a later, valid, response to
       *be received and acted upon.
       */
      InvalidQuerier: PlainDescriptor<{
        origin: Anonymize<Ie897ubj3a1vaq>;
        query_id: bigint;
        expected_querier: Anonymize<Ie897ubj3a1vaq>;
        maybe_actual_querier: Anonymize<I189rbbmttkf8v>;
      }>;

      /**
       *A remote has requested XCM version change notification from us and we have honored it.
       *A version information message is sent to them and its cost is included.
       */
      VersionNotifyStarted: PlainDescriptor<{
        destination: Anonymize<Ie897ubj3a1vaq>;
        cost: Anonymize<I2pdjq1umlp617>;
        message_id: Binary;
      }>;

      /**
       *We have requested that a remote chain send us XCM version change notifications.
       */
      VersionNotifyRequested: PlainDescriptor<{
        destination: Anonymize<Ie897ubj3a1vaq>;
        cost: Anonymize<I2pdjq1umlp617>;
        message_id: Binary;
      }>;

      /**
       *We have requested that a remote chain stops sending us XCM version change
       *notifications.
       */
      VersionNotifyUnrequested: PlainDescriptor<{
        destination: Anonymize<Ie897ubj3a1vaq>;
        cost: Anonymize<I2pdjq1umlp617>;
        message_id: Binary;
      }>;

      /**
       *Fees were paid from a location for an operation (often for using `SendXcm`).
       */
      FeesPaid: PlainDescriptor<{
        paying: Anonymize<Ie897ubj3a1vaq>;
        fees: Anonymize<I2pdjq1umlp617>;
      }>;

      /**
       *Some assets have been claimed from an asset trap
       */
      AssetsClaimed: PlainDescriptor<{
        hash: Binary;
        origin: Anonymize<Ie897ubj3a1vaq>;
        assets: Anonymize<I2tnkj3t3en8tf>;
      }>;
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
       *The destination `MultiLocation` provided cannot be inverted.
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
  CumulusXcm: [
    {},
    {},
    {
      /**
       *Downward message is invalid XCM.
       *\[ id \]
       */
      InvalidFormat: PlainDescriptor<Binary>;

      /**
       *Downward message is unsupported version of XCM.
       *\[ id \]
       */
      UnsupportedVersion: PlainDescriptor<Binary>;

      /**
       *Downward message executed with the given outcome.
       *\[ id, outcome \]
       */
      ExecutedDownward: PlainDescriptor<[Binary, XcmV3TraitsOutcome]>;
    },
    {},
    {},
  ];
  MessageQueue: [
    {
      /**
       * The index of the first and last (non-empty) pages.
       */
      BookStateFor: StorageDescriptor<
        [
          Key: Anonymize<
            AnonymousEnum<{ Here: undefined; Parent: undefined; Sibling: Anonymize<number> }>
          >,
        ],
        {
          begin: number;
          end: number;
          count: number;
          ready_neighbours: Anonymize<If4d9hsuqsl01i>;
          message_count: bigint;
          size: bigint;
        },
        false
      >;

      /**
       * The origin at which we should begin servicing.
       */
      ServiceHead: StorageDescriptor<
        [],
        Anonymize<
          AnonymousEnum<{ Here: undefined; Parent: undefined; Sibling: Anonymize<number> }>
        >,
        true
      >;

      /**
       * The map of page indices to pages.
       */
      Pages: StorageDescriptor<
        [Anonymize<Ifqm8uoikppunt>, number],
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
      reap_page: TxDescriptor<{ message_origin: Anonymize<Ifqm8uoikppunt>; page_index: number }>;

      /**
       *See [`Pallet::execute_overweight`].
       */
      execute_overweight: TxDescriptor<{
        message_origin: Anonymize<Ifqm8uoikppunt>;
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
        origin: Anonymize<Ifqm8uoikppunt>;
        error: ProcessMessageError;
      }>;

      /**
       *Message is processed.
       */
      Processed: PlainDescriptor<{
        id: Binary;
        origin: Anonymize<Ifqm8uoikppunt>;
        weight_used: Anonymize<I4q39t5hn830vp>;
        success: boolean;
      }>;

      /**
       *Message placed in overweight queue.
       */
      OverweightEnqueued: PlainDescriptor<{
        id: Binary;
        origin: Anonymize<Ifqm8uoikppunt>;
        page_index: number;
        message_index: number;
      }>;

      /**
       *This page was reaped.
       */
      PageReaped: PlainDescriptor<{ origin: Anonymize<Ifqm8uoikppunt>; index: number }>;
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
  Providers: [
    {
      /**
       * The mapping from an AccountId to a MainStorageProviderId
       *
       * This is used to get a Main Storage Provider's unique identifier to access its relevant data
       */
      AccountIdToMainStorageProviderId: StorageDescriptor<[Key: SS58String], Binary, true>;

      /**
       * The mapping from a MainStorageProviderId to a MainStorageProvider
       *
       * This is used to get a Main Storage Provider's relevant data.
       * It returns `None` if the Main Storage Provider ID does not correspond to any registered Main Storage Provider.
       */
      MainStorageProviders: StorageDescriptor<
        [Key: Binary],
        {
          buckets: Anonymize<I45d79rdcadrnn>;
          capacity: number;
          data_used: number;
          multiaddresses: Anonymize<Itom7fk49o0c9>;
          value_prop: Anonymize<Ienf50imfp828o>;
        },
        true
      >;

      /**
       * The mapping from a BucketId to that bucket's metadata
       *
       * This is used to get a bucket's relevant data, such as root, user ID, and MSP ID.
       * It returns `None` if the Bucket ID does not correspond to any registered bucket.
       */
      Buckets: StorageDescriptor<
        [Key: Binary],
        { root: Binary; user_id: SS58String; msp_id: Binary },
        true
      >;

      /**
       * The mapping from an AccountId to a BackupStorageProviderId
       *
       * This is used to get a Backup Storage Provider's unique identifier to access its relevant data
       */
      AccountIdToBackupStorageProviderId: StorageDescriptor<[Key: SS58String], Binary, true>;

      /**
       * The mapping from a BackupStorageProviderId to a BackupStorageProvider
       *
       * This is used to get a Backup Storage Provider's relevant data.
       * It returns `None` if the Backup Storage Provider ID does not correspond to any registered Backup Storage Provider.
       */
      BackupStorageProviders: StorageDescriptor<
        [Key: Binary],
        {
          capacity: number;
          data_used: number;
          multiaddresses: Anonymize<Itom7fk49o0c9>;
          root: Binary;
        },
        true
      >;

      /**
       * The amount of Main Storage Providers that are currently registered in the runtime.
       */
      MspCount: StorageDescriptor<[], number, false>;

      /**
       * The amount of Backup Storage Providers that are currently registered in the runtime.
       */
      BspCount: StorageDescriptor<[], number, false>;

      /**
       * The total amount of storage capacity all BSPs have. Remember redundancy!
       */
      TotalBspsCapacity: StorageDescriptor<[], number, false>;
    },
    {
      /**
       *See [`Pallet::msp_sign_up`].
       */
      msp_sign_up: TxDescriptor<{
        capacity: number;
        multiaddresses: Anonymize<Itom7fk49o0c9>;
        value_prop: Anonymize<Ienf50imfp828o>;
      }>;

      /**
       *See [`Pallet::bsp_sign_up`].
       */
      bsp_sign_up: TxDescriptor<{ capacity: number; multiaddresses: Anonymize<Itom7fk49o0c9> }>;

      /**
       *See [`Pallet::msp_sign_off`].
       */
      msp_sign_off: TxDescriptor<undefined>;

      /**
       *See [`Pallet::bsp_sign_off`].
       */
      bsp_sign_off: TxDescriptor<undefined>;

      /**
       *See [`Pallet::change_capacity`].
       */
      change_capacity: TxDescriptor<{ new_capacity: number }>;

      /**
       *See [`Pallet::add_value_prop`].
       */
      add_value_prop: TxDescriptor<{ new_value_prop: Anonymize<Ienf50imfp828o> }>;
    },
    {
      /**
       *Event emitted when a Main Storage Provider has signed up successfully. Provides information about
       *that MSP's account id, the total data it can store according to its stake, its multiaddress, and its value proposition.
       */
      MspSignUpSuccess: PlainDescriptor<{
        who: SS58String;
        multiaddresses: Anonymize<Itom7fk49o0c9>;
        capacity: number;
        value_prop: Anonymize<Ienf50imfp828o>;
      }>;

      /**
       *Event emitted when a Backup Storage Provider has signed up successfully. Provides information about
       *that BSP's account id, the total data it can store according to its stake, and its multiaddress.
       */
      BspSignUpSuccess: PlainDescriptor<{
        who: SS58String;
        multiaddresses: Anonymize<Itom7fk49o0c9>;
        capacity: number;
      }>;

      /**
       *Event emitted when a Main Storage Provider has signed off successfully. Provides information about
       *that MSP's account id.
       */
      MspSignOffSuccess: PlainDescriptor<{ who: SS58String }>;

      /**
       *Event emitted when a Backup Storage Provider has signed off successfully. Provides information about
       *that BSP's account id.
       */
      BspSignOffSuccess: PlainDescriptor<{ who: SS58String }>;

      /**
       *Event emitted when a SP has changed is total data (stake) successfully. Provides information about
       *that SP's account id, its old total data that could store, and the new total data.
       */
      TotalDataChanged: PlainDescriptor<{
        who: SS58String;
        old_capacity: number;
        new_capacity: number;
      }>;
    },
    {
      /**
       *Error thrown when a user tries to sign up as a SP but is already registered as a MSP or BSP.
       */
      AlreadyRegistered: PlainDescriptor<undefined>;

      /**
       *Error thrown when a user tries to sign up or change its stake to store less storage than the minimum required by the runtime.
       */
      StorageTooLow: PlainDescriptor<undefined>;

      /**
       *Error thrown when a user does not have enough balance to pay the deposit that it would incur by signing up as a SP or changing its total data (stake).
       */
      NotEnoughBalance: PlainDescriptor<undefined>;

      /**
       *Error thrown when the runtime cannot hold the required deposit from the account to register it as a SP
       */
      CannotHoldDeposit: PlainDescriptor<undefined>;

      /**
       *Error thrown when a user tries to sign up as a BSP but the maximum amount of BSPs has been reached.
       */
      MaxBspsReached: PlainDescriptor<undefined>;

      /**
       *Error thrown when a user tries to sign up as a MSP but the maximum amount of MSPs has been reached.
       */
      MaxMspsReached: PlainDescriptor<undefined>;

      /**
       *Error thrown when a user tries to sign off as a SP but is not registered as a MSP or BSP.
       */
      NotRegistered: PlainDescriptor<undefined>;

      /**
       *Error thrown when a user has a SP ID assigned to it but the SP data does not exist in storage (Inconsistency error).
       */
      SpRegisteredButDataNotFound: PlainDescriptor<undefined>;

      /**
       *Error thrown when a user tries to sign off as a SP but still has used storage.
       */
      StorageStillInUse: PlainDescriptor<undefined>;

      /**
       *Error thrown when a SP tries to change its total data (stake) but it has not been enough time since the last time it changed it.
       */
      NotEnoughTimePassed: PlainDescriptor<undefined>;

      /**
       *Error thrown when trying to get a root from a MSP without passing a User ID
       */
      NoUserId: PlainDescriptor<undefined>;

      /**
       *Error thrown when trying to get a root from a MSP without passing a Bucket ID
       */
      NoBucketId: PlainDescriptor<undefined>;

      /**
       *Error thrown when a user tries to sign up without any multiaddress
       */
      NoMultiAddress: PlainDescriptor<undefined>;

      /**
       *Error thrown when a user tries to sign up as a SP but any of the provided multiaddresses is invalid
       */
      InvalidMultiAddress: PlainDescriptor<undefined>;

      /**
       *Error thrown when overflowing after doing math operations
       */
      Overflow: PlainDescriptor<undefined>;
    },
    {
      /**
       * The minimum amount that an account has to deposit to become a storage provider.
       */
      SpMinDeposit: PlainDescriptor<bigint>;

      /**
       * The amount that a BSP receives as allocation of storage capacity when it deposits SpMinDeposit.
       */
      SpMinCapacity: PlainDescriptor<number>;

      /**
       * The slope of the collateral vs storage capacity curve. In other terms, how many tokens a Storage Provider should add as collateral to increase its storage capacity in one unit of StorageData.
       */
      DepositPerData: PlainDescriptor<bigint>;

      /**
       * The maximum amount of BSPs that can exist.
       */
      MaxBsps: PlainDescriptor<number>;

      /**
       * The maximum amount of MSPs that can exist.
       */
      MaxMsps: PlainDescriptor<number>;

      /**
       * The maximum size of a multiaddress.
       */
      MaxMultiAddressSize: PlainDescriptor<number>;

      /**
       * The maximum amount of multiaddresses that a Storage Provider can have.
       */
      MaxMultiAddressAmount: PlainDescriptor<number>;

      /**
       * The maximum number of protocols the MSP can support (at least within the runtime).
       */
      MaxProtocols: PlainDescriptor<number>;

      /**
       * The maximum amount of Buckets that a MSP can have.
       */
      MaxBuckets: PlainDescriptor<number>;
    },
  ];
  FileSystem: [
    {
      /**

 */
      StorageRequests: StorageDescriptor<
        [Key: Binary],
        {
          requested_at: number;
          owner: SS58String;
          fingerprint: Binary;
          size: bigint;
          user_multiaddresses: Anonymize<Itom7fk49o0c9>;
          data_server_sps: Anonymize<Ia2lhg7l2hilo3>;
          bsps_required: number;
          bsps_confirmed: number;
        },
        true
      >;

      /**
       * A double map of [`storage request`](FileLocation) to [`BSPs`](StorageProviderId) that volunteered to store data.
       *
       * Any BSP under a storage request is considered to be a volunteer and can be removed at any time.
       * Once a BSP submits a valid proof to the `pallet-proofs-dealer`, the `confirmed` field in [`StorageRequestBsps`] should be set to `true`.
       *
       * When a storage request is expired or removed, the corresponding storage request key in this map should be removed.
       */
      StorageRequestBsps: StorageDescriptor<[Binary, SS58String], boolean, true>;

      /**
       * A map of blocks to expired storage requests.
       */
      StorageRequestExpirations: StorageDescriptor<[Key: number], Array<Binary>, false>;

      /**
       * A pointer to the earliest available block to insert a new storage request expiration.
       *
       * This should always be greater or equal than current block + [`Config::StorageRequestTtl`].
       */
      NextAvailableExpirationInsertionBlock: StorageDescriptor<[], number, false>;

      /**
       * A pointer to the starting block to clean up expired storage requests.
       *
       * If this block is behind the current block number, the cleanup algorithm in `on_idle` will
       * attempt to accelerate this block pointer as close to or up to the current block number. This
       * will execute provided that there is enough remaining weight to do so.
       */
      NextStartingBlockToCleanUp: StorageDescriptor<[], number, false>;
    },
    {
      /**
       *See [`Pallet::create_bucket`].
       */
      create_bucket: TxDescriptor<undefined>;

      /**
       *See [`Pallet::issue_storage_request`].
       */
      issue_storage_request: TxDescriptor<{
        location: Binary;
        fingerprint: Binary;
        size: bigint;
        multiaddresses: Anonymize<Itom7fk49o0c9>;
      }>;

      /**
       *See [`Pallet::revoke_storage_request`].
       */
      revoke_storage_request: TxDescriptor<{ location: Binary }>;

      /**
       *See [`Pallet::bsp_volunteer`].
       */
      bsp_volunteer: TxDescriptor<{
        location: Binary;
        fingerprint: Binary;
        multiaddresses: Anonymize<Itom7fk49o0c9>;
      }>;

      /**
       *See [`Pallet::bsp_stop_storing`].
       */
      bsp_stop_storing: TxDescriptor<{
        file_key: Binary;
        location: Binary;
        owner: SS58String;
        fingerprint: Binary;
        size: bigint;
        can_serve: boolean;
      }>;
    },
    {
      /**
       *Notifies that a new file has been requested to be stored.
       */
      NewStorageRequest: PlainDescriptor<{
        who: SS58String;
        location: Binary;
        fingerprint: Binary;
        size: bigint;
        multiaddresses: Anonymize<Itom7fk49o0c9>;
      }>;

      /**
       *Notifies that a BSP has been accepted to store a given file.
       */
      AcceptedBspVolunteer: PlainDescriptor<{
        who: SS58String;
        location: Binary;
        fingerprint: Binary;
        multiaddresses: Anonymize<Itom7fk49o0c9>;
      }>;

      /**
       *Notifies the expiration of a storage request.
       */
      StorageRequestExpired: PlainDescriptor<{ location: Binary }>;

      /**
       *Notifies that a storage request has been revoked by the user who initiated it.
       */
      StorageRequestRevoked: PlainDescriptor<{ location: Binary }>;

      /**
       *Notifies that a BSP has stopped storing a file.
       */
      BspStoppedStoring: PlainDescriptor<{
        bsp: SS58String;
        file_key: Binary;
        owner: SS58String;
        location: Binary;
      }>;
    },
    {
      /**
       *Storage request already registered for the given file.
       */
      StorageRequestAlreadyRegistered: PlainDescriptor<undefined>;

      /**
       *Storage request not registered for the given file.
       */
      StorageRequestNotFound: PlainDescriptor<undefined>;

      /**
       *BSPs required for storage request cannot be 0.
       */
      BspsRequiredCannotBeZero: PlainDescriptor<undefined>;

      /**
       *BSPs required for storage request cannot exceed the maximum allowed.
       */
      BspsRequiredExceedsMax: PlainDescriptor<undefined>;

      /**
       *BSP already volunteered to store the given file.
       */
      BspVolunteerFailed: PlainDescriptor<undefined>;

      /**
       *Number of BSPs required for storage request has been reached.
       */
      StorageRequestBspsRequiredFulfilled: PlainDescriptor<undefined>;

      /**
       *BSP already volunteered to store the given file.
       */
      BspAlreadyVolunteered: PlainDescriptor<undefined>;

      /**
       *No slot available found in blocks to insert storage request expiration time.
       */
      StorageRequestExpiredNoSlotAvailable: PlainDescriptor<undefined>;

      /**
       *Not authorized to delete the storage request.
       */
      StorageRequestNotAuthorized: PlainDescriptor<undefined>;

      /**
       *Error created in 2024. If you see this, you are well beyond the singularity and should
       *probably stop using this pallet.
       */
      MaxBlockNumberReached: PlainDescriptor<undefined>;
    },
    {
      /**
       * Minimum number of BSPs required to store a file.
       *
       * This is also used as a default value if the BSPs required are not specified when creating a storage request.
       */
      TargetBspsRequired: PlainDescriptor<number>;

      /**
       * Maximum number of BSPs that can store a file.
       *
       * This is used to limit the number of BSPs storing a file and claiming rewards for it.
       * If this number is to high, then the reward for storing a file might be to diluted and pointless to store.
       */
      MaxBspsPerStorageRequest: PlainDescriptor<number>;

      /**
       * Maximum byte size of a file path.
       */
      MaxFilePathSize: PlainDescriptor<number>;

      /**
       * Maximum byte size of a libp2p multiaddress.
       */
      MaxMultiAddressSize: PlainDescriptor<number>;

      /**
       * Maximum number of multiaddresses for a storage request.
       */
      MaxMultiAddresses: PlainDescriptor<number>;

      /**
       * Time-to-live for a storage request.
       */
      StorageRequestTtl: PlainDescriptor<number>;

      /**
       * Maximum number of expired storage requests to clean up in a single block.
       */
      MaxExpiredStorageRequests: PlainDescriptor<number>;
    },
  ];
  ProofsDealer: [
    {
      /**
       * A mapping from block number to a vector of challenged file keys for that block.
       *
       * This is used to keep track of the challenges that have been made in the past.
       * The vector is bounded by `MaxChallengesPerBlock`.
       * This mapping goes back only `ChallengeHistoryLength` blocks. Previous challenges are removed.
       */
      BlockToChallenges: StorageDescriptor<[Key: number], Array<Binary>, true>;

      /**
       * A mapping from block number to a vector of challenged Providers for that block.
       *
       * This is used to keep track of the Providers that have been challenged, and should
       * submit a proof by the time of the block used as the key. Providers who do submit
       * a proof are removed from their respective entry and pushed forward to the next block in
       * which they should submit a proof. Those who are still in the entry by the time the block
       * is reached are considered to have failed to submit a proof and subject to slashing.
       */
      BlockToChallengedSps: StorageDescriptor<[Key: number], Array<Binary>, true>;

      /**
       * A mapping from a Provider to the last block number they submitted a proof for.
       * If for a Provider `sp`, `LastBlockSpSubmittedProofFor[sp]` is `n`, then the
       * Provider should submit a proof for block `n + stake_to_challenge_period(sp)`.
       */
      LastBlockSpSubmittedProofFor: StorageDescriptor<[Key: Binary], number, true>;

      /**
       * A queue of file keys that have been challenged manually.
       *
       * The elements in this queue will be challenged in the coming blocks,
       * always ensuring that the maximum number of challenges per block is not exceeded.
       * A `BoundedVec` is used because the `parity_scale_codec::MaxEncodedLen` trait
       * is required, but using a `VecDeque` would be more efficient as this is a FIFO queue.
       */
      ChallengesQueue: StorageDescriptor<[], Array<Binary>, false>;

      /**
       * A priority queue of file keys that have been challenged manually.
       *
       * The difference between this and `ChallengesQueue` is that the challenges
       * in this queue are given priority over the others. So this queue should be
       * emptied before any of the challenges in the `ChallengesQueue` are dispatched.
       * This queue should not be accessible to the public.
       * The elements in this queue will be challenged in the coming blocks,
       * always ensuring that the maximum number of challenges per block is not exceeded.
       * A `BoundedVec` is used because the `parity_scale_codec::MaxEncodedLen` trait
       * is required, but using a `VecDeque` would be more efficient as this is a FIFO queue.
       */
      PriorityChallengesQueue: StorageDescriptor<[], Array<Binary>, false>;

      /**
       * The block number of the last checkpoint challenge round.
       *
       * This is used to determine when to include the challenges from the `ChallengesQueue` and
       * `PriorityChallengesQueue` in the `BlockToChallenges` StorageMap. These checkpoint challenge
       * rounds have to be answered by ALL Providers, and this is enforced by the
       * `submit_proof` extrinsic.
       */
      LastCheckpointBlock: StorageDescriptor<[], number, false>;
    },
    {
      /**
       *See [`Pallet::challenge`].
       */
      challenge: TxDescriptor<{ key: Binary }>;

      /**
       *See [`Pallet::submit_proof`].
       */
      submit_proof: TxDescriptor<{
        proof: Anonymize<Itom7fk49o0c9>;
        root: Binary;
        challenge_block: number;
        provider: Anonymize<I17k3ujudqd5df>;
      }>;

      /**
       *See [`Pallet::new_challenges_round`].
       */
      new_challenges_round: TxDescriptor<undefined>;
    },
    {
      /**
       *A manual challenge was submitted.
       */
      NewChallenge: PlainDescriptor<{ who: SS58String; key_challenged: Binary }>;

      /**
       *A proof was rejected.
       */
      ProofRejected: PlainDescriptor<{
        provider: Binary;
        proof: Anonymize<Itom7fk49o0c9>;
        reason: Anonymize<Ifhhbbpbpeqis>;
      }>;

      /**
       *A proof was accepted.
       */
      ProofAccepted: PlainDescriptor<{ provider: Binary; proof: Anonymize<Itom7fk49o0c9> }>;
    },
    {
      /**
       *The ChallengesQueue is full. No more manual challenges can be made
       *until some of the challenges in the queue are dispatched.
       */
      ChallengesQueueOverflow: PlainDescriptor<undefined>;

      /**
       *The PriorityChallengesQueue is full. No more priority challenges can be made
       *until some of the challenges in the queue are dispatched.
       */
      PriorityChallengesQueueOverflow: PlainDescriptor<undefined>;

      /**
       *The proof submitter is not a registered Provider.
       */
      NotProvider: PlainDescriptor<undefined>;

      /**
       *The fee for submitting a challenge could not be charged.
       */
      FeeChargeFailed: PlainDescriptor<undefined>;
    },
    {
      /**
       * The maximum number of challenges that can be made in a single block.
       */
      MaxChallengesPerBlock: PlainDescriptor<number>;

      /**
       * The maximum number of Providers that can be challenged in block.
       */
      MaxProvidersChallengedPerBlock: PlainDescriptor<number>;

      /**
       * The number of blocks that challenges history is kept for.
       * After this many blocks, challenges are removed from `Challenges` StorageMap.
       */
      ChallengeHistoryLength: PlainDescriptor<number>;

      /**
       * The length of the `ChallengesQueue` StorageValue.
       * This is to limit the size of the queue, and therefore the number of
       * manual challenges that can be made.
       */
      ChallengesQueueLength: PlainDescriptor<number>;

      /**
       * The number of blocks in between a checkpoint challenges round (i.e. with custom challenges).
       * This is used to determine when to include the challenges from the `ChallengesQueue` and
       * `PriorityChallengesQueue` in the `BlockToChallenges` StorageMap. These checkpoint challenge
       * rounds have to be answered by ALL Providers, and this is enforced by the
       * `submit_proof` extrinsic.
       */
      CheckpointChallengePeriod: PlainDescriptor<number>;

      /**
       * The fee charged for submitting a challenge.
       * This fee goes to the Treasury, and is used to prevent spam. Registered Providers are
       * exempt from this fee.
       */
      ChallengesFee: PlainDescriptor<bigint>;

      /**
       * The Treasury AccountId.
       * The account to which:
       * - The fees for submitting a challenge are transferred.
       * - The slashed funds are transferred.
       */
      Treasury: PlainDescriptor<SS58String>;
    },
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
    },
    {
      ExtrinsicSuccess: evt_System_ExtrinsicSuccess,
      ExtrinsicFailed: evt_System_ExtrinsicFailed,
      CodeUpdated: evt_System_CodeUpdated,
      NewAccount: evt_System_NewAccount,
      KilledAccount: evt_System_KilledAccount,
      Remarked: evt_System_Remarked,
    },
    {
      InvalidSpecName: err_System_InvalidSpecName,
      SpecVersionNeedsToIncrease: err_System_SpecVersionNeedsToIncrease,
      FailedToExtractRuntimeVersion: err_System_FailedToExtractRuntimeVersion,
      NonDefaultComposite: err_System_NonDefaultComposite,
      NonZeroRefCount: err_System_NonZeroRefCount,
      CallFiltered: err_System_CallFiltered,
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
  ParachainSystem: [
    {
      UnincludedSegment: stg_ParachainSystem_UnincludedSegment,
      AggregatedUnincludedSegment: stg_ParachainSystem_AggregatedUnincludedSegment,
      PendingValidationCode: stg_ParachainSystem_PendingValidationCode,
      NewValidationCode: stg_ParachainSystem_NewValidationCode,
      ValidationData: stg_ParachainSystem_ValidationData,
      DidSetValidationCode: stg_ParachainSystem_DidSetValidationCode,
      LastRelayChainBlockNumber: stg_ParachainSystem_LastRelayChainBlockNumber,
      UpgradeRestrictionSignal: stg_ParachainSystem_UpgradeRestrictionSignal,
      UpgradeGoAhead: stg_ParachainSystem_UpgradeGoAhead,
      RelayStateProof: stg_ParachainSystem_RelayStateProof,
      RelevantMessagingState: stg_ParachainSystem_RelevantMessagingState,
      HostConfiguration: stg_ParachainSystem_HostConfiguration,
      LastDmqMqcHead: stg_ParachainSystem_LastDmqMqcHead,
      LastHrmpMqcHeads: stg_ParachainSystem_LastHrmpMqcHeads,
      ProcessedDownwardMessages: stg_ParachainSystem_ProcessedDownwardMessages,
      HrmpWatermark: stg_ParachainSystem_HrmpWatermark,
      HrmpOutboundMessages: stg_ParachainSystem_HrmpOutboundMessages,
      UpwardMessages: stg_ParachainSystem_UpwardMessages,
      PendingUpwardMessages: stg_ParachainSystem_PendingUpwardMessages,
      UpwardDeliveryFeeFactor: stg_ParachainSystem_UpwardDeliveryFeeFactor,
      AnnouncedHrmpMessagesPerCandidate: stg_ParachainSystem_AnnouncedHrmpMessagesPerCandidate,
      ReservedXcmpWeightOverride: stg_ParachainSystem_ReservedXcmpWeightOverride,
      ReservedDmpWeightOverride: stg_ParachainSystem_ReservedDmpWeightOverride,
      AuthorizedUpgrade: stg_ParachainSystem_AuthorizedUpgrade,
      CustomValidationHeadData: stg_ParachainSystem_CustomValidationHeadData,
    },
    {
      set_validation_data: call_ParachainSystem_set_validation_data,
      sudo_send_upward_message: call_ParachainSystem_sudo_send_upward_message,
      authorize_upgrade: call_ParachainSystem_authorize_upgrade,
      enact_authorized_upgrade: call_ParachainSystem_enact_authorized_upgrade,
    },
    {
      ValidationFunctionStored: evt_ParachainSystem_ValidationFunctionStored,
      ValidationFunctionApplied: evt_ParachainSystem_ValidationFunctionApplied,
      ValidationFunctionDiscarded: evt_ParachainSystem_ValidationFunctionDiscarded,
      UpgradeAuthorized: evt_ParachainSystem_UpgradeAuthorized,
      DownwardMessagesReceived: evt_ParachainSystem_DownwardMessagesReceived,
      DownwardMessagesProcessed: evt_ParachainSystem_DownwardMessagesProcessed,
      UpwardMessageSent: evt_ParachainSystem_UpwardMessageSent,
    },
    {
      OverlappingUpgrades: err_ParachainSystem_OverlappingUpgrades,
      ProhibitedByPolkadot: err_ParachainSystem_ProhibitedByPolkadot,
      TooBig: err_ParachainSystem_TooBig,
      ValidationDataNotAvailable: err_ParachainSystem_ValidationDataNotAvailable,
      HostConfigurationNotAvailable: err_ParachainSystem_HostConfigurationNotAvailable,
      NotScheduled: err_ParachainSystem_NotScheduled,
      NothingAuthorized: err_ParachainSystem_NothingAuthorized,
      Unauthorized: err_ParachainSystem_Unauthorized,
    },
    {},
  ],
  Timestamp: [
    { Now: stg_Timestamp_Now, DidUpdate: stg_Timestamp_DidUpdate },
    { set: call_Timestamp_set },
    {},
    {},
    { MinimumPeriod: const_Timestamp_MinimumPeriod },
  ],
  ParachainInfo: [{ ParachainId: stg_ParachainInfo_ParachainId }, {}, {}, {}, {}],
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
    },
    {
      ExistentialDeposit: const_Balances_ExistentialDeposit,
      MaxLocks: const_Balances_MaxLocks,
      MaxReserves: const_Balances_MaxReserves,
      MaxHolds: const_Balances_MaxHolds,
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
  Authorship: [{ Author: stg_Authorship_Author }, {}, {}, {}, {}],
  CollatorSelection: [
    {
      Invulnerables: stg_CollatorSelection_Invulnerables,
      CandidateList: stg_CollatorSelection_CandidateList,
      LastAuthoredBlock: stg_CollatorSelection_LastAuthoredBlock,
      DesiredCandidates: stg_CollatorSelection_DesiredCandidates,
      CandidacyBond: stg_CollatorSelection_CandidacyBond,
    },
    {
      set_invulnerables: call_CollatorSelection_set_invulnerables,
      set_desired_candidates: call_CollatorSelection_set_desired_candidates,
      set_candidacy_bond: call_CollatorSelection_set_candidacy_bond,
      register_as_candidate: call_CollatorSelection_register_as_candidate,
      leave_intent: call_CollatorSelection_leave_intent,
      add_invulnerable: call_CollatorSelection_add_invulnerable,
      remove_invulnerable: call_CollatorSelection_remove_invulnerable,
      update_bond: call_CollatorSelection_update_bond,
      take_candidate_slot: call_CollatorSelection_take_candidate_slot,
    },
    {
      NewInvulnerables: evt_CollatorSelection_NewInvulnerables,
      InvulnerableAdded: evt_CollatorSelection_InvulnerableAdded,
      InvulnerableRemoved: evt_CollatorSelection_InvulnerableRemoved,
      NewDesiredCandidates: evt_CollatorSelection_NewDesiredCandidates,
      NewCandidacyBond: evt_CollatorSelection_NewCandidacyBond,
      CandidateAdded: evt_CollatorSelection_CandidateAdded,
      CandidateBondUpdated: evt_CollatorSelection_CandidateBondUpdated,
      CandidateRemoved: evt_CollatorSelection_CandidateRemoved,
      CandidateReplaced: evt_CollatorSelection_CandidateReplaced,
      InvalidInvulnerableSkipped: evt_CollatorSelection_InvalidInvulnerableSkipped,
    },
    {
      TooManyCandidates: err_CollatorSelection_TooManyCandidates,
      TooFewEligibleCollators: err_CollatorSelection_TooFewEligibleCollators,
      AlreadyCandidate: err_CollatorSelection_AlreadyCandidate,
      NotCandidate: err_CollatorSelection_NotCandidate,
      TooManyInvulnerables: err_CollatorSelection_TooManyInvulnerables,
      AlreadyInvulnerable: err_CollatorSelection_AlreadyInvulnerable,
      NotInvulnerable: err_CollatorSelection_NotInvulnerable,
      NoAssociatedValidatorId: err_CollatorSelection_NoAssociatedValidatorId,
      ValidatorNotRegistered: err_CollatorSelection_ValidatorNotRegistered,
      InsertToCandidateListFailed: err_CollatorSelection_InsertToCandidateListFailed,
      RemoveFromCandidateListFailed: err_CollatorSelection_RemoveFromCandidateListFailed,
      DepositTooLow: err_CollatorSelection_DepositTooLow,
      UpdateCandidateListFailed: err_CollatorSelection_UpdateCandidateListFailed,
      InsufficientBond: err_CollatorSelection_InsufficientBond,
      TargetIsNotCandidate: err_CollatorSelection_TargetIsNotCandidate,
      IdenticalDeposit: err_CollatorSelection_IdenticalDeposit,
      InvalidUnreserve: err_CollatorSelection_InvalidUnreserve,
    },
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
  Aura: [{ Authorities: stg_Aura_Authorities, CurrentSlot: stg_Aura_CurrentSlot }, {}, {}, {}, {}],
  AuraExt: [
    { Authorities: stg_AuraExt_Authorities, SlotInfo: stg_AuraExt_SlotInfo },
    {},
    {},
    {},
    {},
  ],
  XcmpQueue: [
    {
      InboundXcmpSuspended: stg_XcmpQueue_InboundXcmpSuspended,
      OutboundXcmpStatus: stg_XcmpQueue_OutboundXcmpStatus,
      OutboundXcmpMessages: stg_XcmpQueue_OutboundXcmpMessages,
      SignalMessages: stg_XcmpQueue_SignalMessages,
      QueueConfig: stg_XcmpQueue_QueueConfig,
      QueueSuspended: stg_XcmpQueue_QueueSuspended,
      DeliveryFeeFactor: stg_XcmpQueue_DeliveryFeeFactor,
    },
    {
      suspend_xcm_execution: call_XcmpQueue_suspend_xcm_execution,
      resume_xcm_execution: call_XcmpQueue_resume_xcm_execution,
      update_suspend_threshold: call_XcmpQueue_update_suspend_threshold,
      update_drop_threshold: call_XcmpQueue_update_drop_threshold,
      update_resume_threshold: call_XcmpQueue_update_resume_threshold,
    },
    { XcmpMessageSent: evt_XcmpQueue_XcmpMessageSent },
    {
      BadQueueConfig: err_XcmpQueue_BadQueueConfig,
      AlreadySuspended: err_XcmpQueue_AlreadySuspended,
      AlreadyResumed: err_XcmpQueue_AlreadyResumed,
    },
    { MaxInboundSuspended: const_XcmpQueue_MaxInboundSuspended },
  ],
  PolkadotXcm: [
    {
      QueryCounter: stg_PolkadotXcm_QueryCounter,
      Queries: stg_PolkadotXcm_Queries,
      AssetTraps: stg_PolkadotXcm_AssetTraps,
      SafeXcmVersion: stg_PolkadotXcm_SafeXcmVersion,
      SupportedVersion: stg_PolkadotXcm_SupportedVersion,
      VersionNotifiers: stg_PolkadotXcm_VersionNotifiers,
      VersionNotifyTargets: stg_PolkadotXcm_VersionNotifyTargets,
      VersionDiscoveryQueue: stg_PolkadotXcm_VersionDiscoveryQueue,
      CurrentMigration: stg_PolkadotXcm_CurrentMigration,
      RemoteLockedFungibles: stg_PolkadotXcm_RemoteLockedFungibles,
      LockedFungibles: stg_PolkadotXcm_LockedFungibles,
      XcmExecutionSuspended: stg_PolkadotXcm_XcmExecutionSuspended,
    },
    {
      send: call_PolkadotXcm_send,
      teleport_assets: call_PolkadotXcm_teleport_assets,
      reserve_transfer_assets: call_PolkadotXcm_reserve_transfer_assets,
      execute: call_PolkadotXcm_execute,
      force_xcm_version: call_PolkadotXcm_force_xcm_version,
      force_default_xcm_version: call_PolkadotXcm_force_default_xcm_version,
      force_subscribe_version_notify: call_PolkadotXcm_force_subscribe_version_notify,
      force_unsubscribe_version_notify: call_PolkadotXcm_force_unsubscribe_version_notify,
      limited_reserve_transfer_assets: call_PolkadotXcm_limited_reserve_transfer_assets,
      limited_teleport_assets: call_PolkadotXcm_limited_teleport_assets,
      force_suspension: call_PolkadotXcm_force_suspension,
      transfer_assets: call_PolkadotXcm_transfer_assets,
    },
    {
      Attempted: evt_PolkadotXcm_Attempted,
      Sent: evt_PolkadotXcm_Sent,
      UnexpectedResponse: evt_PolkadotXcm_UnexpectedResponse,
      ResponseReady: evt_PolkadotXcm_ResponseReady,
      Notified: evt_PolkadotXcm_Notified,
      NotifyOverweight: evt_PolkadotXcm_NotifyOverweight,
      NotifyDispatchError: evt_PolkadotXcm_NotifyDispatchError,
      NotifyDecodeFailed: evt_PolkadotXcm_NotifyDecodeFailed,
      InvalidResponder: evt_PolkadotXcm_InvalidResponder,
      InvalidResponderVersion: evt_PolkadotXcm_InvalidResponderVersion,
      ResponseTaken: evt_PolkadotXcm_ResponseTaken,
      AssetsTrapped: evt_PolkadotXcm_AssetsTrapped,
      VersionChangeNotified: evt_PolkadotXcm_VersionChangeNotified,
      SupportedVersionChanged: evt_PolkadotXcm_SupportedVersionChanged,
      NotifyTargetSendFail: evt_PolkadotXcm_NotifyTargetSendFail,
      NotifyTargetMigrationFail: evt_PolkadotXcm_NotifyTargetMigrationFail,
      InvalidQuerierVersion: evt_PolkadotXcm_InvalidQuerierVersion,
      InvalidQuerier: evt_PolkadotXcm_InvalidQuerier,
      VersionNotifyStarted: evt_PolkadotXcm_VersionNotifyStarted,
      VersionNotifyRequested: evt_PolkadotXcm_VersionNotifyRequested,
      VersionNotifyUnrequested: evt_PolkadotXcm_VersionNotifyUnrequested,
      FeesPaid: evt_PolkadotXcm_FeesPaid,
      AssetsClaimed: evt_PolkadotXcm_AssetsClaimed,
    },
    {
      Unreachable: err_PolkadotXcm_Unreachable,
      SendFailure: err_PolkadotXcm_SendFailure,
      Filtered: err_PolkadotXcm_Filtered,
      UnweighableMessage: err_PolkadotXcm_UnweighableMessage,
      DestinationNotInvertible: err_PolkadotXcm_DestinationNotInvertible,
      Empty: err_PolkadotXcm_Empty,
      CannotReanchor: err_PolkadotXcm_CannotReanchor,
      TooManyAssets: err_PolkadotXcm_TooManyAssets,
      InvalidOrigin: err_PolkadotXcm_InvalidOrigin,
      BadVersion: err_PolkadotXcm_BadVersion,
      BadLocation: err_PolkadotXcm_BadLocation,
      NoSubscription: err_PolkadotXcm_NoSubscription,
      AlreadySubscribed: err_PolkadotXcm_AlreadySubscribed,
      CannotCheckOutTeleport: err_PolkadotXcm_CannotCheckOutTeleport,
      LowBalance: err_PolkadotXcm_LowBalance,
      TooManyLocks: err_PolkadotXcm_TooManyLocks,
      AccountNotSovereign: err_PolkadotXcm_AccountNotSovereign,
      FeesNotMet: err_PolkadotXcm_FeesNotMet,
      LockNotFound: err_PolkadotXcm_LockNotFound,
      InUse: err_PolkadotXcm_InUse,
      InvalidAssetNotConcrete: err_PolkadotXcm_InvalidAssetNotConcrete,
      InvalidAssetUnknownReserve: err_PolkadotXcm_InvalidAssetUnknownReserve,
      InvalidAssetUnsupportedReserve: err_PolkadotXcm_InvalidAssetUnsupportedReserve,
      TooManyReserves: err_PolkadotXcm_TooManyReserves,
      LocalExecutionIncomplete: err_PolkadotXcm_LocalExecutionIncomplete,
    },
    {},
  ],
  CumulusXcm: [
    {},
    {},
    {
      InvalidFormat: evt_CumulusXcm_InvalidFormat,
      UnsupportedVersion: evt_CumulusXcm_UnsupportedVersion,
      ExecutedDownward: evt_CumulusXcm_ExecutedDownward,
    },
    {},
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
    },
    {
      HeapSize: const_MessageQueue_HeapSize,
      MaxStale: const_MessageQueue_MaxStale,
      ServiceWeight: const_MessageQueue_ServiceWeight,
    },
  ],
  Providers: [
    {
      AccountIdToMainStorageProviderId: stg_Providers_AccountIdToMainStorageProviderId,
      MainStorageProviders: stg_Providers_MainStorageProviders,
      Buckets: stg_Providers_Buckets,
      AccountIdToBackupStorageProviderId: stg_Providers_AccountIdToBackupStorageProviderId,
      BackupStorageProviders: stg_Providers_BackupStorageProviders,
      MspCount: stg_Providers_MspCount,
      BspCount: stg_Providers_BspCount,
      TotalBspsCapacity: stg_Providers_TotalBspsCapacity,
    },
    {
      msp_sign_up: call_Providers_msp_sign_up,
      bsp_sign_up: call_Providers_bsp_sign_up,
      msp_sign_off: call_Providers_msp_sign_off,
      bsp_sign_off: call_Providers_bsp_sign_off,
      change_capacity: call_Providers_change_capacity,
      add_value_prop: call_Providers_add_value_prop,
    },
    {
      MspSignUpSuccess: evt_Providers_MspSignUpSuccess,
      BspSignUpSuccess: evt_Providers_BspSignUpSuccess,
      MspSignOffSuccess: evt_Providers_MspSignOffSuccess,
      BspSignOffSuccess: evt_Providers_BspSignOffSuccess,
      TotalDataChanged: evt_Providers_TotalDataChanged,
    },
    {
      AlreadyRegistered: err_Providers_AlreadyRegistered,
      StorageTooLow: err_Providers_StorageTooLow,
      NotEnoughBalance: err_Providers_NotEnoughBalance,
      CannotHoldDeposit: err_Providers_CannotHoldDeposit,
      MaxBspsReached: err_Providers_MaxBspsReached,
      MaxMspsReached: err_Providers_MaxMspsReached,
      NotRegistered: err_Providers_NotRegistered,
      SpRegisteredButDataNotFound: err_Providers_SpRegisteredButDataNotFound,
      StorageStillInUse: err_Providers_StorageStillInUse,
      NotEnoughTimePassed: err_Providers_NotEnoughTimePassed,
      NoUserId: err_Providers_NoUserId,
      NoBucketId: err_Providers_NoBucketId,
      NoMultiAddress: err_Providers_NoMultiAddress,
      InvalidMultiAddress: err_Providers_InvalidMultiAddress,
      Overflow: err_Providers_Overflow,
    },
    {
      SpMinDeposit: const_Providers_SpMinDeposit,
      SpMinCapacity: const_Providers_SpMinCapacity,
      DepositPerData: const_Providers_DepositPerData,
      MaxBsps: const_Providers_MaxBsps,
      MaxMsps: const_Providers_MaxMsps,
      MaxMultiAddressSize: const_Providers_MaxMultiAddressSize,
      MaxMultiAddressAmount: const_Providers_MaxMultiAddressAmount,
      MaxProtocols: const_Providers_MaxProtocols,
      MaxBuckets: const_Providers_MaxBuckets,
    },
  ],
  FileSystem: [
    {
      StorageRequests: stg_FileSystem_StorageRequests,
      StorageRequestBsps: stg_FileSystem_StorageRequestBsps,
      StorageRequestExpirations: stg_FileSystem_StorageRequestExpirations,
      NextAvailableExpirationInsertionBlock: stg_FileSystem_NextAvailableExpirationInsertionBlock,
      NextStartingBlockToCleanUp: stg_FileSystem_NextStartingBlockToCleanUp,
    },
    {
      create_bucket: call_FileSystem_create_bucket,
      issue_storage_request: call_FileSystem_issue_storage_request,
      revoke_storage_request: call_FileSystem_revoke_storage_request,
      bsp_volunteer: call_FileSystem_bsp_volunteer,
      bsp_stop_storing: call_FileSystem_bsp_stop_storing,
    },
    {
      NewStorageRequest: evt_FileSystem_NewStorageRequest,
      AcceptedBspVolunteer: evt_FileSystem_AcceptedBspVolunteer,
      StorageRequestExpired: evt_FileSystem_StorageRequestExpired,
      StorageRequestRevoked: evt_FileSystem_StorageRequestRevoked,
      BspStoppedStoring: evt_FileSystem_BspStoppedStoring,
    },
    {
      StorageRequestAlreadyRegistered: err_FileSystem_StorageRequestAlreadyRegistered,
      StorageRequestNotFound: err_FileSystem_StorageRequestNotFound,
      BspsRequiredCannotBeZero: err_FileSystem_BspsRequiredCannotBeZero,
      BspsRequiredExceedsMax: err_FileSystem_BspsRequiredExceedsMax,
      BspVolunteerFailed: err_FileSystem_BspVolunteerFailed,
      StorageRequestBspsRequiredFulfilled: err_FileSystem_StorageRequestBspsRequiredFulfilled,
      BspAlreadyVolunteered: err_FileSystem_BspAlreadyVolunteered,
      StorageRequestExpiredNoSlotAvailable: err_FileSystem_StorageRequestExpiredNoSlotAvailable,
      StorageRequestNotAuthorized: err_FileSystem_StorageRequestNotAuthorized,
      MaxBlockNumberReached: err_FileSystem_MaxBlockNumberReached,
    },
    {
      TargetBspsRequired: const_FileSystem_TargetBspsRequired,
      MaxBspsPerStorageRequest: const_FileSystem_MaxBspsPerStorageRequest,
      MaxFilePathSize: const_FileSystem_MaxFilePathSize,
      MaxMultiAddressSize: const_FileSystem_MaxMultiAddressSize,
      MaxMultiAddresses: const_FileSystem_MaxMultiAddresses,
      StorageRequestTtl: const_FileSystem_StorageRequestTtl,
      MaxExpiredStorageRequests: const_FileSystem_MaxExpiredStorageRequests,
    },
  ],
  ProofsDealer: [
    {
      BlockToChallenges: stg_ProofsDealer_BlockToChallenges,
      BlockToChallengedSps: stg_ProofsDealer_BlockToChallengedSps,
      LastBlockSpSubmittedProofFor: stg_ProofsDealer_LastBlockSpSubmittedProofFor,
      ChallengesQueue: stg_ProofsDealer_ChallengesQueue,
      PriorityChallengesQueue: stg_ProofsDealer_PriorityChallengesQueue,
      LastCheckpointBlock: stg_ProofsDealer_LastCheckpointBlock,
    },
    {
      challenge: call_ProofsDealer_challenge,
      submit_proof: call_ProofsDealer_submit_proof,
      new_challenges_round: call_ProofsDealer_new_challenges_round,
    },
    {
      NewChallenge: evt_ProofsDealer_NewChallenge,
      ProofRejected: evt_ProofsDealer_ProofRejected,
      ProofAccepted: evt_ProofsDealer_ProofAccepted,
    },
    {
      ChallengesQueueOverflow: err_ProofsDealer_ChallengesQueueOverflow,
      PriorityChallengesQueueOverflow: err_ProofsDealer_PriorityChallengesQueueOverflow,
      NotProvider: err_ProofsDealer_NotProvider,
      FeeChargeFailed: err_ProofsDealer_FeeChargeFailed,
    },
    {
      MaxChallengesPerBlock: const_ProofsDealer_MaxChallengesPerBlock,
      MaxProvidersChallengedPerBlock: const_ProofsDealer_MaxProvidersChallengedPerBlock,
      ChallengeHistoryLength: const_ProofsDealer_ChallengeHistoryLength,
      ChallengesQueueLength: const_ProofsDealer_ChallengesQueueLength,
      CheckpointChallengePeriod: const_ProofsDealer_CheckpointChallengePeriod,
      ChallengesFee: const_ProofsDealer_ChallengesFee,
      Treasury: const_ProofsDealer_Treasury,
    },
  ],
};

type IRuntimeCalls = {
  /**
   * API necessary for block authorship with aura.
   */
  AuraApi: {
    /**
     * Returns the slot duration for Aura.
     *
     * Currently, only the value provided by this type at genesis will be used.
     */
    slot_duration: RuntimeDescriptor<[], bigint>;

    /**
     * Return the current set of authorities.
     */
    authorities: RuntimeDescriptor<[], Array<Binary>>;
  };

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

 */
  TransactionPaymentCallApi: {
    /**
     * Query information of a dispatch class, weight, and fee of a given encoded `Call`.
     */
    query_call_info: RuntimeDescriptor<
      [
        call: Anonymize<
          AnonymousEnum<{
            System: Anonymize<SystemPalletCall>;
            ParachainSystem: Anonymize<Ia0jlnena5ajog>;
            Timestamp: Anonymize<TimestampPalletCall>;
            ParachainInfo: Anonymize<undefined>;
            Balances: Anonymize<Ibf8j84ii3a3kr>;
            Sudo: Anonymize<Iam913892vifu6>;
            CollatorSelection: Anonymize<I6ggjare8v1go5>;
            Session: Anonymize<I3v8vq7j9grsdj>;
            XcmpQueue: Anonymize<I286uete0pvcbe>;
            PolkadotXcm: Anonymize<I3br2bgla1bs2h>;
            CumulusXcm: Anonymize<undefined>;
            MessageQueue: Anonymize<I8lmlccfrohcqg>;
            Providers: Anonymize<I9jhevh1bis85g>;
            FileSystem: Anonymize<I8u4nbk1d32u7q>;
            ProofsDealer: Anonymize<Iaoc2q2c87hkb1>;
          }>
        >,
        len: number,
      ],
      { weight: Anonymize<I4q39t5hn830vp>; class: DispatchClass; partial_fee: bigint }
    >;

    /**
     * Query fee details of a given encoded `Call`.
     */
    query_call_fee_details: RuntimeDescriptor<
      [
        call: Anonymize<
          AnonymousEnum<{
            System: Anonymize<SystemPalletCall>;
            ParachainSystem: Anonymize<Ia0jlnena5ajog>;
            Timestamp: Anonymize<TimestampPalletCall>;
            ParachainInfo: Anonymize<undefined>;
            Balances: Anonymize<Ibf8j84ii3a3kr>;
            Sudo: Anonymize<Iam913892vifu6>;
            CollatorSelection: Anonymize<I6ggjare8v1go5>;
            Session: Anonymize<I3v8vq7j9grsdj>;
            XcmpQueue: Anonymize<I286uete0pvcbe>;
            PolkadotXcm: Anonymize<I3br2bgla1bs2h>;
            CumulusXcm: Anonymize<undefined>;
            MessageQueue: Anonymize<I8lmlccfrohcqg>;
            Providers: Anonymize<I9jhevh1bis85g>;
            FileSystem: Anonymize<I8u4nbk1d32u7q>;
            ProofsDealer: Anonymize<Iaoc2q2c87hkb1>;
          }>
        >,
        len: number,
      ],
      { inclusion_fee: Anonymize<Id37fum600qfau>; tip: bigint }
    >;

    /**
     * Query the output of the current `WeightToFee` given some input.
     */
    query_weight_to_fee: RuntimeDescriptor<
      [weight: { ref_time: bigint; proof_size: bigint }],
      bigint
    >;

    /**
     * Query the output of the current `LengthToFee` given some input.
     */
    query_length_to_fee: RuntimeDescriptor<[length: number], bigint>;
  };

  /**
   * Runtime api to collect information about a collation.
   */
  CollectCollationInfo: {
    /**
     * Collect information about a collation.
     *
     * The given `header` is the header of the built block for that
     * we are collecting the collation info for.
     */
    collect_collation_info: RuntimeDescriptor<
      [
        header: {
          parent_hash: Binary;
          number: number;
          state_root: Binary;
          extrinsics_root: Binary;
          digest: Anonymize<Idin6nhq46lvdj>;
        },
      ],
      {
        upward_messages: Anonymize<Itom7fk49o0c9>;
        horizontal_messages: Anonymize<I6r5cbv8ttrb09>;
        new_validation_code: Anonymize<Iabpgqcjikia83>;
        processed_downward_messages: number;
        hrmp_watermark: number;
        head_data: Binary;
      }
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
  AuraApi: {
    slot_duration: runtime_AuraApi_slot_duration,
    authorities: runtime_AuraApi_authorities,
  },
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
  TransactionPaymentCallApi: {
    query_call_info: runtime_TransactionPaymentCallApi_query_call_info,
    query_call_fee_details: runtime_TransactionPaymentCallApi_query_call_fee_details,
    query_weight_to_fee: runtime_TransactionPaymentCallApi_query_weight_to_fee,
    query_length_to_fee: runtime_TransactionPaymentCallApi_query_length_to_fee,
  },
  CollectCollationInfo: {
    collect_collation_info: runtime_CollectCollationInfo_collect_collation_info,
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
