// Sui ObjectID / SuiAddress are 32-byte hex strings prefixed with "0x".
// In their canonical form they are 66 characters total. Shorter values
// are accepted as input and left-padded with zeros — mirroring Rust's
// `ObjectID::from_hex_literal` behavior.

import { SuibaseError, type SuibaseErrorCode } from "./error.js";

const HEX_RE = /^[0-9a-fA-F]*$/;

export function normalizeObjectId(
  input: string,
  invalidCode: SuibaseErrorCode,
  context: Record<string, string> = {},
): string {
  if (!input.startsWith("0x") && !input.startsWith("0X")) {
    throw new SuibaseError(
      invalidCode,
      `Invalid object id hexadecimal '${input}'.`,
      { id: input, ...context },
    );
  }
  const hex = input.slice(2);
  if (hex.length === 0 || hex.length > 64 || !HEX_RE.test(hex)) {
    throw new SuibaseError(
      invalidCode,
      `Invalid object id hexadecimal '${input}'.`,
      { id: input, ...context },
    );
  }
  const padded = hex.padStart(64, "0").toLowerCase();
  return `0x${padded}`;
}
