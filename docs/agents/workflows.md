# Workflow Notes For Agents

## State Checks

When branch, remote, or worktree state is unclear, check it before editing:

```bash
git status --short --branch
git remote -v
```

Prefer reading the surrounding implementation before making a change. Use the search tool that fits the agent environment:

- Codex terminal: `rg` / `rg --files`.
- Claude Code: `Grep` / `Glob`.
- Other agents: equivalent fast file and text search.

## Build And Test Matrix

Use the narrowest useful check first, then broaden if the change crosses boundaries.

| Change type | Minimum verification |
|-------------|----------------------|
| README/docs only | `git diff --check` |
| Frontend TypeScript/React | `pnpm build` and `git diff --check` |
| Rust backend | targeted `cargo test --manifest-path src-tauri/Cargo.toml <name>` when possible, then `cargo test --manifest-path src-tauri/Cargo.toml --locked` |
| Tauri IPC contract | `pnpm build`, Rust tests, and a payload/schema review |
| R2/security behavior | Rust tests plus frontend build if UI or serialized errors changed |

## Local Development

Frontend-only dev server:

```bash
pnpm dev
```

Tauri app:

```bash
pnpm tauri dev
```

Local SSH server for manual SSH/SFTP testing:

```bash
make ssh-up
ssh -p 2222 testuser@localhost
```

Default test credentials from the Makefile:

```text
host: localhost
port: 2222
user: testuser
password: testpass
```

Stop the container:

```bash
make ssh-down
```

## Tauri Build Signing

`pnpm tauri build` can fail when an updater public key exists but no private key is configured:

```text
A public key has been found, but no private key.
```

For signed releases, set `TAURI_SIGNING_PRIVATE_KEY`. For local unsigned builds, adjust updater signing config rather than working around the error in application code.

## Frontend Notes

- Follow the existing dense operations-app style.
- Use lucide icons where icon buttons are needed.
- Avoid marketing-page patterns inside the app.
- Keep long text from breaking compact controls.
- Reuse `src/components/explorer/` for file-browser behavior where possible.
- Keep provider-specific branching close to provider capability boundaries.

## Backend Notes

- Keep long-running transfer work in managers, not Tauri command glue.
- Keep validation near external API command boundaries.
- Prefer typed errors over stringly errors when frontend behavior depends on the failure mode.
- For serde IPC, watch `camelCase` vs acronym casing (`minTls` vs `minTLS` was a prior issue).
- If adding database fields, add defensive migrations and tests for preservation behavior.

## Release Workflow

Release automation exists under `.github/workflows/release.yml` and builds on tags matching `v*`.

Before changing release behavior, inspect:

- `.github/workflows/release.yml`
- `src-tauri/tauri.conf.json`
- package/Cargo version metadata
- updater signing settings

## GitHub Repository Rename Checklist

The repo still uses `teotll/anySCP` until the GitHub repository is renamed. When it moves to the Retoom repo name, update these together:

- `README.md` clone instructions and current-repo note.
- `AGENTS.md` expected remote line.
- `package.json` repository URL.
- `src-tauri/Cargo.toml` repository and homepage URLs.
- `src-tauri/tauri.conf.json` updater endpoint.
- local git remote: `git remote set-url origin <new-url>`.
