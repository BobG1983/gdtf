//! The **authoring spec** — the `WeaponSpec` an `assets/weapons/*.weapon.ron`
//! deserializes into, plus [`into_bundle`](WeaponSpec::into_bundle) which resolves
//! it into a spawnable [`WeaponBundle`] (GTW-257).

use bevy::reflect::TypePath;
use serde::Deserialize;

use super::{
    Accuracy, BaseSpread, DamageProfile, DamageType, FatalBias, FireMode, HandlingProfile,
    Kickback, MagazineSize, Stable, WeaponBundle, WeaponDamage, WeaponName, WeaponPunch,
    WeaponShred,
};

/// The **authoring struct** an `assets/weapons/*.ron` deserializes into — every
/// weapon NUMBER the §1/§6 math reads, MINUS the [`WeaponName`] (the name is the
/// FILE KEY, supplied by the loader from the file's stem) and MINUS the
/// [`Weapon`](super::Weapon) marker (that is added by [`WeaponBundle::new`]).
///
/// This is the data-driven, folder-loaded weapon model (the
/// [[weapons-armor-data-driven]] end-state, GTW-257): a per-weapon loose `.ron`
/// file is parsed into a `WeaponSpec`, keyed by its filename stem into the
/// [`WeaponRegistry`](super::WeaponRegistry), and resolved at battle setup into a
/// [`WeaponBundle`] via [`into_bundle`](WeaponSpec::into_bundle). It mirrors
/// [`WeaponBundle`]'s data exactly, dropping only the two fields the loader /
/// spawn-side own: the name (the file key) and the marker (the armed-entity tag).
///
/// Every field is an existing weapon-number newtype authored as its
/// `#[serde(transparent)]` bare RON scalar (the [`crate::tuning`] / GTW-200 house
/// style); the authored magnitudes are tuning DATA (commented in the `.ron`), NOT
/// pinned by tests (the brittle-test rule). Derives [`Deserialize`] so the loose
/// `.ron` parses, and [`TypePath`] because the [`RonAsset<WeaponSpec>`](gdtf_assets::RonAsset)
/// the loader wraps it in requires its payload to be [`TypePath`] (the same bound
/// [`Situation`](crate::situation::Situation) / [`CombatTuning`](crate::tuning::CombatTuning)
/// satisfy).
///
/// **Not `Copy`** — it owns a [`FireMode`] (which holds a `Vec` of specs); it is
/// `Clone`, so the registry can hold specs BY VALUE.
#[derive(Debug, Clone, PartialEq, Deserialize, TypePath)]
pub struct WeaponSpec {
    /// The intrinsic angular spread before situational multipliers (`base_spread`).
    pub base_spread:   BaseSpread,
    /// The concentration weapon term (`accuracy`; may exceed 1.0).
    pub accuracy:      Accuracy,
    /// The per-round recoil added in a burst (`kickback`).
    pub kickback:      Kickback,
    /// The severity-score addend, consumed by E3 (`fatal_bias`).
    pub fatal_bias:    FatalBias,
    /// The base damage a hit deals before armor (`damage`).
    pub damage:        WeaponDamage,
    /// The armor protection a hit ignores — penetration (`punch`).
    pub punch:         WeaponPunch,
    /// The extra integrity damage a hit deals to armor durability (`shred`).
    pub shred:         WeaponShred,
    /// The damage type the weapon emits — its matchup-wheel node.
    pub damage_type:   DamageType,
    /// The round capacity before a reload (`magazine_size`).
    pub magazine_size: MagazineSize,
    /// The authored fire-mode selector — the list of offered modes, each a
    /// [`FireModeSpec`](super::FireModeSpec) carrying its [`ModeKind`](super::ModeKind)
    /// + cone/TU%/shots.
    pub fire_mode:     FireMode,
    /// The `stable` tag — `true` engages the §1a brace bonus unconditionally.
    pub stable:        Stable,
}

impl WeaponSpec {
    /// Resolve this authored spec into a spawnable [`WeaponBundle`], supplying the
    /// [`WeaponName`] from the registry KEY (the weapon file's filename stem).
    ///
    /// Groups the per-hit damage fields into a [`DamageProfile`] and the
    /// magazine/fire-mode/`stable` fields into a [`HandlingProfile`], then calls
    /// [`WeaponBundle::new`] — the [`Weapon`](super::Weapon) marker is added there.
    /// Consumes the spec by value (it owns the [`FireMode`]); a caller holding a
    /// borrowed spec clones it first (the registry's specs are `Clone`).
    #[must_use]
    pub fn into_bundle(self, name: WeaponName) -> WeaponBundle {
        WeaponBundle::new(
            name,
            self.base_spread,
            self.accuracy,
            self.kickback,
            self.fatal_bias,
            DamageProfile::new(self.damage, self.punch, self.shred, self.damage_type),
            HandlingProfile::new(self.magazine_size, self.fire_mode, self.stable),
        )
    }
}
