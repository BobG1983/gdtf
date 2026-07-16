//! [`GangerView`] — the per-ganger read-model card (GTW-734).

use serde::{Deserialize, Serialize};

use super::{
    injury::InjurySummaryNet,
    life::LifeStateNet,
    stat::{FactionNet, GangerNameNet, HpMaxNet, HpNet, TuMaxNet, TuNet, WoundsMaxNet, WoundsNet},
    weapon::WeaponView,
};
use crate::{
    ids::{CellLevelNet, GangerToken},
    intent::{AimNet, FacingNet, StanceNet},
};

/// One ganger's wire snapshot — the curated card a QA client reads and targets.
///
/// Carries the contract's required fields — position, [`LifeStateNet`] mirror, TU, HP,
/// stance, injury summary, and the weapon with its INDEXED fire-mode list — plus the
/// identity + posture context a client needs to reason and target: the
/// [`token`](Self::token) (echoed back by an entity-targeted
/// [`NetIntent`](crate::intent::NetIntent)), the [`name`](Self::name) /
/// [`faction`](Self::faction), the [`facing`](Self::facing) / [`aiming`](Self::aiming)
/// posture, and the Wounds pool that (with HP) drives the [`life`](Self::life) machine.
/// An independent serde struct built from the sim's per-ganger components; never a leak
/// of them.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GangerView {
    /// The ganger's wire handle — echoed back to target it.
    pub token:      GangerToken,
    /// The ganger's display name.
    pub name:       GangerNameNet,
    /// Which side the ganger fights for.
    pub faction:    FactionNet,
    /// The `(cell, level)` key the ganger occupies.
    pub position:   CellLevelNet,
    /// The direction the ganger faces.
    pub facing:     FacingNet,
    /// Whether the ganger is aiming (vs hip-firing).
    pub aiming:     AimNet,
    /// The ganger's posture.
    pub stance:     StanceNet,
    /// The ganger's terminal life state.
    pub life:       LifeStateNet,
    /// Current hit points (the knock-down pool).
    pub hp:         HpNet,
    /// The HP ceiling (the bar denominator).
    pub hp_max:     HpMaxNet,
    /// Current Wounds (the life pool).
    pub wounds:     WoundsNet,
    /// The Wounds ceiling (the pip count).
    pub wounds_max: WoundsMaxNet,
    /// Current Time Units (the action budget).
    pub tu:         TuNet,
    /// The TU ceiling (the round-start budget).
    pub tu_max:     TuMaxNet,
    /// The ganger's durable injuries, summarized.
    pub injuries:   InjurySummaryNet,
    /// The ganger's wielded weapon + its indexed fire-mode list.
    pub weapon:     WeaponView,
}
