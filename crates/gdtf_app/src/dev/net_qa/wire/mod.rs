//! Network wire types for the game QA control channel.

/// Act sequence and intent payloads.
pub mod act;
/// Stance, aim, facing, and melee target payloads.
pub mod act_payload;
/// Cell and cell-level coordinates.
pub mod cell;
/// Act-log deed kinds.
pub mod deed;
/// Cover blocks and the inspect panel's decision.
pub mod inspect;
/// Keyboard and focus step keys.
pub mod key;
/// Combat log entries, provenance and read caps.
pub mod log;
/// Fire mode, seed, request id, and stepper commands.
pub mod misc;
/// Contextual act offers.
pub mod offer;
pub(crate) mod phase;
/// Pointer position and mouse buttons.
pub mod pointer;
/// Roster cards.
pub mod roster;
/// Menu and shell read payloads.
pub mod shell;
/// Sightline answers.
pub mod sight;
/// Entity tokens for gangers, doors, and emplacements.
pub mod token;
/// Enemies, doors and cover inside the lit area.
pub mod visible;
/// Ganger vitals.
pub mod vitals;
pub(crate) mod wait;
/// Wounds and lasting injuries.
pub mod wound;

#[cfg(test)]
mod test;

crate::support_use!(
    phase::{
        AfterMathPhaseNet, AppPhaseNet, BattleScapePhaseNet, GamePhaseNet, LifecyclePhaseNet,
        RunningPhaseNet,
    };
);
crate::support_use!(wait::{AppPhaseTargetNet, WaitConditionNet};);
#[cfg(feature = "headless_test")]
pub use wait::ActCountNet;
