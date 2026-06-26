//! The [`Bleeding`] signal + the per-round [`tick_bleed`] drain — the bleed-out
//! clock's message and system (the §9 once-per-round drain + terminal gate).

use bevy::prelude::{Entity, Message, MessageWriter, Query, Res};

use crate::{
    ganger::{Hp, LifeState, Stabilized, Wounds},
    injuries::BleedAfflicted,
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
);

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
) {
    let rate = *tuning.bleed_rate;
    for (entity, hp, mut wounds, mut life, stabilized, bleed) in &mut q {
        // A Dead ganger is a corpse — neither bleed source touches it (the once-only
        // property: once Dead, the next tick skips it).
        if *life == LifeState::Dead {
            continue;
        }

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
            // Down-not-kill gate: an injury HP bleed that empties the pool downs an Alive
            // ganger; it NEVER sets Dead (the Wounds bleed-out owns the kill).
            if *hp == Hp::new(0) && *life == LifeState::Alive {
                *life = LifeState::Downed;
            }
        }

        // (1) The §9 Downed Wounds bleed-out — only the Downed bleed (Alive is fighting;
        // the Dead skip already `continue`d above).
        if *life != LifeState::Downed {
            continue;
        }

        // (2) A stabilized ganger's bleed-out clock is halted — no Wounds drain, no
        // Bleeding from THIS source (the Wounds already lost stay lost; it remains
        // Downed). E3.8 SETS the flag; this slice only READS it.
        if stabilized.is_some_and(|s| **s) {
            continue;
        }

        // (3) Drain a flat bleed_rate from the life pool (saturating at 0 — Wounds
        // is unsigned, so the lethal tick floors it, never underflows) and emit the
        // per-tick bleeding signal.
        *wounds = Wounds::new(wounds.saturating_sub(rate));
        writer.write(Bleeding::new(entity));

        // (4) Terminal gate — Wounds depleted to 0 → Dead (the same once-only gate
        // E3.6's apply_hit runs; "≤ 0" is "== 0 after the saturating drain").
        if *wounds == Wounds::new(0) {
            *life = LifeState::Dead;
        }
    }
}
