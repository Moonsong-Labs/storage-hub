import { runCoreDemo } from "./core-demo.js";
import { runMspDemo } from "./msp-demo.js";

async function main() {
  console.log("--- Running core demo ---");
  await runCoreDemo();
  console.log("--- Running msp demo ---");
  await runMspDemo();
}

await main().catch((err) => {
  console.error("Example failed:", err);
  process.exit(1);
});
