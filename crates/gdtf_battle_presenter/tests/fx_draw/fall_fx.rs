//! Fall impact flash + Fell FCT pop (GTW-524).

use bevy::{ecs::message::Messages, transform::components::Transform};
use gdtf_battle_presenter::{FctValence, FloatingCombatText, cell_to_world, valence_color};
use gdtf_battle_sim::{
    falls::{FallOccurred, StoreysFallen},
    prelude::{BattleInProgress, Cell, CellLevel, Level, Position},
};

use super::{harness::*, probes::*};

/// GTW-524 C1/C3 — a `FallOccurred` spawns EXACTLY ONE `FxFlash` at `cell_to_world(landing
/// cell, to_level)` with the table's `fall_impact` atlas index (read structurally from
/// `EffectRoles`, never a literal) AND a `"Fell"` FCT pop in neutral GREY over the same cell.
///
/// Pin-discriminating:
/// - Dropping the flash spawn leaves `fx_count == 0` and fails the flash assert.
/// - Hardcoding a tile index (not reading `roles.fall_impact`) causes the index assert to
///   mismatch if the authored RON is changed.
/// - Omitting the FCT pop leaves `pop_count_for(.., "Fell") == 0` and fails the log assert.
/// - A `FallOccurred` for an entity with no `Position` spawns nothing, no panic (fail-closed).
#[test]
fn fall_occurred_spawns_one_flash_at_landing_cell_and_a_fell_fct_pop() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);
    app.world_mut().insert_resource(BattleInProgress);

    // Spawn a ganger at the LANDING cell (to_level — the sim overwrites Position before
    // FallOccurred is emitted, so the reader sees the landing location in Position).
    let landing_cell = Cell::new(4, 7);
    let from_level = Level::new(2);
    let to_level = Level::new(0);
    let storeys = StoreysFallen::new(2);
    let ganger = app
        .world_mut()
        .spawn(Position::new(CellLevel::new(landing_cell, to_level)))
        .id();

    // Write the FallOccurred message and run one update — the registered `read_fall_occurred`
    // drains it and spawns the flash + FCT pop on this frame's SpawnScene schedule.
    app.world_mut()
        .resource_mut::<Messages<FallOccurred>>()
        .write(FallOccurred::new(ganger, from_level, to_level, storeys));
    app.update();

    // C1: EXACTLY ONE FxFlash at the landing cell's world position.
    let roles = effect_roles(&app);
    assert!(
        roles.is_some(),
        "EffectRoles must be resident after settle_resources",
    );
    let Some(roles) = roles else { return };

    assert_eq!(
        fx_count(&mut app),
        1,
        "exactly one fall-impact flash must spawn for one FallOccurred",
    );
    let flash = single_flash(&mut app);
    assert!(
        flash.is_some(),
        "exactly one FxFlash sprite must exist after FallOccurred",
    );
    let Some((translation, index)) = flash else {
        return;
    };
    assert_eq!(
        translation,
        cell_to_world(landing_cell, to_level),
        "the fall-impact flash must sit at cell_to_world(landing cell, to_level)",
    );
    assert_eq!(
        index,
        Some(*roles.fall_impact),
        "the fall-impact flash's atlas index must equal the table's fall_impact role index \
         (data-driven, never a literal)",
    );

    // C3: a `"Fell"` FCT pop in neutral GREY is produced (the "log line" the contract requires).
    let pops = fct_pops(&mut app);
    assert_eq!(
        pop_count_for(&pops, "Fell"),
        1,
        "exactly one \"Fell\" FCT pop must spawn for one FallOccurred, got {pops:?}",
    );
    assert!(
        has_fct_pop(&pops, "Fell", valence_color(FctValence::Neutral)),
        "the \"Fell\" pop must be drawn in neutral GREY (FctValence::Neutral), got {pops:?}",
    );
    // The pop is anchored at the landing cell (planar x of cell_to_world).
    let anchor = cell_to_world(landing_cell, to_level);
    let mut q = app.world_mut().query::<(&FloatingCombatText, &Transform)>();
    let any_at_cell = q
        .iter(app.world())
        .any(|(_, transform)| (transform.translation.x - anchor.x).abs() < 0.001);
    assert!(
        any_at_cell,
        "the \"Fell\" pop must anchor at the landing cell's x position ({})",
        anchor.x,
    );

    // Fail-closed: a FallOccurred for an entity with no Position spawns no flash + no pop, no panic.
    advance_past_ttl(&mut app);
    assert_eq!(
        fx_count(&mut app),
        0,
        "the fall-impact flash must have expired"
    );
    let bare = app.world_mut().spawn_empty().id();
    app.world_mut()
        .resource_mut::<Messages<FallOccurred>>()
        .write(FallOccurred::new(
            bare,
            from_level,
            to_level,
            StoreysFallen::new(1),
        ));
    app.update();
    assert_eq!(
        fx_count(&mut app),
        0,
        "a FallOccurred for a Position-less entity must spawn no flash (fail-closed)",
    );
}
