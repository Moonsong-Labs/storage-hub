export const NODE_INFOS = {
  user: {
    containerName: "docker-sh-user-1",
    port: 9977,
    p2pPort: 30444,
    AddressId: "5CombC1j5ZmdNMEpWYpeEWcKPPYcKsC1WgMPgzGLU72SLa4o",
    expectedPeerId: "12D3KooWMvbhtYjbhgjoDzbnf71SFznJAKBBkSGYEUtnpES1y9tM"
  },
  bsp: {
    containerName: "docker-sh-bsp-1",
    port: 9966,
    p2pPort: 30350,
    AddressId: "5FHSHEFWHVGDnyiw66DoRUpLyh5RouWkXo9GT1Sjk8qw7MAg",
    expectedPeerId: "12D3KooWNEZ8PGNydcdXTYy1SPHvkP9mbxdtTqGGFVrhorDzeTfU"
  },
  collator: {
    containerName: "docker-sh-collator-1",
    port: 9955,
    p2pPort: 30333,
    AddressId: "5C8NC6YuAivp3knYC58Taycx2scQoDcDd3MCEEgyw36Gh1R4"
  },
  toxiproxy: {
    containerName: "toxiproxy",
    port: 8474
  }
} as const;

export const TEST_ARTEFACTS = {
  "res/adolphus.jpg": {
    size: 416400n,
    checksum: "739fb97f7c2b8e7f192b608722a60dc67ee0797c85ff1ea849c41333a40194f2",
    fingerprint: "0x34eb5f637e05fc18f857ccb013250076534192189894d174ee3aa6d3525f6970"
  },
  "res/smile.jpg": {
    size: 633160n,
    checksum: "12094d47c2fdf1a984c0b950c2c0ede733722bea3bee22fef312e017383b410c",
    fingerprint: "0x535dd863026735ffe0919cc0fc3d8e5da45b9203f01fbf014dbe98005bd8d2fe"
  },
  "res/whatsup.jpg": {
    size: 216211n,
    checksum: "585ed00a96349499cbc8a3882b0bd6f6aec5ce3b7dbee2d8b3d33f3c09a38ec6",
    fingerprint: "0x2b83b972e63f52abc0d4146c4aee1f1ec8aa8e274d2ad1b626529446da93736c"
  },
  "res/cloud.jpg": {
    size: 346248n,
    checksum: "8e06811883fc3e5e6a0331825b365e4bd7b83ba7683fa9da17e4daea25d7a9f5",
    fingerprint: "0x5559299bc73782b5ad7e9dd57ba01bb06b8c44f5cab8d7afab5e1db2ea93da4c"
  },
  "res/empty-file": {
    size: 0n,
    checksum: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    fingerprint: "0x03170a2e7597b7b7e3d84c05391d139a62b157e78786d8c082f29dcf4c111314"
  },
  "res/half-chunk-file": {
    size: 512n,
    checksum: "c7b3b7dd37d7e0947b04550613692950c72b0551e038a01ab8679a3ea5631104",
    fingerprint: "0xade3ca4ff2151a2533e816eb9402ae17e21160c6c52b1855ecff29faea8880b5"
  },
  "res/one-chunk-file": {
    size: 1024n,
    checksum: "1f006b6a97eeb0dfd8cbc91ed815e6a429dcfdc2f3f32f2ac3e7977e70df4988",
    fingerprint: "0x0904317e4977ad6f872cd9672d2733da9a628fda86ee9add68623a66918cbd8c"
  }
} as const;

export const DUMMY_MSP_ID = "0x0000000000000000000000000000000000000000000000000000000000000300";
export const VALUE_PROP = "0x0000000000000000000000000000000000000000000000000000000000000770";

export const DUMMY_BSP_ID = TEST_ARTEFACTS["res/whatsup.jpg"].fingerprint;
export const BSP_TWO_ID = "0x0000000000000000000000000000000000000000000000000000000000000002";
export const BSP_THREE_ID = "0x0000000000000000000000000000000000000000000000000000000000000003";
export const BSP_DOWN_ID = "0xf000000000000000000000000000000000000000000000000000000000000000";

export const CAPACITY_5 = 1024n * 1024n * 5n; // 5 MB
export const CAPACITY_256 = 1024n * 1024n * 256n; // 256 MB
export const CAPACITY_512 = 1024n * 1024n * 512n; // 512 MB
export const CAPACITY_1024 = 1024n * 1024n * 1024n; // 1024 MB

export const CAPACITY = {
  5: CAPACITY_5,
  256: CAPACITY_256,
  512: CAPACITY_512,
  1024: CAPACITY_1024
} as const;

export const U32_MAX = (BigInt(1) << BigInt(32)) - BigInt(1);
export const MAX_STORAGE_CAPACITY = CAPACITY[1024] * 4n - 1n;
