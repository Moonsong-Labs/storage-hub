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

async function main() {
  const nodePath = path.join(process.cwd(), "..", "target", "release", "storage-hub-node");
  const metadataPath = path.join(process.cwd(), "metadata-sh-parachain.json");

  if (!fs.existsSync(nodePath)) {
    console.error("Storage Hub Node not found at path: ", nodePath);
    throw new Error("File not found");
  }

  // TODO: replace with dockerode
  spawn(
    "docker",
    [
      "compose",
      "-f=../docker/local-parachain-compose.yml",
      "up",
      "--remove-orphans",
      "--renew-anon-volumes"
    ],
    {
      stdio: "inherit"
    }
  );

  const metadata = await fetchMetadata();
  const jsonResponse = await new Response(metadata).json();
  fs.writeFileSync(metadataPath, JSON.stringify(jsonResponse, null, 2));

  console.log("✅ Metadata file written to:", metadataPath);

  // TODO: Scrape metadata for solochain-evm
}

main()
  .catch((error) => {
    console.error(error);
    process.exitCode = 1;
  })
  .finally(() => {
    execSync(
      "docker compose -f=../docker/local-parachain-compose.yml down --remove-orphans --volumes"
    );
  });
