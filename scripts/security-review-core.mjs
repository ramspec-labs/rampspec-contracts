const STATUS = new Set(["not-started", "in-progress", "completed"]);
const HASH = /^[0-9a-f]{64}$/;
const REVISION = /^[0-9a-f]{40}$/;

export function validateReviewStatus(status) {
  const fields = [
    "schemaVersion", "status", "reviewTargetRevision", "provider", "reportPath",
    "reportSha256", "startedAt", "completedAt", "productionClaimsAllowed",
    "pubnetAllowed", "updatedAt",
  ];
  if (!status || JSON.stringify(Object.keys(status).sort()) !== JSON.stringify(fields.sort())) {
    throw new Error("security review status fields do not match schema");
  }
  if (status.schemaVersion !== 1 || !STATUS.has(status.status)) throw new Error("invalid security review status");
  if (!Number.isFinite(Date.parse(status.updatedAt))) throw new Error("review update timestamp is invalid");
  if (status.status !== "completed") {
    if (status.productionClaimsAllowed || status.pubnetAllowed) throw new Error("pending review cannot allow production or pubnet");
    if (status.completedAt !== null) throw new Error("pending review cannot have a completion time");
  }
  if (status.status === "not-started") {
    for (const field of ["reviewTargetRevision", "provider", "reportPath", "reportSha256", "startedAt"]) {
      if (status[field] !== null) throw new Error(`not-started review must not set ${field}`);
    }
  }
  if (status.status === "in-progress") {
    if (!REVISION.test(status.reviewTargetRevision ?? "") || !status.provider || !Number.isFinite(Date.parse(status.startedAt))) {
      throw new Error("in-progress review requires a revision, provider, and start time");
    }
    if (status.reportPath !== null || status.reportSha256 !== null) throw new Error("in-progress review cannot claim a final report");
  }
  if (status.status === "completed") {
    if (!REVISION.test(status.reviewTargetRevision ?? "") || !status.provider || !status.reportPath || !HASH.test(status.reportSha256 ?? "")) {
      throw new Error("completed review requires revision-bound provider and report evidence");
    }
    if (!Number.isFinite(Date.parse(status.startedAt)) || !Number.isFinite(Date.parse(status.completedAt))) {
      throw new Error("completed review requires valid timestamps");
    }
    if (status.productionClaimsAllowed || status.pubnetAllowed) {
      throw new Error("review completion alone cannot authorize production or pubnet before remediation");
    }
  }
  return status;
}
