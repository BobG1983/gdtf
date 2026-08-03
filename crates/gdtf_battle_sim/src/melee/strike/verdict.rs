use crate::{
    armor_wear::ArmorWearOutcome, injuries::RolledInjury, melee::Connected, resolve_hit::HpDamage,
    severity::Severity,
};

#[derive(Debug, Clone, PartialEq)]
pub struct MeleeStrike {
        pub connect:   Connected,
        pub severity:  Severity,
        pub hp_damage: HpDamage,
                    pub wear:      ArmorWearOutcome,
                        pub injury:    Option<RolledInjury>,
}

impl MeleeStrike {
                pub(super) const MISS: Self = Self {
        connect:   Connected::new(false),
        severity:  Severity::None,
        hp_damage: HpDamage::new(0),
        wear:      ArmorWearOutcome::Unaffected,
        injury:    None,
    };

                                    pub(super) const CORPSE: Self = Self {
        connect:   Connected::new(true),
        severity:  Severity::None,
        hp_damage: HpDamage::new(0),
        wear:      ArmorWearOutcome::Unaffected,
        injury:    None,
    };
}
