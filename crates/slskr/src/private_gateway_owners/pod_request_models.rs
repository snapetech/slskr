#[derive(Debug, serde::Deserialize)]
pub(super) struct PodIdRequest {
    #[serde(alias = "PodId")]
    pub(super) pod_id: String,
}

#[derive(Debug, serde::Deserialize)]
pub(super) struct PodMessageRequest {
    #[serde(alias = "PodId")]
    pub(super) pod_id: String,
    #[serde(alias = "ChannelId")]
    pub(super) channel_id: String,
    #[serde(alias = "Body")]
    pub(super) body: String,
    #[serde(default, alias = "Signature")]
    pub(super) signature: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub(super) struct PodControlMessage {
    #[serde(rename = "MessageId", alias = "messageId")]
    pub(super) message_id: String,
    #[serde(rename = "SenderPeerId", alias = "senderPeerId")]
    pub(super) sender_peer_id: String,
    #[serde(rename = "TimestampUnixMs", alias = "timestampUnixMs")]
    pub(super) timestamp_unix_ms: i64,
}

#[derive(Debug, serde::Deserialize)]
pub(super) struct PodMessagesRequest {
    #[serde(alias = "PodId")]
    pub(super) pod_id: String,
    #[serde(alias = "ChannelId")]
    pub(super) channel_id: String,
    #[serde(default, alias = "SinceTimestamp")]
    pub(super) since_timestamp: Option<i64>,
}
