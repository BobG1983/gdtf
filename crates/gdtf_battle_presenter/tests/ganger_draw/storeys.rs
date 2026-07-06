//! Storey band visibility on `ActiveLevel` change + the ganger z above its own floor
//! (AC5, GTW-283, GTW-520).

use bevy::{app::App, prelude::Visibility, transform::components::Transform};
use gdtf_battle_presenter::{ActiveLevel, TerrainSprite};
use gdtf_battle_sim::{
    prelude::{Cell, CellLevel, Direction, Level, Position},
    test_support::SituationBuilder,
};

use super::{harness::*, probes::*};

/// The world-space `z` of the terrain (floor / wall / cover) sprite drawn at `at`, if
/// the static-battlefield draw spawned one there. Every in-range cell is at least a
/// floor tile, so a co-located terrain sprite exists at a spawned ganger's cell.
fn terrain_z_at(app: &mut App, at: CellLevel) -> Option<f32> {
    let mut q = app.world_mut().query::<(&TerrainSprite, &Transform)>();
    q.iter(app.world())
        .find(|(marker, _)| marker.at == at)
        .map(|(_, transform)| transform.translation.z)
}

/// Drive bounded `update()`s until a co-located terrain sprite exists at `at`, returning
/// its `z` (or `None` if none appeared within `MAX_UPDATES`).
///
/// `draw_static_battlefield` (`terrain/draw.rs`) fires only when a `BattleReady` is drained
/// THIS update (or `ActiveLevel.is_changed()`) and `commands.spawn`s the `TerrainSprite`,
/// so the sprite is queryable only AFTER the end-of-update command flush. `drive_setup`'s
/// extra update is tuned for the ganger draw's `Added<Position>`, NOT the terrain draw's
/// `BattleReady`-read + spawn flush, which can land a frame later under parallel test
/// contention. Settling on the terrain z directly makes the AC2 assertion deterministic.
fn settle_terrain_z_at(app: &mut App, at: CellLevel) -> Option<f32> {
    for _ in 0..MAX_UPDATES {
        if let Some(z) = terrain_z_at(app, at) {
            return Some(z);
        }
        app.update();
    }
    terrain_z_at(app, at)
}

/// AC5 (GTW-520-widened, GTW-627-churned) — the ganger-visibility RESOLVER in BAND-ONLY
/// mode shows every ganger WITHIN the drawn band `0..=active` and hides those strictly
/// ABOVE it, on an `ActiveLevel` change.
///
/// This pins the resolver's band-only mode DIRECTLY (GTW-627 C7): [`band_only_fog`]
/// removes the setup-inserted `SquadVisibility`, so the one classifier takes its
/// ABSENT-fog branch and only drawn-band membership decides — the mixed-faction pair
/// below is the discriminator (an empty COMPOSED fog would hide the faction-1 ganger; in
/// band-only mode both factions show when in-band). Pre-GTW-520 this asserted the hard
/// cut (the level-0 ganger HIDDEN at active 1); GTW-520 widens it — the level-0 ganger is
/// now SHOWN at active 1 (a lower drawn storey), while the level-1 ganger is hidden at
/// active 0 (above the band). The fog-COMPOSED final visibility is asserted by the
/// `fog_visibility.rs` tests.
#[test]
fn active_level_change_shows_drawn_band_hides_above() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let l0_at = CellLevel::new(Cell::new(5, 6), Level::new(0));
    let l1_at = CellLevel::new(Cell::new(7, 8), Level::new(1));
    // Author both endpoints as slabs so the (implicit) cell existence is clean — not
    // strictly required for the draw, but keeps the situation valid.
    let situation = SituationBuilder::new()
        .with_ganger(ganger_at(l0_at, 0, Direction::East))
        .with_ganger(ganger_at(l1_at, 1, Direction::West))
        .slab_at(l0_at)
        .slab_at(l1_at)
        .build();
    assert!(
        drive_setup(&mut app, situation),
        "setup_battle must complete"
    );
    // GTW-627: clear the setup-inserted fog sets so the resolver runs BAND-ONLY (the
    // classifier's absent-fog branch) — this test isolates the storey axis.
    band_only_fog(&mut app);
    app.update();

    let l0_sim = sim_entity_at(&mut app, l0_at);
    let l1_sim = sim_entity_at(&mut app, l1_at);
    assert!(
        l0_sim.is_some() && l1_sim.is_some(),
        "both gangers must have spawned",
    );

    // At active level 0: the level-0 ganger is in-band (shown); the level-1 ganger is strictly
    // ABOVE the band (hidden).
    assert_eq!(
        visibility_of_sim(&mut app, l0_sim),
        Some(Visibility::Inherited),
        "the level-0 ganger sprite is visible at active level 0 (in the band 0..=0)",
    );
    assert_eq!(
        visibility_of_sim(&mut app, l1_sim),
        Some(Visibility::Hidden),
        "the level-1 ganger sprite is hidden at active level 0 (strictly above the band)",
    );

    // Change the active level to 1 — the band widens to 0..=1.
    *app.world_mut().resource_mut::<ActiveLevel>() = ActiveLevel::new(Level::new(1));
    app.update();

    // Now BOTH gangers are within the drawn band 0..=1, so both are shown (GTW-520): the level-0
    // ganger is a LOWER drawn storey (no longer hidden), the level-1 ganger is the active storey.
    assert_eq!(
        visibility_of_sim(&mut app, l1_sim),
        Some(Visibility::Inherited),
        "after the change, the level-1 (active) ganger sprite is visible",
    );
    assert_eq!(
        visibility_of_sim(&mut app, l0_sim),
        Some(Visibility::Inherited),
        "after the change, the level-0 ganger sprite is now SHOWN — it is a LOWER drawn storey \
         within the band 0..=1 (GTW-520 drawn-band widening, not the old hard cut)",
    );
}

/// GTW-283 AC2 — a ganger sprite draws ON TOP of (in front of) its OWN floor tile, both
/// at spawn and after a move. Drives the REAL setup spawn + the REAL terrain draw, then
/// asserts (a) the ganger's `z` is strictly greater than the co-located terrain (floor)
/// `z`, (b) `0.0 < ganger.z < 1.0` at level 0 (the lift stays within its own storey band,
/// never crossing into the next storey), and (c) after a `Changed<Position>` move the
/// moved ganger STILL has `z` > the terrain `z` at its NEW cell. RED before the fix (both
/// projected to `z = 0.0`, so neither the strict-greater nor the `> 0.0` bound held),
/// GREEN after. The rasterized "the figure draws over the floor" itself is AC7 in-engine QA.
#[test]
fn ganger_draws_above_its_own_floor_at_spawn_and_after_move() {
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

    // Settle the terrain draw first: its BattleReady-read + spawn flush can land a frame
    // behind drive_setup's ganger-tuned update under parallel test contention, so drive
    // bounded update()s until the co-located floor sprite exists (deterministic, no flake).
    let floor_z = settle_terrain_z_at(&mut app, start);
    assert!(
        floor_z.is_some(),
        "a co-located floor tile must be drawn under the ganger's cell",
    );
    let Some(floor_z) = floor_z else { return };

    // The single ganger sprite's z (read after the terrain settle; the ganger has not
    // moved, so its spawn z is unchanged by the extra terrain-settle updates).
    let drawn = drawn_gangers(&mut app);
    assert_eq!(drawn.len(), 1, "one ganger sprite at spawn");
    let ganger_z = drawn[0].translation.z;

    // (a) the ganger draws strictly in FRONT of its own floor tile.
    assert!(
        ganger_z > floor_z,
        "the ganger sprite z ({ganger_z}) must be strictly greater than its own floor z \
         ({floor_z}) — it draws on top of, not behind, its floor",
    );
    // (b) the lift stays within the storey-0 band: 0.0 < z < 1.0.
    assert!(
        ganger_z > 0.0 && ganger_z < 1.0,
        "the level-0 ganger z ({ganger_z}) must satisfy 0.0 < z < 1.0 (within its own storey)",
    );

    // (c) after a move the moved ganger STILL draws above the floor at its NEW cell.
    let sim = sim_entity_at(&mut app, start);
    assert!(sim.is_some(), "the spawned ganger sim entity must exist");
    let Some(sim) = sim else { return };
    let dest = CellLevel::new(Cell::new(7, 8), Level::new(0));
    let mut pos_q = app.world_mut().query::<&mut Position>();
    if let Ok(mut pos) = pos_q.get_mut(app.world_mut(), sim) {
        *pos = Position::new(dest);
    }
    app.update();

    let drawn_after = drawn_gangers(&mut app);
    assert_eq!(
        drawn_after.len(),
        1,
        "still one ganger sprite after the move"
    );
    let moved_z = drawn_after[0].translation.z;
    let dest_floor_z = settle_terrain_z_at(&mut app, dest);
    assert!(
        dest_floor_z.is_some(),
        "a co-located floor tile must be drawn under the moved ganger's new cell",
    );
    let Some(dest_floor_z) = dest_floor_z else {
        return;
    };
    assert!(
        moved_z > dest_floor_z,
        "after the move the ganger z ({moved_z}) must STILL be strictly greater than the floor \
         z at its new cell ({dest_floor_z}) — the bias holds across moves",
    );
}
