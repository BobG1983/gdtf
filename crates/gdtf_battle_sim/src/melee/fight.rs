//! The §7 opposed-Fight resolution **core** — the deterministic melee math
//! (GTW-506).
//!
//! `docs/combat/resolution.md` §7 (lines 146-150) specifies the math exactly:
//!
//! ```text
//! atk = Fight_attacker × roll        (roll ∈ [1−v, 1+v], variance v = tunable)
//! def = Fight_defender × roll
//! connect if atk > def
//! margin      = atk / def − 1        (relative dominance — scale-independent, unbounded)
//! damage_mult = clamp(mult_min + k_margin × margin, mult_min, mult_max)
//! ```
//!
//! A connecting hit's resolved damage is **multiplied** by `damage_mult`, then
//! runs the normal §5 damage → §6 wound steps. `margin` is **relative**
//! (`atk/def − 1`), so it is scale-independent and rewards a genuine skill gap.
//!
//! This module is **pure functions over plain data + injected seeded RNG** — no
//! systems, no `&mut World`, no ECS trigger. The variance and curve coefficients
//! it reads live in [`MeleeTuning`](crate::tuning::MeleeTuning) (GTW-506); the
//! roll draws come from the dedicated [`FightRng`](crate::rng::FightRng) stream
//! (a DEDICATED stream so melee and reaction-fire determinism stay isolated). The
//! live melee ACT (adjacency / LOS gate / input / presenter) is GTW-507.

use bevy::prelude::Deref;

use crate::{
    ganger::Fight,
    resolve_hit::{HitResult, HpDamage, IntegrityWear, PenetratingDamage},
    rng::FightRng,
    tuning::{FightVariance, MeleeTuning},
};

// ── Output newtypes ───────────────────────────────────────────────────────────

/// The **relative dominance** of an opposed-Fight exchange — the §7
/// `margin = atk / def − 1` (`docs/combat/resolution.md` §7 line 149).
///
/// Scale-independent and **unbounded**: a positive margin means the attacker's
/// rolled Fight beat the defender's (the bigger the gap, the harder the blow,
/// before the clamp); `0.0` means a dead tie; a negative margin means the
/// attacker LOST the exchange (no connect). It is the sole input to
/// [`melee_damage_mult`]. A distinct domain newtype (no-bare-types) so a raw
/// dominance ratio is never confused with a damage multiplier or a Fight stat. A
/// domain math value — **zero pixels**; private inner + derived [`Deref`].
#[derive(Deref, Debug, Clone, Copy, PartialEq)]
pub struct FightMargin(f32);

impl FightMargin {
    /// Build a fight margin from its computed magnitude.
    ///
    /// The constructor (house style) — keeps the inner `f32` private so a margin
    /// is only produced by [`opposed_fight`] (or this ctor in a test), never a
    /// bare `atk/def − 1` masquerading as a margin.
    #[must_use]
    pub const fn new(margin: f32) -> Self {
        Self(margin)
    }
}

/// The **damage multiplier** a connecting melee hit applies — the §7
/// `clamp(mult_min + k_margin × margin, mult_min, mult_max)`
/// (`docs/combat/resolution.md` §7 line 150).
///
/// Always lies in `[mult_min, mult_max]` once produced by [`melee_damage_mult`].
/// May be **below 1** (a glancing connect, resolution.md §7 line 153). The melee
/// hit pipeline scales the resolved [`HitResult`] by this factor via
/// [`apply_melee_multiplier`] before the §6 wound step. A distinct newtype from
/// [`FightMargin`] (no-bare-types rule 3): a margin is an unbounded relative
/// dominance, a multiplier is a bounded `[mult_min, mult_max]` scalar — never
/// interchangeable. A domain math value — **zero pixels**; private inner +
/// derived [`Deref`].
#[derive(Deref, Debug, Clone, Copy, PartialEq)]
pub struct MeleeDamageMult(f32);

impl MeleeDamageMult {
    /// Build a melee damage multiplier from its computed magnitude.
    ///
    /// The constructor (house style) — keeps the inner `f32` private so a
    /// multiplier is only produced by [`melee_damage_mult`] (or this ctor in a
    /// test), never a bare unclamped product.
    #[must_use]
    pub const fn new(mult: f32) -> Self {
        Self(mult)
    }
}

/// The **frozen outcome** of one §7 opposed-Fight exchange — whether the attack
/// connects and by what relative margin (`docs/combat/resolution.md` §7 lines
/// 148-149).
///
/// Pure data produced by [`opposed_fight`]: a `Copy` value object of a
/// `connect` flag plus the [`FightMargin`]. The caller feeds the margin to
/// [`melee_damage_mult`] only when `connect` is `true`; on a miss the margin is
/// still defined (negative or zero) but unused. No bare primitive — the margin is
/// wrapped (no-bare-types); the `connect` flag is structural framework plumbing,
/// not a domain value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FightOutcome {
    /// `true` iff `atk > def` — the attack landed (resolution.md §7 line 148).
    pub connect: bool,
    /// The relative dominance `atk / def − 1` (resolution.md §7 line 149).
    pub margin:  FightMargin,
}

// ── The degenerate-margin constant ─────────────────────────────────────────────

/// The defined [`FightMargin`] returned when the defender's effective Fight is
/// `≤ 0` (a div-by-zero would otherwise produce NaN/inf).
///
/// A large FINITE positive value — so the attack `connect`s and
/// [`melee_damage_mult`] clamps the multiplier to `mult_max` (a defenceless
/// target eats the maximum blow). Chosen as `1e6`: comfortably above any sane
/// `mult_max / k_margin` ratio so the clamp always saturates, yet finite (passes
/// the `is_finite()` degenerate-case test — no NaN/inf, no panic). See
/// [`opposed_fight`]'s degenerate-case doc.
const DEGENERATE_MARGIN: f32 = 1.0e6;

// ── Pure functions ─────────────────────────────────────────────────────────────

/// Resolve the §7 **opposed-Fight** exchange — does the attack connect, and by
/// what relative margin? (`docs/combat/resolution.md` §7 lines 146-149.)
///
/// Draws **exactly two** uniform rolls from the injected [`FightRng`] in a fixed
/// deterministic order — the **attacker's roll first, then the defender's** —
/// each uniform in `[1 − v, 1 + v]` for the [`FightVariance`] `v` from
/// [`MeleeTuning`]. Then `atk = Fight_attacker × roll_atk`,
/// `def = Fight_defender × roll_def`, `connect = atk > def`, and
/// `margin = atk / def − 1`. The band's bounds are TUNABLE (hot-reloadable), so
/// both rolls go through the stream's safe-draw verb
/// ([`FightRng::random_range_or_midpoint`](crate::rng::FightRng)): a DEGENERATE
/// band (`v <= 0.0` — documented-legal tuning, GTW-640) never panics — each roll
/// factor collapses to the band's midpoint `1.0` while STILL consuming its draw
/// (draw-count stability, GTW-644), so the exchange resolves as a pure
/// `Fight_attacker` vs `Fight_defender` comparison and the stream stays aligned.
///
/// Two draws in a fixed order keep the stream deterministically replayable: two
/// [`FightRng`]s from the same [`BattleSeed`](crate::rng::BattleSeed) yield the
/// same [`FightOutcome`] sequence for the same inputs. Drawing advances the stream
/// cursor — the caller (GTW-507's live act) owns the `ResMut<FightRng>` and passes
/// it here by `&mut` (this pure fn never holds a `Res`/`ResMut` itself).
///
/// # Degenerate case
///
/// When the defender's effective Fight `def ≤ 0.0` (e.g. a defender Fight of `0`,
/// which defaults so for an un-derived sheet) the `atk / def` margin would divide
/// by zero. By rule this returns a DEFINED outcome: `connect = true` with a large
/// FINITE `margin` (the `DEGENERATE_MARGIN` const), so [`melee_damage_mult`] clamps
/// the multiplier to `mult_max` — a defenceless target eats the maximum blow. No
/// `0/0` or `x/0` NaN/inf, no panic. (A negative `def` cannot arise from a
/// non-negative Fight × a non-negative roll, but `≤ 0.0` guards it belt-and-braces.)
///
/// # Arguments
///
/// - `attacker` — the attacker's [`Fight`] computed stat.
/// - `defender` — the defender's [`Fight`] computed stat.
/// - `variance` — the [`FightVariance`] `v` from [`MeleeTuning`].
/// - `rng` — the injected seeded [`FightRng`], borrowed mutably for the two draws.
#[must_use]
pub fn opposed_fight(
    attacker: Fight,
    defender: Fight,
    variance: FightVariance,
    rng: &mut FightRng,
) -> FightOutcome {
    // Two draws in a FIXED order — attacker roll first, then defender roll — so
    // the stream advances identically on every replay. Each roll is uniform in
    // [1 − v, 1 + v]; the band is TUNABLE-driven, so each roll takes the safe-draw
    // verb: a degenerate band (v <= 0.0, GTW-640) collapses the factor to the
    // midpoint 1.0 while still consuming the draw (GTW-644 — never a panic, never
    // a skipped draw).
    let lo = 1.0 - *variance;
    let hi = 1.0 + *variance;
    let roll_atk: f32 = rng.random_range_or_midpoint(lo..hi);
    let roll_def: f32 = rng.random_range_or_midpoint(lo..hi);

    let atk = *attacker * roll_atk;
    let def = *defender * roll_def;

    // DEGENERATE: def <= 0.0 would make atk/def a div-by-zero (NaN/inf). Defined
    // fallback: a connecting hit at a large FINITE margin → mult_max after the
    // clamp. (A defenceless target eats the maximum blow.)
    if def <= 0.0 {
        return FightOutcome {
            connect: true,
            margin:  FightMargin::new(DEGENERATE_MARGIN),
        };
    }

    FightOutcome {
        connect: atk > def,
        // margin = atk/def − 1 (resolution.md §7 line 149). Relative, unbounded.
        margin:  FightMargin::new(atk / def - 1.0),
    }
}

/// The §7 **melee damage multiplier**
/// `clamp(mult_min + k_margin × margin, mult_min, mult_max)`
/// (`docs/combat/resolution.md` §7 line 150).
///
/// Maps a connecting hit's [`FightMargin`] to the multiplier the resolved damage
/// is scaled by. Monotone non-decreasing in `margin` (a bigger skill gap hits
/// harder, up to the clamp). The result is pinned to `[mult_min, mult_max]`, so it
/// can be **below 1** (a glancing connect at a tiny positive margin) when
/// `mult_min < 1`, and never exceeds `mult_max`. The huge degenerate margin from a
/// defenceless defender (see [`opposed_fight`]) saturates to `mult_max`.
///
/// # Arguments
///
/// - `margin` — the [`FightMargin`] from [`opposed_fight`] (only meaningful on a
///   connecting hit; the caller gates on [`FightOutcome::connect`]).
/// - `tuning` — the [`MeleeTuning`] group from [`crate::tuning::CombatTuning`].
#[must_use]
pub fn melee_damage_mult(margin: FightMargin, tuning: &MeleeTuning) -> MeleeDamageMult {
    // FMA form: k_margin × margin + mult_min (suboptimal_flops lint), then clamp.
    let raw = (*tuning.k_margin).mul_add(*margin, *tuning.mult_min);
    MeleeDamageMult::new(raw.clamp(*tuning.mult_min, *tuning.mult_max))
}

/// Round an `f32` product to the nearest `i32`, clamped into the `i32` range so a
/// wild value can never wrap on the cast.
///
/// The melee multiplier scales the resolved damage in `f32`; the [`HitResult`]
/// then works in `i32`. Uses round-half-away-from-zero ([`f32::round`]) — matching
/// the [`resolve_hit`](crate::resolve_hit::resolve_hit) `round_to_i32` idiom. The
/// clamp + localized `#[expect]` is the crate's guarded-cast idiom, so no
/// `unwrap`/`expect` is needed.
const fn round_to_i32(value: f32) -> i32 {
    let rounded = value.round();
    #[expect(
        clippy::cast_possible_truncation,
        reason = "clamped to the i32 range below, so the cast cannot wrap; fractional part is gone after round"
    )]
    let clamped = rounded.clamp(i32::MIN as f32, i32::MAX as f32) as i32;
    clamped
}

/// Scale one `i32` damage component by the melee multiplier and round back to
/// `i32`.
///
/// Computed in `f32` (`component × *mult`) then rounded via [`round_to_i32`]. The
/// shared helper for the two damage components [`apply_melee_multiplier`] scales.
#[expect(
    clippy::cast_precision_loss,
    reason = "i32 damage → f32 for the melee multiply; resolved-damage magnitudes are far inside f32's exact-integer range"
)]
fn scale_damage(component: i32, mult: MeleeDamageMult) -> i32 {
    round_to_i32(component as f32 * *mult)
}

/// Apply the §7 [`MeleeDamageMult`] to a resolved [`HitResult`] — the seam between
/// [`resolve_hit`](crate::resolve_hit::resolve_hit) and the §6 wound step
/// (`docs/combat/resolution.md` §7 line 153: "a connecting hit's damage is
/// multiplied by `damage_mult`, then runs the normal damage → wound steps").
///
/// Pure (no ECS): returns a NEW [`HitResult`] with the damage scaled, mutating
/// nothing. GTW-507's live act calls this as
/// `resolve_hit(…) → apply_melee_multiplier(hit, damage_mult) → §6 wound` — this
/// child only provides the helper, it does NOT modify
/// [`resolve_hit`](crate::resolve_hit::resolve_hit) or the fold.
///
/// ## What is scaled (the integrity-wear decision)
///
/// - [`HpDamage`] — scaled. It is the HP-loss the §6 step and E3.6 spend; a
///   harder blow takes more HP.
/// - [`PenetratingDamage`] — scaled. It **gates** §6 wound severity, so scaling it
///   is what lets a dominant blow reach a worse wound bucket (the whole point of
///   §7: "hits much harder"). Scaling HP-loss WITHOUT penetration would change the
///   HP number but never the wound severity, defeating the design.
/// - [`IntegrityWear`] — **scaled too** (the chosen default). The §7 multiplier is
///   the force of the blow; a harder strike wears the struck armour proportionally
///   harder, consistent with scaling both damage components. (The alternative —
///   leaving wear unscaled — would make the blow's force visible in HP and wound
///   severity but NOT in armour degradation, an inconsistency; so wear is scaled.)
///
/// # Arguments
///
/// - `hit` — the frozen [`HitResult`] from
///   [`resolve_hit`](crate::resolve_hit::resolve_hit) (ranged-style armour/damage math).
/// - `mult` — the [`MeleeDamageMult`] from [`melee_damage_mult`].
#[must_use]
pub fn apply_melee_multiplier(hit: HitResult, mult: MeleeDamageMult) -> HitResult {
    HitResult {
        penetrating: PenetratingDamage::new(scale_damage(*hit.penetrating, mult)),
        hp_damage:   HpDamage::new(scale_damage(*hit.hp_damage, mult)),
        wear:        IntegrityWear::new(scale_damage(*hit.wear, mult)),
    }
}
