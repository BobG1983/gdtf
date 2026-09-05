use bevy::prelude::*;
use cobalt_test_utils::press_left;
use gdtf_battle_input::InspectTarget;
use gdtf_battle_sim::{
    ganger::{Aiming, TuMax},
    magazine::{LoadedRounds, Magazine, ReloadTu},
    prelude::{CellLevel, Direction, Level, LifeState, OccupancyGrid, StanceKind, Tu},
    tuning::CombatTuning,
    visibility::SquadVisibility,
    weapon::{FireMode, MagazineSize, ModeKind},
};

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

/// Rewrite `ganger`'s gun with `modes`, leaving everything else on it alone.
fn rewrite_gun(app: &mut App, ganger: Entity, modes: FireMode) {
    let Some(weapon) = gun_of(app, ganger) else {
        return;
    };
    app.world_mut().entity_mut(weapon).insert(modes);
}

#[test]
fn a_ganger_keeps_the_mode_picked_for_it_across_a_reselect() {
    let mut app = acts_app();
    let first = armed_ganger(
        &mut app,
        sbf_selector(),
        StanceKind::Standing,
        Direction::North,
    );
    let second = armed_ganger(
        &mut app,
        sbf_selector(),
        StanceKind::Standing,
        Direction::North,
    );
    let burst = spec(ModeKind::Burst, 0.4, 3);

    select_ganger_at(&mut app, first, SHOOTER_CURSOR_OFFSET);
    pick_mode(&mut app, first, ModeKind::Burst);
    app.update();
    assert_eq!(
        mode_of(&app, first),
        Some(burst),
        "the pick has to land before the case can ask whether it survives",
    );

    select_ganger_at(&mut app, second, TARGET_CURSOR_OFFSET);
    app.update();
    select_ganger_at(&mut app, first, SHOOTER_CURSOR_OFFSET);
    app.update();

    assert_eq!(
        mode_of(&app, first),
        Some(burst),
        "the mode lives on the gun the ganger holds, so selecting away and back leaves it on \
         Burst; the gun's single here means something put it back",
    );
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
        mode_of(&app, ganger),
        Some(selector.single()),
        "a gun with no mode picked for it reads as its own FireMode::single()",
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
    assert_eq!(
        msg.mode,
        selector.single(),
        "mode = the spec the gun the shooter fires is set to",
    );
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
fn a_fire_request_carries_the_spec_the_gun_holds_now() {
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
    let rewritten = spec(ModeKind::Single, 0.35, 1);
    rewrite_gun(&mut app, ganger, FireMode::new(vec![rewritten]));
    assert_ne!(
        rewritten,
        sbf_selector().single(),
        "the rewrite has to change the spec, or the case cannot tell which one was carried",
    );

    press_left(&mut app);
    app.update();

    let emitted = fires(&app);
    assert_eq!(
        emitted.len(),
        1,
        "exactly one FireRequested must be emitted by a left-click on an in-bounds target",
    );
    assert_eq!(
        emitted[0].mode, rewritten,
        "the request carries the entry the gun holds now, so the spec it was armed with here \
         means the shot was built from a copy taken earlier",
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
