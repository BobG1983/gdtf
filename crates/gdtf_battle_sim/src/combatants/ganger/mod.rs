//! Ganger components: attributes, vitals, pose, life, gang roster.

mod attributes;
mod derive_stats;
mod direction;
mod gang;
pub(crate) mod injury_projection;
mod life;
mod position;
pub(crate) mod rederive;
mod stance;
mod suppression;
mod vitals;

#[cfg(test)]
mod test;

pub use attributes::{Aim, Cool, GangerAttributes, Grit, Reflexes, Speed, Strength};
pub use derive_stats::{DerivedStats, derive_stats};
pub use direction::{Direction, Facing, ForwardStep, RingSteps};
pub use gang::{GangMember, GangName, GangRegistry, GangRoster};
pub use injury_projection::{derive_stats_with_injuries, effective_luck, effective_toughness};
pub use life::{Active, LifeState};
pub use position::Position;
pub use rederive::{rederive_stats_on_injury_change, rederive_stats_on_tuning_change};
pub use stance::{Aiming, Faction, Stance, StanceKind};
pub use suppression::{Suppressed, SuppressorCell};
pub use vitals::{
    Bottle, Fight, GangerName, Hp, HpMax, Luck, Morale, Reactions, Shooting, Toughness, Tu, TuMax,
    Wounds, WoundsMax,
};
