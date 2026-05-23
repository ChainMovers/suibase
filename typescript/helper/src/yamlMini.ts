// Minimal YAML reader for the specific fields Suibase needs to read.
// We do NOT implement a general YAML parser — only a top-level
// `key: value` lookup with optional double-quoted strings.
//
// This is sufficient for ~/.../config/client.yaml, where the only field
// the helper cares about is `active_address`.

export function readTopLevelString(
  yamlText: string,
  key: string,
): string | undefined {
  const re = new RegExp(`^\\s*${escapeRegExp(key)}\\s*:\\s*(.+?)\\s*$`, "m");
  const match = yamlText.match(re);
  if (!match || match[1] === undefined) return undefined;
  let raw = match[1].trim();
  // Strip an inline `# comment` if there is one (outside quotes).
  if (!raw.startsWith('"') && !raw.startsWith("'")) {
    const hashIdx = raw.indexOf("#");
    if (hashIdx >= 0) raw = raw.slice(0, hashIdx).trim();
  }
  // Strip surrounding quotes if present.
  if (
    (raw.startsWith('"') && raw.endsWith('"')) ||
    (raw.startsWith("'") && raw.endsWith("'"))
  ) {
    raw = raw.slice(1, -1);
  }
  return raw;
}

function escapeRegExp(s: string): string {
  return s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}
