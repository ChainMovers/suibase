// Mirrors rust/helper/tests/helper_tests.rs.
//
// Assumes:
//  - suibase is installed at ~/suibase
//  - localnet is started
//  - 'demo' package is published to localnet
//
// In CI/CD this is set up by scripts/tests/030_rust_cargo_tests/__test_common.sh

import { test } from "node:test";
import assert from "node:assert/strict";

import { Helper } from "../../src/index.js";

test("integration: isInstalled", () => {
  const sbh = new Helper();
  assert.equal(sbh.isInstalled(), true);
});

test("integration: localnet workdir resolves to 'localnet'", () => {
  const sbh = new Helper();
  assert.equal(sbh.isInstalled(), true);
  sbh.selectWorkdir("localnet");
  assert.equal(sbh.workdir(), "localnet");
});

test("integration: demo package_id is a valid 66-char hex string", () => {
  const sbh = new Helper();
  assert.equal(sbh.isInstalled(), true);
  sbh.selectWorkdir("localnet");
  assert.equal(sbh.workdir(), "localnet");
  const id = sbh.packageId("demo");
  assert.ok(id.startsWith("0x"), `expected 0x-prefixed id, got ${id}`);
  assert.equal(id.length, 66, `expected length 66, got ${id.length}`);
});
