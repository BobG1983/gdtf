//! The GAME host's own wire types — the shapes its commands publish that the shared
//! protocol crate does not own (GTW-942).
//!
//! A command's reply is DERIVED from its Rust type, so any type a reply embeds has to
//! carry a `schemars` derive. `gdtf_qa_protocol` cannot mint these: the game's five state
//! enums are the game's, and a wire mirror of them belongs beside the command that
//! publishes them rather than in a crate shared with the editor.
//!
//! ## Members (one concern per file, per module-layout)
//!
//! - [`phase`] — [`AppPhaseNet`] and the five state mirrors it is built from.

pub(crate) mod phase;

#[cfg(test)]
mod test;

pub(crate) use phase::{
    AfterMathPhaseNet, AppPhaseNet, BattleScapePhaseNet, GamePhaseNet, LifecyclePhaseNet,
    RunningPhaseNet,
};
