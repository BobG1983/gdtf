//! The selection-highlight sprite snaps to the selected cell and hides on clear.

use bevy::{camera::visibility::RenderLayers, prelude::*};
use gdtf_battle_input::{ActIntent, SelectionHighlight};
use gdtf_battle_presenter::{CELL_PX, WORLD_RENDER_LAYER, cell_to_world};
use gdtf_battle_sim::prelude::{Cell, CellLevel, Level};
use gdtf_test_utils::{clear_mouse, press_left};

use super::harness::*;

// ---------------------------------------------------------------------------------
// AC5 — the selection highlight snaps to the selected cell + hides on clear.
// ---------------------------------------------------------------------------------

/// Exactly ONE `SelectionHighlight` sprite is drawn at `cell_to_world(selected cell)`,
/// visible, on the world render layer at one-cell size; clearing the selection (via the
/// `SelectionClear` seam — a player selection no longer clears on an empty-cell click,
/// it MOVEs) hides it (still one entity).
#[test]
fn selection_highlight_snaps_to_cell_and_hides_on_clear() {
    let level = Level::new(0);
    let mut app = selection_app(level);

    let cell = Cell::new(8, 3);
    let ganger = place_player_ganger(&mut app, CellLevel::new(cell, level));
    set_hovered(&mut app, Some(CellLevel::new(cell, level)));
    press_left(&mut app);
    app.update();

    assert_eq!(
        selected(&app),
        Some(ganger),
        "the player-faction occupant must be selected before checking the highlight",
    );
    assert_eq!(
        highlight_count(&mut app),
        1,
        "exactly one selection-highlight sprite exists after a selection",
    );
    assert_eq!(
        highlight_state(&mut app),
        Some((cell_to_world(cell, level), Visibility::Visible)),
        "the selection highlight must be visible at cell_to_world(selected cell)",
    );
    assert!(
        highlight_on_world_layer_at_cell_size(&mut app),
        "the selection highlight must be CELL_PX-sized on the WORLD_RENDER_LAYER",
    );

    // Clear the selection through the seam and confirm the highlight hides. (A
    // player-faction selection + an empty-cell click MOVEs under GTW-238, so the clear
    // is driven by the SelectionClear intent, not an empty click.) The drain clears the
    // selection in `dispatch_act_intents`, which is unordered vs the highlight system, so
    // a SECOND update lets the highlight observe the cleared selection.
    clear_mouse(&mut app);
    push_intent(&mut app, ActIntent::SelectionClear);
    app.update();
    assert_eq!(
        selected(&app),
        None,
        "the SelectionClear intent cleared the selection"
    );
    app.update();
    assert_eq!(
        highlight_count(&mut app),
        1,
        "the highlight entity persists (hidden, not duplicated) on clear",
    );
    assert_eq!(
        highlight_state(&mut app).map(|(_, v)| v),
        Some(Visibility::Hidden),
        "the selection highlight must hide when nothing is selected",
    );
}

/// Whether the one selection-highlight sprite is `CELL_PX`-sized + on the world layer.
fn highlight_on_world_layer_at_cell_size(app: &mut App) -> bool {
    let mut q = app
        .world_mut()
        .query_filtered::<(&Sprite, &RenderLayers), With<SelectionHighlight>>();
    let world_layer = RenderLayers::layer(WORLD_RENDER_LAYER);
    q.iter(app.world()).all(|(sprite, layers)| {
        sprite.custom_size == Some(Vec2::splat(CELL_PX)) && layers.intersects(&world_layer)
    })
}
