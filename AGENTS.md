# AGENTS.md

This file is the entry point for future Codex sessions in this repository. Read it before making changes.

## Project Snapshot

AnySCP is a Tauri v2 desktop app with a React/TypeScript frontend and Rust backend. This repository is the `teotll/anySCP` fork of the original `macnev2013/anySCP` project and is expected to diverge quickly.

Current fork direction:

- Local-first SSH, SFTP, S3-compatible storage, and Cloudflare R2 operations client.
- Telemetry disabled.
- SSH host key verification must stay intact.
- Credentials and admin tokens belong in the OS keychain, not frontend state.
- R2 dashboard is a first-class feature, not a minor S3 variant.

More context:

- [Architecture Notes](docs/agents/architecture.md)
- [Security Notes](docs/agents/security.md)
- [Workflow Notes](docs/agents/workflows.md)

## First Checks

Run these before editing unless the user explicitly asks for something narrower:

```bash
git status --short --branch
git remote -v
```

Expected normal remote:

```text
origin  https://github.com/teotll/anySCP.git
```

Do not assume the working tree is clean. Never revert user changes unless the user explicitly asks.

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
- Use `rg` for searching.
- Use `apply_patch` for manual edits.
- Keep changes scoped to the user request.
- Keep generated/build artifacts out of commits unless explicitly requested.
- Do not add telemetry, analytics, crash uploaders, or remote reporting.
- Do not log credentials, private keys, tokens, bucket object names, local paths, or remote paths unless there is an explicit user-facing reason and the data is not sensitive.

## Git And Commit Notes

Recent work has been committed directly on `main`. Confirm branch state before assuming that is still true.

If the user asks to push, push `main` to `origin` only after verifying the remote points to the fork.

Prefer concise commit messages that describe the behavior change, for example:

```text
Harden R2 dashboard review issues
Surface Cloudflare R2 API codes
Refresh README for fork direction
```

