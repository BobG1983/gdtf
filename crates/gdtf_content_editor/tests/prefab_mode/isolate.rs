//! The editor DEFAULT is the ISOLATE view (GTW-594 C2 — the structural GTW-592 fix):
//! at an upper edit storey the viewport draws ONLY the active storey plus ONE onion
//! storey below — never the whole `0..=active` stack.

use bevy::prelude::*;
use gdtf_battle_sim::level::{GridHeight, GridLevels, GridSize, GridWidth};
use gdtf_content_editor::{CurrentEditLevel, EditorMap, LevelStep, MapEditorSession};

use super::harness::*;

/// The per-storey z-lift the preview applies (mirrors `tiles/sprites.rs`'s `STOREY_Z_GAP` so a
/// sprite's `Transform` z buckets back to its storey — the `storeys.rs` recipe).
const STOREY_Z_GAP: f32 = 0.01;

/// Half a storey z-gap — the tolerance for classifying a sprite's z into a storey band.
const Z_TOLERANCE: f32 = STOREY_Z_GAP / 2.0;

/// Count the preview [`Sprite`]s whose `Transform` z places them on storey `storey` (the harness
/// spawns no other world sprites, so this IS the drawn content of that storey — base tiles AND
/// any per-cell overlay sprites parked inside the storey's z band).
fn sprite_count_on_storey(app: &mut App, storey: u8) -> usize {
    let target_z = STOREY_Z_GAP * f32::from(storey);
    app.world_mut()
        .query::<(&Sprite, &Transform)>()
        .iter(app.world())
        .filter(|(_, transform)| (transform.translation.z - target_z).abs() < Z_TOLERANCE)
        .count()
}

/// GTW-594 C2 (REAL PATH — the GTW-592 repro): the editor's DEFAULT view ISOLATES the edit
/// storey. On a 6×6×3 volume with the edit cursor on storey 2 and NOTHING painted anywhere:
///
/// - (a) storey 0 (two below the active) draws NOTHING — no default-floor fill, no tiles. This
///   is the exact GTW-592 symptom fixed: under the old always-DownToActive default the whole
///   `0..=active` stack drew, so the ground fill visually swamped the storey being authored.
/// - (b) the ACTIVE storey (2) draws content (its unpainted cells read as a faint void grid).
/// - (c) the ONE onion storey below (1) is allowed context — asserted by painting a cell there
///   and seeing it drawn (the categorical below-ghost).
///
/// RED against the pre-GTW-594 tree (assertion (a) fails: 36 ground-fill sprites), GREEN after.
#[test]
fn editor_default_isolates_the_active_storey_with_one_onion_below() {
    let mut app = editor_app();
    advance_to_editing(&mut app);
    // Let the theme seed so the default floor / palette resolves (the ground fill exists to
    // NOT be drawn — without it assertion (a) would pass vacuously).
    for _ in 0..8 {
        app.update();
    }
    let tile = {
        let world = app.world();
        let Some(session) = world.get_resource::<MapEditorSession>() else {
            unreachable!("session inserted in Editing");
        };
        let Some(tile) = session.default_floor() else {
            // Soft-skip on async seed timing (the storeys.rs recipe): no default floor resolved
            // means no fill exists to assert against.
            assert!(
                world.get_resource::<EditorMap>().is_some(),
                "map inserted in Editing"
            );
            return;
        };
        tile
    };

    // A 6×6×3 volume (storeys 0, 1, 2), edit cursor on the TOP storey 2; paint ONE cell on
    // storey 1 (the onion-below witness) and NOTHING else.
    {
        let world = app.world_mut();
        let Some(mut session) = world.get_resource_mut::<MapEditorSession>() else {
            unreachable!("session inserted in Editing");
        };
        let Ok(size) = GridSize::new(GridWidth::new(6), GridHeight::new(6), GridLevels::new(3))
        else {
            unreachable!("a 6×6×3 grid is valid");
        };
        session.set_grid_size(size);
        let Some(mut map) = world.get_resource_mut::<EditorMap>() else {
            unreachable!("map inserted in Editing");
        };
        map.paint_at(
            gdtf_battle_sim::metric::CellLevel::new(
                gdtf_battle_sim::prelude::Cell::new(2, 2),
                gdtf_battle_sim::metric::Level::new(1),
            ),
            tile,
            size,
        );
        let Some(mut edit_level) = world.get_resource_mut::<CurrentEditLevel>() else {
            unreachable!("edit level inserted in Editing");
        };
        *edit_level = CurrentEditLevel::ground()
            .stepped(LevelStep::up(), size)
            .stepped(LevelStep::up(), size);
    }
    for _ in 0..4 {
        app.update();
    }

    let ground = sprite_count_on_storey(&mut app, 0);
    let onion = sprite_count_on_storey(&mut app, 1);
    let active = sprite_count_on_storey(&mut app, 2);

    // (a) The GTW-592 repro fixed: storey 0 is OUTSIDE the isolate band (active − 2) — it must
    // draw NOTHING, ground fill included.
    assert_eq!(
        ground, 0,
        "the editor's DEFAULT (Isolate) view at edit storey 2 must draw NOTHING on storey 0 — \
         the whole-stack DownToActive draw is the GTW-592 symptom (GTW-594 C2); got {ground} \
         sprite(s) on storey 0",
    );
    // (b) The ACTIVE storey draws (its unpainted cells read as the faint void grid).
    assert!(
        active > 0,
        "the ACTIVE edit storey (2) must draw content in the Isolate view (the void grid on \
         unpainted cells); got {active}",
    );
    // (c) The ONE onion storey below draws its authored cell (the categorical below-ghost).
    assert!(
        onion > 0,
        "the ONE onion storey below the active (1) must draw its painted cell as the \
         categorical below-ghost (GTW-594 C2); got {onion}",
    );
}
