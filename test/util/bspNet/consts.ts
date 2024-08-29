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
    fingerprint: "0x9e86ecad3f86f52891acf9c0d47c5c5243a3592fc7bcf99811eb7db078997dcb"
  },
  "res/smile.jpg": {
    size: 633160n,
    checksum: "12094d47c2fdf1a984c0b950c2c0ede733722bea3bee22fef312e017383b410c",
    fingerprint: "0x00d6e8c6410561b5e59266706b7e82dc1148735c93cbc33bd6d7b6d62d435200"
  },
  "res/whatsup.jpg": {
    size: 216211n,
    checksum: "585ed00a96349499cbc8a3882b0bd6f6aec5ce3b7dbee2d8b3d33f3c09a38ec6",
    fingerprint: "0x0e2aaf768af5b738eea96084f10dac7ad4f6efa257782bdb9823994ffb233344"
  },
  "res/cloud.jpg": {
    size: 346248n,
    checksum: "8e06811883fc3e5e6a0331825b365e4bd7b83ba7683fa9da17e4daea25d7a9f5",
    fingerprint: "0x00b122b1b40969f3b6ce9277d511d7f771f8a4e213fa1a8a2951662bd3044000"
  },
  "res/empty-file": {
    size: 0n,
    checksum: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    fingerprint: "0x03170a2e7597b7b7e3d84c05391d139a62b157e78786d8c082f29dcf4c111314"
  },
  "res/half-chunk-file": {
    size: 512n,
    checksum: "c7b3b7dd37d7e0947b04550613692950c72b0551e038a01ab8679a3ea5631104",
    fingerprint: "0x6a62615cbe76b0ad7052849b414d96f847ea29953ba73ac3d98476d3f54109fe"
  },
  "res/one-chunk-file": {
    size: 1024n,
    checksum: "1f006b6a97eeb0dfd8cbc91ed815e6a429dcfdc2f3f32f2ac3e7977e70df4988",
    fingerprint: "0xce794511342582a9a466fbf3a02fb81ae4b8d4632ec88f39895e4700d03fb902"
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
