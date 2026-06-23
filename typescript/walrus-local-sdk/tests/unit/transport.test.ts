// Copyright (c) Suibase contributors
// SPDX-License-Identifier: Apache-2.0

// Pure unit tests (no localnet, no network) for SbLocalTransport's error decoding.
// Locks the ERR-3 wire-format fix: the machine-readable reason is read from the Walrus
// aggregator/publisher `Status` envelope at `error.details[0].reason`, with a fallback to
// the legacy `error.reason` so a new client still understands an older sb-local build.

import { test, describe } from "node:test";
import assert from "node:assert/strict";

import { SbLocalTransport } from "../../src/sbLocal.js";
import { WalrusLocalError } from "../../src/errors.js";

/** A fetch stub returning one canned Response (status + JSON/text body). Never hits the network. */
function stubFetch(status: number, body: unknown, json = true): typeof fetch {
  return (async () =>
    new Response(json ? JSON.stringify(body) : String(body), {
      status,
      headers: json ? { "content-type": "application/json" } : {},
    })) as unknown as typeof fetch;
}

/** The real Walrus aggregator/publisher `Status` envelope sb-local now emits. */
function statusEnvelope(reason: string, message: string, status: string, code: number) {
  return {
    error: {
      status,
      code,
      message,
      details: [{ "@type": "ErrorInfo", reason, domain: "daemon.walrus.space", metadata: {} }],
    },
  };
}

describe("SbLocalTransport error envelope decoding (ERR-3)", () => {
  test("reads the reason from error.details[0].reason (Status envelope)", async () => {
    const t = new SbLocalTransport("http://sb-local", {
      fetch: stubFetch(404, statusEnvelope("BLOB_NOT_FOUND", "blob X not found", "NOT_FOUND", 404)),
      timeoutMs: 0,
    });
    await assert.rejects(
      () => t.readBlob("X"),
      (e: unknown) =>
        e instanceof WalrusLocalError &&
        e.code === "BLOB_NOT_FOUND" &&
        e.status === 404 &&
        e.context.reason === "BLOB_NOT_FOUND",
    );
  });

  test("maps a quilt-patch 404 envelope to QUILT_PATCH_NOT_FOUND", async () => {
    const t = new SbLocalTransport("http://sb-local", {
      fetch: stubFetch(404, statusEnvelope("QUILT_PATCH_NOT_FOUND", "patch gone", "NOT_FOUND", 404)),
      timeoutMs: 0,
    });
    await assert.rejects(
      () => t.readQuiltPatchById("patchid"),
      (e: unknown) => e instanceof WalrusLocalError && e.code === "QUILT_PATCH_NOT_FOUND",
    );
  });

  test("maps a 400 INVALID_ARGUMENT envelope to BAD_REQUEST", async () => {
    const t = new SbLocalTransport("http://sb-local", {
      fetch: stubFetch(400, statusEnvelope("BAD_REQUEST", "bad id", "INVALID_ARGUMENT", 400)),
      timeoutMs: 0,
    });
    await assert.rejects(
      () => t.readBlob("bad"),
      (e: unknown) => e instanceof WalrusLocalError && e.code === "BAD_REQUEST" && e.status === 400,
    );
  });

  test("falls back to the legacy error.reason shape (older sb-local builds)", async () => {
    const legacy = { error: { reason: "BLOB_NOT_FOUND", message: "blob X not found" } };
    const t = new SbLocalTransport("http://sb-local", { fetch: stubFetch(404, legacy), timeoutMs: 0 });
    await assert.rejects(
      () => t.readBlob("X"),
      (e: unknown) => e instanceof WalrusLocalError && e.code === "BLOB_NOT_FOUND",
    );
  });

  test("a non-JSON error body falls back to the HTTP-status-derived code", async () => {
    const t = new SbLocalTransport("http://sb-local", {
      fetch: stubFetch(500, "upstream exploded", false),
      timeoutMs: 0,
    });
    await assert.rejects(
      () => t.readBlob("X"),
      (e: unknown) => e instanceof WalrusLocalError && e.code === "INTERNAL" && e.status === 500,
    );
  });
});
