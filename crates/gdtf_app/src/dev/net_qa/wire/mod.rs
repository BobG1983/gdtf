//! Network wire types for the game QA control channel.

/// Act sequence and intent payloads.
pub mod act;
/// Stance, aim, facing, and melee target payloads.
pub mod act_payload;
/// Cell and cell-level coordinates.
pub mod cell;
/// Keyboard and focus step keys.
pub mod key;
/// Combat log provenance and read caps.
pub mod log;
/// Fire mode, seed, request id, and stepper commands.
pub mod misc;
pub(crate) mod phase;
/// Pointer position and mouse buttons.
pub mod pointer;
/// Menu and shell read payloads.
pub mod shell;
/// Entity tokens for gangers, doors, and emplacements.
pub mod token;
pub(crate) mod wait;

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
