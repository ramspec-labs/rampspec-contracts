import { resolve } from "node:path";
import { verifyReleaseBundle } from "./release-core.mjs";

if (!process.argv[2]) throw new Error("usage: node scripts/verify-release.mjs <bundle> [--require-release]");
const manifest = await verifyReleaseBundle(resolve(process.argv[2]));
if (process.argv.includes("--require-release") && manifest.status !== "released") throw new Error("candidate bundle is not a release");
process.stdout.write(`verified ${manifest.status} bundle ${manifest.version}\n`);
