//! Net-QA integration tests (debug + `net_qa` feature only).
#![cfg(debug_assertions)]

mod app_phase_depth;
mod battle_fixture;
mod command_exchange;
mod command_set;
mod commands;
mod deadline;
mod hello_socket;
mod socket_support;
