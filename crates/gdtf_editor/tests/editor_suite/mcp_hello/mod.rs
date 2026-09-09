//! Hello/version negotiation, and the editor's lifecycle command layer — phase, mode tab, blank,
//! load, save and last-save — over the editor MCP listener.

mod assertions;
mod client;
mod command_set;
mod drafts;
mod keys;
mod last_save_command;
mod lifecycle;
mod load_command;
mod load_tab_scope;
mod negotiation;
mod new_command;
mod phase_command;
mod phase_rows;
mod rows;
mod save_command;
mod set_mode_command;
