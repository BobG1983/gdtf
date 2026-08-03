//! Outcome of a melee strike attempt.

use crate::{
    armor_wear::ArmorWearOutcome, injuries::RolledInjury, melee::Connected, resolve_hit::HpDamage,
    severity::Severity,
};

/// Full result of resolving a melee strike.
#[derive(Debug, Clone, PartialEq)]
pub struct MeleeStrike {
    /// Whether the attack connected.
    pub connect: Connected,
    /// Wound severity.
    pub severity: Severity,
    /// HP damage applied.
    pub hp_damage: HpDamage,
    /// Armor wear outcome.
    pub wear: ArmorWearOutcome,
    /// Optional rolled injury.
    pub injury: Option<RolledInjury>,
}

impl MeleeStrike {
    /// Miss: no connect, no damage.
    pub(super) const MISS: Self = Self {
        connect: Connected::new(false),
        severity: Severity::None,
        hp_damage: HpDamage::new(0),
        wear: ArmorWearOutcome::Unaffected,
        injury: None,
    };

    /// Target already dead / no synthesis.
    pub(super) const CORPSE: Self = Self {
        connect: Connected::new(true),
        severity: Severity::None,
        hp_damage: HpDamage::new(0),
        wear: ArmorWearOutcome::Unaffected,
        injury: None,
    };
}
