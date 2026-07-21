//! T16 / T17 — act intents are DISCARDED (never deferred) while the presenter catches up,
//! and the view controls stay live throughout.

use bevy::{ecs::system::RunSystemOnce, prelude::*};
use gdtf_battle_presenter::{ActiveLevel, PlaybackCursor, PlaybackTuning, ViewMode};
use gdtf_battle_sim::{
    act_log::{ActDeed, ActLog, ActProvenance, RecordedAct},
    acts::SetStanceRequested,
    ganger::{Aiming, Direction, Facing, Faction, LifeState, Position, Stance, StanceKind},
    metric::{Cell, CellLevel, Level},
};

use super::support::*;

/// **T16 — DISCARDED, NOT DEFERRED.** An act intent pushed while the presenter is catching
/// up must NEVER be dispatched — not while the gate is shut, and not later when it opens.
///
/// This is the ruling the whole gate rests on. The obvious implementation — put a `run_if`
/// on the drain — is wrong in a way that looks right: `PendingActIntent::drain` is a
/// `mem::take`, so a skipped drain does not defer the intents, it ACCUMULATES them, and the
/// instant the gate opens the whole backlog fires at once. That is still acting on
/// information the player never saw, just later and in a burst.
///
/// So the drain runs unconditionally and discards per arm. An intent formed against a stale
/// screen is stale whenever it would execute.
#[test]
fn an_act_intent_pushed_while_catching_up_is_discarded_not_deferred() {
    let mut app = gated_app();
    let actor = spawn_actor(&mut app);
    select(&mut app, actor);

    // The gate is SHUT: the log holds an unplayed entry.
    close_gate(&mut app, actor);
    assert!(!gate_open(&mut app), "the fixture must start gated shut");

    // Push an act-bearing intent while shut, and drain.
    push_stance_cycle(&mut app);
    drain(&mut app);
    assert_eq!(
        stance_requests(&mut app),
        0,
        "an act pushed while the presenter is catching up must not be dispatched",
    );

    // OPEN the gate and drain again. Nothing may fire now either — the intent was dropped
    // at the drain, not parked in the queue.
    open_gate(&mut app);
    assert!(gate_open(&mut app), "the fixture must now be caught up");
    drain(&mut app);
    assert_eq!(
        stance_requests(&mut app),
        0,
        "the discarded intent must NOT resurface when the gate opens — a `run_if` on the \
         drain would have accumulated it and fired it here, in a burst",
    );

    // And the gate really is open: an intent pushed NOW does dispatch, so the test is not
    // passing merely because nothing works.
    push_stance_cycle(&mut app);
    drain(&mut app);
    assert_eq!(
        stance_requests(&mut app),
        1,
        "with the screen caught up, an act dispatches normally",
    );
}

/// **T17 — THE POSITIVE HALF.** The presenter-owned VIEW controls stay live while the gate
/// is shut, asserted positively so an over-broad gate regresses loudly.
///
/// Cycling the drawn storey and toggling the full view change nothing in the world — and,
/// more to the point, they are the player's means of WATCHING the reaction fire that closed
/// the gate in the first place. A gate that blocked them would remove the very affordance
/// this feature exists to serve.
#[test]
fn hover_and_level_cycling_stay_live_while_the_gate_is_closed() {
    let mut app = gated_app();
    let actor = spawn_actor(&mut app);
    close_gate(&mut app, actor);
    assert!(!gate_open(&mut app), "the fixture must be gated shut");

    let level_before = **app.world().resource::<ActiveLevel>();
    let mode_before = *app.world().resource::<ViewMode>();

    push_level_up(&mut app);
    push_toggle_full_view(&mut app);
    drain(&mut app);

    assert_ne!(
        **app.world().resource::<ActiveLevel>(),
        level_before,
        "cycling the drawn storey must still work while the presenter is catching up — it \
         changes nothing in the world and it is how the player watches the exchange",
    );
    assert_ne!(
        *app.world().resource::<ViewMode>(),
        mode_before,
        "the full-view toggle must stay live for the same reason",
    );
}

/// Build the focused gate app: the intent queue, the drain's reads, and the playback
/// resources the gate keys on.
fn gated_app() -> App {
    let mut app = App::new();
    app.init_resource::<PlaybackCursor>();
    app.init_resource::<PlaybackTuning>();
    install_intent_drain(&mut app);
    app
}

/// Spawn a stand-in actor carrying the posture components the drain reads.
fn spawn_actor(app: &mut App) -> Entity {
    app.world_mut()
        .spawn((
            Position::new(CellLevel::new(Cell::new(1, 1), Level::new(0))),
            Facing::new(Direction::North),
            Stance::new(StanceKind::Standing),
            Aiming::new(false),
            LifeState::Alive,
            Faction::new(0),
        ))
        .id()
}

/// SHUT the gate: insert a log holding one unplayed entry, so the cursor is behind.
fn close_gate(app: &mut App, actor: Entity) {
    let mut log = ActLog::default();
    log.append(RecordedAct::new(
        actor,
        ActProvenance::Commanded,
        ActDeed::BleedStarted,
    ));
    app.insert_resource(log);
}

/// OPEN the gate: replace the log with an empty one, so the cursor is level with its head.
fn open_gate(app: &mut App) {
    app.insert_resource(ActLog::default());
}

/// Whether the gate currently reads open (through the REAL exported run condition).
fn gate_open(app: &mut App) -> bool {
    app.world_mut()
        .run_system_once(gdtf_battle_presenter::playback_caught_up)
        .unwrap_or(false)
}

/// The stance-request payload the drain writes for a cycle.
fn stance_requests(app: &mut App) -> usize {
    drained::<SetStanceRequested>(app)
}
