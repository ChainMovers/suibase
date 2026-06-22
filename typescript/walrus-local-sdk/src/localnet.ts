// Copyright (c) Suibase contributors
// SPDX-License-Identifier: Apache-2.0

/**
 * Resolve the localnet Walrus deployment into a `@mysten/walrus` client config + the
 * sb-local HTTP endpoint, by reading the suibase workdir:
 *   - `<root>/workdirs/localnet/config/walrus-localnet.yaml` — the deploy descriptor
 *     (package id + system/staking/exchange object ids), and
 *   - `<root>/workdirs/localnet/suibase.yaml` — the sb-local host/port (defaults 45840).
 *
 * This is localnet-only by construction: there is no network switch (you use the genuine
 * `@mysten/walrus` directly for testnet/mainnet).
 */

import { existsSync, readFileSync } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";

import { SuiJsonRpcClient } from "@mysten/sui/jsonRpc";
import type { ClientWithCoreApi } from "@mysten/sui/client";
import type { WalrusClientConfig, WalrusPackageConfig } from "@mysten/walrus";

import { WalrusLocalError } from "./errors.js";

/** Direct fullnode RPC of a suibase localnet (the deploy + sb-local talk to 9000). */
export const DEFAULT_SUI_RPC_URL = "http://127.0.0.1:9000";
/** Default sb-local port — the localnet slot of the Walrus 458xx range. */
export const DEFAULT_SB_LOCAL_PORT = 45840;
/** Default sb-local host. sb-local binds the IPv4 loopback. */
export const DEFAULT_SB_LOCAL_HOST = "127.0.0.1";

/** Options shared by the resolver and the client constructor. */
export interface LocalnetOptions {
  /** Override the suibase install dir. Defaults to `~/suibase`. */
  suibaseRoot?: string;
  /** A Sui client to use (must point at the localnet fullnode). Defaults to a JSON-RPC client on `suiRpcUrl`. */
  suiClient?: ClientWithCoreApi;
  /** Localnet fullnode RPC URL. Defaults to `http://127.0.0.1:9000`. Ignored if `suiClient` is given. */
  suiRpcUrl?: string;
  /** sb-local base URL (aggregator+publisher). Defaults to the host/port in `suibase.yaml` (else `http://127.0.0.1:45840`). */
  aggregatorUrl?: string;
}

/** What {@link resolveLocalnetConfig} produces. */
export interface ResolvedLocalnet {
  /** The config to pass to `WalrusClient`'s constructor (packageConfig + suiClient). */
  walrusConfig: WalrusClientConfig;
  /** The sb-local base URL (no trailing slash). */
  aggregatorUrl: string;
  /** The deployed Walrus Move package id (`0x…`). */
  packageId: string;
  /** The localnet Sui client used. */
  suiClient: ClientWithCoreApi;
}

/** Minimal top-level `key: value` YAML reader (quotes + inline `# comment` stripped). */
function readTopLevelString(yamlText: string, key: string): string | undefined {
  const re = new RegExp(`^\\s*${escapeRegExp(key)}\\s*:\\s*(.+?)\\s*$`, "m");
  const match = yamlText.match(re);
  if (!match || match[1] === undefined) return undefined;
  let raw = match[1].trim();
  if (!raw.startsWith('"') && !raw.startsWith("'")) {
    const hashIdx = raw.indexOf("#");
    if (hashIdx >= 0) raw = raw.slice(0, hashIdx).trim();
  }
  if (
    (raw.startsWith('"') && raw.endsWith('"')) ||
    (raw.startsWith("'") && raw.endsWith("'"))
  ) {
    raw = raw.slice(1, -1);
  }
  return raw === "null" || raw === "" ? undefined : raw;
}

function escapeRegExp(s: string): string {
  return s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

/** Resolve the sb-local base URL from the workdir's `suibase.yaml` (or the defaults). */
function resolveAggregatorUrl(configDir: string): string {
  const yamlPath = join(configDir, "..", "suibase.yaml");
  let port = DEFAULT_SB_LOCAL_PORT;
  let host = DEFAULT_SB_LOCAL_HOST;
  if (existsSync(yamlPath)) {
    let text = "";
    try {
      text = readFileSync(yamlPath, "utf8");
    } catch {
      text = "";
    }
    const portStr = readTopLevelString(text, "sb_local_walrus_port");
    if (portStr !== undefined) {
      const parsed = Number.parseInt(portStr, 10);
      if (Number.isInteger(parsed) && parsed > 0 && parsed < 65536) port = parsed;
    }
    const hostStr = readTopLevelString(text, "sb_local_host_ip");
    if (hostStr !== undefined) host = hostStr === "localhost" ? "127.0.0.1" : hostStr;
  }
  return `http://${host}:${port}`;
}

/**
 * Read the localnet deploy descriptor + sb-local endpoint and build a `WalrusClient`
 * config. Throws {@link WalrusLocalError} `NOT_LOCALNET` if the descriptor is absent
 * (no walrus_local_enabled localnet has been regen'd).
 */
export function resolveLocalnetConfig(options: LocalnetOptions = {}): ResolvedLocalnet {
  const root = options.suibaseRoot ?? join(homedir(), "suibase");
  const configDir = join(root, "workdirs", "localnet", "config");
  const descriptorPath = join(configDir, "walrus-localnet.yaml");

  if (!existsSync(descriptorPath)) {
    throw new WalrusLocalError(
      "NOT_LOCALNET",
      `localnet Walrus descriptor not found at ${descriptorPath} — enable walrus_local_enabled ` +
        `in workdirs/localnet/suibase.yaml and run \`localnet regen\` first`,
      { context: { descriptorPath } },
    );
  }

  let text = "";
  try {
    text = readFileSync(descriptorPath, "utf8");
  } catch (cause) {
    throw new WalrusLocalError("NOT_LOCALNET", `could not read ${descriptorPath}`, {
      context: { descriptorPath },
      cause,
    });
  }

  const packageId = required(text, "package_id", descriptorPath);
  const systemObjectId = required(text, "system_object", descriptorPath);
  const stakingPoolId = required(text, "staking_object", descriptorPath);
  const exchangeObject = readTopLevelString(text, "exchange_object");

  const packageConfig: WalrusPackageConfig = {
    systemObjectId,
    stakingPoolId,
    ...(exchangeObject ? { exchangeIds: [exchangeObject] } : {}),
  };

  const suiRpcUrl = options.suiRpcUrl ?? DEFAULT_SUI_RPC_URL;
  const suiClient = options.suiClient ?? new SuiJsonRpcClient({ url: suiRpcUrl, network: "localnet" });
  const aggregatorUrl = (options.aggregatorUrl ?? resolveAggregatorUrl(configDir)).replace(/\/+$/, "");

  return {
    walrusConfig: {
      packageConfig,
      suiClient,
      // Storage nodes are never contacted (we override those paths), but set http so any
      // accidental node URL build does not assume TLS on localnet.
      storageNodeUrlScheme: "http",
    } as WalrusClientConfig,
    aggregatorUrl,
    packageId,
    suiClient,
  };
}

function required(text: string, key: string, path: string): string {
  const v = readTopLevelString(text, key);
  if (!v) {
    throw new WalrusLocalError(
      "NOT_LOCALNET",
      `localnet descriptor ${path} is missing required key '${key}' (re-run \`localnet regen\`)`,
      { context: { key, path } },
    );
  }
  return v;
}
