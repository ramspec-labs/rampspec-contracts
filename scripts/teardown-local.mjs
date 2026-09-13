import { execFileSync } from "node:child_process";
import { rm } from "node:fs/promises";
import { resolve } from "node:path";

const runtimeDirectory = resolve(process.argv[2] ?? "deployments/local/runtime");
const containerName = process.env.RAMPSPEC_LOCAL_CONTAINER ?? "rampspec-local";
execFileSync("stellar", ["container", "stop", containerName], { stdio: "inherit" });
await rm(runtimeDirectory, { recursive: true, force: true });
process.stdout.write("local RampSpec network state removed\n");
