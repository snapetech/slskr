use wasm_bindgen::prelude::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NavItem {
    pub href: &'static str,
    pub icon: &'static str,
    pub label: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AppSection {
    pub description: &'static str,
    pub endpoint: &'static str,
    pub title: &'static str,
}

pub const fn api_base_path() -> &'static str {
    "/api/v0"
}

pub const fn nav_items() -> &'static [NavItem] {
    &[
        NavItem {
            href: "#search",
            icon: "search",
            label: "Search",
        },
        NavItem {
            href: "#transfers",
            icon: "download",
            label: "Transfers",
        },
        NavItem {
            href: "#messages",
            icon: "message",
            label: "Messages",
        },
        NavItem {
            href: "#rooms",
            icon: "users",
            label: "Rooms",
        },
        NavItem {
            href: "#browse",
            icon: "folder",
            label: "Browse",
        },
        NavItem {
            href: "#system",
            icon: "settings",
            label: "System",
        },
    ]
}

pub const fn app_sections() -> &'static [AppSection] {
    &[
        AppSection {
            description: "Create searches, review result counts, and open discovery context.",
            endpoint: "/searches",
            title: "Search",
        },
        AppSection {
            description: "Track downloads and uploads with queue, speed, and status state.",
            endpoint: "/transfers",
            title: "Transfers",
        },
        AppSection {
            description: "Read private messages, room activity, and acknowledgement state.",
            endpoint: "/messages",
            title: "Messages",
        },
        AppSection {
            description: "Inspect joined rooms, available rooms, users, and recent messages.",
            endpoint: "/rooms",
            title: "Rooms",
        },
        AppSection {
            description: "Request peer browse data and display cached shared folders.",
            endpoint: "/browse",
            title: "Browse",
        },
        AppSection {
            description:
                "Show daemon health, version, metrics, telemetry, and configuration state.",
            endpoint: "/telemetry",
            title: "System",
        },
    ]
}

pub fn endpoint_url(endpoint: &str) -> String {
    format!("{}{}", api_base_path(), endpoint)
}

pub fn shell_html() -> String {
    let nav = nav_items()
        .iter()
        .map(|item| {
            format!(
                r#"<a class="slskr-nav-item" href="{href}" title="{label}"><span class="slskr-nav-icon">{icon}</span><span>{label}</span></a>"#,
                href = item.href,
                icon = item.icon,
                label = item.label
            )
        })
        .collect::<Vec<_>>()
        .join("");

    let sections = app_sections()
        .iter()
        .map(|section| {
            format!(
                r#"<section class="slskr-panel" id="{id}"><div><h2>{title}</h2><p>{description}</p></div><code>{endpoint}</code></section>"#,
                id = section.title.to_ascii_lowercase(),
                title = section.title,
                description = section.description,
                endpoint = endpoint_url(section.endpoint)
            )
        })
        .collect::<Vec<_>>()
        .join("");

    format!(
        r#"<div class="slskr-shell"><nav class="slskr-nav">{nav}</nav><main class="slskr-main"><header class="slskr-hero"><p class="slskr-kicker">Rust web migration target</p><h1>slskr</h1><p>Native Rust/WASM app shell for porting the existing browser UI one route at a time.</p></header><div class="slskr-grid">{sections}</div></main></div>"#
    )
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("window is unavailable"))?;
    let document = window
        .document()
        .ok_or_else(|| JsValue::from_str("document is unavailable"))?;
    let root = document
        .get_element_by_id("root")
        .ok_or_else(|| JsValue::from_str("#root is missing"))?;
    root.set_inner_html(&shell_html());
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
#[wasm_bindgen]
pub fn start() -> Result<(), JsValue> {
    Ok(())
}

#[wasm_bindgen(js_name = renderShellHtml)]
pub fn render_shell_html() -> String {
    shell_html()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_endpoints_are_versioned() {
        for section in app_sections() {
            assert!(endpoint_url(section.endpoint).starts_with("/api/v0/"));
        }
    }

    #[test]
    fn shell_contains_primary_routes() {
        let html = shell_html();
        for item in nav_items() {
            assert!(html.contains(item.label), "missing {}", item.label);
        }
        assert!(html.contains("Rust/WASM"));
        assert!(html.contains("/api/v0/searches"));
    }
}
