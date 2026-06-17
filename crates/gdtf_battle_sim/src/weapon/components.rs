//! The per-stat weapon **components** — each weapon NUMBER as its own
//! `#[derive(Component)]` newtype (GTW-200), the [`DamageType`] vocabulary, the
//! [`WeaponName`], and the unit [`Weapon`] marker. Every numeric leaf is a named
//! newtype (no-bare-types): a private inner value, a derived [`Deref`], and
//! `#[serde(transparent)]` so it round-trips as a bare RON scalar.

use bevy::prelude::{Component, Deref};
use serde::{Deserialize, Serialize};

/// A weapon's **base spread** — the intrinsic angular dispersion before the
/// situational multipliers, the `base_spread` term of `θ_cone` (resolution.md
/// §1a). The widest the cone can throw from this weapon's mechanics alone, in the
/// sim's angular unit (radians; the cone math is angle-only, no pixel).
///
/// A weapon NUMBER (lives on the weapon, not in tuning). Private inner + derived
/// [`Deref`]; `#[serde(transparent)]` parses a bare RON scalar. A
/// `#[derive(Component)]` so it lives as a sibling component on the armed entity
/// (GTW-200).
#[derive(Component, Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct BaseSpread(f32);

impl BaseSpread {
    /// Build a base-spread value from its magnitude (radians).
    #[must_use]
    pub const fn new(radians: f32) -> Self {
        Self(radians)
    }
}

/// A weapon's **accuracy** — the weapon term of the concentration exponent
/// `p = concentration_p(Shooting, weapon.accuracy)` (resolution.md §1b): higher
/// accuracy clusters the in-cone draw toward dead-center. **Can exceed 1.0**
/// (resolution.md §1b: "The weapon term can exceed 1.0").
///
/// A weapon NUMBER (not a tuning coefficient). It sets how *likely* a shot stays
/// near center — independent of how *wide* the cone can throw (the two levers of
/// §1b). Private inner + derived [`Deref`]; `#[serde(transparent)]`. A
/// `#[derive(Component)]` (GTW-200) — a sibling component on the armed entity.
#[derive(Component, Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct Accuracy(f32);

impl Accuracy {
    /// Build an accuracy value from its magnitude (dimensionless; may exceed 1.0).
    #[must_use]
    pub const fn new(accuracy: f32) -> Self {
        Self(accuracy)
    }
}

/// A weapon's **kickback** — the per-round recoil it adds, the `kickback` term of
/// the recoil factor `recoil = 1 + prior_shots × kickback × recoil_growth` (resolution.md §1a):
/// each round in a burst widens the cone for the next. A sloppy weapon sprays on
/// auto; a tight one stays usable (resolution.md §1a "Scaling recoil").
///
/// A weapon NUMBER. Private inner + derived [`Deref`]; `#[serde(transparent)]`. A
/// `#[derive(Component)]` (GTW-200) — a sibling component on the armed entity.
#[derive(Component, Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct Kickback(f32);

impl Kickback {
    /// Build a kickback value from its per-round recoil magnitude.
    #[must_use]
    pub const fn new(kickback: f32) -> Self {
        Self(kickback)
    }
}

/// A weapon's **fatal bias** — the `weapon.fatal_bias` term that stacks the
/// wound-severity score (resolution.md §6: `… + weapon.fatal_bias + …`), pushing
/// the table toward nastier buckets. **Carried here, consumed by E3** (severity,
/// resolution.md §6) — authored on the weapon but unused in this data slice.
///
/// A weapon NUMBER. Private inner + derived [`Deref`]; `#[serde(transparent)]`. A
/// `#[derive(Component)]` (GTW-200) — a sibling component on the armed entity.
#[derive(Component, Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct FatalBias(f32);

impl FatalBias {
    /// Build a fatal-bias value from its magnitude (a severity-score addend).
    #[must_use]
    pub const fn new(fatal_bias: f32) -> Self {
        Self(fatal_bias)
    }
}

/// A weapon's **base damage** — the damage a hit deals before armor
/// (`weapons-and-armor.md` §"Weapon stats": "damage — base damage of a hit"). The
/// `damage` term of the per-hit formula step 2 (`dmg = max(floor, damage −
/// max(0, protection − effPen))`).
///
/// A weapon NUMBER. An `i32` to share the signed arithmetic of the per-hit
/// formula, which subtracts and clamps these against the `i32` armor stats
/// ([`crate::armor`]) — the same honest-signed reasoning the armor side uses.
/// Private inner + derived [`Deref`]; `#[serde(transparent)]` parses a bare RON
/// scalar. A magnitude is TBD tuning (no shipped weapons yet). A
/// `#[derive(Component)]` (GTW-200) — a sibling component on the armed entity.
#[derive(Component, Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(transparent)]
pub struct WeaponDamage(i32);

impl WeaponDamage {
    /// Build a base-damage value from its magnitude (TBD tuning).
    #[must_use]
    pub const fn new(damage: i32) -> Self {
        Self(damage)
    }
}

/// A weapon's **punch** — the armor protection a hit ignores, i.e. penetration
/// (`weapons-and-armor.md` §"Weapon stats": "punch — armor protection it ignores
/// (penetration)"). The `punch` term of the per-hit formula step 1
/// (`effPen = max(0, punch − hardness)`); the matchup wheel multiplies it (a later
/// E3 slice).
///
/// A weapon NUMBER. An `i32` for the signed `punch − hardness` subtraction against
/// the `i32` armor hardness ([`crate::armor::ArmorHardness`]). Private inner +
/// derived [`Deref`]; `#[serde(transparent)]`. A magnitude is TBD tuning. A
/// `#[derive(Component)]` (GTW-200) — a sibling component on the armed entity.
#[derive(Component, Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(transparent)]
pub struct WeaponPunch(i32);

impl WeaponPunch {
    /// Build a punch (penetration) value from its magnitude (TBD tuning).
    #[must_use]
    pub const fn new(punch: i32) -> Self {
        Self(punch)
    }
}

/// A weapon's **shred** — the extra **integrity** damage a hit deals on top of the
/// normal soak/penetration wear (`weapons-and-armor.md` §"Weapon stats": "shred —
/// extra integrity damage per hit … attacks armor durability"). The `shred` term
/// of the per-hit formula step 3 (`integrity −= min(protection, damage) + effPen +
/// shred`); the matchup wheel multiplies it (a later E3 slice).
///
/// A weapon NUMBER. An `i32` to share the signed integrity arithmetic of the armor
/// side ([`crate::armor::ArmorIntegrity`], which tracks below zero). Private inner,
/// a derived [`Deref`], and `#[serde(transparent)]`; a magnitude is TBD tuning. A
/// `#[derive(Component)]` (GTW-200) — a sibling component on the armed entity.
#[derive(Component, Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(transparent)]
pub struct WeaponShred(i32);

impl WeaponShred {
    /// Build a shred (extra-integrity-damage) value from its magnitude (TBD
    /// tuning).
    #[must_use]
    pub const fn new(shred: i32) -> Self {
        Self(shred)
    }
}

/// The **damage type** a weapon emits — one of the seven shared wheel nodes
/// (`docs/combat/matchup.md` §"The 7 types", Table 1).
///
/// This is the weapon/damage half of the **dual vocabulary**: each variant names
/// the same wheel node `#` as the mirror [`crate::armor::ArmorType`] variant (node
/// `i` of one mirrors node `i` of the other), so one tournament wheel drives both
/// sides (matchup.md §"The 7 types": "same wheel node `#`, two names"). The order
/// here pins the node index — variant `i` is wheel node `i`:
///
/// ```text
/// 0 Shock · 1 Blast · 2 Chem · 3 Kinetic · 4 Plasma · 5 Rend · 6 Las
/// ```
///
/// A named domain enum, not a bare `u8` (no-bare-types). The matchup lookup itself
/// — which node penetrates which — is a later E3 slice; this only fixes the
/// vocabulary and its node order. `Deserialize` so a weapon's authored RON names
/// its type by variant. A `#[derive(Component)]` (GTW-200) — a sibling component
/// on the armed entity.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub enum DamageType {
    /// Wheel node 0 — arc / EMP (mirror of [`crate::armor::ArmorType::Plated`]).
    Shock,
    /// Wheel node 1 — explosives / concussion (mirror of [`ArmorType::Refractive`](crate::armor::ArmorType::Refractive)).
    Blast,
    /// Wheel node 2 — toxin / acid / gas (mirror of [`ArmorType::Flak`](crate::armor::ArmorType::Flak)).
    Chem,
    /// Wheel node 3 — slugs / autoguns / shrapnel (mirror of [`ArmorType::Void`](crate::armor::ArmorType::Void)).
    Kinetic,
    /// Wheel node 4 — superheated (mirror of [`ArmorType::Hazard`](crate::armor::ArmorType::Hazard)).
    Plasma,
    /// Wheel node 5 — chain / power edges (mirror of [`ArmorType::Reinforced`](crate::armor::ArmorType::Reinforced)).
    Rend,
    /// Wheel node 6 — beams (mirror of [`ArmorType::Ceramic`](crate::armor::ArmorType::Ceramic)).
    Las,
}

impl DamageType {
    /// The seven damage types in **wheel-node order** (`docs/combat/matchup.md`
    /// Table 1) — index `i` is wheel node `i`, the mirror of
    /// [`crate::armor::ArmorType::ALL`]`[i]`. The exhaustive sweep the
    /// variant-count and dual-vocabulary-parity tests iterate.
    pub const ALL: [Self; 7] = [
        Self::Shock,
        Self::Blast,
        Self::Chem,
        Self::Kinetic,
        Self::Plasma,
        Self::Rend,
        Self::Las,
    ];
}

/// A weapon's **magazine size** — how many rounds it holds before a reload
/// (resolution.md §"What's tunable" names the `reload_tu` refill; the magazine's
/// capacity is a weapon number). The ammo clamp `fire()` honors (resolution.md
/// §"What's pure math vs sim": "ammo clamp") reads this — carried here, spent by
/// the firing act in a later slice.
///
/// A weapon NUMBER, a small non-negative count (`u16`). Private inner + derived
/// [`Deref`]; `#[serde(transparent)]`. A `#[derive(Component)]` (GTW-200) — a
/// sibling component on the armed entity (the capacity; the live ammo count is the
/// separate [`crate::magazine::Magazine`] battle-state component).
#[derive(Component, Deref, Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct MagazineSize(u16);

impl MagazineSize {
    /// Build a magazine size from its round count.
    #[must_use]
    pub const fn new(rounds: u16) -> Self {
        Self(rounds)
    }
}

/// A weapon's **`stable` tag** — whether the weapon is braced-by-design
/// (bipod-mounted / a heavy, inherently-steady piece). A `stable` weapon engages
/// the §1a brace / cover-stability bonus **unconditionally** — the brace
/// contribution applies regardless of the faced cell's cover height or the
/// ganger's stance (it bypasses the normal §1a brace min-height gate, so a stable
/// weapon is as steady as a properly-braced one even facing an empty/unsuitable
/// cell). This is the model the user corrected to: weapons carry **no** intrinsic
/// stability *points* — only this boolean tag.
///
/// A weapon NUMBER (a boolean flag, lives on the weapon, not in tuning). A named
/// newtype (no-bare-types: a `bool` carrying domain meaning is wrapped). Private
/// inner + derived [`Deref`]; `#[serde(transparent)]` parses a bare RON `true` /
/// `false`. A `#[derive(Component)]` (GTW-200) — a sibling component on the armed
/// entity.
#[derive(Component, Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(transparent)]
pub struct Stable(bool);

impl Stable {
    /// Build a `stable` tag from its boolean value (`true` = braced-by-design,
    /// unconditional brace).
    #[must_use]
    pub const fn new(stable: bool) -> Self {
        Self(stable)
    }
}

/// A weapon's **name** — its human-facing identity (e.g. an authored weapon's
/// display name). Carried on the armed entity for the weapon display / picker
/// (GTW-254) and the loader (GTW-257); the §1/§6 cone/severity math NEVER reads it,
/// so it is deliberately absent from the [`WeaponStats`](super::WeaponStats)
/// borrow-view.
///
/// A weapon-identity newtype over [`String`] (no-bare-types: a name is a domain
/// value). Private inner + derived [`Deref`]; `#[serde(transparent)]` parses a bare
/// RON string. A `#[derive(Component)]` (GTW-200) — its OWN sibling component on the
/// armed entity, NOT packed into the [`Weapon`] unit marker. (A fire mode has no
/// stored name — its label is [`ModeKind`](super::ModeKind)'s [`Display`](std::fmt::Display),
/// GTW-260.)
#[derive(Component, Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct WeaponName(String);

impl WeaponName {
    /// Build a weapon name from its display string.
    #[must_use]
    pub const fn new(name: String) -> Self {
        Self(name)
    }
}

/// The **`Weapon` MARKER** — a unit `#[derive(Component)]` tag (no data) marking an
/// entity as armed (GTW-200's user-corrected model).
///
/// The weapon's stats are **not** packed inside this — each is its own sibling
/// `#[derive(Component)]` newtype on the same entity (`BaseSpread`, `Accuracy`,
/// `Kickback`, `FatalBias`, `WeaponDamage`, `WeaponPunch`, `WeaponShred`,
/// `DamageType`, `MagazineSize`, [`FireMode`](super::FireMode), `Stable`), spawned
/// together via [`WeaponBundle`](super::WeaponBundle). Because the stats are direct
/// components, the combat act (E4.5 `fire()`) is a proper query-based Bevy system
/// with NO `&mut World` indirection: it queries the individual stat components off
/// the ganger entity (or assembles a transient [`WeaponStats`](super::WeaponStats)
/// borrow-view from them). There is **no** `Weapon::new` and no packed data struct
/// any more — the stats live as components.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Weapon;
