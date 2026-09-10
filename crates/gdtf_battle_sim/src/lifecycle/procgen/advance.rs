//! How far a staged generation may run this frame.

use bevy::prelude::Resource;

/// How far the staged driver may run this frame. Absent means [`ProcgenAdvance::AllStages`].
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcgenAdvance {
    /// Run no stage.
    Hold,
    /// Run one stage, then hold again.
    OneStage,
    /// Run every remaining stage.
    AllStages,
}
