//! Weapon-side data — the per-weapon and per-fire-mode NUMBERS the §1
//! cone/stability/recoil/aim math reads (`docs/combat/resolution.md` §1(a)+(b),
//! §"Coefficients live in the combat-tuning data": "Weapon-side numbers
//! (`base_spread`, `accuracy`, `kickback`, `fatal_bias`, `magazine_size`; per-mode
//! cone mult / TU% / shots) live on the weapon / fire-mode data").
//!
//! This is the home conflict's other side: *coefficients* live in [`crate::tuning`],
//! but a *weapon's own numbers* live here on [`Weapon`] / [`FireMode`]. The cone
//! equation `θ_cone = base_spread × stability × aim × firemode × recoil`
//! (resolution.md §1a) reads a weapon's `base_spread` and per-round `kickback`
//! from [`Weapon`] and its selector term from a [`FireMode`]; the in-cone draw
//! `p = concentration_p(Shooting, weapon.accuracy)` (§1b) reads `accuracy`; and
//! the §6 severity score reads `fatal_bias` (carried here, consumed by E3 — not
//! used in this slice).
//!
//! This is the E2.1 **data/types/serde** slice: types + serde only, no math or
//! behavior. Every numeric leaf is a named newtype (no-bare-types: a private inner
//! value, a derived [`Deref`], and `#[serde(transparent)]` so it round-trips as a
//! bare RON scalar), matching the [`crate::tuning`] house style.
//!
//! ## E3.1 — damage stats + the damage-type vocabulary
//!
//! The E3.1 slice adds the three per-hit damage NUMBERS the
//! `docs/combat/weapons-and-armor.md` §"Weapon stats" / §"Per-hit resolution"
//! formula reads — [`WeaponDamage`], [`WeaponPunch`], [`WeaponShred`] — plus the
//! [`DamageType`] a weapon emits. [`DamageType`] is one half of the dual
//! vocabulary of `docs/combat/matchup.md` §"The 7 types": its seven variants name
//! the same seven wheel nodes as [`crate::armor::ArmorType`], node `i` of one
//! mirroring node `i` of the other. This is the DATA substrate only — the per-hit
//! formula and the matchup lookup (the wheel itself) are later E3 slices; nothing
//! here computes a hit or a matchup.
//!
//! ## GTW-200 — a weapon is ECS components, not a data struct
//!
//! The user-corrected model: a weapon is **not** one packed data struct — each
//! sub-value is its own `#[derive(Component)]` newtype that lives as a sibling
//! component on the (ganger) entity, and [`Weapon`] is a unit MARKER component
//! (no data). The stats are then queryable directly, so the combat act
//! (E4.5 `fire()`) is a proper query-based Bevy system with NO `&mut World`
//! indirection. [`WeaponBundle`] spawns an armed entity carrying the full set;
//! the §1/§6 readers take a transient [`WeaponStats`] borrow-view (refs assembled
//! from the components at the call site — NOT a stored component).

use bevy::{
    platform::collections::HashMap,
    prelude::{Bundle, Component, Deref, Resource},
    reflect::TypePath,
};
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
/// so it is deliberately absent from the [`WeaponStats`] borrow-view.
///
/// A weapon-identity newtype over [`String`] (no-bare-types: a name is a domain
/// value). Private inner + derived [`Deref`]; `#[serde(transparent)]` parses a bare
/// RON string. A `#[derive(Component)]` (GTW-200) — its OWN sibling component on the
/// armed entity, NOT packed into the [`Weapon`] unit marker. (A fire mode has no
/// stored name — its label is [`ModeKind`]'s [`Display`], GTW-260.)
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

/// A per-mode **cone multiplier** — the selector term of `θ_cone` (the `firemode`
/// factor, resolution.md §1a): single ≈ 1, full-auto ≥ 1 (inherently sloppier).
/// A multiplier on the cone's angular size for that fire mode.
///
/// A weapon NUMBER (per-mode, on the [`FireMode`]). Private inner + derived
/// [`Deref`]; `#[serde(transparent)]`.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ModeConeMult(f32);

impl ModeConeMult {
    /// Build a per-mode cone multiplier from its magnitude.
    #[must_use]
    pub const fn new(mult: f32) -> Self {
        Self(mult)
    }
}

/// A per-mode **TU percentage** — the fraction of the shooter's TU pool a shot in
/// this mode costs (resolution.md §1: per-`FireMode` TU%). A dimensionless
/// fraction of the per-shot TU charge (the aim-mode ×1.5 premium, a tuning
/// coefficient, stacks on top — resolution.md §1a).
///
/// A weapon NUMBER (per-mode). Private inner + derived [`Deref`];
/// `#[serde(transparent)]`.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ModeTuPercent(f32);

impl ModeTuPercent {
    /// Build a per-mode TU percentage from its magnitude (a fraction of the TU
    /// pool).
    #[must_use]
    pub const fn new(percent: f32) -> Self {
        Self(percent)
    }
}

/// A per-mode **shot count** — how many rounds the mode fires per shot action
/// (resolution.md §1: per-`FireMode` shots; single = 1, a burst > 1, full-auto
/// more), the burst loop `fire()` runs (resolution.md §"What's pure math vs sim").
/// Each successive round adds the weapon's [`Kickback`] to the recoil factor.
///
/// A weapon NUMBER (per-mode), a small non-negative count (`u16`). Private inner
/// + derived [`Deref`]; `#[serde(transparent)]`.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ModeShots(u16);

impl ModeShots {
    /// Build a per-mode shot count from its round-per-action number.
    #[must_use]
    pub const fn new(shots: u16) -> Self {
        Self(shots)
    }
}

/// A fire mode's **kind** — the closed set of mechanics a mode can be: a single
/// shot, a burst, or full-auto (resolution.md §1: the selector offers "single /
/// burst / full-auto" modes). This is the model the user corrected to: a mode is
/// identified by a **closed enum** (`Single` / `Burst` / `Full`), not by a stored
/// human-facing name string — the label is derived from this kind via [`Display`].
///
/// A named domain enum (no-bare-types: a fire-mode kind is a domain value, not a
/// bare `u8`/label string). It is a FIELD value on a [`FireModeSpec`], NOT a
/// `#[derive(Component)]` — the [`FireMode`] selector that holds the specs is the
/// component. `Deserialize` so a weapon's authored RON names its mode kind by
/// variant; `Serialize` for the RON round-trip; `Copy`/`Eq`/`Hash` so a
/// [`FireModeSpec`] is `Copy` and the cycle compares mode identity by kind.
///
/// [`Display`] yields the canonical human labels (`"single"` / `"burst"` /
/// `"full-auto"`) — the picker (GTW-254) shows these, derived from the kind rather
/// than stored as a per-mode `String`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ModeKind {
    /// A single aimed/snap shot — one round per shot action (the baseline mode).
    Single,
    /// A short burst — a few rounds per shot action, wider than single.
    Burst,
    /// Full-auto — the most rounds per shot action, the sloppiest spread.
    Full,
}

impl std::fmt::Display for ModeKind {
    /// The canonical human-facing mode LABEL the picker shows — `"single"` /
    /// `"burst"` / `"full-auto"` — derived from the kind (no stored name string).
    /// These preserve the prior per-mode label strings (the deleted GTW-256
    /// name-string newtype).
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let label = match self {
            Self::Single => "single",
            Self::Burst => "burst",
            Self::Full => "full-auto",
        };
        f.write_str(label)
    }
}

/// One fire mode's per-mode numbers — its kind, cone multiplier, TU%, and shot count.
///
/// The selector term carrier of `θ_cone` (resolution.md §1a) plus the mode's kind,
/// TU cost, and round count. A named struct (not a bare tuple) so each per-mode
/// number keeps its [`FireMode`] meaning; every field is a weapon NUMBER newtype or
/// the closed [`ModeKind`]. The human-facing label comes from
/// [`ModeKind`]'s [`Display`] (`kind.to_string()`), not a stored string.
///
/// Every field is `Copy`, so the spec is `Copy` (it regained the derive once the
/// `String` mode name was dropped — GTW-260; it was only `Clone`-not-`Copy` because
/// of the old owned name).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FireModeSpec {
    /// Which mode this is — its closed kind (`Single` / `Burst` / `Full`); the
    /// human-facing label is `kind.to_string()`.
    pub kind:       ModeKind,
    /// The selector cone multiplier for this mode (single ≈ 1, full-auto ≥ 1).
    pub cone_mult:  ModeConeMult,
    /// The fraction of the TU pool a shot in this mode costs.
    pub tu_percent: ModeTuPercent,
    /// How many rounds this mode fires per shot action.
    pub shots:      ModeShots,
}

impl FireModeSpec {
    /// Build one fire mode's spec from its kind and its three per-mode numbers.
    #[must_use]
    pub const fn new(
        kind: ModeKind,
        cone_mult: ModeConeMult,
        tu_percent: ModeTuPercent,
        shots: ModeShots,
    ) -> Self {
        Self {
            kind,
            cone_mult,
            tu_percent,
            shots,
        }
    }
}

/// A weapon's **fire-mode selector** — the LIST of modes a weapon offers, each a
/// [`FireModeSpec`] paired with its closed [`ModeKind`] (resolution.md §1: the
/// selector is authored per weapon). A weapon offers **any subset of `{Single,
/// Burst, Full}` in authored order** (GTW-260 broadened the old three fixed ladders
/// — single / single+burst / single+burst+full-auto — to an arbitrary ordered list;
/// the docs-sync recording this is GTW-258).
///
/// A named newtype over `Vec<`[`FireModeSpec`]`>` (no-bare-types: the selector is a
/// domain value; the inner `Vec` is the collection-of-domain-values carve-out). The
/// private inner + derived [`Deref`] gives slice access (`.iter()` / `.len()` /
/// `.get()`); `#[serde(transparent)]` so it deserializes from a **bare RON list** of
/// mode entries (`fire_mode: [ (kind: Single, …), … ]`). `Clone`-not-`Copy` (it
/// holds a `Vec`). A `#[derive(Component)]` (GTW-200) — the selector lives as a
/// sibling component on the armed entity (its per-mode [`FireModeSpec`] sub-values
/// ride inside it, not as separate components).
///
/// **Invariant:** a well-authored weapon lists at least one mode, with `Single`
/// first. The code is DEFENSIVE if that is violated — every read has a total
/// fallback and never panics (see [`FireMode::single`]).
#[derive(Component, Deref, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FireMode(Vec<FireModeSpec>);

impl FireMode {
    /// Build a fire-mode selector from its authored list of modes.
    #[must_use]
    pub const fn new(modes: Vec<FireModeSpec>) -> Self {
        Self(modes)
    }

    /// The **single-shot spec** — the mode whose [`ModeKind`] is
    /// [`ModeKind::Single`]; else the FIRST authored mode; else a structural
    /// single-shot default (`Single`, cone ×1.0, 0% TU, 1 shot). Returns BY VALUE
    /// ([`FireModeSpec`] is `Copy` again). The fallback chain is TOTAL — NO `unwrap`
    /// / `panic` even for an empty or `Single`-less (mis-authored) selector.
    #[must_use]
    pub fn single(&self) -> FireModeSpec {
        if let Some(single) = self.0.iter().find(|spec| spec.kind == ModeKind::Single) {
            *single
        } else if let Some(first) = self.0.first() {
            *first
        } else {
            FireModeSpec::new(
                ModeKind::Single,
                ModeConeMult::new(1.0),
                ModeTuPercent::new(0.0),
                ModeShots::new(1),
            )
        }
    }
}

/// The **`Weapon` MARKER** — a unit `#[derive(Component)]` tag (no data) marking an
/// entity as armed (GTW-200's user-corrected model).
///
/// The weapon's stats are **not** packed inside this — each is its own sibling
/// `#[derive(Component)]` newtype on the same entity (`BaseSpread`, `Accuracy`,
/// `Kickback`, `FatalBias`, `WeaponDamage`, `WeaponPunch`, `WeaponShred`,
/// `DamageType`, `MagazineSize`, `FireMode`, `Stable`), spawned together via
/// [`WeaponBundle`]. Because the stats are direct components, the combat act
/// (E4.5 `fire()`) is a proper query-based Bevy system with NO `&mut World`
/// indirection: it queries the individual stat components off the ganger entity (or
/// assembles a transient [`WeaponStats`] borrow-view from them). There is **no**
/// `Weapon::new` and no packed data struct any more — the stats live as components.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Weapon;

/// A transient **borrow-view** of a weapon's stats — refs assembled at the call
/// site from the individual weapon components, the read-shape the §1/§6 readers
/// take in place of the old `&Weapon` data struct (GTW-200).
///
/// This is **NOT** a stored `Component` — it is a short-lived bundle of borrows a
/// caller (a Bevy system, or the E4.5 `fire()` act) builds from a queried entity's
/// weapon stat components, exactly as [`crate::aim::Shooter`] /
/// [`crate::resolve_and_apply::TargetGanger`] do for ganger state. Grouping the
/// refs keeps [`crate::resolve_and_apply::resolve_and_apply`] and
/// [`crate::aim::cone_for`] under clippy's argument-count gate while letting the
/// weapon be plain ECS components. Every field is a borrowed weapon-number newtype
/// (no bare primitive); the view itself is a transparent borrow record, never a
/// wrapped domain scalar.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WeaponStats<'a> {
    /// The intrinsic angular spread before situational multipliers (`base_spread`).
    pub base_spread: &'a BaseSpread,
    /// The concentration weapon term (`accuracy`; may exceed 1.0).
    pub accuracy:    &'a Accuracy,
    /// The per-round recoil added in a burst (`kickback`).
    pub kickback:    &'a Kickback,
    /// The severity-score addend, consumed by E3 (`fatal_bias`).
    pub fatal_bias:  &'a FatalBias,
    /// The base damage a hit deals before armor (`damage`).
    pub damage:      &'a WeaponDamage,
    /// The armor protection a hit ignores — penetration (`punch`).
    pub punch:       &'a WeaponPunch,
    /// The extra integrity damage a hit deals to armor durability (`shred`).
    pub shred:       &'a WeaponShred,
    /// The damage type the weapon emits — its matchup-wheel node.
    pub damage_type: &'a DamageType,
    /// The `stable` tag — `true` engages the §1a brace bonus unconditionally
    /// (regardless of faced cover / stance); `false` is a normal weapon (braces
    /// only when the faced cover suits the stance).
    pub stable:      &'a Stable,
}

/// The **spawn bundle for an armed entity** — the [`Weapon`] marker plus the full
/// set of weapon stat components, inserted together (GTW-200).
///
/// A Bevy [`Bundle`] so spawning an armed (ganger) entity carries the marker and
/// every stat component in one `commands.spawn(...)` / `entity.insert(...)` call.
/// The current ammo count is **not** here — that is the separate
/// [`crate::magazine::Magazine`] battle-state component (this bundle carries the
/// [`MagazineSize`] capacity only). Every field is a weapon-number newtype, the
/// [`WeaponName`], or the marker (no bare primitive); build one with
/// [`WeaponBundle::new`].
///
/// **Not `Copy`** — it carries a [`WeaponName`] ([`String`]) and a [`FireMode`]
/// (which holds a `Vec`); the bundle is `Clone`.
#[derive(Bundle, Debug, Clone, PartialEq)]
pub struct WeaponBundle {
    /// The [`Weapon`] marker tagging the entity as armed.
    pub marker:        Weapon,
    /// The weapon's human-facing name (its own sibling component).
    pub name:          WeaponName,
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
    /// The authored fire-mode selector and its per-mode numbers.
    pub fire_mode:     FireMode,
    /// The `stable` tag — `true` engages the §1a brace bonus unconditionally.
    pub stable:        Stable,
}

/// A weapon's **damage block** for spawning — the three per-hit damage numbers plus
/// the emitted [`DamageType`] (`weapons-and-armor.md` §"Weapon stats" / matchup.md
/// §"The 7 types"), handed to [`WeaponBundle::new`] as one cohesive value.
///
/// An owned ctor-input grouping (the [`FireModeSpec`] precedent: related data
/// travels as one named record, not a loose tuple) so [`WeaponBundle::new`] stays
/// under clippy's argument-count gate. Distinct from the borrow-view
/// [`WeaponStats`]: this owns its four newtypes (spawn-side input), the view borrows
/// the live components (read-side). Every field is a weapon-number newtype.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DamageProfile {
    /// The base damage a hit deals before armor.
    pub damage:      WeaponDamage,
    /// The armor protection a hit ignores — penetration.
    pub punch:       WeaponPunch,
    /// The extra integrity damage a hit deals to armor durability.
    pub shred:       WeaponShred,
    /// The damage type the weapon emits — its matchup-wheel node.
    pub damage_type: DamageType,
}

impl DamageProfile {
    /// Build a damage block from the three per-hit numbers and the emitted
    /// [`DamageType`] (all magnitudes TBD tuning).
    #[must_use]
    pub const fn new(
        damage: WeaponDamage,
        punch: WeaponPunch,
        shred: WeaponShred,
        damage_type: DamageType,
    ) -> Self {
        Self {
            damage,
            punch,
            shred,
            damage_type,
        }
    }
}

/// A weapon's **handling block** for spawning — its [`MagazineSize`] capacity,
/// authored [`FireMode`] selector, and [`Stable`] tag, handed to
/// [`WeaponBundle::new`] as one cohesive value.
///
/// An owned ctor-input grouping (the [`DamageProfile`] / [`FireModeSpec`]
/// precedent) so [`WeaponBundle::new`] stays under clippy's argument-count gate.
/// Every field is a weapon-number newtype / the [`FireMode`] selector.
///
/// **Not `Copy`** — it owns a [`FireMode`] (which holds a `Vec` of specs); it is
/// `Clone`.
#[derive(Debug, Clone, PartialEq)]
pub struct HandlingProfile {
    /// The round capacity before a reload.
    pub magazine_size: MagazineSize,
    /// The authored fire-mode selector and its per-mode numbers.
    pub fire_mode:     FireMode,
    /// The `stable` tag — `true` engages the §1a brace bonus unconditionally.
    pub stable:        Stable,
}

impl HandlingProfile {
    /// Build a handling block from a weapon's magazine capacity, fire-mode selector,
    /// and `stable` tag.
    #[must_use]
    pub const fn new(magazine_size: MagazineSize, fire_mode: FireMode, stable: Stable) -> Self {
        Self {
            magazine_size,
            fire_mode,
            stable,
        }
    }
}

impl WeaponBundle {
    /// Build an armed-entity bundle from a weapon's full stat set — the [`Weapon`]
    /// marker is supplied automatically; the stats are handed in as the weapon's
    /// [`WeaponName`], the §1/§6 cone/severity numbers, a [`DamageProfile`], and a
    /// [`HandlingProfile`].
    ///
    /// Takes the cohesive groups (the [`DamageProfile`] / [`HandlingProfile`]
    /// precedent) rather than a dozen loose params, keeping the ctor under clippy's
    /// argument-count gate while every stat lands as its own component on the
    /// spawned entity.
    #[must_use]
    pub fn new(
        name: WeaponName,
        base_spread: BaseSpread,
        accuracy: Accuracy,
        kickback: Kickback,
        fatal_bias: FatalBias,
        damage: DamageProfile,
        handling: HandlingProfile,
    ) -> Self {
        Self {
            marker: Weapon,
            name,
            base_spread,
            accuracy,
            kickback,
            fatal_bias,
            damage: damage.damage,
            punch: damage.punch,
            shred: damage.shred,
            damage_type: damage.damage_type,
            magazine_size: handling.magazine_size,
            fire_mode: handling.fire_mode,
            stable: handling.stable,
        }
    }

    /// Assemble a transient [`WeaponStats`] borrow-view over this bundle's stat
    /// components — the read-shape the §1/§6 readers take. A convenience for callers
    /// holding a whole bundle; a query-based system assembles a [`WeaponStats`] from
    /// its individually-queried components instead.
    #[must_use]
    pub const fn stats(&self) -> WeaponStats<'_> {
        WeaponStats {
            base_spread: &self.base_spread,
            accuracy:    &self.accuracy,
            kickback:    &self.kickback,
            fatal_bias:  &self.fatal_bias,
            damage:      &self.damage,
            punch:       &self.punch,
            shred:       &self.shred,
            damage_type: &self.damage_type,
            stable:      &self.stable,
        }
    }
}

/// The **authoring struct** an `assets/weapons/*.ron` deserializes into — every
/// weapon NUMBER the §1/§6 math reads, MINUS the [`WeaponName`] (the name is the
/// FILE KEY, supplied by the loader from the file's stem) and MINUS the [`Weapon`]
/// marker (that is added by [`WeaponBundle::new`]).
///
/// This is the data-driven, folder-loaded weapon model (the
/// [[weapons-armor-data-driven]] end-state, GTW-257): a per-weapon loose `.ron`
/// file is parsed into a `WeaponSpec`, keyed by its filename stem into the
/// [`WeaponRegistry`], and resolved at battle setup into a [`WeaponBundle`] via
/// [`into_bundle`](WeaponSpec::into_bundle). It mirrors [`WeaponBundle`]'s data
/// exactly, dropping only the two fields the loader / spawn-side own: the name
/// (the file key) and the marker (the armed-entity tag).
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
    /// [`FireModeSpec`] carrying its [`ModeKind`] + cone/TU%/shots.
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
    /// [`WeaponBundle::new`] — the [`Weapon`] marker is added there. Consumes the
    /// spec by value (it owns the [`FireMode`]); a caller holding a borrowed spec
    /// clones it first (the registry's specs are `Clone`).
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

/// The **weapon registry** — a name→spec map the folder loader builds and the
/// battle setup resolves [`GangerSpawn`](crate::situation::GangerSpawn) weapon keys
/// against (GTW-257).
///
/// A named newtype [`Resource`] over a [`HashMap`]`<`[`WeaponName`]`,
/// `[`WeaponSpec`]`>` (no-bare-types: a registry is a domain value, not a bare
/// `HashMap`). The sim OWNS the weapon model, so the type lives here; the app's
/// `Load` flow POPULATES it from the loaded `assets/weapons/*.ron` folder (keyed by
/// each file's stem) and inserts it as a resource. It holds the specs BY VALUE
/// ([`WeaponSpec`] is `Clone`), so they survive even if the loaded-folder asset
/// handle is dropped.
///
/// Private inner with small accessors (the registry answers a weapon LOOKUP, not a
/// raw-map question — so no derived [`Deref`]). The setup resolves
/// [`GangerSpawn::weapon`](crate::situation::GangerSpawn) through [`spec`](WeaponRegistry::spec).
#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub struct WeaponRegistry(HashMap<WeaponName, WeaponSpec>);

impl WeaponRegistry {
    /// Build a weapon registry from a `(name, spec)` iterator — the shape the
    /// folder loader (and a test) keys by filename stem.
    #[must_use]
    pub fn new(weapons: impl IntoIterator<Item = (WeaponName, WeaponSpec)>) -> Self {
        Self(weapons.into_iter().collect())
    }

    /// Insert one weapon spec under its [`WeaponName`] key, returning the previous
    /// spec at that key (if any) — the per-file insert the folder loader calls as it
    /// iterates the loaded folder.
    pub fn insert(&mut self, name: WeaponName, spec: WeaponSpec) -> Option<WeaponSpec> {
        self.0.insert(name, spec)
    }

    /// Look up the [`WeaponSpec`] for a weapon KEY, or [`None`] if no weapon file
    /// with that stem was loaded — the setup-time resolution the battle reads.
    #[must_use]
    pub fn spec(&self, name: &WeaponName) -> Option<&WeaponSpec> {
        self.0.get(name)
    }

    /// How many weapons the registry holds — the count the folder-load test asserts.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether the registry holds no weapons.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use bevy::prelude::World;

    use super::*;

    /// Build an arbitrary `Single`-kind fire-mode spec from raw literals — NOT
    /// shipped magnitudes (these only exercise the type surface). For an explicit
    /// kind, use [`kind_spec`].
    const fn spec(cone: f32, tu: f32, shots: u16) -> FireModeSpec {
        kind_spec(ModeKind::Single, cone, tu, shots)
    }

    /// Build a fire-mode spec with an explicit [`ModeKind`] — for the kind / label
    /// round-trip tests (the kind is the thing under test).
    const fn kind_spec(kind: ModeKind, cone: f32, tu: f32, shots: u16) -> FireModeSpec {
        FireModeSpec::new(
            kind,
            ModeConeMult::new(cone),
            ModeTuPercent::new(tu),
            ModeShots::new(shots),
        )
    }

    /// Build an arbitrary damage block from raw literals — NOT shipped magnitudes.
    /// Pins the damage-leaf mechanism only.
    fn profile(damage: i32, punch: i32, shred: i32, damage_type: DamageType) -> DamageProfile {
        DamageProfile::new(
            WeaponDamage::new(damage),
            WeaponPunch::new(punch),
            WeaponShred::new(shred),
            damage_type,
        )
    }

    /// Build an arbitrary handling block (magazine + single-mode selector +
    /// `stable` tag) from raw literals — NOT shipped magnitudes.
    fn handling(mag: u16, stable: bool) -> HandlingProfile {
        HandlingProfile::new(
            MagazineSize::new(mag),
            FireMode::new(vec![spec(1.0, 0.5, 1)]),
            Stable::new(stable),
        )
    }

    /// Build an arbitrary armed-entity bundle from the four §1/§6 cone/severity
    /// numbers plus a [`DamageProfile`] / [`HandlingProfile`] — NOT shipped
    /// magnitudes. The construction path every weapon test fixture migrates to;
    /// takes the cohesive groups so the helper stays under the argument-count gate.
    fn weapon_bundle(
        base: f32,
        accuracy: f32,
        kick: f32,
        bias: f32,
        damage: DamageProfile,
        handling: HandlingProfile,
    ) -> WeaponBundle {
        WeaponBundle::new(
            WeaponName::new("test-weapon".to_owned()),
            BaseSpread::new(base),
            Accuracy::new(accuracy),
            Kickback::new(kick),
            FatalBias::new(bias),
            damage,
            handling,
        )
    }

    /// AC1 — `Weapon` is a **unit MARKER** (no data fields): it constructs from the
    /// unit value, is `Copy`, and compares by value. A struct with a field could not
    /// be built this way.
    #[test]
    fn weapon_is_a_unit_marker() {
        let marker = Weapon;
        let copied = marker; // Copy, not a move
        assert_eq!(marker, copied, "the Weapon marker is a Copy unit type");
        // A unit marker carries no data — two independently-built values are equal.
        let another = Weapon;
        assert_eq!(marker, another, "the marker is a fieldless unit type");
    }

    /// AC1 — each weapon sub-value derives `Component`: insert each onto a fresh
    /// `World` entity and query it back. If any newtype lacked the `Component`
    /// derive this would not compile, so the test IS the proof the derive is present
    /// (and the values round-trip through the ECS).
    #[test]
    fn each_sub_value_is_a_component() {
        let mut world = World::new();
        let entity = world
            .spawn((
                Weapon,
                WeaponName::new("autogun".to_owned()),
                BaseSpread::new(0.25),
                Accuracy::new(1.3),
                Kickback::new(0.4),
                FatalBias::new(7.0),
                WeaponDamage::new(12),
                WeaponPunch::new(5),
                WeaponShred::new(3),
                DamageType::Kinetic,
                MagazineSize::new(30),
                FireMode::new(vec![spec(1.0, 0.5, 1)]),
                Stable::new(true),
            ))
            .id();

        // Every component queries back off the entity (mechanism, not magnitude;
        // distinct arbitrary literals so a mix-up would surface).
        assert!(
            world.get::<Weapon>(entity).is_some(),
            "the marker is present"
        );
        let Some(name) = world.get::<WeaponName>(entity) else {
            return;
        };
        assert_eq!(
            &**name, "autogun",
            "the weapon name round-trips through the ECS"
        );
        let Some(base) = world.get::<BaseSpread>(entity) else {
            return;
        };
        assert_eq!((**base).to_bits(), 0.25_f32.to_bits());
        let Some(damage) = world.get::<WeaponDamage>(entity) else {
            return;
        };
        assert_eq!(**damage, 12i32);
        let Some(damage_type) = world.get::<DamageType>(entity) else {
            return;
        };
        assert_eq!(*damage_type, DamageType::Kinetic);
        let Some(stable) = world.get::<Stable>(entity) else {
            return;
        };
        assert!(**stable, "the stable tag round-trips through the ECS");
    }

    /// AC2 — a `WeaponBundle` spawns an entity carrying the full weapon component
    /// set + the `Weapon` marker; every component (and the marker) queries back off
    /// the spawned entity. Built from arbitrary literals (mechanism, not magnitude).
    #[test]
    fn weapon_bundle_spawns_an_armed_entity() {
        let mut world = World::new();
        let bundle = weapon_bundle(
            0.25,
            1.3,
            0.4,
            7.0,
            profile(12, 5, 3, DamageType::Kinetic),
            handling(30, false),
        );
        let entity = world.spawn(bundle).id();

        // The marker + the name + all eleven stat components are present.
        assert!(
            world.get::<Weapon>(entity).is_some(),
            "the bundle carries the Weapon marker",
        );
        assert!(
            world.get::<WeaponName>(entity).is_some(),
            "the bundle carries the WeaponName",
        );
        assert!(world.get::<BaseSpread>(entity).is_some(), "base_spread");
        assert!(world.get::<Accuracy>(entity).is_some(), "accuracy");
        assert!(world.get::<Kickback>(entity).is_some(), "kickback");
        assert!(world.get::<FatalBias>(entity).is_some(), "fatal_bias");
        assert!(world.get::<WeaponDamage>(entity).is_some(), "damage");
        assert!(world.get::<WeaponPunch>(entity).is_some(), "punch");
        assert!(world.get::<WeaponShred>(entity).is_some(), "shred");
        assert!(world.get::<DamageType>(entity).is_some(), "damage_type");
        assert!(world.get::<MagazineSize>(entity).is_some(), "magazine_size");
        assert!(world.get::<FireMode>(entity).is_some(), "fire_mode");
        assert!(world.get::<Stable>(entity).is_some(), "stable");

        // A spot value round-trips off the spawned entity (distinct literals).
        let Some(damage) = world.get::<WeaponDamage>(entity) else {
            return;
        };
        assert_eq!(**damage, 12i32);
    }

    /// AC1 — each weapon-number leaf derefs to its inner value (the no-bare-types
    /// mechanism), read directly off the component newtypes. Built from
    /// **arbitrary** literals (never pinned magnitudes).
    #[test]
    fn weapon_leaves_deref_to_inner() {
        let bundle = weapon_bundle(
            0.25,
            1.3,
            0.4,
            7.0,
            profile(12, 5, 3, DamageType::Kinetic),
            handling(30, false),
        );

        // Each f32 weapon number: deref reaches the inner f32 (bit-exact arbitrary
        // value — exactly representable literals, so this is an integer equality,
        // no float_cmp lint).
        assert_eq!((*bundle.base_spread).to_bits(), 0.25_f32.to_bits());
        assert_eq!((*bundle.accuracy).to_bits(), 1.3_f32.to_bits());
        assert_eq!((*bundle.kickback).to_bits(), 0.4_f32.to_bits());
        assert_eq!((*bundle.fatal_bias).to_bits(), 7.0_f32.to_bits());
        // The u16 weapon number: deref reaches the inner u16.
        assert_eq!(*bundle.magazine_size, 30u16);
    }

    /// AC1 — the `damage` / `punch` / `shred` components are distinct newtypes that
    /// deref to their inner `i32`. Built from **arbitrary** literals (no pinned
    /// magnitude); reads each leaf back through `Deref`.
    #[test]
    fn weapon_damage_leaves_deref_to_inner() {
        let bundle = weapon_bundle(
            0.1,
            1.0,
            0.2,
            4.0,
            profile(18, 7, 2, DamageType::Plasma),
            handling(12, false),
        );

        // Each i32 damage number derefs to its inner value (distinct arbitrary
        // literals so a field swap would be caught — mechanism, not magnitude).
        assert_eq!(*bundle.damage, 18i32);
        assert_eq!(*bundle.punch, 7i32);
        assert_eq!(*bundle.shred, 2i32);
    }

    /// AC3 — the `DamageType` component round-trips off the bundle (mechanism, not
    /// magnitude). Pairs with `armor_piece_carries_armor_type` in `armor.rs`.
    #[test]
    fn weapon_carries_damage_type_round_trip() {
        let bundle = weapon_bundle(
            0.1,
            1.0,
            0.2,
            4.0,
            profile(1, 1, 1, DamageType::Rend),
            handling(1, false),
        );
        assert_eq!(bundle.damage_type, DamageType::Rend);
    }

    /// AC1 (GTW-199) — the `Stable` component round-trips and derefs to its inner
    /// `bool`. A stable and a non-stable bundle are built so a field swap (or a
    /// default) would be caught — mechanism, not a pinned balance value.
    #[test]
    fn weapon_carries_stable_tag_round_trip() {
        let stable_bundle = weapon_bundle(
            0.1,
            1.0,
            0.2,
            4.0,
            profile(1, 1, 1, DamageType::Kinetic),
            handling(1, true),
        );
        let plain_bundle = weapon_bundle(
            0.1,
            1.0,
            0.2,
            4.0,
            profile(1, 1, 1, DamageType::Kinetic),
            handling(1, false),
        );
        // The tag round-trips through the newtype's Deref to the inner bool.
        assert!(*stable_bundle.stable);
        assert!(!*plain_bundle.stable);
        assert_eq!(stable_bundle.stable, Stable::new(true));
        assert_eq!(plain_bundle.stable, Stable::new(false));
    }

    /// AC3 — a [`WeaponStats`] borrow-view assembled off a bundle reads the same
    /// stats as the bundle's components: the read-shape the §1/§6 readers take is
    /// faithful to the stored components. (`WeaponBundle::stats` is the convenience
    /// assembler; a query-based system builds the view from its queried components.)
    #[test]
    fn weapon_stats_view_borrows_the_components() {
        let bundle = weapon_bundle(
            0.2,
            1.1,
            0.3,
            5.0,
            profile(14, 6, 4, DamageType::Blast),
            handling(24, true),
        );
        let stats = bundle.stats();
        // Both sides deref to the same inner value: the view borrows the components.
        assert_eq!(
            (**stats.base_spread).to_bits(),
            (*bundle.base_spread).to_bits()
        );
        assert_eq!(**stats.damage, *bundle.damage);
        assert_eq!(**stats.punch, *bundle.punch);
        assert_eq!(**stats.shred, *bundle.shred);
        assert_eq!(*stats.damage_type, bundle.damage_type);
        assert_eq!(
            (**stats.fatal_bias).to_bits(),
            (*bundle.fatal_bias).to_bits()
        );
        assert!(**stats.stable);
    }

    /// AC1 (GTW-199) — the `Stable` tag deserializes from a bare RON boolean
    /// (`#[serde(transparent)]`): a `true` fragment parses to a stable tag, a
    /// `false` fragment to a non-stable one (value read back, not a pinned score).
    #[test]
    fn stable_parses_from_bare_ron_bool() {
        let yes = ron::from_str::<Stable>("true");
        let no = ron::from_str::<Stable>("false");
        assert!(
            yes.is_ok(),
            "Stable must parse from a bare RON `true`: {yes:?}"
        );
        assert!(
            no.is_ok(),
            "Stable must parse from a bare RON `false`: {no:?}"
        );
        let (Ok(yes), Ok(no)) = (yes, no) else {
            return;
        };
        assert!(*yes);
        assert!(!*no);
    }

    /// C5 — each weapon-number leaf still round-trips as a bare RON scalar
    /// (`#[serde(transparent)]`) even now it is a `Component`: parse a bare fragment
    /// for each transparent leaf (value-agnostic — only that it parses into the
    /// type).
    #[test]
    fn weapon_leaves_parse_from_bare_ron_scalars() {
        assert!(ron::from_str::<BaseSpread>("0.2").is_ok(), "base_spread");
        assert!(ron::from_str::<Accuracy>("1.1").is_ok(), "accuracy");
        assert!(ron::from_str::<Kickback>("0.3").is_ok(), "kickback");
        assert!(ron::from_str::<FatalBias>("5.0").is_ok(), "fatal_bias");
        assert!(ron::from_str::<WeaponDamage>("14").is_ok(), "damage");
        assert!(ron::from_str::<WeaponPunch>("6").is_ok(), "punch");
        assert!(ron::from_str::<WeaponShred>("4").is_ok(), "shred");
        assert!(ron::from_str::<MagazineSize>("24").is_ok(), "magazine_size");
        assert!(
            ron::from_str::<DamageType>("Kinetic").is_ok(),
            "damage_type"
        );
    }

    /// AC2 (one half) — `DamageType` has exactly 7 variants, in wheel-node order.
    /// The mirror-parity half lives in `armor.rs` (it needs both enums).
    #[test]
    fn damage_type_has_seven_variants() {
        assert_eq!(DamageType::ALL.len(), 7);
        // Node order is pinned (matchup.md Table 1) — the parity test in armor.rs
        // relies on it.
        assert_eq!(
            DamageType::ALL,
            [
                DamageType::Shock,
                DamageType::Blast,
                DamageType::Chem,
                DamageType::Kinetic,
                DamageType::Plasma,
                DamageType::Rend,
                DamageType::Las,
            ]
        );
    }

    /// AC2 — a [`FireMode`] is a `Vec` of modes: `.iter()` yields the authored modes
    /// in order, `.len()` is the count, and `single()` returns the `Single`-kind
    /// spec. Built from arbitrary literals (mechanism, not magnitude).
    #[test]
    fn fire_mode_is_a_vec_of_modes_in_authored_order() {
        // single only
        let single_only = FireMode::new(vec![spec(1.0, 0.5, 1)]);
        assert_eq!(single_only.len(), 1, "a single-only weapon offers one mode");
        assert_eq!(single_only.single().kind, ModeKind::Single);
        assert_eq!(*single_only.single().shots, 1u16);

        // single + burst + full-auto, in authored order
        let full = FireMode::new(vec![
            kind_spec(ModeKind::Single, 1.0, 0.5, 1),
            kind_spec(ModeKind::Burst, 1.2, 0.8, 3),
            kind_spec(ModeKind::Full, 1.6, 1.0, 8),
        ]);
        assert_eq!(full.len(), 3, "a three-mode weapon offers three modes");
        // `.iter()` yields the authored modes in order.
        let kinds: Vec<ModeKind> = full.iter().map(|spec| spec.kind).collect();
        assert_eq!(
            kinds,
            vec![ModeKind::Single, ModeKind::Burst, ModeKind::Full],
            "the modes iterate in authored order",
        );
        // `single()` finds the Single-kind spec regardless of list length.
        assert_eq!(full.single().kind, ModeKind::Single);
        assert_eq!(*full.single().shots, 1u16);

        // single() falls back to the FIRST mode when no Single-kind is authored
        // (defensive — never panics on a mis-authored weapon).
        let no_single = FireMode::new(vec![kind_spec(ModeKind::Burst, 1.2, 0.8, 3)]);
        assert_eq!(
            no_single.single().kind,
            ModeKind::Burst,
            "single() falls back to the first authored mode",
        );

        // single() falls back to a structural default on an EMPTY selector.
        let empty = FireMode::new(vec![]);
        assert_eq!(
            empty.single().kind,
            ModeKind::Single,
            "single() returns a structural Single default for an empty selector",
        );
        assert_eq!(*empty.single().shots, 1u16);
    }

    /// AC1 — [`ModeKind`] renders its canonical human labels via [`Display`]:
    /// `Single` → `"single"`, `Burst` → `"burst"`, `Full` → `"full-auto"`.
    #[test]
    fn mode_kind_display_labels() {
        assert_eq!(ModeKind::Single.to_string(), "single");
        assert_eq!(ModeKind::Burst.to_string(), "burst");
        assert_eq!(ModeKind::Full.to_string(), "full-auto");
    }

    /// AC1 — [`ModeKind`] round-trips as the closed enum through RON (parses each
    /// variant by name).
    #[test]
    fn mode_kind_round_trips_through_ron() {
        assert_eq!(
            ron::from_str::<ModeKind>("Single").ok(),
            Some(ModeKind::Single)
        );
        assert_eq!(
            ron::from_str::<ModeKind>("Burst").ok(),
            Some(ModeKind::Burst)
        );
        assert_eq!(ron::from_str::<ModeKind>("Full").ok(), Some(ModeKind::Full));
    }

    /// AC2 / C5 — the [`FireMode`] selector deserializes from a **bare RON list** of
    /// mode entries (each `( kind: …, cone_mult: …, tu_percent: …, shots: … )`):
    /// parses a hand-written list and asserts the per-mode KINDS in order (the thing
    /// under test), NOT the tunable cone/TU magnitudes (brittle-test rule).
    #[test]
    fn fire_mode_parses_from_a_bare_ron_list() {
        let ron = r"[
            ( kind: Single, cone_mult: 1.0, tu_percent: 0.5, shots: 1),
            ( kind: Burst,  cone_mult: 1.3, tu_percent: 0.8, shots: 3),
            ( kind: Full,   cone_mult: 1.7, tu_percent: 1.0, shots: 10),
        ]";
        let parsed = ron::from_str::<FireMode>(ron);
        assert!(
            parsed.is_ok(),
            "a FireMode selector must deserialize from a bare RON list: {parsed:?}",
        );
        let Ok(fire_mode) = parsed else {
            return;
        };
        // The authored per-mode KINDS round-trip onto each list entry, in order.
        let kinds: Vec<ModeKind> = fire_mode.iter().map(|spec| spec.kind).collect();
        assert_eq!(
            kinds,
            vec![ModeKind::Single, ModeKind::Burst, ModeKind::Full],
        );
    }

    /// AC2 — a [`FireModeSpec`] round-trips through serialize → deserialize unchanged
    /// (`deserialize(serialize(x)) == x`): the per-mode kind and numbers survive a
    /// RON serialize and re-parse. Value-equality of the whole spec, exercising the
    /// serde mechanism (not a pinned tunable).
    #[test]
    fn fire_mode_spec_ron_round_trip_is_identity() {
        let spec = kind_spec(ModeKind::Burst, 1.3, 0.8, 3);
        let Ok(serialized) = ron::to_string(&spec) else {
            return;
        };
        let parsed = ron::from_str::<FireModeSpec>(&serialized);
        assert_eq!(
            parsed.ok(),
            Some(spec),
            "deserialize(serialize(spec)) must equal the original FireModeSpec",
        );
    }

    /// AC1 — a [`WeaponName`] is a documented `#[derive(Component, Deref,
    /// Deserialize)]` newtype over `String`: it is a [`WeaponBundle`] field (read back
    /// off a constructed bundle) and parses from a bare RON string
    /// (`#[serde(transparent)]`).
    #[test]
    fn weapon_name_is_a_bundle_field_and_parses_from_ron() {
        let bundle = WeaponBundle::new(
            WeaponName::new("boltgun".to_owned()),
            BaseSpread::new(0.2),
            Accuracy::new(1.0),
            Kickback::new(0.1),
            FatalBias::new(3.0),
            profile(10, 4, 2, DamageType::Kinetic),
            handling(20, false),
        );
        // The name is a bundle field, read back through its Deref to the inner String.
        assert_eq!(
            &*bundle.name, "boltgun",
            "WeaponName is a WeaponBundle field"
        );
        // And it deserializes from a bare RON string.
        let Ok(parsed) = ron::from_str::<WeaponName>(r#""boltgun""#) else {
            return;
        };
        assert_eq!(
            &*parsed, "boltgun",
            "WeaponName must parse from a bare RON string"
        );
    }

    /// AC1/AC4 — a [`ModeKind`] rides on each [`FireModeSpec`] (read back via the
    /// spec's `kind` field) and its human label is `kind.to_string()` (no stored
    /// name string — the GTW-256 name-string newtype is gone).
    #[test]
    fn mode_kind_rides_on_fire_mode_spec_and_labels_via_display() {
        let spec = kind_spec(ModeKind::Full, 1.7, 1.0, 10);
        assert_eq!(
            spec.kind,
            ModeKind::Full,
            "ModeKind is a FireModeSpec field"
        );
        assert_eq!(
            spec.kind.to_string(),
            "full-auto",
            "the label derives from ModeKind's Display",
        );
    }

    // === GTW-257: WeaponSpec authoring struct + WeaponRegistry ===

    /// A shipped weapon `.ron`, read at compile time via the same `include_str!`
    /// pattern `tuning.rs` / `situation.rs` use — the REAL on-disk authored file
    /// (`assets/weapons/autogun.weapon.ron`), so a regression in the authored file
    /// turns this red.
    const SHIPPED_AUTOGUN_RON: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../assets/weapons/autogun.weapon.ron"
    ));

    /// GTW-257 AC1 — the shipped `assets/weapons/autogun.weapon.ron` parses into a
    /// `WeaponSpec`, and `into_bundle(name)` yields a `WeaponBundle` carrying that
    /// `WeaponName` + a `FireMode` list whose modes carry their `ModeKind`.
    /// Value-agnostic on the tunable cone/TU/damage magnitudes (the authored numbers
    /// are DATA, not pinned by the test): it asserts the NAME landed and the
    /// fire-mode offers at least one mode whose `single()` is `Single`-kind, never a
    /// magnitude.
    #[test]
    fn shipped_weapon_spec_parses_and_converts_to_a_bundle() {
        let parsed = ron::de::from_str::<WeaponSpec>(SHIPPED_AUTOGUN_RON);
        assert!(
            parsed.is_ok(),
            "the shipped assets/weapons/autogun.weapon.ron must parse into a WeaponSpec: {parsed:?}",
        );
        let Ok(spec) = parsed else {
            return;
        };

        // The file does NOT author a name — the name is the FILE KEY, supplied here
        // (the loader supplies the filename stem). into_bundle carries it through.
        let key = WeaponName::new("autogun".to_owned());
        let bundle = spec.into_bundle(key);
        assert_eq!(
            &*bundle.name, "autogun",
            "into_bundle must carry the supplied WeaponName (the file key) onto the bundle",
        );
        // The Weapon marker is added by into_bundle (it is NOT authored in the file).
        assert_eq!(bundle.marker, Weapon, "into_bundle adds the Weapon marker");
        // The selector offers at least one mode, and single() is the Single-kind one
        // (mechanism, not a value) — the authored list round-tripped.
        assert!(
            !bundle.fire_mode.is_empty(),
            "the authored fire-mode must offer at least one mode",
        );
        let single = bundle.fire_mode.single();
        assert_eq!(
            single.kind,
            ModeKind::Single,
            "the shipped autogun's single() mode is Single-kind",
        );
    }

    /// GTW-257 AC1 — a `WeaponSpec` round-trips from inline RON (no shipped magnitudes)
    /// and `into_bundle` groups the damage / handling blocks faithfully: a spot value
    /// read back off the bundle equals the authored one. Arbitrary literals
    /// (mechanism, not a balance pin), proving the authoring shape and the conversion.
    #[test]
    fn weapon_spec_round_trips_and_into_bundle_groups_faithfully() {
        let authored = r"(
            base_spread: 0.2, accuracy: 1.1, kickback: 0.3, fatal_bias: 5.0,
            damage: 14, punch: 6, shred: 4, damage_type: Kinetic,
            magazine_size: 24,
            fire_mode: [
                ( kind: Single, cone_mult: 1.0, tu_percent: 0.5, shots: 1),
                ( kind: Burst,  cone_mult: 1.3, tu_percent: 0.8, shots: 3),
            ],
            stable: false,
        )";
        let parsed = ron::de::from_str::<WeaponSpec>(authored);
        assert!(
            parsed.is_ok(),
            "inline WeaponSpec RON must parse: {parsed:?}"
        );
        let Ok(spec) = parsed else {
            return;
        };

        let bundle = spec.into_bundle(WeaponName::new("test-gun".to_owned()));
        // Spot values flowed through the DamageProfile / HandlingProfile grouping
        // (distinct arbitrary literals so a field swap would surface).
        assert_eq!(*bundle.damage, 14i32, "damage flows through DamageProfile");
        assert_eq!(*bundle.punch, 6i32, "punch flows through DamageProfile");
        assert_eq!(*bundle.shred, 4i32, "shred flows through DamageProfile");
        assert_eq!(bundle.damage_type, DamageType::Kinetic);
        assert_eq!(
            *bundle.magazine_size, 24u16,
            "magazine_size flows through HandlingProfile",
        );
        assert!(!*bundle.stable, "stable flows through HandlingProfile");
        // The two authored modes survived the parse + grouping, in order.
        let kinds: Vec<ModeKind> = bundle.fire_mode.iter().map(|spec| spec.kind).collect();
        assert_eq!(kinds, vec![ModeKind::Single, ModeKind::Burst]);
    }

    /// GTW-257 — a `WeaponRegistry` keys specs by `WeaponName` and resolves a lookup:
    /// a present key returns the spec, an absent key returns `None`. Built directly
    /// from `WeaponRegistry::new` (no `AssetServer` — the sim-unit shape AC2/AC3 use).
    #[test]
    fn weapon_registry_keys_and_resolves_by_name() {
        let Ok(spec) = ron::de::from_str::<WeaponSpec>(SHIPPED_AUTOGUN_RON) else {
            return;
        };
        let autogun = WeaponName::new("autogun".to_owned());
        let registry = WeaponRegistry::new([(autogun.clone(), spec)]);

        assert_eq!(
            registry.len(),
            1,
            "the registry holds the one inserted weapon"
        );
        assert!(!registry.is_empty(), "a one-weapon registry is non-empty");
        assert!(
            registry.spec(&autogun).is_some(),
            "a present key resolves to its spec",
        );
        assert!(
            registry
                .spec(&WeaponName::new("missing".to_owned()))
                .is_none(),
            "an absent key resolves to None (the setup-time WeaponNotFound trigger)",
        );
    }
}
