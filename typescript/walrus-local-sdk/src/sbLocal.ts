// Copyright (c) Suibase contributors
// SPDX-License-Identifier: Apache-2.0

/**
 * Thin HTTP transport to the `sb-local` server (the nodeless localnet Walrus
 * aggregator+publisher). Used by {@link WalrusLocalClient} to satisfy the node-talking
 * blob/quilt operations without storage nodes. Errors surface as {@link WalrusLocalError}
 * with codes derived from sb-local's wire error contract.
 */

import { codeFromResponse, WalrusLocalError } from "./errors.js";

/** sb-local's camelCase `Blob` move struct (as serialized over the wire). */
export interface WireBlobObject {
  id: string;
  registeredEpoch: number;
  blobId: string;
  size: number;
  encodingType: string;
  certifiedEpoch: number | null;
  storage: { id: string; startEpoch: number; endEpoch: number; storageSize: number };
  deletable: boolean;
}

/** sb-local's `BlobStoreResult` (externally tagged). */
export type WireBlobStoreResult =
  | {
      newlyCreated: {
        blobObject: WireBlobObject;
        resourceOperation: { registerFromScratch: { encodedLength: number; epochsAhead: number } };
        cost: number;
        sharedBlobObject?: string;
      };
    }
  | { alreadyCertified: { blobId: string; object: string; endEpoch: number } };

/** sb-local's `QuiltStoreResult`. */
export interface WireQuiltStoreResult {
  blobStoreResult: WireBlobStoreResult;
  storedQuiltBlobs: { identifier: string; quiltPatchId: string; range?: [number, number] }[];
}

/** sb-local's `QuiltPatchItem` (list endpoint; snake_case `patch_id`, matching the real daemon). */
export interface WireQuiltPatchItem {
  identifier: string;
  patch_id: string;
  tags: Record<string, string>;
}

export interface PublisherParams {
  epochs?: number;
  deletable?: boolean;
  sendObjectTo?: string;
  share?: boolean;
}

/** A quilt patch read back (bytes + identifier from the `X-Quilt-Patch-Identifier` header). */
export interface WireQuiltPatch {
  identifier: string;
  data: Uint8Array;
}

/** sb-local's localnet status probe (`GET /v1/blobs/{id}/status`), derived from chain. */
export interface WireBlobStatus {
  exists: boolean;
  deletable?: boolean;
  certifiedEpoch?: number | null;
  endEpoch?: number;
}

const DEFAULT_TIMEOUT_MS = 30_000;

/** HTTP client for one sb-local server. */
export class SbLocalTransport {
  readonly baseUrl: string;
  private readonly fetchImpl: typeof fetch;
  private readonly timeoutMs: number;

  constructor(baseUrl: string, options: { fetch?: typeof fetch; timeoutMs?: number } = {}) {
    this.baseUrl = baseUrl.replace(/\/+$/, "");
    const f = options.fetch ?? globalThis.fetch;
    this.fetchImpl = f === globalThis.fetch ? f.bind(globalThis) : f;
    this.timeoutMs = options.timeoutMs ?? DEFAULT_TIMEOUT_MS;
  }

  /** `GET /status` → true iff sb-local reports OK. Never throws. */
  async status(): Promise<boolean> {
    try {
      const r = await this.request("GET", "/status");
      return r.ok && (await r.text()).trim() === "OK";
    } catch {
      return false;
    }
  }

  /** `PUT /v1/blobs` — store bytes, returning the wire `BlobStoreResult`. */
  async storeBlob(bytes: Uint8Array, params: PublisherParams): Promise<WireBlobStoreResult> {
    const resp = await this.request("PUT", `/v1/blobs${this.query(params)}`, { body: bytes });
    return this.json<WireBlobStoreResult>(resp);
  }

  /** `GET /v1/blobs/{blobId}` — read bytes. */
  async readBlob(blobId: string): Promise<Uint8Array> {
    const resp = await this.request("GET", `/v1/blobs/${encodeURIComponent(blobId)}`);
    return this.bytes(resp);
  }

  /** `GET /v1/blobs/{blobId}/status` — the localnet status probe (derived from chain). */
  async blobStatus(blobId: string): Promise<WireBlobStatus> {
    const resp = await this.request("GET", `/v1/blobs/${encodeURIComponent(blobId)}/status`);
    return this.json<WireBlobStatus>(resp);
  }

  /** `PUT /v1/quilts` (multipart) — store the patches into one quilt. */
  async storeQuilt(
    patches: { identifier: string; contents: Uint8Array; tags?: Record<string, string> }[],
    params: PublisherParams,
  ): Promise<WireQuiltStoreResult> {
    const form = new FormData();
    const metadata: { identifier: string; tags: Record<string, string> }[] = [];
    for (const p of patches) {
      if (p.identifier === "_metadata") {
        throw new WalrusLocalError(
          "BAD_REQUEST",
          '"_metadata" is reserved and cannot be used as a quilt patch identifier',
        );
      }
      form.append(p.identifier, new Blob([p.contents]), p.identifier);
      if (p.tags && Object.keys(p.tags).length > 0) metadata.push({ identifier: p.identifier, tags: p.tags });
    }
    if (metadata.length > 0) form.append("_metadata", JSON.stringify(metadata));
    const resp = await this.request("PUT", `/v1/quilts${this.query(params)}`, { body: form });
    return this.json<WireQuiltStoreResult>(resp);
  }

  /** `GET /v1/blobs/by-quilt-patch-id/{id}` — read a quilt patch by its public id. */
  async readQuiltPatchById(quiltPatchId: string): Promise<WireQuiltPatch> {
    const resp = await this.request(
      "GET",
      `/v1/blobs/by-quilt-patch-id/${encodeURIComponent(quiltPatchId)}`,
    );
    return this.quiltPatch(resp);
  }

  /** `GET /v1/blobs/by-quilt-id/{quiltId}/{identifier}` — read a quilt patch by quilt id + identifier. */
  async readQuiltPatchByIdentifier(quiltId: string, identifier: string): Promise<WireQuiltPatch> {
    const resp = await this.request(
      "GET",
      `/v1/blobs/by-quilt-id/${encodeURIComponent(quiltId)}/${encodeURIComponent(identifier)}`,
    );
    return this.quiltPatch(resp, identifier);
  }

  /** `GET /v1/quilts/{quiltId}/patches` — list a quilt's patches. */
  async listQuiltPatches(quiltId: string): Promise<WireQuiltPatchItem[]> {
    const resp = await this.request("GET", `/v1/quilts/${encodeURIComponent(quiltId)}/patches`);
    return this.json<WireQuiltPatchItem[]>(resp);
  }

  // ----- internals -------------------------------------------------------

  private query(p: PublisherParams): string {
    if (p.sendObjectTo !== undefined && p.share) {
      throw new WalrusLocalError("BAD_REQUEST", "`sendObjectTo` and `share` are mutually exclusive");
    }
    const params = new URLSearchParams();
    if (p.epochs !== undefined) params.set("epochs", String(p.epochs));
    if (p.deletable) params.set("deletable", "true");
    if (p.sendObjectTo !== undefined) params.set("send_object_to", p.sendObjectTo);
    if (p.share) params.set("share", "true");
    const q = params.toString();
    return q ? `?${q}` : "";
  }

  private async request(
    method: string,
    path: string,
    init: { body?: Uint8Array | FormData; headers?: Record<string, string> } = {},
  ): Promise<Response> {
    const url = `${this.baseUrl}${path}`;
    const controller = this.timeoutMs > 0 ? new AbortController() : undefined;
    const timer = controller ? setTimeout(() => controller.abort(), this.timeoutMs) : undefined;
    try {
      return await this.fetchImpl(url, {
        method,
        ...(init.body !== undefined ? { body: init.body } : {}),
        ...(init.headers !== undefined ? { headers: init.headers } : {}),
        ...(controller ? { signal: controller.signal } : {}),
      });
    } catch (cause) {
      const aborted = cause instanceof Error && cause.name === "AbortError";
      throw new WalrusLocalError(
        "SERVER_UNREACHABLE",
        aborted
          ? `request to ${url} timed out after ${this.timeoutMs}ms`
          : `could not reach sb-local at ${url} (is a walrus_local_enabled localnet running?)`,
        { context: { url, method }, cause },
      );
    } finally {
      if (timer !== undefined) clearTimeout(timer);
    }
  }

  private async json<T>(resp: Response): Promise<T> {
    if (!resp.ok) throw await this.error(resp);
    try {
      return (await resp.json()) as T;
    } catch (cause) {
      throw new WalrusLocalError("UNEXPECTED_RESPONSE", "could not parse sb-local JSON response", {
        status: resp.status,
        cause,
      });
    }
  }

  private async bytes(resp: Response): Promise<Uint8Array> {
    if (!resp.ok) throw await this.error(resp);
    return new Uint8Array(await resp.arrayBuffer());
  }

  private async quiltPatch(resp: Response, fallbackIdentifier = ""): Promise<WireQuiltPatch> {
    if (!resp.ok) throw await this.error(resp);
    const identifier = resp.headers.get("x-quilt-patch-identifier") ?? fallbackIdentifier;
    return { identifier, data: new Uint8Array(await resp.arrayBuffer()) };
  }

  private async error(resp: Response): Promise<WalrusLocalError> {
    let reason: string | undefined;
    let message = `HTTP ${resp.status}`;
    try {
      const body = (await resp.json()) as { error?: { reason?: string; message?: string } };
      if (body.error?.reason) reason = body.error.reason;
      if (body.error?.message) message = body.error.message;
    } catch {
      // non-JSON error body
    }
    return new WalrusLocalError(codeFromResponse(resp.status, reason), message, {
      status: resp.status,
      context: { url: resp.url || this.baseUrl, ...(reason ? { reason } : {}) },
    });
  }
}
