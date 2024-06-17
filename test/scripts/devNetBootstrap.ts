import * as compose from "docker-compose";
import * as util from "node:util";
import * as child_process from "node:child_process";
import * as path from "node:path";

const exec = util.promisify(child_process.exec);

async function getContainerIp(containerName: string): Promise<string> {
  const { stdout } = await exec(
    `docker inspect -f '{{range .NetworkSettings.Networks}}{{.IPAddress}}{{end}}' ${containerName}`
  );
  return stdout.trim();
}

async function getContainerPeerId(containerIp: string): Promise<string> {
  const url = `http://${containerIp}:9944`;
  const response = await fetch(url, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      jsonrpc: "2.0",
      id: 1,
      method: "system_localPeerId",
      params: [],
    }),
  });
  const data = await response.json();
  return data.result;
}

async function main() {
  const composeFilePath = path.resolve(process.cwd(), "..", "docker", "local-dev-bsp-compose.yml");

  await compose.upOne("sh-collator", { config: composeFilePath, log: true });

  console.log("Waiting for sh-collator to start...");
  await new Promise((resolve) => setTimeout(resolve, 10000)); // Adjust as necessary

  const collatorIp = await getContainerIp("docker-sh-collator-1");
  console.log(`sh-collator IP: ${collatorIp}`);

  const collatorPeerId = await getContainerPeerId(collatorIp);
  console.log(`sh-collator Peer ID: ${collatorPeerId}`);

  process.env.COLLATOR_IP = collatorIp;
  process.env.COLLATOR_PEER_ID = collatorPeerId;

  await compose.upMany(["sh-bsp", "sh-user"], {
    config: composeFilePath,
    log: true,
    env: { ...process.env, COLLATOR_IP: collatorIp, COLLATOR_PEER_ID: collatorPeerId },
  });
}

main().catch((err) => {
  console.error("Error running bootstrap script:", err);
});
