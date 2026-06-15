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

use bevy::prelude::{Bundle, Component, Deref};
use serde::Deserialize;

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

/// A per-mode **cone multiplier** — the selector term of `θ_cone` (the `firemode`
/// factor, resolution.md §1a): single ≈ 1, full-auto ≥ 1 (inherently sloppier).
/// A multiplier on the cone's angular size for that fire mode.
///
/// A weapon NUMBER (per-mode, on the [`FireMode`]). Private inner + derived
/// [`Deref`]; `#[serde(transparent)]`.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
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
#[derive(Deref, Debug, Clone, Copy, PartialEq, Deserialize)]
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
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct ModeShots(u16);

impl ModeShots {
    /// Build a per-mode shot count from its round-per-action number.
    #[must_use]
    pub const fn new(shots: u16) -> Self {
        Self(shots)
    }
}

/// One fire mode's per-mode numbers — its cone multiplier, TU%, and shot count.
///
/// The selector term carrier of `θ_cone` (resolution.md §1a) plus the mode's TU
/// cost and round count. A named struct (not a bare tuple) so each per-mode
/// number keeps its [`FireMode`] meaning; every field is a weapon NUMBER newtype.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct FireModeSpec {
    /// The selector cone multiplier for this mode (single ≈ 1, full-auto ≥ 1).
    pub cone_mult:  ModeConeMult,
    /// The fraction of the TU pool a shot in this mode costs.
    pub tu_percent: ModeTuPercent,
    /// How many rounds this mode fires per shot action.
    pub shots:      ModeShots,
}

impl FireModeSpec {
    /// Build one fire mode's spec from its three per-mode numbers.
    #[must_use]
    pub const fn new(cone_mult: ModeConeMult, tu_percent: ModeTuPercent, shots: ModeShots) -> Self {
        Self {
            cone_mult,
            tu_percent,
            shots,
        }
    }
}

/// A weapon's **fire-mode selector** — which modes a weapon offers, authored as
/// single / single+burst / single+burst+full-auto (resolution.md §1: "selector is
/// single / single+burst / single+burst+full-auto, authored per weapon").
///
/// A named domain enum (no-bare-types: a selector is a domain value, not a bare
/// `u8`/option list). The three variants are the authored ladders: a weapon that
/// only fires single shots, one that adds a burst, and one that adds full-auto on
/// top. Every variant carries the [`FireModeSpec`] for *each* mode it offers (so
/// `single` is present in all three), keeping the per-mode numbers on the
/// selector itself. A `#[derive(Component)]` (GTW-200) — the selector lives as a
/// sibling component on the armed entity (its per-mode [`FireModeSpec`] sub-values
/// ride inside it, not as separate components).
#[derive(Component, Debug, Clone, Copy, PartialEq, Deserialize)]
pub enum FireMode {
    /// Single-shot only.
    Single {
        /// The single-shot mode's per-mode numbers.
        single: FireModeSpec,
    },
    /// Single shot plus a burst.
    SingleBurst {
        /// The single-shot mode's per-mode numbers.
        single: FireModeSpec,
        /// The burst mode's per-mode numbers.
        burst:  FireModeSpec,
    },
    /// Single shot, a burst, and full-auto.
    SingleBurstFullAuto {
        /// The single-shot mode's per-mode numbers.
        single:    FireModeSpec,
        /// The burst mode's per-mode numbers.
        burst:     FireModeSpec,
        /// The full-auto mode's per-mode numbers.
        full_auto: FireModeSpec,
    },
}

impl FireMode {
    /// The single-shot spec — present on every selector variant (single is the
    /// base mode of all three).
    #[must_use]
    pub const fn single(&self) -> FireModeSpec {
        match *self {
            Self::Single { single }
            | Self::SingleBurst { single, .. }
            | Self::SingleBurstFullAuto { single, .. } => single,
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
/// [`MagazineSize`] capacity only). Every field is a weapon-number newtype or the
/// marker (no bare primitive); build one with [`WeaponBundle::new`].
#[derive(Bundle, Debug, Clone, Copy, PartialEq)]
pub struct WeaponBundle {
    /// The [`Weapon`] marker tagging the entity as armed.
    pub marker:        Weapon,
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
#[derive(Debug, Clone, Copy, PartialEq)]
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
    /// marker is supplied automatically; the stats are handed in as the §1/§6
    /// cone/severity numbers, a [`DamageProfile`], and a [`HandlingProfile`].
    ///
    /// Takes the cohesive groups (the [`DamageProfile`] / [`HandlingProfile`]
    /// precedent) rather than eleven loose params, keeping the ctor under clippy's
    /// argument-count gate while every stat lands as its own component on the
    /// spawned entity.
    #[must_use]
    pub const fn new(
        base_spread: BaseSpread,
        accuracy: Accuracy,
        kickback: Kickback,
        fatal_bias: FatalBias,
        damage: DamageProfile,
        handling: HandlingProfile,
    ) -> Self {
        Self {
            marker: Weapon,
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

#[cfg(test)]
mod tests {
    use bevy::prelude::World;

    use super::*;

    /// Build an arbitrary fire-mode spec from raw literals — NOT shipped
    /// magnitudes (there are no shipped weapons yet; these only exercise the type
    /// surface).
    fn spec(cone: f32, tu: f32, shots: u16) -> FireModeSpec {
        FireModeSpec::new(
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
            FireMode::Single {
                single: spec(1.0, 0.5, 1),
            },
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
                BaseSpread::new(0.25),
                Accuracy::new(1.3),
                Kickback::new(0.4),
                FatalBias::new(7.0),
                WeaponDamage::new(12),
                WeaponPunch::new(5),
                WeaponShred::new(3),
                DamageType::Kinetic,
                MagazineSize::new(30),
                FireMode::Single {
                    single: spec(1.0, 0.5, 1),
                },
                Stable::new(true),
            ))
            .id();

        // Every component queries back off the entity (mechanism, not magnitude;
        // distinct arbitrary literals so a mix-up would surface).
        assert!(
            world.get::<Weapon>(entity).is_some(),
            "the marker is present"
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

        // The marker + all eleven stat components are present.
        assert!(
            world.get::<Weapon>(entity).is_some(),
            "the bundle carries the Weapon marker",
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

    /// C2 — each [`FireMode`] selector variant constructs and its per-mode fields
    /// read back. The selector ladder is single / single+burst /
    /// single+burst+full-auto; `single` is present on all three.
    #[test]
    fn fire_mode_selector_variants_carry_per_mode_fields() {
        // single only
        let single_only = FireMode::Single {
            single: spec(1.0, 0.5, 1),
        };
        assert_eq!(*single_only.single().shots, 1u16);
        assert_eq!(
            (*single_only.single().cone_mult).to_bits(),
            1.0_f32.to_bits()
        );
        assert_eq!(
            (*single_only.single().tu_percent).to_bits(),
            0.5_f32.to_bits()
        );

        // single + burst — assert the variant matches, then bind it (the let-else
        // avoids the denied `assert!(false)` guard on the no-match arm).
        let single_burst = FireMode::SingleBurst {
            single: spec(1.0, 0.5, 1),
            burst:  spec(1.2, 0.8, 3),
        };
        assert!(matches!(single_burst, FireMode::SingleBurst { .. }));
        let FireMode::SingleBurst { single, burst } = single_burst else {
            return;
        };
        assert_eq!(*single.shots, 1u16);
        assert_eq!(*burst.shots, 3u16);
        assert_eq!((*burst.cone_mult).to_bits(), 1.2_f32.to_bits());
        assert_eq!((*burst.tu_percent).to_bits(), 0.8_f32.to_bits());
        // `single()` reaches the base mode regardless of variant.
        assert_eq!(*single_burst.single().shots, 1u16);

        // single + burst + full-auto
        let full = FireMode::SingleBurstFullAuto {
            single:    spec(1.0, 0.5, 1),
            burst:     spec(1.2, 0.8, 3),
            full_auto: spec(1.6, 1.0, 8),
        };
        assert!(matches!(full, FireMode::SingleBurstFullAuto { .. }));
        let FireMode::SingleBurstFullAuto {
            single: full_single,
            burst: full_burst,
            full_auto,
        } = full
        else {
            return;
        };
        assert_eq!(*full_single.shots, 1u16);
        assert_eq!(*full_burst.shots, 3u16);
        assert_eq!(*full_auto.shots, 8u16);
        assert_eq!((*full_auto.cone_mult).to_bits(), 1.6_f32.to_bits());
        assert_eq!(*full.single().shots, 1u16);
    }

    /// C5 — the [`FireMode`] selector (the one composite serde leaf, now also a
    /// `Component`) round-trips from authored RON with bare-scalar per-mode fields:
    /// parses a hand-written `SingleBurstFullAuto` fragment and asserts structural
    /// success (value-agnostic — only that it parses into the type). The flat-leaf
    /// transparent round-trips are in `weapon_leaves_parse_from_bare_ron_scalars`.
    #[test]
    fn fire_mode_parses_from_ron_with_bare_scalar_per_mode_fields() {
        let ron = r"SingleBurstFullAuto(
            single:    (cone_mult: 1.0, tu_percent: 0.5, shots: 1),
            burst:     (cone_mult: 1.3, tu_percent: 0.8, shots: 3),
            full_auto: (cone_mult: 1.7, tu_percent: 1.0, shots: 10),
        )";
        let parsed = ron::from_str::<FireMode>(ron);
        assert!(
            parsed.is_ok(),
            "a FireMode selector must deserialize from bare-scalar per-mode RON: {parsed:?}",
        );
    }
}
