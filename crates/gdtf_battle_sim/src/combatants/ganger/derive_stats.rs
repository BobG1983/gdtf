//! [`derive_stats`] — the SINGLE SOURCE OF TRUTH that turns a ganger's eight authored
//! [`GangerAttributes`] into the [`DerivedStats`] computed combat stats, via the
//! [`GangerStatTuning`] weights (`docs/combat/stats.md` §"Computed combat stats",
//! GTW-384).
//!
//! Render-free, deterministic, and PURE (no RNG, no entity-iteration order) so it is
//! unit-testable in isolation — and so the setup-time derivation and the hot-reload
//! re-derivation share ONE implementation. The computed stats are NEVER authored: the
//! situation authors attributes only, this derives the rest.
//!
//! **Rounding rule (documented + consistent).** The weighted attribute sums are `f32`,
//! but the pools they seed are integers ([`Hp`] `u16`, [`Wounds`] / [`Tu`] / [`Bottle`]
//! `u8`). Every f32 → integer crossing uses **round-to-nearest** (`f32::round`), clamped
//! into the target integer's range (the crate's sanctioned no-`unwrap` cast idiom — a
//! localized `#[expect]` stating why the cast cannot wrap). [`Shooting`] / [`Fight`] /
//! [`Reactions`] / [`Morale`] stay `f32` (skill stats, not pools), so they carry the raw
//! weighted sum with no rounding.

use bevy::prelude::Deref;

use crate::{
    ganger::{
        Bottle, Fight, Hp, HpMax, Morale, Reactions, Shooting, Tu, TuMax, Wounds, WoundsMax,
        attributes::GangerAttributes,
    },
    tuning::GangerStatTuning,
};

/// A computed combat stat's **raw real magnitude** — the `f32` weighted-attribute
/// sum a skill stat carries or a pool derivation rounds, before it is committed to
/// its typed field (`docs/combat/stats.md` §"Computed combat stats", GTW-384).
///
/// The shared real intermediate every derivation produces: the skill stats
/// ([`Shooting`] / [`Fight`] / [`Reactions`] / [`Morale`]) store it verbatim, the
/// integer pools round it via [`round_to_u16`] / [`round_to_u8`]. Names the
/// pre-storage magnitude so a derivation helper does not carry a bare `f32`
/// (no-bare-types). Private inner + derived [`Deref`]; the `const fn` rounding
/// helpers read the inner directly (same module) since derived `Deref` is not
/// `const`.
#[derive(Deref, Debug, Clone, Copy, PartialEq)]
pub(crate) struct StatMagnitude(f32);

impl StatMagnitude {
    /// Build a stat magnitude from its computed real value.
    #[must_use]
    pub(crate) const fn new(magnitude: f32) -> Self {
        Self(magnitude)
    }
}

/// The **integer value a small pool derivation rounds to** — the `u8` count a
/// [`Tu`] / [`Wounds`] / [`Bottle`] (or their maxes) derivation yields before the
/// caller commits it to that specific pool type.
///
/// A transient rounding result shared by the three `u8` pool derivations (each stays
/// its own distinct stored type — [`Tu`] / [`Wounds`] / [`Bottle`] — this only names
/// the rounded count on the way there). Names it so [`round_to_u8`] does not return a
/// bare `u8` (no-bare-types). Private inner + derived [`Deref`].
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PoolValue(u8);

impl PoolValue {
    /// Build a pool value from its rounded count.
    #[must_use]
    pub(crate) const fn new(value: u8) -> Self {
        Self(value)
    }
}

/// The full set of **computed combat stats** [`derive_stats`] yields for one ganger
/// (`docs/combat/stats.md` §"Computed combat stats", GTW-384).
///
/// Carries the live stats ([`Shooting`] / [`Tu`] + the [`Hp`] / [`Wounds`] pools and
/// their maxes) AND the designed-dormant ones ([`Fight`] / [`Reactions`] / [`Morale`] /
/// [`Bottle`]). The MAXES equal their current pools (full at battle start — the
/// "current pool spawns == max" contract); a [`setup_battle`](crate::situation::setup_battle)
/// caller spawns each field as the matching component, and the hot-reload re-derive
/// overwrites the skill stats + maxes while CLAMPING the (possibly damaged) current pools.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DerivedStats {
    /// `Shooting = aim·Aim + reflexes·Reflexes + cool·Cool` — feeds the §1b shot
    /// concentration (LIVE).
    pub shooting:   Shooting,
    /// `Fight = speed·Speed + strength·Strength + grit·Grit + cool·Cool` — melee skill
    /// (DORMANT, GTW-51).
    pub fight:      Fight,
    /// `Reactions = speed·Speed + reflexes·Reflexes + cool·Cool` — enemy-turn responses
    /// (DORMANT, GTW-38).
    pub reactions:  Reactions,
    /// `Morale = grit·Grit + cool·Cool` — the psychological damage pool (DORMANT, GTW-13).
    pub morale:     Morale,
    /// `Tu = round(tu_base + tu_per_speed·Speed)` — the per-turn action budget (LIVE).
    pub tu:         Tu,
    /// The round-start TU ceiling — equals the derived [`Tu`] (full at battle start).
    pub tu_max:     TuMax,
    /// `Hp = round(grit·Grit + toughness·Toughness + cool·Cool)` — the in-battle
    /// knock-down pool (LIVE).
    pub hp:         Hp,
    /// The HP ceiling — equals the derived [`Hp`] (full at battle start).
    pub hp_max:     HpMax,
    /// `Wounds = round(Hp / wounds_per_hp)` — the life pool (LIVE).
    pub wounds:     Wounds,
    /// The Wounds ceiling — equals the derived [`Wounds`] (full at battle start).
    pub wounds_max: WoundsMax,
    /// `Bottle = round(Morale / bottle_per_morale)` — the psychological life pool
    /// (DORMANT, GTW-13).
    pub bottle:     Bottle,
}

/// Round a non-negative `f32` to the nearest `u16`, clamped into range — the pool
/// rounding rule for [`Hp`] (`docs/combat/stats.md`, GTW-384).
const fn round_to_u16(value: StatMagnitude) -> Hp {
    let rounded = value.0.round();
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "clamped to the u16 range below, so the cast cannot wrap or go negative; \
                  the fractional part is gone after round"
    )]
    let clamped = rounded.clamp(0.0, u16::MAX as f32) as u16;
    Hp::new(clamped)
}

/// The weighted-attribute sum `Σ weightᵢ · attributeᵢ` — the SHAPE of every skill / pool
/// derivation (`docs/combat/stats.md` §"weighted attribute sums", GTW-384).
///
/// Each `(weight, attribute)` term is fused into the running total via
/// [`f32::mul_add`] (a single fused multiply-add — faster + more accurate, and the form
/// clippy's `suboptimal_flops` lint requires over a bare `w*a + …`). Made `pub(crate)`
/// so the C8(a) derivation-relation test computes its EXPECTED value through the SAME
/// fused chain — bit-identical, so the relation assert is exact (not tolerance-based).
#[must_use]
pub(crate) fn weighted_sum(terms: &[(f32, f32)]) -> StatMagnitude {
    StatMagnitude::new(terms.iter().fold(0.0, |acc, &(weight, attribute)| {
        weight.mul_add(attribute, acc)
    }))
}

/// Round a non-negative `f32` to the nearest `u8`, clamped into range — the pool
/// rounding rule for the tiny pools ([`Wounds`] / [`Tu`] / [`Bottle`]).
const fn round_to_u8(value: StatMagnitude) -> PoolValue {
    let rounded = value.0.round();
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "clamped to the u8 range below, so the cast cannot wrap or go negative; \
                  the fractional part is gone after round"
    )]
    let clamped = rounded.clamp(0.0, u8::MAX as f32) as u8;
    PoolValue::new(clamped)
}

/// **Derive** the computed combat stats from the eight authored attributes × the
/// [`GangerStatTuning`] weights (`docs/combat/stats.md` §"Computed combat stats",
/// GTW-384) — the single source of truth used by BOTH the setup-time spawn and the
/// hot-reload re-derive.
///
/// The formulas, exactly as stats.md specifies:
///
/// - `Shooting  = aim·Aim + reflexes·Reflexes + cool·Cool`
/// - `Fight     = speed·Speed + strength·Strength + grit·Grit + cool·Cool`
/// - `Reactions = speed·Speed + reflexes·Reflexes + cool·Cool`
/// - `Morale    = grit·Grit + cool·Cool`
/// - `Tu        = round(tu_base + tu_per_speed·Speed)`
/// - `Hp        = round(grit·Grit + toughness·Toughness + cool·Cool)`
/// - `Wounds    = round(Hp / wounds_per_hp)` (a 2nd derivation level, off the ROUNDED Hp)
/// - `Bottle    = round(Morale / bottle_per_morale)`
///
/// The skill stats ([`Shooting`] / [`Fight`] / [`Reactions`] / [`Morale`]) keep the raw
/// `f32` weighted sum; the pools round to their integer types (round-to-nearest). Every
/// max equals its current pool (full at battle start). Pure — no RNG, no global state.
#[must_use]
pub fn derive_stats(attributes: &GangerAttributes, tuning: &GangerStatTuning) -> DerivedStats {
    // Deref every attribute + weight once into a local f32 (the newtypes Deref to f32);
    // the const cast helpers can't Deref a derived-Deref newtype, so unwrap here.
    let speed = *attributes.speed;
    let aim = *attributes.aim;
    let strength = *attributes.strength;
    let toughness = *attributes.toughness;
    let reflexes = *attributes.reflexes;
    let cool = *attributes.cool;
    let grit = *attributes.grit;

    // Each weighted sum is expressed via [`weighted_sum`] (an `f32::mul_add` chain), so the
    // single helper IS the formula the derivation and the C8(a) relation test both compute
    // through (bit-identical, and clippy-clean — the bare `w*a + …` form trips the
    // `suboptimal_flops`/`mul_add` lint).
    //
    // Shooting = aim·Aim + reflexes·Reflexes + cool·Cool (the §1b concentration skill term).
    let shooting = weighted_sum(&[
        (*tuning.shooting.aim, aim),
        (*tuning.shooting.reflexes, reflexes),
        (*tuning.shooting.cool, cool),
    ]);

    // Fight = speed·Speed + strength·Strength + grit·Grit + cool·Cool (melee skill, dormant).
    let fight = weighted_sum(&[
        (*tuning.fight.speed, speed),
        (*tuning.fight.strength, strength),
        (*tuning.fight.grit, grit),
        (*tuning.fight.cool, cool),
    ]);

    // Reactions = speed·Speed + reflexes·Reflexes + cool·Cool (enemy-turn responses, dormant).
    let reactions = weighted_sum(&[
        (*tuning.reactions.speed, speed),
        (*tuning.reactions.reflexes, reflexes),
        (*tuning.reactions.cool, cool),
    ]);

    // Morale = grit·Grit + cool·Cool (psychological damage pool, dormant).
    let morale = weighted_sum(&[(*tuning.morale.grit, grit), (*tuning.morale.cool, cool)]);

    // TU = tu_base + tu_per_speed·Speed (the action budget; rounded to the u8 pool).
    let tu_f = (*tuning.tu_per_speed).mul_add(speed, *tuning.tu_base);
    let tu = round_to_u8(StatMagnitude::new(tu_f));

    // HP = grit·Grit + toughness·Toughness + cool·Cool (the knock-down pool; Cool ~0.5 by default).
    let hp = round_to_u16(weighted_sum(&[
        (*tuning.hp.grit, grit),
        (*tuning.hp.toughness, toughness),
        (*tuning.hp.cool, cool),
    ]));

    // Wounds = round(HP / wounds_per_hp) — the second derivation level, off the ROUNDED hp
    // so the displayed knock-down pool and the divided life pool agree.
    let wounds = round_to_u8(StatMagnitude::new(f32::from(*hp) / *tuning.wounds_per_hp));

    // Bottle = round(Morale / bottle_per_morale) — the psychological mirror of Wounds.
    let bottle = round_to_u8(StatMagnitude::new(*morale / *tuning.bottle_per_morale));

    DerivedStats {
        shooting: Shooting::new(*shooting),
        fight: Fight::new(*fight),
        reactions: Reactions::new(*reactions),
        morale: Morale::new(*morale),
        tu: Tu::new(*tu),
        tu_max: TuMax::new(*tu),
        hp,
        hp_max: HpMax::new(*hp),
        wounds: Wounds::new(*wounds),
        wounds_max: WoundsMax::new(*wounds),
        bottle: Bottle::new(*bottle),
    }
}
