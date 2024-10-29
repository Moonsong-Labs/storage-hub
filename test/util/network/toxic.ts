import * as ShConsts from "./consts";

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

/**
 * Represents information about a network toxicity.
 * This interface is used to describe a Toxic "debuff" that can be applied to a running toxiproxy.
 *
 * @interface
 * @property {("latency"|"down"|"bandwidth"|"slow_close"|"timeout"|"reset_peer"|"slicer"|"limit_data")} type - The type of network toxic.
 * @property {string} name - The name of the network toxic.
 * @property {("upstream"|"downstream")} stream - The link direction of the network toxic.
 * @property {number} toxicity - The probability of the toxic being applied to a link (defaults to 1.0, 100%)
 * @property {Object} attributes - A map of toxic-specific attributes
 */
export interface ToxicInfo {
  type:
    | "latency"
    | "down"
    | "bandwidth"
    | "slow_close"
    | "timeout"
    | "reset_peer"
    | "slicer"
    | "limit_data";
  name: string;
  stream: "upstream" | "downstream";
  toxicity: number;
  attributes: {
    [key: string]: string | number | undefined;
  };
}
