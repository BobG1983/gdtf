//! [`move_on`] — advance `BattleRunning → AnimateOut`, DEFERRED past the deciding shot's FX
//! (GTW-334).
//!
//! The sim's victory census (`check_outcome`, GTW-237) emits its `BattleWon` / `BattleLost`
//! the SAME drain frame the killing shot resolves: `fire()` applies the damage, the struck
//! ganger goes [`Dead`](gdtf_battle_sim::LifeState::Dead), the census declares the outcome,
//! and the killing shot's [`ShotFired`](gdtf_battle_sim::ShotFired) is emitted — all one drain.
//! [`end_battle_on_outcome`](super::end_battle_on_outcome) latches the
//! [`BattleRunningComplete`](super::super::resources::BattleRunningComplete) marker that frame; that latch is the correct "outcome DECIDED"
//! signal and is UNCHANGED.
//!
//! The BUG (GTW-334) was that the marker-gated transition fired the SAME frame the outcome was
//! decided — before the deciding tracer animates. The presenter spawns the killing bolt a beat
//! later ([`spawn_shot_projectiles`](gdtf_battle_presenter::spawn_shot_projectiles) reads
//! `ShotFired`), flies it at constant velocity ([`advance_projectiles`](gdtf_battle_presenter::advance_projectiles)),
//! and resolves its impact when it lands ([`animate_impact`](gdtf_battle_presenter::animate_impact)).
//! So the battle must leave `BattleRunning` only once that FX pipeline has DRAINED for the
//! deciding shot — not at the drain.
//!
//! This module DEFERS only the transition (the same `input → presenter` app-FX coupling
//! GTW-328's combat log already established + accepted; the sim, the census, and the latch are
//! untouched). It reuses the established presenter FX types — no new sim signal.
//!
//! SCAFFOLD DIVERGENCE (GTW-575): this `move_on` stays BESPOKE rather than collapsing into
//! `scaffold::advance_state_to` because it is not a bare `NextState::set` — it classifies the
//! end (deciding shot vs flee), persists the [`EndTransition`] phase, and holds the transition
//! until the FX pipeline drains (P9: a scene that outgrows the scaffold graduates to its own
//! system).

use bevy::prelude::*;
use gdtf_battle_presenter::{PendingImpact, ShotProjectile};
use gdtf_battle_sim::ShotFired;

use crate::states::BattleScapeState;

/// Where the deferred end-transition is in its decision (GTW-334) — the per-run latch the
/// [`move_on`] gate carries from the outcome-decided frame to the deciding shot's impact.
///
/// A typed phase (the no-bare-types discipline): a bare `bool` "should wait" could not tell the
/// FLEE / non-shot end (transition at once) from the in-flight deciding shot (wait for the
/// tracer) apart, which is exactly trap B. Inserted by [`move_on`] the first frame the
/// [`BattleRunningComplete`](super::super::resources::BattleRunningComplete) marker is present, and removed `OnExit(BattleRunning)` by
/// the plugin's `scaffold::remove_scoped_resource::<EndTransition>()` registration alongside
/// the marker (a per-run state-scoped resource, `bevy-traps.md` #1).
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::states::running::game::battlescape::battle_running) enum EndTransition {
    /// A deciding shot WAS in flight at the outcome-decided frame (its `ShotFired` was drained):
    /// the FX pipeline is expected to go busy (spawn the bolt) then drain (land the impact). The
    /// transition WAITS until the pipeline has gone busy since the latch AND then drained back to
    /// idle. The flag records whether the busy phase has been observed yet — idle does NOT count
    /// as "drained" until the pipeline has first been seen busy (closing the spawn-race, trap A:
    /// the deferred `spawn_shot_projectiles` materializes the bolt only on a LATER schedule, so
    /// the latch frame reads idle before the deciding bolt exists).
    AwaitingDecidingShot {
        /// Whether the FX pipeline has been observed BUSY (a bolt / impact present) since the
        /// latch. While `false`, an idle read is the pre-spawn gap, NOT a drained pipeline.
        seen_busy: bool,
    },
    /// NO deciding shot was in flight at the outcome-decided frame (a FLEE, a bleed-out census, or
    /// any non-projectile end — trap B): there is no tracer to wait on, so the transition is
    /// PROMPT. A sentinel phase so the gate transitions on the next frame without ever waiting for
    /// a pipeline that never goes busy.
    NoDecidingShot,
}

/// `Update` (gated `in_state(BattleRunning) && resource_exists::<BattleRunningComplete>`,
/// ordered `.after(PresenterSystems::Draw)` so this frame's FX spawn / advance / impact have
/// already run): advance `BattleRunning → AnimateOut` once the DECIDING shot's FX pipeline has
/// drained (GTW-334).
///
/// On the FIRST frame the [`BattleRunningComplete`](super::super::resources::BattleRunningComplete) latch is present it CLASSIFIES the end by
/// draining [`MessageReader<ShotFired>`](gdtf_battle_sim::ShotFired) (its own cursor, independent
/// of the presenter's projectile spawner): a `ShotFired` present that frame is the deciding shot
/// the sim emitted on the killing drain, so a tracer is coming — it records
/// [`EndTransition::AwaitingDecidingShot`]. With NO `ShotFired` it is a flee / non-shot end and it
/// records [`EndTransition::NoDecidingShot`].
///
/// `ShotFired` is the race-free discriminator, NOT a query of the in-flight bolt: the bolt is
/// spawned via a deferred `commands.spawn_scene` and its [`ShotProjectile`] / `ProjectileTravel`
/// materialize only on a LATER schedule, so they are NOT queryable on the latch frame the outcome
/// is decided — but the `ShotFired` message IS present that exact frame (the same insight GTW-331
/// uses). A naive "no projectile exists → end now" gate would fire in that pre-spawn gap (trap A);
/// classifying off `ShotFired` instead, and then requiring the pipeline to have gone busy BEFORE
/// idle counts as drained, closes it.
///
/// Thereafter:
/// - [`NoDecidingShot`](EndTransition::NoDecidingShot) → transition PROMPTLY (trap B: never hang
///   on a pipeline that never goes busy).
/// - [`AwaitingDecidingShot`](EndTransition::AwaitingDecidingShot) → transition once the FX
///   pipeline has been observed BUSY (a [`ShotProjectile`] in flight or a [`PendingImpact`]
///   outstanding) and has since drained back to idle.
///
/// Param-only (`bevy-traps.md` #7): [`Commands`] (insert the phase),
/// [`ResMut<NextState<BattleScapeState>>`] (the transition), [`Option<Res<EndTransition>>`] (the
/// per-run phase, absent on the first latch frame), the two FX-state queries
/// (`ShotProjectile` / `PendingImpact`), and the [`MessageReader<ShotFired>`] classifier.
pub(in crate::states::running::game::battlescape::battle_running) fn move_on(
    mut commands: Commands,
    mut state: ResMut<NextState<BattleScapeState>>,
    phase: Option<Res<EndTransition>>,
    projectiles: Query<(), With<ShotProjectile>>,
    pending: Query<(), With<PendingImpact>>,
    mut shots: MessageReader<ShotFired>,
) {
    // The FX pipeline is BUSY while any deciding-shot bolt is in flight OR any arrival seed is
    // outstanding (an impact not yet consumed by `animate_impact`). Once both are empty the
    // deciding shot has flown and resolved.
    let pipeline_busy = !projectiles.is_empty() || !pending.is_empty();

    // CLASSIFY the end on the first latch frame (no phase yet): a `ShotFired` drained this frame
    // is the deciding shot (the sim emits it on the SAME drain as the outcome), so a tracer is
    // coming; otherwise it is a flee / non-shot end. This frame's `ShotFired` MUST be read whether
    // or not the phase already exists, so the reader does not back up — but it only DECIDES the
    // phase on the first frame.
    let deciding_shot_fired = shots.read().next().is_some();
    let phase = if let Some(existing) = phase.as_deref().copied() {
        existing
    } else {
        // First latch frame — classify and persist the phase for the following frames.
        let initial = if deciding_shot_fired {
            EndTransition::AwaitingDecidingShot { seen_busy: false }
        } else {
            EndTransition::NoDecidingShot
        };
        commands.insert_resource(initial);
        initial
    };

    match phase {
        // No tracer to wait on — transition at once (trap B: prompt flee / non-shot end).
        EndTransition::NoDecidingShot => {
            state.set(BattleScapeState::AnimateOut);
        }
        EndTransition::AwaitingDecidingShot { seen_busy } => {
            if pipeline_busy {
                // The deciding bolt / impact is in flight — keep waiting, and record that the
                // pipeline has now been seen busy (so a later idle read is a true drain, not the
                // pre-spawn gap — trap A).
                if !seen_busy {
                    commands
                        .insert_resource(EndTransition::AwaitingDecidingShot { seen_busy: true });
                }
            } else if seen_busy {
                // The pipeline went busy and has now drained: the deciding tracer has landed and
                // its impact resolved — transition BattleRunning → AnimateOut, at the impact.
                state.set(BattleScapeState::AnimateOut);
            }
            // else: idle but never-yet-busy — the pre-spawn gap (the deferred bolt has not
            // materialized yet). Hold the transition; do NOT treat this idle as a drain.
        }
    }
}

#[cfg(test)]
mod test {
    use bevy::{
        app::App,
        ecs::system::RunSystemOnce,
        prelude::*,
        state::{app::StatesPlugin, state::State},
    };
    use gdtf_battle_presenter::ShotProjectile;
    use gdtf_battle_sim::ShotFired;

    use super::{EndTransition, move_on};
    use crate::states::{
        BattleScapeState,
        running::game::battlescape::battle_running::resources::BattleRunningComplete,
    };

    /// Builds a headless `MinimalPlugins` app with the `BattleScapeState` machine registered and
    /// rested at `BattleRunning`, with the `BattleRunningComplete` latch present — the exact gate
    /// `move_on` runs under. Drives the state transitions deterministically so the unit tests can
    /// pin `move_on`'s decision frame by frame WITHOUT the heavy real-asset FX harness (the
    /// integration repro in `tests/battle_end_at_impact.rs` proves it against the REAL pipeline).
    fn battle_running_gate_app() -> App {
        let mut app = App::new();
        app.add_plugins(StatesPlugin);
        app.init_state::<BattleScapeState>();
        app.add_message::<ShotFired>();
        app.insert_resource(BattleRunningComplete);
        // Enter BattleRunning so `state.set` writes against the live machine.
        app.world_mut()
            .resource_mut::<NextState<BattleScapeState>>()
            .set(BattleScapeState::BattleRunning);
        app.update();
        app
    }

    /// The current `BattleScapeState`.
    fn state_of(app: &App) -> BattleScapeState {
        *app.world().resource::<State<BattleScapeState>>().get()
    }

    /// Runs `move_on` once, then applies the queued state transition so the next read sees it.
    fn run_move_on(app: &mut App) {
        // `run_system_once` returns a `Result`; the system is infallible, so a failed run would be
        // a harness bug — assert it ran rather than dropping the result (clippy `let_underscore_drop`).
        let ran = app.world_mut().run_system_once(move_on);
        assert!(ran.is_ok(), "move_on must run once in the unit harness");
        // Apply the buffered NextState set (the StateTransition schedule runs each update).
        app.update();
    }

    /// Spawns a stand-in bolt (a `ShotProjectile`) so the FX-busy query reads busy.
    fn spawn_bolt(app: &mut App) -> Entity {
        app.world_mut().spawn(ShotProjectile).id()
    }

    /// FLEE / NON-SHOT END (trap B): with NO `ShotFired` drained on the latch frame and an idle FX
    /// pipeline, `move_on` classifies `NoDecidingShot` and transitions to `AnimateOut` PROMPTLY —
    /// it must not hang waiting for a pipeline that never goes busy.
    #[test]
    fn no_deciding_shot_transitions_promptly() {
        let mut app = battle_running_gate_app();
        // No ShotFired written, no bolt spawned: a flee / non-shot end.
        run_move_on(&mut app);
        assert_eq!(
            state_of(&app),
            BattleScapeState::AnimateOut,
            "a non-shot end (no ShotFired, idle pipeline) must transition to AnimateOut promptly",
        );
        // The classifier recorded the prompt phase.
        assert_eq!(
            app.world().get_resource::<EndTransition>().copied(),
            Some(EndTransition::NoDecidingShot),
            "a non-shot end must record the NoDecidingShot phase",
        );
    }

    /// SPAWN-RACE (trap A): a deciding `ShotFired` is drained on the latch frame but the deferred
    /// bolt has not materialized yet (the FX pipeline reads IDLE in the pre-spawn gap). `move_on`
    /// must NOT transition in that gap — it must hold until the pipeline has gone busy and then
    /// drained.
    #[test]
    fn a_deciding_shot_does_not_transition_in_the_pre_spawn_gap() {
        let mut app = battle_running_gate_app();
        // The deciding shot's ShotFired is present on the latch frame, but NO bolt exists yet
        // (the real spawn is deferred to a later schedule).
        let shot = deciding_shot(&mut app);
        app.world_mut()
            .resource_mut::<Messages<ShotFired>>()
            .write(shot);
        run_move_on(&mut app);
        assert_eq!(
            state_of(&app),
            BattleScapeState::BattleRunning,
            "a deciding shot whose bolt has not yet spawned (the pre-spawn gap) must NOT transition \
             — idle here is the gap, not a drained pipeline (trap A)",
        );
        assert_eq!(
            app.world().get_resource::<EndTransition>().copied(),
            Some(EndTransition::AwaitingDecidingShot { seen_busy: false }),
            "a deciding shot must record AwaitingDecidingShot, not yet seen-busy",
        );

        // Now the bolt materializes: still in flight, so still no transition (the pipeline is busy)
        // — and the phase flips to seen_busy.
        spawn_bolt(&mut app);
        run_move_on(&mut app);
        assert_eq!(
            state_of(&app),
            BattleScapeState::BattleRunning,
            "while the deciding bolt is in flight the transition must be deferred",
        );
        assert_eq!(
            app.world().get_resource::<EndTransition>().copied(),
            Some(EndTransition::AwaitingDecidingShot { seen_busy: true }),
            "once the pipeline is busy the phase must record seen_busy",
        );
    }

    /// AT THE IMPACT: once a deciding shot's pipeline has gone busy and then DRAINED (the bolt flew
    /// and its impact resolved), `move_on` transitions to `AnimateOut` — at the impact, not at the
    /// drain. (The busy phase is exercised here with a `ShotProjectile` bolt; the `PendingImpact`
    /// arm of "busy" is FX-sealed and covered by the real-pipeline integration repro in
    /// `tests/battle_end_at_impact.rs`.)
    #[test]
    fn a_deciding_shot_transitions_once_its_pipeline_drains() {
        let mut app = battle_running_gate_app();
        let shot = deciding_shot(&mut app);
        app.world_mut()
            .resource_mut::<Messages<ShotFired>>()
            .write(shot);
        // Bolt in flight: busy, no transition (seen_busy becomes true).
        let bolt = spawn_bolt(&mut app);
        run_move_on(&mut app);
        assert_eq!(
            state_of(&app),
            BattleScapeState::BattleRunning,
            "the transition must be deferred while the deciding bolt is in flight",
        );
        assert_eq!(
            app.world().get_resource::<EndTransition>().copied(),
            Some(EndTransition::AwaitingDecidingShot { seen_busy: true }),
            "an in-flight bolt must flip the phase to seen_busy",
        );
        // The bolt arrives, resolves, and the pipeline drains (no bolt, no impact seed left):
        // the deciding tracer has landed → transition BattleRunning → AnimateOut.
        app.world_mut().entity_mut(bolt).despawn();
        run_move_on(&mut app);
        assert_eq!(
            state_of(&app),
            BattleScapeState::AnimateOut,
            "once the deciding shot's pipeline has gone busy and then drained, the transition must \
             advance BattleRunning → AnimateOut (at the impact, not the drain)",
        );
    }

    /// A deciding `ShotFired` carrying no report — geometry-only is enough: ANY shot in flight at
    /// the outcome means a tracer is animating that the transition must wait on (the deciding-shot
    /// classification keys off the shot's PRESENCE, not its verdict).
    fn deciding_shot(app: &mut App) -> ShotFired {
        let shooter = app.world_mut().spawn_empty().id();
        let struck = app.world_mut().spawn_empty().id();
        ShotFired {
            shooter,
            muzzle: gdtf_battle_sim::SimPos::new(0.0, 0.0, 0.0),
            trajectory: gdtf_battle_sim::ShotDir::from_direction(Vec3::new(1.0, 0.0, 0.0)),
            impact_cell: gdtf_battle_sim::Cell::new(1, 0),
            impact_level: gdtf_battle_sim::Level::new(0),
            kind: gdtf_battle_sim::ShotKind::Ganger(struck),
            damage: gdtf_battle_sim::DamageType::Kinetic,
            report: None,
        }
    }
}
