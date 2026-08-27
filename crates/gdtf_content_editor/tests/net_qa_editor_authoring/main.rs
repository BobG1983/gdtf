//! One authoring session over the real editor socket: open a tab, blank its draft, write it
//! field by field and list by list, save it, and read the result back.
#![cfg(debug_assertions)]

#[path = "../net_qa_shared/draft_reply.rs"]
mod draft_reply;
#[path = "../net_qa_shared/harness.rs"]
mod harness;
#[path = "../net_qa_shared/hello.rs"]
mod hello;
#[path = "../net_qa_shared/mirror.rs"]
mod mirror;
mod names;
#[path = "../net_qa_shared/outcome.rs"]
mod outcome;
mod rows;
#[path = "../net_qa_shared/save_fault.rs"]
mod save_fault;
mod session;
mod setup;
#[path = "../net_qa_shared/socket.rs"]
mod socket;
#[path = "../net_qa_shared/support.rs"]
mod support;
mod writes;
