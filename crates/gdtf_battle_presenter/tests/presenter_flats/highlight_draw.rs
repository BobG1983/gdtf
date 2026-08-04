//! Highlight draw: request spawn, move, hide, recolour by visibility verdict.
use bevy::{camera::visibility::RenderLayers, math::Vec3, prelude::*};
use gdtf_battle_presenter::{
    CELL_PX, CellVisibility, HighlightRequest, HoverHighlight, TopDownRendererPlugin,
    WORLD_RENDER_LAYER, cell_to_world,
};
use gdtf_battle_sim::prelude::{BattleInProgress, Cell, CellLevel, Level};

const fn visible_request(cell: Option<CellLevel>) -> HighlightRequest {
    HighlightRequest::new(cell, CellVisibility::SquadVisible)
}

fn highlight_app() -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        bevy::asset::AssetPlugin::default(),
        bevy::scene::ScenePlugin,
    ))
    .add_plugins(TopDownRendererPlugin);
    app.world_mut().insert_resource(BattleInProgress);
    app
}

fn send_request(app: &mut App, request: HighlightRequest) {
    app.world_mut()
        .resource_mut::<bevy::ecs::message::Messages<HighlightRequest>>()
        .write(request);
}

fn highlight_count(app: &mut App) -> usize {
    let mut q = app
        .world_mut()
        .query_filtered::<Entity, With<HoverHighlight>>();
    q.iter(app.world()).count()
}

fn highlight_state(app: &mut App) -> Option<(Vec3, Visibility)> {
    let mut q = app
        .world_mut()
        .query_filtered::<(&Transform, &Visibility), With<HoverHighlight>>();
    let mut iter = q.iter(app.world());
    iter.next().map(|(t, v)| (t.translation, *v))
}

fn highlight_color(app: &mut App) -> Option<Color> {
    let mut q = app
        .world_mut()
        .query_filtered::<&Sprite, With<HoverHighlight>>();
    let mut iter = q.iter(app.world());
    iter.next().map(|sprite| sprite.color)
}

fn highlight_on_world_layer_at_cell_size(app: &mut App) -> bool {
    let mut q = app
        .world_mut()
        .query_filtered::<(&Sprite, &RenderLayers), With<HoverHighlight>>();
    let world_layer = RenderLayers::layer(WORLD_RENDER_LAYER);
    q.iter(app.world()).all(|(sprite, layers)| {
        sprite.custom_size == Some(Vec2::splat(CELL_PX)) && layers.intersects(&world_layer)
    })
}

#[test]
fn presenter_draws_highlight_from_the_request() {
    let level = Level::new(0);
    let mut app = highlight_app();

    app.update();
    assert_eq!(
        highlight_count(&mut app),
        0,
        "no highlight sprite before any Some request (lazy spawn)",
    );

    let cell_a = CellLevel::new(Cell::new(4, 7), level);
    send_request(&mut app, visible_request(Some(cell_a)));
    app.update();
    assert_eq!(
        highlight_count(&mut app),
        1,
        "exactly one highlight sprite after the first Some request",
    );
    assert_eq!(
        highlight_state(&mut app),
        Some((cell_to_world(cell_a.cell(), level), Visibility::Visible,)),
        "the highlight must be visible at cell_to_world(requested cell A)",
    );
    assert!(
        highlight_on_world_layer_at_cell_size(&mut app),
        "the highlight must be CELL_PX-sized on the WORLD_RENDER_LAYER",
    );

    let cell_b = CellLevel::new(Cell::new(11, 2), level);
    send_request(&mut app, visible_request(Some(cell_b)));
    app.update();
    assert_eq!(
        highlight_count(&mut app),
        1,
        "still exactly one highlight sprite after a second Some request (no duplicate)",
    );
    assert_eq!(
        highlight_state(&mut app),
        Some((cell_to_world(cell_b.cell(), level), Visibility::Visible,)),
        "the highlight must have MOVED to cell_to_world(requested cell B)",
    );

    send_request(&mut app, visible_request(None));
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

    send_request(&mut app, visible_request(Some(cell_a)));
    app.update();
    assert_eq!(
        highlight_state(&mut app),
        Some((cell_to_world(cell_a.cell(), level), Visibility::Visible,)),
        "a Some request after a None must re-show the highlight at the new cell",
    );
}

#[test]
fn no_highlight_drawn_without_battle_in_progress() {
    let level = Level::new(0);
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(TopDownRendererPlugin);

    let cell = CellLevel::new(Cell::new(4, 7), level);
    send_request(&mut app, visible_request(Some(cell)));
    app.update();

    assert_eq!(
        highlight_count(&mut app),
        0,
        "no highlight may be drawn when BattleInProgress is absent (the draw is battle-gated)",
    );
}

#[test]
fn presenter_recolours_reticle_off_the_verdict() {
    let level = Level::new(0);
    let mut app = highlight_app();
    let cell = CellLevel::new(Cell::new(6, 6), level);

    send_request(
        &mut app,
        HighlightRequest::new(Some(cell), CellVisibility::NotSquadVisible),
    );
    app.update();
    assert_eq!(
        highlight_count(&mut app),
        1,
        "one reticle after the first request",
    );
    let unseen_tint = highlight_color(&mut app);
    assert!(unseen_tint.is_some(), "the reticle must carry a tint");

    send_request(
        &mut app,
        HighlightRequest::new(Some(cell), CellVisibility::SquadVisible),
    );
    app.update();
    assert_eq!(
        highlight_count(&mut app),
        1,
        "still ONE reticle after the verdict flip (no duplicate)",
    );
    let visible_tint = highlight_color(&mut app);
    assert!(visible_tint.is_some(), "the reticle must carry a tint");

    assert_ne!(
        unseen_tint, visible_tint,
        "the unseen tint must DIFFER from the visible tint (the reticle recolours)",
    );

    send_request(
        &mut app,
        HighlightRequest::new(Some(cell), CellVisibility::NotSquadVisible),
    );
    app.update();
    assert_eq!(
        highlight_color(&mut app),
        unseen_tint,
        "an EXPLORED-collapsed NotSquadVisible verdict takes the SAME unseen tint as UNSEEN",
    );
    assert_eq!(
        highlight_count(&mut app),
        1,
        "still ONE reticle (the recolour mutates in place)",
    );
}
