// resolveLocalnetConfig: descriptor parsing + sb-local URL resolution + the localnet-only
// guard. Uses a throwaway temp suibase root (no live suibase / no network).

import { test, describe } from "node:test";
import assert from "node:assert/strict";
import { mkdtempSync, mkdirSync, writeFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { resolveLocalnetConfig, WalrusLocalError } from "../../src/index.js";

function makeRoot(opts: { descriptor?: string; suibaseYaml?: string } = {}): { root: string; cleanup: () => void } {
  const root = mkdtempSync(join(tmpdir(), "wls-localnet-"));
  const cfgDir = join(root, "workdirs", "localnet", "config");
  mkdirSync(cfgDir, { recursive: true });
  if (opts.descriptor !== undefined) writeFileSync(join(cfgDir, "walrus-localnet.yaml"), opts.descriptor);
  if (opts.suibaseYaml !== undefined) writeFileSync(join(root, "workdirs", "localnet", "suibase.yaml"), opts.suibaseYaml);
  return { root, cleanup: () => rmSync(root, { recursive: true, force: true }) };
}

const DESCRIPTOR = [
  "chain_id: deadbeef",
  "epoch: 1",
  "package_id: 0xpkg",
  "system_object: 0xsys",
  "staking_object: 0xstake",
  "exchange_object: 0xexch",
  "treasury_object: 0xtreasury",
  "n_shards: 1000",
  "committee_protocol_keypair: AAAA",
  "",
].join("\n");

describe("resolveLocalnetConfig", () => {
  test("parses the descriptor into a WalrusClient package config", () => {
    const fx = makeRoot({ descriptor: DESCRIPTOR });
    try {
      const r = resolveLocalnetConfig({ suibaseRoot: fx.root });
      assert.equal(r.packageId, "0xpkg");
      assert.equal(r.walrusConfig.packageConfig?.systemObjectId, "0xsys");
      assert.equal(r.walrusConfig.packageConfig?.stakingPoolId, "0xstake");
      assert.deepEqual(r.walrusConfig.packageConfig?.exchangeIds, ["0xexch"]);
      // No suibase.yaml -> default sb-local endpoint.
      assert.equal(r.aggregatorUrl, "http://127.0.0.1:45840");
    } finally {
      fx.cleanup();
    }
  });

  test("reads sb_local_walrus_port / host from suibase.yaml", () => {
    const fx = makeRoot({
      descriptor: DESCRIPTOR,
      suibaseYaml: ["walrus_local_enabled: true", "sb_local_walrus_port: 46000", 'sb_local_host_ip: "192.168.0.9"', ""].join("\n"),
    });
    try {
      const r = resolveLocalnetConfig({ suibaseRoot: fx.root });
      assert.equal(r.aggregatorUrl, "http://192.168.0.9:46000");
    } finally {
      fx.cleanup();
    }
  });

  test("normalizes a 'localhost' sb-local host to 127.0.0.1", () => {
    const fx = makeRoot({ descriptor: DESCRIPTOR, suibaseYaml: "sb_local_host_ip: localhost\n" });
    try {
      assert.equal(resolveLocalnetConfig({ suibaseRoot: fx.root }).aggregatorUrl, "http://127.0.0.1:45840");
    } finally {
      fx.cleanup();
    }
  });

  test("honors an explicit aggregatorUrl override", () => {
    const fx = makeRoot({ descriptor: DESCRIPTOR });
    try {
      const r = resolveLocalnetConfig({ suibaseRoot: fx.root, aggregatorUrl: "http://127.0.0.1:9999/" });
      assert.equal(r.aggregatorUrl, "http://127.0.0.1:9999"); // trailing slash trimmed
    } finally {
      fx.cleanup();
    }
  });

  test("throws NOT_LOCALNET when the descriptor is absent", () => {
    const fx = makeRoot(); // no descriptor
    try {
      assert.throws(
        () => resolveLocalnetConfig({ suibaseRoot: fx.root }),
        (e: unknown) => e instanceof WalrusLocalError && e.code === "NOT_LOCALNET",
      );
    } finally {
      fx.cleanup();
    }
  });

  test("throws NOT_LOCALNET when a required descriptor key is missing", () => {
    const fx = makeRoot({ descriptor: "package_id: 0xpkg\nsystem_object: 0xsys\n" }); // no staking_object
    try {
      assert.throws(
        () => resolveLocalnetConfig({ suibaseRoot: fx.root }),
        (e: unknown) => e instanceof WalrusLocalError && e.code === "NOT_LOCALNET",
      );
    } finally {
      fx.cleanup();
    }
  });
});
