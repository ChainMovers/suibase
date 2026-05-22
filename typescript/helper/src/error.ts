export type SuibaseErrorCode =
  // API caller errors
  | "NotInstalled"
  | "WorkdirNotSelected"
  | "WorkdirAccessError"
  | "WorkdirNotExists"
  | "SuibaseKeystoreNotExists"
  | "PublishedDataNotFound"
  | "MissingLinkDefinition"
  | "MissingAtLeastOneLinkDefinition"
  | "MissingLinkField"
  | "ConfigAccessError"
  | "ConfigReadError"
  | "ConfigActiveAddressParseError"
  // Bad parameter errors
  | "WorkdirNameEmpty"
  | "PackageNameEmpty"
  | "AddressNameEmpty"
  | "ObjectTypeMissingField"
  | "ObjectTypeInvalidFormat"
  | "AddressNameNotFound"
  // Suibase filesystem related
  | "WorkdirStateNameAccessFailed"
  | "WorkdirStateDNSAccessFailed"
  | "WorkdirStateNameNotSet"
  | "PackageIdJsonInvalidFormat"
  | "PackageIdInvalidHex"
  | "PublishedNewObjectReadError"
  | "PublishedNewObjectParseError"
  | "WorkdirInitializationIncomplete"
  | "WorkdirStateDNSReadError"
  | "WorkdirStateDNSParseError"
  | "PublishedDataAccessError"
  | "PublishedDataAccessErrorInvalidSymlink"
  | "PublishedDataAccessErrorSymlinkNotFound"
  | "PublishedNewObjectAccessError"
  | "WorkdirStateLinkReadError"
  // Internal
  | "WorkdirNameNotSet"
  | "WorkdirPathNotSet"
  | "FileNameEmpty"
  | "StateNameEmpty";

export class SuibaseError extends Error {
  readonly code: SuibaseErrorCode;
  readonly context: Record<string, string>;

  constructor(
    code: SuibaseErrorCode,
    message: string,
    context: Record<string, string> = {},
  ) {
    super(`suibase: ${message}`);
    this.name = "SuibaseError";
    this.code = code;
    this.context = context;
  }
}
