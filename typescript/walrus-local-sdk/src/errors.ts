// Copyright (c) Suibase contributors
// SPDX-License-Identifier: Apache-2.0

/**
 * Error codes raised by {@link WalrusLocalError}. The `*_NOT_FOUND` / `BAD_REQUEST` /
 * `INVALID_BYTE_RANGE` / `INTERNAL` codes are derived from sb-local's wire error body —
 * the Walrus aggregator/publisher `Status` envelope (`{ "error": { "status", "code",
 * "message", "details": [{ "@type": "ErrorInfo", "reason", "domain", … }] } }`, with the
 * machine-readable reason at `error.details[0].reason`) — and the HTTP status, so they
 * match the real Walrus aggregator/publisher, which emits the same contract.
 */
export type WalrusLocalErrorCode =
  // --- mapped from sb-local's wire error responses ---
  /** A valid-format blob id that is not stored (HTTP 404). Mirrors the SDK's `BlobIdDoesNotExist`. */
  | "BLOB_NOT_FOUND"
  /** A quilt patch (by patch-id or quilt-id+identifier) that does not exist (HTTP 404). */
  | "QUILT_PATCH_NOT_FOUND"
  /** A quilt whose patches were requested but which does not exist (HTTP 404). */
  | "QUILT_NOT_FOUND"
  /** Malformed input the server rejected (HTTP 400) — e.g. a malformed id or `epochs=0`. */
  | "BAD_REQUEST"
  /** Range request the server could not satisfy (HTTP 416). */
  | "INVALID_BYTE_RANGE"
  /** The server hit an internal error (HTTP 500). */
  | "INTERNAL"
  // --- client-side (raised before/around the request) ---
  /** Invalid byte-range INPUT (zero length, negative, non-integer, overflow) — validated locally. */
  | "INVALID_RANGE_INPUT"
  /** A workdir other than `localnet` was requested (the nodeless mock is localnet-only). */
  | "NOT_LOCALNET"
  /** sb-local could not be reached (connection refused / DNS / timeout). */
  | "SERVER_UNREACHABLE"
  /** The server replied, but not in the shape this client expects. */
  | "UNEXPECTED_RESPONSE"
  /**
   * A `@mysten/walrus` method that is inherently node-coupled (sliver/encoding plumbing,
   * upload-relay, step-by-step flows) and has no meaning on a nodeless localnet. Use the
   * high-level `writeBlob`/`writeFiles`/`readBlob`/`getFiles` instead.
   */
  | "UNSUPPORTED";

/** Structured error for all walrus-local-sdk failures. */
export class WalrusLocalError extends Error {
  readonly code: WalrusLocalErrorCode;
  /** The HTTP status code, when the error came from a server response. */
  readonly status: number | undefined;
  /** Extra debugging context (url, reason, requested range, …). Always string-valued. */
  readonly context: Record<string, string>;

  constructor(
    code: WalrusLocalErrorCode,
    message: string,
    options: { status?: number; context?: Record<string, string>; cause?: unknown } = {},
  ) {
    super(`walrus-local-sdk: ${message}`, { cause: options.cause });
    this.name = "WalrusLocalError";
    this.code = code;
    this.status = options.status;
    this.context = options.context ?? {};
  }
}

/**
 * Map an HTTP status + sb-local error `reason` to a {@link WalrusLocalErrorCode}.
 * `reason` is read from the Status envelope's `error.details[0].reason` (legacy
 * `error.reason` as fallback) — one of `BLOB_NOT_FOUND`, `QUILT_PATCH_NOT_FOUND`,
 * `QUILT_NOT_FOUND`, `BAD_REQUEST`, `INVALID_BYTE_RANGE`, `INTERNAL`; the status is the
 * fallback when no/unknown reason is present.
 */
export function codeFromResponse(status: number, reason?: string): WalrusLocalErrorCode {
  switch (reason) {
    case "BLOB_NOT_FOUND":
      return "BLOB_NOT_FOUND";
    case "QUILT_PATCH_NOT_FOUND":
      return "QUILT_PATCH_NOT_FOUND";
    case "QUILT_NOT_FOUND":
      return "QUILT_NOT_FOUND";
    case "BAD_REQUEST":
      return "BAD_REQUEST";
    case "INVALID_BYTE_RANGE":
      return "INVALID_BYTE_RANGE";
    case "INTERNAL":
      return "INTERNAL";
    default:
      break;
  }
  if (status === 404) return "BLOB_NOT_FOUND";
  if (status === 400) return "BAD_REQUEST";
  if (status === 416) return "INVALID_BYTE_RANGE";
  return "INTERNAL";
}
