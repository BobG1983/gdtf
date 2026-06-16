//! GTW-217 (GTW-48 S3): the AC4 headless atlas-load test — proof that
//! `TopDownRendererPlugin` builds the role-keyed `TopDownAtlases` resource ONCE from
//! a real `AssetServer`, with one `TextureAtlasLayout` per render sheet of the
//! expected STRUCTURAL tile count.
//!
//! It reuses the [`GdtfLoadTestAppBuilder`] harness (`DefaultPlugins`/`no_renderer`
//! with a live `AssetServer` rooted at the workspace `assets/` dir) because the plain
//! `MinimalPlugins` [`GdtfTestAppBuilder`](gdtf_test_utils::GdtfTestAppBuilder) has NO
//! `AssetServer`. That harness registers the real scene stack, whose
//! `GameBattleScapeScenePlugin` adds `BattlePresenterPlugin::default()` →
//! `TopDownRendererPlugin`, so the atlas-load `Startup` system is already scheduled —
//! the test just drives a bounded number of `update()`s for it to run, then asserts.
//!
//! It asserts the STRUCTURAL tile COUNT of each layout (never pixel content — not
//! brittle): terrain 352 (16×22), characters 288 (16×18), effects 128 (16×8). Each
//! layout handle is resolved against `Assets<TextureAtlasLayout>` without
//! `unwrap`/`expect` — the `Option` is matched.
//!
//! NO function here takes `&mut World`/`&World`; every `app.world()` call is in the
//! TEST BODY (the `bevy-traps.md` #7 carve-out a headless idiom).

use bevy::{app::App, asset::Assets, image::TextureAtlasLayout};
use gdtf_app::test_support::AppState;
use gdtf_battle_presenter::{SheetRole, TopDownAtlases};
use gdtf_test_utils::{GdtfLoadTestAppBuilder, advance_until};

/// Generous so a slow CI box never flakes; the resource is inserted by the `Startup`
/// system on the first `update()` and the layouts are added synchronously there
/// (only the image bytes load async, which this test does not wait on — it asserts
/// the layout STRUCTURE, available immediately).
const MAX_UPDATES: u32 = 64;

/// Reads the tile count of `role`'s layout out of the `TopDownAtlases` resource and
/// `Assets<TextureAtlasLayout>`, without panicking on a missing resource/handle.
fn layout_tile_count(app: &App, role: SheetRole) -> Option<usize> {
    let atlases = app.world().get_resource::<TopDownAtlases>()?;
    let sheet = atlases.role(role)?;
    let layouts = app.world().get_resource::<Assets<TextureAtlasLayout>>()?;
    let layout = layouts.get(&sheet.layout)?;
    Some(layout.textures.len())
}

/// AC4 — `TopDownRendererPlugin` builds the `TopDownAtlases` resource ONCE with one
/// `TextureAtlasLayout` per render sheet of the expected structural tile count.
///
/// Pin-discriminating: it drives the REAL `AssetServer` + the registered
/// `TopDownRendererPlugin` (via the real scene stack). If the resource were never
/// inserted, the wait times out and the asserts fail; if a sheet's `from_grid`
/// dimensions were wrong, the tile-count assert fails; if a role were missing
/// (e.g. terrain not loaded), `role()` returns `None` and the count is absent.
#[test]
fn topdown_renderer_builds_the_three_sheet_atlases() {
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    // Drive until the resource is present and terrain's layout has resolved (the
    // Startup system runs on the first update; this is just settle headroom).
    let built = advance_until(
        &mut app,
        |app| layout_tile_count(app, SheetRole::Terrain).is_some(),
        MAX_UPDATES,
    );
    assert!(
        built,
        "TopDownRendererPlugin must build the TopDownAtlases resource within \
         {MAX_UPDATES} updates — proves the Startup load system ran on the real \
         scene stack with a live AssetServer",
    );

    assert_eq!(
        layout_tile_count(&app, SheetRole::Terrain),
        Some(352),
        "terrain layout must have 16×22 == 352 tiles",
    );
    assert_eq!(
        layout_tile_count(&app, SheetRole::Characters),
        Some(288),
        "characters layout must have 16×18 == 288 tiles",
    );
    assert_eq!(
        layout_tile_count(&app, SheetRole::Effects),
        Some(128),
        "effects layout must have 16×8 == 128 tiles",
    );
}
