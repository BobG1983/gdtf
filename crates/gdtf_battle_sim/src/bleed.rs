//! The bleed-out clock + the ganger-bleeding signal — the E3.7 slice (GTW-189).
//!
//! [`tick_bleed`] is the per-round drain of `docs/combat/resolution.md` §9 (and
//! `docs/combat/wounds-and-roster.md` §"Downed → death … state machine"): **once
//! per full round** every un-stabilized [`LifeState::Downed`] ganger gains a
//! "Bleeding Out" stack draining a flat tuning [`crate::tuning::BleedRate`] of
//! [`Wounds`] (the stack count = turns down = total Wounds lost, a clock you can
//! read), and when its [`Wounds`] life pool empties the ganger becomes
//! [`LifeState::Dead`] through the **same once-only terminal gate** as the E3.6
//! `apply_hit` path (`Wounds ≤ 0` → Dead). Each draining tick also emits a
//! [`Bleeding`] message carrying the ganger [`Entity`] so the presenter can surface
//! the clock on screen.
//!
//! ## Who bleeds (and who is skipped)
//!
//! - **Only [`LifeState::Downed`] gangers bleed.** An [`LifeState::Alive`] ganger is
//!   up and fighting; an [`LifeState::Dead`] one is already a corpse — both are
//!   skipped, mutate nothing, and emit no [`Bleeding`] (the "once-only" property:
//!   once Dead, the next tick skips it).
//! - **A [`Stabilized`] Downed ganger is skipped.** Once an ally has dressed the
//!   wound ([`Stabilized`] present and `true`), the clock halts: no drain, no new
//!   stack, no [`Bleeding`] — the Wounds already lost stay lost and the ganger
//!   **remains Downed** (E3.8 sets the flag; this slice only **reads** it).
//!
//! The drain, the emit, and the terminal gate all happen on the **same** draining
//! tick — including the lethal one (the tick that drops Wounds to `0` both emits a
//! [`Bleeding`] and flips the ganger to [`LifeState::Dead`]).
//!
//! [`Bleeding`] is a buffered Bevy **message** (`#[derive(Message)]`), mirroring
//! the [`crate::occupancy_sync::CoverDestroyed`] / [`crate::armor_wear::ArmorBroken`]
//! precedent (`docs/combat/resolution.md` §9 names the analogous ganger-bleeding
//! signal) — **NOT** the targeted/observer `Event` API (`bevy-traps.md` #4: Bevy
//! 0.18 renamed buffered `Event`/`EventReader` to `Message`/`MessageReader`), so it
//! is written with [`bevy::prelude::MessageWriter`] and read with
//! [`bevy::prelude::MessageReader`]. Pure, render-free model logic: no renderer, no
//! pixel; the drain saturates (no underflow, no `unwrap`).

use bevy::prelude::{Entity, Message, MessageWriter, Query, Res};

use crate::{
    ganger::{LifeState, Stabilized, Wounds},
    tuning::CombatTuning,
};

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
pub fn tick_bleed(
    mut q: Query<(Entity, &mut Wounds, &mut LifeState, Option<&Stabilized>)>,
    tuning: Res<CombatTuning>,
    mut writer: MessageWriter<Bleeding>,
) {
    let rate = *tuning.bleed_rate;
    for (entity, mut wounds, mut life, stabilized) in &mut q {
        // (1) Only the Downed bleed — Alive is fighting, Dead is a corpse (skip it,
        // the once-only property).
        if *life != LifeState::Downed {
            continue;
        }

        // (2) A stabilized ganger's clock is halted — no drain, no Bleeding (the
        // Wounds already lost stay lost; it remains Downed). E3.8 SETS the flag;
        // this slice only READS it.
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

#[cfg(test)]
mod tests {
    use bevy::prelude::{
        App, Entity, IntoScheduleConfigs, MessageReader, MinimalPlugins, ResMut, Resource, Update,
    };

    use super::*;
    use crate::ganger::{Hp, Wounds};

    /// Captures the [`Bleeding`] messages a reader system drained, so a test can
    /// assert on them after `update()` (no `unwrap` in the test body).
    #[derive(Resource, Default)]
    struct Captured(Vec<Bleeding>);

    /// Drains the buffered [`Bleeding`] messages (a [`MessageReader`], NOT an
    /// observer) into the [`Captured`] resource for assertion.
    fn consume(mut reader: MessageReader<Bleeding>, mut captured: ResMut<Captured>) {
        for bled in reader.read() {
            captured.0.push(*bled);
        }
    }

    /// Build a headless app wired for the bleed-out tick: `MinimalPlugins` (no
    /// window/renderer), the [`Bleeding`] message buffer registered, a default
    /// [`CombatTuning`] resource, and `tick_bleed` chained before [`consume`] in
    /// [`Update`] so the captured messages reflect the same tick.
    ///
    /// The sim crate cannot depend on `gdtf_test_utils` (that would cycle through
    /// `gdtf_app` → `gdtf_battle_sim`), so this uses the bare-`App` + `MinimalPlugins`
    /// fallback the contract sanctions — the same pattern `armor_wear.rs` and
    /// `occupancy_sync.rs` already use. The default tuning supplies the bleed rate
    /// (read, never pinned — tests assert the drop's relation to it).
    fn bleed_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_message::<Bleeding>();
        app.init_resource::<CombatTuning>();
        app.init_resource::<Captured>();
        app.add_systems(Update, (tick_bleed, consume).chain());
        app
    }

    /// The bleed rate the default [`CombatTuning`] carries — read, never pinned; the
    /// tests assert the per-tick drop EQUALS this, a relation to the tuning value.
    fn bleed_rate() -> u8 {
        *CombatTuning::default().bleed_rate
    }

    /// Read a spawned ganger's current `Wounds` back out of the app's world (returns
    /// `0` if the entity or component is somehow absent, so the test body needs no
    /// `unwrap`). A read-only world access — the real component, not a copy.
    fn wounds_of(app: &App, ganger: Entity) -> u8 {
        app.world().get::<Wounds>(ganger).map_or(0, |w| **w)
    }

    /// Read a spawned ganger's current `LifeState` back out of the app's world
    /// (defaults to [`LifeState::Alive`] if absent, so no `unwrap`).
    fn life_of(app: &App, ganger: Entity) -> LifeState {
        app.world()
            .get::<LifeState>(ganger)
            .copied()
            .unwrap_or(LifeState::Alive)
    }

    /// Count the captured [`Bleeding`] messages carrying a given ganger.
    fn bleeding_count_for(app: &App, ganger: Entity) -> usize {
        app.world()
            .get_resource::<Captured>()
            .map_or(0, |c| c.0.iter().filter(|b| b.ganger == ganger).count())
    }

    /// AC1 — one `tick_bleed` drains exactly `bleed_rate` Wounds from an un-stabilized
    /// Downed ganger. Spawns a Downed ganger with a comfortable Wounds pool, runs one
    /// `update()`, and asserts the drop equals the tuning rate (a RELATION to the
    /// tuning value, never a pinned magnitude).
    #[test]
    fn one_tick_drains_exactly_bleed_rate_from_a_downed_ganger() {
        let rate = bleed_rate();
        // A pool well above the rate so this single tick can't reach the gate (we are
        // measuring the per-tick drop, not the death transition).
        let start = rate.saturating_add(10);

        let mut app = bleed_app();
        let ganger = app
            .world_mut()
            .spawn((Wounds::new(start), LifeState::Downed))
            .id();

        app.update();

        assert_eq!(
            wounds_of(&app, ganger),
            start - rate,
            "one tick must drain exactly the tuning bleed_rate from a Downed ganger",
        );
        assert_eq!(
            life_of(&app, ganger),
            LifeState::Downed,
            "a non-lethal tick must leave the ganger Downed",
        );
    }

    /// AC2 — the clock is a stack: N ticks drain N×`bleed_rate`, and ticking until the
    /// pool empties flips the ganger to `Dead` via the terminal gate. Drives several
    /// ticks asserting the cumulative drop, then keeps ticking to depletion and
    /// asserts the Dead transition (Wounds floored at 0).
    #[test]
    fn ticks_stack_and_deplete_to_dead() {
        let rate = bleed_rate();
        // A multiple of the rate so depletion lands cleanly at 0 (and is > 0 so the
        // first tick is non-lethal). Five rounds of bleed.
        let rounds = 5u8;
        let start = rate.saturating_mul(rounds);

        let mut app = bleed_app();
        let ganger = app
            .world_mut()
            .spawn((Wounds::new(start), LifeState::Downed))
            .id();

        // Tick three rounds — cumulative drop is 3×rate, still alive-but-downed.
        for n in 1..=3u8 {
            app.update();
            assert_eq!(
                wounds_of(&app, ganger),
                start - rate * n,
                "after {n} ticks the cumulative drop must be {n}×bleed_rate (a stack)",
            );
            assert_eq!(
                life_of(&app, ganger),
                LifeState::Downed,
                "while Wounds remain the ganger stays Downed",
            );
        }

        // Keep ticking until the pool empties — the terminal gate flips it to Dead.
        for _ in 0..rounds {
            app.update();
        }

        assert_eq!(
            wounds_of(&app, ganger),
            0,
            "bleeding to the floor must leave Wounds at 0 (saturating, no underflow)",
        );
        assert_eq!(
            life_of(&app, ganger),
            LifeState::Dead,
            "Wounds depleted to 0 by the clock must transition the ganger to Dead",
        );
    }

    /// AC3 — only Downed gangers bleed: an Alive ganger and a Dead ganger are
    /// untouched across a tick (Wounds unchanged, no `Bleeding`). Spawns one of each
    /// alongside a Downed control, ticks once, and asserts the non-Downed pools are
    /// unchanged while the Downed one drained.
    #[test]
    fn alive_and_dead_gangers_are_untouched() {
        let rate = bleed_rate();
        let start = rate.saturating_add(10);

        let mut app = bleed_app();
        let alive = app
            .world_mut()
            .spawn((Wounds::new(start), LifeState::Alive))
            .id();
        let dead = app
            .world_mut()
            .spawn((Wounds::new(start), LifeState::Dead))
            .id();
        let downed = app
            .world_mut()
            .spawn((Wounds::new(start), LifeState::Downed))
            .id();

        app.update();

        // The Alive ganger is up and fighting — no drain, no Bleeding.
        assert_eq!(
            wounds_of(&app, alive),
            start,
            "an Alive ganger must not bleed (Wounds unchanged)",
        );
        assert_eq!(
            bleeding_count_for(&app, alive),
            0,
            "no Bleeding for an Alive ganger"
        );
        assert_eq!(
            life_of(&app, alive),
            LifeState::Alive,
            "an Alive ganger stays Alive"
        );

        // The Dead ganger is a corpse — the once-only property skips it.
        assert_eq!(
            wounds_of(&app, dead),
            start,
            "a Dead ganger must not bleed (the once-only skip)",
        );
        assert_eq!(
            bleeding_count_for(&app, dead),
            0,
            "no Bleeding for a Dead ganger"
        );
        assert_eq!(
            life_of(&app, dead),
            LifeState::Dead,
            "a Dead ganger stays Dead"
        );

        // The Downed control DID bleed — proving the tick ran and only Downed drain.
        assert_eq!(
            wounds_of(&app, downed),
            start - rate,
            "the Downed control must bleed by exactly bleed_rate (the tick ran)",
        );
        assert_eq!(
            bleeding_count_for(&app, downed),
            1,
            "the Downed control must emit exactly one Bleeding",
        );
    }

    /// AC4 — a `Stabilized(true)` Downed ganger is SKIPPED: a tick drains nothing AND
    /// any previously-lost Wounds stay lost. Spawns a Downed ganger at a partial pool
    /// (modelling earlier bleeding), stabilizes it, ticks, and asserts no further drop
    /// and no `Bleeding`.
    #[test]
    fn stabilized_downed_ganger_is_skipped() {
        let rate = bleed_rate();
        // A partial pool — Wounds already lost before stabilizing (e.g. some bled),
        // chosen above the rate so a drain (if it wrongly ran) would be measurable.
        let partial = rate.saturating_add(2);

        let mut app = bleed_app();
        let ganger = app
            .world_mut()
            .spawn((
                Wounds::new(partial),
                LifeState::Downed,
                Stabilized::new(true),
            ))
            .id();

        app.update();

        assert_eq!(
            wounds_of(&app, ganger),
            partial,
            "a stabilized ganger must lose NO further Wounds (the already-lost stay lost)",
        );
        assert_eq!(
            bleeding_count_for(&app, ganger),
            0,
            "a stabilized ganger must emit no Bleeding",
        );
        assert_eq!(
            life_of(&app, ganger),
            LifeState::Downed,
            "a stabilized ganger remains Downed",
        );
    }

    /// AC4 (flag value matters) — `Stabilized(false)` does NOT skip: a Downed ganger
    /// carrying the flag set `false` still bleeds (the skip is the flag being present
    /// AND true, not merely present). Distinguishes "has the component" from "is
    /// stabilized".
    #[test]
    fn stabilized_false_still_bleeds() {
        let rate = bleed_rate();
        let start = rate.saturating_add(10);

        let mut app = bleed_app();
        let ganger = app
            .world_mut()
            .spawn((
                Wounds::new(start),
                LifeState::Downed,
                Stabilized::new(false),
            ))
            .id();

        app.update();

        assert_eq!(
            wounds_of(&app, ganger),
            start - rate,
            "Stabilized(false) must NOT skip — the ganger still bleeds by bleed_rate",
        );
        assert_eq!(
            bleeding_count_for(&app, ganger),
            1,
            "a Stabilized(false) Downed ganger still emits one Bleeding",
        );
    }

    /// AC5 — the `Stabilized` flag is DEFINED and OWNED here (re-exported from
    /// `lib.rs`): spawn an entity carrying `Stabilized` alone and query it back. This
    /// is the home-of-the-flag proof — an entity with no other ganger component still
    /// carries and yields its `Stabilized`. (The deeper re-export/Deref pins live in
    /// `ganger::tests`; this confirms it round-trips through a spawned entity here.)
    #[test]
    fn stabilized_is_owned_here_and_queryable_alone() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        let ganger = app.world_mut().spawn(Stabilized::new(true)).id();

        let flag = app.world().get::<Stabilized>(ganger).copied();
        assert_eq!(
            flag,
            Some(Stabilized::new(true)),
            "an entity carrying Stabilized alone must query its flag back (the flag's home)",
        );
    }

    /// AC6 — `Bleeding` is a buffered `#[derive(Message)]` (NOT the observer `Event`
    /// API, `bevy-traps.md` #4), read with a `MessageReader`: one per draining Downed
    /// ganger carrying the right `Entity`, and none for a stabilized or alive ganger.
    /// Spawns a Downed, a Stabilized-Downed, and an Alive ganger, ticks once, and
    /// asserts exactly one captured `Bleeding` — for the bleeding ganger only.
    #[test]
    fn bleeding_is_a_buffered_message_one_per_draining_ganger() {
        let rate = bleed_rate();
        let start = rate.saturating_add(10);

        let mut app = bleed_app();
        let bleeder = app
            .world_mut()
            .spawn((Wounds::new(start), LifeState::Downed))
            .id();
        let stabilized = app
            .world_mut()
            .spawn((Wounds::new(start), LifeState::Downed, Stabilized::new(true)))
            .id();
        let alive = app
            .world_mut()
            .spawn((Wounds::new(start), LifeState::Alive))
            .id();

        app.update();

        let captured = app
            .world()
            .get_resource::<Captured>()
            .map_or_else(Vec::new, |c| c.0.clone());

        // Exactly one Bleeding overall — only the un-stabilized Downed ganger bled.
        assert_eq!(
            captured.len(),
            1,
            "exactly one Bleeding must be buffered (one per draining Downed ganger)",
        );
        assert_eq!(
            captured.first(),
            Some(&Bleeding::new(bleeder)),
            "the buffered Bleeding must carry the bleeding ganger's Entity",
        );
        // None for the stabilized or alive ganger.
        assert_eq!(
            bleeding_count_for(&app, stabilized),
            0,
            "a stabilized Downed ganger emits no Bleeding",
        );
        assert_eq!(
            bleeding_count_for(&app, alive),
            0,
            "an Alive ganger emits no Bleeding",
        );
    }

    /// The lethal tick still emits a `Bleeding`: the draining-to-empty tick both
    /// emits the per-tick signal AND flips the ganger to Dead (drain + emit + gate on
    /// the same tick, including the lethal one). Spawns a ganger with exactly one
    /// round of bleed left and asserts both happen on that single tick.
    #[test]
    fn the_lethal_tick_emits_bleeding_and_flips_to_dead() {
        let rate = bleed_rate();
        // Exactly one round of bleed left, so this single tick is the lethal one.
        let start = rate;

        let mut app = bleed_app();
        let ganger = app
            .world_mut()
            .spawn((Wounds::new(start), LifeState::Downed))
            .id();

        app.update();

        assert_eq!(
            wounds_of(&app, ganger),
            0,
            "the lethal tick must drain the pool to 0",
        );
        assert_eq!(
            life_of(&app, ganger),
            LifeState::Dead,
            "the lethal tick must flip the ganger to Dead (the terminal gate)",
        );
        assert_eq!(
            bleeding_count_for(&app, ganger),
            1,
            "the lethal tick still emits one Bleeding (drain + emit + gate on the same tick)",
        );
    }

    /// The once-only property end to end: a ganger bled to death is skipped by the
    /// NEXT tick — no second drain, no second `Bleeding`. After the lethal tick (Dead,
    /// Wounds 0), a further `update()` must mutate nothing and emit nothing.
    #[test]
    fn a_dead_bled_out_ganger_is_skipped_next_tick() {
        let rate = bleed_rate();
        let start = rate; // one round to death

        let mut app = bleed_app();
        let ganger = app
            .world_mut()
            .spawn((Wounds::new(start), LifeState::Downed, Hp::new(0)))
            .id();

        // Tick 1: the lethal tick — Dead, Wounds 0, one Bleeding.
        app.update();
        assert_eq!(life_of(&app, ganger), LifeState::Dead);
        assert_eq!(bleeding_count_for(&app, ganger), 1);

        // Tick 2: the corpse is skipped — still one Bleeding total, still Dead.
        app.update();
        assert_eq!(
            life_of(&app, ganger),
            LifeState::Dead,
            "a dead ganger stays Dead across the next tick",
        );
        assert_eq!(
            bleeding_count_for(&app, ganger),
            1,
            "a dead (bled-out) ganger emits NO second Bleeding (the once-only property)",
        );
    }
}
