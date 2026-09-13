export function recoveryDecision({ integrityValid, oldEvidenceReadable, newEvidenceWritable }) {
  if (!integrityValid) return "stop";
  if (!oldEvidenceReadable) return "rollback";
  if (!newEvidenceWritable) return "forward-fix";
  return "accept";
}

export function assertIdentityAlias(value, name) {
  if (!value || value.startsWith("S") || /\s/.test(value)) {
    throw new Error(`${name} must be a configured identity alias without secret material`);
  }
  return value;
}
