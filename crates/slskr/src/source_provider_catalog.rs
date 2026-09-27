pub(super) fn source_provider_catalog_json(acquisition_planning_enabled: bool) -> String {
    let definitions = [
        (
            "LocalLibrary",
            "Local Library",
            "Already indexed or shared files on this slskr node.",
            "local",
            "Music",
            &["search", "download", "checksum", "metadata", "preview"] as &[&str],
            false,
            "No peer traffic. Uses local catalogue/share data only.",
            "Always available when acquisition planning is enabled.",
            true,
            0,
        ),
        (
            "Soulseek",
            "Soulseek",
            "Public Soulseek search and peer transfer path.",
            "public-network",
            "Music",
            &["search", "download", "preview", "metadata"],
            true,
            "Rate-limited searches, queue/speed filtering, and user-visible profile caps.",
            "Primary compatibility provider.",
            true,
            10,
        ),
        (
            "NativeMesh",
            "Native Mesh",
            "Trusted native overlay peers with content descriptors.",
            "trusted-mesh",
            "Music",
            &["search", "download", "checksum", "preview"],
            false,
            "Trusted overlay only; no public peer probing.",
            "Disabled until mesh content publication is enabled.",
            false,
            20,
        ),
        (
            "MeshDht",
            "Mesh DHT",
            "DHT-backed hints for verified content candidates.",
            "trusted-mesh",
            "Music",
            &["search", "checksum", "preview"],
            false,
            "DHT operations must use rate limits and trust thresholds.",
            "Disabled until trusted mesh discovery is enabled.",
            false,
            30,
        ),
        (
            "Http",
            "HTTP",
            "User-configured HTTP or HTTPS repositories.",
            "configured-network",
            "Any",
            &["download", "checksum", "preview"],
            true,
            "Requires allowlisted hosts, size limits, and SSRF-safe fetches.",
            "Disabled until allowlisted repositories are configured.",
            false,
            40,
        ),
        (
            "WebDav",
            "WebDAV",
            "User-configured WebDAV repositories.",
            "configured-network",
            "Any",
            &["search", "download", "checksum", "preview", "auth"],
            true,
            "Requires allowlisted hosts, size limits, and configured credentials when needed.",
            "Disabled until allowlisted repositories are configured.",
            false,
            50,
        ),
        (
            "S3",
            "S3",
            "User-configured S3-compatible object storage.",
            "configured-network",
            "Any",
            &["search", "download", "checksum", "auth"],
            true,
            "Requires bucket allowlists, object size limits, and configured credentials.",
            "Disabled until allowlisted buckets are configured.",
            false,
            60,
        ),
        (
            "Lan",
            "LAN",
            "User-configured local network shares.",
            "configured-lan",
            "Any",
            &["search", "download", "checksum", "preview"],
            true,
            "Restricted to allowed local-network roots.",
            "Disabled until LAN roots are configured.",
            false,
            70,
        ),
        (
            "Torrent",
            "Private Torrent",
            "Explicitly configured private torrent or magnet sources.",
            "high-risk",
            "Any",
            &["search", "download", "checksum"],
            true,
            "High-risk provider. Must remain disabled until configured and explicitly enabled.",
            "Disabled by default.",
            false,
            90,
        ),
    ];

    let providers = definitions
        .iter()
        .map(
            |(
                id,
                name,
                description,
                risk_level,
                domain,
                capabilities,
                requires_configuration,
                network_policy,
                default_disabled_reason,
                enabled_by_default,
                sort_order,
            )| {
                let active = acquisition_planning_enabled && *enabled_by_default;
                let disabled_reason = if active {
                    serde_json::Value::Null
                } else if !acquisition_planning_enabled {
                    serde_json::json!("VirtualSoulfind v2 acquisition planning is disabled.")
                } else {
                    serde_json::json!(default_disabled_reason)
                };
                serde_json::json!({
                    "id": id,
                    "name": name,
                    "description": description,
                    "riskLevel": risk_level,
                    "domain": domain,
                    "capabilities": capabilities,
                    "requiresConfiguration": requires_configuration,
                    "registered": true,
                    "active": active,
                    "networkPolicy": network_policy,
                    "disabledReason": disabled_reason,
                    "sortOrder": sort_order,
                })
            },
        )
        .collect::<Vec<_>>();

    let profile_policies = vec![
        serde_json::json!({
            "profileId": "lossless-exact",
            "profileName": "Lossless Exact",
            "providerPriority": ["LocalLibrary", "Soulseek", "NativeMesh", "MeshDht"],
            "autoDownloadEnabled": false,
            "notes": "Prefer exact local and public-network music matches before trusted mesh fallback.",
        }),
        serde_json::json!({
            "profileId": "fast-good-enough",
            "profileName": "Fast Good Enough",
            "providerPriority": ["LocalLibrary", "Soulseek"],
            "autoDownloadEnabled": false,
            "notes": "Prefer quick local or public-network candidates with bounded result collection.",
        }),
        serde_json::json!({
            "profileId": "album-complete",
            "profileName": "Album Complete",
            "providerPriority": ["LocalLibrary", "Soulseek", "NativeMesh", "MeshDht"],
            "autoDownloadEnabled": false,
            "notes": "Prefer folder-level candidates and trusted hash evidence after public search.",
        }),
        serde_json::json!({
            "profileId": "rare-hunt",
            "profileName": "Rare Hunt",
            "providerPriority": ["LocalLibrary", "Soulseek", "NativeMesh", "MeshDht", "Http", "WebDav", "S3"],
            "autoDownloadEnabled": false,
            "notes": "Allow broader configured-source review while keeping every acquisition manual.",
        }),
        serde_json::json!({
            "profileId": "conservative-network",
            "profileName": "Conservative Network",
            "providerPriority": ["LocalLibrary", "Soulseek"],
            "autoDownloadEnabled": false,
            "notes": "Keep public-network breadth narrow and avoid configured-source fallback by default.",
        }),
        serde_json::json!({
            "profileId": "mesh-preferred",
            "profileName": "Mesh Preferred",
            "providerPriority": ["LocalLibrary", "NativeMesh", "MeshDht", "Soulseek"],
            "autoDownloadEnabled": false,
            "notes": "Prefer trusted mesh candidates, then fall back to public Soulseek compatibility.",
        }),
        serde_json::json!({
            "profileId": "metadata-strict",
            "profileName": "Metadata Strict",
            "providerPriority": ["LocalLibrary", "Soulseek", "NativeMesh", "MeshDht"],
            "autoDownloadEnabled": false,
            "notes": "Prefer sources that can be explained and verified before import.",
        }),
    ];

    serde_json::json!({
        "acquisitionPlanningEnabled": acquisition_planning_enabled,
        "providers": providers,
        "profilePolicies": profile_policies,
    })
    .to_string()
}
