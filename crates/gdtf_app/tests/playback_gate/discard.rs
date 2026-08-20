use bevy::{ecs::system::RunSystemOnce, prelude::*};
use gdtf_battle_presenter::{ActiveLevel, PlaybackCursor, PlaybackTuning, ViewMode};
use gdtf_battle_sim::{
    act_log::{ActDeed, ActLog, ActProvenance, ActWitnesses, RecordedAct},
    acts::SetStanceRequested,
    ganger::{Aiming, Direction, Facing, Faction, LifeState, Position, Stance, StanceKind},
    metric::{Cell, CellLevel, Level},
};

use super::support::*;

#[test]
fn an_act_intent_pushed_while_catching_up_is_discarded_not_deferred() {
    let mut app = gated_app();
    let actor = spawn_actor(&mut app);
    select(&mut app, actor);

    close_gate(&mut app, actor);
    assert!(!gate_open(&mut app), "the fixture must start gated shut");

    push_stance_cycle(&mut app);
    drain(&mut app);
    assert_eq!(
        stance_requests(&mut app),
        0,
        "an act pushed while the presenter is catching up must not be dispatched",
    );

    open_gate(&mut app);
    assert!(gate_open(&mut app), "the fixture must now be caught up");
    drain(&mut app);
    assert_eq!(
        stance_requests(&mut app),
        0,
        "the discarded intent must NOT resurface when the gate opens — a `run_if` on the \
         drain would have accumulated it and fired it here, in a burst",
    );

    push_stance_cycle(&mut app);
    drain(&mut app);
    assert_eq!(
        stance_requests(&mut app),
        1,
        "with the screen caught up, an act dispatches normally",
    );
}

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

fn gated_app() -> App {
    let mut app = App::new();
    app.init_resource::<PlaybackCursor>();
    app.init_resource::<PlaybackTuning>();
    install_intent_drain(&mut app);
    app
}

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

fn close_gate(app: &mut App, actor: Entity) {
    let mut log = ActLog::default();
    log.append(RecordedAct::new(
        actor,
        ActProvenance::Commanded,
        ActDeed::BleedStarted,
        ActWitnesses::unseen(),
    ));
    app.insert_resource(log);
}

fn open_gate(app: &mut App) {
    app.insert_resource(ActLog::default());
}

fn gate_open(app: &mut App) -> bool {
    app.world_mut()
        .run_system_once(gdtf_battle_presenter::playback_caught_up)
        .unwrap_or(false)
}

fn stance_requests(app: &mut App) -> usize {
    drained::<SetStanceRequested>(app)
}
