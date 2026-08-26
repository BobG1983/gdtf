//! The editor's four shared reads: validation, families, session and draft, over the editor
//! net-QA listener.
#![cfg(debug_assertions)]

mod draft_command;
#[path = "../net_qa_shared/draft_reply.rs"]
mod draft_reply;
mod families_command;
#[path = "../net_qa_shared/harness.rs"]
mod harness;
#[path = "../net_qa_shared/hello.rs"]
mod hello;
#[path = "../net_qa_shared/load_case.rs"]
mod load_case;
#[path = "../net_qa_shared/mirror.rs"]
mod mirror;
mod names;
#[path = "../net_qa_shared/outcome.rs"]
mod outcome;
mod rows;
#[path = "../net_qa_shared/save_fault.rs"]
mod save_fault;
mod session_command;
#[path = "../net_qa_shared/socket.rs"]
mod socket;
#[path = "../net_qa_shared/support.rs"]
mod support;
mod validation_command;
