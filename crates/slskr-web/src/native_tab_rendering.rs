//! Native Web native tab rendering.

use super::*;

pub(super) fn native_stat_html(label: &str, value: &str) -> String {
    format!(
        r#"<span class="slskr-native-stat"><strong>{}</strong><em>{}</em></span>"#,
        escape_html(value),
        escape_html(label)
    )
}

pub(super) fn native_tab_labels(kind: RouteKind) -> &'static [&'static str] {
    match kind {
        RouteKind::Search => &[
            "Results",
            "Searches",
            "Planner",
            "Filters",
            "Download Preview",
        ],
        RouteKind::DiscoveryGraph => &["Graph", "Recommendations", "Review Queue", "Profiles"],
        RouteKind::PlaylistIntake => &["Parser", "Rows", "Classification", "Plans"],
        RouteKind::Wishlist => &["Wanted", "Review", "History", "Discovery Inbox"],
        RouteKind::Downloads => &["Active", "Queued", "Completed", "Failed"],
        RouteKind::Uploads => &["Active", "Queued", "Completed", "Policy"],
        RouteKind::Messages | RouteKind::Rooms => {
            &["Conversations", "Thread", "Rooms", "Pods", "Search"]
        }
        RouteKind::Users => &["Directory", "Detail", "Watched", "Notes"],
        RouteKind::Contacts => &["Contacts", "Groups", "Nearby", "Invites", "Notes"],
        RouteKind::Solid => &["Identity", "Storage", "Session", "Sync", "Related"],
        RouteKind::Collections => &["Collections", "Items", "Picker", "Sharing"],
        RouteKind::ShareGroups => &["Groups", "Members", "Grants", "Tokens", "Permissions"],
        RouteKind::SharedWithMe => &["Inbound", "Collections", "Tokens", "Owners", "Access"],
        RouteKind::Browse => &["Tabs", "Tree", "Files", "Selected", "Queue"],
        RouteKind::System => &[
            "Info",
            "Network",
            "Mesh",
            "Bridge",
            "MediaCore",
            "Security Policies",
            "Experience",
            "Integrations",
            "Options",
            "Shares",
            "Jobs",
            "Automations",
            "Source Providers",
            "Swarm Analytics",
            "Library Health",
            "Quarantine Jury",
            "Files",
            "Data",
            "Events",
            "Logs",
            "Metrics",
        ],
    }
}

pub(super) fn native_tab_detail(kind: RouteKind, label: &str) -> &'static str {
    match (kind, label) {
        (RouteKind::Search, "Results") => {
            "Grouped file results with peer, queue, score, warning, and download actions."
        }
        (RouteKind::Search, "Searches") => {
            "Active and historical searches with stop, clear, and reopen controls."
        }
        (RouteKind::Search, "Planner") => {
            "Review selected results before acquisition plans or downloads are created."
        }
        (RouteKind::Search, "Filters") => "Format, bitrate, size, queue, and duplicate filters.",
        (RouteKind::Search, "Download Preview") => {
            "Selected files, peers, destination, and queued download summary."
        }
        (RouteKind::DiscoveryGraph, "Graph") => {
            "Artist, album, track, query, and provider nodes with weighted links."
        }
        (RouteKind::DiscoveryGraph, "Recommendations") => {
            "Next searches suggested from the selected graph neighborhood."
        }
        (RouteKind::DiscoveryGraph, "Review Queue") => {
            "Candidate searches staged for acquisition review."
        }
        (RouteKind::DiscoveryGraph, "Profiles") => {
            "Acquisition profile selector for graph-generated searches."
        }
        (RouteKind::PlaylistIntake, "Parser") => {
            "Paste or upload playlist text before provider or network work starts."
        }
        (RouteKind::PlaylistIntake, "Rows") => {
            "Parsed rows with artist, title, source, and row-level validation."
        }
        (RouteKind::PlaylistIntake, "Classification") => {
            "Track, album, ambiguous, and error buckets for review."
        }
        (RouteKind::PlaylistIntake, "Plans") => {
            "Queue searches or acquisition plans after validation."
        }
        (RouteKind::Wishlist, "Wanted") => {
            "Saved wanted searches with enabled state, filters, and result limits."
        }
        (RouteKind::Wishlist, "Review") => {
            "Result review state before automatic or manual download decisions."
        }
        (RouteKind::Wishlist, "History") => "Last run, result counts, failures, and audit trail.",
        (RouteKind::Wishlist, "Discovery Inbox") => {
            "Bridge selected wanted searches into acquisition request review."
        }
        (RouteKind::Downloads, "Active") => "Running downloads with progress, speed, and ETA.",
        (RouteKind::Downloads, "Queued") => "Pending downloads ordered by peer and slot state.",
        (RouteKind::Downloads, "Completed") => "Finished downloads ready to clear or inspect.",
        (RouteKind::Downloads, "Failed") => "Failed downloads with retry and remove actions.",
        (RouteKind::Uploads, "Active") => "Running uploads with requester, speed, and progress.",
        (RouteKind::Uploads, "Queued") => "Peer requests waiting for an upload slot.",
        (RouteKind::Uploads, "Completed") => "Finished uploads and clear-completed controls.",
        (RouteKind::Uploads, "Policy") => "Allow, deny, queue, and sharing policy controls.",
        (RouteKind::Messages | RouteKind::Rooms, "Conversations") => {
            "Direct message list with unread and acknowledge state."
        }
        (RouteKind::Messages | RouteKind::Rooms, "Thread") => {
            "Selected direct message, room, or pod channel conversation."
        }
        (RouteKind::Messages | RouteKind::Rooms, "Rooms") => {
            "Joined and available rooms with join, leave, and compose actions."
        }
        (RouteKind::Messages | RouteKind::Rooms, "Pods") => {
            "Pod channels stay secondary inside Messages."
        }
        (RouteKind::Messages | RouteKind::Rooms, "Search") => {
            "Search conversations and room names without leaving the messenger."
        }
        (RouteKind::Users, "Directory") => "Watched and searched users with online state.",
        (RouteKind::Users, "Detail") => {
            "Readable user status, privileges, stats, and endpoint info."
        }
        (RouteKind::Users, "Watched") => "Watch list controls for peers you monitor.",
        (RouteKind::Users, "Notes") => "Private notes tied to selected users.",
        (RouteKind::Contacts, "Contacts") => {
            "Saved contacts with message, browse, and remove actions."
        }
        (RouteKind::Contacts, "Groups") => "Contact grouping for trusted or nearby peers.",
        (RouteKind::Contacts, "Nearby") => "Nearby contacts and invite candidates.",
        (RouteKind::Contacts, "Invites") => "Invite, accept, and link handling.",
        (RouteKind::Contacts, "Notes") => "Contact notes and verification context.",
        (RouteKind::Solid, "Identity") => "WebID identity resolution and connection state.",
        (RouteKind::Solid, "Storage") => "Solid storage root and linked-data persistence.",
        (RouteKind::Solid, "Session") => "Authentication and session state.",
        (RouteKind::Solid, "Sync") => "Linked-data sync status and retry controls.",
        (RouteKind::Solid, "Related") => "Bridge, pod, and source-provider context.",
        (RouteKind::Collections, "Collections") => {
            "Collection list with create and select actions."
        }
        (RouteKind::Collections, "Items") => "Selected collection item table with remove controls.",
        (RouteKind::Collections, "Picker") => "Library item browser used as an add-item picker.",
        (RouteKind::Collections, "Sharing") => "Collection share controls and current grants.",
        (RouteKind::ShareGroups, "Groups") => "Share group list with selected group detail.",
        (RouteKind::ShareGroups, "Members") => "Add, remove, and inspect group members.",
        (RouteKind::ShareGroups, "Grants") => "Collection grants issued to the selected group.",
        (RouteKind::ShareGroups, "Tokens") => "Issue, copy, and revoke access tokens.",
        (RouteKind::ShareGroups, "Permissions") => {
            "Read, download, stream, and expiration settings."
        }
        (RouteKind::SharedWithMe, "Inbound") => "Inbound grants and tokens shared by other users.",
        (RouteKind::SharedWithMe, "Collections") => {
            "Shared collections and files available to open."
        }
        (RouteKind::SharedWithMe, "Tokens") => "Token status, copy actions, and expiration.",
        (RouteKind::SharedWithMe, "Owners") => "Owner identity, trust, and contact actions.",
        (RouteKind::SharedWithMe, "Access") => "Open, leave, revoke, or backfill where allowed.",
        (RouteKind::Browse, "Tabs") => {
            "Multiple peer browse sessions, matching the old tabbed browser."
        }
        (RouteKind::Browse, "Tree") => "Directory tree with breadcrumbs and folder expansion.",
        (RouteKind::Browse, "Files") => "File list with size, type, filter, and selection state.",
        (RouteKind::Browse, "Selected") => "Multi-select download preview before queueing.",
        (RouteKind::Browse, "Queue") => "Download queue action for selected browse files.",
        (RouteKind::System, "Info") => "Server, version, session, and operator overview.",
        (RouteKind::System, "Network") => "Connection, ports, privileges, and server state.",
        (RouteKind::System, "Mesh") => "Mesh and federation diagnostics.",
        (RouteKind::System, "Bridge") => "External bridge and integration status.",
        (RouteKind::System, "MediaCore") => {
            "MediaCore routing, validation, storage, and content tools."
        }
        (RouteKind::System, "Security Policies") => "Security policy status and decisions.",
        (RouteKind::System, "Experience") => "User experience preferences.",
        (RouteKind::System, "Integrations") => {
            "Lidarr, MusicBrainz, SongID, and source-provider integrations."
        }
        (RouteKind::System, "Options") => "Daemon options and preferences.",
        (RouteKind::System, "Shares") => "Share roots, scan status, and rescan controls.",
        (RouteKind::System, "Jobs") => "Jobs, queues, and execution history.",
        (RouteKind::System, "Automations") => "Automation recipes and bounded execution.",
        (RouteKind::System, "Source Providers") => {
            "Search, metadata, and verification source providers."
        }
        (RouteKind::System, "Swarm Analytics") => "Swarm and peer analytics.",
        (RouteKind::System, "Library Health") => "Library health issues and replacement searches.",
        (RouteKind::System, "Quarantine Jury") => "Quarantine review and decision workflow.",
        (RouteKind::System, "Files") => "File index, fingerprints, and library records.",
        (RouteKind::System, "Data") => "Database and storage maintenance.",
        (RouteKind::System, "Events") => "Filterable event stream.",
        (RouteKind::System, "Logs") => "Operator logs with filters.",
        (RouteKind::System, "Metrics") => "Raw metrics summarized for operators.",
        _ => "Route-specific workflow section.",
    }
}

pub(super) fn native_tabs_html(kind: RouteKind, responses: Option<&[EndpointBody]>) -> String {
    let labels = native_tab_labels(kind);
    let buttons = labels
        .iter()
        .enumerate()
        .map(|(index, label)| {
            let selected = if index == 0 { "true" } else { "false" };
            let class = if index == 0 { " is-active" } else { "" };
            format!(
                r#"<button type="button" role="tab" class="slskr-native-tab{class}" aria-selected="{selected}" data-slskr-native-tab="{index}">{}</button>"#,
                escape_html(label)
            )
        })
        .collect::<Vec<_>>()
        .join("");
    let panels = labels
        .iter()
        .enumerate()
        .map(|(index, label)| {
            let hidden = if index == 0 { "" } else { " hidden" };
            format!(
                r#"<section class="slskr-native-subpanel" data-slskr-native-panel="{index}" data-slskr-native-panel-label="{}"{hidden}>{}</section>"#,
                escape_html(label),
                native_tab_panel_html(kind, label, responses),
            )
        })
        .collect::<Vec<_>>()
        .join("");
    format!(
        r#"<div class="slskr-native-subviews"><div class="slskr-native-subnav" role="tablist">{buttons}</div><div class="slskr-native-subpanels">{panels}</div></div>"#
    )
}

pub(super) fn native_tab_panel_html(
    kind: RouteKind,
    label: &str,
    responses: Option<&[EndpointBody]>,
) -> String {
    if kind == RouteKind::System {
        return native_system_tab_panel_html(label, responses);
    }
    let detail = native_tab_detail(kind, label);
    let controls = native_tab_controls(kind, label)
        .iter()
        .map(|control| format!(r#"<button type="button">{}</button>"#, escape_html(control)))
        .collect::<Vec<_>>()
        .join("");
    let fields = native_tab_fields(kind, label)
        .iter()
        .map(|(field, placeholder)| {
            format!(
                r#"<label><span>{}</span><input type="text" aria-label="{}" placeholder="{}"></label>"#,
                escape_html(field),
                escape_html(field),
                escape_html(placeholder)
            )
        })
        .collect::<Vec<_>>()
        .join("");
    let facts = native_tab_facts(kind, label)
        .iter()
        .map(|fact| format!(r#"<span>{}</span>"#, escape_html(fact)))
        .collect::<Vec<_>>()
        .join("");
    format!(
        r#"<header><div><h4>{}</h4><p>{}</p></div><div class="slskr-native-panel-actions">{}</div></header><div class="slskr-native-panel-fields">{}</div><div class="slskr-native-panel-facts">{}</div>"#,
        escape_html(label),
        escape_html(detail),
        controls,
        fields,
        facts,
    )
}

pub(super) fn native_system_tab_panel_html(
    label: &str,
    responses: Option<&[EndpointBody]>,
) -> String {
    let detail = native_tab_detail(RouteKind::System, label);
    let controls = if label == "Experience" {
        String::new()
    } else {
        native_tab_controls(RouteKind::System, label)
            .iter()
            .map(|control| format!(r#"<button type="button">{}</button>"#, escape_html(control)))
            .collect::<Vec<_>>()
            .join("")
    };
    let fields = if label == "Experience" {
        String::new()
    } else {
        native_tab_fields(RouteKind::System, label)
            .iter()
            .map(|(field, placeholder)| {
                format!(
                    r#"<label><span>{}</span><input type="text" aria-label="{}" placeholder="{}"></label>"#,
                    escape_html(field),
                    escape_html(field),
                    escape_html(placeholder)
                )
            })
            .collect::<Vec<_>>()
            .join("")
    };
    let rows = native_system_live_rows(label, responses);
    let facts = native_tab_facts(RouteKind::System, label)
        .iter()
        .map(|fact| format!(r#"<span>{}</span>"#, escape_html(fact)))
        .collect::<Vec<_>>()
        .join("");
    let local_panel = match label {
        "Experience" => experience_settings_panel_html(),
        "Automations" => automation_center_panel_html(),
        _ => String::new(),
    };
    format!(
        r#"<header><div><h4>{label}</h4><p>{detail}</p></div><div class="slskr-native-panel-actions">{controls}</div></header><div class="slskr-native-panel-fields">{fields}</div><div class="slskr-native-panel-facts">{facts}</div><div class="slskr-native-table-wrap"><table class="slskr-native-table slskr-system-panel-table"><thead><tr><th>Area</th><th>State</th><th>Detail</th><th>Action</th></tr></thead><tbody>{rows}</tbody></table></div>{local_panel}"#,
        label = escape_html(label),
        detail = escape_html(detail),
        controls = controls,
        fields = fields,
        facts = facts,
        rows = rows,
        local_panel = local_panel,
    )
}

pub(super) fn native_system_live_rows(label: &str, responses: Option<&[EndpointBody]>) -> String {
    let endpoints: &[&str] = match label {
        "Info" => &["/server", "/database/stats"],
        "Network" => &["/server", "/mesh/stats", "/bridge/status"],
        "Mesh" => &["/mesh/stats", "/security/dashboard"],
        "Bridge" => &["/bridge/status"],
        "MediaCore" => &["/songid/runs", "/transfers/speeds"],
        "Security Policies" => &["/security/dashboard"],
        "Experience" => &[],
        "Integrations" => &[
            "/source-providers",
            "/integrations/lidarr/status",
            "/integrations/lidarr/sync/status",
            "/musicbrainz/albums/completion",
            "/musicbrainz/release-radar/subscriptions",
            "/songid/runs",
        ],
        "Options" => &["/options", "/config"],
        "Shares" => &["/shares"],
        "Jobs" | "Automations" => &["/jobs"],
        "Source Providers" => &["/source-providers"],
        "Swarm Analytics" => &["/mesh/stats", "/telemetry/metrics"],
        "Library Health" => &[
            "/library/health/issues",
            "/library/health/issues/by-type",
            "/library/health/issues/by-artist",
            "/library/health/issues/by-release",
            "/library/health/issues/by-codec",
        ],
        "Files" => &["/library/items", "/songid/runs"],
        "Quarantine Jury" => &["/quarantine-jury/audit", "/quarantine-jury/requests"],
        "Data" => &["/database/stats"],
        "Events" => &["/events"],
        "Logs" => &["/logs"],
        "Metrics" => &[
            "/telemetry/metrics",
            "/telemetry/metrics/kpis",
            "/telemetry/reports/transfers/summary",
        ],
        _ => &[],
    };
    let mut rows = endpoints
        .iter()
        .filter_map(|endpoint| {
            let value = json_endpoint_value(responses, endpoint)?;
            let title = endpoint.trim_start_matches('/').replace('/', " / ");
            let detail = compact_preview(&value.to_string());
            Some(format!(
                r#"<tr tabindex="0" data-slskr-native-select data-slskr-native-title="{title}" data-slskr-native-detail="{endpoint}" data-slskr-native-meta="returned"><td><strong>{title}</strong></td><td>returned</td><td><code>{detail}</code></td><td><button type="button">Review Selection</button></td></tr>"#,
                title = escape_html(&title),
                endpoint = escape_html(endpoint),
                detail = escape_html(&detail),
            ))
        })
        .collect::<Vec<_>>();
    if label == "Experience" {
        rows.push(
            r#"<tr tabindex="0" data-slskr-native-select data-slskr-native-title="Browser preferences" data-slskr-native-detail="local storage" data-slskr-native-meta="local"><td><strong>Browser preferences</strong></td><td>local</td><td><code>Stored in this browser</code></td><td><button type="button">Review Selection</button></td></tr>"#.to_string(),
        );
    }
    if rows.is_empty() {
        r#"<tr><td colspan="4"><div class="slskr-native-empty"><strong>No live data returned for this tab</strong><span>Use the route action or refresh after enabling the corresponding service.</span></div></td></tr>"#.to_string()
    } else {
        rows.join("")
    }
}

#[allow(dead_code)]
pub(super) fn native_system_tab_rows(
    label: &str,
) -> &'static [(&'static str, &'static str, &'static str, &'static str)] {
    match label {
        "Info" => &[
            (
                "Daemon",
                "running",
                "version and build channel",
                "Check for Updates",
            ),
            (
                "Session",
                "pending",
                "account, privileges, and uptime",
                "Get Privileges",
            ),
            (
                "Diagnostics",
                "ready",
                "support bundle and health summary",
                "Diagnostic Bundle",
            ),
        ],
        "Network" => &[
            (
                "Soulseek server",
                "disconnected",
                "connect, disconnect, reconnect",
                "Connect",
            ),
            (
                "Listening port",
                "unknown",
                "port mapping and reachability",
                "Refresh",
            ),
            (
                "Privileges",
                "unknown",
                "privilege expiry and purchase status",
                "Get Privileges",
            ),
        ],
        "Mesh" => &[
            (
                "Federation",
                "observing",
                "mesh peers and evidence policy",
                "Refresh",
            ),
            (
                "Conflict queue",
                "empty",
                "remote claims and merge decisions",
                "Review Selection",
            ),
            (
                "Relay health",
                "pending",
                "bridge latency and retry window",
                "Diagnostic Bundle",
            ),
        ],
        "Bridge" => &[
            ("Gateway", "idle", "external bridge listener", "Refresh"),
            (
                "Relay",
                "not linked",
                "remote relay credentials",
                "Setup Health",
            ),
            (
                "Sync cursor",
                "pending",
                "last successful bridge sync",
                "Run Selected",
            ),
        ],
        "MediaCore" => &[
            (
                "Routing",
                "ready",
                "stream routes and player handoff",
                "Refresh",
            ),
            (
                "Validation",
                "enabled",
                "file metadata and format checks",
                "Run Selected",
            ),
            (
                "Content tools",
                "available",
                "transcode, fingerprint, repair",
                "Diagnostic Bundle",
            ),
        ],
        "Security Policies" => &[
            (
                "Admin policy",
                "enforced",
                "dangerous actions require confirmation",
                "Review Selection",
            ),
            (
                "Quarantine",
                "enabled",
                "approval, rejection, and evidence retention",
                "Approve",
            ),
            (
                "Outbound webhooks",
                "guarded",
                "allowlist and secret handling",
                "Setup Health",
            ),
        ],
        "Experience" => &[
            (
                "Theme",
                "saved locally",
                "density, contrast, and shell preferences",
                "Save",
            ),
            (
                "Player",
                "compact",
                "bottom player reserve and radio seed mode",
                "Reset",
            ),
            (
                "Notifications",
                "quiet",
                "toast and event notification defaults",
                "Save",
            ),
        ],
        "Integrations" => &[
            (
                "Lidarr",
                "not configured",
                "metadata and import automation",
                "Setup Health",
            ),
            (
                "FTP",
                "not configured",
                "remote drop and library import",
                "Setup Health",
            ),
            (
                "ListenBrainz",
                "not linked",
                "scrobble and feedback sync",
                "Setup Health",
            ),
        ],
        "Options" => &[
            ("Config", "loaded", "daemon options and overrides", "Save"),
            (
                "Debug",
                "off",
                "developer diagnostics stay collapsed",
                "Reset",
            ),
            (
                "Automation bounds",
                "default",
                "limits for unattended jobs",
                "Save",
            ),
        ],
        "Shares" => &[
            (
                "Roots",
                "pending",
                "shared folders and exclusions",
                "Rescan Shares",
            ),
            (
                "Scan progress",
                "idle",
                "last scan, changed files, failures",
                "Refresh",
            ),
            (
                "Contents",
                "indexed",
                "share counts and locked files",
                "Review Selection",
            ),
        ],
        "Jobs" => &[
            (
                "Queued",
                "0",
                "scheduled work waiting to run",
                "Run Selected",
            ),
            ("Running", "0", "active backend jobs", "Cancel Selected"),
            (
                "History",
                "pending",
                "recent completions and failures",
                "Refresh",
            ),
        ],
        "Automations" => &[
            (
                "Recipes",
                "available",
                "library health, replacements, cleanup",
                "Run Selected",
            ),
            (
                "Approvals",
                "required",
                "bounded unattended actions",
                "Review Selection",
            ),
            (
                "Dry run",
                "enabled",
                "preview before mutation",
                "Copy Action Plan",
            ),
        ],
        "Source Providers" => &[
            (
                "Search",
                "ready",
                "provider search and fallback order",
                "Refresh",
            ),
            (
                "Metadata",
                "pending",
                "MusicBrainz and provider enrichment",
                "Run Selected",
            ),
            (
                "Verification",
                "manual",
                "confidence and match warnings",
                "Review Selection",
            ),
        ],
        "Swarm Analytics" => &[
            (
                "Peers",
                "sampling",
                "availability and quality signals",
                "Refresh",
            ),
            (
                "Availability",
                "unknown",
                "result recurrence and queue health",
                "Run Selected",
            ),
            (
                "Quality",
                "pending",
                "bitrate, format, and duplicate signals",
                "Copy Action Plan",
            ),
        ],
        "Library Health" => &[
            (
                "Issues",
                "review",
                "missing, corrupt, duplicate, and low-quality files",
                "Run Replacement Searches",
            ),
            (
                "Replacements",
                "staged",
                "candidate searches for bad files",
                "Copy Action Plan",
            ),
            (
                "Reports",
                "ready",
                "operator review packets",
                "Diagnostic Bundle",
            ),
        ],
        "Quarantine Jury" => &[
            ("Pending", "0", "files awaiting decision", "Approve"),
            ("Rejected", "0", "blocked items and reasons", "Reject"),
            (
                "Evidence",
                "retained",
                "packet copied for review",
                "Copy Packet",
            ),
        ],
        "Files" => &[
            ("Index", "ready", "library records and paths", "Refresh"),
            (
                "Fingerprints",
                "pending",
                "hashes and acoustic IDs",
                "Run Selected",
            ),
            (
                "Records",
                "filterable",
                "open file detail and repair context",
                "Review Selection",
            ),
        ],
        "Data" => &[
            (
                "Database",
                "ready",
                "stats, size, and vacuum state",
                "Vacuum Database",
            ),
            (
                "Storage",
                "bounded",
                "cache and archive cleanup",
                "Run Selected",
            ),
            (
                "Backups",
                "manual",
                "operator export and restore checks",
                "Diagnostic Bundle",
            ),
        ],
        "Events" => &[
            ("Stream", "live", "filterable operator events", "Refresh"),
            (
                "Acknowledgements",
                "pending",
                "review and clear handled events",
                "Clear Filter",
            ),
            (
                "Severity",
                "warn+",
                "level and source filters",
                "Review Selection",
            ),
        ],
        "Logs" => &[
            ("Level", "info", "filter by level and source", "Refresh"),
            (
                "Search",
                "ready",
                "text filter across recent logs",
                "Clear Filter",
            ),
            (
                "Export",
                "available",
                "include logs in diagnostic bundle",
                "Diagnostic Bundle",
            ),
        ],
        "Metrics" => &[
            (
                "Transfers",
                "summarized",
                "speed, slots, and failure rate",
                "Refresh",
            ),
            (
                "API",
                "summarized",
                "requests, errors, and latency",
                "Diagnostic Bundle",
            ),
            (
                "Raw metrics",
                "developer-only",
                "hidden outside Developer drawer",
                "Review Selection",
            ),
        ],
        _ => &[("Status", "pending", "operator workflow", "Review Selection")],
    }
}

pub(super) fn native_tab_controls(kind: RouteKind, label: &str) -> &'static [&'static str] {
    match (kind, label) {
        (RouteKind::Messages | RouteKind::Rooms, "Thread") => {
            &["Reply", "Acknowledge", "Delete Conversation"]
        }
        (RouteKind::Messages | RouteKind::Rooms, "Rooms") => &["Join Room", "Leave Room"],
        (RouteKind::Browse, "Tabs") => &["Open a New Browse Tab", "New Tab"],
        (RouteKind::Browse, "Tree") => &["Browse", "Refresh Folder"],
        (RouteKind::Browse, "Selected" | "Queue") => &["Download Selected"],
        (RouteKind::Collections, "Collections") => &[
            "Create Collection",
            "Update Collection",
            "Delete Collection",
        ],
        (RouteKind::Collections, "Items" | "Picker") => &["Add Item"],
        (RouteKind::Collections, "Sharing") => &["Share"],
        (RouteKind::ShareGroups, "Groups") => &["Create Group"],
        (RouteKind::ShareGroups, "Members") => &["Add Member"],
        (RouteKind::ShareGroups, "Tokens") => &["Issue Token"],
        (RouteKind::ShareGroups, "Grants" | "Permissions") => {
            &["Create Share Grant", "Update Share Grant"]
        }
        (RouteKind::SharedWithMe, "Access") => &["Open", "Stream", "Backfill", "Copy token"],
        (RouteKind::System, "Network") => &["Connect", "Disconnect", "Get Privileges"],
        (RouteKind::System, "Shares") => &["Rescan Shares"],
        (RouteKind::System, "Data") => &["Vacuum Database"],
        (RouteKind::System, "Info") => &["Check for Updates", "Diagnostic Bundle"],
        (RouteKind::System, "Integrations") => &[
            "Check Lidarr",
            "Refresh Lidarr Sync",
            "Track MusicBrainz Target",
            "Refresh MusicBrainz",
            "Start SongID Run",
            "Refresh SongID",
        ],
        (RouteKind::System, "Logs" | "Events") => &["Refresh", "Clear Filter"],
        (RouteKind::System, "Options" | "Experience") => &[],
        (RouteKind::System, "Jobs" | "Automations") => &["Refresh"],
        (RouteKind::System, "Library Health") => &[
            "Scan Library Health",
            "Fix Library Issues",
            "Copy Action Plan",
        ],
        (RouteKind::System, "Quarantine Jury") => &["Refresh", "Copy Packet"],
        (RouteKind::Search, "Results" | "Download Preview") => &["Download", "Queue Selected"],
        (RouteKind::Search, "Searches") => &["Search", "Stop", "Clear"],
        (RouteKind::DiscoveryGraph, "Graph" | "Recommendations") => {
            &["Build Atlas", "Queue Nearby"]
        }
        (RouteKind::PlaylistIntake, "Parser" | "Rows") => &["Import Playlist", "Queue Plans"],
        (RouteKind::Wishlist, "Wanted" | "Discovery Inbox") => &["Add Search", "Run Enabled"],
        (RouteKind::Downloads, "Active" | "Failed") => &["Retry All", "Cancel All"],
        (RouteKind::Downloads, "Completed") => &["Clear Completed"],
        (RouteKind::Uploads, "Active" | "Queued" | "Policy") => {
            &["Allow selected", "Deny selected"]
        }
        (RouteKind::Uploads, "Completed") => &["Clear Completed"],
        (RouteKind::Users, "Directory" | "Detail") => &["Watch", "Browse", "Message"],
        (RouteKind::Users, "Notes") => &["Save note"],
        (RouteKind::Contacts, "Contacts" | "Nearby") => &["Add Friend", "Message", "Browse"],
        (RouteKind::Contacts, "Invites") => &["Create Invite", "Add Friend"],
        (RouteKind::Solid, "Identity" | "Session") => &["Resolve WebID", "Connect Identity"],
        (RouteKind::Solid, "Storage" | "Sync") => &["Sync Storage"],
        _ => &["Review Selection"],
    }
}

pub(super) fn native_tab_fields(
    kind: RouteKind,
    label: &str,
) -> &'static [(&'static str, &'static str)] {
    match (kind, label) {
        (RouteKind::Messages | RouteKind::Rooms, "Thread") => {
            &[("Chat username", "Username"), ("Message", "Message")]
        }
        (RouteKind::Messages | RouteKind::Rooms, "Rooms") => &[("Search rooms", "Room name")],
        (RouteKind::Browse, "Tabs") => &[("Username", "Username")],
        (RouteKind::Browse, "Tree" | "Files") => &[("Username", "Username"), ("Folder", "/")],
        (RouteKind::Collections, "Collections") => &[
            ("Title", "Collection title"),
            ("Description", "Optional description"),
        ],
        (RouteKind::Collections, "Picker" | "Items") => &[("Search for item", "Filename or title")],
        (RouteKind::ShareGroups, "Groups") => &[("Group Name", "Trusted peers")],
        (RouteKind::ShareGroups, "Members") => &[("Soulseek Username", "Username")],
        (RouteKind::ShareGroups, "Permissions") => &[("Permissions", "read,download,stream")],
        (RouteKind::Solid, "Identity") => &[("WebID", "https://example.com/profile/card#me")],
        (RouteKind::Search, "Searches" | "Results") => &[("Search text", "public domain jazz")],
        (RouteKind::DiscoveryGraph, "Graph") => &[("Artist Name", "Artist or query")],
        (RouteKind::PlaylistIntake, "Parser") => &[("Playlist rows", "One track per line")],
        (RouteKind::Wishlist, "Wanted") => &[("Search Text", "Search text")],
        (RouteKind::Users, "Directory" | "Detail") => &[("Username", "Username")],
        (RouteKind::Contacts, "Contacts" | "Invites") => &[("Nickname", "Nickname")],
        (RouteKind::System, "Logs" | "Events") => &[("Filter", "Filter text")],
        (RouteKind::System, "Library Health") => &[("Library Path", "Optional library path")],
        (RouteKind::System, "Options") => &[("Option key", "Configuration key")],
        (RouteKind::System, "Integrations") => &[
            ("Release ID", "MusicBrainz release identifier"),
            ("SongID source", "local path, Spotify URL, or query"),
        ],
        _ => &[],
    }
}

pub(super) fn native_tab_facts(kind: RouteKind, label: &str) -> &'static [&'static str] {
    match (kind, label) {
        (RouteKind::System, "Info") => &["Version", "Session", "Privileges", "Uptime"],
        (RouteKind::System, "Network") => &["Server state", "Ports", "Rate limits", "Proxy trust"],
        (RouteKind::System, "Mesh") => &["Federation health", "Evidence policy", "Conflicts"],
        (RouteKind::System, "Bridge") => &["Bridge status", "Gateway", "Relay"],
        (RouteKind::System, "MediaCore") => &["Routing", "Validation", "Storage", "Content tools"],
        (RouteKind::System, "Security Policies") => {
            &["Admin policy", "Quarantine", "Outbound webhooks"]
        }
        (RouteKind::System, "Experience") => &["Theme", "Density", "Player", "Notifications"],
        (RouteKind::System, "Integrations") => {
            &["Lidarr", "MusicBrainz", "SongID", "Source providers"]
        }
        (RouteKind::System, "Options") => &["Config", "Debug", "Overrides"],
        (RouteKind::System, "Shares") => &["Roots", "Exclusions", "Scan progress", "Contents"],
        (RouteKind::System, "Jobs") => &["Queued", "Running", "Failed", "History"],
        (RouteKind::System, "Automations") => &["Recipes", "Bounds", "Approvals"],
        (RouteKind::System, "Source Providers") => &["Search", "Metadata", "Verification"],
        (RouteKind::System, "Swarm Analytics") => &["Peers", "Availability", "Quality"],
        (RouteKind::System, "Library Health") => &["Issues", "Replacements", "Reports"],
        (RouteKind::System, "Quarantine Jury") => &["Pending", "Approved", "Rejected"],
        (RouteKind::System, "Files") => &["Index", "Fingerprints", "Records"],
        (RouteKind::System, "Data") => &["Database", "Vacuum", "Cleanup", "Storage"],
        (RouteKind::System, "Events") => &["Filterable stream", "Acknowledgements"],
        (RouteKind::System, "Logs") => &["Level", "Source", "Search"],
        (RouteKind::System, "Metrics") => &["KPIs", "Transfer summary", "Raw metrics"],
        (RouteKind::Browse, _) => &[
            "Cached browse",
            "Breadcrumb",
            "Multi-select",
            "Queue preview",
        ],
        (RouteKind::Messages | RouteKind::Rooms, _) => {
            &["Conversations", "Unread", "Rooms", "Pods", "Compose"]
        }
        (RouteKind::Collections, _) => &["Collection detail", "Items", "Picker", "Sharing"],
        (RouteKind::ShareGroups, _) => &["Members", "Grants", "Tokens", "Permissions"],
        (RouteKind::SharedWithMe, _) => &["Owner", "Expiration", "Access", "Manifest"],
        _ => &["Loading", "Empty", "Error", "Success"],
    }
}
