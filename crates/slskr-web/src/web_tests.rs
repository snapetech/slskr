use super::*;

const REACT_ROUTES: &str = include_str!("../../../web/src/components/AppRouteTable.jsx");
const REACT_NAV: &str = include_str!("../../../web/src/components/AppNavigationPrimary.jsx");
const REACT_HEADER: &str = include_str!("../../../web/src/components/AppHeaderMenu.jsx");
const STATIC_INDEX: &str = include_str!("../static/index.html");

#[path = "web_tests/fixtures.rs"]
mod fixtures;
use fixtures::{populated_route_html, rounded_vec};
#[path = "web_tests/native_actions.rs"]
mod native_actions;
#[path = "web_tests/player_search.rs"]
mod player_search;
#[path = "web_tests/response_projection.rs"]
mod response_projection;
#[path = "web_tests/route_inventory.rs"]
mod route_inventory;
#[path = "web_tests/rustymilk_geometry.rs"]
mod rustymilk_geometry;
#[path = "web_tests/rustymilk_gpu.rs"]
mod rustymilk_gpu;
#[path = "web_tests/rustymilk_presets.rs"]
mod rustymilk_presets;
#[path = "web_tests/rustymilk_runtime.rs"]
mod rustymilk_runtime;
#[path = "web_tests/rustymilk_shaders.rs"]
mod rustymilk_shaders;
#[path = "web_tests/shell_lifecycle.rs"]
mod shell_lifecycle;
#[path = "web_tests/workflow_rendering.rs"]
mod workflow_rendering;
