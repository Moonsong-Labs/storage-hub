import path from "node:path";
import { execSync } from "node:child_process";

export const checkBspForFile = async (filePath: string) => {
  const containerId = "docker-sh-bsp-1";
  const loc = path.join("/storage", filePath);

  for (let i = 0; i < 10; i++) {
    try {
      // TODO: Replace with dockerode
      execSync(`docker exec ${containerId} ls ${loc}`, { stdio: "ignore" });
      return;
    } catch {
      await new Promise((resolve) => setTimeout(resolve, 1000));
    }
  }
  throw new Error(`File not found: ${loc} in ${containerId}`);
};

export const checkFileChecksum = async (filePath: string) => {
  const containerId = "docker-sh-bsp-1";
  const loc = path.join("/storage", filePath);
  const output = execSync(`docker exec ${containerId} sha256sum ${loc}`);
  return output.toString().split(" ")[0];
};

export const showContainers = () => {
  try {
    // TODO: Replace with dockerode
    execSync("docker ps -a", { stdio: "inherit" });
  } catch (e) {
    console.log(e);
    console.log("Error displaying docker containers");
  }
};
