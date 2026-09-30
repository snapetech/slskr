use super::*;

#[path = "controller_yaml_projection.rs"]
mod projection;
#[path = "controller_yaml_validation.rs"]
mod validation;

pub(super) use self::projection::{camel_to_snake_case, controller_yaml_api_projection};
pub(super) use self::validation::controller_yaml_target_validation_error;

pub(super) fn controller_options_config_text_response(config: &AppConfig) -> HttpResponse {
    match read_controller_compatibility_yaml(config) {
        Ok(Some(text)) => HttpResponse {
            status: "200 OK",
            content_type: "application/json; charset=utf-8",
            body: serde_json::Value::String(text).to_string(),
        },
        Ok(None) => routing::not_found_response(),
        Err(error) => {
            eprintln!("configuration file read failed: {error}");
            routing::internal_server_error_response("Failed to read configuration file")
        }
    }
}

pub(super) fn controller_options_config_body(body: &str) -> Result<String, String> {
    let value = serde_json::from_str::<serde_json::Value>(body)
        .map_err(|error| format!("invalid config JSON string: {error}"))?;
    let Some(text) = value.as_str() else {
        return Err("config payload must be a JSON string".to_owned());
    };
    Ok(text.to_owned())
}

fn validate_controller_yaml_node(
    value: &serde_yaml::Value,
    depth: usize,
    nodes: &mut usize,
) -> Result<(), String> {
    if depth > MAX_CONTROLLER_YAML_DEPTH {
        return Err("Invalid YAML configuration".to_owned());
    }
    *nodes = nodes.saturating_add(1);
    if *nodes > MAX_CONTROLLER_YAML_NODES {
        return Err("Invalid YAML configuration".to_owned());
    }
    match value {
        serde_yaml::Value::Mapping(mapping) => {
            for (key, value) in mapping {
                if !matches!(key, serde_yaml::Value::String(_)) {
                    return Err("Invalid YAML configuration".to_owned());
                }
                validate_controller_yaml_node(value, depth + 1, nodes)?;
            }
        }
        serde_yaml::Value::Sequence(sequence) => {
            for value in sequence {
                validate_controller_yaml_node(value, depth + 1, nodes)?;
            }
        }
        serde_yaml::Value::Tagged(tagged) => {
            validate_controller_yaml_node(&tagged.value, depth + 1, nodes)?;
        }
        _ => {}
    }
    Ok(())
}

pub(super) fn parse_controller_yaml(text: &str) -> Result<serde_json::Value, String> {
    if text.len() > MAX_CONTROLLER_YAML_BYTES {
        return Err("YAML is too large".to_owned());
    }
    let yaml = serde_yaml::from_str::<serde_yaml::Value>(text)
        .map_err(|_| "Invalid YAML configuration".to_owned())?;
    if !matches!(
        yaml,
        serde_yaml::Value::Mapping(_) | serde_yaml::Value::Null
    ) {
        return Err("Invalid YAML configuration".to_owned());
    }
    validate_controller_yaml_node(&yaml, 0, &mut 0)?;
    serde_json::to_value(yaml).map_err(|_| "Invalid YAML configuration".to_owned())
}
