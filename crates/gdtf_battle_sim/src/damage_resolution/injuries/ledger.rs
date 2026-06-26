//! The per-ganger injury ledger [`InflictedInjuries`] — the single source of every
//! injury stat delta — and its two accumulators ([`StatDeltaLedger`] /
//! [`BleedAfflicted`]).

use bevy::prelude::{Component, Deref};

use super::{BleedAmount, GainedInjury, InjuryEffect, StatDelta, StatTarget};

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
}

/// The per-[`StatTarget`] **summed-delta store** of a ledger — the sixteen running
/// [`StatDeltaSum`]s in canonical [`StatTarget::ALL`] order, keyed by
/// [`StatTarget::index`] (`docs/combat/resolution.md` injury tables; GTW-405).
///
/// The modifier-layer delta source the GTW-436 projector reads per stat (one fold
/// for the eight attributes PRE-derivation, one for the eight derived stats
/// POST-derivation). A no-bare-types newtype over the fixed `[StatDeltaSum; 16]`
/// array (the store is a domain value; each element is a typed sum, keyed by index):
/// private inner + derived [`Deref`] (read the slice); the only mutation is
/// [`add_delta`](StatDeltaLedger::add_delta). Defaults to all-zero.
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

    /// The running summed delta for `stat` — the value the GTW-436 projector adds to
    /// that stat.
    #[must_use]
    pub const fn delta_for(&self, stat: StatTarget) -> StatDeltaSum {
        self.0[stat.index()]
    }
}

/// A ganger's **accrued per-turn bleed** — the summed [`BleedAmount`] of every
/// [`InjuryEffect::Bleeding`] on its ledger, drained each turn by the existing bleed
/// runtime (`docs/combat/resolution.md` §9; GTW-405).
///
/// Widened to **`u16`** deliberately: a single [`BleedAmount`] is `u8`
/// (`0..=255`), and several bleeding injuries can stack, so the accrual needs
/// headroom — `u16` holds over 256 worst-case `u8` amounts without overflow (and
/// [`accumulate`](BleedAfflicted::accumulate) saturates beyond that). A no-bare-types
/// newtype: private inner + derived [`Deref`]; defaults to `0` (no bleed). Distinct
/// from the Downed Wounds bleed-out: this drains HP and can down but never kill.
///
/// GTW-438: ALSO a [`Component`] — the standalone per-ganger accrual the bleed runtime
/// ([`tick_bleed`](crate::bleed::tick_bleed)) queries to drain HP each round. The
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
/// every projection, never applied-once, so a `stat_tuning.ron` hot-reload re-applies
/// deltas by construction rather than wiping them.
///
/// Private fields with named accessors (the sole mutator is
/// [`gain`](InflictedInjuries::gain)); seeded empty ([`Default`]) on every spawned
/// ganger. THIS slice owns the ledger + folding; the projector that READS the deltas
/// is GTW-436, and the loader/roll/apply wiring is GTW-437/438.
#[derive(Component, Debug, Clone, PartialEq, Eq, Default)]
pub struct InflictedInjuries {
    /// The ordered named-condition list, in infliction order (additive — only grows
    /// until a future GTW-23 heal removes an entry).
    gained: Vec<GainedInjury>,
    /// The per-stat summed deltas — the modifier-layer delta source.
    deltas: StatDeltaLedger,
    /// The accrued per-turn HP bleed.
    bleed:  BleedAfflicted,
}

impl InflictedInjuries {
    /// **Gain** one injury: append its [`GainedInjury`] to the ordered ledger AND
    /// fold each of its effects into the accumulators — a [`Modify`](InjuryEffect::Modify)
    /// adds its [`StatDelta`] to the named stat's running sum, a
    /// [`Bleeding`](InjuryEffect::Bleeding) adds its [`BleedAmount`] to the bleed
    /// accrual.
    ///
    /// The sole mutator of the ledger (the GTW-437 apply boundary calls this once per
    /// inflicted injury). Folding the effects here keeps the summed-delta store
    /// consistent with the named-condition list at all times, so the projector never
    /// re-walks the list. The match is EXHAUSTIVE — a new [`InjuryEffect`] variant
    /// forces a new arm here (compile-checked, never a silent no-op).
    pub fn gain(&mut self, record: GainedInjury) {
        for effect in &record.effects {
            match *effect {
                InjuryEffect::Modify { stat, amount } => self.deltas.add_delta(stat, amount),
                InjuryEffect::Bleeding { amount } => self.bleed = self.bleed.accumulate(amount),
            }
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
}
