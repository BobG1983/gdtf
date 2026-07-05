//! The §5/§6 per-hit damage terms — [`FatalBias`], [`WeaponDamage`], [`WeaponPunch`],
//! [`WeaponShred`] — and the [`DamageType`] wheel vocabulary.

use bevy::prelude::{Component, Deref};
use serde::{Deserialize, Serialize};

/// A weapon's **fatal bias** — the `weapon.fatal_bias` term that stacks the
/// wound-severity score (resolution.md §6: `… + weapon.fatal_bias + …`), pushing
/// the table toward nastier buckets. **Carried here, consumed by E3** (severity,
/// resolution.md §6) — authored on the weapon but unused in this data slice.
///
/// A weapon NUMBER. Private inner + derived [`Deref`]; `#[serde(transparent)]`. A
/// `#[derive(Component)]` (GTW-200) — a sibling component on the armed entity.
/// `Default` (`FatalBias(0.0)`) is a **spawn-seed sentinel only** — the `bsn!`
/// spawn path seeds the slot via `Default` before `FatalBias::new(..)` overwrites
/// it (GTW-322). It is NOT a valid authored weapon stat.
#[derive(Component, Deref, Debug, Clone, Copy, PartialEq, Deserialize, Default)]
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
/// `Default` (`WeaponDamage(0)`) is a **spawn-seed sentinel only** — the `bsn!`
/// spawn path seeds the slot via `Default` before `WeaponDamage::new(..)`
/// overwrites it (GTW-322). It is NOT a valid authored weapon stat.
#[derive(Component, Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Default)]
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
/// `Default` (`WeaponPunch(0)`) is a **spawn-seed sentinel only** — the `bsn!`
/// spawn path seeds the slot via `Default` before `WeaponPunch::new(..)`
/// overwrites it (GTW-322). It is NOT a valid authored weapon stat.
#[derive(Component, Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Default)]
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
/// `Default` (`WeaponShred(0)`) is a **spawn-seed sentinel only** — the `bsn!`
/// spawn path seeds the slot via `Default` before `WeaponShred::new(..)`
/// overwrites it (GTW-322). It is NOT a valid authored weapon stat.
#[derive(Component, Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Default)]
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
/// `Default` (`DamageType::Kinetic`) is a **spawn-seed sentinel only** — the
/// `bsn!` spawn path seeds the slot via `Default` before the authored
/// `DamageType::<Variant>` patch overwrites it (GTW-322). `Kinetic` is chosen as
/// the most ordinary node; it carries no special meaning as the default.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize, Default)]
pub enum DamageType {
    /// Wheel node 0 — arc / EMP (mirror of [`crate::armor::ArmorType::Plated`]).
    Shock,
    /// Wheel node 1 — explosives / concussion (mirror of [`ArmorType::Refractive`](crate::armor::ArmorType::Refractive)).
    Blast,
    /// Wheel node 2 — toxin / acid / gas (mirror of [`ArmorType::Flak`](crate::armor::ArmorType::Flak)).
    Chem,
    /// Wheel node 3 — slugs / autoguns / shrapnel (mirror of [`ArmorType::Void`](crate::armor::ArmorType::Void)).
    #[default]
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
