//! The editor's two draft-write commands driven over the Attachment, Armor and Sprite tabs:
//! `editor.set_field` and `editor.list_op`.
#![cfg(debug_assertions)]

mod armor_fields;
mod attachment_fields;
mod availability;
#[path = "../net_qa_shared/harness.rs"]
mod harness;
#[path = "../net_qa_shared/hello.rs"]
mod hello;
mod list_ops;
mod names;
#[path = "../net_qa_shared/outcome.rs"]
mod outcome;
mod refusal;
mod rows;
mod setup;
#[path = "../net_qa_shared/socket.rs"]
mod socket;
mod sprite_fields;
#[path = "../net_qa_shared/support.rs"]
mod support;
mod values;
