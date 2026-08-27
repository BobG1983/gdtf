//! The editor's theme helpers, its Injury sub-tab write, and the two host-level commands it
//! publishes under the game's own spellings — `capture.screenshot` and `wait` — driven over the
//! editor net-QA listener.
#![cfg(debug_assertions)]

mod capture_screenshot_command;
mod drafts;
#[path = "../net_qa_shared/harness.rs"]
mod harness;
#[path = "../net_qa_shared/hello.rs"]
mod hello;
mod keys;
#[path = "../net_qa_shared/mirror.rs"]
mod mirror;
mod names;
#[path = "../net_qa_shared/outcome.rs"]
mod outcome;
mod rows;
mod select_injury_tab_command;
mod select_theme_command;
mod set_default_floor_command;
mod setup;
#[path = "../net_qa_shared/socket.rs"]
mod socket;
#[path = "../net_qa_shared/support.rs"]
mod support;
mod toggle_terrain_command;
mod wait_command;
#[path = "../net_qa_shared/world.rs"]
mod world;
