//! The editor's two draft-write commands driven over every form tab they reach:
//! `editor.set_field` and `editor.list_op`.
#![cfg(debug_assertions)]

mod armor_fields;
mod attachment_fields;
mod availability;
#[path = "../net_qa_shared/harness.rs"]
mod harness;
#[path = "../net_qa_shared/hello.rs"]
mod hello;
mod injury_fields;
mod injury_lists;
mod list_ops;
#[path = "../net_qa_shared/load_case.rs"]
mod load_case;
mod melee_weapon_attachments;
mod melee_weapon_fields;
mod melee_weapon_lists;
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
mod terrain_clears;
mod terrain_fields;
mod terrain_gates;
mod values;
mod walk;
