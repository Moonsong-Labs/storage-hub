import { DockerComposeEnvironment, Wait } from "testcontainers";

async function startNetwork() {
  console.log("Starting simple network...");

  const composePath = "../docker/";
  const composeFile = "storage-hub-node-compose.yml";
  const environment = await new DockerComposeEnvironment(composePath, composeFile)
    .withWaitStrategy(
      "storage-hub-node-1",
      Wait.forLogMessage(
        '[Parachain] Running JSON-RPC server: addr=0.0.0.0:9944, allowed origins=["*"]'
      )
    )
    .up();

  console.log(environment.getContainer("storage-hub-node-1").getNetworkNames());
}

startNetwork();
