# Retoom

**Remote tools for macOS.**

Retoom is a macOS-focused desktop app for SSH terminals, SFTP file browsing, S3-compatible storage, and Cloudflare R2 administration. It is built with Tauri v2, Rust, React, and TypeScript.

## Fork Status

Retoom was forked from [macnev2013/anySCP](https://github.com/macnev2013/anySCP) on **May 20, 2026**.

This fork keeps the MIT license and credits the original project, but it is expected to diverge quickly. The current direction is a local-first macOS operations client with stronger security defaults, revived SFTP explorer workflows, first-class Cloudflare R2 management, and future adapters for additional remote filesystems.

The GitHub repository now lives at [teotll/Retoom](https://github.com/teotll/Retoom).

## Platform Scope

Retoom is macOS-only by design.

The original anySCP project had cross-platform support, and some Windows or Linux behavior may still work because of that inherited code. That is incidental, not a current Retoom design goal.

## Rename Migration

On first launch, Retoom looks for legacy anySCP local state and migrates it only when the new Retoom database does not already exist. The migration copies `anyscp.db` into `retoom.db` and attempts to copy known SSH, S3, and R2 keychain entries into Retoom's new keychain namespace.

The legacy anySCP data is left in place. Retoom does not delete old databases or old keychain entries during migration.

## Overview

Retoom combines remote terminal, file, object-storage, and Cloudflare R2 workflows into one local desktop workspace. The main UI is a left sidebar plus unified tabs, so SSH sessions, SFTP explorers, S3 browsers, and the R2 dashboard feel like parts of one operations app rather than separate tools.

This fork is intentionally local-first:

- Telemetry is disabled.
- Credentials are stored in the operating system keychain.
- SSH host key verification is implemented for safer first-use and repeat connections.
- Cloud and remote operations are handled by the Rust backend instead of exposing secrets to the frontend.

## Features

### SSH Terminal

- SSH terminal sessions powered by xterm.js.
- Split terminal panes within a session.
- Searchable terminal output.
- Saved hosts with labels, notes, colors, groups, and environment metadata.
- Password and key-based authentication.
- SSH config import from `~/.ssh/config`.
- Host key verification support.
- Proxy jump / bastion host support.
- Connection history and recent connections.

### SFTP Explorer

- Sidebar Explorer page for choosing active SFTP sessions.
- Per-session SFTP browser tabs.
- Browse, create, rename, move, copy, delete, upload, and download remote files and folders.
- Multi-select support for bulk SFTP operations.
- Multi-select downloads for files, folders, and mixed selections.
- Recursive folder downloads through the transfer manager.
- Save-as paths are preserved for single-file downloads.
- Drag-and-drop uploads.
- Remote file editing through VS Code with re-upload on save.
- Transfer queue with progress, speed, ETA, and configurable concurrency.

### S3-Compatible Storage

- Browser for Amazon S3, MinIO, Cloudflare R2 S3 endpoints, Backblaze B2, Wasabi, DigitalOcean Spaces, and other compatible providers.
- Bucket and object navigation with the shared Explorer UI.
- Object upload, download, delete, folder-style prefixes, and bulk operations.
- Presigned URL generation.
- VS Code edit-and-upload workflow for objects.
- Transfer progress tracking.

### Cloudflare R2 Dashboard

- R2-aware connection metadata with account ID and admin API token storage.
- Bucket listing and bucket lifecycle operations.
- CORS policy viewing, editing, and deletion.
- Lifecycle policy viewing, editing, and deletion.
- Custom domain management with domain validation and typed destructive confirmations.
- Raw metrics view for account and bucket inspection.
- Hardened Cloudflare API client with HTTPS-only requests, timeouts, no redirects, and typed error handling.
- Cloudflare API error codes surfaced in the UI with inline connection-edit actions for credential and permission issues.

### Remote Filesystem Direction

Retoom will include adapters for additional remote filesystems over time. The exact providers are still to be confirmed, but new adapters should fit the existing Explorer/provider model instead of becoming isolated one-off screens.

### Connection Tools

- Saved SSH hosts and object-storage connections.
- OS keychain-backed credential storage.
- Color-coded groups.
- Snippets library with parameterized command templates.
- SSH local and remote port forwarding.
- Settings for transfer behavior and app preferences.

### Security And Privacy

- No telemetry collection in this fork.
- Secrets are kept out of frontend state where practical.
- Passwords, SSH keys, S3 secrets, and R2 admin tokens are stored in the OS keychain.
- R2 admin tokens are removed when an R2 connection is converted to a non-R2 provider.
- Tauri command inputs are validated for the newer R2 management surface.
- Cloudflare custom domain path segments are DNS-validated and percent-encoded before API calls.

## Building

### Prerequisites

- macOS
- [Node.js](https://nodejs.org) 18+
- [pnpm](https://pnpm.io)
- [Rust](https://rustup.rs) stable
- Platform-specific [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)

### Development

```bash
git clone https://github.com/teotll/Retoom.git
cd Retoom

pnpm install
pnpm tauri dev
```

### Production Build

```bash
pnpm tauri build
```

Retoom currently publishes macOS release artifacts through GitHub Releases. The in-app updater is disabled for now, so local and release builds do not require a Tauri updater signing key.

## Tech Stack

| Layer | Technology |
|-------|------------|
| Desktop runtime | Tauri v2 |
| Backend | Rust, Tokio, russh, russh-sftp, rust-s3, rusqlite, reqwest |
| Frontend | React 19, TypeScript, Tailwind CSS v4 |
| Terminal | xterm.js |
| State management | Zustand |
| Credential storage | macOS Keychain via the `keyring` crate |
| Database | SQLite |

## Project Structure

```text
src/                          # React frontend
  components/
    dashboard/                # Host cards, groups, connection editing, recent connections
    explorer/                 # Shared file browser table, toolbar, drop zone, context menus
    history/                  # Connection and activity history
    layout/                   # App shell, tab bar, status bar
    port-forwarding/          # SSH tunnel management UI
    quick-connect/            # Inline quick-connect controls
    r2/                       # Cloudflare R2 dashboard
    s3/                       # S3-compatible browser and connection dialog
    sftp/                     # SFTP explorer page, browser, path bar, session picker, session tabs
    settings/                 # Application settings
    sidebar/                  # Main navigation
    snippets/                 # Command snippet library and quick panel
    terminal/                 # SSH terminal, panes, search, disconnect overlay
    transfers/                # Transfer popover and transfer rows
  hooks/                      # SSH, SFTP, transfer, keyboard, debounce, and resize hooks
  providers/                  # Shared filesystem provider implementations
  stores/                     # Zustand stores for tabs, sessions, hosts, transfers, settings, snippets
  types/                      # TypeScript types for SSH, SFTP, S3, R2, explorer, layout
  utils/                      # Formatting, time, and snippet resolution helpers

src-tauri/src/                # Rust backend
  ai/                         # Local AI command surface placeholder
  db/                         # SQLite schema, migrations, persistence commands
  import/                     # SSH config import
  portforward/                # SSH tunnel manager and commands
  r2/                         # Cloudflare R2 API client, validation, and commands
  s3/                         # S3 sessions, commands, and transfer manager
  sftp/                       # SFTP commands and transfer manager
  snippets/                   # Snippet persistence
  ssh/                        # SSH manager, sessions, keys, handlers, commands
  telemetry.rs                # Disabled telemetry surface
  types/                      # Shared backend event, error, and session types
  vault/                      # macOS Keychain integration
  lib.rs                      # Tauri app setup and command registration
```

## Troubleshooting

### SSH Connections

- Verify host, port, username, credentials, and firewall access.
- If host key verification blocks a connection, inspect whether the server host key changed intentionally before trusting it.
- Check SSH key permissions and key format if authentication fails.

### SFTP Transfers

- Confirm the remote account has read/write permissions for the target path.
- For folder downloads, make sure the selected local destination has enough disk space.
- Check the transfer popover for failed items and retry details.

### S3 And R2

- Confirm endpoint, region, bucket name, access key, and secret key.
- For R2 dashboard actions, add the Cloudflare account ID and an admin API token to the connection.
- If Cloudflare returns a permission error, edit the connection and verify the token scopes.

### macOS

If macOS reports the app is damaged during local testing, remove quarantine metadata:

```bash
xattr -cr /Applications/Retoom.app
```

## License

This project is licensed under the MIT License. See [LICENSE](LICENSE) for details.

## Acknowledgments

- Forked from [macnev2013/anySCP](https://github.com/macnev2013/anySCP) on May 20, 2026.
- Built with [Tauri](https://tauri.app).
- SSH implementation powered by [russh](https://github.com/warp-tech/russh).
- Terminal emulation powered by [xterm.js](https://xtermjs.org).
- S3 support powered by [rust-s3](https://github.com/durch/rust-s3).
