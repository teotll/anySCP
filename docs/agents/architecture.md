# Architecture Notes For Agents

## Frontend

Frontend source lives under `src/`.

Important areas:

- `src/components/layout/` - app shell, unified tab bar, status bar.
- `src/components/sidebar/` - main navigation.
- `src/components/dashboard/` - saved hosts, groups, recent connections, connection editing.
- `src/components/terminal/` - SSH terminal UI and split panes.
- `src/components/explorer/` - shared file browser table, toolbar, context menus, and drop zone.
- `src/components/sftp/` - SFTP Explorer page, browser, session picker, and SFTP tabs.
- `src/components/s3/` - S3-compatible browser and connection dialog.
- `src/components/r2/` - Cloudflare R2 dashboard.
- `src/components/snippets/` - command snippets and quick insert UI.
- `src/components/port-forwarding/` - SSH tunnel UI.
- `src/components/transfers/` - transfer status UI.
- `src/stores/` - Zustand stores.
- `src/providers/` - filesystem provider implementations for shared browser behavior.
- `src/types/` - frontend IPC and domain types.

Prefer extending existing components and stores before adding new architectural layers.

## Backend

Rust source lives under `src-tauri/src/`.

Important areas:

- `lib.rs` - Tauri app setup, managed state, command registration.
- `db/` - SQLite migrations, persistence, and database commands.
- `ssh/` - SSH manager, sessions, handlers, key handling, and commands.
- `sftp/` - SFTP commands and transfer manager.
- `s3/` - S3-compatible commands and transfer manager.
- `r2/` - Cloudflare R2 API client, DTOs, validation, and commands.
- `vault/` - OS keychain integration.
- `portforward/` - SSH tunnel manager and commands.
- `import/` - SSH config parser and import commands.
- `snippets/` - snippet persistence.
- `telemetry.rs` - disabled telemetry surface.

For new Tauri commands, register them in `lib.rs` and keep TypeScript invoke payloads aligned with Rust serde naming.

## Current Product Surfaces

### SSH

SSH supports saved hosts, key/password auth, proxy jump, terminal panes, search, snippets, history, and port forwarding. Host key verification is part of the security posture and should not be bypassed for convenience.

### SFTP

The SFTP Explorer has two related surfaces:

- Sidebar Explorer page for selecting/opening SFTP sessions.
- Per-session Explorer tabs opened from SSH/session flows.

Both should remain coherent. Multi-select downloads are supported for files, folders, and mixed selections. Single-file Save As should preserve the full selected destination path.

### S3

S3-compatible browsing uses the shared Explorer UI where possible. Keep provider-specific capabilities behind capability flags instead of scattering provider checks through table components.

### Cloudflare R2

The R2 dashboard uses Cloudflare's management API in addition to S3-compatible object access. R2 account IDs and admin API tokens are connection metadata; admin tokens live in the keychain.

R2 commands use a shared reqwest client with:

- HTTPS only.
- Connect and request timeouts.
- Redirects disabled.
- Typed error variants for frontend handling.

