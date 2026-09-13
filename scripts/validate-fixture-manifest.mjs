import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { validateFixtureManifest } from "./fixture-manifest-core.mjs";

const manifestPath = resolve(process.argv[2] ?? "fixtures/manifest.json");
const manifest = JSON.parse(await readFile(manifestPath, "utf8"));
validateFixtureManifest(manifest);
process.stdout.write(`validated ${manifestPath}\n`);
