# Security Notes For Agents

Retoom started from a code audit/security-hardening workflow after forking anySCP. Treat security regressions as high priority.

## Non-Negotiables

- Do not reintroduce telemetry or phone-home behavior.
- Do not add analytics, crash reporting, update pings, or remote diagnostics without explicit user approval.
- Do not log secrets.
- Do not expose passwords, private keys, S3 secrets, or R2 admin tokens to React state unless there is no viable backend-only design.
- Do not bypass SSH host key verification.
- Do not make destructive cloud operations single-click.

## Secrets

Secrets should use `src-tauri/src/vault/` and the OS keychain.

Sensitive values include:

- SSH passwords.
- SSH private key passphrases.
- S3 access keys and secret keys.
- R2 admin API tokens.
- Any future OAuth refresh token or API token.

Avoid adding these to:

- Zustand stores.
- Tauri event payloads.
- logs or tracing fields.
- error strings returned to the frontend.
- README examples.

## R2 Dashboard Guardrails

R2 management calls are security-sensitive because they can change bucket policy, CORS, lifecycle, and custom domains.

Keep these properties intact:

- Validate Cloudflare account IDs and zone IDs as 32-character hex IDs.
- Validate custom domains as DNS names.
- Percent-encode custom domain path segments.
- Reject non-object JSON policy roots and cap accepted JSON body size.
- Require typed confirmation for destructive operations.
- Keep R2 admin tokens in the keychain and delete them when a connection stops being an R2 provider.
- Preserve typed Cloudflare API errors so the frontend can show actionable messages.

## SFTP Transfer Guardrails

- Do not collapse multi-select downloads into a single destination path.
- Single-file downloads may use an explicit local path from Save As.
- Multi-downloads should use a local directory and each remote basename.
- Folder downloads should continue through the recursive transfer path.
- Avoid string-splitting local paths in TypeScript; pass paths to Rust as path data and let Rust handle platform separators.

## Logging

Use tracing sparingly and intentionally. Safe high-level fields include counts, provider type, and operation names. Avoid path/name leakage unless the user explicitly needs that diagnostic and the value is not sensitive.

Examples of safer fields:

- `file_count`
- `provider = "r2"`
- `operation = "delete_cors"`

Examples to avoid:

- full local paths.
- remote object keys.
- bucket names in broad diagnostics.
- token/account secrets.
