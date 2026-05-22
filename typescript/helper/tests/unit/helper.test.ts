import { test, describe, before, after } from "node:test";
import assert from "node:assert/strict";

import { Helper, SuibaseError } from "../../src/index.js";
import { makeFixture, type Fixture } from "./fixtures.js";

describe("Helper.isInstalled()", () => {
  test("returns true when root + workdirs/ exist", () => {
    const fx = makeFixture([]);
    try {
      const sbh = new Helper({ rootPath: fx.root });
      assert.equal(sbh.isInstalled(), true);
    } finally {
      fx.cleanup();
    }
  });

  test("returns false when root does not exist", () => {
    const sbh = new Helper({ rootPath: "/nonexistent/path/suibase" });
    assert.equal(sbh.isInstalled(), false);
  });
});

describe("Helper.selectWorkdir()", () => {
  let fx: Fixture;
  before(() => {
    fx = makeFixture([
      { name: "localnet", withDemo: true, withKeystore: true, withClientYaml: true, active: true },
      { name: "devnet" },
    ]);
  });
  after(() => fx.cleanup());

  test("selects a real workdir by name", () => {
    const sbh = new Helper({ rootPath: fx.root });
    sbh.selectWorkdir("localnet");
    assert.equal(sbh.workdir(), "localnet");
  });

  test("'active' resolves to the workdir behind the symlink", () => {
    const sbh = new Helper({ rootPath: fx.root });
    sbh.selectWorkdir("active");
    // The fixture set localnet as active.
    assert.equal(sbh.workdir(), "localnet");
  });

  test("empty name throws", () => {
    const sbh = new Helper({ rootPath: fx.root });
    assert.throws(() => sbh.selectWorkdir(""), (e: unknown) =>
      e instanceof SuibaseError && e.code === "WorkdirNameEmpty",
    );
  });

  test("nonexistent name throws", () => {
    const sbh = new Helper({ rootPath: fx.root });
    assert.throws(() => sbh.selectWorkdir("ghost"), (e: unknown) =>
      e instanceof SuibaseError &&
        (e.code === "WorkdirNotExists" || e.code === "WorkdirAccessError"),
    );
  });

  test("workdir() before selectWorkdir() throws", () => {
    const sbh = new Helper({ rootPath: fx.root });
    assert.throws(() => sbh.workdir(), (e: unknown) =>
      e instanceof SuibaseError && e.code === "WorkdirNotSelected",
    );
  });
});

describe("Helper.packageId() / packageObjectId()", () => {
  let fx: Fixture;
  before(() => {
    fx = makeFixture([{ name: "localnet", withDemo: true }]);
  });
  after(() => fx.cleanup());

  test("returns the published package id (0x... 66 chars)", () => {
    const sbh = new Helper({ rootPath: fx.root });
    sbh.selectWorkdir("localnet");
    const id = sbh.packageId("demo");
    assert.ok(id.startsWith("0x"));
    assert.equal(id.length, 66);
  });

  test("missing package throws PublishedData* error", () => {
    const sbh = new Helper({ rootPath: fx.root });
    sbh.selectWorkdir("localnet");
    assert.throws(() => sbh.packageId("nopkg"), (e: unknown) =>
      e instanceof SuibaseError && e.code.startsWith("PublishedData"),
    );
  });

  test("empty package name throws", () => {
    const sbh = new Helper({ rootPath: fx.root });
    sbh.selectWorkdir("localnet");
    assert.throws(() => sbh.packageId(""), (e: unknown) =>
      e instanceof SuibaseError && e.code === "PackageNameEmpty",
    );
  });

  test("without selectWorkdir throws WorkdirNotSelected", () => {
    const sbh = new Helper({ rootPath: fx.root });
    assert.throws(() => sbh.packageId("demo"), (e: unknown) =>
      e instanceof SuibaseError && e.code === "WorkdirNotSelected",
    );
  });
});

describe("Helper.publishedNewObjects()", () => {
  let fx: Fixture;
  before(() => {
    fx = makeFixture([{ name: "localnet", withDemo: true }]);
  });
  after(() => fx.cleanup());

  test("returns matching object IDs for module::type", () => {
    const sbh = new Helper({ rootPath: fx.root });
    sbh.selectWorkdir("localnet");
    const ids = sbh.publishedNewObjects("demo::Counter::Counter");
    assert.equal(ids.length, 2);
    for (const id of ids) {
      assert.ok(id.startsWith("0x"));
      assert.equal(id.length, 66);
    }
  });

  test("returns empty array when no objects match", () => {
    const sbh = new Helper({ rootPath: fx.root });
    sbh.selectWorkdir("localnet");
    const ids = sbh.publishedNewObjects("demo::Nope::Missing");
    assert.deepEqual(ids, []);
  });

  test("invalid format throws", () => {
    const sbh = new Helper({ rootPath: fx.root });
    sbh.selectWorkdir("localnet");
    assert.throws(() => sbh.publishedNewObjects("just_one_part"), (e: unknown) =>
      e instanceof SuibaseError && e.code === "ObjectTypeInvalidFormat",
    );
    assert.throws(() => sbh.publishedNewObjects("demo::Counter::Counter::Extra"), (e: unknown) =>
      e instanceof SuibaseError && e.code === "ObjectTypeInvalidFormat",
    );
  });

  test("empty field throws", () => {
    const sbh = new Helper({ rootPath: fx.root });
    sbh.selectWorkdir("localnet");
    assert.throws(() => sbh.publishedNewObjects("demo:: ::Counter"), (e: unknown) =>
      e instanceof SuibaseError && e.code === "ObjectTypeMissingField",
    );
  });
});

describe("Helper.clientAddress()", () => {
  let fx: Fixture;
  before(() => {
    fx = makeFixture([{ name: "localnet", withClientYaml: true }]);
  });
  after(() => fx.cleanup());

  test("returns 'active' address from client.yaml", () => {
    const sbh = new Helper({ rootPath: fx.root });
    sbh.selectWorkdir("localnet");
    const addr = sbh.clientAddress("active");
    assert.equal(
      addr,
      "0xabcd1234ef567890abcd1234ef567890abcd1234ef567890abcd1234ef567890",
    );
  });

  test("returns named address from .state/dns", () => {
    const sbh = new Helper({ rootPath: fx.root });
    sbh.selectWorkdir("localnet");
    const addr = sbh.clientAddress("sb-1-ed25519");
    assert.equal(
      addr,
      "0x0fc530455ee4132b761ed82dab732990cb7af73e69cd6e719a2a5badeaed105b",
    );
  });

  test("empty name throws", () => {
    const sbh = new Helper({ rootPath: fx.root });
    sbh.selectWorkdir("localnet");
    assert.throws(() => sbh.clientAddress(""), (e: unknown) =>
      e instanceof SuibaseError && e.code === "AddressNameEmpty",
    );
  });

  test("unknown name throws AddressNameNotFound", () => {
    const sbh = new Helper({ rootPath: fx.root });
    sbh.selectWorkdir("localnet");
    assert.throws(() => sbh.clientAddress("sb-99-bogus"), (e: unknown) =>
      e instanceof SuibaseError && e.code === "AddressNameNotFound",
    );
  });
});

describe("Helper.rpcUrl() / wsUrl()", () => {
  let fx: Fixture;
  before(() => {
    fx = makeFixture([{ name: "localnet" }]);
  });
  after(() => fx.cleanup());

  test("rpcUrl reads from .state/links primary link", () => {
    const sbh = new Helper({ rootPath: fx.root });
    sbh.selectWorkdir("localnet");
    assert.equal(sbh.rpcUrl(), "http://localhost:9000/localnet");
  });

  test("wsUrl reads from .state/links primary link", () => {
    const sbh = new Helper({ rootPath: fx.root });
    sbh.selectWorkdir("localnet");
    assert.equal(sbh.wsUrl(), "ws://localhost:9000/localnet");
  });
});

describe("Helper.keystorePathname()", () => {
  test("returns path when keystore exists", () => {
    const fx = makeFixture([{ name: "localnet", withKeystore: true }]);
    try {
      const sbh = new Helper({ rootPath: fx.root });
      sbh.selectWorkdir("localnet");
      const p = sbh.keystorePathname();
      assert.ok(p.endsWith("config/sui.keystore"));
    } finally {
      fx.cleanup();
    }
  });

  test("throws when keystore missing", () => {
    const fx = makeFixture([{ name: "localnet" }]);
    try {
      const sbh = new Helper({ rootPath: fx.root });
      sbh.selectWorkdir("localnet");
      assert.throws(() => sbh.keystorePathname(), (e: unknown) =>
        e instanceof SuibaseError && e.code === "SuibaseKeystoreNotExists",
      );
    } finally {
      fx.cleanup();
    }
  });
});
