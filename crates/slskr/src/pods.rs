use std::{
    collections::{BTreeMap, HashSet},
    fs,
    io::Read,
    path::{Path, PathBuf},
};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;

const MAX_PODS: usize = 128;
const MAX_CHANNELS: usize = 64;
const MAX_MEMBERS: usize = 1_000;
const MAX_TAGS: usize = 64;
const MAX_BINDINGS: usize = 64;
const MAX_STATE_BYTES: u64 = 8 * 1024 * 1024;
const MAX_POD_ID_BYTES: usize = 512;
const MAX_NAME_BYTES: usize = 512;
const MAX_DESCRIPTION_BYTES: usize = 8 * 1024;
const MAX_PEER_ID_BYTES: usize = 512;
const MAX_PUBLIC_KEY_BYTES: usize = 2 * 1024;
const MAX_POLICY_BYTES: usize = 256 * 1024;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PodChannel {
    pub channel_id: String,
    #[serde(default = "default_zero")]
    pub kind: Value,
    pub name: String,
    #[serde(default)]
    pub binding_info: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalBinding {
    pub kind: String,
    #[serde(default = "default_readonly")]
    pub mode: String,
    pub identifier: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PodMember {
    pub peer_id: String,
    #[serde(default = "default_member_role")]
    pub role: String,
    #[serde(default)]
    pub is_banned: bool,
    #[serde(default)]
    pub public_key: Option<String>,
    #[serde(default)]
    pub joined_at: Option<String>,
    #[serde(default)]
    pub last_seen: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PodRecord {
    pub pod_id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default = "default_visibility")]
    pub visibility: Value,
    #[serde(default)]
    pub is_public: bool,
    #[serde(default = "default_max_members")]
    pub max_members: usize,
    #[serde(default)]
    pub allow_guests: bool,
    #[serde(default)]
    pub require_approval: bool,
    #[serde(default = "current_timestamp")]
    pub updated_at: String,
    #[serde(default)]
    pub focus_content_id: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub channels: Vec<PodChannel>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub members: Option<Vec<PodMember>>,
    #[serde(default)]
    pub external_bindings: Vec<ExternalBinding>,
    #[serde(default)]
    pub capabilities: Vec<Value>,
    #[serde(default)]
    pub private_service_policy: Option<Value>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct StoredPod {
    pod: PodRecord,
    members: Vec<PodMember>,
}

#[derive(Debug, Deserialize, Serialize)]
struct PodStateFile {
    version: u32,
    pods: Vec<StoredPod>,
}

#[derive(Debug)]
pub struct PodStore {
    pods: BTreeMap<String, StoredPod>,
    state_path: PathBuf,
}

impl PodStore {
    pub fn load(state_dir: &Path) -> Result<Self, String> {
        let state_path = state_dir.join("pods.json");
        let pods = load_state(&state_path)?;
        Ok(Self { pods, state_path })
    }

    #[cfg(test)]
    pub fn empty(state_dir: &Path) -> Self {
        Self {
            pods: BTreeMap::new(),
            state_path: state_dir.join("pods.json"),
        }
    }

    pub fn list_visible(&self, peer_id: Option<&str>) -> Vec<PodRecord> {
        self.pods
            .values()
            .filter(|stored| {
                stored.pod.is_public
                    || peer_id.is_some_and(|peer_id| {
                        stored.members.iter().any(|member| {
                            !member.is_banned && member.peer_id.eq_ignore_ascii_case(peer_id)
                        })
                    })
            })
            .map(|stored| public_pod(stored, false))
            .collect()
    }

    pub fn get(&self, pod_id: &str) -> Option<PodRecord> {
        self.pods
            .get(pod_id)
            .map(|stored| public_pod(stored, false))
    }

    pub fn members(&self, pod_id: &str) -> Option<Vec<PodMember>> {
        self.pods.get(pod_id).map(|stored| {
            stored
                .members
                .iter()
                .filter(|member| !member.is_banned)
                .cloned()
                .collect()
        })
    }

    pub fn channel_exists(&self, pod_id: &str, channel_id: &str) -> bool {
        self.pods.get(pod_id).is_some_and(|stored| {
            stored
                .pod
                .channels
                .iter()
                .any(|channel| channel.channel_id == channel_id)
        })
    }

    pub fn is_public(&self, pod_id: &str) -> bool {
        self.pods
            .get(pod_id)
            .is_some_and(|stored| stored.pod.is_public)
    }

    pub fn is_member(&self, pod_id: &str, peer_id: &str) -> bool {
        self.pods.get(pod_id).is_some_and(|stored| {
            stored
                .members
                .iter()
                .any(|member| !member.is_banned && member.peer_id.eq_ignore_ascii_case(peer_id))
        })
    }

    pub fn can_moderate(&self, pod_id: &str, peer_id: &str) -> bool {
        self.pods.get(pod_id).is_some_and(|stored| {
            stored.members.iter().any(|member| {
                !member.is_banned
                    && member.peer_id.eq_ignore_ascii_case(peer_id)
                    && matches!(member.role.as_str(), "owner" | "mod")
            })
        })
    }

    pub fn create(&mut self, mut pod: PodRecord, creator: String) -> Result<PodRecord, String> {
        normalize_pod(&mut pod)?;
        validate_peer_id(&creator)?;
        if self.pods.contains_key(&pod.pod_id) {
            return Err("Pod already exists".to_owned());
        }
        if self.pods.len() >= MAX_PODS {
            return Err("Pod capacity is full".to_owned());
        }
        if private_gateway_enabled(&pod)
            && pod
                .private_service_policy
                .as_ref()
                .and_then(|policy| policy.get("gatewayPeerId"))
                .and_then(Value::as_str)
                .is_none_or(|gateway| gateway != creator)
        {
            return Err(
                "When creating a VPN pod, RequestingPeerId must match GatewayPeerId".to_owned(),
            );
        }
        pod.members = None;
        let now = current_timestamp();
        pod.updated_at = now.clone();
        let stored = StoredPod {
            pod: pod.clone(),
            members: vec![PodMember {
                peer_id: creator,
                role: "owner".to_owned(),
                is_banned: false,
                public_key: None,
                joined_at: Some(now.clone()),
                last_seen: Some(now),
            }],
        };
        self.commit_change(|pods| {
            pods.insert(pod.pod_id.clone(), stored);
        })?;
        Ok(pod)
    }

    pub fn update(
        &mut self,
        pod_id: &str,
        mut pod: PodRecord,
    ) -> Result<Option<PodRecord>, String> {
        normalize_pod(&mut pod)?;
        if pod.pod_id != pod_id {
            return Err("PodId in URL must match PodId in body".to_owned());
        }
        let Some(previous) = self.pods.get(pod_id).cloned() else {
            return Ok(None);
        };
        pod.members = None;
        pod.updated_at = current_timestamp();
        let updated = pod.clone();
        self.commit_change(|pods| {
            pods.insert(
                pod_id.to_owned(),
                StoredPod {
                    pod,
                    members: previous.members,
                },
            );
        })?;
        Ok(Some(updated))
    }

    pub fn delete(&mut self, pod_id: &str) -> Result<bool, String> {
        if !self.pods.contains_key(pod_id) {
            return Ok(false);
        }
        self.commit_change(|pods| {
            pods.remove(pod_id);
        })?;
        Ok(true)
    }

    pub fn join(&mut self, pod_id: &str, peer_id: String) -> Result<Option<bool>, String> {
        validate_peer_id(&peer_id)?;
        let Some(stored) = self.pods.get(pod_id) else {
            return Ok(None);
        };
        if !stored.pod.is_public || stored.pod.require_approval {
            return Err("Pod requires membership approval".to_owned());
        }
        if stored
            .members
            .iter()
            .any(|member| member.is_banned && member.peer_id.eq_ignore_ascii_case(&peer_id))
        {
            return Err("Peer is banned from this pod".to_owned());
        }
        if stored
            .members
            .iter()
            .any(|member| member.peer_id.eq_ignore_ascii_case(&peer_id))
        {
            return Ok(Some(false));
        }
        if stored.members.len() >= stored.pod.max_members.min(MAX_MEMBERS) {
            return Err("Pod member capacity is full".to_owned());
        }
        let now = current_timestamp();
        self.commit_change(|pods| {
            if let Some(stored) = pods.get_mut(pod_id) {
                stored.members.push(PodMember {
                    peer_id,
                    role: "member".to_owned(),
                    is_banned: false,
                    public_key: None,
                    joined_at: Some(now.clone()),
                    last_seen: Some(now),
                });
                stored.pod.updated_at = current_timestamp();
            }
        })?;
        Ok(Some(true))
    }

    pub fn leave(&mut self, pod_id: &str, peer_id: &str) -> Result<Option<bool>, String> {
        let Some(stored) = self.pods.get(pod_id) else {
            return Ok(None);
        };
        if !stored
            .members
            .iter()
            .any(|member| !member.is_banned && member.peer_id.eq_ignore_ascii_case(peer_id))
        {
            return Ok(Some(false));
        }
        self.commit_change(|pods| {
            if let Some(stored) = pods.get_mut(pod_id) {
                stored
                    .members
                    .retain(|member| !member.peer_id.eq_ignore_ascii_case(peer_id));
                stored.pod.updated_at = current_timestamp();
            }
        })?;
        Ok(Some(true))
    }

    pub fn ban(&mut self, pod_id: &str, peer_id: &str) -> Result<Option<bool>, String> {
        validate_peer_id(peer_id)?;
        let Some(stored) = self.pods.get(pod_id) else {
            return Ok(None);
        };
        if !stored
            .members
            .iter()
            .any(|member| member.peer_id.eq_ignore_ascii_case(peer_id))
        {
            return Ok(Some(false));
        }
        self.commit_change(|pods| {
            if let Some(stored) = pods.get_mut(pod_id) {
                if let Some(member) = stored
                    .members
                    .iter_mut()
                    .find(|member| member.peer_id.eq_ignore_ascii_case(peer_id))
                {
                    member.is_banned = true;
                }
                stored.pod.updated_at = current_timestamp();
            }
        })?;
        Ok(Some(true))
    }

    pub fn bind_room(
        &mut self,
        pod_id: &str,
        channel_id: &str,
        room_name: String,
        mode: String,
    ) -> Result<Option<bool>, String> {
        validate_text("RoomName", &room_name, MAX_NAME_BYTES, false)?;
        if !matches!(mode.as_str(), "readonly" | "mirror") {
            return Err("Mode must be 'readonly' or 'mirror'".to_owned());
        }
        if !self.channel_exists(pod_id, channel_id) {
            return Ok(None);
        }
        self.commit_change(|pods| {
            if let Some(stored) = pods.get_mut(pod_id) {
                let previous_binding = stored
                    .pod
                    .channels
                    .iter()
                    .find(|channel| channel.channel_id == channel_id)
                    .and_then(|channel| channel.binding_info.as_deref())
                    .and_then(|value| value.strip_prefix("soulseek-room:"))
                    .map(str::to_owned);
                if let Some(channel) = stored
                    .pod
                    .channels
                    .iter_mut()
                    .find(|channel| channel.channel_id == channel_id)
                {
                    channel.binding_info = Some(format!("soulseek-room:{room_name}"));
                }
                if let Some(previous_binding) = previous_binding {
                    let is_still_referenced = stored.pod.channels.iter().any(|channel| {
                        channel.binding_info.as_deref()
                            == Some(format!("soulseek-room:{previous_binding}").as_str())
                    });
                    if !is_still_referenced {
                        stored.pod.external_bindings.retain(|binding| {
                            !(binding.kind == "soulseek-room"
                                && binding.identifier == previous_binding)
                        });
                    }
                }
                stored.pod.external_bindings.retain(|binding| {
                    !(binding.kind == "soulseek-room" && binding.identifier == room_name)
                });
                stored.pod.external_bindings.push(ExternalBinding {
                    kind: "soulseek-room".to_owned(),
                    mode,
                    identifier: room_name,
                });
                stored.pod.updated_at = current_timestamp();
            }
        })?;
        Ok(Some(true))
    }

    pub fn unbind_room(&mut self, pod_id: &str, channel_id: &str) -> Result<Option<bool>, String> {
        let Some(stored) = self.pods.get(pod_id) else {
            return Ok(None);
        };
        let Some(binding) = stored
            .pod
            .channels
            .iter()
            .find(|channel| channel.channel_id == channel_id)
            .and_then(|channel| channel.binding_info.as_deref())
            .and_then(|value| value.strip_prefix("soulseek-room:"))
            .map(str::to_owned)
        else {
            return Ok(Some(false));
        };
        self.commit_change(|pods| {
            if let Some(stored) = pods.get_mut(pod_id) {
                if let Some(channel) = stored
                    .pod
                    .channels
                    .iter_mut()
                    .find(|channel| channel.channel_id == channel_id)
                {
                    channel.binding_info = None;
                }
                stored.pod.external_bindings.retain(|candidate| {
                    !(candidate.kind == "soulseek-room" && candidate.identifier == binding)
                });
                stored.pod.updated_at = current_timestamp();
            }
        })?;
        Ok(Some(true))
    }

    fn commit_change(
        &mut self,
        mutate: impl FnOnce(&mut BTreeMap<String, StoredPod>),
    ) -> Result<(), String> {
        let mut pods = self.pods.clone();
        mutate(&mut pods);
        for stored in pods.values_mut() {
            normalize_pod(&mut stored.pod)?;
        }
        write_state(&self.state_path, &pods)?;
        self.pods = pods;
        Ok(())
    }
}

fn public_pod(stored: &StoredPod, include_members: bool) -> PodRecord {
    let mut pod = stored.pod.clone();
    pod.members = include_members.then(|| {
        stored
            .members
            .iter()
            .filter(|member| !member.is_banned)
            .cloned()
            .collect()
    });
    pod
}

fn normalize_pod(pod: &mut PodRecord) -> Result<(), String> {
    pod.pod_id = pod.pod_id.trim().to_owned();
    pod.name = pod.name.trim().to_owned();
    validate_text("PodId", &pod.pod_id, MAX_POD_ID_BYTES, false)?;
    validate_text("Name", &pod.name, MAX_NAME_BYTES, false)?;
    normalize_optional(&mut pod.description, MAX_DESCRIPTION_BYTES, "Description")?;
    normalize_optional(&mut pod.focus_content_id, MAX_NAME_BYTES, "FocusContentId")?;
    if pod.max_members == 0 || pod.max_members > MAX_MEMBERS {
        return Err(format!("MaxMembers must be between 1 and {MAX_MEMBERS}"));
    }
    if pod.channels.len() > MAX_CHANNELS {
        return Err(format!("Channel count exceeds {MAX_CHANNELS}"));
    }
    let mut channel_ids = HashSet::new();
    for channel in &mut pod.channels {
        channel.channel_id = channel.channel_id.trim().to_owned();
        channel.name = channel.name.trim().to_owned();
        validate_text("ChannelId", &channel.channel_id, MAX_NAME_BYTES, false)?;
        validate_text("Channel name", &channel.name, MAX_NAME_BYTES, false)?;
        normalize_optional(&mut channel.binding_info, MAX_NAME_BYTES, "BindingInfo")?;
        normalize_optional(
            &mut channel.description,
            MAX_DESCRIPTION_BYTES,
            "Description",
        )?;
        if !channel_ids.insert(channel.channel_id.clone()) {
            return Err("ChannelIds must be unique".to_owned());
        }
    }
    pod.tags = normalize_strings(
        std::mem::take(&mut pod.tags),
        MAX_TAGS,
        MAX_NAME_BYTES,
        "Tag",
    )?;
    if pod.external_bindings.len() > MAX_BINDINGS {
        return Err(format!("External binding count exceeds {MAX_BINDINGS}"));
    }
    for binding in &mut pod.external_bindings {
        binding.kind = binding.kind.trim().to_owned();
        binding.mode = binding.mode.trim().to_ascii_lowercase();
        binding.identifier = binding.identifier.trim().to_owned();
        validate_text("Binding kind", &binding.kind, MAX_NAME_BYTES, false)?;
        validate_text("Binding mode", &binding.mode, 32, false)?;
        validate_text(
            "Binding identifier",
            &binding.identifier,
            MAX_NAME_BYTES,
            false,
        )?;
    }
    if serde_json::to_vec(&pod.private_service_policy)
        .map_err(|error| format!("PrivateServicePolicy is invalid: {error}"))?
        .len()
        > MAX_POLICY_BYTES
    {
        return Err(format!(
            "PrivateServicePolicy exceeds {MAX_POLICY_BYTES} bytes"
        ));
    }
    pod.members = None;
    Ok(())
}

fn normalize_strings(
    values: Vec<String>,
    maximum_count: usize,
    maximum_bytes: usize,
    name: &str,
) -> Result<Vec<String>, String> {
    if values.len() > maximum_count {
        return Err(format!("{name} count exceeds {maximum_count}"));
    }
    let mut seen = HashSet::new();
    let mut normalized = Vec::new();
    for value in values {
        let value = value.trim().to_owned();
        if value.is_empty() {
            continue;
        }
        validate_text(name, &value, maximum_bytes, false)?;
        if seen.insert(value.clone()) {
            normalized.push(value);
        }
    }
    Ok(normalized)
}

fn normalize_optional(
    value: &mut Option<String>,
    maximum: usize,
    name: &str,
) -> Result<(), String> {
    let normalized = value.take().map(|value| value.trim().to_owned());
    *value = normalized.filter(|value| !value.is_empty());
    if let Some(value) = value {
        validate_text(name, value, maximum, true)?;
    }
    Ok(())
}

fn validate_peer_id(peer_id: &str) -> Result<(), String> {
    validate_text("PeerId", peer_id, MAX_PEER_ID_BYTES, false)
}

fn validate_text(name: &str, value: &str, maximum: usize, allow_empty: bool) -> Result<(), String> {
    if !allow_empty && value.trim().is_empty() {
        return Err(format!("{name} is required"));
    }
    if value.len() > maximum {
        return Err(format!("{name} must be at most {maximum} bytes"));
    }
    Ok(())
}

fn private_gateway_enabled(pod: &PodRecord) -> bool {
    pod.capabilities.iter().any(|capability| {
        capability.as_i64() == Some(0)
            || capability
                .as_str()
                .is_some_and(|value| value.eq_ignore_ascii_case("PrivateServiceGateway"))
    })
}

fn load_state(path: &Path) -> Result<BTreeMap<String, StoredPod>, String> {
    #[cfg(not(unix))]
    {
        let metadata = match fs::symlink_metadata(path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(BTreeMap::new());
            }
            Err(error) => return Err(format!("pod state metadata failed: {error}")),
        };
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err("pod state path must be a regular file".to_owned());
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
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(BTreeMap::new()),
        Err(error) => return Err(format!("pod state open failed: {error}")),
    };
    let metadata = file
        .metadata()
        .map_err(|error| format!("pod state metadata failed: {error}"))?;
    if !metadata.is_file() {
        return Err("pod state path must be a regular file".to_owned());
    }
    if metadata.len() > MAX_STATE_BYTES {
        return Err(format!(
            "pod state exceeds the {MAX_STATE_BYTES} byte limit"
        ));
    }
    let mut body = String::new();
    file.take(MAX_STATE_BYTES + 1)
        .read_to_string(&mut body)
        .map_err(|error| format!("pod state read failed: {error}"))?;
    if body.len() as u64 > MAX_STATE_BYTES {
        return Err(format!(
            "pod state exceeds the {MAX_STATE_BYTES} byte limit"
        ));
    }
    let state = serde_json::from_str::<PodStateFile>(&body)
        .map_err(|error| format!("pod state parse failed: {error}"))?;
    if state.version != 1 {
        return Err(format!("unsupported pod state version: {}", state.version));
    }
    let mut pods = BTreeMap::new();
    for mut stored in state.pods.into_iter().take(MAX_PODS) {
        if normalize_pod(&mut stored.pod).is_err() || pods.contains_key(&stored.pod.pod_id) {
            continue;
        }
        let mut peers = HashSet::new();
        stored.members.retain(|member| {
            !member.peer_id.trim().is_empty()
                && member.peer_id.len() <= MAX_PEER_ID_BYTES
                && member
                    .public_key
                    .as_ref()
                    .is_none_or(|key| key.len() <= MAX_PUBLIC_KEY_BYTES)
                && peers.insert(member.peer_id.to_ascii_lowercase())
        });
        stored
            .members
            .truncate(MAX_MEMBERS.min(stored.pod.max_members));
        pods.insert(stored.pod.pod_id.clone(), stored);
    }
    Ok(pods)
}

fn write_state(path: &Path, pods: &BTreeMap<String, StoredPod>) -> Result<(), String> {
    let body = serde_json::to_vec_pretty(&PodStateFile {
        version: 1,
        pods: pods.values().cloned().collect(),
    })
    .map_err(|error| format!("pod state encode failed: {error}"))?;
    super::write_file_atomic(path, body).map_err(|error| format!("pod state write failed: {error}"))
}

fn current_timestamp() -> String {
    Utc::now().to_rfc3339()
}

fn default_zero() -> Value {
    Value::from(0)
}

fn default_visibility() -> Value {
    Value::from(1)
}

fn default_max_members() -> usize {
    50
}

fn default_readonly() -> String {
    "readonly".to_owned()
}

fn default_member_role() -> String {
    "member".to_owned()
}

#[cfg(test)]
mod tests {
    use super::{PodRecord, PodStore};

    fn fixture() -> PodRecord {
        serde_json::from_value(serde_json::json!({
            "podId": "pod:test",
            "name": "Test Pod",
            "isPublic": true,
            "maxMembers": 3,
            "channels": [{ "channelId": "general", "kind": 0, "name": "General" }]
        }))
        .unwrap()
    }

    #[test]
    fn pod_crud_membership_and_restart_are_durable() {
        let state_dir =
            std::env::temp_dir().join(format!("slskr-pods-{}", uuid::Uuid::new_v4().simple()));
        std::fs::create_dir_all(&state_dir).unwrap();
        let mut store = PodStore::empty(&state_dir);
        store.create(fixture(), "owner".to_owned()).unwrap();
        assert!(store
            .join("pod:test", "member".to_owned())
            .unwrap()
            .unwrap());
        assert_eq!(store.members("pod:test").unwrap().len(), 2);
        assert!(store.ban("pod:test", "member").unwrap().unwrap());
        assert_eq!(store.members("pod:test").unwrap().len(), 1);
        assert!(store
            .bind_room(
                "pod:test",
                "general",
                "music".to_owned(),
                "mirror".to_owned()
            )
            .unwrap()
            .unwrap());

        let loaded = PodStore::load(&state_dir).unwrap();
        assert_eq!(loaded.list_visible(None).len(), 1);
        assert_eq!(loaded.members("pod:test").unwrap().len(), 1);
        assert_eq!(
            loaded.get("pod:test").unwrap().channels[0]
                .binding_info
                .as_deref(),
            Some("soulseek-room:music")
        );
        std::fs::remove_dir_all(state_dir).unwrap();
    }

    #[test]
    fn rebinding_a_channel_replaces_stale_external_bindings() {
        let state_dir = std::env::temp_dir().join(format!(
            "slskr-pod-rebinding-{}",
            uuid::Uuid::new_v4().simple()
        ));
        std::fs::create_dir_all(&state_dir).unwrap();
        let mut store = PodStore::empty(&state_dir);
        store.create(fixture(), "owner".to_owned()).unwrap();
        for index in 0..100 {
            assert!(store
                .bind_room(
                    "pod:test",
                    "general",
                    format!("music-{index}"),
                    "mirror".to_owned(),
                )
                .unwrap()
                .unwrap());
        }

        let loaded = PodStore::load(&state_dir).unwrap();
        let pod = loaded.get("pod:test").unwrap();
        assert_eq!(pod.external_bindings.len(), 1);
        assert_eq!(pod.external_bindings[0].identifier, "music-99");
        assert_eq!(
            pod.channels[0].binding_info.as_deref(),
            Some("soulseek-room:music-99")
        );
        std::fs::remove_dir_all(state_dir).unwrap();
    }
}
