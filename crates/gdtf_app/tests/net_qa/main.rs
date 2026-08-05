//! Net-QA integration tests (debug + `net_qa` feature only).
#![cfg(debug_assertions)]

mod app_phase_depth;
mod battle_fixture;
mod battle_inspect;
mod battle_offers;
mod battle_reads;
mod battle_roster;
mod battle_screen_lag;
mod battle_selection;
mod battle_setup;
mod battle_sightline;
mod battle_socket;
mod battle_turn;
mod battle_visible;
mod capture_fixture;
mod capture_screenshot;
mod catalogue_shapes;
mod command_exchange;
mod command_set;
mod commands;
mod deadline;
mod facts_probe;
mod hello_socket;
mod log_read;
mod menu_spawned;
mod playback_state;
mod settings_read;
mod socket_support;
mod ui_focus;
