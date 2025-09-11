import { Keyring } from "@polkadot/api";
import type { KeyringPair } from "@polkadot/keyring/types";

const keyringEth = new Keyring({ type: "ethereum" });

// Prefunded accounts.
export const ALITH_ADDRESS: `0x${string}` = "0xf24FF3a9CF04c71Dbc94D0b566f7A27B94566cac";
export const ALITH_PRIVATE_KEY: `0x${string}` =
  "0x5fb92d6e98884f76de468fa3f6278f8807c48bebc13595d45af5bdc4da702133";

export const BALTATHAR_ADDRESS: `0x${string}` = "0x3Cd0A705a2DC65e5b1E1205896BaA2be8A07c6e0";
export const BALTATHAR_PRIVATE_KEY: `0x${string}` =
  "0x8075991ce870b93a8870eca0c0f91913d12f47948ca0fd25b49c6fa7cdbeee8b";

export const CHARLETH_ADDRESS: `0x${string}` = "0x798d4Ba9baf0064Ec19eB4F0a1a45785ae9D6DFc";
export const CHARLETH_PRIVATE_KEY: `0x${string}` =
  "0x0b6e18cafb6ed99687ec547bd28139cafdd2bffe70e6b688025de6b445aa5c5b";

export const DOROTHY_ADDRESS: `0x${string}` = "0x773539d4Ac0e786233D90A233654ccEE26a613D9";
export const DOROTHY_PRIVATE_KEY: `0x${string}` =
  "0x39539ab1876910bbf3a223d84a29e28f1cb4e2e456503e7e91ed39b2e7223d68";

export const ETHAN_ADDRESS: `0x${string}` = "0xFf64d3F6efE2317EE2807d223a0Bdc4c0c49dfDB";
export const ETHAN_PRIVATE_KEY: `0x${string}` =
  "0x7dce9bc8babb68fec1409be38c8e1a52650206a7ed90ff956ae8a6d15eeaaef4";

export const FAITH_ADDRESS: `0x${string}` = "0xC0F0f4ab324C46e55D02D0033343B4Be8A55532d";
export const FAITH_PRIVATE_KEY: `0x${string}` =
  "0xb9d2ea9a615f3165812e8d44de0d24da9bbd164b65c4f0573e1ce2c8dbd9c8df";

export const GOLIATH_ADDRESS: `0x${string}` = "0x7BF369283338E12C90514468aa3868A551AB2929";
export const GOLIATH_PRIVATE_KEY: `0x${string}` =
  "0x96b8a38e12e1a31dee1eab2fffdf9d9990045f5b37e44d8cc27766ef294acf18";

export const alith: KeyringPair = keyringEth.addFromUri(ALITH_PRIVATE_KEY);
export const baltathar: KeyringPair = keyringEth.addFromUri(BALTATHAR_PRIVATE_KEY);
export const charleth: KeyringPair = keyringEth.addFromUri(CHARLETH_PRIVATE_KEY);
export const dorothy: KeyringPair = keyringEth.addFromUri(DOROTHY_PRIVATE_KEY);
export const ethan: KeyringPair = keyringEth.addFromUri(ETHAN_PRIVATE_KEY);
export const faith: KeyringPair = keyringEth.addFromUri(FAITH_PRIVATE_KEY);
export const goliath: KeyringPair = keyringEth.addFromUri(GOLIATH_PRIVATE_KEY);

export { keyringEth };
