//! every authored field round-trips by construction). Field edits flow through
use std::num::NonZeroU8;

use bevy::prelude::*;
use gdtf_battle_sim::{
    equipment::attachments::WeaponSlots,
    magazine::Magazine,
    weapon::{
        Accuracy, AmmoType, BaseSpread, DamageType, DotTurns, FatalBias, FireMode, FireModeSpec,
        Handedness, Kickback, ModeConeMult, ModeKind, ModeShots, ModeTuPercent, Shove, Stable,
        TrajectoryStyle, WeaponDamage, WeaponName, WeaponPunch, WeaponShred, WeaponSpec,
    },
};

fn seed_spec() -> WeaponSpec {
    WeaponSpec {
        base_spread: BaseSpread::new(0.0),
        accuracy:    Accuracy::new(0.0),
        kickback:    Kickback::new(0.0),
        fatal_bias:  FatalBias::new(0.0),
        damage:      WeaponDamage::new(0),
        punch:       WeaponPunch::new(0),
        shred:       WeaponShred::new(0),
        damage_type: DamageType::Kinetic,
        accepts:     AmmoType::Slug,
        magazine:    Magazine::default(),
        fire_mode:   FireMode::new(vec![structural_single_mode()]),
        stable:      Stable::new(false),
        shove:       Shove::new(false),
        handedness:  Handedness::OneHanded,
        trajectory:  TrajectoryStyle::Straight,
        slots:       WeaponSlots::default(),
        attachments: Vec::new(),
        dot:         None,
        on_death:    None,
    }
}

#[must_use]
pub(crate) const fn structural_single_mode() -> FireModeSpec {
    FireModeSpec::new(
        ModeKind::Single,
        ModeConeMult::new(1.0),
        ModeTuPercent::new(0.0),
        ModeShots::new(1),
    )
}

#[must_use]
pub(crate) fn dot_turns_from_raw(raw: u8) -> DotTurns {
    DotTurns::new(NonZeroU8::new(raw).unwrap_or(NonZeroU8::MIN))
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum AutoloadState {
        Pending,
        Done,
}

#[derive(Resource, Clone, PartialEq, Debug)]
pub struct WeaponDraft {
        name:     String,
        spec:     WeaponSpec,
        autoload: AutoloadState,
}

impl WeaponDraft {
                    #[must_use]
    pub fn new_weapon() -> Self {
        Self {
            name:     String::new(),
            spec:     seed_spec(),
            autoload: AutoloadState::Done,
        }
    }

                #[must_use]
    pub const fn autoload_pending(&self) -> bool {
        matches!(self.autoload, AutoloadState::Pending)
    }

                pub const fn mark_autoloaded(&mut self) {
        self.autoload = AutoloadState::Done;
    }

                    pub fn load_weapon(&mut self, name: &WeaponName, spec: &WeaponSpec) {
        name.as_str().clone_into(&mut self.name);
        self.spec = spec.clone();
        self.autoload = AutoloadState::Done;
    }

        #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

        pub fn set_name(&mut self, name: String) {
        self.name = name;
    }

            #[must_use]
    pub const fn spec(&self) -> &WeaponSpec {
        &self.spec
    }

                pub const fn spec_mut(&mut self) -> &mut WeaponSpec {
        &mut self.spec
    }
}

impl Default for WeaponDraft {
                    fn default() -> Self {
        Self {
            name:     String::new(),
            spec:     seed_spec(),
            autoload: AutoloadState::Pending,
        }
    }
}
