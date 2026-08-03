use std::path::PathBuf;

use gdtf_app::test_support::{AppState, app_state, load_released};
use gdtf_battle_sim::level::{
    GridHeight, GridLevels, GridSize, GridWidth, PrefabKey, PrefabRegistry, SpawnRole, ThemeUuid,
};
use gdtf_test_utils::{GdtfLoadTestAppBuilder, advance_until, advance_until_resource_exists};

const LOAD_SAFETY_NET: u32 = 10_000;

const fn known_theme_uuid() -> ThemeUuid {
    ThemeUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0184_0a3e_0901))
}

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("prefab_root")
}

fn fixture_size() -> Option<GridSize> {
    GridSize::new(GridWidth::new(3), GridHeight::new(3), GridLevels::new(1)).ok()
}

#[test]
fn real_asset_resolves_prefab_registry_by_theme_uuid() {
    let mut app = GdtfLoadTestAppBuilder::with_asset_root(fixture_root())
        .starting_in(AppState::Load)
        .build();

    advance_until_resource_exists::<PrefabRegistry>(&mut app, LOAD_SAFETY_NET);

    if let Some(registry) = app.world().get_resource::<PrefabRegistry>() {
        assert!(
            !registry.is_empty(),
            "the resolved PrefabRegistry must carry the authored fixture prefab — an empty \
             registry means resolve_prefabs is broken (fell back to the empty default)",
        );

        let Some(size) = fixture_size() else { return };
        let key = PrefabKey::new(known_theme_uuid(), size, SpawnRole::Fill);
        let bucket = registry.prefabs_for(&key);
        assert!(
            !bucket.is_empty(),
            "the (authored ThemeUuid, 3x3x1, Fill) bucket must hold the fixture prefab — \
             bucketing by (theme, size, role) keyed on the ThemeUuid is the C1 contract",
        );

        let carries_placements = bucket
            .iter()
            .any(|prefab| !prefab.spec().placements.is_empty());
        assert!(
            carries_placements,
            "the resolved prefab must carry its authored placement(s)",
        );
    }

    let released = advance_until(&mut app, load_released, LOAD_SAFETY_NET);
    assert!(
        released,
        "with a real AssetServer, Load must release to Intro (or beyond) once every folder \
         (incl. the prefabs) resolves; last AppState was {:?}",
        app_state(&app),
    );
    assert!(
        app.world().get_resource::<PrefabRegistry>().is_some(),
        "a PrefabRegistry must be present when Load reaches Intro (the GTW-489 gate clause \
         waited for it — not a hand-seeded default)",
    );
}
