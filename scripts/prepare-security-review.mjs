import { createHash } from "node:crypto";
import { mkdir, readFile, stat, writeFile } from "node:fs/promises";
import { join, relative, resolve } from "node:path";
import { validateReviewStatus } from "./security-review-core.mjs";

const outputDirectory = resolve(process.argv[2] ?? "artifacts/security-review");
const evidence = JSON.parse(await readFile("security/review-evidence.json", "utf8"));
const status = validateReviewStatus(JSON.parse(await readFile("security/review-status.json", "utf8")));
if (evidence.schemaVersion !== 1 || !Array.isArray(evidence.requiredPaths) || evidence.requiredPaths.length === 0) {
  throw new Error("review evidence index is invalid");
}

async function filesUnder(path) {
  const metadata = await stat(path);
  if (metadata.isFile()) return [path];
  const { readdir } = await import("node:fs/promises");
  const entries = await readdir(path, { withFileTypes: true });
  const nested = await Promise.all(entries.sort((a, b) => a.name.localeCompare(b.name, "en"))
    .map((entry) => filesUnder(join(path, entry.name))));
  return nested.flat();
}

const paths = [];
for (const requiredPath of evidence.requiredPaths) paths.push(...await filesUnder(resolve(requiredPath)));
const unique = [...new Set(paths.map((path) => resolve(path)))].sort();
const inventory = [];
for (const path of unique) {
  const content = await readFile(path);
  inventory.push({
    path: relative(process.cwd(), path).replaceAll("\\", "/"),
    bytes: content.length,
    sha256: createHash("sha256").update(content).digest("hex"),
  });
}
await mkdir(outputDirectory, { recursive: true });
const bundle = {
  schemaVersion: 1,
  reviewStatus: status.status,
  productionClaimsAllowed: status.productionClaimsAllowed,
  pubnetAllowed: status.pubnetAllowed,
  files: inventory,
  generatedAt: new Date().toISOString(),
};
await writeFile(join(outputDirectory, "evidence-inventory.json"), `${JSON.stringify(bundle, null, 2)}\n`, "utf8");
process.stdout.write(`${inventory.length} review evidence files inventoried\n`);
