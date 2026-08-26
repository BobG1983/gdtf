//! Field form draft resource.

use bevy::prelude::*;
use gdtf_battle_sim::{
    armor::ArmorType,
    effects::fields::{FieldDamage, FieldDef, FieldDuration, FieldKey, ImmuneArmorTypes},
    weapon::DamageType,
};

fn seed_def() -> FieldDef {
    FieldDef::new(
        FieldDamage::default(),
        DamageType::default(),
        ImmuneArmorTypes::default(),
        FieldDuration::Permanent,
    )
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum AutoloadState {
    Pending,
    Done,
}

/// In-progress field definition being authored.
#[derive(Resource, Clone, PartialEq, Eq, Debug)]
pub struct FieldDraft {
    key:      String,
    def:      FieldDef,
    autoload: AutoloadState,
}

impl FieldDraft {
    /// Range the damage input offers.
    pub const DAMAGE_RANGE: core::ops::RangeInclusive<u16> = 0..=1000;
    /// Fewest turns a finite duration may name; `Turns(0)` fails the loader.
    pub const MIN_TURNS: u8 = 1;

    /// Empty draft ready for a new field.
    #[must_use]
    pub fn new_field() -> Self {
        Self {
            key:      String::new(),
            def:      seed_def(),
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

    /// Load an existing field into the draft.
    pub fn load_field(&mut self, key: &FieldKey, def: &FieldDef) {
        key.as_str().clone_into(&mut self.key);
        self.def = def.clone();
        self.autoload = AutoloadState::Done;
    }

    /// Save stem this field writes under.
    #[must_use]
    pub fn key(&self) -> &str {
        &self.key
    }

    /// Set the save stem.
    pub fn set_key(&mut self, key: String) {
        self.key = key;
    }

    /// The definition the save would write.
    #[must_use]
    pub const fn def(&self) -> &FieldDef {
        &self.def
    }

    /// Damage drained per tick.
    #[must_use]
    pub const fn damage(&self) -> FieldDamage {
        self.def.damage
    }

    /// Set the damage drained per tick.
    pub const fn set_damage(&mut self, damage: FieldDamage) {
        self.def.damage = damage;
    }

    /// Damage channel, which is presentation flavour only.
    #[must_use]
    pub const fn damage_type(&self) -> DamageType {
        self.def.damage_type
    }

    /// Set the damage channel.
    pub const fn set_damage_type(&mut self, damage_type: DamageType) {
        self.def.damage_type = damage_type;
    }

    /// How long a placement of this field lasts.
    #[must_use]
    pub const fn duration(&self) -> FieldDuration {
        self.def.duration
    }

    /// Set how long a placement lasts.
    pub const fn set_duration(&mut self, duration: FieldDuration) {
        self.def.duration = duration;
    }

    /// The armor types this field never drains through, in [`ArmorType::ALL`] order.
    #[must_use]
    pub fn immune_armor_types(&self) -> Vec<ArmorType> {
        self.def.immune_armor_types.iter().collect()
    }

    /// Whether a type is on the immune list.
    #[must_use]
    pub fn is_immune(&self, armor_type: ArmorType) -> bool {
        self.def.immune_armor_types.contains(&armor_type)
    }

    /// Replace the immune list wholesale.
    pub fn set_immune_armor_types(&mut self, types: impl IntoIterator<Item = ArmorType>) {
        self.def.immune_armor_types = ImmuneArmorTypes::new(types);
    }

    /// Add a type if absent, remove it if present, the way the form's tick box does.
    pub fn toggle_immune_armor_type(&mut self, armor_type: ArmorType) {
        let held = self.is_immune(armor_type);
        let kept = self
            .immune_armor_types()
            .into_iter()
            .filter(|listed| *listed != armor_type)
            .chain(if held { None } else { Some(armor_type) });
        self.set_immune_armor_types(kept);
    }
}

impl Default for FieldDraft {
    fn default() -> Self {
        Self {
            key:      String::new(),
            def:      seed_def(),
            autoload: AutoloadState::Pending,
        }
    }
}
