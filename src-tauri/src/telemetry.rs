//! Telemetry is intentionally disabled.
//!
//! Keep this module as a no-op shim so existing call sites remain explicit
//! usage markers without starting a background worker or making network calls.

use serde_json::Value;

/// Initialize the telemetry background worker.
/// No-op: telemetry is disabled.
pub fn init() {
}

/// Send a telemetry event.  Non-blocking; safe to call from any thread or task.
/// No-op: telemetry is disabled.
pub fn capture(_event: &str, _properties: Value) {
}
