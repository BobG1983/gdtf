//! What kind of act a log line records, on the wire.

use gdtf_battle_sim::{
    act_log::ActDeed,
    acts::{MoveRejection, ReloadOutcome},
};
use serde::{Deserialize, Serialize};

/// How a reload the sim heard turned out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReloadOutcomeNet {
    /// The magazine was refilled.
    Reloaded,
    /// The magazine was already full.
    AlreadyFull,
    /// The actor could not afford the reload.
    NoTu,
}

impl ReloadOutcomeNet {
    /// Mirror the sim's own outcome.
    #[must_use]
    pub const fn from_sim(outcome: ReloadOutcome) -> Self {
        match outcome {
            ReloadOutcome::Reloaded => Self::Reloaded,
            ReloadOutcome::AlreadyFull => Self::AlreadyFull,
            ReloadOutcome::NoTu => Self::NoTu,
        }
    }
}

/// Why the sim turned a move down.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MoveRejectionNet {
    /// No route reaches the destination.
    Unreachable,
    /// The route costs more time units than the actor has.
    Unaffordable,
    /// Suppression pinned the actor down.
    Suppressed,
}

impl MoveRejectionNet {
    /// Mirror the sim's own reason.
    #[must_use]
    pub const fn from_sim(reason: MoveRejection) -> Self {
        match reason {
            MoveRejection::Unreachable => Self::Unreachable,
            MoveRejection::Unaffordable => Self::Unaffordable,
            MoveRejection::Suppressed => Self::Suppressed,
        }
    }
}

/// Kind of a logged act, one variant per sim deed, carrying what a refusal needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ActDeedKindNet {
    /// A faction's turn began.
    TurnBegan,
    /// Facing, stance, aim or suppression changed.
    PostureChanged,
    /// Single-cell step.
    Stepped,
    /// Multi-cell move completed.
    MovedTo,
    /// Move was refused, carrying the sim's own reason.
    MoveRefused {
        /// Why the sim turned it down.
        reason: MoveRejectionNet,
    },
    /// Fire was declared.
    Fired,
    /// A single round resolved.
    RoundResolved,
    /// Reload finished, carrying how it turned out.
    Reloaded {
        /// What the sim did with the reload.
        outcome: ReloadOutcomeNet,
    },
    /// Magazine contents changed.
    MagazineChanged,
    /// Injury applied.
    Injured,
    /// Vitals snapshot changed.
    VitalsChanged,
    /// Fall completed.
    Fell,
    /// HP damage applied to a target.
    Struck,
    /// Actor died.
    DiedAt,
    /// Suppression applied.
    Suppressed,
    /// Armor piece broke.
    ArmorBroke,
    /// Damage-over-time started.
    DotStarted,
    /// Field effect started on a cell.
    FieldStarted,
    /// Bleed effect started.
    BleedStarted,
    /// Bleed tick occurred.
    Bled,
    /// Damage-over-time tick.
    DotTicked,
    /// Field tick.
    FieldTicked,
    /// Cover was destroyed.
    CoverSmashed,
    /// Melee hit landed.
    MeleeLanded,
    /// Thrown weapon or item landed.
    ThrowLanded,
    /// Life state transition.
    LifeChanged,
}

impl ActDeedKindNet {
    /// Mirror the sim's deed as a kind, keeping only what a refusal needs.
    #[must_use]
    pub const fn from_deed(deed: &ActDeed) -> Self {
        match deed {
            ActDeed::TurnBegan { .. } => Self::TurnBegan,
            ActDeed::PostureChanged { .. } => Self::PostureChanged,
            ActDeed::Stepped { .. } => Self::Stepped,
            ActDeed::MovedTo { .. } => Self::MovedTo,
            ActDeed::MoveRefused { reason } => Self::MoveRefused {
                reason: MoveRejectionNet::from_sim(*reason),
            },
            ActDeed::Fired { .. } => Self::Fired,
            ActDeed::RoundResolved { .. } => Self::RoundResolved,
            ActDeed::Reloaded { outcome } => Self::Reloaded {
                outcome: ReloadOutcomeNet::from_sim(*outcome),
            },
            ActDeed::MagazineChanged { .. } => Self::MagazineChanged,
            ActDeed::Injured { .. } => Self::Injured,
            ActDeed::VitalsChanged { .. } => Self::VitalsChanged,
            ActDeed::Fell { .. } => Self::Fell,
            ActDeed::Struck { .. } => Self::Struck,
            ActDeed::DiedAt { .. } => Self::DiedAt,
            ActDeed::Suppressed { .. } => Self::Suppressed,
            ActDeed::ArmorBroke { .. } => Self::ArmorBroke,
            ActDeed::DotStarted { .. } => Self::DotStarted,
            ActDeed::FieldStarted { .. } => Self::FieldStarted,
            ActDeed::BleedStarted => Self::BleedStarted,
            ActDeed::Bled => Self::Bled,
            ActDeed::DotTicked { .. } => Self::DotTicked,
            ActDeed::FieldTicked { .. } => Self::FieldTicked,
            ActDeed::CoverSmashed { .. } => Self::CoverSmashed,
            ActDeed::MeleeLanded { .. } => Self::MeleeLanded,
            ActDeed::ThrowLanded { .. } => Self::ThrowLanded,
            ActDeed::LifeChanged { .. } => Self::LifeChanged,
        }
    }
}
