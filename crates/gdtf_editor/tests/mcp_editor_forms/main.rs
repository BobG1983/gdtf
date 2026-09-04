//! The editor's two draft-write commands driven over every form tab they reach:
//! `editor.set_field` and `editor.list_op`.
#[cfg(not(feature = "mcp"))]
compile_error!("the mcp test suites need the host package's `mcp` feature");

mod armor_fields;
mod attachment_fields;
mod availability;
#[path = "../mcp_shared/bad_arguments.rs"]
mod bad_arguments;
#[path = "../mcp_shared/draft_reply.rs"]
mod draft_reply;
mod field_fields;
mod field_lists;
mod gang_fields;
mod gang_lists;
#[path = "../mcp_shared/harness.rs"]
mod harness;
#[path = "../mcp_shared/hello.rs"]
mod hello;
mod injury_fields;
mod injury_lists;
mod list_ops;
#[path = "../mcp_shared/load_case.rs"]
mod load_case;
mod melee_weapon_attachments;
mod melee_weapon_fields;
mod melee_weapon_lists;
#[path = "../mcp_shared/mirror.rs"]
mod mirror;
mod names;
mod on_death_lists;
#[path = "../mcp_shared/outcome.rs"]
mod outcome;
mod refusal;
mod rows;
#[path = "../mcp_shared/save_fault.rs"]
mod save_fault;
mod set_at_refusals;
mod setup;
#[path = "../mcp_shared/socket.rs"]
mod socket;
mod sprite_fields;
#[path = "../mcp_shared/support.rs"]
mod support;
mod terrain_clamps;
mod terrain_clears;
mod terrain_fields;
mod terrain_gates;
mod terrain_leaves_behind;
mod terrain_on_death;
mod theme_tab;
mod values;
mod walk;
mod weapon_attachments;
mod weapon_fields;
mod weapon_lists;
mod weapon_on_death;
