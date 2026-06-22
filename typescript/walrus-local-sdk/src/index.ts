// Copyright (c) Suibase contributors
// SPDX-License-Identifier: Apache-2.0

/**
 * `@suibase/walrus-local`
 *
 * A localnet-only drop-in for `@mysten/walrus`. {@link WalrusLocalClient} **extends** the
 * real `WalrusClient`: it inherits every on-chain operation (delete/extend/attributes/
 * storageCost/systemState/…) against the localnet-deployed Walrus, and overrides only the
 * node-talking blob/quilt operations to use Suibase's nodeless `sb-local` HTTP server.
 *
 * The API and signatures are identical to `@mysten/walrus`, so code written against
 * localnet runs verbatim on testnet/mainnet — there you construct the genuine
 * `WalrusClient` (with `network: 'testnet'`) instead. Use `@mysten/walrus`'s own
 * `WalrusFile` for the quilt/file APIs.
 *
 * @example
 * ```ts
 * import { WalrusLocalClient } from "@suibase/walrus-local";
 *
 * const client = new WalrusLocalClient();                 // localnet defaults
 * const { blobId, blobObject } = await client.writeBlob({ blob, deletable: true, epochs: 5, signer });
 * const bytes = await client.readBlob({ blobId });
 * await client.executeDeleteBlobTransaction({ blobObjectId: blobObject.id, signer }); // inherited, on-chain
 * ```
 */

export { WalrusLocalClient } from "./client.js";
export type { WalrusLocalClientOptions } from "./client.js";

export { WalrusLocalError, codeFromResponse } from "./errors.js";
export type { WalrusLocalErrorCode } from "./errors.js";

export {
  DEFAULT_SB_LOCAL_HOST,
  DEFAULT_SB_LOCAL_PORT,
  DEFAULT_SUI_RPC_URL,
  resolveLocalnetConfig,
} from "./localnet.js";
export type { LocalnetOptions, ResolvedLocalnet } from "./localnet.js";
