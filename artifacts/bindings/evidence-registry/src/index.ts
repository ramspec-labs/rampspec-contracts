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
  15: {message:"UpgradeNotAllowed"}
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

export type InstanceKeyV1 = {tag: "Admin", values: void} | {tag: "PendingAdmin", values: void} | {tag: "Paused", values: void} | {tag: "SchemaVersion", values: void} | {tag: "CurrentWasmHash", values: void};


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
   * Construct and simulate a upgrade transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  upgrade: ({new_wasm_hash}: {new_wasm_hash: Buffer}, options?: MethodOptions) => Promise<AssembledTransaction<Result<void>>>

  /**
   * Construct and simulate a is_active transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  is_active: ({id}: {id: Buffer}, options?: MethodOptions) => Promise<AssembledTransaction<boolean>>

  /**
   * Construct and simulate a initialize transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  initialize: ({admin, schema_version}: {admin: string, schema_version: u32}, options?: MethodOptions) => Promise<AssembledTransaction<Result<void>>>

  /**
   * Construct and simulate a set_paused transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  set_paused: ({paused}: {paused: boolean}, options?: MethodOptions) => Promise<AssembledTransaction<Result<void>>>

  /**
   * Construct and simulate a accept_admin transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  accept_admin: (options?: MethodOptions) => Promise<AssembledTransaction<Result<void>>>

  /**
   * Construct and simulate a get_attestor transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  get_attestor: ({attestor}: {attestor: string}, options?: MethodOptions) => Promise<AssembledTransaction<Option<AttestorRecord>>>

  /**
   * Construct and simulate a get_evidence transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  get_evidence: ({id}: {id: Buffer}, options?: MethodOptions) => Promise<AssembledTransaction<Option<EvidenceRecord>>>

  /**
   * Construct and simulate a propose_admin transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  propose_admin: ({new_admin}: {new_admin: string}, options?: MethodOptions) => Promise<AssembledTransaction<Result<void>>>

  /**
   * Construct and simulate a schema_version transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  schema_version: (options?: MethodOptions) => Promise<AssembledTransaction<Result<u32>>>

  /**
   * Construct and simulate a revoke_evidence transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  revoke_evidence: ({id, reason_hash, revoker}: {id: Buffer, reason_hash: Buffer, revoker: string}, options?: MethodOptions) => Promise<AssembledTransaction<Result<void>>>

  /**
   * Construct and simulate a publish_evidence transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  publish_evidence: ({input}: {input: EvidenceInput}, options?: MethodOptions) => Promise<AssembledTransaction<Result<Buffer>>>

  /**
   * Construct and simulate a maintain_attestor transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  maintain_attestor: ({attestor}: {attestor: string}, options?: MethodOptions) => Promise<AssembledTransaction<Result<void>>>

  /**
   * Construct and simulate a maintain_evidence transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  maintain_evidence: ({id}: {id: Buffer}, options?: MethodOptions) => Promise<AssembledTransaction<Result<void>>>

  /**
   * Construct and simulate a register_attestor transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  register_attestor: ({attestor, metadata_hash}: {attestor: string, metadata_hash: Buffer}, options?: MethodOptions) => Promise<AssembledTransaction<Result<void>>>

  /**
   * Construct and simulate a supersede_evidence transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  supersede_evidence: ({old_id, replacement, authorizer}: {old_id: Buffer, replacement: EvidenceInput, authorizer: string}, options?: MethodOptions) => Promise<AssembledTransaction<Result<Buffer>>>

  /**
   * Construct and simulate a set_attestor_enabled transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  set_attestor_enabled: ({attestor, enabled}: {attestor: string, enabled: boolean}, options?: MethodOptions) => Promise<AssembledTransaction<Result<void>>>

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
      new ContractSpec([ "AAAABQAAAAAAAAAAAAAACFVwZ3JhZGVkAAAAAQAAAAh1cGdyYWRlZAAAAAIAAAAAAAAADW9sZF93YXNtX2hhc2gAAAAAAAPuAAAAIAAAAAAAAAAAAAAADW5ld193YXNtX2hhc2gAAAAAAAPuAAAAIAAAAAEAAAAC",
        "AAAABQAAAAAAAAAAAAAAC0F0dGVzdG9yU2V0AAAAAAEAAAAMYXR0ZXN0b3Jfc2V0AAAAAwAAAAAAAAAIYXR0ZXN0b3IAAAATAAAAAQAAAAAAAAAHZW5hYmxlZAAAAAABAAAAAAAAAAAAAAANbWV0YWRhdGFfaGFzaAAAAAAAA+4AAAAgAAAAAAAAAAI=",
        "AAAABQAAAAAAAAAAAAAAC0luaXRpYWxpemVkAAAAAAEAAAALaW5pdGlhbGl6ZWQAAAAAAgAAAAAAAAAFYWRtaW4AAAAAAAATAAAAAQAAAAAAAAAOc2NoZW1hX3ZlcnNpb24AAAAAAAQAAAAAAAAAAg==",
        "AAAABQAAAAAAAAAAAAAADEFkbWluQ2hhbmdlZAAAAAEAAAANYWRtaW5fY2hhbmdlZAAAAAAAAAIAAAAAAAAACW9sZF9hZG1pbgAAAAAAABMAAAABAAAAAAAAAAluZXdfYWRtaW4AAAAAAAATAAAAAQAAAAI=",
        "AAAABQAAAAAAAAAAAAAADFBhdXNlQ2hhbmdlZAAAAAEAAAANcGF1c2VfY2hhbmdlZAAAAAAAAAIAAAAAAAAABWFkbWluAAAAAAAAEwAAAAEAAAAAAAAABnBhdXNlZAAAAAAAAQAAAAAAAAAC",
        "AAAABQAAAAAAAAAAAAAADUFkbWluUHJvcG9zZWQAAAAAAAABAAAADmFkbWluX3Byb3Bvc2VkAAAAAAACAAAAAAAAAA1jdXJyZW50X2FkbWluAAAAAAAAEwAAAAEAAAAAAAAADXBlbmRpbmdfYWRtaW4AAAAAAAATAAAAAQAAAAI=",
        "AAAABQAAAAAAAAAAAAAAD0V2aWRlbmNlUmV2b2tlZAAAAAABAAAAEGV2aWRlbmNlX3Jldm9rZWQAAAADAAAAAAAAAAJpZAAAAAAD7gAAACAAAAABAAAAAAAAAAdyZXZva2VyAAAAABMAAAABAAAAAAAAAAtyZWFzb25faGFzaAAAAAPuAAAAIAAAAAAAAAAC",
        "AAAAAAAAAAAAAAAHdXBncmFkZQAAAAABAAAAAAAAAA1uZXdfd2FzbV9oYXNoAAAAAAAD7gAAACAAAAABAAAD6QAAAAIAAAfQAAAADUNvbnRyYWN0RXJyb3IAAAA=",
        "AAAABQAAAAAAAAAAAAAAEUV2aWRlbmNlUHVibGlzaGVkAAAAAAAAAQAAABJldmlkZW5jZV9wdWJsaXNoZWQAAAAAAAUAAAAAAAAAAmlkAAAAAAPuAAAAIAAAAAEAAAAAAAAACXB1Ymxpc2hlcgAAAAAAABMAAAABAAAAAAAAAAtyZXBvcnRfaGFzaAAAAAPuAAAAIAAAAAAAAAAAAAAAC3RhcmdldF9oYXNoAAAAA+4AAAAgAAAAAAAAAAAAAAAKc3VpdGVfaGFzaAAAAAAD7gAAACAAAAAAAAAAAg==",
        "AAAAAAAAAAAAAAAJaXNfYWN0aXZlAAAAAAAAAQAAAAAAAAACaWQAAAAAA+4AAAAgAAAAAQAAAAE=",
        "AAAABQAAAAAAAAAAAAAAEkV2aWRlbmNlU3VwZXJzZWRlZAAAAAAAAQAAABNldmlkZW5jZV9zdXBlcnNlZGVkAAAAAAMAAAAAAAAABm9sZF9pZAAAAAAD7gAAACAAAAABAAAAAAAAAAZuZXdfaWQAAAAAA+4AAAAgAAAAAQAAAAAAAAAKYXV0aG9yaXplcgAAAAAAEwAAAAAAAAAC",
        "AAAAAAAAAAAAAAAKaW5pdGlhbGl6ZQAAAAAAAgAAAAAAAAAFYWRtaW4AAAAAAAATAAAAAAAAAA5zY2hlbWFfdmVyc2lvbgAAAAAABAAAAAEAAAPpAAAAAgAAB9AAAAANQ29udHJhY3RFcnJvcgAAAA==",
        "AAAAAAAAAAAAAAAKc2V0X3BhdXNlZAAAAAAAAQAAAAAAAAAGcGF1c2VkAAAAAAABAAAAAQAAA+kAAAACAAAH0AAAAA1Db250cmFjdEVycm9yAAAA",
        "AAAAAAAAAAAAAAAMYWNjZXB0X2FkbWluAAAAAAAAAAEAAAPpAAAAAgAAB9AAAAANQ29udHJhY3RFcnJvcgAAAA==",
        "AAAAAAAAAAAAAAAMZ2V0X2F0dGVzdG9yAAAAAQAAAAAAAAAIYXR0ZXN0b3IAAAATAAAAAQAAA+gAAAfQAAAADkF0dGVzdG9yUmVjb3JkAAA=",
        "AAAAAAAAAAAAAAAMZ2V0X2V2aWRlbmNlAAAAAQAAAAAAAAACaWQAAAAAA+4AAAAgAAAAAQAAA+gAAAfQAAAADkV2aWRlbmNlUmVjb3JkAAA=",
        "AAAAAAAAAAAAAAANcHJvcG9zZV9hZG1pbgAAAAAAAAEAAAAAAAAACW5ld19hZG1pbgAAAAAAABMAAAABAAAD6QAAAAIAAAfQAAAADUNvbnRyYWN0RXJyb3IAAAA=",
        "AAAAAAAAAAAAAAAOc2NoZW1hX3ZlcnNpb24AAAAAAAAAAAABAAAD6QAAAAQAAAfQAAAADUNvbnRyYWN0RXJyb3IAAAA=",
        "AAAAAAAAAAAAAAAPcmV2b2tlX2V2aWRlbmNlAAAAAAMAAAAAAAAAAmlkAAAAAAPuAAAAIAAAAAAAAAALcmVhc29uX2hhc2gAAAAD7gAAACAAAAAAAAAAB3Jldm9rZXIAAAAAEwAAAAEAAAPpAAAAAgAAB9AAAAANQ29udHJhY3RFcnJvcgAAAA==",
        "AAAAAAAAAAAAAAAQcHVibGlzaF9ldmlkZW5jZQAAAAEAAAAAAAAABWlucHV0AAAAAAAH0AAAAA1FdmlkZW5jZUlucHV0AAAAAAAAAQAAA+kAAAPuAAAAIAAAB9AAAAANQ29udHJhY3RFcnJvcgAAAA==",
        "AAAAAAAAAAAAAAARbWFpbnRhaW5fYXR0ZXN0b3IAAAAAAAABAAAAAAAAAAhhdHRlc3RvcgAAABMAAAABAAAD6QAAAAIAAAfQAAAADUNvbnRyYWN0RXJyb3IAAAA=",
        "AAAAAAAAAAAAAAARbWFpbnRhaW5fZXZpZGVuY2UAAAAAAAABAAAAAAAAAAJpZAAAAAAD7gAAACAAAAABAAAD6QAAAAIAAAfQAAAADUNvbnRyYWN0RXJyb3IAAAA=",
        "AAAAAAAAAAAAAAARcmVnaXN0ZXJfYXR0ZXN0b3IAAAAAAAACAAAAAAAAAAhhdHRlc3RvcgAAABMAAAAAAAAADW1ldGFkYXRhX2hhc2gAAAAAAAPuAAAAIAAAAAEAAAPpAAAAAgAAB9AAAAANQ29udHJhY3RFcnJvcgAAAA==",
        "AAAAAAAAAAAAAAASc3VwZXJzZWRlX2V2aWRlbmNlAAAAAAADAAAAAAAAAAZvbGRfaWQAAAAAA+4AAAAgAAAAAAAAAAtyZXBsYWNlbWVudAAAAAfQAAAADUV2aWRlbmNlSW5wdXQAAAAAAAAAAAAACmF1dGhvcml6ZXIAAAAAABMAAAABAAAD6QAAA+4AAAAgAAAH0AAAAA1Db250cmFjdEVycm9yAAAA",
        "AAAAAAAAAAAAAAAUc2V0X2F0dGVzdG9yX2VuYWJsZWQAAAACAAAAAAAAAAhhdHRlc3RvcgAAABMAAAAAAAAAB2VuYWJsZWQAAAAAAQAAAAEAAAPpAAAAAgAAB9AAAAANQ29udHJhY3RFcnJvcgAAAA==",
        "AAAAAgAAAAAAAAAAAAAAC05ldHdvcmtLaW5kAAAAAAQAAAAAAAAAAAAAAAdUZXN0bmV0AAAAAAAAAAAAAAAACUZ1dHVyZW5ldAAAAAAAAAAAAAAAAAAABlB1Ym5ldAAAAAAAAAAAAAAAAAAKU3RhbmRhbG9uZQAA",
        "AAAABAAAAAAAAAAAAAAADUNvbnRyYWN0RXJyb3IAAAAAAAAPAAAAAAAAABJBbHJlYWR5SW5pdGlhbGl6ZWQAAAAAAAEAAAAAAAAADk5vdEluaXRpYWxpemVkAAAAAAACAAAAAAAAAAxVbmF1dGhvcml6ZWQAAAADAAAAAAAAAAZQYXVzZWQAAAAAAAQAAAAAAAAAC0ludmFsaWRIYXNoAAAAAAUAAAAAAAAADUludmFsaWRDb3VudHMAAAAAAAAGAAAAAAAAAAxJbnZhbGlkU2NvcmUAAAAHAAAAAAAAABVBdHRlc3Rvck5vdFJlZ2lzdGVyZWQAAAAAAAAIAAAAAAAAABBBdHRlc3RvckRpc2FibGVkAAAACQAAAAAAAAAVRXZpZGVuY2VBbHJlYWR5RXhpc3RzAAAAAAAACgAAAAAAAAAQRXZpZGVuY2VOb3RGb3VuZAAAAAsAAAAAAAAAEUV2aWRlbmNlTm90QWN0aXZlAAAAAAAADAAAAAAAAAATSW52YWxpZFN1cGVyc2Vzc2lvbgAAAAANAAAAAAAAABRBZG1pblByb3Bvc2FsTWlzc2luZwAAAA4AAAAAAAAAEVVwZ3JhZGVOb3RBbGxvd2VkAAAAAAAADw==",
        "AAAAAQAAAAAAAAAAAAAADUV2aWRlbmNlSW5wdXQAAAAAAAANAAAAAAAAAA1hcnRpZmFjdF9yb290AAAAAAAD7gAAACAAAAAAAAAABmZhaWxlZAAAAAAABAAAAAAAAAAHbmV0d29yawAAAAfQAAAAC05ldHdvcmtLaW5kAAAAAAAAAAAGcGFzc2VkAAAAAAAEAAAAAAAAAA9wcm90b2NvbF9iaXRtYXAAAAAABgAAAAAAAAAJcHVibGlzaGVyAAAAAAAAEwAAAAAAAAALcmVwb3J0X2hhc2gAAAAD7gAAACAAAAAAAAAACXNjb3JlX2JwcwAAAAAAAAQAAAAAAAAAB3NraXBwZWQAAAAABAAAAAAAAAAKc3BlY3NfaGFzaAAAAAAD7gAAACAAAAAAAAAACnN1aXRlX2hhc2gAAAAAA+4AAAAgAAAAAAAAAAt0YXJnZXRfaGFzaAAAAAPuAAAAIAAAAAAAAAAId2FybmluZ3MAAAAE",
        "AAAAAgAAAAAAAAAAAAAADUluc3RhbmNlS2V5VjEAAAAAAAAFAAAAAAAAAAAAAAAFQWRtaW4AAAAAAAAAAAAAAAAAAAxQZW5kaW5nQWRtaW4AAAAAAAAAAAAAAAZQYXVzZWQAAAAAAAAAAAAAAAAADVNjaGVtYVZlcnNpb24AAAAAAAAAAAAAAAAAAA9DdXJyZW50V2FzbUhhc2gA",
        "AAAAAQAAAAAAAAAAAAAADkF0dGVzdG9yUmVjb3JkAAAAAAAFAAAAAAAAAAhhdHRlc3RvcgAAABMAAAAAAAAAB2VuYWJsZWQAAAAAAQAAAAAAAAANbWV0YWRhdGFfaGFzaAAAAAAAA+4AAAAgAAAAAAAAABFyZWdpc3RlcmVkX2xlZGdlcgAAAAAAAAQAAAAAAAAADnVwZGF0ZWRfbGVkZ2VyAAAAAAAE",
        "AAAAAQAAAAAAAAAAAAAADkV2aWRlbmNlUmVjb3JkAAAAAAARAAAAAAAAAA1hcnRpZmFjdF9yb290AAAAAAAD7gAAACAAAAAAAAAADmNyZWF0ZWRfbGVkZ2VyAAAAAAAEAAAAAAAAAAZmYWlsZWQAAAAAAAQAAAAAAAAAAmlkAAAAAAPuAAAAIAAAAAAAAAAHbmV0d29yawAAAAfQAAAAC05ldHdvcmtLaW5kAAAAAAAAAAAGcGFzc2VkAAAAAAAEAAAAAAAAAA9wcm90b2NvbF9iaXRtYXAAAAAABgAAAAAAAAAJcHVibGlzaGVyAAAAAAAAEwAAAAAAAAALcmVwb3J0X2hhc2gAAAAD7gAAACAAAAAAAAAACXNjb3JlX2JwcwAAAAAAAAQAAAAAAAAAB3NraXBwZWQAAAAABAAAAAAAAAAKc3BlY3NfaGFzaAAAAAAD7gAAACAAAAAAAAAABnN0YXR1cwAAAAAH0AAAAA5FdmlkZW5jZVN0YXR1cwAAAAAAAAAAAApzdWl0ZV9oYXNoAAAAAAPuAAAAIAAAAAAAAAAKc3VwZXJzZWRlcwAAAAAD6AAAA+4AAAAgAAAAAAAAAAt0YXJnZXRfaGFzaAAAAAPuAAAAIAAAAAAAAAAId2FybmluZ3MAAAAE",
        "AAAAAgAAAAAAAAAAAAAADkV2aWRlbmNlU3RhdHVzAAAAAAADAAAAAAAAAAAAAAAGQWN0aXZlAAAAAAAAAAAAAAAAAApTdXBlcnNlZGVkAAAAAAAAAAAAAAAAAAdSZXZva2VkAA==",
        "AAAAAgAAAAAAAAAAAAAAD1BlcnNpc3RlbnRLZXlWMQAAAAAEAAAAAQAAAAAAAAAIQXR0ZXN0b3IAAAABAAAAEwAAAAEAAAAAAAAACEV2aWRlbmNlAAAAAQAAA+4AAAAgAAAAAQAAAAAAAAAMQWN0aXZlUmVwb3J0AAAAAwAAABMAAAPuAAAAIAAAB9AAAAALTmV0d29ya0tpbmQAAAAAAQAAAAAAAAAMU3VwZXJzZWRlZEJ5AAAAAQAAA+4AAAAg" ]),
      options
    )
  }
  public readonly fromJSON = {
    upgrade: this.txFromJSON<Result<void>>,
        is_active: this.txFromJSON<boolean>,
        initialize: this.txFromJSON<Result<void>>,
        set_paused: this.txFromJSON<Result<void>>,
        accept_admin: this.txFromJSON<Result<void>>,
        get_attestor: this.txFromJSON<Option<AttestorRecord>>,
        get_evidence: this.txFromJSON<Option<EvidenceRecord>>,
        propose_admin: this.txFromJSON<Result<void>>,
        schema_version: this.txFromJSON<Result<u32>>,
        revoke_evidence: this.txFromJSON<Result<void>>,
        publish_evidence: this.txFromJSON<Result<Buffer>>,
        maintain_attestor: this.txFromJSON<Result<void>>,
        maintain_evidence: this.txFromJSON<Result<void>>,
        register_attestor: this.txFromJSON<Result<void>>,
        supersede_evidence: this.txFromJSON<Result<Buffer>>,
        set_attestor_enabled: this.txFromJSON<Result<void>>
  }
}