include!("route_catalog.rs");
include!("route_controls.rs");
mod route_data_projection;
pub use self::route_data_projection::*;
mod route_reference_panels;
use self::route_reference_panels::*;
mod native_row_rendering;
use self::native_row_rendering::*;
mod native_tab_rendering;
use self::native_tab_rendering::*;
mod native_workspace_panels;
use self::native_workspace_panels::*;
mod route_workflow_rendering;
pub use self::route_workflow_rendering::*;
mod route_shell_runtime;
pub use self::route_shell_runtime::*;
#[cfg(target_arch = "wasm32")]
mod native_table_navigation;
#[cfg(target_arch = "wasm32")]
use self::native_table_navigation::*;
#[cfg(target_arch = "wasm32")]
mod native_transfer_controls;
#[cfg(target_arch = "wasm32")]
use self::native_transfer_controls::*;
#[path = "web_action_owners/experience_action_models.rs"]
mod experience_action_models;
use self::experience_action_models::*;
pub use self::experience_action_models::{AutomationRecipe, ExperiencePreference};
#[cfg(target_arch = "wasm32")]
#[path = "web_action_owners/native_action_execution.rs"]
mod native_action_execution;
#[cfg(target_arch = "wasm32")]
use self::native_action_execution::*;
#[cfg(any(target_arch = "wasm32", test))]
#[path = "web_action_owners/native_action_projection.rs"]
mod native_action_projection;
#[cfg(any(target_arch = "wasm32", test))]
use self::native_action_projection::*;
#[cfg(target_arch = "wasm32")]
#[path = "web_action_owners/native_form_values.rs"]
mod native_form_values;
#[cfg(target_arch = "wasm32")]
use self::native_form_values::*;
#[cfg(target_arch = "wasm32")]
#[path = "web_action_owners/native_reference_controls.rs"]
mod native_reference_controls;
#[cfg(target_arch = "wasm32")]
use self::native_reference_controls::*;
#[cfg(target_arch = "wasm32")]
#[path = "web_action_owners/native_row_context.rs"]
mod native_row_context;
#[cfg(target_arch = "wasm32")]
use self::native_row_context::*;
#[cfg(target_arch = "wasm32")]
#[path = "web_action_owners/native_share_access.rs"]
mod native_share_access;
#[cfg(target_arch = "wasm32")]
use self::native_share_access::*;
#[cfg(target_arch = "wasm32")]
#[path = "web_action_owners/native_table_filters.rs"]
mod native_table_filters;
#[cfg(target_arch = "wasm32")]
use self::native_table_filters::*;
#[cfg(target_arch = "wasm32")]
#[path = "web_action_owners/native_table_selection.rs"]
mod native_table_selection;
#[cfg(target_arch = "wasm32")]
use self::native_table_selection::*;
#[cfg(target_arch = "wasm32")]
#[path = "web_action_owners/native_table_sorting.rs"]
mod native_table_sorting;
#[cfg(target_arch = "wasm32")]
use self::native_table_sorting::*;
#[cfg(target_arch = "wasm32")]
#[path = "web_action_owners/native_wishlist_actions.rs"]
mod native_wishlist_actions;
#[cfg(target_arch = "wasm32")]
use self::native_wishlist_actions::*;
#[path = "web_action_owners/player_action_models.rs"]
mod player_action_models;
#[cfg(target_arch = "wasm32")]
use self::player_action_models::percent_encode_player_stream_component;
pub use self::player_action_models::{
    player_rating_key, player_rating_summary, player_stream_url, PlayerRadioPlan, PlayerRadioQuery,
    SimilarQueueCandidate,
};
#[path = "web_action_owners/search_action_models.rs"]
mod search_action_models;
pub use self::search_action_models::{
    SearchActionPreview, SearchCandidateRank, SearchDuplicateGroup,
};
#[cfg(target_arch = "wasm32")]
#[path = "search_planning_owners/browser_local_preferences.rs"]
mod browser_local_preferences;
#[cfg(target_arch = "wasm32")]
use self::browser_local_preferences::*;
#[path = "search_planning_owners/experience_reports.rs"]
mod experience_reports;
pub use self::experience_reports::{
    automation_dry_run_report, automation_history_report, automation_summary_from_state,
    default_experience_preferences, experience_preferences_report,
};
#[cfg(target_arch = "wasm32")]
#[path = "search_planning_owners/player_browser_controls.rs"]
mod player_browser_controls;
#[cfg(target_arch = "wasm32")]
use self::player_browser_controls::*;
#[path = "search_planning_owners/player_queue_planning.rs"]
mod player_queue_planning;
#[cfg(target_arch = "wasm32")]
use self::player_queue_planning::current_player_track;
pub use self::player_queue_planning::{
    build_similar_queue_candidates, player_radio_query_from_now_playing_body,
    player_similarity_score, similar_queue_search_queries,
};
#[path = "search_planning_owners/player_radio_planning.rs"]
mod player_radio_planning;
use self::player_radio_planning::*;
pub use self::player_radio_planning::{
    build_player_radio_plan, build_player_radio_search_path, player_radio_copy_text,
    player_radio_queries,
};
#[path = "search_planning_owners/player_status_projection.rs"]
mod player_status_projection;
pub use self::player_status_projection::{
    player_now_playing_text, player_party_text, player_transfer_text, player_visualizer_text,
};
#[path = "search_planning_owners/search_previews.rs"]
mod search_previews;
pub use self::search_previews::{
    build_search_action_preview, deduplicate_search_response_groups, format_search_action_preview,
    search_planner_report,
};
#[path = "search_planning_owners/search_ranking.rs"]
mod search_ranking;
pub use self::search_ranking::rank_search_candidate;
use self::search_ranking::*;
#[cfg(target_arch = "wasm32")]
#[path = "search_planning_owners/visualizer_startup.rs"]
mod visualizer_startup;
#[cfg(target_arch = "wasm32")]
use self::visualizer_startup::*;
#[cfg(target_arch = "wasm32")]
#[path = "search_planning_owners/workspace_action_mounting.rs"]
mod workspace_action_mounting;
#[cfg(target_arch = "wasm32")]
use self::workspace_action_mounting::*;
#[cfg(target_arch = "wasm32")]
#[path = "search_planning_owners/workspace_table_controls.rs"]
mod workspace_table_controls;
#[cfg(target_arch = "wasm32")]
use self::workspace_table_controls::*;
#[cfg(any(target_arch = "wasm32", test))]
#[path = "rustymilk_ui_owners/browser_http_requests.rs"]
mod browser_http_requests;
#[cfg(target_arch = "wasm32")]
use self::browser_http_requests::*;
#[cfg(target_arch = "wasm32")]
#[path = "rustymilk_ui_owners/live_player_refresh.rs"]
mod live_player_refresh;
#[cfg(target_arch = "wasm32")]
use self::live_player_refresh::*;
#[cfg(target_arch = "wasm32")]
#[path = "rustymilk_ui_owners/live_route_refresh.rs"]
mod live_route_refresh;
#[cfg(target_arch = "wasm32")]
use self::live_route_refresh::*;
#[cfg(target_arch = "wasm32")]
#[path = "rustymilk_ui_owners/visualizer_audio_analysis.rs"]
mod visualizer_audio_analysis;
#[cfg(target_arch = "wasm32")]
use self::visualizer_audio_analysis::*;
#[cfg(target_arch = "wasm32")]
#[path = "rustymilk_ui_owners/visualizer_automation.rs"]
mod visualizer_automation;
#[cfg(target_arch = "wasm32")]
use self::visualizer_automation::*;
#[cfg(target_arch = "wasm32")]
#[path = "rustymilk_ui_owners/visualizer_browser_controls.rs"]
mod visualizer_browser_controls;
#[cfg(target_arch = "wasm32")]
use self::visualizer_browser_controls::*;
#[cfg(target_arch = "wasm32")]
#[path = "rustymilk_ui_owners/visualizer_file_imports.rs"]
mod visualizer_file_imports;
#[cfg(target_arch = "wasm32")]
use self::visualizer_file_imports::*;
#[cfg(target_arch = "wasm32")]
#[path = "rustymilk_ui_owners/visualizer_library_actions.rs"]
mod visualizer_library_actions;
#[cfg(target_arch = "wasm32")]
use self::visualizer_library_actions::*;
#[cfg(target_arch = "wasm32")]
#[path = "rustymilk_ui_owners/visualizer_library_projection.rs"]
mod visualizer_library_projection;
#[cfg(target_arch = "wasm32")]
use self::visualizer_library_projection::*;
#[cfg(target_arch = "wasm32")]
#[path = "rustymilk_ui_owners/visualizer_library_storage.rs"]
mod visualizer_library_storage;
#[cfg(target_arch = "wasm32")]
use self::visualizer_library_storage::*;
#[cfg(target_arch = "wasm32")]
#[path = "rustymilk_ui_owners/visualizer_playlist_actions.rs"]
mod visualizer_playlist_actions;
#[cfg(target_arch = "wasm32")]
use self::visualizer_playlist_actions::*;
#[cfg(target_arch = "wasm32")]
#[path = "rustymilk_ui_owners/visualizer_preset_editor.rs"]
mod visualizer_preset_editor;
#[cfg(target_arch = "wasm32")]
use self::visualizer_preset_editor::*;
#[cfg(test)]
#[path = "web_tests.rs"]
mod tests;
