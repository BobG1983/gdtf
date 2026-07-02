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
/// `Default` (`BaseSpread(0.0)`) is a **spawn-seed sentinel only** — the
/// `bsn!`-scene spawn path's `get_or_insert_template` seeds the component slot
/// via `Default` before the authored `BaseSpread::new(..)` overwrites it
/// (GTW-322). It is NOT a valid authored weapon stat.
#[derive(Component, Deref, Debug, Clone, Copy, PartialEq, Deserialize, Default)]
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
/// `Default` (`Accuracy(0.0)`) is a **spawn-seed sentinel only** — the `bsn!`
/// spawn path seeds the slot via `Default` before `Accuracy::new(..)` overwrites
/// it (GTW-322). It is NOT a valid authored weapon stat.
#[derive(Component, Deref, Debug, Clone, Copy, PartialEq, Deserialize, Default)]
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
/// `Default` (`Kickback(0.0)`) is a **spawn-seed sentinel only** — the `bsn!`
/// spawn path seeds the slot via `Default` before `Kickback::new(..)` overwrites
/// it (GTW-322). It is NOT a valid authored weapon stat.
#[derive(Component, Deref, Debug, Clone, Copy, PartialEq, Deserialize, Default)]
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

/// A weapon's **magazine size** — how many rounds it holds before a reload
/// (resolution.md §"What's tunable" names the `reload_tu` refill; the magazine's
/// capacity is a weapon number). The ammo clamp `fire()` honors (resolution.md
/// §"What's pure math vs sim": "ammo clamp") reads this.
///
/// A weapon NUMBER, a small non-negative count (`u16`). Private inner + derived
/// [`Deref`]; `#[serde(transparent)]`. Since GTW-275 it is **not** a standalone
/// `#[derive(Component)]` — it is the `size` LEAF of the [`crate::magazine::Magazine`]
/// grouping component (the user's `Magazine { size, reload_tu, … }` model), which
/// also carries the per-weapon [`ReloadTu`](crate::magazine::ReloadTu) and the live
/// [`LoadedRounds`](crate::magazine::LoadedRounds) battle-state count.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Deserialize)]
#[serde(transparent)]
pub struct MagazineSize(u16);

impl MagazineSize {
    /// Build a magazine size from its round count.
    #[must_use]
    pub const fn new(rounds: u16) -> Self {
        Self(rounds)
    }

    /// The capacity as its inner round count — a `const` accessor (the derived
    /// [`Deref`] is not `const`, so `const fn` callers like
    /// [`Magazine::loaded`](crate::magazine::Magazine::loaded) read the capacity
    /// through this).
    #[must_use]
    pub const fn get(self) -> u16 {
        self.0
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
/// `Default` (`Stable(false)`) is a **spawn-seed sentinel only** — the `bsn!`
/// spawn path seeds the slot via `Default` before `Stable::new(..)` overwrites it
/// (GTW-322). It is NOT a valid authored weapon tag (though `false` happens to
/// coincide with "not braced-by-design", it is overwritten regardless).
#[derive(Component, Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Default)]
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

/// A weapon's **`shove` tag** — whether the weapon KNOCKS BACK the target on a
/// connecting attack (GTW-525). A `shove` weapon (a shock maul, a heavy bolter's
/// muzzle-thump, a boarding shield) AUTO-shoves its target one cell directly away
/// from the attacker on EVERY connecting hit — a MELEE strike connect OR a RANGED
/// shot connect — in ADDITION to the attack's damage. A miss does not shove; a
/// non-`shove` weapon never shoves.
///
/// The shove is PURE DISPLACEMENT: the tag adds no wound of its own (the attack's
/// own damage stands; any FALL the displacement triggers does the extra harm,
/// through the shared GTW-523 fall path). It mirrors the [`Stable`] tag exactly — a
/// data-driven boolean MARKER set in the weapon `.ron`, present on only SOME
/// weapons; unlike [`Stable`] it lives on BOTH the ranged AND the melee weapon
/// model (any weapon can knock back).
///
/// A weapon NUMBER (a boolean flag, lives on the weapon, not in tuning). A named
/// newtype (no-bare-types: a `bool` carrying domain meaning is wrapped). Private
/// inner + derived [`Deref`]; `#[serde(transparent)]` parses a bare RON `true` /
/// `false`. A `#[derive(Component)]` (GTW-200) — a sibling component on the armed
/// entity, the [`Stable`] precedent.
/// `Default` (`Shove(false)`) is BOTH the spawn-seed sentinel (the `bsn!` spawn path
/// seeds the slot via `Default` before an authored `Shove::new(..)` overwrites it,
/// GTW-322) AND the defensible authored default: a weapon authored WITHOUT a `shove:`
/// field is a NON-shove weapon (the field is `#[serde(default)]` on the spec, so the
/// vast majority of existing weapons that never author it keep shoving off). Unlike
/// [`Stable`] — a required RON field — `shove` is opt-in.
#[derive(Component, Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Default)]
#[serde(transparent)]
pub struct Shove(bool);

impl Shove {
    /// Build a `shove` tag from its boolean value (`true` = knocks the target back
    /// one cell on a connecting hit).
    #[must_use]
    pub const fn new(shove: bool) -> Self {
        Self(shove)
    }
}

/// A weapon's **`scoped` tag** — whether the weapon carries a precision optic
/// (a scope / red-dot / target-designator) fitted as a GTW-542 attachment. A
/// `scoped` weapon adds a positive [`SightStability`](crate::stability::SightStability)
/// contribution to the §1a stability score (a steadier aim → a tighter dispersion
/// cone), the sight-attachment's whole effect. It is a **weapon-side** tag exactly
/// like [`Stable`], but where `stable` engages the BINARY brace, `scoped` feeds an
/// ADDITIVE stability seam (the [`SuppressionStability`](crate::stability::SuppressionStability)
/// mirror), so a scoped weapon is steadier without pretending to be braced.
///
/// (Named `Scoped`, not `Scoped`, to avoid the pre-existing LOS-visibility
/// `Scoped` newtype — a DIFFERENT concept.)
///
/// A named newtype (no-bare-types: a `bool` carrying domain meaning is wrapped).
/// Private inner + derived [`Deref`]; `#[serde(transparent)]` parses a bare RON
/// `true` / `false` (though the tag is normally SPAWNED — not authored — by the
/// [`Scoped`] attachment folder-fn). A `#[derive(Component)]` (GTW-200) — a sibling
/// component on the armed entity, present ONLY when a sight attachment is fitted; the
/// firing path reads it as an `Option<&Scoped>` (absent = the zero-identity
/// [`SightStability::none`](crate::stability::SightStability::none), byte-identical to
/// an un-scoped weapon).
/// `Default` (`Scoped(true)`) exists solely so the `bsn!` spawn-seed path can seed
/// the slot before the folder-fn's `Scoped::new(true)` overwrites it (GTW-322) — the
/// component is only ever inserted when a sight is fitted, so `true` is the only
/// meaningful value.
#[derive(Component, Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(transparent)]
pub struct Scoped(bool);

impl Scoped {
    /// Build a `scoped` tag from its boolean value (`true` = a precision optic is
    /// fitted, engaging the additive sight-stability bonus).
    #[must_use]
    pub const fn new(scoped: bool) -> Self {
        Self(scoped)
    }
}

impl Default for Scoped {
    /// The spawn-seed sentinel (GTW-322): `Scoped(true)`. The component is inserted
    /// only when a sight attachment is fitted, so `true` — a fitted sight — is the
    /// only meaningful value; the folder-fn overwrites this seed regardless.
    fn default() -> Self {
        Self(true)
    }
}

/// A weapon's **per-weapon sight-stability override** — a §1a stability-score point
/// bonus carried BY the weapon (GTW-542), for attachments whose steadying magnitude is
/// authored per-fitting rather than by the universal tuning leaf.
///
/// The [`Scoped`] tag alone reads the universal
/// [`SightStabilityBonus`](crate::tuning::SightStabilityBonus) tuning leaf (the NAMED
/// sight attachment). But two GRIMDARK attachments steady by a DIFFERENT, per-fitting
/// magnitude — the whisper-bore's modest bonus and the dead-man's-brace's authored brace
/// bonus — so they carry this component ALONGSIDE [`Scoped`] to OVERRIDE the tuning leaf
/// with their own points. The §1a composer ([`stability_for`](crate::aim::stability_for))
/// reads `Option<&WeaponSightBonus>`: present → use its points; absent (a plain
/// [`Scoped`] weapon) → fall back to the tuning leaf.
///
/// A weapon NUMBER (stability-score points, the [`SightStabilityBonus`](crate::tuning::SightStabilityBonus)
/// unit). A named newtype: private inner + derived [`Deref`]; `#[serde(transparent)]`. A
/// `#[derive(Component)]` (GTW-200) — a sibling on the armed entity, present ONLY when a
/// per-fitting-bonus attachment is fitted.
/// `Default` (`WeaponSightBonus(0.0)`) is the `bsn!` spawn-seed sentinel (GTW-322),
/// overwritten by the folder-fn's authored value.
#[derive(Component, Deref, Debug, Clone, Copy, PartialEq, Deserialize, Default)]
#[serde(transparent)]
pub struct WeaponSightBonus(f32);

impl WeaponSightBonus {
    /// Build a per-weapon sight-stability bonus from its stability-score point magnitude.
    #[must_use]
    pub const fn new(points: f32) -> Self {
        Self(points)
    }
}

/// A weapon's **`silenced` tag** — whether the weapon carries a suppressor / silencer
/// fitted as a GTW-542 attachment. A `silenced` weapon fires QUIETLY: its shots do
/// NOT propagate the two "loud" battle signals a normal shot does —
///
/// - **suppression** — a silenced shot never PINS opposing gangers (the GTW-526
///   [`apply_suppression`](crate::suppression::apply_suppression) producer skips a silenced
///   shooter's [`FireRequested`](crate::acts::FireRequested)); and
/// - **reaction/reveal** — a silenced shot never TRIPS an opposing reactor's
///   interrupt (the GTW-468 [`reaction_trigger`](crate::reaction::reaction_trigger)
///   producer skips a [`FireDeclaration`](crate::acts::FireDeclaration) whose shooter
///   wields a silenced weapon, including a silenced INTERRUPT shot — a silenced shot
///   stays silent even when it is itself a reaction).
///
/// The shot still resolves its damage normally; only its NOISE footprint is removed.
///
/// A named newtype (no-bare-types: a `bool` carrying domain meaning is wrapped).
/// Private inner + derived [`Deref`]; `#[serde(transparent)]`. A `#[derive(Component)]`
/// (GTW-200) — a sibling component on the armed entity, present ONLY when a suppressor
/// is fitted; both producers gate on its presence via `shooter → Wields → weapon →
/// Option<&Silenced>` (absent = a normal, LOUD shot, byte-identical to before this
/// tag).
/// `Default` (`Silenced(true)`) is the `bsn!` spawn-seed sentinel (GTW-322); the
/// component is inserted only when a suppressor is fitted, so `true` is the only
/// meaningful value, overwritten by the folder-fn's `Silenced::new(true)` regardless.
#[derive(Component, Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(transparent)]
pub struct Silenced(bool);

impl Silenced {
    /// Build a `silenced` tag from its boolean value (`true` = a suppressor is fitted,
    /// so the shot propagates neither suppression nor reaction/reveal).
    #[must_use]
    pub const fn new(silenced: bool) -> Self {
        Self(silenced)
    }
}

impl Default for Silenced {
    /// The spawn-seed sentinel (GTW-322): `Silenced(true)`. The component is inserted
    /// only when a suppressor attachment is fitted, so `true` is the only meaningful
    /// value; the folder-fn overwrites this seed regardless.
    fn default() -> Self {
        Self(true)
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
/// `Default` (`WeaponName(String::new())`, the empty string) is a **spawn-seed
/// sentinel only** — the `bsn!` spawn path seeds the slot via `Default` before
/// `WeaponName::new(..)` overwrites it (GTW-322). It is NOT a valid authored
/// weapon name.
#[derive(Component, Deref, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct WeaponName(String);

impl WeaponName {
    /// Build a weapon name from its display string.
    #[must_use]
    pub const fn new(name: String) -> Self {
        Self(name)
    }
}

/// A weapon's **handedness** — how many hands it takes to fire (GTW-443). A
/// [`OneHanded`](Handedness::OneHanded) weapon (a pistol) can be fired with a single
/// working hand; a [`TwoHanded`](Handedness::TwoHanded) weapon (a long-arm or heavy
/// piece) needs BOTH hands and is refused once a hand-disabling injury leaves the
/// shooter with fewer than two (see [`HandsAvailable`](crate::injuries::HandsAvailable)
/// and the shared `can_fire` guard).
///
/// A named domain enum (no-bare-types-exempt: an enum carries its meaning in its
/// variants, not a wrapped primitive). It lives as its OWN sibling
/// `#[derive(Component)]` newtype on the armed entity (GTW-200), parsed from the
/// `handedness:` field of a weapon's `.weapon.ron`. `Copy` + `Hash` + `Eq` so it can be
/// a value field of the [`FireActor`](crate::magazine::FireActor) read-bundle.
/// `Default` ([`OneHanded`](Handedness::OneHanded)) is a **spawn-seed sentinel only** —
/// the `bsn!` spawn path seeds the slot via `Default` before the authored
/// `Handedness::<Variant>` patch overwrites it (GTW-322). It carries no special meaning
/// as the default; `OneHanded` is chosen as the least-restrictive node so an
/// un-authored weapon never spuriously gates on hand count.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Default)]
pub enum Handedness {
    /// A one-handed weapon — fires with a single working hand (a pistol).
    #[default]
    OneHanded,
    /// A two-handed weapon — needs BOTH hands; refused at fewer than two available
    /// hands (a long-arm or heavy piece).
    TwoHanded,
}

/// The **`Weapon` MARKER** — a unit `#[derive(Component)]` tag (no data) marking an
/// entity as armed (GTW-200's user-corrected model).
///
/// The weapon's stats are **not** packed inside this — each is its own sibling
/// `#[derive(Component)]` newtype on the same entity (`BaseSpread`, `Accuracy`,
/// `Kickback`, `FatalBias`, `WeaponDamage`, `WeaponPunch`, `WeaponShred`,
/// `DamageType`, the [`crate::magazine::Magazine`] grouping (its `MagazineSize` /
/// `ReloadTu` / `LoadedRounds` leaves), [`FireMode`](super::FireMode), `Stable`),
/// spawned together via [`WeaponBundle`](super::WeaponBundle). Because the stats are direct
/// components, the combat act (E4.5 `fire()`) is a proper query-based Bevy system
/// with NO `&mut World` indirection: it queries the individual stat components off
/// the ganger entity (or assembles a transient [`WeaponStats`](super::WeaponStats)
/// borrow-view from them). There is **no** `Weapon::new` and no packed data struct
/// any more — the stats live as components.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Weapon;

/// The **`MountedWeapon` MARKER** — a unit `#[derive(Component)]` tag (no data) marking a
/// RANGED weapon entity as the bolted-down gun of a **weapon emplacement** the occupant is
/// currently manning (GTW-543, child GTW-41c of the emplacements epic GTW-41).
///
/// A mounted weapon is a NORMAL ranged weapon — it carries the ranged [`Weapon`] marker + the
/// full stat set + a [`Magazine`](crate::magazine::Magazine), spawned via
/// [`WeaponBundle`](super::WeaponBundle) — that ALSO carries THIS marker. It is spawned onto
/// the OCCUPANT (related via [`WieldedBy`](super::WieldedBy)) when a ganger ENTERS an
/// emplacement, and despawned when it EXITS, so it exists only for the duration of occupancy.
///
/// While present it is the shooter's PREFERRED ranged weapon: the fire path resolves
/// `ganger → Wields → the weapon entity` through
/// [`Wields::mounted_weapon`](super::Wields::mounted_weapon) FIRST (a mounted-marked entity),
/// falling back to [`Wields::ranged_weapon`](super::Wields::ranged_weapon) (the ganger's own
/// carried gun) when no mount is present — so a manning ganger fires the heavy mounted gun
/// instead of its side-arm, and reverts to its own weapon on exit. It is the emplacement
/// analogue of the [`MeleeWeapon`](super::MeleeWeapon) marker: a distinguishing tag on a
/// wielded weapon entity that a keyed [`Wields`](super::Wields) accessor selects.
///
/// `Default` lets a spawn path derive it; the marker carries no data.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MountedWeapon;
