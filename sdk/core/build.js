#!/usr/bin/env node
import { runBuild } from "../scripts/build-common.js";

const watch = process.argv.includes("--watch");
await runBuild({ isCorePackage: true, watch });
