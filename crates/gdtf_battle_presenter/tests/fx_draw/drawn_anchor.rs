//! GTW-889 — WHERE a consequence pop lands: the `PopAnchor::GangerPosition` families anchor
//! on the cell the playback cursor has SHOWN the ganger at, not the cell the sim has already
//! run ahead to.
//!
//! Pacing the pops onto the cursor's clock moved WHEN they appear. This is the other half:
//! by the time the cursor plays an injury or a bleed tick, the sim may have walked that
//! ganger several cells on, while the sprite still stands where the cursor last drew it
//! (`move_ganger_sprites` retargets on `Changed<DrawnPosition>`). A pop resolved from the
//! live `Position` would render over empty floor.

use bevy::{app::App, prelude::Text2d, transform::components::Transform};
use gdtf_battle_presenter::{
    DrawnPosition, FctValence, FloatingCombatText, cell_to_world, valence_color,
};
use gdtf_battle_sim::{
    effects::bleed::Bleeding,
    prelude::{BattleInProgress, Cell, CellLevel, Direction, Level, Position, StanceKind},
    test_support::GangerEntityBuilder,
};

use super::{harness::*, probes::*};

/// Spawns a ganger carrying the FULL component set `seed_drawn_state` reads, so the playback
/// cursor gives it the `Drawn*` mirrors a real battle ganger gets.
///
/// `probes::wounded_ganger` sets only `Position` + `Wounds`, which the seed query (it also
/// requires `Facing` / `Stance` / `Aiming` / `LifeState` / `Tu` / `Hp`) never matches — a
/// ganger spawned that way would silently never be mirrored, and this test's premise would
/// evaporate. Single-consumer, so it stays here rather than in `probes`.
fn mirrored_ganger(app: &mut App, at: CellLevel) -> bevy::ecs::entity::Entity {
    GangerEntityBuilder::new()
        .at(at)
        .facing(Direction::North)
        .stance(StanceKind::Standing)
        .aiming(false)
        .tu(60)
        .combat_vitals(10, 2)
        .spawn(app.world_mut())
}

/// The planar world x of every live FCT pop whose rendered string equals `text`.
fn pop_xs(app: &mut App, text: &str) -> Vec<f32> {
    let mut q = app
        .world_mut()
        .query::<(&FloatingCombatText, &Text2d, &Transform)>();
    q.iter(app.world())
        .filter(|(_, label, _)| ***label == *text)
        .map(|(_, _, transform)| transform.translation.x)
        .collect()
}

/// The drawn cell of `ganger`'s mirror, or `None` if the cursor has not seeded one.
fn drawn_cell(app: &App, ganger: bevy::ecs::entity::Entity) -> Option<CellLevel> {
    app.world()
        .get::<DrawnPosition>(ganger)
        .map(|drawn| *drawn.position())
}

/// A ganger the cursor has drawn at one cell, while the sim has already moved it to another,
/// pops its `"Bleeding"` tag over the DRAWN cell.
///
/// The setup is the reported situation in miniature: one update seeds the ganger's
/// `DrawnPosition` mirror at the shown cell, then the live `Position` is overwritten with a
/// far cell (the sim outrunning the cursor — this ticket's whole premise), and only then is
/// the bleed fact played. The pop must land where the sprite is.
///
/// Pin-discriminating: resolving the anchor from the live `Position` puts the pop at the far
/// cell's x, which this test asserts against explicitly.
#[test]
fn a_consequence_pop_anchors_at_the_drawn_cell_not_the_cell_the_sim_ran_ahead_to() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);
    app.world_mut().insert_resource(BattleInProgress);

    let shown = CellLevel::new(Cell::new(3, 6), Level::new(0));
    let ahead = CellLevel::new(Cell::new(17, 6), Level::new(0));
    let ganger = mirrored_ganger(&mut app, shown);

    // One update lets `seed_drawn_state` give the ganger its mirror at the shown cell.
    app.update();
    assert_eq!(
        drawn_cell(&app, ganger),
        Some(shown),
        "the mirror must be seeded at the shown cell for this test to mean anything",
    );

    // The sim walks on while the cursor has shown none of those steps: only the LIVE
    // position moves.
    app.world_mut()
        .entity_mut(ganger)
        .insert(Position::new(ahead));
    assert_eq!(
        drawn_cell(&app, ganger),
        Some(shown),
        "the drawn mirror must NOT follow the live move — that lag is the situation under test",
    );

    play(&mut app, Bleeding::new(ganger));
    app.update();

    let pops = fct_pops(&mut app);
    assert!(
        has_fct_pop(&pops, "Bleeding", valence_color(FctValence::Status)),
        "the played bleed must still pop its tag, got {pops:?}",
    );

    let (shown_cell, shown_level) = shown.split();
    let (ahead_cell, ahead_level) = ahead.split();
    let drawn_x = cell_to_world(shown_cell, shown_level).x;
    let ahead_x = cell_to_world(ahead_cell, ahead_level).x;
    assert!(
        (drawn_x - ahead_x).abs() > 1.0,
        "the two cells must be far apart for the assertions below to discriminate",
    );
    let xs = pop_xs(&mut app, "Bleeding");
    assert!(
        xs.iter().any(|x| (x - drawn_x).abs() < 0.001),
        "the pop must sit on the DRAWN cell x ({drawn_x}), got {xs:?}",
    );
    assert!(
        !xs.iter().any(|x| (x - ahead_x).abs() < 0.001),
        "no pop may sit on the cell the sim ran ahead to ({ahead_x}), got {xs:?}",
    );
}
