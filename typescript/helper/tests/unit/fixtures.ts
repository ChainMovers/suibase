import { mkdirSync, writeFileSync, symlinkSync, rmSync } from "node:fs";
import { mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

export interface FixtureWorkdirOptions {
  name: string;
  active?: boolean;
  withDemo?: boolean;
  withKeystore?: boolean;
  withState?: boolean;
  withClientYaml?: boolean;
}

export interface Fixture {
  root: string;
  workdirsPath: string;
  workdirPath(name: string): string;
  cleanup(): void;
}

export function makeFixture(workdirs: FixtureWorkdirOptions[] = []): Fixture {
  const root = mkdtempSync(join(tmpdir(), "suibase-test-"));
  const workdirsPath = join(root, "workdirs");
  mkdirSync(workdirsPath, { recursive: true });

  let activeTarget: string | undefined;

  for (const wd of workdirs) {
    const wdPath = join(workdirsPath, wd.name);
    mkdirSync(wdPath, { recursive: true });

    if (wd.withState !== false) {
      const statePath = join(wdPath, ".state");
      mkdirSync(statePath, { recursive: true });
      writeFileSync(join(statePath, "name"), `${wd.name}\n`);
      writeFileSync(
        join(statePath, "dns"),
        JSON.stringify({
          known: {
            "sb-1-ed25519": {
              address:
                "0x0fc530455ee4132b761ed82dab732990cb7af73e69cd6e719a2a5badeaed105b",
            },
            "sb-2-ed25519": {
              address:
                "0x1cf34ae2e006fbfa9cee6ae4703b1a6c4ef627ab22e92e226bc6975521d0d705",
            },
          },
        }),
      );
      writeFileSync(
        join(statePath, "links"),
        JSON.stringify({
          selection: { primary: 0, secondary: 0, n_links: 1 },
          links: [
            {
              id: 0,
              alias: wd.name,
              rpc: `http://localhost:9000/${wd.name}`,
              ws: `ws://localhost:9000/${wd.name}`,
            },
          ],
        }),
      );
    }

    if (wd.withKeystore) {
      const cfgPath = join(wdPath, "config");
      mkdirSync(cfgPath, { recursive: true });
      writeFileSync(join(cfgPath, "sui.keystore"), "[]");
    }

    if (wd.withClientYaml) {
      const cfgPath = join(wdPath, "config");
      mkdirSync(cfgPath, { recursive: true });
      writeFileSync(
        join(cfgPath, "client.yaml"),
        `---
keystore:
  File: ${cfgPath}/sui.keystore
envs:
  - alias: ${wd.name}
    rpc: "http://localhost:9000"
active_env: ${wd.name}
active_address: "0xabcd1234ef567890abcd1234ef567890abcd1234ef567890abcd1234ef567890"
`,
      );
    }

    if (wd.withDemo) {
      // Build a demo package publication snapshot at
      // published-data/demo/<uuid>/<timestamp>/{package-id.json,created-objects.json}
      // with a "most-recent" symlink that resolves to it.
      const pkgRoot = join(wdPath, "published-data", "demo");
      const uuid = "U123";
      const ts = "T456";
      const target = join(pkgRoot, uuid, ts);
      mkdirSync(target, { recursive: true });

      const packageId =
        "0xfeedfacefeedfacefeedfacefeedfacefeedfacefeedfacefeedfacefeedface";
      writeFileSync(
        join(target, "package-id.json"),
        `["${packageId}"]\n`,
      );
      writeFileSync(
        join(target, "created-objects.json"),
        JSON.stringify([
          {
            type: `${packageId}::Counter::Counter`,
            objectId:
              "0x1111111111111111111111111111111111111111111111111111111111111111",
          },
          {
            type: `${packageId}::Counter::Counter`,
            objectId:
              "0x2222222222222222222222222222222222222222222222222222222222222222",
          },
          {
            type: `${packageId}::Other::Thing`,
            objectId:
              "0x3333333333333333333333333333333333333333333333333333333333333333",
          },
        ]),
      );
      // most-recent symlink: relative for portability
      symlinkSync(`${uuid}/${ts}`, join(pkgRoot, "most-recent"));
    }

    if (wd.active) {
      activeTarget = wdPath;
    }
  }

  if (activeTarget) {
    symlinkSync(activeTarget, join(workdirsPath, "active"));
  }

  return {
    root,
    workdirsPath,
    workdirPath: (name: string) => join(workdirsPath, name),
    cleanup() {
      try {
        rmSync(root, { recursive: true, force: true });
      } catch {
        // ignore
      }
    },
  };
}
