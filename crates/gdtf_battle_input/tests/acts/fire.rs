use bevy::prelude::*;
use gdtf_battle_input::{InspectTarget, SelectedFireMode};
use gdtf_battle_sim::{
    ganger::{Aiming, TuMax},
    magazine::{LoadedRounds, Magazine, ReloadTu},
    prelude::{CellLevel, Direction, Level, LifeState, OccupancyGrid, StanceKind, Tu},
    tuning::CombatTuning,
    visibility::SquadVisibility,
    weapon::{FireModeSpec, MagazineSize},
};
use gdtf_test_utils::press_left;

use super::harness::*;

fn empty_wielded_magazine(app: &mut App, ganger: Entity) {
    if let Some(weapon) = app
        .world()
        .get::<gdtf_battle_sim::weapon::Wields>(ganger)
        .and_then(gdtf_battle_sim::weapon::Wields::weapon)
    {
        app.world_mut().entity_mut(weapon).insert(Magazine::new(
            LoadedRounds::new(0),
            MagazineSize::new(30),
            ReloadTu::new(12),
        ));
    }
}

fn place_enemy(app: &mut App, cell: CellLevel) -> Entity {
    let enemy = app.world_mut().spawn(ENEMY_FACTION).id();
    app.world_mut()
        .resource_mut::<OccupancyGrid>()
        .set_occupant(cell, Some(enemy));
    if let Some(fog) = app.world_mut().get_resource::<SquadVisibility>() {
        let mut visible: bevy::platform::collections::HashSet<CellLevel> =
            fog.visible_cells().copied().collect();
        let mut explored: bevy::platform::collections::HashSet<CellLevel> =
            fog.explored_cells().copied().collect();
        visible.insert(cell);
        explored.insert(cell);
        app.world_mut()
            .insert_resource(SquadVisibility::new(visible, explored));
    }
    enemy
}

fn fire_mode(app: &App) -> Option<FireModeSpec> {
    app.world().get_resource::<SelectedFireMode>().map(|m| **m)
}


#[test]
fn selecting_armed_ganger_defaults_fire_mode_to_single() {
    let mut app = acts_app();
    let selector = sbf_selector();
    let ganger = armed_ganger(
        &mut app,
        selector.clone(),
        StanceKind::Standing,
        Direction::North,
    );

    select_ganger(&mut app, ganger);

    assert_eq!(
        fire_mode(&app),
        Some(selector.single()),
        "selecting an armed ganger must default SelectedFireMode to its FireMode::single()",
    );
}



#[test]
fn left_click_emits_one_fire_requested() {
    let mut app = acts_app();
    add_probes(&mut app);
    let selector = sbf_selector();
    let ganger = armed_ganger(
        &mut app,
        selector.clone(),
        StanceKind::Standing,
        Direction::North,
    );

    select_ganger(&mut app, ganger);
    let target = hover_at(&mut app, TARGET_CURSOR_OFFSET);
    let target_cell = target.cell();
    let target_level = Level::new(0);
    place_enemy(&mut app, target);

    press_left(&mut app);
    app.update();

    let emitted = fires(&app);
    assert_eq!(
        emitted.len(),
        1,
        "exactly one FireRequested must be emitted by a left-click on an in-bounds target",
    );
    let msg = &emitted[0];
    assert_eq!(msg.shooter, ganger, "shooter = *SelectedShooter");
    assert_eq!(msg.mode, selector.single(), "mode = *SelectedFireMode");
    assert_eq!(
        msg.target_cell, target_cell,
        "target cell from the hovered cell"
    );
    assert_eq!(
        msg.target_level, target_level,
        "target level from the hovered cell"
    );
}


#[test]
fn can_fire_failure_blocks_fire_requested() {
    {
        let mut app = acts_app();
        add_probes(&mut app);
        let ganger = armed_ganger(
            &mut app,
            sbf_selector(),
            StanceKind::Standing,
            Direction::North,
        );
        select_ganger(&mut app, ganger);
        let target = hover_at(&mut app, TARGET_CURSOR_OFFSET);
        place_enemy(&mut app, target);
        app.world_mut().entity_mut(ganger).insert(LifeState::Downed);
        press_left(&mut app);
        app.update();
        assert!(
            fires(&app).is_empty(),
            "a Downed shooter must emit no FireRequested"
        );
    }

    {
        let mut app = acts_app();
        add_probes(&mut app);
        let ganger = armed_ganger(
            &mut app,
            sbf_selector(),
            StanceKind::Standing,
            Direction::North,
        );
        select_ganger(&mut app, ganger);
        let target = hover_at(&mut app, TARGET_CURSOR_OFFSET);
        place_enemy(&mut app, target);
        empty_wielded_magazine(&mut app, ganger);
        press_left(&mut app);
        app.update();
        assert!(
            fires(&app).is_empty(),
            "an empty magazine must emit no FireRequested"
        );
    }

    {
        let mut app = acts_app();
        add_probes(&mut app);
        let ganger = armed_ganger(
            &mut app,
            sbf_selector(),
            StanceKind::Standing,
            Direction::North,
        );
        select_ganger(&mut app, ganger);
        let target = hover_at(&mut app, TARGET_CURSOR_OFFSET);
        place_enemy(&mut app, target);
        let charge = gdtf_battle_sim::magazine::mode_tu_cost(
            &sbf_selector().single(),
            &TuMax::new(100),
            &Aiming::new(false),
            &CombatTuning::default(),
        );
        assert!(*charge > 0, "the mode charge must be positive for the test");
        app.world_mut()
            .entity_mut(ganger)
            .insert(Tu::new(charge.saturating_sub(1)));
        press_left(&mut app);
        app.update();
        assert!(
            fires(&app).is_empty(),
            "TU one below the mode charge must emit no FireRequested",
        );
    }

    {
        let mut app = acts_app();
        add_probes(&mut app);
        let ganger = armed_ganger(
            &mut app,
            sbf_selector(),
            StanceKind::Standing,
            Direction::North,
        );
        select_ganger(&mut app, ganger);
        set_cursor(&mut app, None);
        app.update();
        assert_eq!(
            app.world()
                .get_resource::<InspectTarget>()
                .and_then(InspectTarget::hovered),
            None,
            "an off-window cursor must resolve the hovered cell to None",
        );
        press_left(&mut app);
        app.update();
        assert!(
            fires(&app).is_empty(),
            "no in-bounds target (off-window cursor) must emit no FireRequested",
        );
    }
}
