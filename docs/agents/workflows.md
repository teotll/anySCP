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

## Releases And Updates

GitHub release automation is active for macOS tag builds. The in-app updater is intentionally disabled for now.

Do not re-enable updater signing casually. If Retoom adds in-app updates later, add a fresh updater key, restore the Tauri updater config, document the required GitHub secrets, and test a signed update path before tagging a release.

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
- updater posture if in-app updates are being reintroduced

## GitHub Repository

The repository is `teotll/Retoom`. If it is renamed again, update these together:

- `README.md` clone instructions and repository note.
- `AGENTS.md` expected remote line.
- `package.json` repository URL.
- `src-tauri/Cargo.toml` repository and homepage URLs.
- `src-tauri/tauri.conf.json` release/updater-related URLs, if any.
- local git remote.
