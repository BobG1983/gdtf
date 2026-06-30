//! The deterministic reaction-fire opposed-check **core** (GTW-467).
//!
//! `docs/combat/resolution.md` §8 (lines 160-162) specifies the math exactly:
//!
//! ```text
//! score        = Reactions × (TU_left / TU_max)
//! P(interrupt) = score_watcher / (score_watcher + score_mover)
//! max interrupts this enemy turn = cap(Reactions)        (tunable)
//! ```
//!
//! and the §8 rules (line 165): a probability clamp (`p_min`/`p_max`) so the
//! extremes are never an absolute 0%/100% — **no ganger is ever hard-locked
//! out** — a mover who spends MORE TU becomes EASIER to interrupt (their score
//! falls), and hoarding TU keeps you dangerous; per-turn interrupts are capped
//! by the watcher's `Reactions`.
//!
//! This module is **pure functions over plain data + injected seeded RNG** — no
//! systems, no `&mut World`, no ECS trigger. The cap and clamp tuning leaves it
//! reads live in [`super::leaves`] ([`reaction_cap`] / [`clamp_probability`],
//! GTW-466). The live trigger (interrupt an enemy acting in LOS, advance the
//! per-turn counter at the turn boundary) is GTW-468 — including the WIRING of
//! [`ReactionsUsed::reset`] to the turn boundary.

use bevy::prelude::{Component, Deref};

use super::leaves::{ReactionTuning, clamp_probability, reaction_cap};
use crate::ganger::{Reactions, Tu, TuMax};

// ── Output newtypes ───────────────────────────────────────────────────────────

/// A combatant's **reaction score** — the §8 opposed-check term
/// `Reactions × (TU_left / TU_max)` (`docs/combat/resolution.md` §8 line 160).
///
/// Both sides of the interrupt check (watcher and mover) reduce to this one
/// scalar: a higher `Reactions` and a fuller TU pool both raise it. It is the
/// numerator AND part of the denominator of [`interrupt_probability`], so it is
/// the single value the §8 probability is built from. Wrapping it (no-bare-types
/// rule) keeps a raw `f32` score from being confused with a probability or a raw
/// stat. A domain math value — **zero pixels**; private inner + derived
/// [`Deref`].
#[derive(Deref, Debug, Clone, Copy, PartialEq)]
pub struct ReactionScore(f32);

impl ReactionScore {
    /// Build a reaction score from its computed magnitude.
    ///
    /// The constructor (house style) — keeps the inner `f32` private so the only
    /// way a score is produced is [`reaction_score`] (or this ctor in a test),
    /// never a bare arithmetic result masquerading as a score.
    #[must_use]
    pub const fn new(score: f32) -> Self {
        Self(score)
    }
}

/// A reaction-fire **interrupt probability** — the clamped §8 opposed-check
/// result `P(interrupt) = score_watcher / (score_watcher + score_mover)`
/// (`docs/combat/resolution.md` §8 lines 161-162, after the line-165 clamp).
///
/// Always lies in `[p_min, p_max]` ⊂ `[0, 1]` once produced by
/// [`interrupt_probability`] — never an absolute 0 or 1, so no ganger is ever
/// hard-locked out of reacting. A distinct newtype from [`ReactionScore`]
/// (no-bare-types rule 3): a probability is a bounded `[0, 1]`-ish quantity, a
/// score is unbounded — never interchangeable. The roll
/// ([`rolls_interrupt`]) consumes it. A domain math value — **zero pixels**;
/// private inner + derived [`Deref`].
#[derive(Deref, Debug, Clone, Copy, PartialEq)]
pub struct ReactionProbability(f32);

impl ReactionProbability {
    /// Build an interrupt probability from its computed magnitude.
    ///
    /// The constructor (house style) — keeps the inner `f32` private so a
    /// probability is only produced by [`interrupt_probability`] (or this ctor in
    /// a test), never a bare unclamped ratio.
    #[must_use]
    pub const fn new(p: f32) -> Self {
        Self(p)
    }
}

/// A watcher's **interrupts used this enemy turn** — the per-turn counter the §8
/// cap gates (`docs/combat/resolution.md` §8: `max interrupts this enemy turn =
/// cap(Reactions)`).
///
/// Counts how many reaction shots a watcher has already taken on the current
/// enemy turn; [`may_interrupt`] compares it against [`reaction_cap`] to decide
/// whether another is allowed. A per-watcher per-turn count — a `u32` (the same
/// units as [`reaction_cap`]'s result), wrapped (no-bare-types) so it can never
/// be confused with any other count. A [`Component`] so GTW-468's live trigger
/// can carry it on the watcher entity; [`reset`](Self::reset) zeroes it at the
/// turn boundary — but WIRING that reset to the turn boundary is GTW-468, NOT
/// this ticket. Defaults to `0` (a fresh turn). Private inner + derived
/// [`Deref`]; **zero pixels**.
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ReactionsUsed(u32);

impl ReactionsUsed {
    /// Build an interrupts-used counter from a starting count.
    ///
    /// The constructor (house style) — keeps the inner `u32` private; GTW-468
    /// spawns the counter at `new(0)` (or via [`Default`]) and mutates it only
    /// through [`increment`](Self::increment) / [`reset`](Self::reset).
    #[must_use]
    pub const fn new(used: u32) -> Self {
        Self(used)
    }

    /// Count one more interrupt taken this turn (saturating).
    ///
    /// Called by GTW-468's live trigger after a reaction shot fires, so the next
    /// [`may_interrupt`] check sees the higher count. Saturating so an absurd
    /// count can never wrap to zero and silently re-open the cap.
    pub const fn increment(&mut self) {
        self.0 = self.0.saturating_add(1);
    }

    /// Zero the counter — the per-turn reset.
    ///
    /// At each enemy-turn boundary the watcher's used-count returns to zero so
    /// the §8 cap applies afresh. This is the pure reset BEHAVIOR; WIRING it to
    /// the turn boundary (the schedule placement / trigger) is GTW-468.
    pub const fn reset(&mut self) {
        self.0 = 0;
    }
}

// ── Pure functions ─────────────────────────────────────────────────────────────

/// The §8 **reaction score** `Reactions × (TU_left / TU_max)`
/// (`docs/combat/resolution.md` §8 line 160).
///
/// A fuller TU pool (`tu_left` close to `tu_max`) and a higher `Reactions` both
/// raise the score — so **hoarding TU keeps you dangerous** and spending TU
/// lowers your priority (a mover who has spent more TU has a lower `tu_left`,
/// hence a lower score, hence is easier to interrupt). Used for BOTH the watcher
/// and the mover term of [`interrupt_probability`].
///
/// # Degenerate case
///
/// `tu_max == 0` would divide by zero. By rule this returns [`ReactionScore`]
/// `(0.0)` — a combatant with a zero TU ceiling has no TU economy to fund or
/// fuel a reaction, so its score is zero. No NaN/inf, no panic.
///
/// # Arguments
///
/// - `reactions` — the combatant's [`Reactions`] computed stat.
/// - `tu_left` — the combatant's current [`Tu`] pool.
/// - `tu_max` — the combatant's round-start [`TuMax`] ceiling (the denominator).
#[must_use]
pub fn reaction_score(reactions: Reactions, tu_left: Tu, tu_max: TuMax) -> ReactionScore {
    // DEGENERATE: a zero TU ceiling has no economy → score 0.0 (no div-by-zero).
    if *tu_max == 0 {
        return ReactionScore::new(0.0);
    }
    let tu_fraction = f32::from(*tu_left) / f32::from(*tu_max);
    ReactionScore::new(*reactions * tu_fraction)
}

/// The §8 **interrupt probability**
/// `clamp(score_watcher / (score_watcher + score_mover))`
/// (`docs/combat/resolution.md` §8 lines 161-162, with the line-165 clamp).
///
/// A higher watcher score (more `Reactions`, more hoarded TU) raises the
/// probability; a higher mover score lowers it — so a mover who spends more TU
/// (a lower mover score) becomes EASIER to interrupt. The raw ratio is routed
/// through [`clamp_probability`] so the result is pinned to `[p_min, p_max]` and
/// never an absolute 0%/100% — **no ganger is ever hard-locked out**.
///
/// # Degenerate case
///
/// When both scores are zero the denominator `watcher + mover` is `0.0`, which
/// would yield `0.0 / 0.0 == NaN`. By rule this returns
/// `clamp_probability(0.5, tuning)` — with both sides equally (un)able to react,
/// the unbiased even-odds `0.5` is the defined fallback, then clamped into
/// `[p_min, p_max]`. No NaN/inf, no panic. (A negative denominator cannot arise:
/// scores are non-negative by construction in [`reaction_score`].)
///
/// # Arguments
///
/// - `watcher` — the watcher's [`ReactionScore`] (the numerator).
/// - `mover` — the moving enemy's [`ReactionScore`].
/// - `tuning` — the [`ReactionTuning`] group from [`crate::tuning::CombatTuning`].
#[must_use]
pub fn interrupt_probability(
    watcher: ReactionScore,
    mover: ReactionScore,
    tuning: &ReactionTuning,
) -> ReactionProbability {
    let denominator = *watcher + *mover;
    // DEGENERATE: both scores zero → 0/0 would be NaN. Defined fallback: even
    // odds (0.5), then clamped into [p_min, p_max]. <= 0.0 also guards any
    // theoretically-negative denominator (scores are non-negative, so this is
    // belt-and-braces — never produce NaN/inf).
    if denominator <= 0.0 {
        return ReactionProbability::new(clamp_probability(0.5, tuning));
    }
    let raw = *watcher / denominator;
    ReactionProbability::new(clamp_probability(raw, tuning))
}

/// Roll the §8 interrupt check against a seeded stream — `true` iff the watcher
/// interrupts (`docs/combat/resolution.md` §8: "Because it's a probability …").
///
/// Performs **exactly one** seeded draw: a single uniform `f32` in `[0.0, 1.0)`
/// compared `< *p`. Exactly one draw per call keeps the stream deterministically
/// replayable — two [`ReactionRng`](crate::rng::ReactionRng)s from the same
/// [`BattleSeed`](crate::rng::BattleSeed) yield the same interrupt sequence for
/// the same probabilities.
///
/// Drawing advances the stream cursor — the caller (GTW-468's live trigger) owns
/// the `ResMut<ReactionRng>` and passes it here by `&mut` (this pure fn never
/// holds a `Res`/`ResMut` itself).
///
/// # Arguments
///
/// - `p` — the [`ReactionProbability`] from [`interrupt_probability`].
/// - `rng` — the injected seeded [`ReactionRng`](crate::rng::ReactionRng),
///   borrowed mutably for the one draw.
#[must_use]
pub fn rolls_interrupt(p: ReactionProbability, rng: &mut crate::rng::ReactionRng) -> bool {
    // Exactly one draw: a uniform [0.0, 1.0) sample. `< p` so p == 0.0 never
    // fires and p == 1.0 always fires — but the §8 clamp keeps p strictly inside
    // (0, 1), so neither extreme is reachable in practice.
    let roll: f32 = rng.random_range(0.0_f32..1.0);
    roll < *p
}

/// The §8 **per-turn cap gate** — may this watcher take another reaction
/// interrupt this enemy turn? (`docs/combat/resolution.md` §8:
/// `max interrupts this enemy turn = cap(Reactions)`.)
///
/// Returns `true` iff the watcher has used FEWER interrupts than its
/// [`reaction_cap`] allows: `*used < reaction_cap(reactions, tuning)`. Per-watcher
/// per-turn. A pure predicate — the WIRING that increments `used` after a shot
/// and resets it at the turn boundary is GTW-468.
///
/// # Arguments
///
/// - `used` — the watcher's [`ReactionsUsed`] count so far this turn.
/// - `reactions` — the watcher's [`Reactions`] computed stat (the cap input).
/// - `tuning` — the [`ReactionTuning`] group from [`crate::tuning::CombatTuning`].
#[must_use]
pub fn may_interrupt(used: ReactionsUsed, reactions: Reactions, tuning: &ReactionTuning) -> bool {
    *used < reaction_cap(reactions, tuning)
}
