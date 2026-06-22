// Shared test helpers (not a *.test.ts, so not run as a suite).
import { readFileSync } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";
import { Ed25519Keypair } from "@mysten/sui/keypairs/ed25519";

/** Load the active localnet keypair from the suibase keystore (for signing on-chain ops). */
export function loadLocalnetSigner(suibaseRoot = join(homedir(), "suibase")): Ed25519Keypair {
  const cfg = join(suibaseRoot, "workdirs", "localnet", "config");
  const ks: string[] = JSON.parse(readFileSync(join(cfg, "sui.keystore"), "utf8"));
  const active = readFileSync(join(cfg, "client.yaml"), "utf8").match(/active_address:\s*"?(0x[0-9a-f]+)"?/)?.[1];
  for (const entry of ks) {
    const bytes = new Uint8Array(Buffer.from(entry, "base64"));
    if (bytes[0] === 0) {
      const kp = Ed25519Keypair.fromSecretKey(bytes.slice(1));
      if (kp.toSuiAddress() === active) return kp;
    }
  }
  throw new Error(`could not load active ed25519 signer for ${active}`);
}

/**
 * The proven cross-environment fixture: this exact content yields this exact blob id on
 * localnet AND the real testnet publisher (n_shards=1000, walrus-core encoder). Identical
 * to rust/walrus-local-sdk/tests/common/mod.rs.
 */
export const FIXTURE_CONTENT = "walrus-local-sdk cross-environment blob_id fixture v1";
export const FIXTURE_BLOB_ID = "x37bth2QxQZBbjZS6F-6l9mU_-bp46CRfOo33IAwe2U";

export const utf8 = (s: string) => new TextEncoder().encode(s);
export const fromUtf8 = (b: Uint8Array) => new TextDecoder().decode(b);
