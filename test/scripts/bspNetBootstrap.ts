import { runBspNet } from "../util";

runBspNet()
  .then(() => {
    console.log("✅ BSPNet Bootstrap script completed successfully");
  })
  .catch((err) => {
    console.error("Error running bootstrap script:", err);
    console.log("❌ BSPNet Bootstrap script not completed successfully");
  });
