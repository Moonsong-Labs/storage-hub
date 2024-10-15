import * as ShConsts from "../consts";
import type { ToxicInfo } from "../types";

export const registerToxic = async (toxicDef: ToxicInfo) => {
  const url = `http://localhost:${ShConsts.NODE_INFOS.toxiproxy.port}/proxies/sh-bsp-proxy/toxics`;

  const options: RequestInit = {
    method: "POST",
    headers: {
      "Content-Type": "application/json"
    },
    body: JSON.stringify(toxicDef)
  };

  const resp = await fetch(url, options);

  return resp.json();
};

export const getToxics = async () => {
  const url = `http://localhost:${ShConsts.NODE_INFOS.toxiproxy.port}/proxies/sh-bsp-proxy/toxics`;
  const resp = await fetch(url);
  return (await resp.json()) as ToxicInfo[];
};

export const registerToxics = async (toxics: ToxicInfo[]) => {
  // Register toxics with proxy server
  const promises = toxics.map(registerToxic);
  await Promise.all(promises);

  // Verify each toxic is registered
  const receivedToxics: any = await getToxics();
  console.log(receivedToxics);

  if (receivedToxics.length !== toxics.length) {
    console.log("❌ Toxic registration failed");
    console.log(receivedToxics);
    throw "Toxic registration failed";
  }
};
