//! The editor's theme helpers, its Injury sub-tab write, and the two host-level commands it
//! publishes under the game's own spellings — `capture.screenshot` and `wait` — driven over the
//! editor MCP listener.

mod capture_screenshot_command;
mod delete_record_command;
mod drafts;
mod fixture_root;
mod keys;
mod names;
mod rows;
mod select_injury_tab_command;
mod select_theme_command;
mod set_default_floor_command;
mod setup;
mod toggle_terrain_command;
mod wait_command;
