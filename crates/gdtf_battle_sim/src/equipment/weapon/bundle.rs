//! The **spawn-side** weapon shapes — the [`WeaponBundle`] that spawns an armed
//! entity, the owned ctor-input groupings ([`DamageProfile`] / [`HandlingProfile`])
//! it takes, and the transient [`WeaponStats`] borrow-view the §1/§6 readers take
//! (GTW-200).

use bevy::prelude::Bundle;

use super::{
    Accuracy, BaseSpread, DamageType, DotProfile, FatalBias, FireMode, Handedness, Kickback, Shove,
    Stable, TrajectoryStyle, Weapon, WeaponBraceBonus, WeaponDamage, WeaponName, WeaponPunch,
    WeaponShred,
};
use crate::magazine::Magazine;

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
    /// The GTW-549 per-item **brace bonus** attachment — `Some` when a data-driven
    /// [`Stability`](super::AttachmentEffect::Stability) attachment fitted a
    /// [`WeaponBraceBonus`] component, feeding its graduated additive §1a stability
    /// contribution into the cone read; `None` for a weapon with no brace attachment (the
    /// zero-identity term, byte-identical to before the attachment). An `Option` because the
    /// component is present only when a brace attachment is fitted (unlike the always-present
    /// [`Stable`] tag). SUPERSEDES the GTW-542 sight-stability seam — a sight now boosts AIM
    /// (the [`Accuracy`] stat), not stability.
    pub brace_bonus: Option<&'a WeaponBraceBonus>,
    /// The GTW-544 optional **damage-over-time profile** — `Some` when the weapon carries a
    /// [`DotProfile`] sibling (a DOT weapon), feeding the fold's DOT-attach decision: a hit
    /// that PENETRATES armor attaches (or REFRESHES) a [`Dot`](super::Dot) on the struck
    /// ganger built from this profile. `None` for a non-DOT weapon (no attach, byte-identical
    /// to before this slice). An `Option` because the [`DotProfile`] sibling is present only
    /// on a DOT weapon (like the optional `Silenced` attachment tag).
    pub dot:         Option<&'a DotProfile>,
}

/// The **spawn bundle for an armed entity** — the [`Weapon`] marker plus the full
/// set of weapon stat components, inserted together (GTW-200).
///
/// A Bevy [`Bundle`] so spawning an armed (ganger) entity carries the marker and
/// every stat component in one `commands.spawn(...)` / `entity.insert(...)` call.
/// The ammo state is the [`crate::magazine::Magazine`] grouping component (GTW-275:
/// the [`MagazineSize`](super::MagazineSize) capacity, the per-weapon
/// [`ReloadTu`](crate::magazine::ReloadTu), and the live
/// [`LoadedRounds`](crate::magazine::LoadedRounds) count) — spawned FULL by
/// [`WeaponSpec::into_bundle`](super::WeaponSpec::into_bundle). Every field is a
/// weapon-number newtype, the [`Magazine`] grouping, the [`WeaponName`], or the marker
/// (no bare primitive); build one with [`WeaponBundle::new`].
///
/// **Not `Copy`** — it carries a [`WeaponName`] ([`String`]) and a [`FireMode`]
/// (which holds a `Vec`); the bundle is `Clone`.
#[derive(Bundle, Debug, Clone, PartialEq)]
pub struct WeaponBundle {
    /// The [`Weapon`] marker tagging the entity as armed.
    pub marker:      Weapon,
    /// The weapon's human-facing name (its own sibling component).
    pub name:        WeaponName,
    /// The intrinsic angular spread before situational multipliers (`base_spread`).
    pub base_spread: BaseSpread,
    /// The concentration weapon term (`accuracy`; may exceed 1.0).
    pub accuracy:    Accuracy,
    /// The per-round recoil added in a burst (`kickback`).
    pub kickback:    Kickback,
    /// The severity-score addend, consumed by E3 (`fatal_bias`).
    pub fatal_bias:  FatalBias,
    /// The base damage a hit deals before armor (`damage`).
    pub damage:      WeaponDamage,
    /// The armor protection a hit ignores — penetration (`punch`).
    pub punch:       WeaponPunch,
    /// The extra integrity damage a hit deals to armor durability (`shred`).
    pub shred:       WeaponShred,
    /// The damage type the weapon emits — its matchup-wheel node.
    pub damage_type: DamageType,
    /// The ammo state — the [`Magazine`] grouping (capacity + per-weapon reload cost +
    /// the live loaded-rounds count, spawned full).
    pub magazine:    Magazine,
    /// The authored fire-mode selector and its per-mode numbers.
    pub fire_mode:   FireMode,
    /// The `stable` tag — `true` engages the §1a brace bonus unconditionally.
    pub stable:      Stable,
    /// The `shove` tag (GTW-525) — `true` KNOCKS BACK the target one cell on a
    /// connecting ranged shot (in addition to the shot's damage). `false` is a
    /// normal weapon (no knockback).
    pub shove:       Shove,
    /// The weapon's [`Handedness`] (GTW-443) — `OneHanded` / `TwoHanded`; the shared
    /// `can_fire` guard refuses a `TwoHanded` weapon below two available hands.
    pub handedness:  Handedness,
    /// The weapon's [`TrajectoryStyle`] (GTW-546) — `Straight` (the default, a flat ray) or
    /// `Arc` (a lobbed grenade parabola). The throw / fire path reads it to pick the straight
    /// [`march_vector`](crate::march::march_vector) or the parabolic
    /// [`march_arc`](crate::march::march_arc). Spawned from the spec's `#[serde(default)]`
    /// `trajectory` field, so every existing weapon carries `Straight`.
    pub trajectory:  TrajectoryStyle,
}

/// A weapon's **damage block** for spawning — the three per-hit damage numbers plus
/// the emitted [`DamageType`] (`weapons-and-armor.md` §"Weapon stats" / matchup.md
/// §"The 7 types"), handed to [`WeaponBundle::new`] as one cohesive value.
///
/// An owned ctor-input grouping (the [`FireModeSpec`](super::FireModeSpec)
/// precedent: related data travels as one named record, not a loose tuple) so
/// [`WeaponBundle::new`] stays under clippy's argument-count gate. Distinct from the
/// borrow-view [`WeaponStats`]: this owns its four newtypes (spawn-side input), the
/// view borrows the live components (read-side). Every field is a weapon-number
/// newtype.
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

/// A weapon's **handling block** for spawning — its [`Magazine`] grouping (capacity +
/// per-weapon reload cost + the spawn-full loaded count), authored [`FireMode`]
/// selector, and [`Stable`] tag, handed to [`WeaponBundle::new`] as one cohesive
/// value.
///
/// An owned ctor-input grouping (the [`DamageProfile`] /
/// [`FireModeSpec`](super::FireModeSpec) precedent) so [`WeaponBundle::new`] stays
/// under clippy's argument-count gate. Every field is a weapon-number grouping / the
/// [`FireMode`] selector.
///
/// **Not `Copy`** — it owns a [`FireMode`] (which holds a `Vec` of specs); it is
/// `Clone`.
#[derive(Debug, Clone, PartialEq)]
pub struct HandlingProfile {
    /// The ammo state — the [`Magazine`] grouping (capacity + reload cost + loaded
    /// count).
    pub magazine:   Magazine,
    /// The authored fire-mode selector and its per-mode numbers.
    pub fire_mode:  FireMode,
    /// The `stable` tag — `true` engages the §1a brace bonus unconditionally.
    pub stable:     Stable,
    /// The `shove` tag (GTW-525) — `true` knocks the target back one cell on a
    /// connecting shot.
    pub shove:      Shove,
    /// The weapon's [`Handedness`] (GTW-443).
    pub handedness: Handedness,
    /// The weapon's [`TrajectoryStyle`] (GTW-546) — `Straight` (the default) or `Arc` (a
    /// lobbed grenade).
    pub trajectory: TrajectoryStyle,
}

impl HandlingProfile {
    /// Build a handling block from a weapon's [`Magazine`] grouping, fire-mode
    /// selector, `stable` tag, `shove` tag (GTW-525), and [`Handedness`] (GTW-443).
    ///
    /// The [`TrajectoryStyle`] (GTW-546) defaults to [`TrajectoryStyle::Straight`] (a flat
    /// ray — every existing weapon), OVERRIDDEN by [`with_trajectory`](HandlingProfile::with_trajectory)
    /// for a lobbed grenade. Keeping the base ctor's arity unchanged means the many existing
    /// callers (tests + the spawn seam) spawn byte-identical `Straight` weapons untouched.
    #[must_use]
    pub const fn new(
        magazine: Magazine,
        fire_mode: FireMode,
        stable: Stable,
        shove: Shove,
        handedness: Handedness,
    ) -> Self {
        Self {
            magazine,
            fire_mode,
            stable,
            shove,
            handedness,
            trajectory: TrajectoryStyle::Straight,
        }
    }

    /// Set this handling block's [`TrajectoryStyle`] (GTW-546) — the builder a grenade /
    /// grenade-launcher spec calls to lob its charge (`Arc`), leaving the default `Straight`
    /// weapons untouched. Consumes and returns `self` (the builder idiom), so it chains off
    /// [`new`](HandlingProfile::new) without rippling that ctor's arity into every call site.
    #[must_use]
    pub const fn with_trajectory(mut self, trajectory: TrajectoryStyle) -> Self {
        self.trajectory = trajectory;
        self
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
            magazine: handling.magazine,
            fire_mode: handling.fire_mode,
            stable: handling.stable,
            shove: handling.shove,
            handedness: handling.handedness,
            trajectory: handling.trajectory,
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
            // A WeaponBundle carries no brace attachment (the WeaponBraceBonus component is
            // applied post-spawn by the GTW-549 attachment extension), so a bundle-derived
            // view is always un-braced — the zero-identity brace term.
            brace_bonus: None,
            // A WeaponBundle carries no DOT profile (the DotProfile sibling is spawned
            // separately from the spec's `dot` field, GTW-544), so a bundle-derived view
            // never attaches a Dot — a non-DOT read, byte-identical to before the slice.
            dot:         None,
        }
    }
}
