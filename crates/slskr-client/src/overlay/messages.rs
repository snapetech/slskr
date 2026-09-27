use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SoulseekPorts {
    pub peer: u16,
    pub file: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeshHello {
    pub magic: String,
    #[serde(rename = "type")]
    pub message_type: String,
    pub version: i32,
    pub username: String,
    pub features: Vec<String>,
    pub soulseek_ports: Option<SoulseekPorts>,
    pub overlay_port: Option<u16>,
    pub nonce: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth_public_key: Option<[u8; 32]>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "optional_base64_bytes"
    )]
    pub auth_signature: Option<Vec<u8>>,
}

impl MeshHello {
    pub fn new(
        username: impl Into<String>,
        features: Vec<String>,
        soulseek_ports: Option<SoulseekPorts>,
        overlay_port: Option<u16>,
        nonce: impl Into<String>,
    ) -> Result<Self, OverlayError> {
        let message = Self {
            magic: OVERLAY_MAGIC.to_owned(),
            message_type: "mesh_hello".to_owned(),
            version: OVERLAY_VERSION,
            username: username.into(),
            features,
            soulseek_ports,
            overlay_port,
            nonce: Some(nonce.into()),
            auth_public_key: None,
            auth_signature: None,
        };
        message.validate()?;
        Ok(message)
    }

    pub fn validate(&self) -> Result<(), OverlayError> {
        validate_handshake(
            &self.magic,
            &self.message_type,
            "mesh_hello",
            self.version,
            &self.username,
            &self.features,
            self.nonce.as_deref(),
        )?;
        validate_advertised_ports(self.soulseek_ports.as_ref(), self.overlay_port)?;
        if self.auth_public_key.is_some() != self.auth_signature.is_some()
            || self
                .auth_signature
                .as_ref()
                .is_some_and(|signature| signature.len() != 64)
        {
            return Err(OverlayError::InvalidPeerAuthentication);
        }
        Ok(())
    }

    pub fn authenticate(
        &mut self,
        signing_key: &SigningKey,
        gateway_certificate_sha256: &[u8; 32],
    ) -> Result<(), OverlayError> {
        self.validate()?;
        let signature = signing_key.sign(&self.authentication_payload(gateway_certificate_sha256)?);
        self.auth_public_key = Some(signing_key.verifying_key().to_bytes());
        self.auth_signature = Some(signature.to_bytes().to_vec());
        Ok(())
    }

    pub fn verify_authentication(
        &self,
        expected_public_key: &[u8; 32],
        gateway_certificate_sha256: &[u8; 32],
    ) -> Result<(), OverlayError> {
        self.validate()?;
        if self.auth_public_key.as_ref() != Some(expected_public_key) {
            return Err(OverlayError::InvalidPeerAuthentication);
        }
        let signature = self
            .auth_signature
            .as_deref()
            .and_then(|signature| <&[u8; 64]>::try_from(signature).ok())
            .ok_or(OverlayError::InvalidPeerAuthentication)?;
        let verifying_key = VerifyingKey::from_bytes(expected_public_key)
            .map_err(|_| OverlayError::InvalidPeerAuthentication)?;
        verifying_key
            .verify_strict(
                &self.authentication_payload(gateway_certificate_sha256)?,
                &Signature::from_bytes(signature),
            )
            .map_err(|_| OverlayError::InvalidPeerAuthentication)
    }

    fn authentication_payload(
        &self,
        gateway_certificate_sha256: &[u8; 32],
    ) -> Result<Vec<u8>, OverlayError> {
        let nonce = self.nonce.as_deref().ok_or(OverlayError::InvalidNonce)?;
        let mut payload = b"slskr-overlay-auth-v1\0".to_vec();
        for value in [&self.magic, &self.message_type, &self.username, nonce] {
            let length =
                u32::try_from(value.len()).map_err(|_| OverlayError::InvalidPeerAuthentication)?;
            payload.extend_from_slice(&length.to_be_bytes());
            payload.extend_from_slice(value.as_bytes());
        }
        payload.extend_from_slice(&self.version.to_be_bytes());
        let feature_count = u32::try_from(self.features.len())
            .map_err(|_| OverlayError::InvalidPeerAuthentication)?;
        payload.extend_from_slice(&feature_count.to_be_bytes());
        for feature in &self.features {
            let length = u32::try_from(feature.len())
                .map_err(|_| OverlayError::InvalidPeerAuthentication)?;
            payload.extend_from_slice(&length.to_be_bytes());
            payload.extend_from_slice(feature.as_bytes());
        }
        match &self.soulseek_ports {
            Some(ports) => {
                payload.push(1);
                payload.extend_from_slice(&ports.peer.to_be_bytes());
                payload.extend_from_slice(&ports.file.to_be_bytes());
            }
            None => payload.push(0),
        }
        match self.overlay_port {
            Some(port) => {
                payload.push(1);
                payload.extend_from_slice(&port.to_be_bytes());
            }
            None => payload.push(0),
        }
        payload.extend_from_slice(gateway_certificate_sha256);
        Ok(payload)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeshHelloAck {
    pub magic: String,
    #[serde(rename = "type")]
    pub message_type: String,
    pub version: i32,
    pub username: String,
    pub features: Vec<String>,
    pub soulseek_ports: Option<SoulseekPorts>,
    pub overlay_port: Option<u16>,
    pub nonce_echo: Option<String>,
}

impl MeshHelloAck {
    pub fn validate(&self) -> Result<(), OverlayError> {
        validate_handshake(
            &self.magic,
            &self.message_type,
            "mesh_hello_ack",
            self.version,
            &self.username,
            &self.features,
            self.nonce_echo.as_deref(),
        )?;
        validate_advertised_ports(self.soulseek_ports.as_ref(), self.overlay_port)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ping {
    pub magic: String,
    #[serde(rename = "type")]
    pub message_type: String,
    pub version: i32,
    pub timestamp: i64,
}

impl Ping {
    pub fn validate(&self) -> Result<(), OverlayError> {
        validate_overlay_base(&self.magic, &self.message_type, self.version)?;
        if self.message_type != "ping" {
            return Err(OverlayError::InvalidMessageType);
        }
        validate_control_timestamp(self.timestamp)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pong {
    pub magic: String,
    #[serde(rename = "type")]
    pub message_type: String,
    pub version: i32,
    pub timestamp: i64,
}

impl Pong {
    pub fn validate(&self) -> Result<(), OverlayError> {
        validate_overlay_base(&self.magic, &self.message_type, self.version)?;
        if self.message_type != "pong" {
            return Err(OverlayError::InvalidMessageType);
        }
        validate_control_timestamp(self.timestamp)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Disconnect {
    pub magic: String,
    #[serde(rename = "type")]
    pub message_type: String,
    pub version: i32,
    pub reason: Option<String>,
}

impl Disconnect {
    pub fn validate(&self) -> Result<(), OverlayError> {
        validate_overlay_base(&self.magic, &self.message_type, self.version)?;
        if self.message_type != "disconnect" {
            return Err(OverlayError::InvalidMessageType);
        }
        if self.reason.as_ref().is_some_and(|reason| {
            reason.len() > MAX_DISCONNECT_REASON_BYTES || reason.chars().any(char::is_control)
        }) {
            return Err(OverlayError::InvalidDisconnectReason);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeshSearchFileDto {
    pub filename: String,
    pub size: i64,
    pub extension: Option<String>,
    pub bitrate: Option<i32>,
    pub duration: Option<i32>,
    pub codec: Option<String>,
    #[serde(rename = "mediaKinds")]
    pub media_kinds: Option<Vec<String>>,
    #[serde(rename = "contentId")]
    pub content_id: Option<String>,
    pub hash: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeshSearchRequestMessage {
    pub magic: String,
    #[serde(rename = "type")]
    pub message_type: String,
    pub version: i32,
    #[serde(rename = "request_id")]
    pub request_id: String,
    #[serde(rename = "search_text")]
    pub search_text: String,
    #[serde(rename = "max_results")]
    pub max_results: i32,
    pub scope: Option<String>,
}

impl MeshSearchRequestMessage {
    pub fn new(
        request_id: impl Into<String>,
        search_text: impl Into<String>,
        max_results: i32,
        scope: Option<String>,
    ) -> Result<Self, OverlayError> {
        let message = Self {
            magic: OVERLAY_MAGIC.to_owned(),
            message_type: "mesh_search_req".to_owned(),
            version: OVERLAY_VERSION,
            request_id: request_id.into(),
            search_text: search_text.into(),
            max_results,
            scope,
        };
        message.validate()?;
        Ok(message)
    }

    pub fn validate(&self) -> Result<(), OverlayError> {
        validate_overlay_base(&self.magic, &self.message_type, self.version)?;
        if self.message_type != "mesh_search_req" {
            return Err(OverlayError::InvalidMessageType);
        }
        if Uuid::parse_str(&self.request_id).is_err() {
            return Err(OverlayError::InvalidMeshSearchRequest("request_id"));
        }
        if self.search_text.trim().is_empty()
            || self.search_text.len() > MAX_SEARCH_TEXT_BYTES
            || self.search_text.chars().any(char::is_control)
            || self.search_text.encode_utf16().count() > 256
        {
            return Err(OverlayError::InvalidMeshSearchRequest("search_text"));
        }
        if !(1..=200).contains(&self.max_results) {
            return Err(OverlayError::InvalidMeshSearchRequest("max_results"));
        }
        if self
            .scope
            .as_deref()
            .is_some_and(|scope| !valid_text_field(scope, MAX_SERVICE_FIELD_BYTES))
        {
            return Err(OverlayError::InvalidMeshSearchRequest("scope"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeshSearchResponseMessage {
    pub magic: String,
    #[serde(rename = "type")]
    pub message_type: String,
    pub version: i32,
    #[serde(rename = "request_id")]
    pub request_id: String,
    pub files: Vec<MeshSearchFileDto>,
    pub truncated: bool,
    pub error: Option<String>,
}

impl MeshSearchResponseMessage {
    pub fn new(
        request_id: impl Into<String>,
        files: Vec<MeshSearchFileDto>,
        truncated: bool,
        error: Option<String>,
    ) -> Result<Self, OverlayError> {
        let message = Self {
            magic: OVERLAY_MAGIC.to_owned(),
            message_type: "mesh_search_resp".to_owned(),
            version: OVERLAY_VERSION,
            request_id: request_id.into(),
            files,
            truncated,
            error,
        };
        message.validate()?;
        Ok(message)
    }

    pub fn validate(&self) -> Result<(), OverlayError> {
        validate_overlay_base(&self.magic, &self.message_type, self.version)?;
        if self.message_type != "mesh_search_resp" {
            return Err(OverlayError::InvalidMessageType);
        }
        if Uuid::parse_str(&self.request_id).is_err() {
            return Err(OverlayError::InvalidMeshSearchResponse("request_id"));
        }
        if self.files.len() > 500 {
            return Err(OverlayError::InvalidMeshSearchResponse("files"));
        }
        if self.files.iter().any(|file| {
            !valid_text_field(&file.filename, MAX_SERVICE_FIELD_BYTES)
                || !(0..=10_000_000_000).contains(&file.size)
                || file
                    .extension
                    .as_deref()
                    .is_some_and(|value| !valid_text_field(value, MAX_SERVICE_FIELD_BYTES))
                || file
                    .codec
                    .as_deref()
                    .is_some_and(|value| !valid_text_field(value, MAX_SERVICE_FIELD_BYTES))
                || file
                    .content_id
                    .as_deref()
                    .is_some_and(|value| !valid_text_field(value, MAX_SERVICE_FIELD_BYTES))
                || file
                    .hash
                    .as_deref()
                    .is_some_and(|value| !valid_text_field(value, MAX_SERVICE_FIELD_BYTES))
                || file.media_kinds.as_ref().is_some_and(|values| {
                    values.len() > MAX_HANDSHAKE_FEATURES
                        || values
                            .iter()
                            .any(|value| !valid_text_field(value, MAX_FEATURE_BYTES))
                })
        }) {
            return Err(OverlayError::InvalidMeshSearchResponse("file"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeshServiceCall {
    pub magic: String,
    #[serde(rename = "type")]
    pub message_type: String,
    pub version: i32,
    pub correlation_id: String,
    pub service_name: String,
    pub method: String,
    #[serde(with = "base64_bytes")]
    pub payload: Vec<u8>,
}

impl MeshServiceCall {
    pub fn new(
        correlation_id: impl Into<String>,
        service_name: impl Into<String>,
        method: impl Into<String>,
        payload: Vec<u8>,
    ) -> Result<Self, OverlayError> {
        let call = Self {
            magic: OVERLAY_MAGIC.to_owned(),
            message_type: "mesh_service_call".to_owned(),
            version: OVERLAY_VERSION,
            correlation_id: correlation_id.into(),
            service_name: service_name.into(),
            method: method.into(),
            payload,
        };
        call.validate()?;
        Ok(call)
    }

    pub fn validate(&self) -> Result<(), OverlayError> {
        validate_overlay_base(&self.magic, &self.message_type, self.version)?;
        if self.message_type != "mesh_service_call" {
            return Err(OverlayError::InvalidMessageType);
        }
        validate_service_field("correlation_id", &self.correlation_id)?;
        validate_service_field("service_name", &self.service_name)?;
        validate_service_field("method", &self.method)?;
        if self.payload.len() > MAX_OVERLAY_MESSAGE_BYTES {
            return Err(OverlayError::FrameTooLarge(self.payload.len()));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeshServiceReply {
    pub magic: String,
    #[serde(rename = "type")]
    pub message_type: String,
    pub version: i32,
    pub correlation_id: String,
    pub status_code: i32,
    #[serde(with = "base64_bytes")]
    pub payload: Vec<u8>,
    pub error_message: Option<String>,
}

impl MeshServiceReply {
    pub fn validate(&self) -> Result<(), OverlayError> {
        validate_overlay_base(&self.magic, &self.message_type, self.version)?;
        if self.message_type != "mesh_service_reply" {
            return Err(OverlayError::InvalidMessageType);
        }
        validate_service_field("correlation_id", &self.correlation_id)?;
        if self.payload.len() > MAX_OVERLAY_MESSAGE_BYTES {
            return Err(OverlayError::FrameTooLarge(self.payload.len()));
        }
        if self.error_message.as_ref().is_some_and(|message| {
            message.len() > MAX_SERVICE_ERROR_BYTES || message.chars().any(char::is_control)
        }) {
            return Err(OverlayError::InvalidServiceField("error_message"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "OpenTunnelRequestWire")]
#[serde(rename_all = "PascalCase")]
pub struct OpenTunnelRequest {
    pub pod_id: String,
    pub destination_host: String,
    pub destination_port: u16,
    pub service_name: Option<String>,
    pub request_nonce: String,
    pub request_timestamp: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct OpenTunnelRequestWire {
    pod_id: String,
    destination_host: String,
    destination_port: u16,
    service_name: Option<String>,
    request_nonce: String,
    request_timestamp: i64,
}

impl TryFrom<OpenTunnelRequestWire> for OpenTunnelRequest {
    type Error = OverlayError;

    fn try_from(request: OpenTunnelRequestWire) -> Result<Self, Self::Error> {
        let request = Self {
            pod_id: request.pod_id,
            destination_host: request.destination_host,
            destination_port: request.destination_port,
            service_name: request.service_name,
            request_nonce: request.request_nonce,
            request_timestamp: request.request_timestamp,
        };
        request.validate()?;
        Ok(request)
    }
}

impl OpenTunnelRequest {
    pub fn new(
        pod_id: impl Into<String>,
        destination_host: impl Into<String>,
        destination_port: u16,
        service_name: Option<String>,
        request_nonce: impl Into<String>,
    ) -> Result<Self, OverlayError> {
        let request = Self {
            pod_id: pod_id.into(),
            destination_host: destination_host.into(),
            destination_port,
            service_name,
            request_nonce: request_nonce.into(),
            request_timestamp: unix_seconds()?,
        };
        request.validate()?;
        Ok(request)
    }

    pub fn validate(&self) -> Result<(), OverlayError> {
        if self.pod_id.trim().is_empty()
            || self.pod_id.len() > MAX_POD_ID_BYTES
            || self.pod_id.chars().any(char::is_control)
            || self.destination_host.trim().is_empty()
            || self.destination_host.len() > MAX_DESTINATION_HOST_BYTES
            || self.destination_host.chars().any(char::is_control)
            || self.destination_port == 0
            || self.service_name.as_ref().is_some_and(|service| {
                service.trim().is_empty()
                    || service.len() > MAX_SERVICE_FIELD_BYTES
                    || service.chars().any(char::is_control)
            })
            || self.request_nonce.trim().is_empty()
            || self.request_nonce.len() > MAX_NONCE_BYTES
            || self.request_nonce.chars().any(char::is_control)
        {
            return Err(OverlayError::InvalidPrivateGatewayRequest);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct OpenTunnelResponse {
    pub tunnel_id: String,
    pub accepted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct TunnelDataRequest {
    pub tunnel_id: String,
    #[serde(with = "base64_bytes")]
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct GetTunnelDataRequest {
    pub tunnel_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct TunnelDataResponse {
    #[serde(with = "base64_bytes")]
    pub data: Vec<u8>,
    pub bytes_received: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CloseTunnelRequest {
    pub tunnel_id: String,
}

#[derive(Debug, thiserror::Error)]
pub enum OverlayError {
    #[error("overlay I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("overlay JSON failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error("overlay frame is too small: {0}")]
    FrameTooSmall(usize),
    #[error("overlay frame is too large: {0}")]
    FrameTooLarge(usize),
    #[error("overlay message must be a JSON object")]
    InvalidJsonObject,
    #[error("overlay protocol magic is invalid")]
    InvalidMagic,
    #[error("overlay protocol version {0} is invalid")]
    InvalidVersion(i32),
    #[error("overlay message type is invalid")]
    InvalidMessageType,
    #[error("overlay username is invalid")]
    InvalidUsername,
    #[error("overlay feature list is invalid")]
    InvalidFeatures,
    #[error("overlay nonce is invalid")]
    InvalidNonce,
    #[error("overlay peer authentication is invalid")]
    InvalidPeerAuthentication,
    #[error("overlay handshake nonce does not match")]
    NonceMismatch,
    #[error("overlay handshake advertises an invalid {0} port")]
    InvalidAdvertisedPort(&'static str),
    #[error("overlay service field {0} is invalid")]
    InvalidServiceField(&'static str),
    #[error("remote overlay does not advertise mesh_service")]
    MeshServiceUnsupported,
    #[error("remote overlay does not advertise mesh_search")]
    MeshSearchUnsupported,
    #[error("overlay peer disconnected")]
    Disconnected,
    #[error("matching overlay service reply was not received")]
    ReplyNotFound,
    #[error("private-gateway request is invalid")]
    InvalidPrivateGatewayRequest,
    #[error("system clock is before the Unix epoch")]
    InvalidTime,
    #[error("overlay control timestamp is invalid")]
    InvalidControlTimestamp,
    #[error("overlay disconnect reason is invalid")]
    InvalidDisconnectReason,
    #[error("overlay base64 payload is invalid")]
    InvalidBase64,
    #[error("mesh search request field {0} is invalid")]
    InvalidMeshSearchRequest(&'static str),
    #[error("mesh search response field {0} is invalid")]
    InvalidMeshSearchResponse(&'static str),
    #[error("overlay {0} timed out")]
    Timeout(&'static str),
    #[error("overlay TLS failed: {0}")]
    Tls(String),
}

pub(super) fn message_type(payload: &[u8]) -> Result<String, OverlayError> {
    serde_json::from_slice::<Value>(payload)?
        .get("type")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or(OverlayError::InvalidMessageType)
}

fn validate_handshake(
    magic: &str,
    message_type: &str,
    expected_type: &str,
    version: i32,
    username: &str,
    features: &[String],
    nonce: Option<&str>,
) -> Result<(), OverlayError> {
    validate_overlay_base(magic, message_type, version)?;
    if message_type != expected_type {
        return Err(OverlayError::InvalidMessageType);
    }
    if username.is_empty()
        || username.len() > MAX_USERNAME_BYTES
        || !username
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
    {
        return Err(OverlayError::InvalidUsername);
    }
    if features.len() > MAX_HANDSHAKE_FEATURES
        || features.iter().any(|feature| {
            feature.is_empty()
                || feature.len() > MAX_FEATURE_BYTES
                || !feature
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
        })
    {
        return Err(OverlayError::InvalidFeatures);
    }
    if nonce.is_some_and(|nonce| {
        nonce.is_empty()
            || nonce.len() > MAX_NONCE_BYTES
            || !nonce
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    }) {
        return Err(OverlayError::InvalidNonce);
    }
    Ok(())
}

pub(super) fn validate_overlay_base(
    magic: &str,
    message_type: &str,
    version: i32,
) -> Result<(), OverlayError> {
    if magic.as_bytes() != OVERLAY_MAGIC.as_bytes() {
        return Err(OverlayError::InvalidMagic);
    }
    if version != OVERLAY_VERSION {
        return Err(OverlayError::InvalidVersion(version));
    }
    if message_type.trim().is_empty() {
        return Err(OverlayError::InvalidMessageType);
    }
    Ok(())
}

fn validate_advertised_ports(
    soulseek_ports: Option<&SoulseekPorts>,
    overlay_port: Option<u16>,
) -> Result<(), OverlayError> {
    if soulseek_ports.is_some_and(|ports| ports.peer == 0) {
        return Err(OverlayError::InvalidAdvertisedPort("Soulseek peer"));
    }
    if soulseek_ports.is_some_and(|ports| ports.file == 0) {
        return Err(OverlayError::InvalidAdvertisedPort("Soulseek file"));
    }
    if overlay_port == Some(0) {
        return Err(OverlayError::InvalidAdvertisedPort("overlay"));
    }
    Ok(())
}

fn validate_service_field(field: &'static str, value: &str) -> Result<(), OverlayError> {
    if value.trim().is_empty()
        || value.len() > MAX_SERVICE_FIELD_BYTES
        || value.chars().any(char::is_control)
    {
        Err(OverlayError::InvalidServiceField(field))
    } else {
        Ok(())
    }
}

fn valid_text_field(value: &str, max_bytes: usize) -> bool {
    !value.trim().is_empty() && value.len() <= max_bytes && !value.chars().any(char::is_control)
}

pub(super) fn validate_mesh_search_response_limit(
    response: &MeshSearchResponseMessage,
    max_results: i32,
) -> Result<(), OverlayError> {
    let max_results = usize::try_from(max_results)
        .map_err(|_| OverlayError::InvalidMeshSearchResponse("max_results"))?;
    if response.files.len() > max_results {
        return Err(OverlayError::InvalidMeshSearchResponse("files"));
    }
    Ok(())
}

fn unix_seconds() -> Result<i64, OverlayError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| i64::try_from(duration.as_secs()).ok())
        .ok_or(OverlayError::InvalidTime)
}

fn validate_control_timestamp(timestamp: i64) -> Result<(), OverlayError> {
    let timestamp = u64::try_from(timestamp).map_err(|_| OverlayError::InvalidControlTimestamp)?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| u64::try_from(duration.as_millis()).ok())
        .ok_or(OverlayError::InvalidTime)?;
    if now.abs_diff(timestamp) > CONTROL_TIMESTAMP_SKEW_MILLIS {
        return Err(OverlayError::InvalidControlTimestamp);
    }
    Ok(())
}

mod base64_bytes {
    use super::*;
    use serde::{Deserializer, Serializer};

    pub fn serialize<S>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&STANDARD.encode(bytes))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let encoded = String::deserialize(deserializer)?;
        STANDARD.decode(encoded).map_err(serde::de::Error::custom)
    }
}

mod optional_base64_bytes {
    use super::*;
    use serde::{Deserializer, Serializer};

    pub fn serialize<S>(bytes: &Option<Vec<u8>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        bytes
            .as_ref()
            .map(|bytes| STANDARD.encode(bytes))
            .serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Vec<u8>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        Option::<String>::deserialize(deserializer)?
            .map(|encoded| STANDARD.decode(encoded).map_err(serde::de::Error::custom))
            .transpose()
    }
}
