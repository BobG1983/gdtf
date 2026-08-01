//! The GAME host's own wire types — the shapes its commands publish and take that the
//! shared protocol crate does not own (GTW-942, GTW-943, completed by GTW-944).
//!
//! A command's arguments and reply are DERIVED from their Rust types, so any type either
//! embeds has to carry a `schemars` derive. `gdtf_qa_protocol` cannot mint these: the
//! game's five state enums are the game's, its act vocabulary mirrors the game's own input
//! crate, and a wire mirror of either belongs beside the command that publishes it rather
//! than in a crate shared with the editor.
//!
//! # The type floor
//!
//! GTW-944 landed this module complete, in one go, rather than letting it grow a type at a
//! time under the twelve game-command tickets that follow. Most of it therefore has no
//! consumer yet — the requests that used to take this vocabulary went with GTW-943's cut,
//! and the commands that replace them are later tickets. That is expressed by what this
//! module EXPORTS and by the round-trip and schema suites in `test/`, not by silencing
//! `dead_code`: every type that has no consumer yet is `pub` and reachable from the crate
//! root as [`gdtf_app::qa_wire`](crate::qa_wire), and every type here — those and the
//! already-consumed `phase` mirrors — is exercised by a round-trip test and a schema test.
//! A type added without those cases fails the suite (`test/coverage.rs`), which counts only
//! a case that actually encodes a value or builds a schema — naming a type in an import or
//! a comment is not a case.
//!
//! Every type carries a doc comment saying WHERE A CALLER GETS A VALUE OF IT — which
//! command publishes it, or plainly that none does and which C-phase row would. A derived
//! schema over a transparent `u64` is just `{"type":"integer","format":"uint64"}`, so
//! without that sentence a caller reading the catalogue cannot tell a handle it must echo
//! back from a number it may invent (`04-critiques.md` #2).
//!
//! ## Members (one concern per file, per module-layout)
//!
//! - `phase` — `AppPhaseNet` and the five state mirrors it is built from. Crate-scoped:
//!   it is the one member with a live consumer already.
//! - [`cell`] — the grid coordinates: [`CellXNet`](cell::CellXNet) /
//!   [`CellYNet`](cell::CellYNet) / [`LevelNet`](cell::LevelNet) and the composed
//!   [`CellNet`](cell::CellNet) / [`CellLevelNet`](cell::CellLevelNet).
//! - [`act`] — [`NetIntent`](act::NetIntent), the mirror of the `gdtf_battle_input` act
//!   vocabulary, plus [`ActSeqNet`](act::ActSeqNet), an act-log position.
//! - [`act_payload`] — the small posture / aim / melee-target payloads an act carries.
//! - [`key`] — the keyboard vocabulary the raw-input acts name, plus the focus step an
//!   arrow key walks.
//! - [`token`] — the entity token newtypes (`Entity::to_bits` carriers).
//! - [`mod@pointer`] — the window pointer position and the mouse button a click names.
//! - [`log`] — the act-log vocabulary: why an entry happened, how many to read, how many
//!   the ring dropped.
//! - [`misc`] — the remaining scalar handles (fire-mode index, situation ref, seed, frame
//!   delay, request id) and the procgen-stepper drive command.

pub mod act;
pub mod act_payload;
pub mod cell;
pub mod key;
pub mod log;
pub mod misc;
// `phase` stays crate-scoped: `AppPhaseNet` is the `app.phase` command's reply, so it
// already has a consumer and needs no public path. Its mirrors name the game's own private
// state enums, which do not belong on any public surface.
pub(crate) mod phase;
pub mod pointer;
pub mod token;

#[cfg(test)]
mod test;

pub(crate) use phase::{
    AfterMathPhaseNet, AppPhaseNet, BattleScapePhaseNet, GamePhaseNet, LifecyclePhaseNet,
    RunningPhaseNet,
};
