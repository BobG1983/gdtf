//! Spawn one faction-tinted sprite per ganger + the same-sprite movement mirror
//! (AC1/AC3).

use bevy::{app::App, math::Vec2, prelude::Entity, time::TimeUpdateStrategy};
use gdtf_battle_presenter::{CELL_PX, GangerSprites, Layer, cell_to_world_layered};
use gdtf_battle_sim::{
    prelude::{Cell, CellLevel, Direction, Faction, Level},
    test_support::SituationBuilder,
};

use super::{harness::*, probes::*};

/// Drive bounded `update()`s until the presenter sprite `entity` GLIDES to within a hair
/// of `target` (GTW-359 C4: the move is now a tween, so it settles over frames, not in one
/// update).
///
/// Installs [`TimeUpdateStrategy::ManualDuration`] so each `app.update()` advances
/// `Res<Time>` by a FIXED, generous delta (longer than the tween's own glide duration),
/// making the settle DETERMINISTIC rather than dependent on real wall-clock deltas under
/// parallel test load — a couple of updates land the glide exactly on `target`. The loop
/// is bounded so a never-arriving glide surfaces as a failed assertion, not a hang.
fn settle_sprite_at(app: &mut App, entity: Entity, target: bevy::math::Vec3) {
    app.insert_resource(TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_millis(500),
    ));
    for _ in 0..MAX_UPDATES {
        app.update();
        if sprite_translation(app, entity).is_some_and(|t| t.distance(target) < 1.0e-3) {
            return;
        }
    }
}

/// AC1 — two `Added<Position>` gangers (distinct factions, distinct facings) on the
/// active level spawn exactly two faction-coloured sprites at `cell_to_world`, each with
/// the facing-correct atlas index read structurally; the two factions resolve to two
/// distinct base indices; `custom_size == Some(Vec2::splat(CELL_PX))`.
#[test]
fn added_gangers_spawn_one_faction_coloured_sprite_each() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let g0_at = CellLevel::new(Cell::new(5, 6), Level::new(0));
    let g1_at = CellLevel::new(Cell::new(12, 9), Level::new(0));
    let situation = SituationBuilder::new()
        .with_ganger(ganger_at(g0_at, 0, Direction::East))
        .with_ganger(ganger_at(g1_at, 1, Direction::North))
        .build();
    assert!(
        drive_setup(&mut app, situation),
        "setup_battle must complete"
    );

    let roles = character_roles(&app);
    assert!(roles.is_some(), "CharacterRoles must be resident");
    let Some(roles) = roles else { return };

    // The two factions resolve to two distinct base indices (visibly distinct actors).
    assert_ne!(
        roles.base_for(Faction::new(0)),
        roles.base_for(Faction::new(1)),
        "the two factions must resolve to two distinct actor base indices",
    );

    let drawn = drawn_gangers(&mut app);
    assert_eq!(
        drawn.len(),
        2,
        "exactly two ganger sprites spawn (one per authored ganger), got {}",
        drawn.len(),
    );

    // Map each drawn sprite back to its authored cell + faction + facing.
    let g0_sim = sim_entity_at(&mut app, g0_at);
    let g1_sim = sim_entity_at(&mut app, g1_at);
    assert!(
        g0_sim.is_some() && g1_sim.is_some(),
        "both authored gangers must have spawned sim entities",
    );

    for d in &drawn {
        // Sizing: every ganger sprite is one cell.
        assert_eq!(
            d.custom_size,
            Some(Vec2::splat(CELL_PX)),
            "every ganger sprite must be custom_size Some(Vec2::splat(CELL_PX))",
        );
        // Every drawn sprite mirrors one of the two authored gangers.
        let mirrors_authored = Some(d.sim_entity) == g0_sim || Some(d.sim_entity) == g1_sim;
        assert!(
            mirrors_authored,
            "every drawn sprite must mirror one of the two authored gangers",
        );
        // Index + position, structural per sim entity.
        if Some(d.sim_entity) == g0_sim {
            assert_eq!(
                d.atlas_index,
                Some(expected_index(&roles, 0, Direction::East)),
                "faction-0 ganger sprite index = faction_0 base + East(RIGHT) offset",
            );
            assert_eq!(
                d.translation,
                cell_to_world_layered(Cell::new(5, 6), Level::new(0), Layer::Actor),
                "faction-0 sprite at the Actor-layer projection of its authored Position",
            );
        } else if Some(d.sim_entity) == g1_sim {
            assert_eq!(
                d.atlas_index,
                Some(expected_index(&roles, 1, Direction::North)),
                "faction-1 ganger sprite index = faction_1 base + North(UP) offset",
            );
            assert_eq!(
                d.translation,
                cell_to_world_layered(Cell::new(12, 9), Level::new(0), Layer::Actor),
                "faction-1 sprite at the Actor-layer projection of its authored Position",
            );
        }
    }
}

/// AC3 — a `Changed<Position>` MOVES the existing presenter sprite (looked up through
/// `GangerSprites`) and does NOT spawn a second.
#[test]
fn changed_position_moves_the_same_sprite() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let start = CellLevel::new(Cell::new(5, 6), Level::new(0));
    let situation = SituationBuilder::new()
        .with_ganger(ganger_at(start, 0, Direction::East))
        .build();
    assert!(
        drive_setup(&mut app, situation),
        "setup_battle must complete"
    );

    let drawn_before = drawn_gangers(&mut app);
    assert_eq!(drawn_before.len(), 1, "one ganger sprite before the move");
    let sprite_before = drawn_before[0].sprite_entity;

    let sim = sim_entity_at(&mut app, start);
    assert!(sim.is_some(), "the ganger sim entity must exist");
    let Some(sim) = sim else { return };

    // The presenter sprite the map links to this sim entity, captured before the move.
    let mapped_before = app
        .world()
        .get_resource::<GangerSprites>()
        .and_then(|m| m.sprite_for(sim));
    assert_eq!(
        mapped_before,
        Some(sprite_before),
        "the map links the sim ganger to its one sprite",
    );

    // Move the ganger on the presenter's OWN clock: its DrawnPosition mirror — the cell the
    // playback cursor has shown it at, the GTW-727 C17 trigger `move_ganger_sprites` now reads
    // (the live Position remains authoritative for the sim, but the sprite follows the mirror).
    let dest = CellLevel::new(Cell::new(7, 8), Level::new(0));
    set_drawn_position(&mut app, sim, dest);
    // GTW-359 (C4): the move is now GLIDED (a re-targeting tween), not snapped — so the
    // sprite reaches the new cell over a few frames, not in one update. Settle the glide
    // (bounded), then assert it landed exactly on the new cell.
    let dest_world = cell_to_world_layered(Cell::new(7, 8), Level::new(0), Layer::Actor);
    settle_sprite_at(&mut app, sprite_before, dest_world);

    let drawn_after = drawn_gangers(&mut app);
    assert_eq!(
        drawn_after.len(),
        1,
        "still exactly one ganger sprite after the move (not respawned), got {}",
        drawn_after.len(),
    );
    // The SAME presenter sprite entity, now at the new cell.
    assert_eq!(
        drawn_after[0].sprite_entity, sprite_before,
        "the move reuses the SAME presenter sprite entity",
    );
    assert!(
        drawn_after[0].translation.distance(dest_world) < 1.0e-3,
        "the sprite glided to the Actor-layer projection of the new Position (got {:?}, \
         expected {dest_world:?})",
        drawn_after[0].translation,
    );
    // And the map still points at that same sprite.
    let mapped_after = app
        .world()
        .get_resource::<GangerSprites>()
        .and_then(|m| m.sprite_for(sim));
    assert_eq!(
        mapped_after,
        Some(sprite_before),
        "the map still links the sim ganger to its one (moved) sprite",
    );
}
