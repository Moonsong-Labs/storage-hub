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
    size: 416396n,
    checksum: "739fb97f7c2b8e7f192b608722a60dc67ee0797c85ff1ea849c41333a40194f2",
    fingerprint: "0x564c33c2f4cb3417b2dbd69451be58cfce5bd2844786581278b32f939e3e4226"
  },
  "res/smile.jpg": {
    size: 633160n,
    checksum: "12094d47c2fdf1a984c0b950c2c0ede733722bea3bee22fef312e017383b410c",
    fingerprint: "0xa2d6e8c6410561b5e59266706b7e82dc1148735c93cbc33bd6d7b6d62d435271"
  },
  "res/whatsup.jpg": {
    size: 216211n,
    checksum: "1cd8cf94623bffacc718269acdab04844998d84362b8a8449f4628a2ef5a77f5",
    fingerprint: "0x0e2aaf768af5b738eea96084f10dac7ad4f6efa257782bdb9823994ffb233344"
  },
  "res/cloud.jpg": {
    size: 346248n,
    checksum: "8e06811883fc3e5e6a0331825b365e4bd7b83ba7683fa9da17e4daea25d7a9f5",
    fingerprint: "0xcfb122b1b40969f3b6ce9277d511d7f771f8a4e213fa1a8a2951662bd3044016"
  }
} as const;

export const DUMMY_MSP_ID = "0x0000000000000000000000000000000000000000000000000000000000000300";
export const VALUE_PROP = "0x0000000000000000000000000000000000000000000000000000000000000770";

// This is hardcoded to be same as fingerprint of whatsup.jpg
// This is to game the XOR so that this BSP is always chosen by network
export const DUMMY_BSP_ID = TEST_ARTEFACTS["res/whatsup.jpg"].fingerprint;

export const CAPACITY_512 = 1024n * 1024n * 512n; // 512 MB
