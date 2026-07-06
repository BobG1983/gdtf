//! Storey z-anchoring: the default floor fill is anchored to the ground storey (GTW-535).

use bevy::prelude::*;
use gdtf_battle_presenter::ViewMode;
use gdtf_battle_sim::level::{GridHeight, GridLevels, GridSize, GridWidth};
use gdtf_content_editor::{CurrentEditLevel, EditorMap, LevelStep, MapEditorSession};

use super::harness::*;

/// The per-storey z-lift the base fill applies: storey 0 draws at z=0, storey `n` at `n *
/// STOREY_Z_GAP`. Mirrors `tiles.rs`'s `STOREY_Z_GAP` const so the test can bucket a preview
/// sprite back to its storey by its `Transform` z (the harness spawns no other world sprites).
const STOREY_Z_GAP: f32 = 0.01;

/// Half a storey z-gap — the tolerance for classifying a sprite's z into a storey band (a sprite is
/// "on storey `n`" when its z is within this of `n * STOREY_Z_GAP`).
const Z_TOLERANCE: f32 = STOREY_Z_GAP / 2.0;

/// Count the preview tile [`Sprite`]s whose `Transform` z places them on storey `storey` (z ≈
/// `storey * STOREY_Z_GAP`). Lets the test distinguish the GROUND default-floor fill (z≈0) from an
/// UPPER storey's tiles (z≈`storey * gap`) on the real spawned entities.
fn sprite_count_on_storey(app: &mut App, storey: u8) -> usize {
    let target_z = STOREY_Z_GAP * f32::from(storey);
    app.world_mut()
        .query::<(&Sprite, &Transform)>()
        .iter(app.world())
        .filter(|(_, transform)| (transform.translation.z - target_z).abs() < Z_TOLERANCE)
        .count()
}

/// GTW-535 (REAL PATH) — the theme default-floor fill is anchored to the GROUND storey (storey 0),
/// NOT to the active edit level. At an UPPER edit storey (2) with NO painted cells there, the
/// change-driven `redraw_preview_tiles` must:
///
/// - (a) still spawn the default-floor fill on the GROUND storey (storey 0 — z≈0), and
/// - (b) spawn NO fill on the unpainted active upper storey (storey 2 — z≈2·gap).
///
/// This proves the fill follows the ground plane, not the edit cursor. Against the OLD
/// `storey == *level` code the fill would follow the cursor: at edit level 2 it would appear on
/// storey 2 (assertion (b) FAILS) and vanish from storey 0 (assertion (a) FAILS). The default
/// `DownToActive` view draws `0..=2`, so both storeys are in the drawn band and the two z-buckets
/// are directly comparable.
#[test]
fn default_floor_fill_is_anchored_to_the_ground_storey() {
    let mut app = editor_app();
    advance_to_editing(&mut app);
    // Let the theme seed so the default floor / palette resolves and the base fill can draw.
    for _ in 0..8 {
        app.update();
    }

    // The editor opens in PREFAB mode with the default DownToActive view (so the drawn band is
    // `0..=active` — storeys 0..=2 are all in-band at edit level 2).
    assert_eq!(
        app.world().get_resource::<ViewMode>().copied(),
        Some(ViewMode::DownToActive),
        "the prefab viewport opens in the default DownToActive view",
    );

    // Soft-skip if the async seed has not resolved a default floor yet (asset-timing dependent) —
    // there is nothing to fill without one.
    let has_default_floor = app
        .world()
        .get_resource::<MapEditorSession>()
        .and_then(MapEditorSession::default_floor)
        .is_some();
    if !has_default_floor {
        assert!(
            app.world().get_resource::<EditorMap>().is_some(),
            "map inserted in Editing",
        );
        return;
    }

    // A 6×6×3 volume (storeys 0,1,2) with the edit cursor lifted to the UPPER storey 2 and NO cells
    // painted anywhere — so any fill that appears is the default-floor fallback, not painted tiles.
    let width;
    let height;
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
        width = usize::from(*size.width());
        height = usize::from(*size.height());

        // Lift the edit cursor to the UPPER storey 2 (stepped from ground, clamped into range).
        let Some(mut edit_level) = world.get_resource_mut::<CurrentEditLevel>() else {
            unreachable!("edit level inserted in Editing");
        };
        let lifted = CurrentEditLevel::ground()
            .stepped(LevelStep::up(), size)
            .stepped(LevelStep::up(), size);
        assert_eq!(
            *lifted.level(),
            2,
            "the edit cursor is lifted to the upper storey 2",
        );
        *edit_level = lifted;
    }

    // Settle the change-driven redraw on the new size + edit level.
    for _ in 0..4 {
        app.update();
    }

    let ground_fill = sprite_count_on_storey(&mut app, 0);
    let upper_fill = sprite_count_on_storey(&mut app, 2);
    let expected_cells = width * height;

    // (a) The GROUND storey (z≈0) carries the full default-floor fill — one sprite per cell —
    // regardless of the edit cursor being on storey 2 (GTW-535 the fix). Against the OLD
    // `storey == *level` code the ground fill would be ABSENT here (fill followed the cursor to
    // storey 2), so this asserts the anchor moved to the ground plane.
    assert_eq!(
        ground_fill, expected_cells,
        "the default-floor fill must be spawned on the GROUND storey (storey 0, z≈0) — one sprite \
         per cell — even when the edit cursor is on an upper storey (GTW-535); expected \
         {expected_cells}, got {ground_fill}",
    );

    // (b) The unpainted active UPPER storey (storey 2, z≈2·gap) gets NO fill — an unpainted upper
    // cell is empty air, not a floor. Against the OLD `storey == *level` code the fill WOULD appear
    // here (it followed the edit cursor up the stack — the reported bug), so a non-zero count is the
    // exact old-behaviour regression this pins.
    assert_eq!(
        upper_fill, 0,
        "the unpainted ACTIVE upper storey (storey 2, z≈2·gap) must draw NO default-floor fill — \
         the fill does not follow the edit cursor up the stack (GTW-535); got {upper_fill}",
    );
}
