<p align="center">
  <img src="screens/header.png" alt="AnySCP" width="100%"/>
</p>

<p align="center">
  <strong>A desktop client for SSH, SFTP, S3-compatible storage, and Cloudflare R2 administration</strong>
</p>

<p align="center">
  <a href="#features">Features</a> &bull;
  <a href="#screenshots">Screenshots</a> &bull;
  <a href="#building">Building</a> &bull;
  <a href="#project-structure">Project Structure</a> &bull;
  <a href="#license">License</a>
</p>

<p align="center">
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-green" alt="License MIT"/></a>
</p>

---

## Fork Status

This repository is a fork of the original [macnev2013/anySCP](https://github.com/macnev2013/anySCP) project. It keeps the same MIT license and core idea, but this fork is expected to diverge quickly.

The current direction is a more security-conscious operations client with stronger local-only defaults, revived SFTP explorer workflows, and first-class Cloudflare R2 management alongside SSH, SFTP, and S3-compatible storage.

---

<p align="center">
  <img src="screens/anyscp.gif" alt="AnySCP Demo" width="800"/>
</p>
<p align="center"><em>SSH terminals, SFTP file management, S3-compatible storage, and Cloudflare R2 tooling in one desktop app.</em></p>

---

## Overview

AnySCP is a Tauri v2 desktop application with a Rust backend and React frontend. It combines SSH terminal sessions, SFTP file browsing, S3-compatible object storage browsing, Cloudflare R2 account management, snippets, history, and SSH port forwarding in one local application.

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

## Screenshots

| Connection Manager | SSH Terminal |
|:--:|:--:|
| ![Connection Manager](screens/hosts.png) | ![SSH Terminal](screens/terminal.png) |
| *Organize servers with groups, colors, and tags* | *Split panes, search, and tabbed sessions* |

| File Explorer | Command Snippets |
|:--:|:--:|
| ![File Explorer](screens/explorer.png) | ![Snippets](screens/snippets.png) |
| *SFTP and S3-style browsing with drag-and-drop and context menus* | *Parameterized templates with quick insert* |

## Building

### Prerequisites

- [Node.js](https://nodejs.org) 18+
- [pnpm](https://pnpm.io)
- [Rust](https://rustup.rs) stable
- Platform-specific [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)

### Development

```bash
git clone https://github.com/teotll/anySCP.git
cd anySCP

pnpm install
pnpm tauri dev
```

### Production Build

```bash
pnpm tauri build
```

If a public update signing key is present in the Tauri config, release builds require the matching private key in `TAURI_SIGNING_PRIVATE_KEY`. For local unsigned testing, remove or adjust the updater signing configuration before building.

## Tech Stack

| Layer | Technology |
|-------|------------|
| Desktop runtime | Tauri v2 |
| Backend | Rust, Tokio, russh, russh-sftp, rust-s3, rusqlite, reqwest |
| Frontend | React 19, TypeScript, Tailwind CSS v4 |
| Terminal | xterm.js |
| State management | Zustand |
| Credential storage | OS keychain via the `keyring` crate |
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
  vault/                      # OS keychain integration
  lib.rs                      # Tauri app setup and command registration
```

## Troubleshooting

### Tauri Signing Key

If `pnpm tauri build` fails with:

```text
A public key has been found, but no private key.
```

set `TAURI_SIGNING_PRIVATE_KEY` for signed releases, or disable updater signing for local builds.

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
xattr -cr /Applications/anyscp.app
```

## License

This project is licensed under the MIT License. See [LICENSE](LICENSE) for details.

## Acknowledgments

- Forked from [macnev2013/anySCP](https://github.com/macnev2013/anySCP).
- Built with [Tauri](https://tauri.app).
- SSH implementation powered by [russh](https://github.com/warp-tech/russh).
- Terminal emulation powered by [xterm.js](https://xtermjs.org).
- S3 support powered by [rust-s3](https://github.com/durch/rust-s3).
