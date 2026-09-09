use bevy::prelude::*;
use cobalt_test_utils::{clear_keys, clear_mouse, press_key, press_left, probed};
use gdtf_battle_input::PathPreviewTarget;
use gdtf_battle_sim::{
    acts::MoveRequested,
    prelude::{CellLevel, Direction, Level, StanceKind},
    test_support::SituationBuilder,
    vertical::{LinkKind, VerticalLink, build_vertical_link_graph},
};

use super::harness::*;

fn moves(app: &App) -> Vec<MoveRequested> {
    probed::<MoveRequested>(app)
}

fn move_target(app: &App) -> Option<CellLevel> {
    app.world()
        .get_resource::<PathPreviewTarget>()
        .and_then(|t| **t)
}

fn click_left(app: &mut App) {
    press_left(app);
    app.update();
    clear_mouse(app);
}

fn switch_level_up(app: &mut App) -> i32 {
    press_key(app, test_keybinds().level_up());
    app.update();
    clear_keys(app);
    active_storey(app)
}

#[test]
fn two_click_sets_target_then_commits_move() {
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

    click_left(&mut app);
    assert!(
        moves(&app).is_empty(),
        "click-1 on a valid target must emit NO MoveRequested (it only sets the target)",
    );
    assert_eq!(
        move_target(&app),
        Some(target),
        "click-1 must SET PathPreviewTarget to the clicked cell",
    );

    click_left(&mut app);
    let emitted = moves(&app);
    assert_eq!(
        emitted.len(),
        1,
        "click-2 on the SAME cell must emit exactly one MoveRequested (commit)",
    );
    assert_eq!(emitted[0].actor, ganger, "the move actor = the selection");
    assert_eq!(emitted[0].dest, target, "the move dest = the targeted cell");
    assert_eq!(
        move_target(&app),
        None,
        "committing the move must CLEAR PathPreviewTarget",
    );
}

#[test]
fn two_click_default_targets_active_level() {
    let mut app = acts_app();
    add_probes(&mut app);
    let ganger = armed_ganger(
        &mut app,
        sbf_selector(),
        StanceKind::Standing,
        Direction::North,
    );
    select_ganger(&mut app, ganger);

    let active = active_storey(&app);
    let target = hover_at(&mut app, TARGET_CURSOR_OFFSET);
    assert_eq!(
        target.z, active,
        "with no switch the hovered cell resolves at the DEFAULT active storey",
    );

    click_left(&mut app);
    click_left(&mut app);
    let emitted = moves(&app);
    assert_eq!(
        emitted.len(),
        1,
        "the same-cell second click commits a move"
    );
    assert_eq!(
        emitted[0].dest.z, active,
        "the default move dest.z is the active storey (no switch)",
    );
}

#[test]
fn two_click_after_level_switch_targets_switched_storey() {
    let mut app = acts_app();
    add_probes(&mut app);
    let ganger = armed_ganger(
        &mut app,
        sbf_selector(),
        StanceKind::Standing,
        Direction::North,
    );
    select_ganger(&mut app, ganger);

    let switched = switch_level_up(&mut app);
    assert_ne!(
        switched, 0,
        "the level key must raise ActiveLevel off storey 0",
    );

    let target = hover_at(&mut app, TARGET_CURSOR_OFFSET);
    assert_eq!(
        target.z, switched,
        "the hovered cell resolves at the switched active storey",
    );

    click_left(&mut app);
    click_left(&mut app);
    let emitted = moves(&app);
    assert_eq!(
        emitted.len(),
        1,
        "the same-cell second click commits a move"
    );
    assert_eq!(
        emitted[0].dest.z, switched,
        "the committed move dest.z == the SWITCHED storey (cross-storey already flows)",
    );
}

#[test]
fn click_on_a_link_tile_is_not_a_move_target() {
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
    let up_storey = (*target.level()).saturating_add(1);
    let up = CellLevel::new(target.cell(), Level::new(up_storey));
    let link = VerticalLink::new(target, up, LinkKind::stair());
    let graph = SituationBuilder::new()
        .slab_at(target)
        .slab_at(up)
        .vertical_link(link)
        .build();
    if let Ok(graph) = build_vertical_link_graph(&graph) {
        app.world_mut().insert_resource(graph);
    }

    click_left(&mut app);
    click_left(&mut app);
    assert!(
        moves(&app).is_empty(),
        "a click on a vertical-link tile must emit NO MoveRequested (OQ-4: not a move target)",
    );
    assert_eq!(
        move_target(&app),
        None,
        "a click on a vertical-link tile must set NO PathPreviewTarget (OQ-4: not a target)",
    );
}
