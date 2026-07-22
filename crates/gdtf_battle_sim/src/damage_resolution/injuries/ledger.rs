//! The per-ganger injury ledger [`InflictedInjuries`] — the single source of every
//! injury stat delta — and its two accumulators ([`StatDeltaLedger`] /
//! [`BleedAfflicted`]).

use bevy::prelude::{Component, Deref};

use super::{BleedAmount, GainedInjury, HandsAvailable, MovementCostFactor, StatDelta, StatTarget};
use crate::{
    armor::BodyPart,
    effects::injuries::{ApplyInjuryEffect, LedgerAccumulators},
};

/// The running **summed delta** for one [`StatTarget`] across every injury on a
/// ganger's ledger — the modifier-layer total the GTW-436 projector adds to that
/// stat (`docs/combat/resolution.md` injury tables; GTW-405).
///
/// Widened to **`i16`** deliberately: a single [`StatDelta`] is `i8`
/// (`-128..=127`), and several injuries can stack on the same stat, so the sum needs
/// headroom — `i16` holds over 256 worst-case same-sign `i8` deltas without
/// overflow (and [`add`](StatDeltaSum::add) saturates beyond that, never panics). A
/// no-bare-types newtype: private inner + derived [`Deref`] (the projector reads
/// `*sum` as `i16`); defaults to `0` (no injuries).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StatDeltaSum(i16);

impl StatDeltaSum {
    /// Build a summed delta from its `i16` total (mainly for tests; the live total
    /// grows through [`add`](StatDeltaSum::add)).
    #[must_use]
    pub const fn new(sum: i16) -> Self {
        Self(sum)
    }

    /// Fold one [`StatDelta`] into this running sum, saturating (never panics /
    /// overflows even under absurd stacking).
    #[must_use]
    pub const fn add(self, delta: StatDelta) -> Self {
        Self(self.0.saturating_add(delta.raw() as i16))
    }

    /// Fold one [`StatDelta`] back OUT of this running sum (the GTW-550 heal path's
    /// exact inverse of [`add`](StatDeltaSum::add)), saturating — exact unless the
    /// sum ever saturated, which takes 256+ worst-case same-sign stacked deltas.
    #[must_use]
    pub const fn subtract(self, delta: StatDelta) -> Self {
        Self(self.0.saturating_sub(delta.raw() as i16))
    }
}

/// The per-[`StatTarget`] **summed-delta store** of a ledger — the sixteen running
/// [`StatDeltaSum`]s in canonical [`StatTarget::ALL`] order, keyed by
/// [`StatTarget::index`] (`docs/combat/resolution.md` injury tables; GTW-405).
///
/// The modifier-layer delta source the GTW-436 projector reads per stat (one fold
/// for the eight attributes PRE-derivation, one for the eight derived stats
/// POST-derivation). A no-bare-types newtype over the fixed `[StatDeltaSum; 16]`
/// array (the store is a domain value; each element is a typed sum, keyed by index):
/// private inner + derived [`Deref`] (read the slice); the only mutations are
/// [`add_delta`](StatDeltaLedger::add_delta) and its heal-path inverse
/// [`remove_delta`](StatDeltaLedger::remove_delta). Defaults to all-zero.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatDeltaLedger([StatDeltaSum; StatTarget::COUNT]);

impl Default for StatDeltaLedger {
    fn default() -> Self {
        Self([StatDeltaSum::default(); StatTarget::COUNT])
    }
}

impl StatDeltaLedger {
    /// Fold one [`StatDelta`] into the running sum for `stat` (saturating).
    pub const fn add_delta(&mut self, stat: StatTarget, delta: StatDelta) {
        let i = stat.index();
        self.0[i] = self.0[i].add(delta);
    }

    /// Fold one [`StatDelta`] back OUT of the running sum for `stat` (saturating) —
    /// the GTW-550 heal path's exact inverse of [`add_delta`](StatDeltaLedger::add_delta).
    pub const fn remove_delta(&mut self, stat: StatTarget, delta: StatDelta) {
        let i = stat.index();
        self.0[i] = self.0[i].subtract(delta);
    }

    /// The running summed delta for `stat` — the value the GTW-436 projector adds to
    /// that stat.
    #[must_use]
    pub const fn delta_for(&self, stat: StatTarget) -> StatDeltaSum {
        self.0[stat.index()]
    }
}

/// A ganger's **accrued per-turn bleed** — the summed [`BleedAmount`] of every
/// [`Bleeding`](crate::injuries::InjuryEffect::Bleeding) on its ledger, drained each
/// turn by the existing bleed runtime (`docs/combat/resolution.md` §9; GTW-405).
///
/// Widened to **`u16`** deliberately: a single [`BleedAmount`] is `u8`
/// (`0..=255`), and several bleeding injuries can stack, so the accrual needs
/// headroom — `u16` holds over 256 worst-case `u8` amounts without overflow (and
/// [`accumulate`](BleedAfflicted::accumulate) saturates beyond that). A no-bare-types
/// newtype: private inner + derived [`Deref`]; defaults to `0` (no bleed). Distinct
/// from the Downed Wounds bleed-out: this drains HP and can down but never kill.
///
/// GTW-438: ALSO a [`Component`] — the standalone per-ganger accrual the bleed runtime
/// ([`tick_bleed`](crate::effects::bleed::tick_bleed)) queries to drain HP each round. The
/// [`apply_injury`](crate::acts::apply_injury) boundary keeps it in sync with the
/// owning [`InflictedInjuries`] ledger's [`bleed`](InflictedInjuries::bleed) (the SINGLE
/// source: the ledger's `gain` is the only accrual point; this component MIRRORS it so a
/// `Query<&BleedAfflicted>` can read it without the whole ledger). Derives the sentinel
/// [`Default`] the `bsn!` spawn seed needs (bsn-sentinel-defaults convention).
#[derive(Component, Deref, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BleedAfflicted(u16);

impl BleedAfflicted {
    /// Build an accrued bleed from its `u16` total (mainly for tests; the live total
    /// grows through [`accumulate`](BleedAfflicted::accumulate)).
    #[must_use]
    pub const fn new(total: u16) -> Self {
        Self(total)
    }

    /// Fold one [`BleedAmount`] into the accrued per-turn bleed, saturating.
    #[must_use]
    pub const fn accumulate(self, amount: BleedAmount) -> Self {
        Self(self.0.saturating_add(amount.raw() as u16))
    }

    /// Fold one [`BleedAmount`] back OUT of the accrued per-turn bleed (saturating) —
    /// the GTW-550 heal path's exact inverse of [`accumulate`](BleedAfflicted::accumulate).
    #[must_use]
    pub const fn relieve(self, amount: BleedAmount) -> Self {
        Self(self.0.saturating_sub(amount.raw() as u16))
    }
}

/// A ganger's **inflicted-injury ledger** — the ordered named-condition list AND
/// the SOLE source of every injury stat delta and bleed accrual
/// (`docs/combat/resolution.md` injury tables; GTW-405).
///
/// A campaign-persistent component (sibling to
/// [`InflictedWounds`](crate::inflicted_wound::InflictedWounds)), carrying three
/// parts: the ordered [`GainedInjury`] list (the inspect-panel + heal source), the
/// per-[`StatTarget`] [`StatDeltaLedger`] (the modifier-layer delta source the
/// GTW-436 projector re-sums every projection), and the [`BleedAfflicted`] accrual
/// (drained by the bleed runtime). It enforces the single-source-of-truth invariant:
/// every injury delta lives in exactly ONE place — this ledger — and is RE-SUMMED on
/// every projection, never applied-once, so a `stat.tuning.ron` hot-reload re-applies
/// deltas by construction rather than wiping them.
///
/// STORAGE LIVES HERE, behaviour lives in the palette (GTW-550): each effect folds
/// itself into the borrowed [`LedgerAccumulators`] view through the
/// [`ApplyInjuryEffect`] trait, and this ledger stays the single
/// `Changed<InflictedInjuries>` source the GTW-436 projector filters on — never an
/// effect-owned accumulator component.
///
/// Private fields with named accessors (the sole mutator is
/// [`gain`](InflictedInjuries::gain)); seeded empty ([`Default`]) on every spawned
/// ganger. THIS slice owns the ledger + folding; the projector that READS the deltas
/// is GTW-436, and the loader/roll/apply wiring is GTW-437/438.
///
/// Derives [`PartialEq`] but NOT [`Eq`] (GTW-444): the `movement` accumulator is a
/// [`MovementCostFactor`] over `f32`, which is not `Eq`. Nothing keys this ledger in a
/// `HashSet`/`BTreeMap`; it is read through queries and the `Changed<InflictedInjuries>`
/// change-detection filter (tick-based, not `Eq`-based), so dropping `Eq` is safe and
/// keeps the multiply exact + deterministic (a fixed-point rep would forfeit that).
#[derive(Component, Debug, Clone, PartialEq, Default)]
pub struct InflictedInjuries {
    /// The ordered named-condition list, in infliction order (additive — only grows
    /// until a future GTW-23 heal removes an entry).
    gained:   Vec<GainedInjury>,
    /// The per-stat summed deltas — the modifier-layer delta source.
    deltas:   StatDeltaLedger,
    /// The accrued per-turn HP bleed.
    bleed:    BleedAfflicted,
    /// The accumulated MULTIPLICATIVE movement-cost factor (GTW-444) — the PRODUCT of
    /// every [`MovementCostMul`](crate::injuries::InjuryEffect::MovementCostMul) gained,
    /// defaulting to [`MovementCostFactor::IDENTITY`] (`1.0`). DEDICATED + MULTIPLICATIVE:
    /// separate from the summed `deltas` arrays and the summed `bleed` accrual because it
    /// MULTIPLIES, it does not sum (two factors of `1.5` and `2.0` fold to `3.0`).
    movement: MovementCostFactor,
}

impl InflictedInjuries {
    /// **Gain** one injury: append its [`GainedInjury`] to the ordered ledger AND
    /// fold each of its effects into the accumulators by DELEGATING through the
    /// [`ApplyInjuryEffect`] palette trait (GTW-550): each effect's isolated
    /// behaviour (its own file under `crate::effects::injuries`) decides which
    /// accumulator moves and how — a summed stat delta, a summed bleed accrual, a
    /// multiplicative movement factor, or nothing (a read-projected effect). This fn
    /// only lends out the [`LedgerAccumulators`] fold surface; adding a new effect
    /// kind never touches it (the palette's one delegation match is exhaustive, so
    /// the compile-check lives there).
    ///
    /// The sole mutator of the ledger (the GTW-437 apply boundary calls this once per
    /// inflicted injury, SYNCHRONOUSLY on the message-drain path — never a deferred
    /// command — so the gain trips `Changed<InflictedInjuries>` the same tick the
    /// GTW-436 projector re-derives on). Folding the effects here keeps the
    /// summed-delta store consistent with the named-condition list at all times, so
    /// the projector never re-walks the list.
    pub fn gain(&mut self, record: GainedInjury) {
        let mut accumulators = LedgerAccumulators {
            deltas:   &mut self.deltas,
            bleed:    &mut self.bleed,
            movement: &mut self.movement,
        };
        for effect in &record.effects {
            effect.fold_on_gain(&mut accumulators);
        }
        self.gained.push(record);
    }

    /// The ordered named-condition list — the inspect-panel source and the future
    /// GTW-23 heal input.
    #[must_use]
    pub fn gained(&self) -> &[GainedInjury] {
        &self.gained
    }

    /// The running summed delta for `stat` — the value the GTW-436 projector adds to
    /// that stat (PRE-derivation for an attribute, POST-derivation for a derived stat).
    #[must_use]
    pub const fn delta_for(&self, stat: StatTarget) -> StatDeltaSum {
        self.deltas.delta_for(stat)
    }

    /// The accrued per-turn HP bleed — the amount the bleed runtime drains each turn.
    #[must_use]
    pub const fn bleed(&self) -> BleedAfflicted {
        self.bleed
    }

    /// The ganger's accumulated **movement-cost factor** ([`MovementCostFactor`], GTW-444)
    /// — the PRODUCT of every
    /// [`MovementCostMul`](crate::injuries::InjuryEffect::MovementCostMul) gained,
    /// or [`MovementCostFactor::IDENTITY`] (`1.0`) when none.
    ///
    /// The per-step movement TU cost = the GTW-396 terrain per-step floor cost MULTIPLIED
    /// by this factor (rounded UP — see the pathfinder cost-scale). A factor `>= 1.0`
    /// slows the ganger ("Hampered"); `1.0` is the uninjured identity (no slowdown). The
    /// pathfinder (move-range / path preview) and the committed walk's per-step charge
    /// BOTH read this and scale identically, so preview == charge (C3). STACKING
    /// MULTIPLIES (C4): the value is the running product the `gain` fold builds, so it is
    /// order-independent and deterministic.
    #[must_use]
    pub const fn movement_cost_factor(&self) -> MovementCostFactor {
        self.movement
    }

    /// The ganger's **available hand count** ([`HandsAvailable`]), DERIVED on read by
    /// folding the ledger's [`gained`](InflictedInjuries::gained) entries (GTW-443).
    ///
    /// Folds every [`GainedInjury`] whose effects project
    /// [`disables_hand`](ApplyInjuryEffect::disables_hand) (the palette's read-side
    /// verb — this fold never matches on the effect vocabulary) into a SET of distinct
    /// disabled arm-sides — its struck [`part`](GainedInjury::part) maps
    /// [`LeftArm`](crate::armor::BodyPart::LeftArm) → the left side and
    /// [`RightArm`](crate::armor::BodyPart::RightArm) → the right side; a
    /// hand-disabling effect carried by a Head / Torso / Leg injury is INERT (no hand
    /// to disable). The count is `2 − (left disabled) − (right disabled)`, so:
    /// - no arm injury → `2`;
    /// - one arm side disabled → `1`;
    /// - TWO same-side hand-disabling injuries → still `1` (the set holds one side),
    ///   never `0` — the property a per-injury counter could not give without de-duping;
    /// - both sides disabled → `0`.
    ///
    /// Order-independent and deterministic (set membership over sides, not a count over
    /// entries). The result is clamped into `0..=2` by [`HandsAvailable::new`].
    #[must_use]
    pub fn hands_available(&self) -> HandsAvailable {
        let mut left_disabled = false;
        let mut right_disabled = false;
        for record in &self.gained {
            let disables = record.effects.iter().any(|e| *e.disables_hand());
            if !disables {
                continue;
            }
            match record.part {
                BodyPart::LeftArm => left_disabled = true,
                BodyPart::RightArm => right_disabled = true,
                // A hand-disabling effect on a non-arm part is inert (no hand to disable).
                BodyPart::Head | BodyPart::Torso | BodyPart::LeftLeg | BodyPart::RightLeg => {}
            }
        }
        let disabled = u8::from(left_disabled) + u8::from(right_disabled);
        HandsAvailable::new(HandsAvailable::MAX - disabled)
    }
}
