import { readFile } from "node:fs/promises";
import { validateFindingsRegister } from "./security-readiness-core.mjs";

const mode = process.argv[2] ?? "--require-cleared";
if (!['--allow-pending', '--require-cleared'].includes(mode)) throw new Error("use --allow-pending or --require-cleared");
const review = JSON.parse(await readFile("security/review-status.json", "utf8"));
const findings = JSON.parse(await readFile("security/findings.json", "utf8"));
const result = validateFindingsRegister(findings, review);
if (!result.cleared && mode === "--require-cleared") throw new Error(`security readiness blocked: ${result.reason}`);
process.stdout.write(result.cleared ? "security review and remediation are cleared\n" : `security readiness pending: ${result.reason}\n`);
