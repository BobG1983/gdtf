use bevy::{
    platform::collections::HashSet,
    prelude::{App, Visibility, With},
};
use gdtf_battle_presenter::{
    ActiveLevel, CrossLevelBadgeKind, CrossLevelBadgeTile, CrossLevelSignals,
};
use gdtf_battle_sim::{
    battle::PlayerFaction,
    prelude::{Cell, CellLevel, Faction, Level, LifeState, Position},
    visibility::SquadVisibility,
};

use super::harness::{settle, signals_app};

fn key(x: i32, y: i32, z: u8) -> CellLevel {
    CellLevel::new(Cell::new(x, y), Level::new(z))
}

fn spawn_ganger(app: &mut App, at: CellLevel, faction: Faction, life: LifeState) {
    app.world_mut().spawn((Position::new(at), faction, life));
}

#[test]
fn visible_enemy_above_emits_threat_and_the_draw_renders_it_unseen_does_not() {
    let mut app = signals_app();
    app.world_mut()
        .insert_resource(PlayerFaction::new(Faction::new(0)));
    app.world_mut()
        .insert_resource(ActiveLevel::new(Level::new(0)));

    let visible_enemy = key(10, 10, 2);
    let unseen_enemy = key(20, 20, 3);
    let visible: HashSet<CellLevel> = std::iter::once(visible_enemy).collect();
    app.world_mut()
        .insert_resource(SquadVisibility::new(visible.clone(), visible));

    spawn_ganger(&mut app, visible_enemy, Faction::new(1), LifeState::Alive);
    spawn_ganger(&mut app, unseen_enemy, Faction::new(1), LifeState::Alive);

    settle(&mut app);

    let badges: Vec<CrossLevelBadgeKind> = app
        .world()
        .resource::<CrossLevelSignals>()
        .badges_at(Cell::new(10, 10))
        .to_vec();
    assert!(
        badges.iter().any(|b| matches!(
            *b,
            CrossLevelBadgeKind::Threat { delta, .. } if delta.is_above() && delta.magnitude() == 2
        )),
        "the squad-VISIBLE enemy 2 storeys above must emit a Threat badge, got {badges:?}",
    );
    assert!(
        app.world()
            .resource::<CrossLevelSignals>()
            .badges_at(Cell::new(20, 20))
            .is_empty(),
        "the UNSEEN enemy must NEVER leak a threat badge",
    );

    let visible_tiles = app
        .world_mut()
        .query_filtered::<&Visibility, With<CrossLevelBadgeTile>>()
        .iter(app.world())
        .filter(|v| **v == Visibility::Visible)
        .count();
    assert!(
        visible_tiles >= 1,
        "the draw system must render at least one Visible badge tile",
    );
}
