import fs from "node:fs";
import { spawn, type ChildProcess } from "node:child_process";
import path from "node:path";

let nodeProcess: ChildProcess | undefined = undefined;

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

  nodeProcess = spawn("docker", ["compose", "-f=../docker/local-node-compose.yml", "up"]);

  const onProcessExit = () => {
    nodeProcess?.kill();
  };

  process.once("exit", onProcessExit);
  process.once("SIGINT", onProcessExit);

  nodeProcess.once("exit", () => {
    process.removeListener("exit", onProcessExit);
    process.removeListener("SIGINT", onProcessExit);
  });

  const metadata = await fetchMetadata();
  fs.writeFileSync(metadataPath, Buffer.from(metadata));

  console.log("✅ Metadata file written to:", metadataPath);
}

main()
  .catch((error) => {
    console.error(error);
    nodeProcess?.kill();
    process.exit(1);
  })
  .then(() => {
    nodeProcess?.kill();
    process.exit(0);
  });
