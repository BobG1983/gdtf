//! The public [`SimActsPlugin`] — the sim-acts registration unit that wires the
//! `*Requested` message buffers + the per-act dispatch systems (E10.2 / GTW-204; the
//! eighth — reload — added in GTW-275; the ninth — the fieldless [`EndTurnRequested`](crate::acts::request::EndTurnRequested)
//! turn signal + the [`dispatch_end_turn`](crate::turn::dispatch_end_turn) turn-cycle engine — added in GTW-309).

mod acts;
mod messages;
mod reaction_suppression;
mod sim_acts_plugin;
mod turn_clocks;

pub use sim_acts_plugin::SimActsPlugin;
