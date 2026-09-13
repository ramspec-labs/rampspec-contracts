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




export const PolicyError = {
  1: {message:"NotInitialized"},
  2: {message:"AlreadyInitialized"},
  3: {message:"InvalidPolicy"},
  4: {message:"MissingSignature"},
  5: {message:"DuplicateSigner"},
  6: {message:"UnauthorizedSigner"},
  7: {message:"InsufficientWeight"},
  8: {message:"InvalidTimeWindow"},
  9: {message:"NotYetValid"},
  10: {message:"Expired"},
  11: {message:"Rejected"},
  12: {message:"Replay"},
  13: {message:"RequiredSignerMissing"},
  14: {message:"InvalidContext"},
  15: {message:"WrongSignatureMode"},
  16: {message:"InvalidPasskey"}
}


export interface PolicyConfig {
  admin: string;
  controls: PolicyControls;
  passkey_signers: Array<PasskeySigner>;
  signature_mode: SignatureMode;
  signers: Array<Ed25519Signer>;
  threshold: u32;
}


export interface Ed25519Signer {
  public_key: Buffer;
  weight: u32;
}


export interface PasskeySigner {
  public_key: Buffer;
  weight: u32;
}

export type SignatureMode = {tag: "Ed25519", values: void} | {tag: "PasskeyCompatible", values: void};


export interface PolicyControls {
  additional_signer: Option<Buffer>;
  expires_at_ledger: u32;
  reject_all: boolean;
  required_contract: Option<string>;
  required_function: Option<string>;
  valid_from_ledger: u32;
}

export type AccountSignature = {tag: "Ed25519", values: readonly [Ed25519Signature]} | {tag: "Passkey", values: readonly [PasskeySignature]};


export interface Ed25519Signature {
  public_key: Buffer;
  signature: Buffer;
}


export interface PasskeySignature {
  authenticator_data: Buffer;
  challenge: Buffer;
  client_data_hash: Buffer;
  public_key: Buffer;
  signature: Buffer;
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
   * Construct and simulate a config transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  config: (options?: MethodOptions) => Promise<AssembledTransaction<Result<PolicyConfig>>>

  /**
   * Construct and simulate a initialize transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  initialize: ({admin, signers, threshold}: {admin: string, signers: Array<Ed25519Signer>, threshold: u32}, options?: MethodOptions) => Promise<AssembledTransaction<Result<void>>>

  /**
   * Construct and simulate a set_signers transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  set_signers: ({signers, threshold}: {signers: Array<Ed25519Signer>, threshold: u32}, options?: MethodOptions) => Promise<AssembledTransaction<Result<void>>>

  /**
   * Construct and simulate a is_test_only transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  is_test_only: (options?: MethodOptions) => Promise<AssembledTransaction<boolean>>

  /**
   * Construct and simulate a set_controls transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  set_controls: ({controls}: {controls: PolicyControls}, options?: MethodOptions) => Promise<AssembledTransaction<Result<void>>>

  /**
   * Construct and simulate a set_passkeys transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  set_passkeys: ({signers, threshold}: {signers: Array<PasskeySigner>, threshold: u32}, options?: MethodOptions) => Promise<AssembledTransaction<Result<void>>>

  /**
   * Construct and simulate a schema_version transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  schema_version: (options?: MethodOptions) => Promise<AssembledTransaction<u32>>

  /**
   * Construct and simulate a passkey_supported transaction. Returns an `AssembledTransaction` object which will have a `result` field containing the result of the simulation. If this transaction changes contract state, you will need to call `signAndSend()` on the returned object.
   */
  passkey_supported: (options?: MethodOptions) => Promise<AssembledTransaction<boolean>>

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
      new ContractSpec([ "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",
        "AAAAAAAAAAAAAAALc2V0X3NpZ25lcnMAAAAAAgAAAAAAAAAHc2lnbmVycwAAAAPqAAAH0AAAAA1FZDI1NTE5U2lnbmVyAAAAAAAAAAAAAAl0aHJlc2hvbGQAAAAAAAAEAAAAAQAAA+kAAAACAAAH0AAAAAtQb2xpY3lFcnJvcgA=",
        "AAAAAAAAAAAAAAAMX19jaGVja19hdXRoAAAAAwAAAAAAAAARc2lnbmF0dXJlX3BheWxvYWQAAAAAAAPuAAAAIAAAAAAAAAAKc2lnbmF0dXJlcwAAAAAD6gAAB9AAAAAQQWNjb3VudFNpZ25hdHVyZQAAAAAAAAANYXV0aF9jb250ZXh0cwAAAAAAA+oAAAfQAAAAB0NvbnRleHQAAAAAAQAAA+kAAAACAAAH0AAAAAtQb2xpY3lFcnJvcgA=",
        "AAAAAAAAAAAAAAAMaXNfdGVzdF9vbmx5AAAAAAAAAAEAAAAB",
        "AAAAAAAAAAAAAAAMc2V0X2NvbnRyb2xzAAAAAQAAAAAAAAAIY29udHJvbHMAAAfQAAAADlBvbGljeUNvbnRyb2xzAAAAAAABAAAD6QAAAAIAAAfQAAAAC1BvbGljeUVycm9yAA==",
        "AAAAAAAAAAAAAAAMc2V0X3Bhc3NrZXlzAAAAAgAAAAAAAAAHc2lnbmVycwAAAAPqAAAH0AAAAA1QYXNza2V5U2lnbmVyAAAAAAAAAAAAAAl0aHJlc2hvbGQAAAAAAAAEAAAAAQAAA+kAAAACAAAH0AAAAAtQb2xpY3lFcnJvcgA=",
        "AAAAAAAAAAAAAAAOc2NoZW1hX3ZlcnNpb24AAAAAAAAAAAABAAAABA==",
        "AAAAAAAAAAAAAAARcGFzc2tleV9zdXBwb3J0ZWQAAAAAAAAAAAAAAQAAAAE=",
        "AAAAAgAAAAAAAAAAAAAAC05ldHdvcmtLaW5kAAAAAAQAAAAAAAAAAAAAAAdUZXN0bmV0AAAAAAAAAAAAAAAACUZ1dHVyZW5ldAAAAAAAAAAAAAAAAAAABlB1Ym5ldAAAAAAAAAAAAAAAAAAKU3RhbmRhbG9uZQAA",
        "AAAABAAAAAAAAAAAAAAADUNvbnRyYWN0RXJyb3IAAAAAAAAPAAAAAAAAABJBbHJlYWR5SW5pdGlhbGl6ZWQAAAAAAAEAAAAAAAAADk5vdEluaXRpYWxpemVkAAAAAAACAAAAAAAAAAxVbmF1dGhvcml6ZWQAAAADAAAAAAAAAAZQYXVzZWQAAAAAAAQAAAAAAAAAC0ludmFsaWRIYXNoAAAAAAUAAAAAAAAADUludmFsaWRDb3VudHMAAAAAAAAGAAAAAAAAAAxJbnZhbGlkU2NvcmUAAAAHAAAAAAAAABVBdHRlc3Rvck5vdFJlZ2lzdGVyZWQAAAAAAAAIAAAAAAAAABBBdHRlc3RvckRpc2FibGVkAAAACQAAAAAAAAAVRXZpZGVuY2VBbHJlYWR5RXhpc3RzAAAAAAAACgAAAAAAAAAQRXZpZGVuY2VOb3RGb3VuZAAAAAsAAAAAAAAAEUV2aWRlbmNlTm90QWN0aXZlAAAAAAAADAAAAAAAAAATSW52YWxpZFN1cGVyc2Vzc2lvbgAAAAANAAAAAAAAABRBZG1pblByb3Bvc2FsTWlzc2luZwAAAA4AAAAAAAAAEVVwZ3JhZGVOb3RBbGxvd2VkAAAAAAAADw==",
        "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" ]),
      options
    )
  }
  public readonly fromJSON = {
    config: this.txFromJSON<Result<PolicyConfig>>,
        initialize: this.txFromJSON<Result<void>>,
        set_signers: this.txFromJSON<Result<void>>,
        is_test_only: this.txFromJSON<boolean>,
        set_controls: this.txFromJSON<Result<void>>,
        set_passkeys: this.txFromJSON<Result<void>>,
        schema_version: this.txFromJSON<u32>,
        passkey_supported: this.txFromJSON<boolean>
  }
}