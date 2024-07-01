import { NODE_INFOS, runBspNet, type ToxicInfo } from "../util";

const registerToxic = async (toxicDef: ToxicInfo) => {
  const url = `http://localhost:${NODE_INFOS.toxiproxy.port}/proxies/sh-bsp-proxy/toxics`;

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

const getToxics = async () => {
  const url = `http://localhost:${NODE_INFOS.toxiproxy.port}/proxies/sh-bsp-proxy/toxics`;
  const resp = await fetch(url);
  return resp.json();
};

async function bootStrapNetwork() {
  try {
    await runBspNet(true);

    // For more info on the kind of toxics you can register,
    // see: https://github.com/Shopify/toxiproxy?tab=readme-ov-file#toxics
    const reqToxics = [
      {
        type: "latency",
        name: "lag-down",
        stream: "downstream",
        toxicity: 1,
        attributes: {
          latency: 500,
          jitter: 50
        }
      },
      {
        type: "latency",
        name: "lag-up",
        stream: "upstream",
        toxicity: 1,
        attributes: {
          latency: 500,
          jitter: 50
        }
      }
    ] satisfies ToxicInfo[];

    // Register toxics with proxy server
    const promises = reqToxics.map(registerToxic);
    await Promise.all(promises);

    // Verify each toxic is registered
    const receivedToxics: any = await getToxics();

    if (receivedToxics.length !== reqToxics.length) {
      console.log("❌ Toxic registration failed");
      console.log(receivedToxics);
      throw new Error("Toxic registration failed");
    }

    console.log("✅ NoisyNet Bootstrap success");
  } catch (e) {
    console.error("Error running bootstrap script:", e);
    console.log("❌ BSPNet Bootstrap failure");
    process.exitCode = 1;
  }
}

bootStrapNetwork();
