use super::*;

pub(super) fn overlay_service_enabled(
    service_name: &str,
    features: &crate::config::FeatureGateSettings,
    target: crate::config::ControllerProfile,
) -> bool {
    // The frozen native profile application only registers DHT, hole-punch, and
    // MeshContent services with its remote mesh router.  Its local pods and
    // VirtualSoulfind HTTP controllers still exist, but remote calls to the
    // corresponding overlay services return the router's not-found contract.
    if target == crate::config::ControllerProfile::Native
        && matches!(service_name, "pods" | "private-gateway" | "shadow-index")
    {
        return false;
    }
    match service_name {
        "private-gateway" | "MeshContent" => features.mesh,
        "pods" => features.pods,
        "shadow-index" => features.virtual_soulfind,
        _ => true,
    }
}

pub(super) fn local_service_enabled(
    service_name: &str,
    features: &crate::config::FeatureGateSettings,
) -> bool {
    match service_name {
        "private-gateway" | "MeshContent" => features.mesh,
        "pods" => features.pods,
        "shadow-index" => features.virtual_soulfind,
        "dht" => features.dht,
        _ => true,
    }
}

pub(super) fn service_reply(
    correlation_id: String,
    status_code: i32,
    payload: Vec<u8>,
    error_message: Option<String>,
) -> MeshServiceReply {
    MeshServiceReply {
        magic: OVERLAY_MAGIC.to_owned(),
        message_type: "mesh_service_reply".to_owned(),
        version: OVERLAY_VERSION,
        correlation_id,
        status_code,
        payload,
        error_message,
    }
}

pub(super) fn parse_payload<T: serde::de::DeserializeOwned>(
    payload: &[u8],
) -> Result<T, (i32, String)> {
    serde_json::from_slice(payload).map_err(|_| (4, "Invalid request payload".to_owned()))
}

pub(super) fn bounded_required<'a>(
    value: &'a str,
    maximum: usize,
    name: &str,
) -> Result<&'a str, (i32, String)> {
    let value = value.trim();
    if value.is_empty() || value.len() > maximum {
        return Err((4, format!("{name} is invalid")));
    }
    Ok(value)
}

pub(super) fn valid_service_call(call: &MeshServiceCall) -> bool {
    call.validate().is_ok() && call.payload.len() <= MAX_OVERLAY_MESSAGE_BYTES
}
