import { Buffer } from "buffer";
import { Address } from "@stellar/stellar-sdk";
import {
  AssembledTransaction,
  Client as ContractClient,
  ClientOptions as ContractClientOptions,
  MethodOptions,
  Result,
  Spec as ContractSpec,
} from "@stellar/stellar-sdk/contract";
import type {
  u32,
  i32,
  u64,
  i64,
  u128,
  i128,
  u256,
  i256,
  Option,
  Timepoint,
  Duration,
} from "@stellar/stellar-sdk/contract";
export * from "@stellar/stellar-sdk";
export * as contract from "@stellar/stellar-sdk/contract";
export * as rpc from "@stellar/stellar-sdk/rpc";

if (typeof window !== "undefined") {
  //@ts-ignore Buffer exists
  window.Buffer = window.Buffer || Buffer;
}




export type FixtureMode = {tag: "Accept", values: void} | {tag: "Reject", values: void};

export const WebAuthError = {
  1: {message:"MissingArgument"},
  2: {message:"MalformedArgument"},
  3: {message:"NotInitialized"},
  4: {message:"AlreadyInitialized"},
  5: {message:"InvalidTimeWindow"},
  6: {message:"NotYetValid"},
  7: {message:"Expired"},
  8: {message:"Replay"},
  9: {message:"Rejected"},
  10: {message:"UnknownNonce"}
}


export interface FixtureConfig {
  admin: string;
  expires_at_ledger: u32;
  mode: FixtureMode;
  valid_from_ledger: u32;
}




export type NetworkKind = {tag: "Testnet", values: void} | {tag: "Futurenet", values: void} | {tag: "Pubnet", values: void} | {tag: "Standalone", values: void};

export const ContractError = {
  1: {message:"AlreadyInitialized"},
  2: {message:"NotInitialized"},
  3: {message:"Unauthorized"},
  4: {message:"Paused"},
  5: {message:"InvalidHash"},
  6: {message:"InvalidCounts"},
  7: {message:"InvalidScore"},
  8: {message:"AttestorNotRegistered"},
  9: {message:"AttestorDisabled"},
  10: {message:"EvidenceAlreadyExists"},
  11: {message:"EvidenceNotFound"},
  12: {message:"EvidenceNotActive"},
  13: {message:"InvalidSupersession"},
  14: {message:"AdminProposalMissing"},
  15: {message:"UpgradeNotAllowed"},
  16: {message:"AdminProposalExpired"}
}


export interface EvidenceInput {
  artifact_root: Buffer;
  failed: u32;
  network: NetworkKind;
  passed: u32;
  protocol_bitmap: u64;
  publisher: string;
  report_hash: Buffer;
  score_bps: u32;
  skipped: u32;
  specs_hash: Buffer;
  suite_hash: Buffer;
  target_hash: Buffer;
  warnings: u32;
}

export type InstanceKeyV1 = {tag: "Admin", values: void} | {tag: "PendingAdmin", values: void} | {tag: "PendingAdminDeadline", values: void} | {tag: "Paused", values: void} | {tag: "SchemaVersion", values: void} | {tag: "CurrentWasmHash", values: void};


export interface AttestorRecord {
  attestor: string;
  enabled: boolean;
  metadata_hash: Buffer;
  registered_ledger: u32;
  updated_ledger: u32;
}


export interface EvidenceRecord {
  artifact_root: Buffer;
  created_ledger: u32;
  failed: u32;
  id: Buffer;
  network: NetworkKind;
  passed: u32;
  protocol_bitmap: u64;
  publisher: string;
  report_hash: Buffer;
  score_bps: u32;
  skipped: u32;
  specs_hash: Buffer;
  status: EvidenceStatus;
  suite_hash: Buffer;
  supersedes: Option<Buffer>;
  target_hash: Buffer;
  warnings: u32;
}

export type EvidenceStatus = {tag: "Active", values: void} | {tag: "Superseded", values: void} | {tag: "Revoked", values: void};

export type PersistentKeyV1 = {tag: "Attestor", values: readonly [string]} | {tag: "Evidence", values: readonly [Buffer]} | {tag: "ActiveReport", values: readonly [string, Buffer, NetworkKind]} | {tag: "SupersededBy", values: readonly [Buffer]};

export interface Client {
  /**
   * Construct and simulate a config transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  config: (options?: MethodOptions) => Promise<AssembledTransaction<Result<FixtureConfig>>>

  /**
   * Construct and simulate a configure transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  configure: ({mode, valid_from_ledger, expires_at_ledger}: {mode: FixtureMode, valid_from_ledger: u32, expires_at_ledger: u32}, options?: MethodOptions) => Promise<AssembledTransaction<Result<void>>>

  /**
   * Construct and simulate a initialize transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  initialize: ({admin, mode, valid_from_ledger, expires_at_ledger}: {admin: string, mode: FixtureMode, valid_from_ledger: u32, expires_at_ledger: u32}, options?: MethodOptions) => Promise<AssembledTransaction<Result<void>>>

  /**
   * Construct and simulate a reset_nonce transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  reset_nonce: ({nonce}: {nonce: string}, options?: MethodOptions) => Promise<AssembledTransaction<Result<void>>>

  /**
   * Construct and simulate a sep_version transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  sep_version: (options?: MethodOptions) => Promise<AssembledTransaction<string>>

  /**
   * Construct and simulate a is_test_only transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  is_test_only: (options?: MethodOptions) => Promise<AssembledTransaction<boolean>>

  /**
   * Construct and simulate a schema_version transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  schema_version: (options?: MethodOptions) => Promise<AssembledTransaction<u32>>

  /**
   * Construct and simulate a web_auth_verify transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  web_auth_verify: ({args}: {args: Map<string, string>}, options?: MethodOptions) => Promise<AssembledTransaction<Result<void>>>

  /**
   * Construct and simulate a is_nonce_consumed transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  is_nonce_consumed: ({nonce}: {nonce: string}, options?: MethodOptions) => Promise<AssembledTransaction<boolean>>

}
export class Client extends ContractClient {
  static async deploy<T = Client>(
    /** Options for initializing a Client as well as for calling a method, with extras specific to deploying. */
    options: MethodOptions &
      Omit<ContractClientOptions, "contractId"> & {
        /** The hash of the Wasm blob, which must already be installed on-chain. */
        wasmHash: Buffer | string;
        /** Salt used to generate the contract's ID. Passed through to {@link Operation.createCustomContract}. Default: random. */
        salt?: Buffer | Uint8Array;
        /** The format used to decode `wasmHash`, if it's provided as a string. */
        format?: "hex" | "base64";
      }
  ): Promise<AssembledTransaction<T>> {
    return ContractClient.deploy(null, options)
  }
  constructor(public readonly options: ContractClientOptions) {
    super(
      new ContractSpec([ "AAAAAgAAAAAAAAAAAAAAC0ZpeHR1cmVNb2RlAAAAAAIAAAAAAAAAAAAAAAZBY2NlcHQAAAAAAAAAAAAAAAAABlJlamVjdAAA",
        "AAAABAAAAAAAAAAAAAAADFdlYkF1dGhFcnJvcgAAAAoAAAAAAAAAD01pc3NpbmdBcmd1bWVudAAAAAABAAAAAAAAABFNYWxmb3JtZWRBcmd1bWVudAAAAAAAAAIAAAAAAAAADk5vdEluaXRpYWxpemVkAAAAAAADAAAAAAAAABJBbHJlYWR5SW5pdGlhbGl6ZWQAAAAAAAQAAAAAAAAAEUludmFsaWRUaW1lV2luZG93AAAAAAAABQAAAAAAAAALTm90WWV0VmFsaWQAAAAABgAAAAAAAAAHRXhwaXJlZAAAAAAHAAAAAAAAAAZSZXBsYXkAAAAAAAgAAAAAAAAACFJlamVjdGVkAAAACQAAAAAAAAAMVW5rbm93bk5vbmNlAAAACg==",
        "AAAAAQAAAAAAAAAAAAAADUZpeHR1cmVDb25maWcAAAAAAAAEAAAAAAAAAAVhZG1pbgAAAAAAABMAAAAAAAAAEWV4cGlyZXNfYXRfbGVkZ2VyAAAAAAAABAAAAAAAAAAEbW9kZQAAB9AAAAALRml4dHVyZU1vZGUAAAAAAAAAABF2YWxpZF9mcm9tX2xlZGdlcgAAAAAAAAQ=",
        "AAAAAAAAAAAAAAAGY29uZmlnAAAAAAAAAAAAAQAAA+kAAAfQAAAADUZpeHR1cmVDb25maWcAAAAAAAfQAAAADFdlYkF1dGhFcnJvcg==",
        "AAAAAAAAAAAAAAAJY29uZmlndXJlAAAAAAAAAwAAAAAAAAAEbW9kZQAAB9AAAAALRml4dHVyZU1vZGUAAAAAAAAAABF2YWxpZF9mcm9tX2xlZGdlcgAAAAAAAAQAAAAAAAAAEWV4cGlyZXNfYXRfbGVkZ2VyAAAAAAAABAAAAAEAAAPpAAAAAgAAB9AAAAAMV2ViQXV0aEVycm9y",
        "AAAABQAAAAAAAAAAAAAAEUZpeHR1cmVDb25maWd1cmVkAAAAAAAAAQAAABJmaXh0dXJlX2NvbmZpZ3VyZWQAAAAAAAQAAAAAAAAABWFkbWluAAAAAAAAEwAAAAEAAAAAAAAABG1vZGUAAAfQAAAAC0ZpeHR1cmVNb2RlAAAAAAAAAAAAAAAAEXZhbGlkX2Zyb21fbGVkZ2VyAAAAAAAABAAAAAAAAAAAAAAAEWV4cGlyZXNfYXRfbGVkZ2VyAAAAAAAABAAAAAAAAAAC",
        "AAAABQAAAAAAAAAAAAAAEUZpeHR1cmVOb25jZVJlc2V0AAAAAAAAAQAAABNmaXh0dXJlX25vbmNlX3Jlc2V0AAAAAAIAAAAAAAAABWFkbWluAAAAAAAAEwAAAAEAAAAAAAAACm5vbmNlX2hhc2gAAAAAA+4AAAAgAAAAAQAAAAI=",
        "AAAAAAAAAAAAAAAKaW5pdGlhbGl6ZQAAAAAABAAAAAAAAAAFYWRtaW4AAAAAAAATAAAAAAAAAARtb2RlAAAH0AAAAAtGaXh0dXJlTW9kZQAAAAAAAAAAEXZhbGlkX2Zyb21fbGVkZ2VyAAAAAAAABAAAAAAAAAARZXhwaXJlc19hdF9sZWRnZXIAAAAAAAAEAAAAAQAAA+kAAAACAAAH0AAAAAxXZWJBdXRoRXJyb3I=",
        "AAAAAAAAAAAAAAALcmVzZXRfbm9uY2UAAAAAAQAAAAAAAAAFbm9uY2UAAAAAAAAQAAAAAQAAA+kAAAACAAAH0AAAAAxXZWJBdXRoRXJyb3I=",
        "AAAAAAAAAAAAAAALc2VwX3ZlcnNpb24AAAAAAAAAAAEAAAAQ",
        "AAAAAAAAAAAAAAAMaXNfdGVzdF9vbmx5AAAAAAAAAAEAAAAB",
        "AAAAAAAAAAAAAAAOc2NoZW1hX3ZlcnNpb24AAAAAAAAAAAABAAAABA==",
        "AAAABQAAAAAAAAAAAAAAFkF1dGhlbnRpY2F0aW9uVmVyaWZpZWQAAAAAAAEAAAAXYXV0aGVudGljYXRpb25fdmVyaWZpZWQAAAAAAgAAAAAAAAAHYWNjb3VudAAAAAATAAAAAQAAAAAAAAAKbm9uY2VfaGFzaAAAAAAD7gAAACAAAAABAAAAAg==",
        "AAAAAAAAAAAAAAAPd2ViX2F1dGhfdmVyaWZ5AAAAAAEAAAAAAAAABGFyZ3MAAAPsAAAAEQAAABAAAAABAAAD6QAAAAIAAAfQAAAADFdlYkF1dGhFcnJvcg==",
        "AAAAAAAAAAAAAAARaXNfbm9uY2VfY29uc3VtZWQAAAAAAAABAAAAAAAAAAVub25jZQAAAAAAABAAAAABAAAAAQ==",
        "AAAAAgAAAAAAAAAAAAAAC05ldHdvcmtLaW5kAAAAAAQAAAAAAAAAAAAAAAdUZXN0bmV0AAAAAAAAAAAAAAAACUZ1dHVyZW5ldAAAAAAAAAAAAAAAAAAABlB1Ym5ldAAAAAAAAAAAAAAAAAAKU3RhbmRhbG9uZQAA",
        "AAAABAAAAAAAAAAAAAAADUNvbnRyYWN0RXJyb3IAAAAAAAAQAAAAAAAAABJBbHJlYWR5SW5pdGlhbGl6ZWQAAAAAAAEAAAAAAAAADk5vdEluaXRpYWxpemVkAAAAAAACAAAAAAAAAAxVbmF1dGhvcml6ZWQAAAADAAAAAAAAAAZQYXVzZWQAAAAAAAQAAAAAAAAAC0ludmFsaWRIYXNoAAAAAAUAAAAAAAAADUludmFsaWRDb3VudHMAAAAAAAAGAAAAAAAAAAxJbnZhbGlkU2NvcmUAAAAHAAAAAAAAABVBdHRlc3Rvck5vdFJlZ2lzdGVyZWQAAAAAAAAIAAAAAAAAABBBdHRlc3RvckRpc2FibGVkAAAACQAAAAAAAAAVRXZpZGVuY2VBbHJlYWR5RXhpc3RzAAAAAAAACgAAAAAAAAAQRXZpZGVuY2VOb3RGb3VuZAAAAAsAAAAAAAAAEUV2aWRlbmNlTm90QWN0aXZlAAAAAAAADAAAAAAAAAATSW52YWxpZFN1cGVyc2Vzc2lvbgAAAAANAAAAAAAAABRBZG1pblByb3Bvc2FsTWlzc2luZwAAAA4AAAAAAAAAEVVwZ3JhZGVOb3RBbGxvd2VkAAAAAAAADwAAAAAAAAAUQWRtaW5Qcm9wb3NhbEV4cGlyZWQAAAAQ",
        "AAAAAQAAAAAAAAAAAAAADUV2aWRlbmNlSW5wdXQAAAAAAAANAAAAAAAAAA1hcnRpZmFjdF9yb290AAAAAAAD7gAAACAAAAAAAAAABmZhaWxlZAAAAAAABAAAAAAAAAAHbmV0d29yawAAAAfQAAAAC05ldHdvcmtLaW5kAAAAAAAAAAAGcGFzc2VkAAAAAAAEAAAAAAAAAA9wcm90b2NvbF9iaXRtYXAAAAAABgAAAAAAAAAJcHVibGlzaGVyAAAAAAAAEwAAAAAAAAALcmVwb3J0X2hhc2gAAAAD7gAAACAAAAAAAAAACXNjb3JlX2JwcwAAAAAAAAQAAAAAAAAAB3NraXBwZWQAAAAABAAAAAAAAAAKc3BlY3NfaGFzaAAAAAAD7gAAACAAAAAAAAAACnN1aXRlX2hhc2gAAAAAA+4AAAAgAAAAAAAAAAt0YXJnZXRfaGFzaAAAAAPuAAAAIAAAAAAAAAAId2FybmluZ3MAAAAE",
        "AAAAAgAAAAAAAAAAAAAADUluc3RhbmNlS2V5VjEAAAAAAAAGAAAAAAAAAAAAAAAFQWRtaW4AAAAAAAAAAAAAAAAAAAxQZW5kaW5nQWRtaW4AAAAAAAAAAAAAABRQZW5kaW5nQWRtaW5EZWFkbGluZQAAAAAAAAAAAAAABlBhdXNlZAAAAAAAAAAAAAAAAAANU2NoZW1hVmVyc2lvbgAAAAAAAAAAAAAAAAAAD0N1cnJlbnRXYXNtSGFzaAA=",
        "AAAAAQAAAAAAAAAAAAAADkF0dGVzdG9yUmVjb3JkAAAAAAAFAAAAAAAAAAhhdHRlc3RvcgAAABMAAAAAAAAAB2VuYWJsZWQAAAAAAQAAAAAAAAANbWV0YWRhdGFfaGFzaAAAAAAAA+4AAAAgAAAAAAAAABFyZWdpc3RlcmVkX2xlZGdlcgAAAAAAAAQAAAAAAAAADnVwZGF0ZWRfbGVkZ2VyAAAAAAAE",
        "AAAAAQAAAAAAAAAAAAAADkV2aWRlbmNlUmVjb3JkAAAAAAARAAAAAAAAAA1hcnRpZmFjdF9yb290AAAAAAAD7gAAACAAAAAAAAAADmNyZWF0ZWRfbGVkZ2VyAAAAAAAEAAAAAAAAAAZmYWlsZWQAAAAAAAQAAAAAAAAAAmlkAAAAAAPuAAAAIAAAAAAAAAAHbmV0d29yawAAAAfQAAAAC05ldHdvcmtLaW5kAAAAAAAAAAAGcGFzc2VkAAAAAAAEAAAAAAAAAA9wcm90b2NvbF9iaXRtYXAAAAAABgAAAAAAAAAJcHVibGlzaGVyAAAAAAAAEwAAAAAAAAALcmVwb3J0X2hhc2gAAAAD7gAAACAAAAAAAAAACXNjb3JlX2JwcwAAAAAAAAQAAAAAAAAAB3NraXBwZWQAAAAABAAAAAAAAAAKc3BlY3NfaGFzaAAAAAAD7gAAACAAAAAAAAAABnN0YXR1cwAAAAAH0AAAAA5FdmlkZW5jZVN0YXR1cwAAAAAAAAAAAApzdWl0ZV9oYXNoAAAAAAPuAAAAIAAAAAAAAAAKc3VwZXJzZWRlcwAAAAAD6AAAA+4AAAAgAAAAAAAAAAt0YXJnZXRfaGFzaAAAAAPuAAAAIAAAAAAAAAAId2FybmluZ3MAAAAE",
        "AAAAAgAAAAAAAAAAAAAADkV2aWRlbmNlU3RhdHVzAAAAAAADAAAAAAAAAAAAAAAGQWN0aXZlAAAAAAAAAAAAAAAAAApTdXBlcnNlZGVkAAAAAAAAAAAAAAAAAAdSZXZva2VkAA==",
        "AAAAAgAAAAAAAAAAAAAAD1BlcnNpc3RlbnRLZXlWMQAAAAAEAAAAAQAAAAAAAAAIQXR0ZXN0b3IAAAABAAAAEwAAAAEAAAAAAAAACEV2aWRlbmNlAAAAAQAAA+4AAAAgAAAAAQAAAAAAAAAMQWN0aXZlUmVwb3J0AAAAAwAAABMAAAPuAAAAIAAAB9AAAAALTmV0d29ya0tpbmQAAAAAAQAAAAAAAAAMU3VwZXJzZWRlZEJ5AAAAAQAAA+4AAAAg" ]),
      options
    )
  }
  public readonly fromJSON = {
    config: this.txFromJSON<Result<FixtureConfig>>,
        configure: this.txFromJSON<Result<void>>,
        initialize: this.txFromJSON<Result<void>>,
        reset_nonce: this.txFromJSON<Result<void>>,
        sep_version: this.txFromJSON<string>,
        is_test_only: this.txFromJSON<boolean>,
        schema_version: this.txFromJSON<u32>,
        web_auth_verify: this.txFromJSON<Result<void>>,
        is_nonce_consumed: this.txFromJSON<boolean>
  }
}

const WEB_AUTH_ERROR_NAMES: Record<number, string> = {
  1: "MissingArgument",
  2: "MalformedArgument",
  3: "NotInitialized",
  4: "AlreadyInitialized",
  5: "InvalidTimeWindow",
  6: "NotYetValid",
  7: "Expired",
  8: "Replay",
  9: "Rejected",
  10: "UnknownNonce",
};

export const FIXTURE_CONFIG_GENERATION_METHOD = "config_generation";

/**
 * Decode a web-auth fixture failure. Malformed and unknown payloads stay failures.
 * This never reports success.
 */
export function decodeWebAuthFailure(payload: unknown): { ok: false; code: number | null; name: string } {
  const code = readErrorCode(payload);
  if (code == null || WEB_AUTH_ERROR_NAMES[code] == null) {
    return { ok: false, code, name: "Unknown" };
  }
  return { ok: false, code, name: WEB_AUTH_ERROR_NAMES[code] };
}

/**
 * A verification result is successful only for `{ ok: true }` with no error fields.
 * Unknown codes, malformed bodies, and mixed success/error objects are failures.
 */
export function interpretWebAuthResult(payload: unknown): { ok: true } | { ok: false; code: number | null; name: string } {
  if (!isCleanSuccess(payload)) {
    return decodeWebAuthFailure(payload);
  }
  return { ok: true };
}

function isCleanSuccess(payload: unknown): boolean {
  if (!payload || typeof payload !== "object" || Array.isArray(payload)) return false;
  const record = payload as Record<string, unknown>;
  if (record.ok !== true) return false;
  if ("error" in record || "code" in record || "name" in record) return false;
  return true;
}

function readErrorCode(payload: unknown): number | null {
  if (typeof payload === "number" && Number.isInteger(payload)) return payload;
  if (!payload || typeof payload !== "object") return null;
  const record = payload as Record<string, unknown>;
  for (const key of ["code", "error"]) {
    const value = record[key];
    if (typeof value === "number" && Number.isInteger(value)) return value;
    if (value && typeof value === "object" && "code" in value) {
      const nested = (value as Record<string, unknown>).code;
      if (typeof nested === "number" && Number.isInteger(nested)) return nested;
    }
  }
  return null;
}