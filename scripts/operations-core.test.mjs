import assert from "node:assert/strict";
import test from "node:test";
import { readFile } from "node:fs/promises";
import { validateOperations } from "./operations-core.mjs";

const schedule = JSON.parse(await readFile("operations/schedule.json", "utf8"));
const alerts = JSON.parse(await readFile("operations/alerts.json", "utf8"));
const compatibility = JSON.parse(await readFile("operations/compatibility-policy.json", "utf8"));
const dependencyExceptions = JSON.parse(await readFile("security/dependency-exceptions.json", "utf8"));
const now = Date.parse("2026-09-13T12:00:00Z");

test("accepts owned schedules and honest deployment blocks", () => {
  const result = validateOperations(schedule, alerts, compatibility, dependencyExceptions, now);
  assert.equal(result.scheduled.length, 5);
  assert.equal(result.blocked.length, 3);
  assert.equal(result.alerts, 7);
});

test("fails overdue work and missing backup ownership", () => {
  const overdue = structuredClone(schedule);
  overdue.tasks.find((task) => task.id === "compatibility-review").nextDueAt = "2026-09-01T00:00:00Z";
  assert.throws(() => validateOperations(overdue, alerts, compatibility, dependencyExceptions, now), /overdue/);
  const ownerless = structuredClone(schedule);
  ownerless.tasks[2].backup = ownerless.tasks[2].owner;
  assert.throws(() => validateOperations(ownerless, alerts, compatibility, dependencyExceptions, now), /distinct owner and backup/);
});

test("deployment-dependent tasks cannot masquerade as complete", () => {
  const falseClaim = structuredClone(schedule);
  const ttl = falseClaim.tasks.find((task) => task.id === "ttl-maintenance");
  ttl.status = "scheduled";
  ttl.nextDueAt = "2026-10-01T00:00:00Z";
  assert.throws(() => validateOperations(falseClaim, alerts, compatibility, dependencyExceptions, now), /explicitly blocked/);
  assert.throws(() => validateOperations({ ...schedule, deploymentState: "testnet" }, alerts, compatibility, dependencyExceptions, now), /manifest are inconsistent/);
});

test("rejects incomplete compatibility and alert policies", () => {
  assert.throws(() => validateOperations(schedule, { ...alerts, alerts: [] }, compatibility, dependencyExceptions, now), /alert inventory/);
  assert.throws(() => validateOperations(schedule, alerts, { ...compatibility, supportedStorageSchemas: [2] }, dependencyExceptions, now), /compatibility policy/);
});

test("rejects expired dependency exceptions", () => {
  const expired = structuredClone(dependencyExceptions);
  expired.exceptions[0].reviewBy = "2026-09-01T00:00:00Z";
  assert.throws(() => validateOperations(schedule, alerts, compatibility, expired, now), /exception.*expired/);
});
