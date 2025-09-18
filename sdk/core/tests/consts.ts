// Shared test constants for SDK Core tests
// These values mirror Hardhat's default accounts and common test mnemonics.
// NOTE: You can compute the private key, public key and address using https://iancoleman.io/bip39/
// Test data imported from ./consts.ts
export const TEST_MNEMONIC_12 = 'test test test test test test test test test test test junk' as const;
export const TEST_PRIVATE_KEY_12 = '0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80' as const;
export const TEST_ADDRESS_12 = '0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266' as const;

export const TEST_MNEMONIC_24 = 'abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon art' as const;
export const TEST_ADDRESS_24 = '0xF278cF59F82eDcf871d630F28EcC8056f25C1cdb' as const;

// Prefunded accounts for SH-EVM_SOLO
export const ALITH = {
  address: '0xf24FF3a9CF04c71Dbc94D0b566f7A27B94566cac' as const,
  privateKey: '0x5fb92d6e98884f76de468fa3f6278f8807c48bebc13595d45af5bdc4da702133' as const,
};
export const BALTATHAR = {
  address: '0x3Cd0A705a2DC65e5b1E1205896BaA2be8A07c6e0' as const,
  privateKey: '0x8075991ce870b93a8870eca0c0f91913d12f47948ca0fd25b49c6fa7cdbeee8b' as const,
};
export const CHARLETH = {
  address: '0x798d4Ba9baf0064Ec19eB4F0a1a45785ae9D6DFc' as const,
  privateKey: '0x0b6e18cafb6ed99687ec547bd28139cafdd2bffe70e6b688025de6b445aa5c5b' as const,
};
export const DOROTHY = {
  address: '0x773539d4Ac0e786233D90A233654ccEE26a613D9' as const,
  privateKey: '0x39539ab1876910bbf3a223d84a29e28f1cb4e2e456503e7e91ed39b2e7223d68' as const,
};
export const ETHAN = {
  address: '0xFf64d3F6efE2317EE2807d223a0Bdc4c0c49dfDB' as const,
  privateKey: '0x7dce9bc8babb68fec1409be38c8e1a52650206a7ed90ff956ae8a6d15eeaaef4' as const,
};
export const FAITH = {
  address: '0xC0F0f4ab324C46e55D02D0033343B4Be8A55532d' as const,
  privateKey: '0xb9d2ea9a615f3165812e8d44de0d24da9bbd164b65c4f0573e1ce2c8dbd9c8df' as const,
};
export const GOLIATH = {
  address: '0x7BF369283338E12C90514468aa3868A551AB2929' as const,
  privateKey: '0x96b8a38e12e1a31dee1eab2fffdf9d9990045f5b37e44d8cc27766ef294acf18' as const,
};
export const HEATH = {
  address: '0x931f3600a299fd9B24cEfB3BfF79388D19804BeA' as const,
  privateKey: '0x0d6dcaaef49272a5411896be8ad16c01c35d6f8c18873387b71fbc734759b0ab' as const,
};
export const IDA = {
  address: '0xC41C5F1123ECCd5ce233578B2e7ebd5693869d73' as const,
  privateKey: '0x4c42532034540267bf568198ccec4cb822a025da542861fcb146a5fab6433ff8' as const,
};
export const JUDITH = {
  address: '0x2898FE7a42Be376C8BC7AF536A940F7Fd5aDd423' as const,
  privateKey: '0x94c49300a58d576011096bcb006aa06f5a91b34b4383891e8029c21dc39fbb8b' as const,
};


