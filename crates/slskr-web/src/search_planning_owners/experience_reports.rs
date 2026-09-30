use super::*;
pub fn default_experience_preferences() -> serde_json::Map<String, serde_json::Value> {
    experience_preferences()
        .iter()
        .map(|preference| {
            let value = if preference.input == "checkbox" {
                serde_json::Value::Bool(preference.default_value == "true")
            } else {
                serde_json::Value::String(preference.default_value.to_string())
            };
            (preference.id.to_string(), value)
        })
        .collect()
}

pub(super) fn preference_string(
    values: &serde_json::Map<String, serde_json::Value>,
    key: &str,
    fallback: &str,
) -> String {
    values
        .get(key)
        .map(json_scalar_preview)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| fallback.to_string())
}

pub(super) fn preference_bool(
    values: &serde_json::Map<String, serde_json::Value>,
    key: &str,
    fallback: bool,
) -> bool {
    values
        .get(key)
        .and_then(|value| value.as_bool())
        .unwrap_or(fallback)
}

pub fn experience_preferences_report(
    values: &serde_json::Map<String, serde_json::Value>,
) -> String {
    [
        "slskr experience preferences".to_string(),
        format!(
            "Search: ranking={}, condition={}, duplicate_folding={}, previews={}",
            preference_string(values, "searchRankingProfile", "balanced"),
            preference_string(values, "searchPreferredCondition", "lossless"),
            preference_bool(values, "searchDuplicateFolding", true),
            preference_string(values, "searchActionPreviewDensity", "detailed")
        ),
        format!(
            "Discovery: approval={}, confidence>={}, stale_days={}",
            preference_string(values, "discoveryApprovalFilter", "all"),
            preference_string(values, "discoveryConfidenceFloor", "0.70"),
            preference_string(values, "discoveryStaleDays", "14")
        ),
        format!(
            "Player: queue_auto_fill={}, radio_seed={}, ratings={}, history={}, scrobble={}, visualizer={}, shortcuts={}",
            preference_bool(values, "playerQueueAutoFill", false),
            preference_string(values, "playerRadioSeedMode", "current"),
            preference_bool(values, "playerShowRatings", true),
            preference_bool(values, "playerCaptureHistory", true),
            preference_string(values, "playerScrobbleMode", "manual"),
            preference_string(values, "playerDefaultVisualizer", "last"),
            preference_bool(values, "playerKeyboardShortcuts", true)
        ),
        format!(
            "Messages: dense={}, pinned_restore={}, unread_badges={}, search={}",
            preference_bool(values, "messagesDenseMode", false),
            preference_bool(values, "messagesPinnedRestore", true),
            preference_bool(values, "messagesUnreadBadges", true),
            preference_bool(values, "messagesSearchEnabled", true)
        ),
    ]
    .join("\n")
}

pub fn automation_summary_from_state(
    state: &serde_json::Map<String, serde_json::Value>,
) -> (usize, usize, usize) {
    let enabled = automation_recipes()
        .iter()
        .filter(|recipe| {
            state
                .get(recipe.id)
                .and_then(|entry| entry.get("enabled"))
                .and_then(|value| value.as_bool())
                .unwrap_or(recipe.enabled_by_default)
        })
        .count();
    let total = automation_recipes().len();
    (total, enabled, total.saturating_sub(enabled))
}

pub fn automation_dry_run_report(recipe: AutomationRecipe, timestamp: &str) -> serde_json::Value {
    serde_json::json!({
        "approvalGate": recipe.approval_gate,
        "cooldown": recipe.cooldown,
        "executed": false,
        "fileImpact": recipe.file_impact,
        "generatedAt": timestamp,
        "maxRunTime": recipe.max_run_time,
        "networkImpact": recipe.network_impact,
        "recipeId": recipe.id,
        "title": recipe.title,
    })
}

pub fn automation_history_report(state: &serde_json::Map<String, serde_json::Value>) -> String {
    let mut entries = Vec::new();
    for recipe in automation_recipes() {
        let stored = state.get(recipe.id);
        let enabled = stored
            .and_then(|entry| entry.get("enabled"))
            .and_then(|value| value.as_bool())
            .unwrap_or(recipe.enabled_by_default);
        let last_dry_run = stored
            .and_then(|entry| entry.get("lastDryRunAt"))
            .map(json_scalar_preview)
            .filter(|value| !value.is_empty());
        let last_run = stored
            .and_then(|entry| entry.get("lastRunAt"))
            .map(json_scalar_preview)
            .filter(|value| !value.is_empty());
        if enabled || last_dry_run.is_some() || last_run.is_some() {
            entries.push((recipe, enabled, last_dry_run, last_run));
        }
    }

    let mut lines = vec![
        "slskr automation review history".to_string(),
        format!("Entries: {}", entries.len()),
        String::new(),
    ];
    if entries.is_empty() {
        lines.push("No enabled recipes or dry-run checkpoints.".to_string());
        return lines.join("\n");
    }
    for (recipe, enabled, last_dry_run, last_run) in entries {
        lines.push(format!("- {}", recipe.title));
        lines.push(format!("  Enabled: {}", if enabled { "yes" } else { "no" }));
        lines.push(format!(
            "  Last run: {}",
            last_run.unwrap_or_else(|| "not recorded".to_string())
        ));
        lines.push(format!(
            "  Last dry run: {}",
            last_dry_run.unwrap_or_else(|| "not recorded".to_string())
        ));
        lines.push(format!("  Network impact: {}", recipe.network_impact));
        lines.push(format!("  File impact: {}", recipe.file_impact));
    }
    lines.join("\n")
}
