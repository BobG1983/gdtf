//! T23 — clause (d): an app with NO presenter dispatches acts exactly as it always did.

use bevy::{app::App, ecs::system::RunSystemOnce};
use gdtf_battle_sim::{
    acts::SetStanceRequested,
    ganger::{Aiming, Direction, Facing, Faction, LifeState, Position, Stance, StanceKind},
    metric::{Cell, CellLevel, Level},
};

use super::support::*;

/// **T23 — THE FAIL-OPEN GUARD.** With no playback cursor and no act log anywhere, an act
/// intent dispatches normally.
///
/// This guards the single most dangerous line in the change. Every other absent-resource
/// check in this codebase fails CLOSED, and copying that habit here would be catastrophic:
/// a headless test app, an autobattle run, and any harness without the top-down renderer
/// have no cursor at all, so a fail-closed gate would refuse every act in all of them —
/// forever, with nothing that could ever open it. Gating is a presenter affordance; where
/// there is no presenter there is nothing to wait for.
#[test]
fn an_app_without_a_presenter_dispatches_acts_normally() {
    let mut app = App::new();
    install_intent_drain(&mut app);
    // Deliberately NO PlaybackCursor and NO ActLog — the presenter-less shape.
    assert!(
        app.world()
            .get_resource::<gdtf_battle_presenter::PlaybackCursor>()
            .is_none(),
        "the fixture must genuinely have no cursor",
    );
    assert!(
        app.world()
            .get_resource::<gdtf_battle_sim::act_log::ActLog>()
            .is_none(),
        "the fixture must genuinely have no act log",
    );

    let caught_up = app
        .world_mut()
        .run_system_once(gdtf_battle_presenter::playback_caught_up)
        .unwrap_or(false);
    assert!(
        caught_up,
        "with no cursor and no log the gate must read OPEN — a fail-closed default here is \
         a permanent deadlock in every presenter-less app",
    );

    let actor = app
        .world_mut()
        .spawn((
            Position::new(CellLevel::new(Cell::new(1, 1), Level::new(0))),
            Facing::new(Direction::North),
            Stance::new(StanceKind::Standing),
            Aiming::new(false),
            LifeState::Alive,
            Faction::new(0),
        ))
        .id();
    select(&mut app, actor);

    push_stance_cycle(&mut app);
    drain(&mut app);
    assert_eq!(
        drained::<SetStanceRequested>(&mut app),
        1,
        "an act must dispatch normally in a presenter-less app — the pacing simply does not \
         exist there",
    );
}
