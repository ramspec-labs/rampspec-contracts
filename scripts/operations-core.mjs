const REQUIRED_TASKS = new Set([
  "ttl-maintenance", "event-index-integrity", "privileged-access-review", "cost-and-budget-review",
  "dependency-and-toolchain-review", "incident-recovery-drill", "compatibility-review", "pubnet-decision-review",
]);
const TASK_FIELDS = [
  "id", "owner", "backup", "frequencyDays", "trigger", "requiresDeployment", "status",
  "lastCompletedAt", "nextDueAt", "evidence",
];

export function validateOperations(schedule, alerts, compatibility, dependencyExceptions, now = Date.now()) {
  const scheduleFields = ["schemaVersion", "deploymentState", "deploymentManifest", "timezone", "tasks"];
  if (!schedule || JSON.stringify(Object.keys(schedule).sort()) !== JSON.stringify(scheduleFields.sort())) {
    throw new Error("operations schedule fields do not match schema");
  }
  if (schedule?.schemaVersion !== 1 || !['none', 'local', 'testnet', 'pubnet'].includes(schedule.deploymentState) || schedule.timezone !== "UTC") {
    throw new Error("operations schedule header is invalid");
  }
  if ((schedule.deploymentState === "none") !== (schedule.deploymentManifest === null)) {
    throw new Error("operations deployment state and manifest are inconsistent");
  }
  if (!Array.isArray(schedule.tasks)) throw new Error("operations tasks must be an array");
  const ids = new Set();
  const due = [];
  const blocked = [];
  for (const task of schedule.tasks) {
    if (!task || JSON.stringify(Object.keys(task).sort()) !== JSON.stringify([...TASK_FIELDS].sort())) throw new Error("operations task fields do not match schema");
    if (!REQUIRED_TASKS.has(task.id) || ids.has(task.id)) throw new Error("operations task IDs are missing, unknown, or duplicated");
    ids.add(task.id);
    if (!task.owner || !task.backup || task.owner === task.backup) throw new Error(`${task.id} requires distinct owner and backup`);
    if (!Number.isInteger(task.frequencyDays) || task.frequencyDays < 1 || typeof task.requiresDeployment !== "boolean" || !task.trigger) {
      throw new Error(`${task.id} schedule is invalid`);
    }
    if (task.requiresDeployment && schedule.deploymentState === "none") {
      if (task.status !== "blocked-no-deployment" || task.nextDueAt !== null || task.evidence !== null) throw new Error(`${task.id} must be explicitly blocked without a deployment`);
      blocked.push(task.id);
      continue;
    }
    if (task.status !== "scheduled" || !Number.isFinite(Date.parse(task.nextDueAt))) throw new Error(`${task.id} must have an active due date`);
    if (Date.parse(task.nextDueAt) <= now) throw new Error(`${task.id} is overdue`);
    if (task.lastCompletedAt !== null && !Number.isFinite(Date.parse(task.lastCompletedAt))) throw new Error(`${task.id} completion timestamp is invalid`);
    if (task.lastCompletedAt !== null) {
      if (!task.evidence) throw new Error(`${task.id} completed work requires evidence`);
      const interval = Date.parse(task.nextDueAt) - Date.parse(task.lastCompletedAt);
      if (interval !== task.frequencyDays * 86_400_000) throw new Error(`${task.id} next due date does not match its frequency`);
    }
    due.push({ id: task.id, nextDueAt: task.nextDueAt });
  }
  if (ids.size !== REQUIRED_TASKS.size) throw new Error("operations task inventory is incomplete");

  if (alerts?.schemaVersion !== 1 || alerts.delivery !== "github-actions-failure" || !Array.isArray(alerts.alerts) || alerts.alerts.length < 7) {
    throw new Error("operations alert inventory is invalid");
  }
  const alertIds = new Set();
  for (const alert of alerts.alerts) {
    const fields = ["id", "signal", "severity", "owner", "responseMinutes", "runbook"];
    if (!alert || JSON.stringify(Object.keys(alert).sort()) !== JSON.stringify(fields.sort())) throw new Error("operations alert fields do not match schema");
    if (!alert.id || alertIds.has(alert.id) || !['critical', 'warning'].includes(alert.severity) || !alert.signal || !alert.owner
      || !Number.isInteger(alert.responseMinutes) || alert.responseMinutes < 1 || !/^docs\/operations\.md#/.test(alert.runbook)) {
      throw new Error("operations alert is malformed or duplicated");
    }
    alertIds.add(alert.id);
  }
  if (compatibility?.schemaVersion !== 1 || JSON.stringify(compatibility.supportedStorageSchemas) !== "[1]"
    || JSON.stringify(compatibility.supportedManifestSchemas) !== "[1]" || compatibility.fixturePolicy !== "test-only-no-pubnet"
    || compatibility.upgradePolicy !== "additive-only-with-rehearsal" || compatibility.breakingChangeRequirements.length < 6) {
    throw new Error("compatibility policy is incomplete");
  }
  if (dependencyExceptions?.schemaVersion !== 1 || !Array.isArray(dependencyExceptions.exceptions)) {
    throw new Error("dependency exception inventory is invalid");
  }
  const advisories = new Set();
  const exceptionFields = [
    "ecosystem", "advisory", "package", "version", "classification", "dependencyPath",
    "rationale", "owner", "acceptedAt", "reviewBy",
  ];
  for (const exception of dependencyExceptions.exceptions) {
    if (!exception || JSON.stringify(Object.keys(exception).sort()) !== JSON.stringify([...exceptionFields].sort())
      || exception.ecosystem !== "cargo" || !/^RUSTSEC-[0-9]{4}-[0-9]{4}$/.test(exception.advisory)
      || advisories.has(exception.advisory) || exception.classification !== "unmaintained-not-vulnerable"
      || !exception.package || !exception.version || !exception.dependencyPath || !exception.rationale || !exception.owner
      || !Number.isFinite(Date.parse(exception.acceptedAt)) || Date.parse(exception.reviewBy) <= now) {
      throw new Error("dependency exception is malformed, duplicated, or expired");
    }
    advisories.add(exception.advisory);
  }
  return { deploymentState: schedule.deploymentState, scheduled: due, blocked, alerts: alertIds.size, dependencyExceptions: [...advisories] };
}
