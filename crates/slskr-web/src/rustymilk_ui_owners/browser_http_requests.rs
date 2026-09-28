use super::*;

#[cfg(target_arch = "wasm32")]
pub(super) async fn fetch_text(window: &web_sys::Window, url: &str) -> Result<String, JsValue> {
    let share_response = url.ends_with("/transfers/speeds");
    let response_value =
        wasm_bindgen_futures::JsFuture::from(fetch_text_request(window, url)).await?;
    let response: web_sys::Response = response_value.dyn_into()?;
    if !response.ok() {
        return Err(JsValue::from_str(&format!("HTTP {}", response.status())));
    }
    let response = if share_response {
        response.clone()
    } else {
        Ok(response)
    }?;
    let text = wasm_bindgen_futures::JsFuture::from(response.text()?).await?;
    Ok(text.as_string().unwrap_or_default())
}

#[cfg(target_arch = "wasm32")]
const TRANSFER_SPEEDS_REQUEST_CACHE_TTL_MS: f64 = 200.0;

#[cfg(target_arch = "wasm32")]
std::thread_local! {
    static TRANSFER_SPEEDS_REQUEST_CACHE: RefCell<Option<(f64, js_sys::Promise)>> = const {
        RefCell::new(None)
    };
}

#[cfg(target_arch = "wasm32")]
pub(super) fn fetch_text_request(window: &web_sys::Window, url: &str) -> js_sys::Promise {
    if !url.ends_with("/transfers/speeds") {
        return window.fetch_with_str(url);
    }

    let now = js_sys::Date::now();
    TRANSFER_SPEEDS_REQUEST_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if let Some((requested_at, request)) = cache.as_ref() {
            if now >= *requested_at && now - *requested_at < TRANSFER_SPEEDS_REQUEST_CACHE_TTL_MS {
                return request.clone();
            }
        }

        let request = window.fetch_with_str(url);
        *cache = Some((now, request.clone()));
        request
    })
}

#[cfg(target_arch = "wasm32")]
pub(super) async fn fetch_text_with_method(
    window: &web_sys::Window,
    url: &str,
    method: &str,
    body: Option<&str>,
) -> Result<String, JsValue> {
    fetch_text_with_method_and_headers(window, url, method, body, &[]).await
}

#[cfg(target_arch = "wasm32")]
pub(super) async fn fetch_text_with_method_and_headers(
    window: &web_sys::Window,
    url: &str,
    method: &str,
    body: Option<&str>,
    extra_headers: &[(&str, &str)],
) -> Result<String, JsValue> {
    let init = web_sys::RequestInit::new();
    init.set_method(method);
    if body.is_some() || !extra_headers.is_empty() {
        let headers = web_sys::Headers::new()?;
        if body.is_some() {
            headers.set("Content-Type", "application/json")?;
        }
        for (name, value) in extra_headers {
            headers.set(name, value)?;
        }
        init.set_headers(&headers);
    }
    if let Some(body) = body {
        init.set_body(&JsValue::from_str(body));
    }
    let response_value =
        wasm_bindgen_futures::JsFuture::from(window.fetch_with_str_and_init(url, &init)).await?;
    let response: web_sys::Response = response_value.dyn_into()?;
    if !response.ok() {
        return Err(JsValue::from_str(&format!("HTTP {}", response.status())));
    }
    let text = wasm_bindgen_futures::JsFuture::from(response.text()?).await?;
    Ok(text.as_string().unwrap_or_default())
}
