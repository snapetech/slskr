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
const MAX_REGISTERED_SERVICES: usize = 20;
const MAX_ALLOWED_DESTINATIONS: usize = 50;

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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SoulseekBinding {
    pub pod_id: String,
    pub channel_id: String,
    pub identifier: String,
    pub mode: String,
    pub kind: &'static str,
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
                            !member.is_banned && peer_ids_equal(&member.peer_id, peer_id)
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

    pub fn soulseek_binding(&self, pod_id: &str, channel_id: &str) -> Option<SoulseekBinding> {
        let stored = self.pods.get(pod_id)?;
        let binding_info = stored
            .pod
            .channels
            .iter()
            .find(|channel| channel.channel_id == channel_id)?
            .binding_info
            .as_deref()?;
        if let Some(identifier) = binding_info.strip_prefix("soulseek-dm:") {
            return (!identifier.trim().is_empty()).then(|| SoulseekBinding {
                pod_id: pod_id.to_owned(),
                channel_id: channel_id.to_owned(),
                identifier: identifier.trim().to_owned(),
                mode: "mirror".to_owned(),
                kind: "dm",
            });
        }
        let identifier = binding_info.strip_prefix("soulseek-room:")?.trim();
        if identifier.is_empty() {
            return None;
        }
        let mode = stored
            .pod
            .external_bindings
            .iter()
            .find(|binding| {
                binding.kind == "soulseek-room"
                    && binding.identifier.eq_ignore_ascii_case(identifier)
            })
            .map(|binding| binding.mode.clone())
            .unwrap_or_else(default_readonly);
        Some(SoulseekBinding {
            pod_id: pod_id.to_owned(),
            channel_id: channel_id.to_owned(),
            identifier: identifier.to_owned(),
            mode,
            kind: "room",
        })
    }

    pub fn room_bindings(&self, room_name: &str) -> Vec<SoulseekBinding> {
        self.pods
            .iter()
            .flat_map(|(pod_id, stored)| {
                stored
                    .pod
                    .channels
                    .iter()
                    .filter_map(move |channel| self.soulseek_binding(pod_id, &channel.channel_id))
            })
            .filter(|binding| {
                binding.kind == "room" && binding.identifier.eq_ignore_ascii_case(room_name.trim())
            })
            .collect()
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
                .any(|member| !member.is_banned && peer_ids_equal(&member.peer_id, peer_id))
        })
    }

    pub fn can_moderate(&self, pod_id: &str, peer_id: &str) -> bool {
        self.pods.get(pod_id).is_some_and(|stored| {
            stored.members.iter().any(|member| {
                !member.is_banned
                    && peer_ids_equal(&member.peer_id, peer_id)
                    && matches!(member.role.as_str(), "owner" | "mod")
            })
        })
    }

    pub fn gateway_peer_for_update(&self, pod_id: &str, proposed: &PodRecord) -> Option<String> {
        let existing = self.pods.get(pod_id)?;
        if private_gateway_enabled(&existing.pod) && proposed.private_service_policy.is_some() {
            return Some(
                gateway_peer_id(&existing.pod)
                    .unwrap_or_default()
                    .to_owned(),
            );
        }
        private_gateway_enabled(proposed)
            .then(|| gateway_peer_id(proposed).unwrap_or_default().to_owned())
    }

    pub fn destination_allowed(&self, pod_id: &str, host: &str, port: u16) -> bool {
        self.pods.get(pod_id).is_some_and(|stored| {
            allowed_destinations(&stored.pod).is_some_and(|destinations| {
                destinations.iter().any(|destination| {
                    destination
                        .get("hostPattern")
                        .and_then(Value::as_str)
                        .is_some_and(|allowed| allowed.trim().eq_ignore_ascii_case(host.trim()))
                        && destination.get("port").and_then(Value::as_u64) == Some(u64::from(port))
                        && destination.get("protocol").and_then(Value::as_str) == Some("tcp")
                        && destination.get("allowPublic").and_then(Value::as_bool) != Some(true)
                })
            })
        })
    }

    pub fn gateway_certificate_sha256(&self, pod_id: &str) -> Option<[u8; 32]> {
        self.pods
            .get(pod_id)
            .and_then(|stored| gateway_certificate_sha256(&stored.pod))
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
            && gateway_peer_id(&pod).is_none_or(|gateway| !peer_ids_equal(gateway, &creator))
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
        validate_pod_members(&pod, &previous.members)?;
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
            .any(|member| member.is_banned && peer_ids_equal(&member.peer_id, &peer_id))
        {
            return Err("Peer is banned from this pod".to_owned());
        }
        if stored
            .members
            .iter()
            .any(|member| peer_ids_equal(&member.peer_id, &peer_id))
        {
            return Ok(Some(false));
        }
        let private_limit = private_gateway_member_limit(&stored.pod).unwrap_or(MAX_MEMBERS);
        let active_member_count = stored
            .members
            .iter()
            .filter(|member| !member.is_banned)
            .count();
        if active_member_count >= stored.pod.max_members.min(private_limit).min(MAX_MEMBERS) {
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
            .any(|member| !member.is_banned && peer_ids_equal(&member.peer_id, peer_id))
        {
            return Ok(Some(false));
        }
        if removal_would_orphan_pod(stored, peer_id) {
            return Err("Cannot remove the last Pod moderator".to_owned());
        }
        self.commit_change(|pods| {
            if let Some(stored) = pods.get_mut(pod_id) {
                stored
                    .members
                    .retain(|member| !peer_ids_equal(&member.peer_id, peer_id));
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
            .any(|member| peer_ids_equal(&member.peer_id, peer_id))
        {
            return Ok(Some(false));
        }
        if removal_would_orphan_pod(stored, peer_id) {
            return Err("Cannot remove the last Pod moderator".to_owned());
        }
        self.commit_change(|pods| {
            if let Some(stored) = pods.get_mut(pod_id) {
                if let Some(member) = stored
                    .members
                    .iter_mut()
                    .find(|member| peer_ids_equal(&member.peer_id, peer_id))
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
                let is_still_referenced = stored.pod.channels.iter().any(|channel| {
                    channel.binding_info.as_deref()
                        == Some(format!("soulseek-room:{binding}").as_str())
                });
                if !is_still_referenced {
                    stored.pod.external_bindings.retain(|candidate| {
                        !(candidate.kind == "soulseek-room" && candidate.identifier == binding)
                    });
                }
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

fn removal_would_orphan_pod(stored: &StoredPod, peer_id: &str) -> bool {
    let removes_moderator = stored.members.iter().any(|member| {
        !member.is_banned
            && peer_ids_equal(&member.peer_id, peer_id)
            && matches!(member.role.as_str(), "owner" | "mod")
    });
    removes_moderator
        && stored
            .members
            .iter()
            .filter(|member| !member.is_banned && matches!(member.role.as_str(), "owner" | "mod"))
            .count()
            <= 1
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
    validate_private_gateway_policy(pod)?;
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

fn peer_ids_equal(left: &str, right: &str) -> bool {
    left.eq_ignore_ascii_case(right)
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

fn gateway_peer_id(pod: &PodRecord) -> Option<&str> {
    pod.private_service_policy
        .as_ref()
        .and_then(Value::as_object)
        .and_then(|policy| policy.get("gatewayPeerId"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|gateway| !gateway.is_empty())
}

fn allowed_destinations(pod: &PodRecord) -> Option<&[Value]> {
    pod.private_service_policy
        .as_ref()
        .and_then(Value::as_object)
        .and_then(|policy| policy.get("allowedDestinations"))
        .and_then(Value::as_array)
        .map(Vec::as_slice)
}

fn gateway_certificate_sha256(pod: &PodRecord) -> Option<[u8; 32]> {
    let encoded = pod
        .private_service_policy
        .as_ref()
        .and_then(Value::as_object)
        .and_then(|policy| policy.get("gatewayCertificateSha256"))
        .and_then(Value::as_str)
        .map(str::trim)?;
    let mut fingerprint = [0_u8; 32];
    hex::decode_to_slice(encoded, &mut fingerprint).ok()?;
    Some(fingerprint)
}

fn private_gateway_member_limit(pod: &PodRecord) -> Option<usize> {
    private_gateway_enabled(pod).then(|| {
        pod.private_service_policy
            .as_ref()
            .and_then(|policy| policy.get("maxMembers"))
            .and_then(Value::as_u64)
            .and_then(|value| usize::try_from(value).ok())
            .unwrap_or(3)
    })
}

fn validate_private_gateway_member_count(
    pod: &PodRecord,
    member_count: usize,
) -> Result<(), String> {
    if private_gateway_member_limit(pod).is_some_and(|limit| member_count > limit) {
        return Err("Pod member count exceeds the private service gateway limit".to_owned());
    }
    Ok(())
}

fn validate_pod_members(pod: &PodRecord, members: &[PodMember]) -> Result<(), String> {
    let active_members = members
        .iter()
        .filter(|member| !member.is_banned)
        .collect::<Vec<_>>();
    if active_members.len() > pod.max_members {
        return Err("Pod member count exceeds MaxMembers".to_owned());
    }
    validate_private_gateway_member_count(pod, active_members.len())?;
    if !active_members
        .iter()
        .any(|member| matches!(member.role.as_str(), "owner" | "mod"))
    {
        return Err("Pod must retain at least one moderator".to_owned());
    }
    if private_gateway_enabled(pod) {
        let gateway = gateway_peer_id(pod).unwrap_or_default();
        if !active_members
            .iter()
            .any(|member| peer_ids_equal(&member.peer_id, gateway))
        {
            return Err("GatewayPeerId must be a pod member".to_owned());
        }
    }
    Ok(())
}

fn validate_private_gateway_policy(pod: &PodRecord) -> Result<(), String> {
    if !private_gateway_enabled(pod) {
        return Ok(());
    }
    let policy = pod
        .private_service_policy
        .as_ref()
        .and_then(Value::as_object)
        .ok_or_else(|| "PrivateServiceGateway capability requires a policy".to_owned())?;
    if policy.get("enabled").and_then(Value::as_bool) != Some(true) {
        return Err("PrivateServiceGateway capability requires policy.Enabled = true".to_owned());
    }
    let max_members = policy
        .get("maxMembers")
        .and_then(Value::as_u64)
        .unwrap_or(3);
    if !(2..=3).contains(&max_members) {
        return Err("PrivateServiceGateway allows between 2 and 3 members".to_owned());
    }
    let gateway = policy
        .get("gatewayPeerId")
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or_default();
    validate_peer_id(gateway)?;
    if policy.contains_key("gatewayCertificateSha256") && gateway_certificate_sha256(pod).is_none()
    {
        return Err(
            "GatewayCertificateSha256 must be a 64-character hexadecimal digest".to_owned(),
        );
    }

    let services = policy
        .get("registeredServices")
        .and_then(Value::as_array)
        .ok_or_else(|| "RegisteredServices cannot be null".to_owned())?;
    if services.len() > MAX_REGISTERED_SERVICES {
        return Err(format!(
            "Cannot have more than {MAX_REGISTERED_SERVICES} registered services"
        ));
    }
    for service in services {
        let service = service
            .as_object()
            .ok_or_else(|| "RegisteredService must be an object".to_owned())?;
        validate_policy_text(service, "name", "Service name", 50)?;
        if let Some(description) = service.get("description").and_then(Value::as_str) {
            validate_text("Service description", description, 200, true)?;
        }
        validate_policy_host(service, "host", "Host")?;
        validate_policy_port(service)?;
        validate_policy_protocol(service)?;
    }

    let destinations = policy
        .get("allowedDestinations")
        .and_then(Value::as_array)
        .ok_or_else(|| "AllowedDestinations cannot be null".to_owned())?;
    if destinations.is_empty() {
        return Err(
            "PrivateServiceGateway capability requires at least one allowed destination".to_owned(),
        );
    }
    if destinations.len() > MAX_ALLOWED_DESTINATIONS {
        return Err(format!(
            "Cannot have more than {MAX_ALLOWED_DESTINATIONS} allowed destinations"
        ));
    }
    for destination in destinations {
        let destination = destination
            .as_object()
            .ok_or_else(|| "AllowedDestination must be an object".to_owned())?;
        validate_policy_host(destination, "hostPattern", "HostPattern")?;
        validate_policy_port(destination)?;
        validate_policy_protocol(destination)?;
        if destination.get("allowPublic").and_then(Value::as_bool) == Some(true) {
            return Err("Public destinations are not allowed in MVP".to_owned());
        }
    }
    validate_private_gateway_member_count(pod, 0)
}

fn validate_policy_text(
    object: &serde_json::Map<String, Value>,
    field: &str,
    name: &str,
    maximum: usize,
) -> Result<(), String> {
    let value = object
        .get(field)
        .and_then(Value::as_str)
        .unwrap_or_default();
    validate_text(name, value, maximum, false)
}

fn validate_policy_host(
    object: &serde_json::Map<String, Value>,
    field: &str,
    name: &str,
) -> Result<(), String> {
    let host = object
        .get(field)
        .and_then(Value::as_str)
        .unwrap_or_default();
    validate_text(name, host, 255, false)?;
    if host.contains(['*', '?']) || !valid_exact_host(host) {
        return Err(format!(
            "Invalid {name} format (must be exact hostname or IP address)"
        ));
    }
    Ok(())
}

fn valid_exact_host(host: &str) -> bool {
    if host.parse::<std::net::IpAddr>().is_ok() {
        return true;
    }
    host.len() <= 253
        && host.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        })
}

fn validate_policy_port(object: &serde_json::Map<String, Value>) -> Result<(), String> {
    if !matches!(object.get("port").and_then(Value::as_u64), Some(1..=65_535)) {
        return Err("Port must be between 1 and 65535".to_owned());
    }
    Ok(())
}

fn validate_policy_protocol(object: &serde_json::Map<String, Value>) -> Result<(), String> {
    if object.get("protocol").and_then(Value::as_str) != Some("tcp") {
        return Err("Only TCP protocol is currently supported".to_owned());
    }
    Ok(())
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
                && matches!(member.role.as_str(), "owner" | "mod" | "member")
                && member
                    .public_key
                    .as_ref()
                    .is_none_or(|key| key.len() <= MAX_PUBLIC_KEY_BYTES)
                && member
                    .joined_at
                    .as_ref()
                    .is_none_or(|timestamp| timestamp.len() <= 128)
                && member
                    .last_seen
                    .as_ref()
                    .is_none_or(|timestamp| timestamp.len() <= 128)
                && peers.insert(member.peer_id.to_ascii_lowercase())
        });
        stored.members.truncate(MAX_MEMBERS);
        if validate_pod_members(&stored.pod, &stored.members).is_err() {
            continue;
        }
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

    fn private_gateway_fixture() -> PodRecord {
        serde_json::from_value(serde_json::json!({
            "podId": "pod:gateway",
            "name": "Gateway Pod",
            "maxMembers": 3,
            "capabilities": ["PrivateServiceGateway"],
            "privateServicePolicy": {
                "enabled": true,
                "maxMembers": 3,
                "gatewayPeerId": "owner",
                "registeredServices": [],
                "allowedDestinations": [{
                    "hostPattern": "internal.local",
                    "port": 443,
                    "protocol": "tcp",
                    "allowPublic": false
                }]
            }
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

    #[test]
    fn unbinding_one_channel_preserves_a_shared_external_binding() {
        let state_dir = std::env::temp_dir().join(format!(
            "slskr-pod-shared-binding-{}",
            uuid::Uuid::new_v4().simple()
        ));
        std::fs::create_dir_all(&state_dir).unwrap();
        let mut store = PodStore::empty(&state_dir);
        let mut pod = fixture();
        pod.channels.push(super::PodChannel {
            channel_id: "secondary".to_owned(),
            kind: serde_json::json!(0),
            name: "Secondary".to_owned(),
            binding_info: None,
            description: None,
        });
        store.create(pod, "owner".to_owned()).unwrap();
        for channel_id in ["general", "secondary"] {
            assert!(store
                .bind_room(
                    "pod:test",
                    channel_id,
                    "music".to_owned(),
                    "mirror".to_owned(),
                )
                .unwrap()
                .unwrap());
        }

        assert!(store.unbind_room("pod:test", "general").unwrap().unwrap());
        let loaded = PodStore::load(&state_dir).unwrap();
        let pod = loaded.get("pod:test").unwrap();
        assert_eq!(pod.external_bindings.len(), 1);
        assert_eq!(pod.external_bindings[0].identifier, "music");
        assert_eq!(
            pod.channels[1].binding_info.as_deref(),
            Some("soulseek-room:music")
        );
        std::fs::remove_dir_all(state_dir).unwrap();
    }

    #[test]
    fn private_gateway_update_authorization_matches_the_controller_contract() {
        let state_dir = std::env::temp_dir().join(format!(
            "slskr-pod-gateway-update-{}",
            uuid::Uuid::new_v4().simple()
        ));
        std::fs::create_dir_all(&state_dir).unwrap();
        let mut store = PodStore::empty(&state_dir);
        store
            .create(private_gateway_fixture(), "owner".to_owned())
            .unwrap();

        let mut unrelated_update = store.get("pod:gateway").unwrap();
        unrelated_update.name = "Renamed Gateway Pod".to_owned();
        assert_eq!(
            store.gateway_peer_for_update("pod:gateway", &unrelated_update),
            Some("owner".to_owned())
        );

        let mut gateway_transfer = store.get("pod:gateway").unwrap();
        gateway_transfer.private_service_policy.as_mut().unwrap()["gatewayPeerId"] =
            serde_json::json!("replacement");
        assert_eq!(
            store.gateway_peer_for_update("pod:gateway", &gateway_transfer),
            Some("owner".to_owned())
        );

        let mut removal = store.get("pod:gateway").unwrap();
        removal.capabilities.clear();
        removal.private_service_policy = None;
        assert_eq!(store.gateway_peer_for_update("pod:gateway", &removal), None);
        std::fs::remove_dir_all(state_dir).unwrap();
    }

    #[test]
    fn private_gateway_policy_is_validated_and_limits_membership() {
        let state_dir = std::env::temp_dir().join(format!(
            "slskr-pod-private-policy-{}",
            uuid::Uuid::new_v4().simple()
        ));
        std::fs::create_dir_all(&state_dir).unwrap();
        let mut store = PodStore::empty(&state_dir);
        let mut pod = fixture();
        pod.capabilities = vec![serde_json::json!(0)];
        pod.private_service_policy = Some(serde_json::json!({
            "enabled": true,
            "maxMembers": 2,
            "gatewayPeerId": "owner",
            "registeredServices": [{
                "name": "SSH",
                "host": "server.lan",
                "port": 22,
                "protocol": "tcp"
            }],
            "allowedDestinations": [{
                "hostPattern": "server.lan",
                "port": 22,
                "protocol": "tcp",
                "allowPublic": false
            }]
        }));
        store.create(pod, "owner".to_owned()).unwrap();
        assert!(store.destination_allowed("pod:test", "SERVER.LAN", 22));
        assert!(!store.destination_allowed("pod:test", "server.lan", 23));
        assert!(store.gateway_certificate_sha256("pod:test").is_none());
        assert!(store
            .join("pod:test", "member".to_owned())
            .unwrap()
            .unwrap());
        assert_eq!(
            store.join("pod:test", "overflow".to_owned()).unwrap_err(),
            "Pod member capacity is full"
        );
        assert!(store.ban("pod:test", "member").unwrap().unwrap());
        assert!(store
            .join("pod:test", "replacement".to_owned())
            .unwrap()
            .unwrap());

        let mut invalid_gateway = store.get("pod:test").unwrap();
        invalid_gateway.private_service_policy.as_mut().unwrap()["gatewayPeerId"] =
            serde_json::json!("outsider");
        assert_eq!(
            store.update("pod:test", invalid_gateway).unwrap_err(),
            "GatewayPeerId must be a pod member"
        );

        let mut invalid = fixture();
        invalid.pod_id = "pod:invalid".to_owned();
        invalid.capabilities = vec![serde_json::json!(0)];
        invalid.private_service_policy = Some(serde_json::json!({
            "enabled": true,
            "maxMembers": 3,
            "gatewayPeerId": "owner",
            "registeredServices": [],
            "allowedDestinations": [{
                "hostPattern": "*.lan",
                "port": 80,
                "protocol": "tcp"
            }]
        }));
        assert!(store
            .create(invalid, "owner".to_owned())
            .unwrap_err()
            .contains("exact hostname"));
        std::fs::remove_dir_all(state_dir).unwrap();
    }

    #[test]
    fn the_last_pod_moderator_cannot_leave_or_be_banned() {
        let state_dir = std::env::temp_dir().join(format!(
            "slskr-pod-last-moderator-{}",
            uuid::Uuid::new_v4().simple()
        ));
        std::fs::create_dir_all(&state_dir).unwrap();
        let mut store = PodStore::empty(&state_dir);
        store.create(fixture(), "owner".to_owned()).unwrap();

        assert_eq!(
            store.leave("pod:test", "owner").unwrap_err(),
            "Cannot remove the last Pod moderator"
        );
        assert_eq!(
            store.ban("pod:test", "owner").unwrap_err(),
            "Cannot remove the last Pod moderator"
        );
        assert!(store.is_member("pod:test", "owner"));
        std::fs::remove_dir_all(state_dir).unwrap();
    }

    #[test]
    fn peer_identity_checks_and_bans_are_case_insensitive() {
        let state_dir = std::env::temp_dir().join(format!(
            "slskr-pod-peer-case-{}",
            uuid::Uuid::new_v4().simple()
        ));
        std::fs::create_dir_all(&state_dir).unwrap();
        let mut store = PodStore::empty(&state_dir);
        let mut pod = fixture();
        pod.is_public = false;
        store.create(pod, "Owner".to_owned()).unwrap();

        assert!(store.is_member("pod:test", "owner"));
        assert!(store.can_moderate("pod:test", "OWNER"));
        assert_eq!(store.list_visible(Some("oWnEr")).len(), 1);

        store.pods.get_mut("pod:test").unwrap().pod.is_public = true;
        assert!(store
            .join("pod:test", "Member".to_owned())
            .unwrap()
            .unwrap());
        assert_eq!(
            store.join("pod:test", "MEMBER".to_owned()).unwrap(),
            Some(false)
        );
        assert_eq!(store.ban("pod:test", "member").unwrap(), Some(true));
        assert!(!store.is_member("pod:test", "MEMBER"));
        assert_eq!(
            store.join("pod:test", "mEmBeR".to_owned()).unwrap_err(),
            "Peer is banned from this pod"
        );
        std::fs::remove_dir_all(state_dir).unwrap();
    }
}
