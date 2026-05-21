# AGENTS.md

This file is the entry point for future Codex sessions in this repository. Read it before making changes.

## What The App Does

Retoom is an operations desktop app for people who manage remote machines and object storage from macOS. Users save SSH and object-storage connections, open SSH terminals, browse remote SFTP folders, move files, manage S3-compatible buckets, and administer Cloudflare R2 settings such as CORS, lifecycle rules, domains, and metrics.

The main interaction model is a left sidebar plus unified tabs. SSH sessions, SFTP explorers, S3 browsers, and the R2 dashboard should feel like parts of one local operations workspace rather than separate apps.

## Project Snapshot

Retoom is a Tauri v2 desktop app with a React/TypeScript frontend and Rust backend. This repository is currently the `teotll/anySCP` fork of the original `macnev2013/anySCP` project and is expected to diverge quickly. The fork date is May 20, 2026.

Current fork direction:

- Local-first SSH, SFTP, S3-compatible storage, and Cloudflare R2 operations client for macOS.
- Telemetry disabled.
- SSH host key verification must stay intact.
- Credentials and admin tokens belong in the OS keychain, not frontend state.
- R2 dashboard is a first-class feature, not a minor S3 variant.
- Additional remote filesystem adapters are expected, but providers are still to be confirmed.

More context:

- [Architecture Notes](docs/agents/architecture.md)
- [Security Notes](docs/agents/security.md)
- [Workflow Notes](docs/agents/workflows.md)

Retoom currently keeps GitHub URLs pointing at `teotll/anySCP` until the repository is renamed. Use the checklist in [Workflow Notes](docs/agents/workflows.md#github-repository-rename-checklist) when that happens.

## Repo State Checks

If branch, remote, or worktree state is unclear, check it before editing:

```bash
git status --short --branch
git remote -v
```

Expected normal remote:

```text
origin  https://github.com/teotll/anySCP.git
```

## Common Commands

Frontend build:

```bash
pnpm build
```

Rust tests:

```bash
cargo test --manifest-path src-tauri/Cargo.toml --locked
```

Tauri dev app:

```bash
pnpm tauri dev
```

Production build:

```bash
pnpm tauri build
```

Whitespace check before committing:

```bash
git diff --check
```

Local SSH test server:

```bash
make ssh-up
make ssh-down
```

## Verification Expectations

- Frontend-only changes: run `pnpm build`.
- Rust backend changes: run targeted tests first, then `cargo test --manifest-path src-tauri/Cargo.toml --locked` when feasible.
- IPC/schema changes: verify both TypeScript build and Rust tests.
- Security-sensitive changes: inspect logs/errors for secret exposure and update [Security Notes](docs/agents/security.md) if behavior changes.
- README or docs-only changes: run `git diff --check`.

Existing Vite warnings about dynamic imports and chunk size may appear during `pnpm build`; do not treat them as failures unless the build exits nonzero or a new warning is relevant to the change.

## Editing Rules

- Prefer existing patterns over new abstractions.
- Keep changes scoped to the user request.
- Never revert user changes unless the user explicitly asks.
- Keep generated/build artifacts out of commits unless explicitly requested.
- Do not add telemetry, analytics, crash uploaders, or remote reporting.
- Do not log credentials, private keys, tokens, object keys, local paths, or remote paths unless there is an explicit user-facing reason and the data is not sensitive.

## Agent Tooling

Use the equivalent tools for the agent environment you are running in:

- Codex: prefer shell `rg` / `rg --files` for search and `apply_patch` for manual file edits.
- Claude Code: use `Grep` / `Glob` for search and `Edit` / `MultiEdit` / `Write` for file edits.
- Other agents: use the nearest safe equivalents, and avoid destructive shell edits when a structured edit tool is available.

## Git And Commit Notes

If the user asks to push, push `main` to `origin` only after verifying the remote points to the fork.

Prefer concise commit messages that describe the behavior change, for example:

```text
Harden R2 dashboard review issues
Surface Cloudflare R2 API codes
Refresh README for fork direction
```
