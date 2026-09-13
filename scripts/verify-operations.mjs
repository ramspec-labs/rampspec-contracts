import { mkdir, readFile, writeFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { validateOperations } from "./operations-core.mjs";

const schedule = JSON.parse(await readFile("operations/schedule.json", "utf8"));
const alerts = JSON.parse(await readFile("operations/alerts.json", "utf8"));
const compatibility = JSON.parse(await readFile("operations/compatibility-policy.json", "utf8"));
const dependencyExceptions = JSON.parse(await readFile("security/dependency-exceptions.json", "utf8"));
const auditConfig = await readFile(".cargo/audit.toml", "utf8");
for (const exception of dependencyExceptions.exceptions) {
  if (!auditConfig.includes(`"${exception.advisory}"`)) throw new Error(`${exception.advisory} is not present in Cargo audit configuration`);
}
const result = validateOperations(schedule, alerts, compatibility, dependencyExceptions);
const reportIndex = process.argv.indexOf("--report");
if (reportIndex !== -1) {
  const path = resolve(process.argv[reportIndex + 1] ?? "");
  if (!process.argv[reportIndex + 1]) throw new Error("--report requires a path");
  await mkdir(dirname(path), { recursive: true });
  await writeFile(path, `${JSON.stringify({ schemaVersion: 1, checkedAt: new Date().toISOString(), ...result }, null, 2)}\n`, "utf8");
}
process.stdout.write(`${result.scheduled.length} operational tasks scheduled; ${result.blocked.length} await a deployment\n`);
