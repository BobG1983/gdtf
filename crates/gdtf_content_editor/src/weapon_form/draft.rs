//! Ranged weapon form draft resource.

use std::num::NonZeroU8;

use bevy::prelude::*;
use gdtf_battle_sim::{
    effects::{
        fields::FieldKey,
        on_death::{ExplodeDamage, OnDeathEffect},
    },
    equipment::attachments::{FittedAttachments, WeaponSlots},
    magazine::Magazine,
    weapon::{
        Accuracy, AmmoType, BaseSpread, DamageType, DotTurns, FatalBias, FireMode, FireModeSpec,
        Handedness, HitType, Kickback, ModeConeMult, ModeKind, ModeShots, ModeTuPercent, Shove,
        Stable, TrajectoryStyle, WeaponDamage, WeaponName, WeaponPunch, WeaponShred, WeaponSpec,
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
        attachments: FittedAttachments::default(),
        dot:         None,
        on_death:    Vec::new(),
    }
}

/// Default single-shot fire mode used by a new draft.
#[must_use]
pub(crate) const fn structural_single_mode() -> FireModeSpec {
    FireModeSpec::new(
        ModeKind::Single,
        ModeConeMult::new(1.0),
        ModeTuPercent::new(0.0),
        ModeShots::new(1),
    )
}

/// Build [`DotTurns`] from a raw u8 (zero becomes 1).
#[must_use]
pub(crate) fn dot_turns_from_raw(raw: u8) -> DotTurns {
    DotTurns::new(NonZeroU8::new(raw).unwrap_or(NonZeroU8::MIN))
}

/// The blank explode effect an on-death Add button and its variant combo seed.
#[must_use]
pub(crate) const fn explode_template() -> OnDeathEffect {
    OnDeathEffect::Explode {
        hit_type:    HitType::Single,
        damage:      ExplodeDamage::new(0),
        damage_type: DamageType::Kinetic,
    }
}

/// The blank leave-field effect the on-death variant combo seeds.
#[must_use]
pub(crate) const fn leave_field_template() -> OnDeathEffect {
    OnDeathEffect::LeaveField {
        field: FieldKey::new(String::new()),
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum AutoloadState {
    Pending,
    Done,
}

/// In-progress ranged weapon being authored.
#[derive(Resource, Clone, PartialEq, Debug)]
pub struct WeaponDraft {
    name:     String,
    spec:     WeaponSpec,
    autoload: AutoloadState,
}

impl WeaponDraft {
    /// Empty draft ready for a new weapon.
    #[must_use]
    pub fn new_weapon() -> Self {
        Self {
            name:     String::new(),
            spec:     seed_spec(),
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

    /// Load an existing weapon into the draft.
    pub fn load_weapon(&mut self, name: &WeaponName, spec: &WeaponSpec) {
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

    /// Full weapon spec.
    #[must_use]
    pub const fn spec(&self) -> &WeaponSpec {
        &self.spec
    }

    /// Mutable access to the weapon spec.
    pub const fn spec_mut(&mut self) -> &mut WeaponSpec {
        &mut self.spec
    }

    /// Authored fire modes.
    #[must_use]
    pub fn fire_modes(&self) -> &[FireModeSpec] {
        &self.spec.fire_mode
    }

    /// Whether a fire mode may be removed. A weapon keeps at least one.
    #[must_use]
    pub fn can_remove_fire_mode(&self) -> bool {
        self.spec.fire_mode.len() > 1
    }

    /// Append the structural single-shot mode.
    pub fn add_fire_mode(&mut self) {
        let mut modes = self.spec.fire_mode.to_vec();
        modes.push(structural_single_mode());
        self.spec.fire_mode = FireMode::new(modes);
    }

    /// Remove a fire mode by index. Returns whether one was removed.
    pub fn remove_fire_mode(&mut self, index: usize) -> bool {
        if !self.can_remove_fire_mode() || index >= self.spec.fire_mode.len() {
            return false;
        }
        let mut modes = self.spec.fire_mode.to_vec();
        modes.remove(index);
        self.spec.fire_mode = FireMode::new(modes);
        true
    }

    /// Rewrite a fire mode by index. Returns whether one was written.
    pub fn set_fire_mode(&mut self, index: usize, mode: FireModeSpec) -> bool {
        let mut modes = self.spec.fire_mode.to_vec();
        match modes.get_mut(index) {
            Some(slot) => {
                *slot = mode;
                self.spec.fire_mode = FireMode::new(modes);
                true
            }
            None => false,
        }
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
