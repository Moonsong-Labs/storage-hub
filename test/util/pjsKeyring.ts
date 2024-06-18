import { Keyring } from "@polkadot/api";
import { randomBytes } from "node:crypto";
import { cryptoWaitReady } from "@polkadot/util-crypto";

export const keyring = new Keyring({ type: "sr25519" });
await cryptoWaitReady();

export const alice = keyring.addFromUri("//Alice", { name: "Alice default" });

export const bob = keyring.addFromUri("//Bob", { name: "Bob default" });

export const charlie = keyring.addFromUri("//Charlie", {
  name: "Charlie default",
});

export const dave = keyring.addFromUri("//Dave", { name: "Dave default" });

export const eve = keyring.addFromUri("//Eve", { name: "Eve default" });

export const ferdie = keyring.addFromUri("//Ferdie", {
  name: "Ferdie default",
});

export const bsp = keyring.addFromUri("//Sh-BSP", { name: "Sh-BSP" });

export const collator = keyring.addFromUri("//Sh-collator", {
  name: "Sh-collator",
});

export const shUser = keyring.addFromUri("//Sh-User", {
  name: "Sh-User",
});


export const createSr25519Account = async (privateKey?: string) => {
  const rand = `0x${randomBytes(32).toString("hex")}`;
  console.log("random", rand);
  const keyring = new Keyring({ type: "sr25519" });
  const account = keyring.addFromUri(privateKey || rand);
  return account;
};
