//! Registers act messages and wires dispatch systems.

mod acts;
mod messages;
mod reaction_suppression;
mod sim_acts_plugin;
mod turn_clocks;

pub use sim_acts_plugin::SimActsPlugin;
