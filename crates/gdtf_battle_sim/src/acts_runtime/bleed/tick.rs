//! The [`Bleeding`] signal + the per-round [`tick_bleed`] drain — the bleed-out
//! clock's message and system (the §9 once-per-round drain + terminal gate).

use bevy::prelude::{Commands, Component, Entity, Message, MessageWriter, Query, Res};

use crate::{
    ganger::{Hp, LifeState, Position, Stabilized, Wounds},
    injuries::BleedAfflicted,
    on_death::OnDeathOccurred,
    tuning::CombatTuning,
};

/// The [`tick_bleed`] per-ganger query row — the two bleed sources' surfaces bundled into
/// one `QueryData` tuple alias so the `Query` stays under clippy's type-complexity gate
/// (GTW-438): the [`Entity`], the optional [`Hp`] (the injury-bleed pool — `Option` so a
/// ganger with no Hp component is simply un-drainable), the [`Wounds`] (the §9 Downed
/// bleed-out pool), the [`LifeState`] (the gates), the optional [`Stabilized`] (the §9
/// clock halt), and the optional [`BleedAfflicted`] (the injury-bleed accrual).
type BleedRow = (
    Entity,
    Option<&'static mut Hp>,
    &'static mut Wounds,
    &'static mut LifeState,
    Option<&'static Stabilized>,
    Option<&'static BleedAfflicted>,
    // GTW-547: the ganger's cell — the (cell, level) the terminal-death OnDeathOccurred signal
    // carries when a Wounds bleed-out empties the pool (the tick otherwise has no cell in scope).
    // `Option` so a minimal test ganger with no Position still matches the query (the emit is
    // simply skipped when absent — a live battle ganger always has one).
    Option<&'static Position>,
    // GTW-572: the in-progress-span bookkeeping marker — present iff the ganger drained on the
    // PREVIOUS tick, so THIS tick can tell a fresh affliction span (emit BleedStarted once)
    // from a continuing one (emit nothing).
    Option<&'static BleedOngoing>,
);

/// Sim bookkeeping: this ganger's bleed affliction span is **in progress** — it drained
/// (from either bleed source) on the most recent [`tick_bleed`] round (GTW-572).
///
/// [`tick_bleed`] inserts it on the FIRST draining tick of a span (alongside the one
/// [`BleedStarted`] fact) and removes it the first round the ganger no longer drains
/// (stabilized / healed / dead), so a LATER fresh span re-announces. A marker component
/// (no payload — presence IS the fact), never presenter-read: the presenter reads the
/// [`BleedStarted`] message, not this bookkeeping.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct BleedOngoing;

/// A bleed **affliction span STARTED** — `ganger` began bleeding this round (GTW-572).
///
/// The once-at-affliction-start fact the combat log's bleeding line reads (the Q2 ruling:
/// bleed logs ONCE at affliction start, never per tick). Emitted by [`tick_bleed`] exactly
/// once per span — the FIRST round either bleed source (the §9 Downed Wounds bleed-out or
/// the GTW-438 injury HP bleed) drains the ganger — while the per-tick [`Bleeding`] signal
/// keeps firing every draining round for the FCT pop. A ganger whose bleeding stops
/// (stabilized / healed) and later starts again is a NEW span and re-emits. The sim emits
/// the FACT; the presenter phrases the line (ADR-0001).
///
/// A buffered Bevy [`Message`] (`bevy-traps.md` #4 — NOT the observer `Event`), mirroring
/// [`Bleeding`]. The payload is the ganger [`Entity`] — framework plumbing, the only bare
/// type the no-bare-types rule permits in a payload.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BleedStarted {
    /// The ganger whose bleed span just started — resolved to a name at the presenter boundary.
    pub ganger: Entity,
}

impl BleedStarted {
    /// Build a bleed-span-started fact for `ganger`.
    #[must_use]
    pub const fn new(ganger: Entity) -> Self {
        Self { ganger }
    }
}

/// A [`LifeState::Downed`] ganger **bled** this round — a "Bleeding Out" stack
/// drained a flat [`crate::tuning::BleedRate`] of its [`Wounds`].
///
/// Emitted by [`tick_bleed`] **once per draining tick** for each un-stabilized
/// Downed ganger (including the lethal tick that empties the pool); a stabilized,
/// [`LifeState::Alive`], or [`LifeState::Dead`] ganger drains nothing and emits
/// nothing. The presenter reads this to surface the bleed-out clock on screen
/// (`docs/combat/resolution.md` §9's "a per-tick ganger-bleeding event puts it on
/// screen").
///
/// A buffered Bevy **message** (`#[derive(Message)]`), mirroring
/// [`crate::occupancy_sync::CoverDestroyed`] / [`crate::armor_wear::ArmorBroken`] —
/// NOT the observer `Event` API (`bevy-traps.md` #4), so it is written with
/// [`bevy::prelude::MessageWriter`] and read with [`bevy::prelude::MessageReader`].
/// The payload is the ganger [`Entity`] — Bevy framework plumbing (the only bare
/// type the no-bare-types rule permits: an entity handle, not a domain value).
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Bleeding {
    /// The ganger that bled — the entity whose Wounds the bleed-out drained.
    pub ganger: Entity,
}

impl Bleeding {
    /// Build a bleeding signal for the `ganger` whose Wounds a bleed-out tick just
    /// drained.
    #[must_use]
    pub const fn new(ganger: Entity) -> Self {
        Self { ganger }
    }
}

/// Drain one round of bleed-out from every un-stabilized [`LifeState::Downed`]
/// ganger — the §9 per-round clock (`docs/combat/resolution.md` §9;
/// `docs/combat/wounds-and-roster.md` §"Downed → death … state machine").
///
/// Run once per full round (ticked at the enemy-phase start). For each queried
/// ganger, in order:
///
/// 1. **Skip unless Downed** — an [`LifeState::Alive`] ganger is up and fighting;
///    an [`LifeState::Dead`] ganger is already a corpse (the once-only property:
///    once Dead, the next tick skips it). Both mutate nothing and emit nothing.
/// 2. **Skip if stabilized** — an ally has dressed the wound ([`Stabilized`] present
///    **and** its bool `true`): the clock is halted, so no drain and no
///    [`Bleeding`] (the Wounds already lost stay lost; the ganger remains Downed).
/// 3. **Drain** — subtract the flat tuning [`crate::tuning::BleedRate`] from the
///    ganger's [`Wounds`] (`saturating_sub`, so the unsigned life pool never
///    underflows — it floors at `0`), and emit one [`Bleeding`] carrying the ganger
///    [`Entity`].
/// 4. **Terminal gate** — if [`Wounds`] is now `0` (the doc's `Wounds ≤ 0` on an
///    unsigned pool that saturates), set [`LifeState::Dead`] — the **same** gate
///    E3.6's `apply_hit` runs, applied once on the draining-to-empty tick.
///
/// `tuning` supplies the per-round [`crate::tuning::BleedRate`]; `writer` buffers
/// each [`Bleeding`]. Pure, render-free, saturating arithmetic — no underflow, no
/// `unwrap`, no pixel.
///
/// **GTW-438 — the injury-driven HP bleed.** ALSO drains the per-ganger
/// [`BleedAfflicted`] accrual (the summed [`crate::injuries::InjuryEffect::Bleeding`] on
/// the injury ledger) from the ganger's [`Hp`] each round — a SEPARATE source from the
/// Downed `Wounds` bleed-out above (`docs/combat/resolution.md` injury tables; the GTW-405
/// design §"Bleeding routing"). The two are DISTINCT: the Wounds bleed-out drains the
/// LIFE pool of an un-stabilized **Downed** ganger and CAN KILL (Wounds → 0 → Dead); the
/// injury HP bleed drains the **Hp** pool of ANY non-Dead ganger (Alive or Downed) and
/// **can down but never kill** (Hp → 0 sets [`LifeState::Downed`], never `Dead` — only a
/// Wounds depletion kills). Each draining injury-bleed tick ALSO emits a [`Bleeding`] (the
/// same message the presenter's FCT pop reads), so an injury bleed surfaces on screen like
/// the Downed bleed-out.
pub fn tick_bleed(
    mut q: Query<BleedRow>,
    tuning: Res<CombatTuning>,
    mut writer: MessageWriter<Bleeding>,
    // GTW-547: the terminal-death signal — a Wounds bleed-out that KILLS (Wounds → 0 → Dead)
    // emits one OnDeathOccurred at the dead ganger's cell so `resolve_on_death` fans its
    // on-death effect (a bleed-out death must not silently skip it — the ticket's every-gate AC).
    mut deaths: MessageWriter<OnDeathOccurred>,
    // GTW-572: the once-per-span affliction-start fact — emitted on the FIRST draining tick
    // of a span (the [`BleedOngoing`] marker tracks the span; `bevy-traps.md` #4).
    mut started: MessageWriter<BleedStarted>,
    // GTW-572: inserts/removes the span-bookkeeping [`BleedOngoing`] marker (deferred — the
    // marker is only read NEXT round, so the Commands sync point is early enough).
    mut commands: Commands,
) {
    let rate = *tuning.bleed_rate;
    for (entity, hp, mut wounds, mut life, stabilized, bleed, position, ongoing) in &mut q {
        // A Dead ganger is a corpse — neither bleed source touches it (the once-only
        // property: once Dead, the next tick skips it). Its span bookkeeping is dropped
        // (a corpse never drains again).
        if *life == LifeState::Dead {
            if ongoing.is_some() {
                commands.entity(entity).remove::<BleedOngoing>();
            }
            continue;
        }

        // GTW-572: whether EITHER bleed source drained this ganger THIS round — the
        // span-boundary signal the once-per-span BleedStarted fact keys off.
        let mut drained = false;

        // (A) GTW-438 — the injury-driven HP bleed: drain the accrued BleedAfflicted from
        // the Hp pool of ANY non-Dead ganger (Alive or Downed). It is a SEPARATE source
        // from the Downed Wounds bleed-out below, and the stabilization flag does NOT halt
        // it (stabilization is the §9 Downed-bleedout clock; an injury bleed is its own
        // condition). Drains Hp (saturating at 0); Hp → 0 DOWNS the ganger (never kills —
        // only a Wounds depletion kills). Emits one Bleeding per draining tick (the
        // presenter FCT pop). A zero accrual (or no BleedAfflicted) drains nothing; `Hp`
        // is `Option` (a ganger with an accrual but no Hp pool — e.g. a minimal test
        // ganger — simply has nothing to drain), so the §9 Wounds bleed-out below stays
        // independent of the Hp component's presence.
        let injury_bleed = bleed.map_or(0u16, |b| **b);
        if let Some(mut hp) = hp
            && injury_bleed > 0
        {
            *hp = Hp::new(hp.saturating_sub(injury_bleed));
            writer.write(Bleeding::new(entity));
            drained = true;
            // Down-not-kill gate: an injury HP bleed that empties the pool downs an Alive
            // ganger; it NEVER sets Dead (the Wounds bleed-out owns the kill).
            if *hp == Hp::new(0) && *life == LifeState::Alive {
                *life = LifeState::Downed;
            }
        }

        // (1) The §9 Downed Wounds bleed-out — only the Downed bleed (Alive is fighting;
        // the Dead skip already `continue`d above). (2) A stabilized ganger's bleed-out
        // clock is halted — no Wounds drain, no Bleeding from THIS source (the Wounds
        // already lost stay lost; it remains Downed). E3.8 SETS the flag; this reads it.
        if *life == LifeState::Downed && !stabilized.is_some_and(|s| **s) {
            // (3) Drain a flat bleed_rate from the life pool (saturating at 0 — Wounds
            // is unsigned, so the lethal tick floors it, never underflows) and emit the
            // per-tick bleeding signal.
            *wounds = Wounds::new(wounds.saturating_sub(rate));
            writer.write(Bleeding::new(entity));
            drained = true;

            // (4) Terminal gate — Wounds depleted to 0 → Dead (the same once-only gate
            // E3.6's apply_hit runs; "≤ 0" is "== 0 after the saturating drain").
            if *wounds == Wounds::new(0) {
                *life = LifeState::Dead;
                // GTW-547: emit the terminal-death signal at the dead ganger's cell so its
                // authored on-death effect fans (`**position` derefs Position → CellLevel). A
                // minimal test ganger with no Position simply emits nothing (a live battle
                // ganger always has one).
                if let Some(position) = position {
                    deaths.write(OnDeathOccurred::new(entity, **position));
                }
            }
        }

        // GTW-572 — span maintenance: the FIRST draining tick of a span announces the
        // affliction (one BleedStarted) and marks the span; the first NON-draining tick
        // ends it (so a later fresh span re-announces). Mid-span draining ticks emit
        // only the per-tick Bleeding above — never a second start fact (the Q2 ruling).
        match (drained, ongoing.is_some()) {
            (true, false) => {
                started.write(BleedStarted::new(entity));
                commands.entity(entity).insert(BleedOngoing);
            }
            (false, true) => {
                commands.entity(entity).remove::<BleedOngoing>();
            }
            _ => {}
        }
    }
}
