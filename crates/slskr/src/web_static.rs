use super::*;

fn web_static_content_type(path: &Path) -> &'static str {
    match path.extension().and_then(|extension| extension.to_str()) {
        Some("css") => "text/css; charset=utf-8",
        Some("eot") => "application/vnd.ms-fontobject",
        Some("html") => "text/html; charset=utf-8",
        Some("ico") => "image/x-icon",
        Some("js") => "text/javascript; charset=utf-8",
        Some("json") | Some("webmanifest") => "application/json; charset=utf-8",
        Some("png") => "image/png",
        Some("svg") => "image/svg+xml",
        Some("ttf") => "font/ttf",
        Some("txt") => "text/plain; charset=utf-8",
        Some("wasm") => "application/wasm",
        Some("woff") => "font/woff",
        Some("woff2") => "font/woff2",
        _ => "application/octet-stream",
    }
}

fn inject_web_runtime_profile(
    bytes: Vec<u8>,
    file: &Path,
    runtime_profile: ControllerProfile,
    expose_runtime_profile: bool,
    csp_nonce: Option<&str>,
) -> Vec<u8> {
    if !expose_runtime_profile
        || file.file_name().and_then(|name| name.to_str()) != Some("index.html")
    {
        return bytes;
    }

    let marker = b"<head>";
    let Some(marker_start) = bytes
        .windows(marker.len())
        .position(|candidate| candidate == marker)
    else {
        return bytes;
    };
    let insertion_point = marker_start + marker.len();
    let mut metadata = format!(
        "<meta name=\"slskr-runtime-profile\" content=\"{}\">",
        runtime_profile.as_str()
    );
    if let Some(csp_nonce) = csp_nonce {
        metadata.push_str(&format!(
            "<meta name=\"csp-nonce\" content=\"{csp_nonce}\">"
        ));
    }
    let mut output = Vec::with_capacity(bytes.len() + metadata.len());
    output.extend_from_slice(&bytes[..insertion_point]);
    output.extend_from_slice(metadata.as_bytes());
    output.extend_from_slice(&bytes[insertion_point..]);
    output
}

pub(super) fn web_static_file_for_request(
    path: &str,
    configured_content_path: Option<&Path>,
    runtime_profile: Option<ControllerProfile>,
) -> Option<(PathBuf, PathBuf, &'static str)> {
    if path.starts_with("/api/") || path.starts_with("/hub/") || path == "/api" || path == "/hub" {
        return None;
    }
    let path_without_query = path.split_once('?').map_or(path, |(path, _)| path);
    if matches!(path_without_query, "/health" | "/health/mesh") {
        return None;
    }

    let root = web_build_root(configured_content_path, runtime_profile)?
        .canonicalize()
        .ok()?;
    let (file, content_type) = web_static_file_for_request_under_root(&root, path)?;
    Some((root, file, content_type))
}

pub(super) fn is_spa_navigation_path(path: &str) -> bool {
    let path_without_query = path.split_once('?').map_or(path, |(path, _)| path);
    if path_without_query == "/"
        || path_without_query == "/dashboard"
        || path_without_query == "/api"
        || path_without_query == "/hub"
        || path_without_query.starts_with("/api/")
        || path_without_query.starts_with("/hub/")
        || path_without_query == "/mesh/http"
        || path_without_query.starts_with("/mesh/http/")
    {
        return false;
    }

    Path::new(path_without_query.trim_start_matches('/'))
        .extension()
        .is_none()
}

pub(super) fn web_static_file_for_request_under_root(
    root: &Path,
    path: &str,
) -> Option<(PathBuf, &'static str)> {
    let canonical_root = root.canonicalize().ok()?;
    let path_without_query = path.split_once('?').map_or(path, |(path, _)| path);
    let relative = path_without_query.trim_start_matches('/');
    let requested = if relative.is_empty() {
        PathBuf::from("index.html")
    } else {
        let relative_path = Path::new(relative);
        if relative_path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        }) {
            return None;
        }

        resolve_web_static_relative_path(root, relative_path)?
    };

    let file = root.join(requested);
    if !file.is_file() {
        return None;
    }
    let canonical_file = file.canonicalize().ok()?;
    if !canonical_file.starts_with(canonical_root) {
        return None;
    }
    let content_type = web_static_content_type(&file);
    Some((canonical_file, content_type))
}

fn resolve_web_static_relative_path(root: &Path, relative_path: &Path) -> Option<PathBuf> {
    let candidate = root.join(relative_path);
    if candidate.is_file() {
        return Some(relative_path.to_path_buf());
    }
    if relative_path.extension().is_none() {
        return Some(PathBuf::from("index.html"));
    }

    let components = relative_path
        .components()
        .filter_map(|component| match component {
            Component::Normal(value) => Some(value),
            _ => None,
        })
        .collect::<Vec<_>>();

    if let Some(asset_index) = components
        .iter()
        .position(|component| matches!(component.to_string_lossy().as_ref(), "assets" | "static"))
    {
        let nested_asset = components[asset_index..].iter().collect::<PathBuf>();
        if root.join(&nested_asset).is_file() {
            return Some(nested_asset);
        }
    }

    if components.len() > 1 {
        let basename = components.last()?.to_owned();
        let root_file = PathBuf::from(basename);
        if root.join(&root_file).is_file() {
            return Some(root_file);
        }
    }

    None
}

pub(super) fn read_web_index_html() -> Option<String> {
    let runtime_profile = env::var("SLSKR_CONTROLLER_PROFILE")
        .ok()
        .and_then(|value| ControllerProfile::parse(&value).ok());
    let (root, path, _) = web_static_file_for_request("/", None, runtime_profile)?;
    let bytes = read_bounded_web_static_file_under_root(&root, &path).ok()?;
    String::from_utf8(bytes).ok()
}

fn security_headers(file: &Path, csp_nonce: Option<&str>) -> String {
    let content_security_policy = csp_nonce.map_or_else(
        || web_static_content_security_policy(file).to_owned(),
        |nonce| {
            web_static_content_security_policy(file).replacen(
                "style-src 'self'",
                &format!("style-src 'self' 'nonce-{nonce}'"),
                1,
            )
        },
    );
    format!(
        "X-Content-Type-Options: nosniff\r\n\
Referrer-Policy: no-referrer\r\n\
Content-Security-Policy: {}\r\n\
Strict-Transport-Security: max-age=31536000; includeSubDomains\r\n",
        content_security_policy
    )
}

fn web_static_csp_nonce() -> Result<String, String> {
    let mut bytes = [0_u8; 16];
    SysRng
        .try_fill_bytes(&mut bytes)
        .map_err(|_| "operating system randomness is unavailable".to_owned())?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

pub(super) fn web_static_content_security_policy(file: &Path) -> &'static str {
    // connect-src allows any ws:/wss: target already (mesh/peer WebSocket
    // connections aren't known in advance); http:/https: get the same trust
    // level so a viewer can fetch a share's manifest/stream/backfill
    // directly from the announcing peer's own node, whose address is
    // likewise never known ahead of time.
    if is_rust_wasm_shell(file) {
        return "default-src 'self'; base-uri 'self'; frame-ancestors 'none'; object-src 'none'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self' 'sha256-4QRv9Rc4tDEpbP/UKi2dy9R69BTTCtbO26VNBih7vEw=' 'sha256-AVTm08UMHPqpttgoudpSsvenKKfidtwuSnUVJLIuqcA='; style-src-attr 'unsafe-inline'; img-src 'self' data:; connect-src 'self' http: https: ws: wss:";
    }

    "default-src 'self'; base-uri 'self'; frame-ancestors 'none'; object-src 'none'; script-src 'self'; style-src 'self' https://fonts.googleapis.com 'sha256-4QRv9Rc4tDEpbP/UKi2dy9R69BTTCtbO26VNBih7vEw=' 'sha256-AVTm08UMHPqpttgoudpSsvenKKfidtwuSnUVJLIuqcA='; style-src-attr 'unsafe-inline'; font-src 'self' data: https://fonts.gstatic.com; img-src 'self' data:; connect-src 'self' http: https: ws: wss:"
}

fn is_rust_wasm_shell(file: &Path) -> bool {
    file.file_name().and_then(|name| name.to_str()) == Some("index.html")
        && file
            .parent()
            .is_some_and(|parent| parent.join("slskr_web.wasm").is_file())
}

#[cfg(any(test, feature = "bounded-differential", not(unix)))]
#[allow(dead_code)]
pub(super) fn read_bounded_web_static_file(file: &Path) -> Result<Vec<u8>, String> {
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    let file = options.open(file).map_err(|error| error.to_string())?;
    read_bounded_web_static_handle(file)
}

#[cfg(unix)]
pub(super) fn read_bounded_web_static_file_under_root(
    root: &Path,
    file: &Path,
) -> Result<Vec<u8>, String> {
    use rustix::fs::{open, openat, Mode, OFlags};

    let relative = file
        .strip_prefix(root)
        .map_err(|_| "static asset is outside the web root".to_owned())?;
    let components = relative
        .components()
        .map(|component| match component {
            Component::Normal(value) => Ok(value),
            _ => Err("static asset contains a non-relative component".to_owned()),
        })
        .collect::<Result<Vec<_>, _>>()?;
    let (filename, parents) = components
        .split_last()
        .ok_or_else(|| "static asset path is empty".to_owned())?;
    let directory_flags = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
    let mut directory = open(root, directory_flags, Mode::empty())
        .map_err(|error| format!("static root confined open failed: {error}"))?;
    for component in parents {
        directory = openat(&directory, *component, directory_flags, Mode::empty())
            .map_err(|error| format!("static directory confined open failed: {error}"))?;
    }
    let file = openat(
        &directory,
        *filename,
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|error| format!("static file confined open failed: {error}"))?;
    read_bounded_web_static_handle(fs::File::from(file))
}

#[cfg(not(unix))]
pub(super) fn read_bounded_web_static_file_under_root(
    root: &Path,
    file: &Path,
) -> Result<Vec<u8>, String> {
    let canonical_root = root.canonicalize().map_err(|error| error.to_string())?;
    let canonical_file = file.canonicalize().map_err(|error| error.to_string())?;
    if !canonical_file.starts_with(canonical_root) {
        return Err("static asset is outside the web root".to_owned());
    }
    read_bounded_web_static_file(&canonical_file)
}

fn read_bounded_web_static_handle(mut file: fs::File) -> Result<Vec<u8>, String> {
    use std::io::Read;

    let metadata = file.metadata().map_err(|error| error.to_string())?;
    if !metadata.is_file() {
        return Err("static asset is not a regular file".to_owned());
    }
    if metadata.len() > MAX_WEB_STATIC_BYTES {
        return Err(format!(
            "static asset is too large: {} bytes, max is {MAX_WEB_STATIC_BYTES}",
            metadata.len()
        ));
    }
    let mut bytes = Vec::new();
    file.by_ref()
        .take(MAX_WEB_STATIC_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    if bytes.len() as u64 > MAX_WEB_STATIC_BYTES {
        return Err(format!(
            "static asset is too large: more than {MAX_WEB_STATIC_BYTES} bytes"
        ));
    }
    Ok(bytes)
}

#[cfg(any(test, feature = "bounded-differential"))]
#[allow(dead_code)]
pub(super) fn read_bounded_web_static_string(file: &Path) -> Result<String, String> {
    let bytes = read_bounded_web_static_file(file)?;
    String::from_utf8(bytes).map_err(|error| format!("static asset is not UTF-8: {error}"))
}

pub(super) fn web_static_error_response(error: &str) -> HttpResponse {
    let (status, message) = if error.contains("too large") {
        ("413 Payload Too Large", "static asset is too large")
    } else {
        ("500 Internal Server Error", "static asset is unavailable")
    };
    routing::HttpResponse {
        status,
        content_type: "application/json",
        body: format!("{{\"error\":\"{message}\"}}"),
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn write_web_static_response<W: tokio::io::AsyncWrite + Unpin>(
    writer: &mut W,
    path: &str,
    configured_content_path: Option<&Path>,
    runtime_profile: ControllerProfile,
    expose_runtime_profile: bool,
    include_body: bool,
    keep_alive: bool,
    extra_headers: &str,
) -> Result<Option<usize>, String> {
    let Some((root, file, content_type)) =
        web_static_file_for_request(path, configured_content_path, Some(runtime_profile))
    else {
        return Ok(None);
    };
    let csp_nonce = if content_type.starts_with("text/html") {
        Some(web_static_csp_nonce()?)
    } else {
        None
    };
    let bytes = inject_web_runtime_profile(
        read_bounded_web_static_file_under_root(&root, &file)?,
        &file,
        runtime_profile,
        expose_runtime_profile,
        csp_nonce.as_deref(),
    );
    let connection_header = if keep_alive { "keep-alive" } else { "close" };
    let headers = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: {connection_header}\r\n{}{}\r\n",
        bytes.len(),
        security_headers(&file, csp_nonce.as_deref()),
        extra_headers,
    );
    time::timeout(http_server::RESPONSE_WRITE_TIMEOUT, async {
        writer
            .write_all(headers.as_bytes())
            .await
            .map_err(|error| error.to_string())?;
        if include_body {
            writer
                .write_all(&bytes)
                .await
                .map_err(|error| error.to_string())?;
        }
        writer.flush().await.map_err(|error| error.to_string())
    })
    .await
    .map_err(|_| "static response write deadline exceeded".to_owned())??;
    Ok(Some(bytes.len()))
}

pub(super) fn fallback_dashboard_html() -> String {
    r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>slskR</title>
  <style>
    :root {
      color-scheme: dark;
      --bg: #101317;
      --panel: #181d22;
      --panel-soft: #20262c;
      --line: #313941;
      --text: #edf2f5;
      --muted: #a9b4bd;
      --green: #74c69d;
      --blue: #8ab4f8;
      --amber: #f2c66d;
      --red: #ff8a80;
    }
    * { box-sizing: border-box; }
    body {
      margin: 0;
      min-width: 320px;
      font-family: system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
      background: var(--bg);
      color: var(--text);
    }
    main {
      width: min(1180px, calc(100% - 32px));
      margin: 0 auto;
      padding: 24px 0 40px;
    }
    header {
      display: flex;
      align-items: flex-end;
      justify-content: space-between;
      gap: 16px;
      padding: 0 0 18px;
      border-bottom: 1px solid var(--line);
    }
    h1, h2 {
      margin: 0;
      letter-spacing: 0;
    }
    h1 { font-size: 28px; line-height: 1.1; }
    h2 { font-size: 15px; line-height: 1.3; }
    .version { color: var(--muted); font-size: 13px; margin-top: 6px; }
    .status {
      display: inline-flex;
      align-items: center;
      gap: 8px;
      min-height: 30px;
      padding: 5px 10px;
      border: 1px solid var(--line);
      border-radius: 6px;
      background: var(--panel);
      color: var(--muted);
      font-size: 13px;
      white-space: nowrap;
    }
    .notice {
      margin-top: 18px;
      padding: 12px 14px;
      border: 1px solid var(--amber);
      border-radius: 8px;
      background: rgba(242, 198, 109, .12);
      color: var(--text);
      font-size: 14px;
      line-height: 1.45;
    }
    .notice a {
      color: var(--blue);
      font-weight: 700;
    }
    .dot {
      width: 8px;
      height: 8px;
      border-radius: 50%;
      background: var(--amber);
    }
    .dot.online { background: var(--green); }
    .dot.error { background: var(--red); }
    .grid {
      display: grid;
      grid-template-columns: repeat(4, minmax(0, 1fr));
      gap: 12px;
      margin-top: 18px;
    }
    .panel {
      background: var(--panel);
      border: 1px solid var(--line);
      border-radius: 8px;
      padding: 14px;
      min-width: 0;
    }
    .metric {
      min-height: 96px;
    }
    .label {
      color: var(--muted);
      font-size: 12px;
      text-transform: uppercase;
      letter-spacing: .08em;
    }
    .value {
      margin-top: 8px;
      font-size: 30px;
      line-height: 1;
      font-weight: 720;
      overflow-wrap: anywhere;
    }
    .sub {
      margin-top: 8px;
      color: var(--muted);
      font-size: 13px;
      overflow-wrap: anywhere;
    }
    .wide { grid-column: span 2; }
    .full { grid-column: 1 / -1; }
    .toolbar {
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 12px;
      margin-bottom: 12px;
    }
    .filters {
      display: flex;
      flex-wrap: wrap;
      justify-content: flex-end;
      gap: 8px;
      min-width: 0;
    }
    .filters input, .filters select {
      width: 150px;
      min-height: 32px;
      font-size: 13px;
    }
    .actions {
      display: grid;
      grid-template-columns: repeat(3, minmax(0, 1fr));
      gap: 12px;
    }
    form {
      display: grid;
      grid-template-columns: 1fr auto;
      gap: 8px;
      margin-top: 12px;
    }
    .stack {
      display: grid;
      grid-template-columns: 1fr;
      gap: 8px;
    }
    input, select {
      width: 100%;
      min-height: 34px;
      border: 1px solid var(--line);
      border-radius: 6px;
      background: var(--panel-soft);
      color: var(--text);
      padding: 6px 9px;
      font: inherit;
      min-width: 0;
    }
    input:focus, select:focus, button:focus {
      outline: 2px solid var(--blue);
      outline-offset: 1px;
    }
    button {
      border: 1px solid var(--line);
      border-radius: 6px;
      background: var(--panel-soft);
      color: var(--text);
      min-height: 32px;
      padding: 6px 10px;
      font: inherit;
      cursor: pointer;
    }
    button:hover { border-color: var(--blue); }
    .button-row {
      display: flex;
      flex-wrap: wrap;
      gap: 6px;
    }
    .button-row button {
      min-height: 28px;
      padding: 4px 8px;
      font-size: 12px;
    }
    table {
      width: 100%;
      border-collapse: collapse;
      table-layout: fixed;
      font-size: 13px;
    }
    th, td {
      padding: 9px 8px;
      border-top: 1px solid var(--line);
      text-align: left;
      vertical-align: top;
      overflow-wrap: anywhere;
    }
    th {
      color: var(--muted);
      font-weight: 650;
      font-size: 12px;
    }
    .empty {
      color: var(--muted);
      padding: 10px 0 0;
      font-size: 13px;
    }
    .toast {
      min-height: 20px;
      color: var(--muted);
      font-size: 13px;
      margin-top: 8px;
      overflow-wrap: anywhere;
    }
    @media (max-width: 900px) {
      .grid { grid-template-columns: repeat(2, minmax(0, 1fr)); }
      .wide { grid-column: 1 / -1; }
      .actions { grid-template-columns: 1fr; }
    }
    @media (max-width: 560px) {
      main { width: min(100% - 20px, 1180px); padding-top: 16px; }
      header { align-items: flex-start; flex-direction: column; }
      .grid { grid-template-columns: 1fr; }
      .metric { min-height: 86px; }
      .value { font-size: 26px; }
      table { font-size: 12px; }
      th, td { padding: 8px 6px; }
    }
  </style>
</head>
<body>
  <main>
    <header>
      <div>
        <h1>slskr</h1>
        <div class="version">Fallback dashboard · __VERSION__</div>
      </div>
      <div class="status"><span id="status-dot" class="dot"></span><span id="status-text">Loading</span></div>
    </header>
    <section class="notice">
      This is the minimal fallback dashboard. The full React Web UI is the default at
      <a href="/">/</a> when production assets are installed. If this page appears at
      the root URL, build or install <code>web/build</code>, or set
      <code>SLSKR_WEB_BUILD_DIR</code> to the built UI directory.
    </section>

    <section class="grid" aria-live="polite">
      <article class="panel metric"><div class="label">Session</div><div id="session-state" class="value">-</div><div id="session-sub" class="sub">-</div></article>
      <article class="panel metric"><div class="label">Shares</div><div id="share-files" class="value">-</div><div id="share-sub" class="sub">-</div></article>
      <article class="panel metric"><div class="label">Searches</div><div id="search-count" class="value">-</div><div id="search-sub" class="sub">-</div></article>
      <article class="panel metric"><div class="label">Transfers</div><div id="transfer-count" class="value">-</div><div id="transfer-sub" class="sub">-</div></article>
      <article class="panel metric"><div class="label">Listeners</div><div id="listener-count" class="value">-</div><div id="listener-sub" class="sub">-</div></article>
      <article class="panel metric"><div class="label">Browse Cache</div><div id="browse-count" class="value">-</div><div id="browse-sub" class="sub">-</div></article>
      <article class="panel metric"><div class="label">Messages</div><div id="message-count" class="value">-</div><div id="message-sub" class="sub">-</div></article>
      <article class="panel metric"><div class="label">Rooms</div><div id="room-count" class="value">-</div><div id="room-sub" class="sub">-</div></article>

      <section class="actions full">
        <article class="panel">
          <h2>Session</h2>
          <div class="button-row" id="session-actions">
            <button type="button" data-session-action="connect">Connect</button>
            <button type="button" data-session-action="ping">Ping</button>
            <button type="button" data-session-action="privileges/check">Privileges</button>
            <button type="button" data-session-action="disconnect">Disconnect</button>
          </div>
          <div id="session-action-status" class="toast"></div>
        </article>
        <article class="panel">
          <h2>Search</h2>
          <form id="search-form">
            <div class="stack">
              <input id="search-query" name="query" autocomplete="off" placeholder="artist album track">
              <select id="search-target" name="target">
                <option value="global">Global</option>
                <option value="wishlist">Wishlist</option>
                <option value="user">User</option>
                <option value="room">Room</option>
              </select>
              <input id="search-target-name" name="target_name" autocomplete="off" placeholder="user or room">
            </div>
            <button type="submit">Start</button>
          </form>
          <div id="search-action-status" class="toast"></div>
        </article>
        <article class="panel">
          <h2>Watch User</h2>
          <form id="watch-form">
            <input id="watch-username" name="username" autocomplete="off" placeholder="username">
            <div class="button-row">
              <button type="submit">Watch</button>
              <button id="unwatch-button" type="button">Unwatch</button>
            </div>
          </form>
          <div id="watch-action-status" class="toast"></div>
        </article>
        <article class="panel">
          <h2>Browse User</h2>
          <form id="browse-request-form">
            <input id="browse-username" name="username" autocomplete="off" placeholder="username">
            <input id="browse-folder" name="folder" autocomplete="off" placeholder="folder">
            <button type="submit">Browse</button>
          </form>
          <div id="browse-action-status" class="toast"></div>
        </article>
        <article class="panel">
          <h2>Share Scan</h2>
          <form id="share-rescan-form">
            <button type="submit">Rescan</button>
          </form>
          <div id="share-action-status" class="toast"></div>
        </article>
        <article class="panel">
          <h2>Transfer</h2>
          <form id="transfer-form">
            <div class="stack">
              <input id="transfer-filename" name="filename" autocomplete="off" placeholder="remote/file.ext">
              <input id="transfer-peer" name="peer" autocomplete="off" placeholder="peer">
              <input id="transfer-local-path" name="local_path" autocomplete="off" placeholder="local path">
              <input id="transfer-size" name="size" inputmode="numeric" autocomplete="off" placeholder="size bytes">
              <input id="transfer-progress" name="progress" inputmode="numeric" autocomplete="off" placeholder="progress bytes">
              <select id="transfer-direction" name="direction">
                <option value="0">Download</option>
                <option value="1">Upload</option>
              </select>
            </div>
            <button type="submit">Queue</button>
          </form>
          <div id="transfer-action-status" class="toast"></div>
        </article>
        <article class="panel">
          <h2>Message</h2>
          <form id="message-form">
            <div class="stack">
              <input id="message-username" name="username" autocomplete="off" placeholder="username">
              <input id="message-body" name="body" autocomplete="off" placeholder="message">
            </div>
            <button type="submit">Send</button>
          </form>
          <div id="message-action-status" class="toast"></div>
        </article>
        <article class="panel">
          <h2>Join Room</h2>
          <form id="room-join-form">
            <input id="room-join-name" name="room" autocomplete="off" placeholder="room">
            <button type="submit">Join</button>
          </form>
          <div id="room-join-action-status" class="toast"></div>
        </article>
        <article class="panel">
          <h2>Room Message</h2>
          <form id="room-message-form">
            <div class="stack">
              <input id="room-message-name" name="room" autocomplete="off" placeholder="room">
              <input id="room-message-username" name="username" autocomplete="off" placeholder="username">
              <input id="room-message-body" name="body" autocomplete="off" placeholder="message">
            </div>
            <button type="submit">Send</button>
          </form>
          <div id="room-message-action-status" class="toast"></div>
        </article>
        <article class="panel">
          <h2>Browser Session</h2>
          <form id="token-form">
            <input id="api-token" name="token" type="password" autocomplete="current-password" autocapitalize="none" spellcheck="false" placeholder="bearer token">
            <button type="submit">Sign in</button>
          </form>
          <div id="token-action-status" class="toast"></div>
        </article>
      </section>

      <section class="panel wide">
        <div class="toolbar">
          <h2>Recent Searches</h2>
          <div class="filters">
            <input id="search-filter-q" autocomplete="off" placeholder="filter">
            <select id="search-filter-status">
              <option value="">Any status</option>
              <option value="active">Active</option>
              <option value="completed">Completed</option>
            </select>
            <button id="refresh-searches" type="button">Refresh</button>
          </div>
        </div>
        <div id="search-table"></div>
      </section>
      <section class="panel wide">
        <div class="toolbar">
          <h2>Transfer Queue</h2>
          <div class="filters">
            <input id="transfer-filter-q" autocomplete="off" placeholder="filter">
            <select id="transfer-filter-status">
              <option value="">Any status</option>
              <option value="queued">Queued</option>
              <option value="in_progress">In progress</option>
              <option value="succeeded">Succeeded</option>
              <option value="cancelled">Cancelled</option>
              <option value="failed">Failed</option>
            </select>
            <button id="refresh-transfers" type="button">Refresh</button>
          </div>
        </div>
        <div id="transfer-table"></div>
      </section>
      <section class="panel wide">
        <div class="toolbar"><h2>Users</h2><button id="refresh-users" type="button">Refresh</button></div>
        <div id="user-table"></div>
      </section>
      <section class="panel wide">
        <div class="toolbar">
          <h2>Share Catalog</h2>
          <div class="filters">
            <input id="share-filter-q" autocomplete="off" placeholder="filter">
            <input id="share-filter-extension" autocomplete="off" placeholder="extension">
            <button id="refresh-shares" type="button">Refresh</button>
          </div>
        </div>
        <div id="share-table"></div>
      </section>
      <section class="panel wide">
        <div class="toolbar">
          <h2>Messages</h2>
          <div class="filters">
            <input id="message-filter-q" autocomplete="off" placeholder="filter">
            <select id="message-filter-direction">
              <option value="">Any direction</option>
              <option value="inbound">Inbound</option>
              <option value="outbound">Outbound</option>
            </select>
            <button id="refresh-messages" type="button">Refresh</button>
          </div>
        </div>
        <div id="message-table"></div>
      </section>
      <section class="panel wide">
        <div class="toolbar">
          <h2>Rooms</h2>
          <div class="filters">
            <input id="room-filter-q" autocomplete="off" placeholder="filter">
            <select id="room-filter-joined">
              <option value="">Any room</option>
              <option value="true">Joined</option>
              <option value="false">Not joined</option>
            </select>
            <button id="refresh-rooms" type="button">Refresh</button>
            <button id="sync-rooms" type="button">Sync</button>
          </div>
        </div>
        <div id="room-table"></div>
      </section>
      <section class="panel full">
        <div class="toolbar">
          <h2>Browse Cache</h2>
          <div class="filters">
            <input id="browse-filter-q" autocomplete="off" placeholder="filter">
            <select id="browse-filter-status">
              <option value="">Any status</option>
              <option value="requested">Requested</option>
              <option value="indirect_pending">Indirect</option>
              <option value="partial">Partial</option>
              <option value="ready">Ready</option>
              <option value="failed">Failed</option>
            </select>
            <button id="refresh-browse" type="button">Refresh</button>
          </div>
        </div>
        <div id="browse-table"></div>
      </section>
    </section>
  </main>
  <script>
    const text = (id, value) => { document.getElementById(id).textContent = value; };
    const number = (value) => new Intl.NumberFormat().format(value || 0);
    let apiToken = "";
    const bytes = (value) => {
      let amount = value || 0;
      const units = ["B", "KiB", "MiB", "GiB", "TiB"];
      let index = 0;
      while (amount >= 1024 && index < units.length - 1) {
        amount = amount / 1024;
        index += 1;
      }
      return `${amount.toFixed(index === 0 ? 0 : 1)} ${units[index]}`;
    };
    const requestHeaders = (extra = {}) => {
      const headers = { ...extra };
      if (apiToken) headers.authorization = `Bearer ${apiToken}`;
      return headers;
    };
    const fetchJson = async (path) => {
      const response = await fetch(path, { headers: requestHeaders({ "accept": "application/json" }) });
      if (!response.ok) throw new Error(`${response.status} ${response.statusText}`);
      return response.json();
    };
    const deleteJson = async (path) => {
      const response = await fetch(path, {
        method: "DELETE",
        headers: requestHeaders({ "accept": "application/json" })
      });
      if (!response.ok) throw new Error(`${response.status} ${response.statusText}`);
      return response.json();
    };
    const postJson = async (path, body) => {
      const response = await fetch(path, {
        method: "POST",
        headers: requestHeaders({ "accept": "application/json", "content-type": "application/json" }),
        body: JSON.stringify(body)
      });
      if (!response.ok) {
        let message = `${response.status} ${response.statusText}`;
        try {
          const error = await response.json();
          if (error.error) message = error.error;
        } catch (_) {}
        throw new Error(message);
      }
      return response.json();
    };
    const escapeHtml = (value) => String(value ?? "").replace(/[&<>"']/g, (char) => ({
      "&": "&amp;",
      "<": "&lt;",
      ">": "&gt;",
      "\"": "&quot;",
      "'": "&#39;"
    })[char]);
    const table = (rows, columns, empty) => {
      if (!rows.length) return `<div class="empty">${escapeHtml(empty)}</div>`;
      const head = columns.map((column) => `<th>${escapeHtml(column.label)}</th>`).join("");
      const body = rows.map((row) => `<tr>${columns.map((column) => {
        const value = column.value(row);
        return `<td>${column.html ? value : escapeHtml(value)}</td>`;
      }).join("")}</tr>`).join("");
      return `<table><thead><tr>${head}</tr></thead><tbody>${body}</tbody></table>`;
    };
    const field = (id) => document.getElementById(id).value.trim();
    const queryString = (params) => {
      const query = new URLSearchParams();
      Object.entries(params).forEach(([name, value]) => {
        if (value !== undefined && value !== null && value !== "") query.set(name, value);
      });
      return query.toString();
    };
    async function loadStats() {
      const stats = await fetchJson("/api/v0/stats");
      const connected = stats.session?.connected;
      const dot = document.getElementById("status-dot");
      dot.className = `dot ${connected ? "online" : ""}`;
      text("status-text", connected ? "Connected" : stats.session?.state || "Disconnected");
      text("session-state", stats.session?.state || "-");
      const privileges = stats.session?.privileges_seconds == null ? "unknown privileges" : `${number(stats.session.privileges_seconds)}s privileges`;
      text("session-sub", `${number(stats.session?.server_messages_seen)} server messages, ${number(stats.session?.reconnects)} reconnects, ${privileges}`);
      text("share-files", number(stats.shares?.files));
      text("share-sub", `${bytes(stats.shares?.bytes)} across ${number(stats.shares?.roots)} roots`);
      text("search-count", number(stats.searches?.total));
      text("search-sub", `${number(stats.searches?.active)} active, ${number(stats.searches?.results)} results`);
      text("transfer-count", number(stats.transfers?.total));
      text("transfer-sub", `${number(stats.transfers?.in_progress)} active, ${bytes(stats.transfers?.bytes_transferred)} moved`);
      text("listener-count", number((stats.listeners?.regular_accepts || 0) + (stats.listeners?.obfuscated_accepts || 0)));
      text("listener-sub", `${number(stats.listeners?.peer_messages)} peer, ${number(stats.listeners?.errors)} errors`);
      text("browse-count", number(stats.browse?.total));
      text("browse-sub", `${number(stats.browse?.indirect_pending)} indirect, ${number(stats.browse?.partial)} partial, ${number(stats.browse?.ready)} ready, ${number(stats.browse?.failed)} failed, ${number(stats.browse?.files)} files`);
      text("message-count", number(stats.messages?.total));
      text("message-sub", `${number(stats.messages?.inbound)} in, ${number(stats.messages?.outbound)} out`);
      text("room-count", number(stats.rooms?.total));
      text("room-sub", `${number(stats.rooms?.joined)} joined, ${number(stats.rooms?.messages)} messages`);
    }
    async function loadSearches() {
      const query = queryString({
        q: field("search-filter-q"),
        status: field("search-filter-status"),
        limit: 6
      });
      const data = await fetchJson(`/api/v0/searches/records?${query}`);
      document.getElementById("search-table").innerHTML = table(data.entries || [], [
        { label: "Query", value: (row) => row.query },
        { label: "Target", value: (row) => row.target_name ? `${row.target}:${row.target_name}` : row.target },
        { label: "Status", value: (row) => row.status },
        { label: "Results", value: (row) => row.result_count },
        { label: "Actions", html: true, value: (row) => searchActions(row) }
      ], "No searches");
    }
    function searchActions(row) {
      const disabled = row.status === "completed";
      return `<div class="button-row">
        <button type="button" data-search-action="complete" data-search-token="${row.token}" ${disabled ? "disabled" : ""}>Complete</button>
      </div>`;
    }
    async function loadTransfers() {
      const query = queryString({
        q: field("transfer-filter-q"),
        status: field("transfer-filter-status"),
        limit: 6
      });
      const data = await fetchJson(`/api/v0/transfers?${query}`);
      document.getElementById("transfer-table").innerHTML = table(data.entries || [], [
        { label: "File", value: (row) => row.filename },
        { label: "Peer", value: (row) => row.peer_username || "-" },
        { label: "Status", value: (row) => row.status },
        { label: "Progress", value: (row) => `${bytes(row.bytes_transferred)} / ${row.size ? bytes(row.size) : "-"}` },
        { label: "Actions", html: true, value: (row) => transferActions(row) }
      ], "No transfers");
    }
    function transferActions(row) {
      const active = row.status === "in_progress" || row.status === "peer_lookup" || row.status === "peer_negotiating" || row.status === "accepted" || row.status === "indirect_pending";
      const disabledStart = active || row.status === "succeeded" || row.status === "cancelled" || row.status === "failed";
      const disabledFinish = row.status === "succeeded" || row.status === "cancelled" || row.status === "failed";
      return `<div class="button-row">
        <button type="button" data-transfer-action="start" data-transfer-id="${row.id}" ${disabledStart ? "disabled" : ""}>Start</button>
        <button type="button" data-transfer-action="progress" data-transfer-id="${row.id}" ${disabledFinish ? "disabled" : ""}>Progress</button>
        <button type="button" data-transfer-action="complete" data-transfer-id="${row.id}" ${disabledFinish ? "disabled" : ""}>Done</button>
        <button type="button" data-transfer-action="cancel" data-transfer-id="${row.id}" ${disabledFinish ? "disabled" : ""}>Cancel</button>
        <button type="button" data-transfer-action="fail" data-transfer-id="${row.id}" ${disabledFinish ? "disabled" : ""}>Fail</button>
      </div>`;
    }
    async function loadUsers() {
      const data = await fetchJson("/api/v0/users");
      document.getElementById("user-table").innerHTML = table(data.entries || [], [
        { label: "Username", value: (row) => row.username },
        { label: "Watched", value: (row) => row.watched ? "yes" : "no" },
        { label: "Status", value: (row) => row.status || "-" },
        { label: "Files", value: (row) => row.file_count ?? "-" },
        { label: "Speed", value: (row) => row.average_speed ? `${number(row.average_speed)}/s` : "-" },
        { label: "Actions", html: true, value: (row) => userActions(row) }
      ], "No users");
    }
    function userActions(row) {
      return `<div class="button-row">
        <button type="button" data-user-action="watch" data-username="${escapeHtml(row.username)}">Watch</button>
        <button type="button" data-user-action="stats" data-username="${escapeHtml(row.username)}">Stats</button>
        <button type="button" data-user-action="browse" data-username="${escapeHtml(row.username)}">Browse</button>
        <button type="button" data-user-action="unwatch" data-username="${escapeHtml(row.username)}" ${row.watched ? "" : "disabled"}>Unwatch</button>
      </div>`;
    }
    async function loadShares() {
      const query = queryString({
        q: field("share-filter-q"),
        extension: field("share-filter-extension"),
        limit: 8
      });
      const data = await fetchJson(`/api/v0/shares/catalog?${query}`);
      document.getElementById("share-table").innerHTML = table(data.files || [], [
        { label: "Path", value: (row) => row.path },
        { label: "Extension", value: (row) => row.extension || "-" },
        { label: "Size", value: (row) => bytes(row.size) },
        { label: "Attributes", value: (row) => row.attribute_count }
      ], "No indexed files");
    }
    async function loadMessages() {
      const query = queryString({
        q: field("message-filter-q"),
        direction: field("message-filter-direction"),
        limit: 6
      });
      const data = await fetchJson(`/api/v0/messages?${query}`);
      document.getElementById("message-table").innerHTML = table(data.entries || [], [
        { label: "User", value: (row) => row.username },
        { label: "Direction", value: (row) => row.direction },
        { label: "Body", value: (row) => row.body },
        { label: "Ack", value: (row) => row.acknowledged ? "yes" : "no" },
        { label: "Actions", html: true, value: (row) => messageActions(row) }
      ], "No messages");
    }
    function messageActions(row) {
      const disabled = row.acknowledged;
      return `<div class="button-row">
        <button type="button" data-message-action="ack" data-message-id="${row.id}" ${disabled ? "disabled" : ""}>Ack</button>
      </div>`;
    }
    async function loadRooms() {
      const query = queryString({
        q: field("room-filter-q"),
        joined: field("room-filter-joined"),
        limit: 6
      });
      const data = await fetchJson(`/api/v0/rooms?${query}`);
      document.getElementById("room-table").innerHTML = table(data.entries || [], [
        { label: "Room", value: (row) => row.name },
        { label: "Joined", value: (row) => row.joined ? "yes" : "no" },
        { label: "Users", value: (row) => row.user_count ?? "-" },
        { label: "Messages", value: (row) => row.message_count },
        { label: "Last", value: (row) => (row.messages || []).slice(-1)[0]?.body || "-" },
        { label: "Actions", html: true, value: (row) => roomActions(row) }
      ], "No rooms");
    }
    function roomActions(row) {
      return `<div class="button-row">
        <button type="button" data-room-action="leave" data-room="${escapeHtml(row.name)}" ${row.joined ? "" : "disabled"}>Leave</button>
      </div>`;
    }
    async function loadBrowse() {
      const query = queryString({
        q: field("browse-filter-q"),
        status: field("browse-filter-status"),
        limit: 6
      });
      const data = await fetchJson(`/api/v0/browse?${query}`);
      document.getElementById("browse-table").innerHTML = table(data.entries || [], [
        { label: "User", value: (row) => row.username },
        { label: "Status", value: (row) => row.status },
        { label: "Files", value: (row) => row.count },
        { label: "Bytes", value: (row) => bytes(row.total_bytes) }
      ], "No browse records");
    }
    async function loadAll() {
      try {
        await Promise.all([loadStats(), loadSearches(), loadTransfers(), loadUsers(), loadShares(), loadMessages(), loadRooms(), loadBrowse()]);
      } catch (error) {
        document.getElementById("status-dot").className = "dot error";
        text("status-text", error.message);
      }
    }
    async function runSessionAction(action) {
      try {
        await postJson(`/api/v0/session/${action}`, {});
        text("session-action-status", `${action} accepted`);
        await loadStats();
      } catch (error) {
        text("session-action-status", error.message);
      }
    }
    async function submitSearch(event) {
      event.preventDefault();
      const query = document.getElementById("search-query").value.trim();
      const target = document.getElementById("search-target").value;
      const targetName = document.getElementById("search-target-name").value.trim();
      if (!query) {
        text("search-action-status", "Query required");
        return;
      }
      const body = { query, target };
      if (target === "user") body.username = targetName;
      if (target === "room") body.room = targetName;
      try {
        const record = await postJson("/api/v0/searches", body);
        text("search-action-status", `Search ${record.token} started`);
        document.getElementById("search-query").value = "";
        await Promise.all([loadStats(), loadSearches()]);
      } catch (error) {
        text("search-action-status", error.message);
      }
    }
    async function runSearchAction(token, action) {
      try {
        await postJson(`/api/v0/searches/${token}/${action}`, {});
        text("search-action-status", `Search ${token} ${action}`);
        await Promise.all([loadStats(), loadSearches()]);
      } catch (error) {
        text("search-action-status", error.message);
      }
    }
    async function submitWatch(event) {
      event.preventDefault();
      const username = document.getElementById("watch-username").value.trim();
      if (!username) {
        text("watch-action-status", "Username required");
        return;
      }
      try {
        await postJson("/api/v0/users/watch", { username });
        text("watch-action-status", `${username} watched`);
        document.getElementById("watch-username").value = "";
        await Promise.all([loadStats(), loadUsers()]);
      } catch (error) {
        text("watch-action-status", error.message);
      }
    }
    async function submitUnwatch() {
      const username = document.getElementById("watch-username").value.trim();
      if (!username) {
        text("watch-action-status", "Username required");
        return;
      }
      try {
        await deleteJson(`/api/v0/users/${encodeURIComponent(username)}/watch`);
        text("watch-action-status", `${username} unwatched`);
        await Promise.all([loadStats(), loadUsers()]);
      } catch (error) {
        text("watch-action-status", error.message);
      }
    }
    async function submitBrowseRequest(event) {
      event.preventDefault();
      const username = document.getElementById("browse-username").value.trim();
      const folder = document.getElementById("browse-folder").value.trim();
      if (!username) {
        text("browse-action-status", "Username required");
        return;
      }
      try {
        if (folder) {
          await postJson(`/api/v0/users/${encodeURIComponent(username)}/browse/folder`, { folder });
          text("browse-action-status", `Folder browse requested for ${username}`);
        } else {
          await postJson(`/api/v0/users/${encodeURIComponent(username)}/browse/request`, {});
          text("browse-action-status", `Browse requested for ${username}`);
        }
        await Promise.all([loadStats(), loadBrowse()]);
      } catch (error) {
        text("browse-action-status", error.message);
      }
    }
    async function submitShareRescan(event) {
      event.preventDefault();
      try {
        const snapshot = await postJson("/api/v0/shares/rescan", {});
        text("share-action-status", `${number(snapshot.files)} files indexed`);
        await Promise.all([loadStats(), loadShares()]);
      } catch (error) {
        text("share-action-status", error.message);
      }
    }
    async function submitTransfer(event) {
      event.preventDefault();
      const filename = document.getElementById("transfer-filename").value.trim();
      const peer = document.getElementById("transfer-peer").value.trim();
      const localPath = document.getElementById("transfer-local-path").value.trim();
      const size = document.getElementById("transfer-size").value.trim();
      const direction = Number(document.getElementById("transfer-direction").value);
      if (!filename) {
        text("transfer-action-status", "Filename required");
        return;
      }
      const body = { filename, direction };
      if (peer) body.peer_username = peer;
      if (localPath) body.local_path = localPath;
      if (size) body.size = Number(size);
      try {
        const transfer = await postJson("/api/v0/transfers", body);
        text("transfer-action-status", `Transfer ${transfer.id} queued`);
        document.getElementById("transfer-filename").value = "";
        document.getElementById("transfer-local-path").value = "";
        await Promise.all([loadStats(), loadTransfers()]);
      } catch (error) {
        text("transfer-action-status", error.message);
      }
    }
    async function runTransferAction(id, action) {
      const body = {};
      if (action === "progress") {
        const progress = document.getElementById("transfer-progress").value.trim();
        body.bytes_transferred = progress ? Number(progress) : 1024;
      }
      if (action === "cancel" || action === "fail") body.reason = "dashboard";
      try {
        await postJson(`/api/v0/transfers/${id}/${action}`, body);
        text("transfer-action-status", `Transfer ${id} ${action}`);
        await Promise.all([loadStats(), loadTransfers()]);
      } catch (error) {
        text("transfer-action-status", error.message);
      }
    }
    async function submitMessage(event) {
      event.preventDefault();
      const username = document.getElementById("message-username").value.trim();
      const body = document.getElementById("message-body").value.trim();
      if (!username || !body) {
        text("message-action-status", "Username and message required");
        return;
      }
      try {
        await postJson("/api/v0/messages", { username, body });
        text("message-action-status", `Message queued for ${username}`);
        document.getElementById("message-body").value = "";
        await Promise.all([loadStats(), loadMessages()]);
      } catch (error) {
        text("message-action-status", error.message);
      }
    }
    async function runMessageAction(id, action) {
      try {
        await postJson(`/api/v0/messages/${id}/${action}`, {});
        text("message-action-status", `Message ${id} ${action}`);
        await Promise.all([loadStats(), loadMessages()]);
      } catch (error) {
        text("message-action-status", error.message);
      }
    }
    async function runUserAction(username, action) {
      try {
        if (action === "watch") {
          await postJson("/api/v0/users/watch", { username });
          text("watch-action-status", `${username} watched`);
          await Promise.all([loadStats(), loadUsers()]);
        } else if (action === "browse") {
          await postJson(`/api/v0/users/${encodeURIComponent(username)}/browse/request`, {});
          text("browse-action-status", `Browse requested for ${username}`);
          await Promise.all([loadStats(), loadUsers(), loadBrowse()]);
        } else if (action === "stats") {
          await postJson(`/api/v0/users/${encodeURIComponent(username)}/stats/request`, {});
          text("watch-action-status", `Stats requested for ${username}`);
          await loadUsers();
        } else if (action === "unwatch") {
          await deleteJson(`/api/v0/users/${encodeURIComponent(username)}/watch`);
          text("watch-action-status", `${username} unwatched`);
          await Promise.all([loadStats(), loadUsers()]);
        }
      } catch (error) {
        text("watch-action-status", error.message);
      }
    }
    async function submitRoomJoin(event) {
      event.preventDefault();
      const room = document.getElementById("room-join-name").value.trim();
      if (!room) {
        text("room-join-action-status", "Room required");
        return;
      }
      try {
        await postJson(`/api/v0/rooms/${encodeURIComponent(room)}/join`, {});
        text("room-join-action-status", `${room} joined`);
        document.getElementById("room-message-name").value = room;
        await Promise.all([loadStats(), loadRooms()]);
      } catch (error) {
        text("room-join-action-status", error.message);
      }
    }
    async function submitRoomMessage(event) {
      event.preventDefault();
      const room = document.getElementById("room-message-name").value.trim();
      const username = document.getElementById("room-message-username").value.trim();
      const body = document.getElementById("room-message-body").value.trim();
      if (!room || !username || !body) {
        text("room-message-action-status", "Room, username, and message required");
        return;
      }
      try {
        await postJson(`/api/v0/rooms/${encodeURIComponent(room)}/messages`, { username, body });
        text("room-message-action-status", `Message queued for ${room}`);
        document.getElementById("room-message-body").value = "";
        await Promise.all([loadStats(), loadRooms()]);
      } catch (error) {
        text("room-message-action-status", error.message);
      }
    }
    async function syncRooms() {
      try {
        await postJson("/api/v0/rooms/refresh", {});
        text("room-join-action-status", "Room refresh requested");
        await loadRooms();
      } catch (error) {
        text("room-join-action-status", error.message);
      }
    }
    async function runRoomAction(room, action) {
      try {
        if (action === "leave") {
          await deleteJson(`/api/v0/rooms/${encodeURIComponent(room)}/join`);
          text("room-join-action-status", `${room} left`);
          await Promise.all([loadStats(), loadRooms()]);
        }
      } catch (error) {
        text("room-join-action-status", error.message);
      }
    }
    function saveToken(event) {
      event.preventDefault();
      apiToken = document.getElementById("api-token").value.trim();
      if (apiToken) {
        text("token-action-status", "Session active");
      } else {
        text("token-action-status", "Session cleared");
      }
      loadAll();
    }
    document.getElementById("refresh-searches").addEventListener("click", loadSearches);
    document.getElementById("refresh-transfers").addEventListener("click", loadTransfers);
    document.getElementById("refresh-users").addEventListener("click", loadUsers);
    document.getElementById("refresh-shares").addEventListener("click", loadShares);
    document.getElementById("refresh-messages").addEventListener("click", loadMessages);
    document.getElementById("refresh-rooms").addEventListener("click", loadRooms);
    document.getElementById("sync-rooms").addEventListener("click", syncRooms);
    document.getElementById("refresh-browse").addEventListener("click", loadBrowse);
    ["search-filter-q", "search-filter-status"].forEach((id) => {
      document.getElementById(id).addEventListener("change", loadSearches);
    });
    ["transfer-filter-q", "transfer-filter-status"].forEach((id) => {
      document.getElementById(id).addEventListener("change", loadTransfers);
    });
    ["share-filter-q", "share-filter-extension"].forEach((id) => {
      document.getElementById(id).addEventListener("change", loadShares);
    });
    ["message-filter-q", "message-filter-direction"].forEach((id) => {
      document.getElementById(id).addEventListener("change", loadMessages);
    });
    ["room-filter-q", "room-filter-joined"].forEach((id) => {
      document.getElementById(id).addEventListener("change", loadRooms);
    });
    ["browse-filter-q", "browse-filter-status"].forEach((id) => {
      document.getElementById(id).addEventListener("change", loadBrowse);
    });
    document.getElementById("session-actions").addEventListener("click", (event) => {
      const action = event.target?.dataset?.sessionAction;
      if (action) runSessionAction(action);
    });
    document.getElementById("transfer-table").addEventListener("click", (event) => {
      const action = event.target?.dataset?.transferAction;
      const id = event.target?.dataset?.transferId;
      if (action && id) runTransferAction(id, action);
    });
    document.getElementById("search-table").addEventListener("click", (event) => {
      const action = event.target?.dataset?.searchAction;
      const token = event.target?.dataset?.searchToken;
      if (action && token) runSearchAction(token, action);
    });
    document.getElementById("message-table").addEventListener("click", (event) => {
      const action = event.target?.dataset?.messageAction;
      const id = event.target?.dataset?.messageId;
      if (action && id) runMessageAction(id, action);
    });
    document.getElementById("user-table").addEventListener("click", (event) => {
      const action = event.target?.dataset?.userAction;
      const username = event.target?.dataset?.username;
      if (action && username) runUserAction(username, action);
    });
    document.getElementById("room-table").addEventListener("click", (event) => {
      const action = event.target?.dataset?.roomAction;
      const room = event.target?.dataset?.room;
      if (action && room) runRoomAction(room, action);
    });
    document.getElementById("search-form").addEventListener("submit", submitSearch);
    document.getElementById("watch-form").addEventListener("submit", submitWatch);
    document.getElementById("unwatch-button").addEventListener("click", submitUnwatch);
    document.getElementById("browse-request-form").addEventListener("submit", submitBrowseRequest);
    document.getElementById("share-rescan-form").addEventListener("submit", submitShareRescan);
    document.getElementById("transfer-form").addEventListener("submit", submitTransfer);
    document.getElementById("message-form").addEventListener("submit", submitMessage);
    document.getElementById("room-join-form").addEventListener("submit", submitRoomJoin);
    document.getElementById("room-message-form").addEventListener("submit", submitRoomMessage);
    document.getElementById("token-form").addEventListener("submit", saveToken);
    loadAll();
    setInterval(loadStats, 5000);
  </script>
</body>
</html>"#
        .replace(
            "__VERSION__",
            &format!("{CLIENT_MAJOR_VERSION}.{CLIENT_MINOR_VERSION}"),
        )
}
