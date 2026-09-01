//! The editor's two draft-write commands driven over every form tab they reach:
//! `editor.set_field` and `editor.list_op`.
#![cfg(debug_assertions)]

mod armor_fields;
mod attachment_fields;
mod availability;
#[path = "../net_qa_shared/bad_arguments.rs"]
mod bad_arguments;
#[path = "../net_qa_shared/draft_reply.rs"]
mod draft_reply;
mod field_fields;
mod field_lists;
mod gang_fields;
mod gang_lists;
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
#[path = "../net_qa_shared/mirror.rs"]
mod mirror;
mod names;
mod on_death_lists;
#[path = "../net_qa_shared/outcome.rs"]
mod outcome;
mod refusal;
mod rows;
#[path = "../net_qa_shared/save_fault.rs"]
mod save_fault;
mod set_at_refusals;
mod setup;
#[path = "../net_qa_shared/socket.rs"]
mod socket;
mod sprite_fields;
#[path = "../net_qa_shared/support.rs"]
mod support;
mod terrain_clamps;
mod terrain_clears;
mod terrain_fields;
mod terrain_gates;
mod terrain_on_death;
mod theme_tab;
mod values;
mod walk;
mod weapon_attachments;
mod weapon_fields;
mod weapon_lists;
mod weapon_on_death;
