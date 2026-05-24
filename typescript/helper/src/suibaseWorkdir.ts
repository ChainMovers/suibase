import {
  existsSync,
  readFileSync,
  readlinkSync,
  realpathSync,
  statSync,
} from "node:fs";
import { join } from "node:path";

import { SuibaseError } from "./error.js";
import { normalizeObjectId } from "./objectId.js";
import type { SuibaseRoot } from "./suibaseRoot.js";
import { readTopLevelString } from "./yamlMini.js";

export class SuibaseWorkdir {
  private workdirName?: string;
  private workdirPath?: string;

  initFromExisting(root: SuibaseRoot, workdirName: string): void {
    if (!root.isInstalled()) {
      throw new SuibaseError(
        "NotInstalled",
        "Not installed. Need to run ~/suibase/install",
      );
    }
    if (workdirName.length === 0) {
      throw new SuibaseError(
        "WorkdirNameEmpty",
        "Invalid workdir name (empty string)",
      );
    }

    const initialPath = join(root.workdirsPath, workdirName);
    let resolved: string;
    try {
      resolved = realpathSync(initialPath);
    } catch {
      throw new SuibaseError(
        "WorkdirAccessError",
        "Path could not be accessed. Check suibase is selecting a valid active workdir",
        { path: initialPath },
      );
    }
    if (!existsSync(resolved)) {
      throw new SuibaseError(
        "WorkdirNotExists",
        "Workdir does not exist. Check suibase is selecting a valid active workdir",
        { path: resolved },
      );
    }

    // Read .state/name to get the canonical workdir name (handles "active").
    const namePath = join(resolved, ".state", "name");
    let nameContent: string;
    try {
      nameContent = readFileSync(namePath, "utf8").trim();
    } catch {
      throw new SuibaseError(
        "WorkdirAccessError",
        "Active workdir read of .state/name failed. Try to 'update' the workdir",
        { path: namePath },
      );
    }
    if (nameContent.length === 0) {
      throw new SuibaseError(
        "WorkdirStateNameNotSet",
        "Active workdir .state/name not set. Try to 'update' the workdir",
      );
    }

    this.workdirName = nameContent;
    this.workdirPath = resolved;
  }

  getName(): string {
    if (this.workdirName === undefined) {
      throw new SuibaseError(
        "WorkdirNameNotSet",
        "Workdir name not set. Internal error. Contact developer",
      );
    }
    return this.workdirName;
  }

  keystorePathname(root: SuibaseRoot): string {
    this.assertReady(root);
    const path = join(this.workdirPath!, "config", "sui.keystore");
    if (!existsSync(path)) {
      throw new SuibaseError(
        "SuibaseKeystoreNotExists",
        `Not finding keystore '${path}'. Run the sui client to create it?`,
        { path },
      );
    }
    return path;
  }

  packageObjectId(root: SuibaseRoot, packageName: string): string {
    const pathname = this.pathnamePublishedFile(
      root,
      packageName,
      "package-id",
      "json",
    );
    let raw: string;
    try {
      raw = readFileSync(pathname, "utf8");
    } catch (io_error) {
      throw new SuibaseError(
        "PublishedDataAccessError",
        `[code:1] Could not open published data '${pathname}'. Was the package '${packageName}' published with success?`,
        { package_name: packageName, path: pathname, io_error: String(io_error) },
      );
    }
    raw = raw.trim();
    if (!raw.startsWith('["') || !raw.endsWith('"]')) {
      throw new SuibaseError(
        "PackageIdJsonInvalidFormat",
        "Invalid package-id.json format",
        { path: pathname },
      );
    }
    const idHex = raw.slice(2, raw.length - 2);
    return normalizeObjectId(idHex, "PackageIdInvalidHex", { path: pathname });
  }

  publishedNewObjectIds(root: SuibaseRoot, objectType: string): string[] {
    // Validate the parameter format: "package::module::type", no whitespace-only fields.
    const parts = objectType.split("::");
    const names: string[] = [];
    for (const found of parts) {
      const trimmed = found.trim();
      if (trimmed.length === 0) {
        throw new SuibaseError(
          "ObjectTypeMissingField",
          "Invalid object_type parameter with missing field",
          { object_type: objectType },
        );
      }
      names.push(trimmed);
    }
    if (names.length !== 3) {
      throw new SuibaseError(
        "ObjectTypeInvalidFormat",
        "Invalid object_type parameter",
        { object_type: objectType },
      );
    }

    const pathname = this.pathnamePublishedFile(
      root,
      names[0]!,
      "created-objects",
      "json",
    );

    let text: string;
    try {
      text = readFileSync(pathname, "utf8");
    } catch {
      throw new SuibaseError(
        "PublishedNewObjectAccessError",
        `Could not open published new objects file '${pathname}'. Was the package published with success?`,
        { path: pathname },
      );
    }

    let top: unknown;
    try {
      top = JSON.parse(text);
    } catch {
      throw new SuibaseError(
        "PublishedNewObjectReadError",
        `Could not read published new objects file '${pathname}'`,
        { path: pathname },
      );
    }

    const objects: string[] = [];
    if (Array.isArray(top)) {
      for (const created of top) {
        if (created && typeof created === "object") {
          const obj = created as Record<string, unknown>;
          const typeStr = typeof obj.type === "string" ? obj.type : undefined;
          if (!typeStr) continue;
          const sub = typeStr.split("::");
          // TODO: check package id (sub[0]) once available.
          if (
            sub.length === 3 &&
            sub[1] === names[1] &&
            sub[2] === names[2]
          ) {
            const idStr =
              typeof obj.objectId === "string" ? obj.objectId : undefined;
            if (!idStr) continue;
            try {
              objects.push(
                normalizeObjectId(idStr, "PublishedNewObjectParseError", {
                  path: pathname,
                }),
              );
            } catch (e) {
              if (
                e instanceof SuibaseError &&
                e.code === "PublishedNewObjectParseError"
              ) {
                throw e;
              }
              throw new SuibaseError(
                "PublishedNewObjectParseError",
                `Could not parse published new objects id '${idStr}' from file '${pathname}'`,
                { path: pathname, id: idStr },
              );
            }
          }
        }
      }
    }
    return objects;
  }

  clientSuiAddress(root: SuibaseRoot, addressName: string): string {
    if (addressName.length === 0) {
      throw new SuibaseError(
        "AddressNameEmpty",
        "Invalid address name (empty string)",
      );
    }
    if (addressName === "active") {
      return this.getClientActiveAddress(root);
    }

    const pathname = this.pathnameState(root, "dns");
    let text: string;
    try {
      text = readFileSync(pathname, "utf8");
    } catch {
      throw new SuibaseError(
        "WorkdirStateDNSAccessFailed",
        `Could not open DNS (client address) file '${pathname}'. Try to 'update' the workdir`,
        { path: pathname },
      );
    }
    let top: unknown;
    try {
      top = JSON.parse(text);
    } catch {
      throw new SuibaseError(
        "WorkdirStateDNSReadError",
        `Could not read dns file '${pathname}'`,
        { path: pathname },
      );
    }
    if (top && typeof top === "object") {
      const known = (top as Record<string, unknown>).known;
      if (known && typeof known === "object") {
        const item = (known as Record<string, unknown>)[addressName];
        if (item && typeof item === "object") {
          const addr = (item as Record<string, unknown>).address;
          if (typeof addr === "string") {
            return normalizeObjectId(addr, "WorkdirStateDNSParseError", {
              path: pathname,
              address: addr,
            });
          }
        }
      }
    }
    throw new SuibaseError(
      "AddressNameNotFound",
      `Not finding address name '${addressName}'`,
      { address_name: addressName },
    );
  }

  rpcUrl(root: SuibaseRoot): string {
    return this.urlFromState(root, "rpc");
  }

  // ============ private ============

  private assertReady(root: SuibaseRoot): void {
    if (!root.isInstalled()) {
      throw new SuibaseError(
        "NotInstalled",
        "Not installed. Need to run ~/suibase/install",
      );
    }
    if (this.workdirName === undefined) {
      throw new SuibaseError(
        "WorkdirNameNotSet",
        "Workdir name not set. Internal error. Contact developer",
      );
    }
    if (this.workdirPath === undefined) {
      throw new SuibaseError(
        "WorkdirPathNotSet",
        "Workdir path not set. Internal error. Contact developer",
      );
    }
  }

  private pathnamePublishedFile(
    root: SuibaseRoot,
    packageName: string,
    fileName: string,
    extension: string,
  ): string {
    if (!root.isInstalled()) {
      throw new SuibaseError(
        "NotInstalled",
        "Not installed. Need to run ~/suibase/install",
      );
    }
    if (packageName.length === 0) {
      throw new SuibaseError(
        "PackageNameEmpty",
        "Invalid package name (empty string)",
      );
    }
    if (fileName.length === 0) {
      throw new SuibaseError(
        "FileNameEmpty",
        "Invalid file name (empty string)",
      );
    }
    if (this.workdirName === undefined) {
      throw new SuibaseError(
        "WorkdirNameNotSet",
        "Workdir name not set. Internal error. Contact developer",
      );
    }
    if (this.workdirPath === undefined) {
      throw new SuibaseError(
        "WorkdirPathNotSet",
        "Workdir path not set. Internal error. Contact developer",
      );
    }

    const publishedRoot = join(
      this.workdirPath,
      "published-data",
      packageName,
    );
    const mostRecent = join(publishedRoot, "most-recent");

    let symlinkTarget: string;
    try {
      symlinkTarget = readlinkSync(mostRecent);
    } catch {
      throw new SuibaseError(
        "PublishedDataAccessErrorSymlinkNotFound",
        `[code:3] Could not open published data '${mostRecent}'. Was the package '${packageName}' published with success?`,
        { package_name: packageName, path: mostRecent },
      );
    }

    let resolved: string;
    try {
      resolved = realpathSync(mostRecent);
    } catch {
      throw new SuibaseError(
        "PublishedDataAccessErrorInvalidSymlink",
        `[code:2] Could not open published data '${mostRecent}'. Symlink value is '${symlinkTarget}'. Was the package '${packageName}' published with success?`,
        {
          package_name: packageName,
          path: mostRecent,
          symlink_target: symlinkTarget,
        },
      );
    }

    if (!existsSync(resolved)) {
      throw new SuibaseError(
        "PublishedDataNotFound",
        `No published data found for package '${packageName}'.`,
        {
          package_name: packageName,
          workdir: this.workdirName,
          path: resolved,
        },
      );
    }

    return join(resolved, `${fileName}.${extension}`);
  }

  private pathnameState(root: SuibaseRoot, stateName: string): string {
    if (!root.isInstalled()) {
      throw new SuibaseError(
        "NotInstalled",
        "Not installed. Need to run ~/suibase/install",
      );
    }
    if (stateName.length === 0) {
      throw new SuibaseError(
        "StateNameEmpty",
        "Invalid state name (empty string)",
      );
    }
    if (this.workdirName === undefined) {
      throw new SuibaseError(
        "WorkdirNameNotSet",
        "Workdir name not set. Internal error. Contact developer",
      );
    }
    if (this.workdirPath === undefined) {
      throw new SuibaseError(
        "WorkdirPathNotSet",
        "Workdir path not set. Internal error. Contact developer",
      );
    }
    const stateDir = join(this.workdirPath, ".state");
    if (!existsSync(stateDir)) {
      throw new SuibaseError(
        "WorkdirInitializationIncomplete",
        `Workdir not fully initialized. Do '${this.workdirName} start' or '${this.workdirName} update'`,
        { workdir: this.workdirName },
      );
    }
    return join(stateDir, stateName);
  }

  private urlFromState(root: SuibaseRoot, urlFieldName: string): string {
    const pathname = this.pathnameState(root, "links");
    const workdirName = this.workdirName!;
    let text: string;
    try {
      text = readFileSync(pathname, "utf8");
    } catch {
      throw new SuibaseError(
        "WorkdirInitializationIncomplete",
        `Workdir not fully initialized. Do '${workdirName} start' or '${workdirName} update'`,
        { workdir: workdirName },
      );
    }
    let top: Record<string, unknown>;
    try {
      top = JSON.parse(text) as Record<string, unknown>;
    } catch {
      throw new SuibaseError(
        "WorkdirStateLinkReadError",
        `Could not read link file '${pathname}'`,
        { path: pathname },
      );
    }

    let linkId = 0;
    const selection = top.selection;
    if (selection && typeof selection === "object") {
      const primary = (selection as Record<string, unknown>).primary;
      if (typeof primary === "number" && Number.isInteger(primary)) {
        linkId = primary;
      }
    }

    const links = top.links;
    if (linkId === 0) {
      if (Array.isArray(links)) {
        if (links.length === 0) {
          throw new SuibaseError(
            "MissingLinkDefinition",
            "Missing link definition. Check suibase.yaml links section.",
          );
        }
        const first = links[0];
        if (first && typeof first === "object") {
          const val = (first as Record<string, unknown>)[urlFieldName];
          if (typeof val === "string") return val;
        }
      }
      throw new SuibaseError(
        "MissingLinkField",
        `Missing '${urlFieldName}' link field. May be a problem with the suibase.yaml link section (1).`,
        { url_field: urlFieldName },
      );
    }

    if (Array.isArray(links)) {
      if (links.length === 0) {
        throw new SuibaseError(
          "MissingAtLeastOneLinkDefinition",
          "Missing at least one link definition. Check suibase.yaml links section.",
        );
      }
      for (const link of links) {
        if (link && typeof link === "object") {
          const lid = (link as Record<string, unknown>).id;
          if (typeof lid === "number" && lid === linkId) {
            const val = (link as Record<string, unknown>)[urlFieldName];
            if (typeof val === "string") return val;
          }
        }
      }
    }
    throw new SuibaseError(
      "MissingLinkField",
      `Missing '${urlFieldName}' link field. May be a problem with the suibase.yaml link section (1).`,
      { url_field: urlFieldName },
    );
  }

  private getClientActiveAddress(root: SuibaseRoot): string {
    this.assertReady(root);
    const workdirName = this.workdirName!;
    const initialPath = join(this.workdirPath!, "config", "client.yaml");
    let resolved: string;
    try {
      resolved = realpathSync(initialPath);
    } catch {
      throw new SuibaseError(
        "ConfigAccessError",
        `Missing config.yaml. Did you do '${workdirName} start'?`,
        { workdir: workdirName },
      );
    }
    try {
      statSync(resolved);
    } catch {
      throw new SuibaseError(
        "ConfigAccessError",
        `Missing config.yaml. Did you do '${workdirName} start'?`,
        { workdir: workdirName },
      );
    }
    let text: string;
    try {
      text = readFileSync(resolved, "utf8");
    } catch {
      throw new SuibaseError(
        "ConfigReadError",
        `Access problem with config.yaml. Did you do '${workdirName} start' or run the sui client once?`,
        { workdir: workdirName },
      );
    }
    const active = readTopLevelString(text, "active_address");
    if (!active) {
      throw new SuibaseError(
        "ConfigActiveAddressParseError",
        "Unknown active address. Did you run the sui client at least once?",
        { address: "<missing>" },
      );
    }
    return normalizeObjectId(active, "ConfigActiveAddressParseError", {
      address: active,
    });
  }
}
