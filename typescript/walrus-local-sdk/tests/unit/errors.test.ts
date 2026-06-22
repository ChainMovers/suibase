// codeFromResponse mapping + WalrusLocalError shape (pure; no server, no @mysten deps).

import { test, describe } from "node:test";
import assert from "node:assert/strict";

import { codeFromResponse, WalrusLocalError } from "../../src/errors.js";

describe("codeFromResponse", () => {
  test("maps sb-local error reasons to codes", () => {
    assert.equal(codeFromResponse(404, "BLOB_NOT_FOUND"), "BLOB_NOT_FOUND");
    assert.equal(codeFromResponse(404, "QUILT_PATCH_NOT_FOUND"), "QUILT_PATCH_NOT_FOUND");
    assert.equal(codeFromResponse(404, "QUILT_NOT_FOUND"), "QUILT_NOT_FOUND");
    assert.equal(codeFromResponse(400, "BAD_REQUEST"), "BAD_REQUEST");
    assert.equal(codeFromResponse(416, "INVALID_BYTE_RANGE"), "INVALID_BYTE_RANGE");
    assert.equal(codeFromResponse(500, "INTERNAL"), "INTERNAL");
  });

  test("falls back to status when reason is missing/unknown", () => {
    assert.equal(codeFromResponse(404), "BLOB_NOT_FOUND");
    assert.equal(codeFromResponse(400), "BAD_REQUEST");
    assert.equal(codeFromResponse(416), "INVALID_BYTE_RANGE");
    assert.equal(codeFromResponse(503, "WHATEVER"), "INTERNAL");
  });
});

describe("WalrusLocalError", () => {
  test("carries code/status/context and a prefixed message", () => {
    const e = new WalrusLocalError("BLOB_NOT_FOUND", "blob X not found", {
      status: 404,
      context: { url: "http://x/y" },
    });
    assert.equal(e.code, "BLOB_NOT_FOUND");
    assert.equal(e.status, 404);
    assert.deepEqual(e.context, { url: "http://x/y" });
    assert.match(e.message, /^walrus-local-sdk: /);
    assert.ok(e instanceof Error);
    assert.equal(e.name, "WalrusLocalError");
  });
});
