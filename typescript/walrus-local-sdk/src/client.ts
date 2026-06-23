// Copyright (c) Suibase contributors
// SPDX-License-Identifier: Apache-2.0

/**
 * {@link WalrusLocalClient} — a localnet-only drop-in for `@mysten/walrus`.
 *
 * It **extends the real `WalrusClient`**, constructed against the localnet-deployed Walrus
 * package + a localnet Sui client, so every **on-chain** operation (deleteBlob, extendBlob,
 * read/writeBlobAttributes, storageCost, systemState, createStorage, registerBlob, …) is
 * inherited unchanged and works against localnet. Only the **node-talking** operations are
 * overridden to go through Suibase's nodeless `sb-local` HTTP server (which has no storage
 * nodes): `readBlob`, `writeBlob`, the quilt/file methods, and blob status.
 *
 * Because it IS a `WalrusClient`, the method signatures are identical to `@mysten/walrus`
 * — code written against localnet runs verbatim against testnet/mainnet (where you simply
 * construct the genuine `WalrusClient` instead). The handful of methods that are inherently
 * storage-node plumbing (sliver/encoding/upload-relay/step-flows) throw a clear `UNSUPPORTED`
 * error — they have no meaning on a nodeless localnet and no application calls them directly.
 */

import type { ClientWithCoreApi } from "@mysten/sui/client";
import { SuiJsonRpcClient } from "@mysten/sui/jsonRpc";
import type { Signer } from "@mysten/sui/cryptography";
import { Transaction } from "@mysten/sui/transactions";
import { BlobNotCertifiedError, WalrusBlob, WalrusClient, WalrusFile } from "@mysten/walrus";
import type {
  GetBlobMetadataOptions,
  GetSecondarySliverOptions,
  GetSliversOptions,
  GetStorageConfirmationOptions,
  GetVerifiedBlobStatusOptions,
  ReadBlobOptions,
  WriteBlobFlowOptions,
  WriteBlobOptions,
  WriteBlobToUploadRelayOptions,
  WriteEncodedBlobOptions,
  WriteEncodedBlobToNodesOptions,
  WriteFilesFlowOptions,
  WriteFilesOptions,
  WriteMetadataOptions,
  WriteQuiltOptions,
  WriteSliverOptions,
  WriteSliversToNodeOptions,
} from "@mysten/walrus";

import { WalrusLocalError } from "./errors.js";
import { DEFAULT_SUI_RPC_URL, resolveLocalnetConfig, type LocalnetOptions } from "./localnet.js";
import { SbLocalTransport } from "./sbLocal.js";

/** The `blobObject` shape `@mysten/walrus`'s `writeBlob` returns (the parsed Move `Blob` struct). */
interface BlobObject {
  id: string;
  registered_epoch: number;
  blob_id: string;
  size: string;
  encoding_type: number;
  certified_epoch: number | null;
  storage: { id: string; start_epoch: number; end_epoch: number; storage_size: string };
  deletable: boolean;
}

/** Options for {@link WalrusLocalClient}'s constructor (a localnet-defaulted subset of `WalrusClientConfig`). */
export interface WalrusLocalClientOptions extends LocalnetOptions {
  /** Per-request timeout for sb-local HTTP calls (ms). Defaults to 30000. */
  sbLocalTimeoutMs?: number;
  /** Inject a `fetch` for sb-local calls (tests). Defaults to global `fetch`. */
  fetch?: typeof fetch;
}

function unsupported(method: string): WalrusLocalError {
  return new WalrusLocalError(
    "UNSUPPORTED",
    `${method}() is storage-node plumbing with no meaning on a nodeless localnet — use ` +
      `writeBlob / writeFiles / readBlob / getFiles (or run against testnet with @mysten/walrus)`,
    { context: { method } },
  );
}

/**
 * Read blob bytes through sb-local, translating a not-found (sb-local 404 /
 * `BLOB_NOT_FOUND`) into the real `@mysten/walrus` {@link BlobNotCertifiedError} — the
 * exact class (a `RetryableWalrusClientError`) and message `@mysten/walrus.readBlob`
 * throws for an uncertified/nonexistent blob — so `instanceof` checks and retry-on-
 * retryable loops behave identically on localnet and testnet. Other sb-local failures
 * (quilt-patch not-found, bad-request, server-unreachable) pass through as
 * {@link WalrusLocalError}.
 */
async function readBlobMapped(sbLocal: SbLocalTransport, blobId: string): Promise<Uint8Array> {
  try {
    return await sbLocal.readBlob(blobId);
  } catch (err) {
    if (err instanceof WalrusLocalError && err.code === "BLOB_NOT_FOUND") {
      throw new BlobNotCertifiedError(`The specified blob ${blobId} is not certified.`, { cause: err });
    }
    throw err;
  }
}

export class WalrusLocalClient extends WalrusClient {
  /** sb-local HTTP transport (the nodeless aggregator+publisher). */
  private readonly sbLocal: SbLocalTransport;
  /** A JSON-RPC client for reading parsed on-chain object content (localnet fullnode). */
  private readonly rpc: SuiJsonRpcClient;
  /** The Sui client used for on-chain reads/execution (the one given to the parent). */
  private readonly suiClient: ClientWithCoreApi;

  constructor(options: WalrusLocalClientOptions = {}) {
    const resolved = resolveLocalnetConfig(options);
    super(resolved.walrusConfig);

    this.suiClient = resolved.suiClient;
    this.sbLocal = new SbLocalTransport(resolved.aggregatorUrl, {
      ...(options.fetch ? { fetch: options.fetch } : {}),
      ...(options.sbLocalTimeoutMs !== undefined ? { timeoutMs: options.sbLocalTimeoutMs } : {}),
    });
    this.rpc = new SuiJsonRpcClient({ url: options.suiRpcUrl ?? DEFAULT_SUI_RPC_URL, network: "localnet" });

    // `readBlob` and `getSecondarySliver` are instance arrow-properties on the parent (set
    // in its constructor, after super() ran), so they must be overridden by assignment here.
    this.readBlob = async ({ blobId }: ReadBlobOptions): Promise<Uint8Array> => {
      return readBlobMapped(this.sbLocal, blobId);
    };
    this.getSecondarySliver = async (_options: GetSecondarySliverOptions): Promise<never> => {
      throw unsupported("getSecondarySliver");
    };
  }

  /** The sb-local base URL this client reads/writes through. */
  getAggregatorUrl(): string {
    return this.sbLocal.baseUrl;
  }

  /** True iff sb-local is reachable. */
  async sbLocalReady(): Promise<boolean> {
    return this.sbLocal.status();
  }

  // ----- writeBlob (node-talking -> sb-local) ----------------------------

  /**
   * Store a blob through sb-local (nodeless), returning the same `{ blobId, blobObject }`
   * shape `@mysten/walrus.writeBlob` returns. The blob is created on-chain owned by
   * `owner ?? signer.toSuiAddress()` (so the signer can later delete/extend it), and any
   * `attributes` are written on-chain with the signer.
   */
  override async writeBlob({
    blob,
    deletable,
    epochs,
    signer,
    owner,
    attributes,
  }: WriteBlobOptions): Promise<{ blobId: string; blobObject: BlobObject }> {
    const signerAddress = signer.toSuiAddress();
    const ownerAddress = owner ?? signerAddress;
    const hasAttributes = !!attributes && Object.keys(attributes).length > 0;
    // Reject attributes+third-party-owner BEFORE storing (we can only set attributes if the
    // signer ends up owning the blob), so a rejected call does not mint a wasted blob.
    if (hasAttributes) this.assertSignerOwns(ownerAddress, signerAddress, "blob attributes");

    const wire = await this.sbLocal.storeBlob(blob, {
      epochs,
      deletable,
      sendObjectTo: ownerAddress,
    });
    const newlyCreated = "newlyCreated" in wire;
    const objectId = newlyCreated ? wire.newlyCreated.blobObject.id : wire.alreadyCertified.object;
    const blobId = newlyCreated ? wire.newlyCreated.blobObject.blobId : wire.alreadyCertified.blobId;

    // Attributes are applied only when a FRESH blob was minted. On content dedup
    // (alreadyCertified) sb-local returns the pre-existing object, which already carries its
    // metadata — re-emitting `add_metadata` would abort on-chain. (Re-storing identical
    // content with different attributes therefore keeps the original object's attributes; a
    // localnet dedup nuance, see the README.)
    if (newlyCreated && hasAttributes) {
      await this.applyAttributesToFreshBlob(objectId, attributes!, signer);
    }

    const blobObject = await this.fetchBlobObject(objectId);
    return { blobId, blobObject };
  }

  /**
   * Set attributes on a FRESHLY-minted blob, the way `@mysten/walrus.writeBlob` does at
   * register time (no prior read): passing the blob as a transaction object argument (not a
   * `blobObjectId`) skips the "read existing attributes" step (which would fail on a blob with
   * no metadata yet) and emits `add_metadata` + `insert_or_update_metadata_pair`. Signed by
   * `signer`, whose address is the tx sender and the blob's owner (callers ensure owner===signer).
   */
  private async applyAttributesToFreshBlob(
    objectId: string,
    attributes: Record<string, string | null>,
    signer: Signer,
  ): Promise<void> {
    const tx = new Transaction();
    tx.add(this.writeBlobAttributes({ blobObject: tx.object(objectId), attributes }));
    tx.setSenderIfNotSet(signer.toSuiAddress());
    const result = (await signer.signAndExecuteTransaction({
      transaction: tx,
      client: this.suiClient,
    })) as { Transaction?: { digest: string }; FailedTransaction?: { digest: string; status?: { error?: { message?: string } } } };
    if (result.FailedTransaction) {
      throw new WalrusLocalError(
        "INTERNAL",
        `writing blob attributes failed (${result.FailedTransaction.digest}): ${
          result.FailedTransaction.status?.error?.message ?? "unknown error"
        }`,
        { context: { objectId } },
      );
    }
    // Wait for finality so an immediate readBlobAttributes sees the new metadata field.
    if (result.Transaction?.digest) {
      await this.suiClient.core.waitForTransaction({ digest: result.Transaction.digest });
    }
  }

  /**
   * Attributes are written after sb-local has transferred the blob to its owner, signed by
   * `signer` — so `signer` must own it. On a real network `@mysten/walrus` writes attributes
   * inside the register PTB (before the transfer), so owner≠signer works there; on localnet
   * it does not, so reject it clearly instead of failing with an opaque on-chain error.
   */
  private assertSignerOwns(ownerAddress: string, signerAddress: string, what: string): void {
    if (ownerAddress !== signerAddress) {
      throw new WalrusLocalError(
        "UNSUPPORTED",
        `${what} with owner !== signer is not supported on localnet (attributes are applied ` +
          `after store by the signer, which must own the blob) — omit \`owner\` or set it to the signer's address`,
        { context: { owner: ownerAddress, signer: signerAddress } },
      );
    }
  }

  /** Read a Blob object's parsed fields from chain and flatten to the `@mysten/walrus` shape. */
  private async fetchBlobObject(objectId: string): Promise<BlobObject> {
    const obj = await this.rpc.getObject({ id: objectId, options: { showContent: true } });
    const content = obj.data?.content;
    if (!content || content.dataType !== "moveObject") {
      throw new WalrusLocalError("UNEXPECTED_RESPONSE", `Blob object ${objectId} has no readable content`, {
        context: { objectId },
      });
    }
    const f = content.fields as Record<string, unknown>;
    const storage = (f.storage as { fields: Record<string, unknown> }).fields;
    const idOf = (v: unknown): string => (v as { id: string }).id;
    const certified = f.certified_epoch;
    return {
      id: idOf(f.id),
      registered_epoch: Number(f.registered_epoch),
      blob_id: String(f.blob_id),
      size: String(f.size),
      encoding_type: Number(f.encoding_type),
      certified_epoch: certified === null || certified === undefined ? null : Number(certified),
      storage: {
        id: idOf(storage.id),
        start_epoch: Number(storage.start_epoch),
        end_epoch: Number(storage.end_epoch),
        storage_size: String(storage.storage_size),
      },
      deletable: Boolean(f.deletable),
    };
  }

  // ----- node-talking plumbing: unsupported on a nodeless localnet -------

  override async getSlivers(_options: GetSliversOptions): Promise<never> {
    throw unsupported("getSlivers");
  }
  override async getBlobMetadata(_options: GetBlobMetadataOptions): Promise<never> {
    throw unsupported("getBlobMetadata");
  }
  override async writeSliver(_options: WriteSliverOptions): Promise<never> {
    throw unsupported("writeSliver");
  }
  override async writeMetadataToNode(_options: WriteMetadataOptions): Promise<never> {
    throw unsupported("writeMetadataToNode");
  }
  override async getStorageConfirmationFromNode(_options: GetStorageConfirmationOptions): Promise<never> {
    throw unsupported("getStorageConfirmationFromNode");
  }
  // Override too: the parent fans out to getStorageConfirmationFromNode with a per-node
  // `.catch(() => null)`, which would otherwise mask the UNSUPPORTED throw into an all-null array.
  override async getStorageConfirmations(_options: unknown): Promise<never> {
    throw unsupported("getStorageConfirmations");
  }
  override async writeSliversToNode(_options: WriteSliversToNodeOptions): Promise<never> {
    throw unsupported("writeSliversToNode");
  }
  override async writeEncodedBlobToNodes(_options: WriteEncodedBlobToNodesOptions): Promise<never> {
    throw unsupported("writeEncodedBlobToNodes");
  }
  override async writeEncodedBlobToNode(_options: WriteEncodedBlobOptions): Promise<never> {
    throw unsupported("writeEncodedBlobToNode");
  }
  override async writeBlobToUploadRelay(_options: WriteBlobToUploadRelayOptions): Promise<never> {
    throw unsupported("writeBlobToUploadRelay");
  }
  override writeBlobFlow(_options: WriteBlobFlowOptions): never {
    throw unsupported("writeBlobFlow");
  }
  override writeFilesFlow(_options: WriteFilesFlowOptions): never {
    throw unsupported("writeFilesFlow");
  }

  // ----- quilts / files (node-talking -> sb-local) -----------------------

  /**
   * Store many named blobs as one quilt, returning the same shape `@mysten/walrus.writeQuilt`
   * does (`{ index: { patches }, blobId, blobObject }`). Routes through sb-local's quilt store.
   */
  override async writeQuilt({ blobs, ...options }: WriteQuiltOptions): Promise<{
    index: { patches: { patchId: string; endIndex: number; identifier: string; tags: Record<string, string>; startIndex: number }[] };
    blobId: string;
    blobObject: BlobObject;
  }> {
    const patches = blobs.map((b) => ({
      identifier: b.identifier,
      contents: b.contents,
      ...(b.tags ? { tags: b.tags } : {}),
    }));
    const { blobId, blobObject, storedPatches } = await this.storeQuiltViaLocal(patches, options);
    const tagsByIdent = new Map(blobs.map((b) => [b.identifier, b.tags ?? {}]));
    const indexPatches = storedPatches.map((sp) => ({
      patchId: sp.quiltPatchId,
      identifier: sp.identifier,
      startIndex: sp.range ? sp.range[0] : 0,
      endIndex: sp.range ? sp.range[1] : 0,
      tags: tagsByIdent.get(sp.identifier) ?? {},
    }));
    return { index: { patches: indexPatches }, blobId, blobObject };
  }

  /**
   * Store a set of {@link WalrusFile}s (packed into one quilt), returning one
   * `{ id, blobId, blobObject }` per file — same as `@mysten/walrus.writeFiles`. `id` is the
   * file's `QuiltPatchId`, `blobId` is the quilt blob id, `blobObject` is the quilt Blob.
   */
  override async writeFiles({ files, ...options }: WriteFilesOptions): Promise<
    { id: string; blobId: string; blobObject: BlobObject }[]
  > {
    const patches = await Promise.all(
      files.map(async (f, i) => {
        // Mirror @mysten/walrus.writeFiles: a file with no identifier is auto-named `file-${i}`.
        const identifier = (await f.getIdentifier()) || `file-${i}`;
        return { identifier, contents: await f.bytes(), tags: await f.getTags() };
      }),
    );
    const { blobId, blobObject, storedPatches } = await this.storeQuiltViaLocal(patches, options);
    return storedPatches.map((sp) => ({ id: sp.quiltPatchId, blobId, blobObject }));
  }

  /**
   * Read files by id (`QuiltPatchId`s and/or plain blob ids) — same as
   * `@mysten/walrus.getFiles`. Quilt patches are served by sb-local's quilt routes (identifier
   * + tags from the quilt index); a plain blob id yields a file with a null identifier.
   */
  override async getFiles({ ids }: { ids: string[] }): Promise<WalrusFile[]> {
    const patchInfo = new Map<string, Map<string, { identifier: string; tags: Record<string, string> }>>();
    const out: WalrusFile[] = [];
    for (const id of ids) {
      const cls = classifyWalrusId(id);
      if (cls.kind === "blob") {
        const bytes = await readBlobMapped(this.sbLocal, id);
        out.push(
          new WalrusFile({
            reader: { getIdentifier: async () => null, getTags: async () => ({}), getBytes: async () => bytes },
          }),
        );
        continue;
      }
      if (!patchInfo.has(cls.quiltId)) {
        const list = await this.sbLocal.listQuiltPatches(cls.quiltId);
        patchInfo.set(cls.quiltId, new Map(list.map((p) => [p.patch_id, { identifier: p.identifier, tags: p.tags }])));
      }
      const info = patchInfo.get(cls.quiltId)!.get(id);
      const patch = await this.sbLocal.readQuiltPatchById(id);
      const identifier = info?.identifier ?? patch.identifier;
      const tags = info?.tags ?? {};
      out.push(
        new WalrusFile({
          reader: { getIdentifier: async () => identifier, getTags: async () => tags, getBytes: async () => patch.data },
        }),
      );
    }
    return out;
  }

  /** Shared sb-local quilt store: store patches, mark the quilt blob, return its blobObject + per-patch ids. */
  private async storeQuiltViaLocal(
    patches: { identifier: string; contents: Uint8Array; tags?: Record<string, string> }[],
    options: { epochs: number; deletable: boolean; signer: Signer; owner?: string; attributes?: Record<string, string | null> },
  ): Promise<{ blobId: string; blobObject: BlobObject; storedPatches: { identifier: string; quiltPatchId: string; range?: [number, number] }[] }> {
    const signerAddress = options.signer.toSuiAddress();
    const ownerAddress = options.owner ?? signerAddress;
    const hasUserAttrs = !!options.attributes && Object.keys(options.attributes).length > 0;
    // Explicit quilt attributes require the signer to own the blob (they are written after
    // transfer) — reject owner≠signer early, before storing.
    if (hasUserAttrs) this.assertSignerOwns(ownerAddress, signerAddress, "quilt attributes");

    const wire = await this.sbLocal.storeQuilt(patches, {
      epochs: options.epochs,
      deletable: options.deletable,
      sendObjectTo: ownerAddress,
    });
    const bsr = wire.blobStoreResult;
    const newlyCreated = "newlyCreated" in bsr;
    const objectId = newlyCreated ? bsr.newlyCreated.blobObject.id : bsr.alreadyCertified.object;
    const blobId = newlyCreated ? bsr.newlyCreated.blobObject.blobId : bsr.alreadyCertified.blobId;

    // Mark a FRESH packed blob as a quilt on-chain (matching @mysten/walrus.writeQuilt's
    // `_walrusBlobType: "quilt"` attribute) + apply any user attributes. On dedup the existing
    // object already carries them, so we skip (re-emitting add_metadata would abort). For a
    // third-party owner the implicit marker is skipped (the quilt still works); explicit user
    // attributes were already rejected above.
    if (newlyCreated && ownerAddress === signerAddress) {
      await this.applyAttributesToFreshBlob(
        objectId,
        { _walrusBlobType: "quilt", ...(options.attributes ?? {}) },
        options.signer,
      );
    }
    const blobObject = await this.fetchBlobObject(objectId);
    return { blobId, blobObject, storedPatches: wire.storedQuiltBlobs };
  }

  /**
   * Get a {@link WalrusBlob} handle — same as `@mysten/walrus.getBlob`. It is backed by an
   * sb-local reader (the bytes/quilt index come from sb-local), so `.blobId()`, `.asFile()`,
   * and `.files()` work nodeless; `.exists()`/`.storedUntil()` use {@link getVerifiedBlobStatus}.
   */
  override async getBlob({ blobId }: { blobId: string }): Promise<WalrusBlob> {
    return new WalrusBlob({ reader: this.makeSbLocalReader(blobId), client: this });
  }

  /**
   * A reader satisfying the subset of `BlobReader` that {@link WalrusBlob} consumes
   * (`blobId`, the `FileReader` methods, and `getQuiltReader`), reading everything from
   * sb-local. `BlobReader` is not exported by `@mysten/walrus`, so this is structurally cast.
   */
  private makeSbLocalReader(blobId: string): never {
    const sbLocal = this.sbLocal;
    const reader = {
      blobId,
      getIdentifier: async (): Promise<string | null> => null,
      getTags: async (): Promise<Record<string, string>> => ({}),
      getBytes: async (): Promise<Uint8Array> => readBlobMapped(sbLocal, blobId),
      async getQuiltReader() {
        const list = await sbLocal.listQuiltPatches(blobId);
        const byPatchId = new Map(list.map((p) => [p.patch_id, p]));
        return {
          readIndex: async () =>
            list.map((p) => ({ patchId: p.patch_id, identifier: p.identifier, tags: p.tags })),
          readerForPatchId(patchId: string) {
            const info = byPatchId.get(patchId);
            return {
              getIdentifier: async (): Promise<string | null> => info?.identifier ?? null,
              getTags: async (): Promise<Record<string, string>> => info?.tags ?? {},
              getBytes: async (): Promise<Uint8Array> => (await sbLocal.readQuiltPatchById(patchId)).data,
            };
          },
        };
      },
    };
    return reader as unknown as never;
  }

  // ----- blob status (node quorum on testnet -> chain-derived here) ------

  /**
   * The blob's status — same `BlobStatus` union as `@mysten/walrus.getVerifiedBlobStatus`.
   * On testnet this is a storage-node quorum read; on the nodeless localnet it is derived
   * from the on-chain Blob (via sb-local), so it agrees with what testnet reports for the
   * same blob id. A permanent blob's `statusEvent` is a zero placeholder (localnet has no
   * Sui status event).
   */
  override async getVerifiedBlobStatus({ blobId }: GetVerifiedBlobStatusOptions) {
    const s = await this.sbLocal.blobStatus(blobId);
    if (!s.exists) return { type: "nonexistent" as const };
    const initialCertifiedEpoch = s.certifiedEpoch ?? null;
    const isCertified = initialCertifiedEpoch !== null;
    if (s.deletable) {
      return {
        type: "deletable" as const,
        deletableCounts: { count_deletable_total: 1, count_deletable_certified: isCertified ? 1 : 0 },
        initialCertifiedEpoch,
      };
    }
    return {
      type: "permanent" as const,
      endEpoch: s.endEpoch ?? 0,
      isCertified,
      statusEvent: { eventSeq: "0", txDigest: "11111111111111111111111111111111" },
      initialCertifiedEpoch,
      deletableCounts: { count_deletable_total: 0, count_deletable_certified: 0 },
    };
  }
}

/** Classify a Walrus id by its decoded byte length (32 = blob; longer = quilt patch). */
function classifyWalrusId(id: string): { kind: "blob" } | { kind: "quiltPatch"; quiltId: string } {
  const bytes = new Uint8Array(Buffer.from(id, "base64url"));
  if (bytes.length === 32) return { kind: "blob" };
  return { kind: "quiltPatch", quiltId: Buffer.from(bytes.slice(0, 32)).toString("base64url") };
}
