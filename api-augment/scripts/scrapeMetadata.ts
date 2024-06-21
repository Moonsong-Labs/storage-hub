import fs from "node:fs";
import { execSync, spawn } from "node:child_process";
import path from "node:path";

const fetchMetadata = async () => {
  const url = "http://localhost:9944";
  const payload = {
    id: "1",
    jsonrpc: "2.0",
    method: "state_getMetadata",
    params: [],
  };

  for (let i = 0; i < 10; i++) {
    try {
      const response = await fetch(url, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify(payload),
      });

      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
      }

      return response.arrayBuffer();
    } catch {
      console.log("Waiting for node to launch...");
      await new Promise((resolve) => setTimeout(resolve, 1000));
    }
  }
  throw new Error("Error fetching metadata");
};

async function main() {
  const nodePath = path.join(process.cwd(), "..", "target", "release", "storage-hub-node");
  const metadataPath = path.join(process.cwd(), "storagehub.json");

  if (!fs.existsSync(nodePath)) {
    console.error("Storage Hub Node not found at path: ", nodePath);
    throw new Error("File not found");
  }

  spawn(
    "docker",
    [
      "compose",
      "-f=../docker/local-node-compose.yml",
      "up",
      "--remove-orphans",
      "--renew-anon-volumes",
    ],
    {
      stdio: "inherit",
    }
  );

  const metadata = await fetchMetadata();
  fs.writeFileSync(metadataPath, Buffer.from(metadata));

  console.log("✅ Metadata file written to:", metadataPath);
}

main()
  .catch((error) => {
    console.error(error);
    process.exitCode = 1;
  })
  .finally(() => {
    execSync("docker compose -f=../docker/local-node-compose.yml down --remove-orphans --volumes");
  });
