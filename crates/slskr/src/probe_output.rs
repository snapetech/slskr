//! Structured probe output for automation and certification.
//!
//! When `SLSKR_PROBE_OUTPUT=json` is set, probe commands emit a single JSON
//! object on stdout with status, duration, and redacted detail fields.
//! Otherwise they use human-readable text via `println!`.

use serde::Serialize;
use std::time::Instant;

/// Probe result emitted at the end of every probe command.
#[derive(Debug, Clone, Serialize)]
pub struct ProbeResult {
    pub probe: &'static str,
    pub status: ProbeStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub peer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host_override: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProbeStatus {
    Ok,
    Failed,
    Skipped,
}

/// Tracks probe execution timing and collects the result.
pub struct ProbeContext {
    pub name: &'static str,
    start: Instant,
    peer: Option<String>,
    bytes: Option<u64>,
    sha256: Option<String>,
    host_override: Option<bool>,
}

impl ProbeContext {
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            start: Instant::now(),
            peer: None,
            bytes: None,
            sha256: None,
            host_override: None,
        }
    }

    pub fn with_peer(mut self, peer: &str) -> Self {
        self.peer = Some(redact_username(peer));
        self
    }

    pub fn with_bytes(mut self, bytes: u64) -> Self {
        self.bytes = Some(bytes);
        self
    }

    pub fn with_sha256(mut self, sha256: &str) -> Self {
        self.sha256 = Some(sha256.to_owned());
        self
    }

    pub fn with_host_override(mut self, active: bool) -> Self {
        self.host_override = Some(active);
        self
    }

    pub fn ok(&self, detail: impl Into<String>) -> ProbeResult {
        ProbeResult {
            probe: self.name,
            status: ProbeStatus::Ok,
            duration_ms: Some(self.start.elapsed().as_millis() as u64),
            detail: Some(detail.into()),
            peer: self.peer.clone(),
            bytes: self.bytes,
            sha256: self.sha256.clone(),
            host_override: self.host_override,
        }
    }

    pub fn fail(&self, detail: impl Into<String>) -> ProbeResult {
        ProbeResult {
            probe: self.name,
            status: ProbeStatus::Failed,
            duration_ms: Some(self.start.elapsed().as_millis() as u64),
            detail: Some(detail.into()),
            peer: self.peer.clone(),
            bytes: self.bytes,
            sha256: self.sha256.clone(),
            host_override: self.host_override,
        }
    }
}

/// Returns `true` if probe commands should emit JSON output.
pub fn is_json_output() -> bool {
    matches!(
        std::env::var("SLSKR_PROBE_OUTPUT")
            .as_deref()
            .map(str::trim)
            .map(str::to_ascii_lowercase)
            .as_deref(),
        Ok("json")
    )
}

/// Emit a probe result to stdout. In JSON mode, outputs a single JSON object.
/// In text mode, outputs a human-readable line.
pub fn emit_result(result: &ProbeResult) {
    if is_json_output() {
        let json = serde_json::to_string(result).unwrap_or_else(|_| "{}".to_owned());
        println!("{json}");
    }
}

/// Emit the result and return Ok(()) or Err(String) based on status.
pub fn emit_and_result(result: ProbeResult) -> Result<(), String> {
    emit_result(&result);
    match result.status {
        ProbeStatus::Ok => Ok(()),
        ProbeStatus::Failed => Err(result.detail.unwrap_or_else(|| "probe failed".to_owned())),
        ProbeStatus::Skipped => Ok(()),
    }
}

fn redact_username(username: &str) -> String {
    if username.is_empty() {
        "<empty>".to_owned()
    } else {
        format!("len{}", username.chars().count())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_result_json_serialization() {
        let ctx = ProbeContext::new("test-probe")
            .with_peer("testuser")
            .with_bytes(1234)
            .with_sha256("abcd1234")
            .with_host_override(true);
        let result = ctx.ok("completed");
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("\"probe\":\"test-probe\""));
        assert!(json.contains("\"status\":\"ok\""));
        assert!(json.contains("\"peer\":\"len8\""));
        assert!(json.contains("\"bytes\":1234"));
        assert!(json.contains("\"sha256\":\"abcd1234\""));
        assert!(json.contains("\"host_override\":true"));
    }

    #[test]
    fn probe_result_fail_status() {
        let ctx = ProbeContext::new("test-probe").with_peer("user");
        let result = ctx.fail("connection refused");
        assert_eq!(result.status, ProbeStatus::Failed);
        assert_eq!(result.detail.as_deref(), Some("connection refused"));
    }

    #[test]
    fn probe_result_duration_is_present() {
        let ctx = ProbeContext::new("test-probe");
        std::thread::sleep(std::time::Duration::from_millis(5));
        let result = ctx.ok("done");
        assert!(result.duration_ms.unwrap_or(0) >= 4);
    }

    #[test]
    fn json_output_env_detection() {
        std::env::remove_var("SLSKR_PROBE_OUTPUT");
        assert!(!is_json_output());
    }
}
