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

export const ETH_BSP_ADDRESS: `0x${string}` = "0xaDA5B4d99902df61756551df1504bc000685cf17";
export const ETH_BSP_PRIVATE_KEY: `0x${string}` =
  "0x02d7f4878c6451940547b8fe3b23c7e836f1fa165bf5d7731bfa2421d945b6d0";

export const ETH_BSP_DOWN_ADDRESS: `0x${string}` = "0xAbAc9Eb0BF6535253cF6cCBeBefbBf8eD366D033";
export const ETH_BSP_DOWN_PRIVATE_KEY: `0x${string}` =
  "0x6f5cc50588a4282ab5db2ba46c6b3b0a4b21bda9f056a36ff27a3fe9c7cd1e12";

export const ETH_BSP_TWO_ADDRESS: `0x${string}` = "0x00ccE3AA8D15FAae2398Bb14DeE443c0415FE724";
export const ETH_BSP_TWO_PRIVATE_KEY: `0x${string}` =
  "0x51dda30a0107d7e533a367afe852bea5570dc701e249eefca6ff261809e67da2";

export const ETH_BSP_THREE_ADDRESS: `0x${string}` = "0x1583FF86ab9c94941533224E67CAf9db52609576";
export const ETH_BSP_THREE_PRIVATE_KEY: `0x${string}` =
  "0x7b5c4455651396c78da9c5f6eedb1dcb4175b66a68ab3596b83d4417cd0d2866";

export const ETH_MSP_ADDRESS: `0x${string}` = "0x4C31b93792AB99E2553bfF747199B7A4951185B2";
export const ETH_MSP_PRIVATE_KEY: `0x${string}` =
  "0x5eea060cbd4e447e0adf486fcb45c68b59bd5e4c32b53c7b3af936fa3b44ab62";

export const ETH_MSP_DOWN_ADDRESS: `0x${string}` = "0xAD32cb16E648535D4935D343341131bf02C2AdD2";
export const ETH_MSP_DOWN_PRIVATE_KEY: `0x${string}` =
  "0xaf4def510d0dea37ace14f5a26acd619f0da7a86d65900be683706cefc4fc269";

export const ETH_MSP_TWO_ADDRESS: `0x${string}` = "0xe41DA1011F8F60b4Af9A152FD8081D4d9C48BeA7";
export const ETH_MSP_TWO_PRIVATE_KEY: `0x${string}` =
  "0x1ef15965036fbc70d0bf4e00ef0e6c9f303b29e50d6f915fd690f0140e582f37";

export const ETH_MSP_THREE_ADDRESS: `0x${string}` = "0x84953F1745c147b06649AE875a70Dd2864210011";
export const ETH_MSP_THREE_PRIVATE_KEY: `0x${string}` =
  "0xe32eda6c988cde1a3b37a62a41e34e59bfda6531aedf26d285d1480c0283cbd2";

export const ETH_SH_USER_ADDRESS: `0x${string}` = "0x0B17ca3A1454cD058B231090C6fd635dD348659A";
export const ETH_SH_USER_PRIVATE_KEY: `0x${string}` =
  "0x7b0ea7019f6644a02ff36d8135a55db4db8c4d4727e349c2f2a7463e6aff963c";

export const ETH_FISHERMAN_ADDRESS: `0x${string}` = "0x1B27ca3B1464cD058B231090C6fd635dD348669B";
export const ETH_FISHERMAN_PRIVATE_KEY: `0x${string}` =
  "0x8b0ea7019f6644a02ff36d8135a55db4db8c4d4727e349c2f2a7463e6aff974d";

export const ethBspKey = keyringEth.addFromUri(ETH_BSP_PRIVATE_KEY, { name: "Sh-BSP" });
export const ethBspDownKey = keyringEth.addFromUri(ETH_BSP_DOWN_PRIVATE_KEY, {
  name: "Sh-BSP-Down"
});
export const ethBspTwoKey = keyringEth.addFromUri(ETH_BSP_TWO_PRIVATE_KEY, { name: "Sh-BSP-Two" });
export const ethBspThreeKey = keyringEth.addFromUri(ETH_BSP_THREE_PRIVATE_KEY, {
  name: "Sh-BSP-Three"
});
export const ethMspKey = keyringEth.addFromUri(ETH_MSP_PRIVATE_KEY, { name: "Sh-MSP" });
export const ethMspDownKey = keyringEth.addFromUri(ETH_MSP_DOWN_PRIVATE_KEY, {
  name: "Sh-MSP-Down"
});
export const ethMspTwoKey = keyringEth.addFromUri(ETH_MSP_TWO_PRIVATE_KEY, { name: "Sh-MSP-Two" });
export const ethMspThreeKey = keyringEth.addFromUri(ETH_MSP_THREE_PRIVATE_KEY, {
  name: "Sh-MSP-Three"
});
export const ethShUser = keyringEth.addFromUri(ETH_SH_USER_PRIVATE_KEY, {
  name: "Sh-User"
});
export const ethFishermanKey = keyringEth.addFromUri(ETH_FISHERMAN_PRIVATE_KEY, {
  name: "Sh-Fisherman"
});

export const ethSudo = alith;

export const createEthereumAccount = async (privateKey?: string) => {
  const rand = `0x${randomBytes(32).toString("hex")}`;
  console.log("random", rand);
  const keyring = new Keyring({ type: "ethereum" });
  const account = keyring.addFromUri(privateKey || rand);
  return account;
};
