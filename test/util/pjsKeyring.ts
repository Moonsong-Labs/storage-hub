import { randomBytes } from "node:crypto";
import { Keyring } from "@polkadot/api";
import { cryptoWaitReady } from "@polkadot/util-crypto";

export const keyring = new Keyring({ type: "sr25519" });
await cryptoWaitReady();

export const alice = keyring.addFromUri("//Alice", { name: "Alice default" });

export const bob = keyring.addFromUri("//Bob", { name: "Bob default" });

export const charlie = keyring.addFromUri("//Charlie", {
  name: "Charlie default"
});

export const dave = keyring.addFromUri("//Dave", { name: "Dave default" });

export const eve = keyring.addFromUri("//Eve", { name: "Eve default" });

export const ferdie = keyring.addFromUri("//Ferdie", {
  name: "Ferdie default"
});

export const bspSeed = "//Sh-BSP";
export const bspKey = keyring.addFromUri(bspSeed, { name: "Sh-BSP" });
export const bspDownSeed = "//Sh-BSP-Down";
export const bspDownKey = keyring.addFromUri(bspDownSeed, { name: "Sh-BSP-Down" });
export const bspTwoSeed = "//Sh-BSP-Two";
export const bspTwoKey = keyring.addFromUri(bspTwoSeed, { name: "Sh-BSP-Two" });
export const bspThreeSeed = "//Sh-BSP-Three";
export const bspThreeKey = keyring.addFromUri(bspThreeSeed, { name: "Sh-BSP-Three" });

export const mspSeed = "//Sh-MSP";
export const mspKey = keyring.addFromUri(mspSeed, { name: "Sh-MSP" });
export const mspDownSeed = "//Sh-MSP-Down";
export const mspDownKey = keyring.addFromUri(mspDownSeed, { name: "Sh-MSP-Down" });
export const mspTwoSeed = "//Sh-MSP-Two";
export const mspTwoKey = keyring.addFromUri(mspTwoSeed, { name: "Sh-MSP-Two" });
export const mspThreeSeed = "//Sh-MSP-Three";
export const mspThreeKey = keyring.addFromUri(mspThreeSeed, { name: "Sh-MSP-Three" });

export const collator = keyring.addFromUri("//Sh-collator", {
  name: "Sh-collator"
});

export const shUser = keyring.addFromUri("//Sh-User", {
  name: "Sh-User"
});

export const sudo = alice;

export const createSr25519Account = async (privateKey?: string) => {
  const rand = `0x${randomBytes(32).toString("hex")}`;
  console.log("random", rand);
  const keyring = new Keyring({ type: "sr25519" });
  const account = keyring.addFromUri(privateKey || rand);
  return account;
};
