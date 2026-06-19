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
//! brittle): terrain 352 (16×22), characters 288 (16×18), effects 128 (16×8), and the
//! GTW-278 portraits 100 (10×10) — plus the portraits' per-tile SIZE (32 px), to pin the
//! per-sheet tile-size wiring (a regression to 16 would mis-carve each face). Each layout
//! handle is resolved against `Assets<TextureAtlasLayout>` without `unwrap`/`expect` — the
//! `Option` is matched.
//!
//! NO function here takes `&mut World`/`&World`; every `app.world()` call is in the
//! TEST BODY (the `bevy-traps.md` #7 carve-out a headless idiom).

use bevy::{app::App, asset::Assets, image::TextureAtlasLayout};
use gdtf_app::test_support::AppState;
use gdtf_battle_presenter::{SheetRole, TopDownAtlases};
use gdtf_test_utils::{GdtfLoadTestAppBuilder, advance_until_resource_exists};

/// Generous SAFETY-NET cap for the wait on the `TopDownAtlases` resource being built. It is a
/// safety net against a genuine never-build hang, NOT a timing budget: the wait keys off the
/// resource's inserted SIGNAL (not a fixed frame count), so it stays deterministic under
/// parallel `cargo` load (GTW-305). The resource is inserted by the `Startup` system and its
/// layouts are added synchronously there (only the image bytes load async, which this test
/// does not wait on — it asserts the layout STRUCTURE, available the moment the resource exists).
const LOAD_SAFETY_NET: u32 = 10_000;

/// Reads the tile count of `role`'s layout out of the `TopDownAtlases` resource and
/// `Assets<TextureAtlasLayout>`, without panicking on a missing resource/handle.
fn layout_tile_count(app: &App, role: SheetRole) -> Option<usize> {
    let atlases = app.world().get_resource::<TopDownAtlases>()?;
    let sheet = atlases.role(role)?;
    let layouts = app.world().get_resource::<Assets<TextureAtlasLayout>>()?;
    let layout = layouts.get(&sheet.layout)?;
    Some(layout.textures.len())
}

/// Reads the SIZE (`width`, `height` in px) of the FIRST tile of `role`'s layout — the
/// per-sheet tile size threaded through `from_grid` (GTW-278). `None` on a missing
/// resource/handle/empty layout.
fn first_tile_size(app: &App, role: SheetRole) -> Option<(u32, u32)> {
    let atlases = app.world().get_resource::<TopDownAtlases>()?;
    let sheet = atlases.role(role)?;
    let layouts = app.world().get_resource::<Assets<TextureAtlasLayout>>()?;
    let layout = layouts.get(&sheet.layout)?;
    let rect = layout.textures.first()?;
    let size = rect.size();
    Some((size.x, size.y))
}

/// AC4 — `TopDownRendererPlugin` builds the `TopDownAtlases` resource ONCE with one
/// `TextureAtlasLayout` per sheet of the expected structural tile count, INCLUDING the
/// GTW-278 portraits sheet built at its OWN 32-px tile size.
///
/// Pin-discriminating: it drives the REAL `AssetServer` + the registered
/// `TopDownRendererPlugin` (via the real scene stack). If the resource were never
/// inserted, the wait times out and the asserts fail; if a sheet's `from_grid`
/// dimensions were wrong, the tile-count assert fails; if a role were missing
/// (e.g. portraits not loaded), `role()` returns `None` and the count is absent; if the
/// portraits tile size were reverted to 16, the per-tile-size assert fails (each face
/// would carve into four wrong sub-tiles).
#[test]
fn topdown_renderer_builds_the_sheet_atlases_including_32px_portraits() {
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

    // Signal-poll the built resource: drive until `TopDownAtlases` is inserted (the Startup
    // system builds it once on the real scene stack with a live AssetServer), not a fixed frame
    // count — the cap is a safety net (GTW-305). Its layouts are added synchronously in that same
    // build, so the moment the resource exists the structural asserts below are valid.
    advance_until_resource_exists::<TopDownAtlases>(&mut app, LOAD_SAFETY_NET);
    assert!(
        layout_tile_count(&app, SheetRole::Terrain).is_some(),
        "TopDownRendererPlugin must build the TopDownAtlases resource with terrain's layout — \
         proves the Startup load system ran on the real scene stack with a live AssetServer",
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

    // GTW-278 — the portraits sheet is a 10×10 grid of 32-px faces (100 indices).
    assert_eq!(
        layout_tile_count(&app, SheetRole::Portraits),
        Some(100),
        "portraits layout must have 10×10 == 100 faces",
    );
    // The portraits tile size is its OWN 32 px (the per-sheet tile-size wiring) — NOT the
    // render sheets' 16. A revert to 16 would carve each 32-px face into four wrong tiles.
    assert_eq!(
        first_tile_size(&app, SheetRole::Portraits),
        Some((32, 32)),
        "each portrait face must be 32×32 px (the per-sheet tile size)",
    );
    // The render sheets stay 16 px — the portrait tile size must not have changed theirs.
    assert_eq!(
        first_tile_size(&app, SheetRole::Terrain),
        Some((16, 16)),
        "terrain tiles must stay 16×16 px",
    );
}

// NOTE (GTW-295): the PORTRAITS nearest-sampler decision is pinned by a unit test in
// `crate::topdown::test` (`SheetRole::sampler_override`), NOT here — the headless `no_renderer`
// config never finishes DECODING the image into `Assets<Image>` (the load state stays
// `Loading` without a render device), so the loaded image's sampler is unreachable from an
// integration test. The visual (no white fringe) is in-engine QA.
