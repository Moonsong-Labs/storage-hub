import fs from "node:fs";
import { spawn } from "node:child_process";
import path from "node:path";

let nodeProcess: ChildProcessWithoutNullStreams | undefined = undefined;

const fetchMetadata = async () => {
  const url = "http://localhost:9944";
  const payload = {
    id: "1",
    jsonrpc: "2.0",
    method: "state_getMetadata",
    params: [],
  };

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

    const data = (await response.json()) as JSON;
    console.log(data);
    return data;
  } catch (error) {
    console.error("Error:", error);
    throw new Error("Error fetching metadata");
  }
};

async function main() {
  // launch SH Node
  const filePath = path.join(process.cwd(), "..", "target", "release", "storage-hub-node");

  if (!fs.existsSync(filePath)) {
    console.error("Storage Hub Node not found at path: ", filePath);
    throw new Error("File not found");
  }

  nodeProcess = spawn(
    filePath,
    ["--dev", "--no-hardware-benchmarks", "--no-telemetry", "--no-prometheus", "--rpc-cors=all"],
    { stdio: "inherit" }
  );

  const onProcessExit = () => {
    nodeProcess?.kill();
  };

  process.once("exit", onProcessExit);
  process.once("SIGINT", onProcessExit);

  nodeProcess.once("exit", () => {
    process.removeListener("exit", onProcessExit);
    process.removeListener("SIGINT", onProcessExit);
  });

  await new Promise((resolve) => setTimeout(resolve, 10000));

  // build fetch command
  const request = await fetchMetadata();

  // run fetch command

  // parse metadata

  // write metadata to file
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
