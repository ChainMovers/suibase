# Plan: installer auto-adds `~/.local/bin` to PATH (instead of aborting)

**Status:** TODO — ready for an agent to implement. Self-contained; everything you need is here.

## Problem

`~/suibase/install` installs the user-facing commands (`localnet`, `lsui`, …) as symlinks
in `~/.local/bin`. That directory must be on the user's `PATH` or the commands are not
found. Today, when `~/.local/bin` is **not** on PATH, the installer:

1. Tries sourcing `~/.profile` (Ubuntu/Debian ship a snippet that adds `~/.local/bin` if
   the dir exists) — and if that fixes it, sets `EXIT_TERMINAL_INSTRUCTION=true` and
   proceeds.
2. Otherwise prints a bare message — **"Please add `$OS_DIR_ON_PATH` to your `$PATH`
   variable"** — with **no command to copy** — and **aborts** (`exit 1`).

So a first-time user (especially on macOS, where `~/.local/bin` is not on PATH by default
and `~/.profile` is not consulted by zsh) hits a dead end: an abort with no actionable
fix. The docs only link to a generic StackOverflow page.

This is the weakest option compared to how popular tools handle the exact same problem.

## Goal

Make the installer **fix the PATH for the user automatically** (rustup / `pipx ensurepath`
style): detect the user's shell, append the `export PATH` line to the correct profile file
(idempotently), tell the user to restart their shell, and **continue the install** instead
of aborting. This is the same `~/.local/bin` problem `pipx ensurepath` solves with one
command.

## Current code (the thing to change)

File: **`install`** (repo root; `#!/bin/bash`, ~213 lines).

Key pieces:
- `OS_DIR_ON_PATH="$HOME/.local/bin"` and `OS_DIR_ON_PATH_ALT="~/.local/bin"` (top).
- `is_local_bin_on_path()` — returns true if either form is in `:$PATH:`.
- `setup_local_bin_as_needed()` — creates the dir, returns early if already on PATH, tries
  sourcing `~/.profile`, and **otherwise prints the bare message and `exit 1`**. It also
  contains a **commented-out stub** that already sketches the macOS auto-append (appending
  to `.zprofile`/`.bash_profile`), disabled with the note *"affraid to enable it without
  testing"* — that stub is the seed of this task; replace it with a tested implementation.
- `EXIT_TERMINAL_INSTRUCTION` — a global already used to tell the user at the end of the
  install to close/reopen the terminal. Reuse it after a successful auto-append.

## Requirements / design

Implement an auto-append in `setup_local_bin_as_needed()` that runs only when
`is_local_bin_on_path` is false AND sourcing `~/.profile` did not fix it:

1. **Pick the target profile file** by detecting the shell (prefer `$SHELL` basename;
   fall back to `$ZSH_VERSION`/`$BASH_VERSION`):
   - **zsh** (macOS default since Catalina): append to `~/.zshrc` (interactive). Create it
     if absent.
   - **bash**: append to `~/.bashrc` on Linux; on macOS bash login shells read
     `~/.bash_profile` — append there if it exists, else `~/.bashrc`. Create if absent.
   - **unknown shell**: fall back to the current "print a (now copy-pasteable) command and
     abort" path — but print the EXACT `echo '…' >> <file>` command, not a generic link.
2. **Append the line idempotently** — do not add a duplicate if the file already contains a
   line adding `~/.local/bin` to PATH (grep guard). Line to add:
   `export PATH="$HOME/.local/bin:$PATH"`
3. **Inform + continue** — print what file was modified, set
   `EXIT_TERMINAL_INSTRUCTION=true` (so the end-of-install message tells the user to restart
   their shell), and `return` (do NOT `exit 1`). The symlinks were/will be created in
   `~/.local/bin` regardless; the user just needs a new shell.
4. **Respect `QUIET_OPT`** — route messages through `echo_info`.
5. **Never edit a profile the user did not consent to silently breaking** — only append a
   single, clearly-commented line (e.g. precede it with `# Added by Suibase installer`),
   and never rewrite existing content.

## Edge cases to cover

- Re-running `install` (idempotency — must not append twice).
- `~/.local/bin` already on PATH (no-op; existing early return).
- The `~/.profile` auto-fix path still works (don't regress it).
- macOS zsh (default) vs Linux bash (default).
- Login vs interactive shells — `.zshrc`/`.bashrc` covers the interactive terminal the user
  is most likely in; document the choice in a comment.
- No profile file exists yet (create it).
- Unknown/other shell (fish, etc.) — graceful fallback with an exact copy-paste command.

## Testing

- `scripts/tests/` — add/extend a test that exercises `setup_local_bin_as_needed` with a
  simulated empty PATH + a temp HOME, asserting (a) the correct profile file gets exactly
  one `~/.local/bin` line, (b) a second run does not duplicate it, (c) the function returns
  0 (does not abort). Keep it hermetic (temp HOME, no real profile edits).
- Manual matrix before merge: Linux bash (on/off PATH), macOS zsh (off PATH), fresh user,
  re-run idempotency, unknown shell fallback.

## Acceptance criteria

- A user whose `~/.local/bin` is not on PATH can run `~/suibase/install` and end up with a
  working install after restarting their shell — **no manual step, no abort**.
- Re-running the installer never duplicates the PATH line.
- Quiet mode stays quiet; the end-of-install "restart your terminal" notice fires.
- Existing on-PATH and `~/.profile`-fix paths are unchanged.

## Docs follow-up (same PR or a quick second)

Once the installer auto-fixes PATH, soften the install-page caveat
(`docs/src/how-to/install.md`): the manual `echo … >> ~/.bashrc / ~/.zshrc` snippet (added
as "Tier 1") becomes the rare fallback ("if the installer could not update your shell
profile, add it manually"), since the common case is now handled automatically.

## References (how popular tools solve the identical problem)

- **rustup** — installer edits the shell profiles for you and tells you to restart.
- **`pipx ensurepath`** — a dedicated command that adds `~/.local/bin` to PATH (same dir).
- **Homebrew** — prints the exact `echo '…' >> ~/.zprofile` command to paste.
- **deno / bun / uv** — installer auto-appends to the detected profile, prints which file.
