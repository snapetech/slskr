use super::*;

#[cfg(target_arch = "wasm32")]
pub(super) fn native_shared_grant_id(
    document: &web_sys::Document,
    button: &web_sys::Element,
) -> Option<String> {
    native_selected_row_attribute(document, button, "data-slskr-native-grant-id")
        .filter(|value| safe_route_segment(value))
}

#[cfg(target_arch = "wasm32")]
pub(super) fn native_shared_grant_is_selected(
    document: &web_sys::Document,
    button: &web_sys::Element,
    label: &str,
) -> bool {
    if native_shared_grant_id(document, button).is_some() {
        return true;
    }
    native_set_action_status(
        document,
        label,
        "Select a live inbound share before running this action.",
    );
    false
}

#[cfg(target_arch = "wasm32")]
pub(super) fn native_shared_token_storage_key(grant_id: &str) -> String {
    format!("slskr.shared-grant-token.{grant_id}")
}

#[cfg(target_arch = "wasm32")]
pub(super) fn native_shared_token(window: &web_sys::Window, grant_id: &str) -> Option<String> {
    window
        .session_storage()
        .ok()
        .flatten()
        .and_then(|storage| {
            storage
                .get_item(&native_shared_token_storage_key(grant_id))
                .ok()
                .flatten()
        })
        .filter(|token| !token.trim().is_empty())
}

#[cfg(target_arch = "wasm32")]
pub(super) fn native_store_shared_token(window: &web_sys::Window, grant_id: &str, token: &str) {
    if let Some(storage) = window.session_storage().ok().flatten() {
        let _ = storage.set_item(&native_shared_token_storage_key(grant_id), token);
    }
}

#[cfg(target_arch = "wasm32")]
pub(super) async fn native_issue_shared_access_token(
    window: &web_sys::Window,
    grant_id: &str,
) -> Result<String, JsValue> {
    let path = endpoint_url(&format!("/share-grants/{grant_id}/token"));
    let response =
        fetch_text_with_method(window, &path, "POST", Some(r#"{"expiresInSeconds":600}"#)).await?;
    serde_json::from_str::<serde_json::Value>(&response)
        .ok()
        .and_then(|value| {
            native_json_string_by_key(&value, &["token", "shareToken", "share_token"])
        })
        .filter(|token| !token.trim().is_empty())
        .ok_or_else(|| JsValue::from_str("the token endpoint returned no token"))
}

#[cfg(target_arch = "wasm32")]
pub(super) fn native_issue_share_token(
    window: &web_sys::Window,
    document: &web_sys::Document,
    button: &web_sys::Element,
) {
    let Some(grant_id) = native_shared_grant_id(document, button) else {
        native_set_action_status(document, "Copy token", "Select a live shared grant first.");
        return;
    };
    native_set_action_status(document, "Copy token", "Issuing a short-lived share token.");
    let window_for_request = window.clone();
    let document_for_request = document.clone();
    let grant_id_for_request = grant_id.clone();
    wasm_bindgen_futures::spawn_local(async move {
        match native_issue_shared_access_token(&window_for_request, &grant_id_for_request).await {
            Ok(token) => {
                native_store_shared_token(&window_for_request, &grant_id_for_request, &token);
                copy_reference_text(&window_for_request, &document_for_request, token);
            }
            Err(error) => native_set_action_status(
                &document_for_request,
                "Copy token",
                &error
                    .as_string()
                    .unwrap_or_else(|| "share token request failed".to_string()),
            ),
        }
    });
}

#[cfg(target_arch = "wasm32")]
pub(super) fn native_copy_shared_manifest(
    window: &web_sys::Window,
    document: &web_sys::Document,
    button: &web_sys::Element,
) {
    let Some(grant_id) = native_shared_grant_id(document, button) else {
        native_set_action_status(
            document,
            "Copy manifest",
            "Select a live shared grant first.",
        );
        return;
    };
    let window_for_request = window.clone();
    let document_for_request = document.clone();
    let stored_token = native_shared_token(window, &grant_id);
    native_set_action_status(document, "Copy manifest", "Loading the live manifest.");
    wasm_bindgen_futures::spawn_local(async move {
        let path = endpoint_url(&format!("/share-grants/{grant_id}/manifest"));
        let headers = stored_token
            .as_deref()
            .map(|token| [("X-Share-Token", token)])
            .unwrap_or_default();
        match fetch_text_with_method_and_headers(&window_for_request, &path, "GET", None, &headers)
            .await
        {
            Ok(response) => {
                copy_reference_text(&window_for_request, &document_for_request, response)
            }
            Err(error) => native_set_action_status(
                &document_for_request,
                "Copy manifest",
                &error
                    .as_string()
                    .unwrap_or_else(|| "manifest request failed".to_string()),
            ),
        }
    });
}

#[cfg(target_arch = "wasm32")]
pub(super) fn native_stream_shared_manifest(
    window: &web_sys::Window,
    document: &web_sys::Document,
    button: &web_sys::Element,
) {
    let Some(grant_id) = native_shared_grant_id(document, button) else {
        native_set_action_status(document, "Stream", "Select a live shared grant first.");
        return;
    };
    let window_for_request = window.clone();
    let document_for_request = document.clone();
    native_set_action_status(document, "Stream", "Loading the live shared manifest.");
    wasm_bindgen_futures::spawn_local(async move {
        let path = endpoint_url(&format!("/share-grants/{grant_id}/manifest"));
        let stored_token = native_shared_token(&window_for_request, &grant_id);
        let manifest_headers = stored_token
            .as_deref()
            .map(|token| [("X-Share-Token", token)])
            .unwrap_or_default();
        match fetch_text_with_method_and_headers(
            &window_for_request,
            &path,
            "GET",
            None,
            &manifest_headers,
        )
        .await
        {
            Ok(response) => {
                let value = serde_json::from_str::<serde_json::Value>(&response).ok();
                let Some(value) = value else {
                    native_set_action_status(
                        &document_for_request,
                        "Stream",
                        "The shared manifest was not valid JSON.",
                    );
                    return;
                };
                let content_id = value
                    .get("items")
                    .and_then(serde_json::Value::as_array)
                    .and_then(|items| items.first())
                    .and_then(|item| {
                        ["contentId", "content_id", "id"]
                            .iter()
                            .find_map(|key| item.get(*key).map(json_scalar_preview))
                    })
                    .filter(|content_id| !content_id.trim().is_empty());
                let Some(content_id) = content_id else {
                    native_set_action_status(
                        &document_for_request,
                        "Stream",
                        "The shared manifest contains no streamable items.",
                    );
                    return;
                };
                let token = stored_token
                    .or_else(|| {
                        value.as_object().and_then(|object| {
                            ["shareToken", "share_token", "token"]
                                .iter()
                                .find_map(|key| object.get(*key).map(json_scalar_preview))
                        })
                    })
                    .filter(|token| !token.trim().is_empty());
                let token = match token {
                    Some(token) => token,
                    None => match native_issue_shared_access_token(&window_for_request, &grant_id)
                        .await
                    {
                        Ok(token) => {
                            native_store_shared_token(&window_for_request, &grant_id, &token);
                            token
                        }
                        Err(error) => {
                            native_set_action_status(
                                &document_for_request,
                                "Stream",
                                &error
                                    .as_string()
                                    .unwrap_or_else(|| "share token request failed".to_string()),
                            );
                            return;
                        }
                    },
                };
                let encoded_content_id = percent_encode_player_stream_component(&content_id);
                let ticket_path =
                    endpoint_url(&format!("/streams/{encoded_content_id}/share-ticket"));
                let ticket_headers = [("X-Share-Token", token.as_str())];
                let ticket = match fetch_text_with_method_and_headers(
                    &window_for_request,
                    &ticket_path,
                    "POST",
                    None,
                    &ticket_headers,
                )
                .await
                {
                    Ok(response) => serde_json::from_str::<serde_json::Value>(&response)
                        .ok()
                        .and_then(|value| {
                            ["ticket", "streamTicket"]
                                .iter()
                                .find_map(|key| value.get(*key).map(json_scalar_preview))
                        })
                        .filter(|ticket| !ticket.trim().is_empty()),
                    Err(error) => {
                        native_set_action_status(
                            &document_for_request,
                            "Stream",
                            &error.as_string().unwrap_or_else(|| {
                                "share stream ticket request failed".to_string()
                            }),
                        );
                        None
                    }
                };
                let Some(ticket) = ticket else {
                    if document_for_request
                        .get_element_by_id("slskr-action-status")
                        .and_then(|element| element.text_content())
                        .unwrap_or_default()
                        .is_empty()
                    {
                        native_set_action_status(
                            &document_for_request,
                            "Stream",
                            "The stream ticket endpoint returned no ticket.",
                        );
                    }
                    return;
                };
                let stream_url = endpoint_url(&format!(
                    "/streams/{encoded_content_id}?ticket={}",
                    percent_encode_player_stream_component(&ticket)
                ));
                let url = if stream_url.starts_with("http://") || stream_url.starts_with("https://")
                {
                    stream_url
                } else {
                    format!(
                        "{}{}",
                        window_for_request.location().origin().unwrap_or_default(),
                        stream_url
                    )
                };
                match window_for_request.open_with_url_and_target_and_features(
                    &url,
                    "_blank",
                    "noopener,noreferrer",
                ) {
                    Ok(Some(_)) => native_set_action_status(
                        &document_for_request,
                        "Stream",
                        "Opened the shared stream with a short-lived content ticket.",
                    ),
                    Ok(None) => native_set_action_status(
                        &document_for_request,
                        "Stream",
                        "The browser blocked the shared stream window; allow pop-ups and try again.",
                    ),
                    Err(error) => native_set_action_status(
                        &document_for_request,
                        "Stream",
                        &error
                            .as_string()
                            .unwrap_or_else(|| "unable to open the shared stream window".to_string()),
                    ),
                }
            }
            Err(error) => native_set_action_status(
                &document_for_request,
                "Stream",
                &error
                    .as_string()
                    .unwrap_or_else(|| "shared stream request failed".to_string()),
            ),
        }
    });
}
