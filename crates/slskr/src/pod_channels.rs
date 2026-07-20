use std::{
    collections::{BTreeMap, HashSet},
    fs,
    io::Read,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

const MAX_MESSAGES: usize = 4_096;
const MAX_MESSAGES_PER_CHANNEL: usize = 100;
const MAX_STATE_BYTES: u64 = 4 * 1024 * 1024;
const MAX_POD_ID_BYTES: usize = 512;
const MAX_CHANNEL_ID_BYTES: usize = 512;
const MAX_PEER_ID_BYTES: usize = 512;
const MAX_BODY_BYTES: usize = 16 * 1024;
const MAX_SIGNATURE_BYTES: usize = 2 * 1024;
const MAX_MESSAGE_ID_BYTES: usize = 2 * 1024;
const MAX_SUPPORTED_UNIX_MILLIS: u64 = 253_402_300_799_999;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PodChannelMessage {
    pub message_id: String,
    pub pod_id: String,
    pub channel_id: String,
    pub sender_peer_id: String,
    pub body: String,
    pub timestamp_unix_ms: u64,
    pub signature: String,
    pub sig_version: u8,
}

#[derive(Debug, Deserialize, Serialize)]
struct PodChannelStateFile {
    version: u32,
    messages: Vec<PodChannelMessage>,
}

#[derive(Debug)]
pub struct PodChannelStore {
    messages: Vec<PodChannelMessage>,
    state_path: PathBuf,
}

impl PodChannelStore {
    pub fn load(state_dir: &Path) -> Result<Self, String> {
        let state_path = state_dir.join("pod-channel-messages.json");
        let messages = load_state(&state_path)?;
        Ok(Self {
            messages,
            state_path,
        })
    }

    #[cfg(test)]
    pub fn empty(state_dir: &Path) -> Self {
        Self {
            messages: Vec::new(),
            state_path: state_dir.join("pod-channel-messages.json"),
        }
    }

    pub fn list(
        &self,
        pod_id: &str,
        channel_id: &str,
        since: Option<u64>,
    ) -> Vec<PodChannelMessage> {
        self.messages
            .iter()
            .filter(|message| {
                message.pod_id == pod_id
                    && message.channel_id == channel_id
                    && since.is_none_or(|since| message.timestamp_unix_ms > since)
            })
            .cloned()
            .collect()
    }

    pub fn append(
        &mut self,
        pod_id: String,
        channel_id: String,
        sender_peer_id: String,
        body: String,
        signature: String,
        timestamp_unix_ms: u64,
    ) -> Result<PodChannelMessage, String> {
        validate_field("PodId", &pod_id, MAX_POD_ID_BYTES)?;
        validate_field("ChannelId", &channel_id, MAX_CHANNEL_ID_BYTES)?;
        validate_field("SenderPeerId", &sender_peer_id, MAX_PEER_ID_BYTES)?;
        validate_field("Message body", &body, MAX_BODY_BYTES)?;
        if signature.len() > MAX_SIGNATURE_BYTES {
            return Err(format!(
                "Signature must be at most {MAX_SIGNATURE_BYTES} bytes"
            ));
        }

        let timestamp_unix_ms = match self
            .messages
            .iter()
            .filter(|message| message.pod_id == pod_id && message.channel_id == channel_id)
            .map(|message| message.timestamp_unix_ms)
            .max()
        {
            Some(latest) if timestamp_unix_ms <= latest => latest
                .checked_add(1)
                .ok_or_else(|| "Pod channel message cursor space is exhausted".to_owned())?,
            _ => timestamp_unix_ms,
        };

        let message = PodChannelMessage {
            message_id: uuid::Uuid::new_v4().simple().to_string(),
            pod_id: pod_id.clone(),
            channel_id: channel_id.clone(),
            sender_peer_id,
            body,
            timestamp_unix_ms,
            signature,
            sig_version: 1,
        };
        let mut messages = self.messages.clone();
        messages.push(message.clone());
        let channel_count = messages
            .iter()
            .filter(|candidate| candidate.pod_id == pod_id && candidate.channel_id == channel_id)
            .count();
        if channel_count > MAX_MESSAGES_PER_CHANNEL {
            let remove_count = channel_count - MAX_MESSAGES_PER_CHANNEL;
            let mut removed = 0;
            messages.retain(|candidate| {
                let matches = candidate.pod_id == pod_id && candidate.channel_id == channel_id;
                if matches && removed < remove_count {
                    removed += 1;
                    false
                } else {
                    true
                }
            });
        }
        if messages.len() > MAX_MESSAGES {
            messages.drain(0..messages.len() - MAX_MESSAGES);
        }
        write_state(&self.state_path, &messages)?;
        self.messages = messages;
        Ok(message)
    }

    pub fn delete_pod(&mut self, pod_id: &str) -> Result<Vec<PodChannelMessage>, String> {
        self.delete_matching(|message| message.pod_id == pod_id)
    }

    pub fn delete_channels(
        &mut self,
        pod_id: &str,
        channel_ids: &HashSet<String>,
    ) -> Result<Vec<PodChannelMessage>, String> {
        self.delete_matching(|message| {
            message.pod_id == pod_id && channel_ids.contains(&message.channel_id)
        })
    }

    pub fn delete_older_than(
        &mut self,
        older_than_unix_ms: u64,
        pod_id: Option<&str>,
        channel_id: Option<&str>,
    ) -> Result<Vec<PodChannelMessage>, String> {
        self.delete_matching(|message| {
            message.timestamp_unix_ms < older_than_unix_ms
                && pod_id.is_none_or(|pod_id| message.pod_id == pod_id)
                && channel_id.is_none_or(|channel_id| message.channel_id == channel_id)
        })
    }

    fn delete_matching(
        &mut self,
        matches: impl Fn(&PodChannelMessage) -> bool,
    ) -> Result<Vec<PodChannelMessage>, String> {
        let mut messages = self.messages.clone();
        let mut removed = Vec::new();
        messages.retain(|message| {
            if matches(message) {
                removed.push(message.clone());
                false
            } else {
                true
            }
        });
        if removed.is_empty() {
            return Ok(removed);
        }
        write_state(&self.state_path, &messages)?;
        self.messages = messages;
        Ok(removed)
    }

    pub fn restore(&mut self, restored: Vec<PodChannelMessage>) -> Result<(), String> {
        if restored.is_empty() {
            return Ok(());
        }
        let mut messages = self.messages.clone();
        messages.extend(restored);
        messages.sort_by_key(|message| (message.timestamp_unix_ms, message.message_id.clone()));
        write_state(&self.state_path, &messages)?;
        self.messages = messages;
        Ok(())
    }
}

fn validate_field(name: &str, value: &str, maximum: usize) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("{name} is required"));
    }
    if value.len() > maximum {
        return Err(format!("{name} must be at most {maximum} bytes"));
    }
    Ok(())
}

fn load_state(path: &Path) -> Result<Vec<PodChannelMessage>, String> {
    #[cfg(not(unix))]
    {
        let metadata = match fs::symlink_metadata(path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(format!("pod channel state metadata failed: {error}")),
        };
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err("pod channel state path must be a regular file".to_owned());
        }
    }
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let file = match options.open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(format!("pod channel state open failed: {error}")),
    };
    let metadata = file
        .metadata()
        .map_err(|error| format!("pod channel state metadata failed: {error}"))?;
    if !metadata.is_file() {
        return Err("pod channel state path must be a regular file".to_owned());
    }
    if metadata.len() > MAX_STATE_BYTES {
        return Err(format!(
            "pod channel state exceeds the {MAX_STATE_BYTES} byte limit"
        ));
    }
    let mut body = String::new();
    file.take(MAX_STATE_BYTES + 1)
        .read_to_string(&mut body)
        .map_err(|error| format!("pod channel state read failed: {error}"))?;
    if body.len() as u64 > MAX_STATE_BYTES {
        return Err(format!(
            "pod channel state exceeds the {MAX_STATE_BYTES} byte limit"
        ));
    }
    let state = serde_json::from_str::<PodChannelStateFile>(&body)
        .map_err(|error| format!("pod channel state parse failed: {error}"))?;
    if state.version != 1 {
        return Err(format!(
            "unsupported pod channel state version: {}",
            state.version
        ));
    }
    let mut messages = state.messages;
    messages.retain(|message| {
        validate_field("MessageId", &message.message_id, MAX_MESSAGE_ID_BYTES).is_ok()
            && uuid::Uuid::parse_str(&message.message_id).is_ok()
            && validate_field("PodId", &message.pod_id, MAX_POD_ID_BYTES).is_ok()
            && validate_field("ChannelId", &message.channel_id, MAX_CHANNEL_ID_BYTES).is_ok()
            && validate_field("SenderPeerId", &message.sender_peer_id, MAX_PEER_ID_BYTES).is_ok()
            && validate_field("Message body", &message.body, MAX_BODY_BYTES).is_ok()
            && message.signature.len() <= MAX_SIGNATURE_BYTES
            && message.sig_version == 1
            && message.timestamp_unix_ms <= MAX_SUPPORTED_UNIX_MILLIS
    });
    messages.sort_by_key(|message| (message.timestamp_unix_ms, message.message_id.clone()));
    let mut message_ids = HashSet::new();
    messages.retain(|message| message_ids.insert(message.message_id.clone()));
    messages.reverse();
    let mut channel_counts = BTreeMap::<(String, String), usize>::new();
    messages.retain(|message| {
        let count = channel_counts
            .entry((message.pod_id.clone(), message.channel_id.clone()))
            .or_default();
        if *count >= MAX_MESSAGES_PER_CHANNEL {
            false
        } else {
            *count += 1;
            true
        }
    });
    messages.reverse();
    if messages.len() > MAX_MESSAGES {
        messages.drain(0..messages.len() - MAX_MESSAGES);
    }
    Ok(messages)
}

fn write_state(path: &Path, messages: &[PodChannelMessage]) -> Result<(), String> {
    let body = serde_json::to_vec_pretty(&PodChannelStateFile {
        version: 1,
        messages: messages.to_vec(),
    })
    .map_err(|error| format!("pod channel state encode failed: {error}"))?;
    super::write_file_atomic(path, body)
        .map_err(|error| format!("pod channel state write failed: {error}"))
}

#[cfg(test)]
mod tests {
    use super::PodChannelStore;

    #[test]
    fn messages_are_bounded_incremental_and_durable() {
        let state_dir = std::env::temp_dir().join(format!(
            "slskr-pod-channels-{}",
            uuid::Uuid::new_v4().simple()
        ));
        std::fs::create_dir_all(&state_dir).unwrap();
        let mut store = PodChannelStore::empty(&state_dir);
        for timestamp in 1..=105 {
            store
                .append(
                    "pod-1".to_owned(),
                    "chat".to_owned(),
                    "peer-1".to_owned(),
                    format!("message-{timestamp}"),
                    String::new(),
                    timestamp,
                )
                .unwrap();
        }
        assert_eq!(store.list("pod-1", "chat", None).len(), 100);
        assert_eq!(store.list("pod-1", "chat", Some(100)).len(), 5);

        let loaded = PodChannelStore::load(&state_dir).unwrap();
        let messages = loaded.list("pod-1", "chat", Some(104));
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].body, "message-105");
        assert!(!messages[0].message_id.is_empty());
        std::fs::remove_dir_all(state_dir).unwrap();
    }

    #[test]
    fn timestamps_remain_monotonic_when_the_clock_does_not_advance() {
        let state_dir = std::env::temp_dir().join(format!(
            "slskr-pod-channel-cursors-{}",
            uuid::Uuid::new_v4().simple()
        ));
        std::fs::create_dir_all(&state_dir).unwrap();
        let mut store = PodChannelStore::empty(&state_dir);
        let first = store
            .append(
                "pod-1".to_owned(),
                "chat".to_owned(),
                "peer-1".to_owned(),
                "first".to_owned(),
                String::new(),
                50,
            )
            .unwrap();
        let second = store
            .append(
                "pod-1".to_owned(),
                "chat".to_owned(),
                "peer-1".to_owned(),
                "second".to_owned(),
                String::new(),
                40,
            )
            .unwrap();
        assert_eq!(first.timestamp_unix_ms, 50);
        assert_eq!(second.timestamp_unix_ms, 51);
        assert_eq!(store.list("pod-1", "chat", Some(50)).len(), 1);
        std::fs::remove_dir_all(state_dir).unwrap();
    }

    #[test]
    fn pod_deletion_removes_only_its_messages_and_can_be_rolled_back() {
        let state_dir = std::env::temp_dir().join(format!(
            "slskr-pod-channel-delete-{}",
            uuid::Uuid::new_v4().simple()
        ));
        std::fs::create_dir_all(&state_dir).unwrap();
        let mut store = PodChannelStore::empty(&state_dir);
        for pod_id in ["pod-1", "pod-2"] {
            store
                .append(
                    pod_id.to_owned(),
                    "chat".to_owned(),
                    "peer-1".to_owned(),
                    format!("message-{pod_id}"),
                    String::new(),
                    1,
                )
                .unwrap();
        }

        let removed = store.delete_pod("pod-1").unwrap();
        assert_eq!(removed.len(), 1);
        assert!(store.list("pod-1", "chat", None).is_empty());
        assert_eq!(store.list("pod-2", "chat", None).len(), 1);

        store.restore(removed).unwrap();
        assert_eq!(store.list("pod-1", "chat", None).len(), 1);
        let loaded = PodChannelStore::load(&state_dir).unwrap();
        assert_eq!(loaded.list("pod-1", "chat", None).len(), 1);
        assert_eq!(loaded.list("pod-2", "chat", None).len(), 1);
        std::fs::remove_dir_all(state_dir).unwrap();
    }

    #[test]
    fn channel_deletion_removes_only_selected_channel_messages() {
        let state_dir = std::env::temp_dir().join(format!(
            "slskr-pod-channel-selection-{}",
            uuid::Uuid::new_v4().simple()
        ));
        std::fs::create_dir_all(&state_dir).unwrap();
        let mut store = PodChannelStore::empty(&state_dir);
        for channel_id in ["general", "private"] {
            store
                .append(
                    "pod-1".to_owned(),
                    channel_id.to_owned(),
                    "peer-1".to_owned(),
                    format!("message-{channel_id}"),
                    String::new(),
                    1,
                )
                .unwrap();
        }

        let removed = store
            .delete_channels("pod-1", &["private".to_owned()].into_iter().collect())
            .unwrap();
        assert_eq!(removed.len(), 1);
        assert!(store.list("pod-1", "private", None).is_empty());
        assert_eq!(store.list("pod-1", "general", None).len(), 1);
        let loaded = PodChannelStore::load(&state_dir).unwrap();
        assert!(loaded.list("pod-1", "private", None).is_empty());
        assert_eq!(loaded.list("pod-1", "general", None).len(), 1);
        std::fs::remove_dir_all(state_dir).unwrap();
    }
}
