//! The frozen [`HitResult`] record and its three named damage newtypes — the
//! pure-data output of the E3.3 per-hit formula ([`resolve_hit`](super::resolve_hit)).

use bevy::prelude::Deref;

/// The **penetrating damage** a hit drives into the target — the **pre-floor**
/// `max(0, inner)` of the per-hit formula (`docs/combat/resolution.md` §6
/// `pen_damage`; `weapons-and-armor.md` §"Per-hit resolution").
///
/// This is the value the §6 wound-severity roll is **gated by** (`roll_severity`,
/// E3.4) — "severity is penetration-gated", so a weak hit cannot reach the severe
/// buckets. It is **distinct from** [`HpDamage`]: a fully-soaked hit still bruises
/// HP (the floor) while penetrating `0`, which is exactly the resolution.md §6
/// graze (`< e0 → None`: HP loss only, no wound). A named domain newtype, not a
/// bare `i32` (no-bare-types); private inner + derived [`Deref`]. An `i32` to share
/// the formula's signed arithmetic (it is the clamp of a possibly-negative inner).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PenetratingDamage(i32);

impl PenetratingDamage {
    /// Build a penetrating-damage value from its computed magnitude.
    #[must_use]
    pub const fn new(pen: i32) -> Self {
        Self(pen)
    }
}

/// The **HP-loss damage** a hit deals — the `dmg = max(floor, inner)` of the
/// per-hit formula (`docs/combat/weapons-and-armor.md` §"Per-hit resolution" step
/// 2: "Damage to the ganger").
///
/// This is the HP reduction the (later E3.6) application spends against the
/// ganger's HP. It is floor-clamped: "a hit that lands on armor always deals at
/// least `floor`" (a vest still bruises). It is **distinct from**
/// [`PenetratingDamage`] — that one is pre-floor and gates wound severity, this
/// one is the post-floor HP loss. A named domain newtype (no-bare-types); private
/// inner + derived [`Deref`]; an `i32` matching the armor/weapon signed stats.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HpDamage(i32);

impl HpDamage {
    /// Build an HP-loss-damage value from its computed magnitude.
    #[must_use]
    pub const fn new(dmg: i32) -> Self {
        Self(dmg)
    }
}

/// The **integrity wear** a hit inflicts on the struck armor piece — the
/// `integrity_wear = min(protection, damage) + effPen + shred·mult` of the per-hit
/// formula (`docs/combat/weapons-and-armor.md` §"Per-hit resolution" step 3).
///
/// This is the amount the (later E3.6) application subtracts from the worn piece's
/// [`crate::armor::ArmorIntegrity`] — the soak/penetration wear **plus** the
/// weapon's shred (extra durability damage). This slice only **computes** the
/// wear; the actual `integrity −= …` mutation and the "useless at `≤ 0`" gate are
/// E3.6. A named domain newtype (no-bare-types); private inner + derived [`Deref`];
/// an `i32` matching [`crate::armor::ArmorIntegrity`]'s signed inner.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IntegrityWear(i32);

impl IntegrityWear {
    /// Build an integrity-wear value from its computed magnitude.
    #[must_use]
    pub const fn new(wear: i32) -> Self {
        Self(wear)
    }
}

/// A **signed intermediate damage magnitude** in the per-hit / melee resolution
/// arithmetic — a matchup-scaled weapon stat, an effective-penetration term, or a
/// melee-scaled damage component before it lands in one of the frozen damage
/// newtypes ([`HpDamage`] / [`PenetratingDamage`] / [`IntegrityWear`]).
///
/// The formula reasons in signed `i32` (protection can out-soak damage, driving an
/// inner value negative). This names that transient magnitude so a helper's
/// matchup / melee scaling boundary is not a bare `i32` (no-bare-types). Private
/// inner + derived [`Deref`]; build one via [`DamageMagnitude::new`].
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DamageMagnitude(i32);

impl DamageMagnitude {
    /// Build a signed intermediate damage magnitude from its computed value.
    #[must_use]
    pub const fn new(magnitude: i32) -> Self {
        Self(magnitude)
    }
}

/// A **damage magnitude in real (floating-point) form** — the intermediate
/// representation the matchup / melee scaling and the §6 severity-score penetration
/// term compute in before rounding back to an integer damage newtype.
///
/// The matchup / melee multipliers are `f32`, so a stat × multiplier product (or a
/// penetrating-damage count cast to real for the score) is a real number until it is
/// rounded. This names that pre-rounding real so the guarded-cast helpers do not
/// carry a bare `f32` (no-bare-types). Private inner + derived [`Deref`]; a `const`
/// [`get`](DamageReal::get) exposes the inner for the `const fn` rounding helpers
/// (derived `Deref` is not `const`).
#[derive(Deref, Debug, Clone, Copy, PartialEq)]
pub struct DamageReal(f32);

impl DamageReal {
    /// Build a real-valued damage magnitude from its floating-point value.
    #[must_use]
    pub const fn new(real: f32) -> Self {
        Self(real)
    }

    /// The inner real value — the `const` accessor the `const fn` rounding helpers
    /// read (the derived [`Deref`] is not usable in a `const fn`).
    #[must_use]
    pub const fn get(self) -> f32 {
        self.0
    }
}

/// The **frozen record** a single per-hit resolution produces — the output of
/// [`resolve_hit`](super::resolve_hit) (`docs/combat/resolution.md` §5,
/// `weapons-and-armor.md` §"Per-hit resolution").
///
/// A `Copy` value object of named newtypes (no bare primitive), carrying the three
/// downstream-consumed results of the formula:
/// - [`PenetratingDamage`] — drives the E3.4 wound-severity roll (penetration-gated);
/// - [`HpDamage`] — the HP reduction E3.6 applies;
/// - [`IntegrityWear`] — the armor wear E3.6 spends.
///
/// It is **pure data** — [`resolve_hit`](super::resolve_hit) mutates nothing to
/// make it; HP / Wounds / armor application is E3.6.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HitResult {
    /// Pre-floor penetrating damage — gates wound severity (E3.4).
    pub penetrating: PenetratingDamage,
    /// Post-floor HP-loss damage — the HP reduction (applied in E3.6).
    pub hp_damage:   HpDamage,
    /// Armor integrity wear — the durability damage (applied in E3.6).
    pub wear:        IntegrityWear,
}
