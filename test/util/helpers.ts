import assert from "node:assert";
import fs from "node:fs/promises";
import Docker from "dockerode";
import postgres from "postgres";
import stripAnsi from "strip-ansi";
import tmp from "tmp";
import { DOCKER_IMAGE } from ".";

export const printDockerStatus = async (verbose = false) => {
  const docker = new Docker();

  verbose && console.log("\n=== Docker Container Status ===");

  const containers = await docker.listContainers({ all: true });

  if (containers.length === 0) {
    verbose && console.log("No containers found");
    return;
  }

  // Enhanced diagnostics for CI environment
  if (process.env.CI === "true") {
    const testContainers = containers.filter(
      (container) =>
        container.Image === DOCKER_IMAGE ||
        container.Names.some((name) => name.includes("storage-hub-sh-")) ||
        container.Names.some((name) => name.includes("toxiproxy")) ||
        container.Names.some((name) => name.includes("postgres"))
    );

    console.log(`[DOCKER-DIAGNOSTICS] Total containers: ${containers.length}`);
    console.log(`[DOCKER-DIAGNOSTICS] Test-related containers: ${testContainers.length}`);

    // Breakdown by type
    const breakdown = {
      "storage-hub": containers.filter((c) => c.Image === DOCKER_IMAGE).length,
      postgres: containers.filter((c) => c.Names.some((n) => n.includes("postgres"))).length,
      fisherman: containers.filter((c) => c.Names.some((n) => n.includes("fisherman"))).length,
      msp: containers.filter((c) => c.Names.some((n) => n.includes("msp"))).length,
      bsp: containers.filter((c) => c.Names.some((n) => n.includes("sh-bsp"))).length,
      user: containers.filter((c) => c.Names.some((n) => n.includes("sh-user"))).length
    };
    console.log("[DOCKER-DIAGNOSTICS] Container breakdown:", breakdown);
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
        } catch (_e) {
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

// Global tracking of SQL clients for cleanup
// biome-ignore lint/complexity/noBannedTypes: Good enough until we integrate ORM
const activeSqlClients = new Set<postgres.Sql<{}>>();
let sqlClientCounter = 0;
// biome-ignore lint/complexity/noBannedTypes: Good enough until we integrate ORM
const clientIdMap = new WeakMap<postgres.Sql<{}>, number>();

export const createSqlClient = () => {
  const clientId = ++sqlClientCounter;
  console.log(
    `[SQL] Creating SQL client #${clientId}, active clients before: ${activeSqlClients.size}`
  );

  const client = postgres({
    host: "localhost",
    port: 5432,
    database: "storage_hub",
    username: "postgres",
    password: "postgres"
  });

  // Track the client for cleanup
  activeSqlClients.add(client);
  clientIdMap.set(client, clientId);

  // Warn if too many connections
  if (activeSqlClients.size > 5) {
    console.warn(
      `[SQL] WARNING: ${activeSqlClients.size} SQL connections are active! This may cause connection pool exhaustion.`
    );
  }

  // Override the end method to remove from tracking
  const originalEnd = client.end.bind(client);
  client.end = async () => {
    const id = clientIdMap.get(client) || "unknown";
    console.log(`[SQL] Closing SQL client #${id}`);
    activeSqlClients.delete(client);
    clientIdMap.delete(client);
    return originalEnd();
  };

  console.log(`[SQL] SQL client #${clientId} created, total active: ${activeSqlClients.size}`);
  return client;
};

export const closeAllSqlClients = async () => {
  const count = activeSqlClients.size;
  if (count === 0) {
    console.log("[SQL] No SQL clients to close");
    return;
  }

  console.log(`[SQL] Closing ${count} SQL client(s)...`);
  const clientsToClose = Array.from(activeSqlClients);

  await Promise.allSettled(
    clientsToClose.map(async (client) => {
      try {
        const id = clientIdMap.get(client) || "unknown";
        console.log(`[SQL] Attempting to close client #${id}`);
        await client.end();
      } catch (error) {
        console.warn("[SQL] Error closing SQL client:", error);
      }
    })
  );

  // Clear the set after attempting to close all
  activeSqlClients.clear();
  console.log("[SQL] All SQL clients closed");
};

export const checkSHRunningContainers = async (docker: Docker) => {
  const allContainers = await docker.listContainers({ all: true });
  return allContainers.filter((container) => container.Image === DOCKER_IMAGE);
};

export const cleanupEnvironment = async (verbose = false) => {
  console.log("[CLEANUP] Starting environment cleanup...");
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

  if (existingNodes.length > 0) {
    console.log(
      `[CLEANUP] Extracting logs from ${existingNodes.length} storage-hub container(s)...`
    );
  }

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

  if (toxiproxyContainer) {
    console.log("[CLEANUP] Removing toxiproxy container");
    promises.push(docker.getContainer(toxiproxyContainer.Id).remove({ force: true }));
  } else {
    verbose && console.log("[CLEANUP] No toxiproxy container found, skipping");
  }

  if (postgresContainer) {
    console.log("[CLEANUP] Stopping postgres container");
    promises.push(docker.getContainer(postgresContainer.Id).remove({ force: true }));
  } else {
    verbose && console.log("[CLEANUP] No postgres container found, skipping");
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

  console.log(`[CLEANUP] Removing ${promises.length} container(s)...`);

  // Use allSettled to handle individual container removal failures
  const results = await Promise.allSettled(promises);
  const failedRemovals = results.filter((r) => r.status === "rejected");

  if (failedRemovals.length > 0) {
    console.error(`[CLEANUP] ${failedRemovals.length} container removals failed`);
    failedRemovals.forEach((failure: any) => {
      console.error(`[CLEANUP] Removal error: ${failure.reason?.message || failure.reason}`);
    });
  }

  console.log("[CLEANUP] Pruning containers and volumes...");
  await docker.pruneContainers();
  await docker.pruneVolumes();

  console.log("[CLEANUP] Verifying all containers removed...");
  for (let i = 0; i < 10; i++) {
    allContainers = await docker.listContainers({ all: true });

    // Check for ALL our container types, not just DOCKER_IMAGE
    const remainingNodes = allContainers.filter((container) => {
      return (
        container.Image === DOCKER_IMAGE ||
        container.Names.some(
          (name) =>
            name.includes("toxiproxy") ||
            name.includes("sh-postgres") ||
            name.includes("sh-copyparty") ||
            name.includes("sh-backend") ||
            name.includes("sh_") // Any sh_ prefixed containers
        )
      );
    });

    if (remainingNodes.length === 0) {
      await printDockerStatus();
      console.log("[CLEANUP] All nodes verified to be removed");
      console.log("[CLEANUP] Environment cleanup complete");
      return;
    }

    if (i === 9) {
      console.error(
        `[CLEANUP] Failed after 10 attempts, ${remainingNodes.length} containers still present:`
      );
      remainingNodes.forEach((c) => {
        console.error(`  - ${c.Names.join(",")} (Image: ${c.Image}, State: ${c.State})`);
      });
    } else {
      // Wait a bit before next check
      await new Promise((resolve) => setTimeout(resolve, 500));
    }
  }
  assert(false, "Failed to stop all nodes after cleanup");
};

export const cleanupFishermanTestContainers = async () => {
  console.log("[FISHERMAN-CLEANUP] Starting comprehensive fisherman test cleanup...");
  const docker = new Docker();

  // Get all containers
  const allContainers = await docker.listContainers({ all: true });

  // Identify all test-related containers
  const testContainers = allContainers.filter((container) => {
    const nameMatch = container.Names.some(
      (name) =>
        name.includes("sh_bsp") ||
        name.includes("sh_user") ||
        name.includes("sh_msp") ||
        name.includes("sh_fisherman") ||
        name.includes("sh_postgres") ||
        name.includes("sh-postgres") ||
        name.includes("toxiproxy") ||
        name.includes("sh-copyparty") ||
        name.includes("sh-backend") ||
        name.includes("sh_") // Any sh_ prefixed containers
    );
    const imageMatch =
      container.Image === DOCKER_IMAGE ||
      container.Image.includes("toxiproxy") ||
      container.Image.includes("postgres");
    return nameMatch || imageMatch;
  });

  if (testContainers.length === 0) {
    console.log("[FISHERMAN-CLEANUP] No test containers found");
    return;
  }

  console.log(`[FISHERMAN-CLEANUP] Found ${testContainers.length} test containers to remove`);
  testContainers.forEach((c) => {
    console.log(`  - ${c.Names.join(",")} (Image: ${c.Image}, State: ${c.State})`);
  });

  // Process removals sequentially to avoid conflicts
  let successCount = 0;
  let failureCount = 0;

  for (const containerInfo of testContainers) {
    const container = docker.getContainer(containerInfo.Id);
    const containerName = containerInfo.Names.join(",");

    try {
      // First try to stop if running or paused
      if (containerInfo.State === "running" || containerInfo.State === "paused") {
        console.log(`[FISHERMAN-CLEANUP] Stopping ${containerName}...`);
        try {
          await container.stop({ t: 5 });
        } catch (stopError: any) {
          // Ignore stop errors, proceed to remove
          if (!stopError.message?.includes("not running")) {
            console.warn(
              `[FISHERMAN-CLEANUP] Stop warning for ${containerName}: ${stopError.message}`
            );
          }
        }
      }

      // Then remove the container
      console.log(`[FISHERMAN-CLEANUP] Removing ${containerName}...`);
      await container.remove({ force: true });
      successCount++;
    } catch (error: any) {
      failureCount++;
      console.error(`[FISHERMAN-CLEANUP] Failed to remove ${containerName}: ${error.message}`);
    }
  }

  console.log(`[FISHERMAN-CLEANUP] Removed ${successCount}/${testContainers.length} containers`);

  if (failureCount > 0) {
    console.error(`[FISHERMAN-CLEANUP] Failed to remove ${failureCount} containers`);
  }

  // Verify cleanup
  const remainingContainers = await docker.listContainers({ all: true });
  const stillPresent = remainingContainers.filter((container) => {
    const nameMatch = container.Names.some(
      (name) =>
        name.includes("sh_bsp") ||
        name.includes("sh_user") ||
        name.includes("sh_msp") ||
        name.includes("sh_fisherman") ||
        name.includes("sh_postgres") ||
        name.includes("sh-postgres") ||
        name.includes("toxiproxy")
    );
    const imageMatch = container.Image === DOCKER_IMAGE;
    return nameMatch || imageMatch;
  });

  if (stillPresent.length > 0) {
    console.error(
      `[FISHERMAN-CLEANUP] WARNING: ${stillPresent.length} containers still present after cleanup:`
    );
    stillPresent.forEach((c) => {
      console.error(`  - ${c.Names.join(",")} (State: ${c.State})`);
    });
    throw new Error(`Fisherman test cleanup incomplete: ${stillPresent.length} containers remain`);
  }
  console.log("[FISHERMAN-CLEANUP] Verified: All test containers successfully removed");
};
