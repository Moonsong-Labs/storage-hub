import Docker from "dockerode";
import { DOCKER_IMAGE } from ".";
import postgres from "postgres";
import stripAnsi from "strip-ansi";
import assert from "node:assert";
import fs from "node:fs/promises";
import tmp from "tmp";

export const printDockerStatus = async (verbose = false) => {
  const docker = new Docker();

  verbose && console.log("\n=== Docker Container Status ===");

  const containers = await docker.listContainers({ all: true });

  if (containers.length === 0) {
    verbose && console.log("No containers found");
    return;
  }

  if (verbose) {
    for (const container of containers) {
      console.log(`\nContainer: ${container.Names.join(", ")}`);
      console.log(`ID: ${container.Id}`);
      console.log(`Image: ${container.Image}`);
      console.log(`Status: ${container.State}/${container.Status}`);
      console.log(`Created: ${new Date(container.Created * 1000).toISOString()}`);

      if (container.State === "running") {
        try {
          const stats = await docker.getContainer(container.Id).stats({ stream: false });
          console.log("Memory Usage:", {
            usage: `${Math.round(stats.memory_stats.usage / 1024 / 1024)}MB`,
            limit: `${Math.round(stats.memory_stats.limit / 1024 / 1024)}MB`
          });
        } catch (e) {
          console.log("Could not fetch container stats");
        }
      }
    }
  }
  verbose && console.log("\n===============================\n");
};

export const verifyContainerFreshness = async () => {
  const docker = new Docker();
  const containers = await docker.listContainers({ all: true });

  const existingContainers = containers.filter(
    (container) =>
      container.Image === DOCKER_IMAGE ||
      container.Names.some((name) => name.includes("toxiproxy")) ||
      container.Names.some((name) => name.includes("storage-hub-sh-backend"))
  );

  if (existingContainers.length > 0) {
    console.log("\n=== WARNING: Found existing containers ===");
    for (const container of existingContainers) {
      console.log(`Container: ${container.Names.join(", ")}`);
      console.log(`Created: ${new Date(container.Created * 1000).toISOString()}`);
      console.log(`Status: ${container.State}/${container.Status}`);

      const containerInfo = await docker.getContainer(container.Id).inspect();
      console.log(
        "Mounts:",
        containerInfo.Mounts.map((m) => m.Source)
      );
      console.log("---");
    }
    throw new Error("Test environment is not clean - found existing containers");
  }
};

export const createSqlClient = () => {
  return postgres({
    host: "localhost",
    port: 5432,
    database: "storage_hub",
    username: "postgres",
    password: "postgres"
  });
};

export const checkSHRunningContainers = async (docker: Docker) => {
  const allContainers = await docker.listContainers({ all: true });
  return allContainers.filter((container) => container.Image === DOCKER_IMAGE);
};

export const cleanupEnvironment = async (verbose = false) => {
  await printDockerStatus();

  const docker = new Docker();

  let allContainers = await docker.listContainers({ all: true });

  const existingNodes = allContainers.filter((container) => container.Image === DOCKER_IMAGE);

  const toxiproxyContainer = allContainers.find((container) =>
    container.Names.some((name) => name.includes("toxiproxy"))
  );

  const postgresContainer = allContainers.find((container) =>
    container.Names.some((name) => name.includes("storage-hub-sh-postgres-1"))
  );

  const copypartyContainers = allContainers.filter((container) =>
    container.Names.some((name) => name.includes("storage-hub-sh-copyparty"))
  );

  const backendContainer = allContainers.find((container) =>
    container.Names.some((name) => name.includes("storage-hub-sh-backend"))
  );

  const tmpDir = tmp.dirSync({ prefix: "bsp-logs-", unsafeCleanup: true });

  const logPromises = existingNodes.map(async (node) => {
    const container = docker.getContainer(node.Id);
    try {
      const logs = await container.logs({
        stdout: true,
        stderr: true,
        timestamps: true
      });
      verbose && console.log(`Extracting logs for container ${node.Names[0]}`);
      const containerName = node.Names[0].replace("/", "");

      await fs.writeFile(`${tmpDir.name}/${containerName}.log`, stripAnsi(logs.toString()), {
        encoding: "utf8"
      });
    } catch (e) {
      console.warn(`Failed to extract logs for container ${node.Names[0]}:`, e);
    }
  });

  await Promise.all(logPromises);
  console.log(`Container logs saved to ${tmpDir.name}`);

  const promises = existingNodes.map(async (node) => {
    const container = docker.getContainer(node.Id);
    await container.remove({ force: true });
  });

  if (toxiproxyContainer && toxiproxyContainer.State === "running") {
    console.log("Stopping toxiproxy container");
    promises.push(docker.getContainer(toxiproxyContainer.Id).stop());
  } else {
    verbose && console.log("No running toxiproxy container found, skipping");
  }

  if (postgresContainer) {
    console.log("Stopping postgres container");
    promises.push(docker.getContainer(postgresContainer.Id).remove({ force: true }));
  } else {
    verbose && console.log("No postgres container found, skipping");
  }

  if (copypartyContainers.length > 0) {
    console.log(`Stopping ${copypartyContainers.length} copyparty container(s)`);
    for (const container of copypartyContainers) {
      promises.push(docker.getContainer(container.Id).remove({ force: true }));
    }
  } else {
    verbose && console.log("No copyparty containers found, skipping");
  }

  if (backendContainer) {
    console.log("Stopping backend container");
    promises.push(docker.getContainer(backendContainer.Id).remove({ force: true }));
  } else {
    verbose && console.log("No backend container found, skipping");
  }

  await Promise.all(promises);

  await docker.pruneContainers();
  await docker.pruneVolumes();

  for (let i = 0; i < 10; i++) {
    allContainers = await docker.listContainers({ all: true });
    const remainingNodes = allContainers.filter((container) => container.Image === DOCKER_IMAGE);
    if (remainingNodes.length === 0) {
      await printDockerStatus();
      verbose && console.log("All nodes verified to be removed, continuing");
      return;
    }
  }
  assert(false, `Failed to stop all nodes: ${JSON.stringify(allContainers)}`);
};
