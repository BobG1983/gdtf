//! GTW-251: headless draw-LOGIC tests for the message-driven hover-highlight — the
//! PRESENTER half of the seam.
//!
//! The presenter DEFINES the `HighlightRequest` message and DRAWS from it
//! (`draw_highlight_on_request`). This test proves AC2 end of the contract: sending a
//! `HighlightRequest(Some(cell))` moves/shows the one `HoverHighlight` sprite at
//! `cell_to_world(cell)`; sending `HighlightRequest(None)` hides it; and the draw is
//! wired to the MESSAGE (a no-message update leaves the highlight following the last
//! request, never duplicating).
//!
//! The highlight sprite is a solid-tint reticle (no atlas), so this needs no
//! `AssetServer` — a `MinimalPlugins` app + `TopDownRendererPlugin` (which registers
//! the `HighlightRequest` buffer + the battle-gated `draw_highlight_on_request`) +
//! the `BattleInProgress` gate is sufficient. Messages are written and the highlight
//! is inspected DIRECTLY via `app.world_mut()` in the test body — the accepted
//! headless idiom (`bevy-traps.md` #7 carve-out (a)). No function here takes
//! `&mut World`/`&World`.

use bevy::{camera::visibility::RenderLayers, math::Vec3, prelude::*};
use gdtf_battle_presenter::{
    CELL_PX, HighlightRequest, HoverHighlight, TopDownRendererPlugin, WORLD_RENDER_LAYER,
    cell_to_world,
};
use gdtf_battle_sim::{BattleInProgress, Cell, CellLevel, Level};

/// Builds a focused headless presenter app: `MinimalPlugins` + the
/// `TopDownRendererPlugin` (which registers the `HighlightRequest` buffer + the
/// battle-gated `draw_highlight_on_request`) + the `BattleInProgress` gate. No
/// `AssetServer` — the highlight is a solid-tint sprite, not an atlas tile.
fn highlight_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(TopDownRendererPlugin);
    app.world_mut().insert_resource(BattleInProgress);
    app
}

/// Writes one `HighlightRequest` into the buffer the presenter drains.
fn send_request(app: &mut App, request: HighlightRequest) {
    app.world_mut()
        .resource_mut::<bevy::ecs::message::Messages<HighlightRequest>>()
        .write(request);
}

/// Counts the hover-highlight sprites in the world.
fn highlight_count(app: &mut App) -> usize {
    let mut q = app
        .world_mut()
        .query_filtered::<Entity, With<HoverHighlight>>();
    q.iter(app.world()).count()
}

/// The single hover-highlight sprite's translation + visibility, if exactly one exists.
fn highlight_state(app: &mut App) -> Option<(Vec3, Visibility)> {
    let mut q = app
        .world_mut()
        .query_filtered::<(&Transform, &Visibility), With<HoverHighlight>>();
    let mut iter = q.iter(app.world());
    iter.next().map(|(t, v)| (t.translation, *v))
}

/// Whether the one highlight sprite is `CELL_PX`-sized and on the world render layer.
fn highlight_on_world_layer_at_cell_size(app: &mut App) -> bool {
    let mut q = app
        .world_mut()
        .query_filtered::<(&Sprite, &RenderLayers), With<HoverHighlight>>();
    let world_layer = RenderLayers::layer(WORLD_RENDER_LAYER);
    q.iter(app.world()).all(|(sprite, layers)| {
        sprite.custom_size == Some(Vec2::splat(CELL_PX)) && layers.intersects(&world_layer)
    })
}

/// AC2 — the presenter DRAWS the highlight from the request: a
/// `HighlightRequest(Some(cell))` spawns/moves the ONE `HoverHighlight` sprite to
/// `cell_to_world(cell)` and shows it (sized to one cell, on the world layer); a
/// further `Some(other)` MOVES it (no duplicate); a `None` HIDES it. The draw must be
/// wired to the message — a `None`-then-`Some` round-trip re-shows it at the new cell.
#[test]
fn presenter_draws_highlight_from_the_request() {
    let level = Level::new(0);
    let mut app = highlight_app();

    // No request yet, one settle update: nothing spawned (lazy spawn on first Some).
    app.update();
    assert_eq!(
        highlight_count(&mut app),
        0,
        "no highlight sprite before any Some request (lazy spawn)",
    );

    // First Some(cell) — the highlight spawns at cell_to_world(cell), visible.
    let cell_a = CellLevel::new(Cell::new(4, 7), level);
    send_request(&mut app, HighlightRequest(Some(cell_a)));
    app.update();
    assert_eq!(
        highlight_count(&mut app),
        1,
        "exactly one highlight sprite after the first Some request",
    );
    assert_eq!(
        highlight_state(&mut app),
        Some((
            cell_to_world(Cell::new(cell_a.x, cell_a.y), level),
            Visibility::Visible,
        )),
        "the highlight must be visible at cell_to_world(requested cell A)",
    );
    assert!(
        highlight_on_world_layer_at_cell_size(&mut app),
        "the highlight must be CELL_PX-sized on the WORLD_RENDER_LAYER",
    );

    // A different Some(cell) — the highlight MOVES, no duplicate.
    let cell_b = CellLevel::new(Cell::new(11, 2), level);
    send_request(&mut app, HighlightRequest(Some(cell_b)));
    app.update();
    assert_eq!(
        highlight_count(&mut app),
        1,
        "still exactly one highlight sprite after a second Some request (no duplicate)",
    );
    assert_eq!(
        highlight_state(&mut app),
        Some((
            cell_to_world(Cell::new(cell_b.x, cell_b.y), level),
            Visibility::Visible,
        )),
        "the highlight must have MOVED to cell_to_world(requested cell B)",
    );

    // None — the highlight hides (entity persists, not despawned, not duplicated).
    send_request(&mut app, HighlightRequest(None));
    app.update();
    assert_eq!(
        highlight_count(&mut app),
        1,
        "the highlight entity persists (hidden, not duplicated) on a None request",
    );
    assert_eq!(
        highlight_state(&mut app).map(|(_, v)| v),
        Some(Visibility::Hidden),
        "the highlight must be hidden on a None request",
    );

    // Some again — it re-shows at the new cell, proving the draw tracks the MESSAGE.
    send_request(&mut app, HighlightRequest(Some(cell_a)));
    app.update();
    assert_eq!(
        highlight_state(&mut app),
        Some((
            cell_to_world(Cell::new(cell_a.x, cell_a.y), level),
            Visibility::Visible,
        )),
        "a Some request after a None must re-show the highlight at the new cell",
    );
}

/// AC2 (pin-discriminate) — a non-`BattleInProgress` app NEVER draws the highlight even
/// when a request is sent: the draw is battle-gated, so removing the gate witness leaves
/// the highlight absent. This is the negative half — the draw is genuinely wired through
/// the battle-gated message reader, not spawned unconditionally.
#[test]
fn no_highlight_drawn_without_battle_in_progress() {
    let level = Level::new(0);
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(TopDownRendererPlugin);
    // Deliberately NO BattleInProgress.

    let cell = CellLevel::new(Cell::new(4, 7), level);
    send_request(&mut app, HighlightRequest(Some(cell)));
    app.update();

    assert_eq!(
        highlight_count(&mut app),
        0,
        "no highlight may be drawn when BattleInProgress is absent (the draw is battle-gated)",
    );
}
