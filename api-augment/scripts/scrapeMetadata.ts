import { execSync, spawn } from "node:child_process";
import fs from "node:fs";
import path from "node:path";

const fetchMetadata = async () => {
  const maxRetries = 60;
  const sleepTime = 500;
  const url = "http://localhost:9944";
  const payload = {
    id: "1",
    jsonrpc: "2.0",
    method: "state_getMetadata",
    params: []
  };

  for (let i = 0; i < maxRetries; i++) {
    try {
      const response = await fetch(url, {
        method: "POST",
        headers: {
          "Content-Type": "application/json"
        },
        body: JSON.stringify(payload)
      });

      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
      }

      return response.arrayBuffer();
    } catch {
      console.log("Waiting for node to launch...");
      await new Promise((resolve) => setTimeout(resolve, sleepTime));
    }
  }
  console.log(`Error fetching container IP  after ${(maxRetries * sleepTime) / 1000} seconds`);
  throw new Error("Error fetching metadata");
};

const runCombination = async (metadataPath: string, dockerComposePath: string) => {
  console.log(`\n=== Starting container for compose: ${dockerComposePath} ===`);
  try {
    // Start services in background
    spawn(
      "docker",
      ["compose", "-f", dockerComposePath, "up", "-d", "--remove-orphans", "--renew-anon-volumes"],
      { stdio: "inherit" }
    );

    const metadata = await fetchMetadata();
    const jsonResponse = await new Response(metadata).json();
    fs.writeFileSync(metadataPath, JSON.stringify(jsonResponse, null, 2));
    console.log("✅ Metadata file written to:", metadataPath);
  } finally {
    try {
      execSync(`docker compose -f=${dockerComposePath} down --remove-orphans --volumes`, {
        stdio: "inherit"
      });
    } catch (e) {
      console.warn("⚠️ Error bringing docker compose down:", e);
    }
  }
};

async function main() {
  const nodePath = path.join(process.cwd(), "..", "target", "release", "storage-hub-node");
  const metadataPaths = [
    path.join(process.cwd(), "metadata-sh-parachain.json"),
    path.join(process.cwd(), "metadata-sh-solochain-evm.json")
  ];
  const dockerComposePaths = [
    path.join(process.cwd(), "..", "docker", "local-parachain-compose.yml"),
    path.join(process.cwd(), "..", "docker", "local-solochain-evm-compose.yml")
  ];

  if (!fs.existsSync(nodePath)) {
    console.error("Storage Hub Node not found at path: ", nodePath);
    throw new Error("File not found");
  }

  // Run sequentially for each combination
  for (let index = 0; index < metadataPaths.length; index++) {
    const metadataPath = metadataPaths[index];
    const dockerComposePath = dockerComposePaths[index];
    await runCombination(metadataPath, dockerComposePath);
  }
}

await main();
