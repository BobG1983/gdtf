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
        damage:      WeaponDamage::new(0),
        punch:       WeaponPunch::new(0),
        shred:       WeaponShred::new(0),
        damage_type: DamageType::Kinetic,
        fatal_bias:  FatalBias::new(0.0),
        handedness:  Handedness::OneHanded,
        reach:       Reach::DEFAULT,
        fight_mode:  FightMode::new(vec![structural_swing_mode()]),
        shove:       Shove::new(false),
        slots:       WeaponSlots::default(),
        attachments: Vec::new(),
    }
}

#[must_use]
pub(crate) const fn structural_swing_mode() -> FightModeSpec {
    FightModeSpec::new(FightModeKind::Swing, TuCost::new(0), Strikes::new(1))
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum AutoloadState {
        Pending,
        Done,
}

#[derive(Resource, Clone, PartialEq, Debug)]
pub struct MeleeWeaponDraft {
        name:     String,
        spec:     MeleeWeaponSpec,
        autoload: AutoloadState,
}

impl MeleeWeaponDraft {
                    #[must_use]
    pub fn new_melee_weapon() -> Self {
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

                    pub fn load_melee_weapon(&mut self, name: &WeaponName, spec: &MeleeWeaponSpec) {
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
    pub const fn spec(&self) -> &MeleeWeaponSpec {
        &self.spec
    }

                pub const fn spec_mut(&mut self) -> &mut MeleeWeaponSpec {
        &mut self.spec
    }
}

impl Default for MeleeWeaponDraft {
                        fn default() -> Self {
        Self {
            name:     String::new(),
            spec:     seed_spec(),
            autoload: AutoloadState::Pending,
        }
    }
}
