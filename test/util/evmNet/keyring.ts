import { randomBytes } from "node:crypto";
import { Keyring } from "@polkadot/api";
import type { KeyringPair } from "@polkadot/keyring/types";

export const keyringEth = new Keyring({ type: "ethereum" });

// Pre-funded accounts.
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

export const ETH_BSP_SURI =
  "twelve blame high motor print novel romance trumpet noodle roast poverty labor";
export const ETH_BSP_ADDRESS: `0x${string}` = "0xaDA5B4d99902df61756551df1504bc000685cf17";
export const ETH_BSP_PRIVATE_KEY: `0x${string}` =
  "0x02d7f4878c6451940547b8fe3b23c7e836f1fa165bf5d7731bfa2421d945b6d0";

// TODO
// export const ETH_BSP_DOWN_ADDRESS: `0x${string}` = "0xeC64B758C12cd44Ea7903D7c98BfDf461eaeFd92";
// export const ETH_BSP_DOWN_PRIVATE_KEY: `0x${string}` =
//   "0x6a6d413e804b39b6f5995b2e98a2b9a274e78c6b0408410b3cf8d7b00744c445";

// TODO
// export const ETH_BSP_TWO_ADDRESS: `0x${string}` = "0x13993159c4140Bf257639c540e043Cdfa4911C13";
// export const ETH_BSP_TWO_PRIVATE_KEY: `0x${string}` =
//   "0xf9038c4d3036e9dd6701d1bd03e07ef5b851733d3f767ab0f40a0969d927b160";

// TODO
// export const ETH_BSP_THREE_ADDRESS: `0x${string}` = "0xbdb90Dcf5887F32A2139Fa16C154582c3993B330";
// export const ETH_BSP_THREE_PRIVATE_KEY: `0x${string}` =
//   "0x34e2f9c0dbd3b24bc4a0106e991a6fca9e3560f0249e9ec83ad0040548dd2b3b";

export const ETH_MSP_SURI = "fish fat knife siren learn copper aspect process mad silly judge dawn";
export const ETH_MSP_ADDRESS: `0x${string}` = "0x4C31b93792AB99E2553bfF747199B7A4951185B2";
export const ETH_MSP_PRIVATE_KEY: `0x${string}` =
  "0x5eea060cbd4e447e0adf486fcb45c68b59bd5e4c32b53c7b3af936fa3b44ab62";

// TODO
// export const ETH_MSP_DOWN_ADDRESS: `0x${string}` = "0x494c909696a6BF440468835771B283eDCf23D703";
// export const ETH_MSP_DOWN_PRIVATE_KEY: `0x${string}` =
//   "0x8cd3c8d0c2eb3c88bb83ceeb3077e48e66219673c33abeee65bd3c08d6c9ca45";

export const ETH_MSP_TWO_SURI =
  "embark stock dog abstract caught drama inherit where assume tattoo issue metal";
export const ETH_MSP_TWO_ADDRESS: `0x${string}` = "0xe41DA1011F8F60b4Af9A152FD8081D4d9C48BeA7";
export const ETH_MSP_TWO_PRIVATE_KEY: `0x${string}` =
  "0x1ef15965036fbc70d0bf4e00ef0e6c9f303b29e50d6f915fd690f0140e582f37";

// TODO
// export const ETH_MSP_THREE_ADDRESS: `0x${string}` = "0x77928a85b791767389645d381781785848883545";
// export const ETH_MSP_THREE_PRIVATE_KEY: `0x${string}` =
//   "0x17757925447316846320910170348639369268526441838623640550256243754695006626462";

// TODO
export const ETH_SH_USER_SURI =
  "lens vital off hurry accuse addict fashion wine grunt pool include bright";
export const ETH_SH_USER_ADDRESS: `0x${string}` = "0x0B17ca3A1454cD058B231090C6fd635dD348659A";
export const ETH_SH_USER_PRIVATE_KEY: `0x${string}` =
  "0x7b0ea7019f6644a02ff36d8135a55db4db8c4d4727e349c2f2a7463e6aff963c";

export const ethBspKey = keyringEth.addFromUri(ETH_BSP_PRIVATE_KEY, { name: "Sh-BSP" });
// export const ethBspDownKey = keyringEth.addFromUri(ETH_BSP_DOWN_PRIVATE_KEY, {
//   name: "Sh-BSP-Down"
// });
// export const ethBspTwoKey = keyringEth.addFromUri(ETH_BSP_TWO_PRIVATE_KEY, { name: "Sh-BSP-Two" });
// export const ethBspThreeKey = keyringEth.addFromUri(ETH_BSP_THREE_PRIVATE_KEY, {
//   name: "Sh-BSP-Three"
// });
export const ethMspKey = keyringEth.addFromUri(ETH_MSP_PRIVATE_KEY, { name: "Sh-MSP" });
// export const ethMspDownKey = keyringEth.addFromUri(ETH_MSP_DOWN_PRIVATE_KEY, {
//   name: "Sh-MSP-Down"
// });
// export const ethMspTwoKey = keyringEth.addFromUri(ETH_MSP_TWO_PRIVATE_KEY, { name: "Sh-MSP-Two" });
// export const ethMspThreeKey = keyringEth.addFromUri(ETH_MSP_THREE_PRIVATE_KEY, {
//   name: "Sh-MSP-Three"
// });
export const ethShUser = keyringEth.addFromUri(ETH_SH_USER_PRIVATE_KEY, {
  name: "Sh-User"
});

export const ethSudo = alith;

export const createEthereumAccount = async (privateKey?: string) => {
  const rand = `0x${randomBytes(32).toString("hex")}`;
  console.log("random", rand);
  const keyring = new Keyring({ type: "ethereum" });
  const account = keyring.addFromUri(privateKey || rand);
  return account;
};
