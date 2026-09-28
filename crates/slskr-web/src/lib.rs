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
include!("search_planning.rs");
include!("rustymilk_ui.rs");
#[cfg(test)]
#[path = "web_tests.rs"]
mod tests;
