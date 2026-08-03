//! Melee weapon form draft resource.

use bevy::prelude::*;
use gdtf_battle_sim::{
    equipment::attachments::WeaponSlots,
    weapon::{
        DamageType, FatalBias, FightMode, FightModeKind, FightModeSpec, Handedness,
        MeleeWeaponSpec, Reach, Shove, Strikes, TuCost, WeaponDamage, WeaponName, WeaponPunch,
        WeaponShred,
    },
};

fn seed_spec() -> MeleeWeaponSpec {
    MeleeWeaponSpec {
        damage: WeaponDamage::new(0),
        punch: WeaponPunch::new(0),
        shred: WeaponShred::new(0),
        damage_type: DamageType::Kinetic,
        fatal_bias: FatalBias::new(0.0),
        handedness: Handedness::OneHanded,
        reach: Reach::DEFAULT,
        fight_mode: FightMode::new(vec![structural_swing_mode()]),
        shove: Shove::new(false),
        slots: WeaponSlots::default(),
        attachments: Vec::new(),
    }
}

/// Default swing fight mode used by a new draft.
#[must_use]
pub(crate) const fn structural_swing_mode() -> FightModeSpec {
    FightModeSpec::new(FightModeKind::Swing, TuCost::new(0), Strikes::new(1))
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum AutoloadState {
    Pending,
    Done,
}

/// In-progress melee weapon being authored.
#[derive(Resource, Clone, PartialEq, Debug)]
pub struct MeleeWeaponDraft {
    name: String,
    spec: MeleeWeaponSpec,
    autoload: AutoloadState,
}

impl MeleeWeaponDraft {
    /// Empty draft ready for a new melee weapon.
    #[must_use]
    pub fn new_melee_weapon() -> Self {
        Self {
            name: String::new(),
            spec: seed_spec(),
            autoload: AutoloadState::Done,
        }
    }

    /// Whether the form should still try to autoload from the registry.
    #[must_use]
    pub const fn autoload_pending(&self) -> bool {
        matches!(self.autoload, AutoloadState::Pending)
    }

    /// Mark autoload complete.
    pub const fn mark_autoloaded(&mut self) {
        self.autoload = AutoloadState::Done;
    }

    /// Load an existing melee weapon into the draft.
    pub fn load_melee_weapon(&mut self, name: &WeaponName, spec: &MeleeWeaponSpec) {
        name.as_str().clone_into(&mut self.name);
        self.spec = spec.clone();
        self.autoload = AutoloadState::Done;
    }

    /// Display name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Set the display name.
    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }

    /// Full melee weapon spec.
    #[must_use]
    pub const fn spec(&self) -> &MeleeWeaponSpec {
        &self.spec
    }

    /// Mutable access to the melee weapon spec.
    pub const fn spec_mut(&mut self) -> &mut MeleeWeaponSpec {
        &mut self.spec
    }
}

impl Default for MeleeWeaponDraft {
    fn default() -> Self {
        Self {
            name: String::new(),
            spec: seed_spec(),
            autoload: AutoloadState::Pending,
        }
    }
}
