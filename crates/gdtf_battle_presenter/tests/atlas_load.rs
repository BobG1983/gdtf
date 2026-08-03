use bevy::{app::App, asset::Assets, image::TextureAtlasLayout};
use gdtf_app::test_support::AppState;
use gdtf_battle_presenter::{SheetRole, TopDownAtlases};
use gdtf_test_utils::{GdtfLoadTestAppBuilder, advance_until_resource_exists};

const LOAD_SAFETY_NET: u32 = 10_000;

fn layout_tile_count(app: &App, role: SheetRole) -> Option<usize> {
    let atlases = app.world().get_resource::<TopDownAtlases>()?;
    let sheet = atlases.role(role)?;
    let layouts = app.world().get_resource::<Assets<TextureAtlasLayout>>()?;
    let layout = layouts.get(&sheet.layout)?;
    Some(layout.textures.len())
}

fn first_tile_size(app: &App, role: SheetRole) -> Option<(u32, u32)> {
    let atlases = app.world().get_resource::<TopDownAtlases>()?;
    let sheet = atlases.role(role)?;
    let layouts = app.world().get_resource::<Assets<TextureAtlasLayout>>()?;
    let layout = layouts.get(&sheet.layout)?;
    let rect = layout.textures.first()?;
    let size = rect.size();
    Some((size.x, size.y))
}

#[test]
fn topdown_renderer_builds_the_sheet_atlases_including_32px_portraits() {
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();

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

    assert_eq!(
        layout_tile_count(&app, SheetRole::Portraits),
        Some(100),
        "portraits layout must have 10×10 == 100 faces",
    );
    assert_eq!(
        first_tile_size(&app, SheetRole::Portraits),
        Some((32, 32)),
        "each portrait face must be 32×32 px (the per-sheet tile size)",
    );
    assert_eq!(
        first_tile_size(&app, SheetRole::Terrain),
        Some((16, 16)),
        "terrain tiles must stay 16×16 px",
    );
}

// Portraits nearest-sampler is pinned by a unit test in the image load path.
