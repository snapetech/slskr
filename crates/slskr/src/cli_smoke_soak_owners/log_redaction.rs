use super::*;

pub(super) fn redact_query(query: &str) -> String {
    if query.is_empty() {
        "<empty>".to_owned()
    } else {
        format!("len{}", query.chars().count())
    }
}

pub(super) fn redact_path(path: &str) -> String {
    if path.is_empty() {
        "<empty>".to_owned()
    } else {
        format!("len{}", path.chars().count())
    }
}

pub(in crate::cli) fn redact_peer_text(value: &str) -> String {
    if value.is_empty() {
        "<empty>".to_owned()
    } else {
        format!("len{}", value.chars().count())
    }
}

pub(super) fn peer_close_reason(error: &str) -> &'static str {
    if error.contains("Connection reset by peer") {
        "connection reset by peer"
    } else if error.contains("unexpected end of file") || error.contains("unexpected end of input")
    {
        "unexpected eof"
    } else {
        "closed"
    }
}

pub(super) fn unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

#[derive(Default)]
pub(super) struct LiveSoakIndirectCloseLog {
    pub(super) total: u64,
    pub(super) connection_reset: u64,
    pub(super) unexpected_eof: u64,
    pub(super) timed_out: u64,
    pub(super) other: u64,
}

pub(super) fn log_live_soak_indirect_close(reason: &'static str) {
    const VERBOSE_LIMIT: u64 = 3;
    const SUMMARY_INTERVAL: u64 = 25;

    static CLOSE_LOG: OnceLock<Mutex<LiveSoakIndirectCloseLog>> = OnceLock::new();
    let mut log = CLOSE_LOG
        .get_or_init(|| Mutex::new(LiveSoakIndirectCloseLog::default()))
        .lock()
        .expect("live soak indirect close log poisoned");

    log.total += 1;
    match reason {
        "connection reset by peer" => log.connection_reset += 1,
        "unexpected eof" => log.unexpected_eof += 1,
        "response timed out" => log.timed_out += 1,
        _ => log.other += 1,
    }

    if log.total <= VERBOSE_LIMIT {
        println!("live soak indirect peer-message closed before response: {reason}");
    } else if log.total.is_multiple_of(SUMMARY_INTERVAL) {
        println!(
            "live soak indirect peer-message close summary total={} connection_reset={} unexpected_eof={} timed_out={} other={}",
            log.total, log.connection_reset, log.unexpected_eof, log.timed_out, log.other
        );
    }
}

pub(in crate::cli) fn scrub_socket_addr(address: SocketAddr) -> String {
    format!(
        "{}:{}",
        if address.is_ipv4() { "ipv4" } else { "ipv6" },
        address.port()
    )
}

pub(in crate::cli) fn redact_username(username: &str) -> String {
    if username.is_empty() {
        "<empty>".to_owned()
    } else {
        format!("len{}", username.chars().count())
    }
}

pub(in crate::cli) fn peer_address_ip_detail(
    address: &slskr_client::protocol::server::PeerAddress,
) -> Result<String, String> {
    if env_bool("SLSK_PEER_ADDRESS_SHOW_IP", false)? {
        Ok(format!(" ip={}", address.ip))
    } else {
        Ok(String::new())
    }
}
