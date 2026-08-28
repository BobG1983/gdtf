//! The Injury tab's weighting table over the wire: pick a table, read it, edit its rows, save it.
#![cfg(debug_assertions)]

mod assets_root;
mod autoload;
#[path = "../net_qa_shared/bad_arguments.rs"]
mod bad_arguments;
#[path = "../net_qa_shared/harness.rs"]
mod harness;
#[path = "../net_qa_shared/hello.rs"]
mod hello;
#[path = "../net_qa_shared/mirror.rs"]
mod mirror;
mod names;
#[path = "../net_qa_shared/outcome.rs"]
mod outcome;
mod refusal;
mod row_edits;
mod row_refusals;
mod rows;
mod save;
#[path = "../net_qa_shared/save_fault.rs"]
mod save_fault;
mod select;
mod setup;
#[path = "../net_qa_shared/socket.rs"]
mod socket;
#[path = "../net_qa_shared/support.rs"]
mod support;
