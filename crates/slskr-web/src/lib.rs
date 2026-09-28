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
include!("web_actions.rs");
include!("search_planning.rs");
include!("rustymilk_ui.rs");
include!("web_tests.rs");
