//! Headless integration test for the GTW-417 map-editor shell.
//!
//! Drives the REAL [`MapEditorPlugin`] on a no-renderer `DefaultPlugins` app with a live
//! [`AssetServer`] (the `gdtf_test_utils` UI-harness config, which points the asset root at
//! the workspace `assets/`), so the editor's actual `Load` pass resolves the shipped theme +
//! content registries and its real `Editing` scene spawns the four regions — not a copy.
//!
//! Asserts the AC1/AC2 contract end-to-end: the editor reaches [`EditorState::Editing`]; the
//! theme + theme-catalog + weapon + armor registry resources exist AND are POPULATED (a known
//! shipped key resolves in each) after the load pass; and the FOUR region marker entities are
//! spawned. Value-agnostic (it asserts membership / a known authored key resolves, never a
//! registry's magnitude or a region's geometry) and pin-discriminating: a missing region, an
//! unresolved registry, OR a registry that resolved EMPTY (the broken-loader empty-default
//! fallback) all fail it.

use bevy::prelude::*;
use gdtf_battle_sim::{
    armor::{ArmorName, ArmorRegistry},
    level::{LevelTheme, ThemeCatalogRegistry},
    weapon::{WeaponName, WeaponRegistry},
};
use gdtf_editor::{
    CanvasRegion, EditorState, LeftPaletteRegion, MapEditorPlugin, RightPanelRegion, StatRegion,
};
use gdtf_test_utils::{GdtfUiTestAppBuilder, advance_until};
use gdtf_ui::{UiPlugin, theme::GdtfTheme};

/// A generous frame cap: the async asset loads under parallel `cargo` contention take a
/// non-deterministic number of frames, so this is a SAFETY NET (not a timing budget) — we
/// poll the `EditorState::Editing` SIGNAL, not a fixed count.
const MAX_UPDATES: u32 = 10_000;

/// Builds the real editor app on the no-renderer `DefaultPlugins` UI harness: a live
/// `AssetServer` (rooted at the workspace `assets/`), a UI camera, the `gdtf_ui` `UiPlugin`,
/// and the editor's own [`MapEditorPlugin`]. This is the SAME plugin the binary wires — the
/// test exercises the production code path, only swapping the windowed renderer for the
/// headless harness.
fn editor_app() -> App {
    let mut app = GdtfUiTestAppBuilder::new().with_ui_camera().build();
    app.add_plugins(UiPlugin);
    app.add_plugins(MapEditorPlugin);
    app
}

/// The editor reaches [`EditorState::Editing`] once its `Load` pass resolves the theme +
/// registries — the AC1 "theme + registries LOADED" signal.
#[test]
fn editor_reaches_editing_with_registries_loaded() {
    let mut app = editor_app();

    let reached = advance_until(
        &mut app,
        |app| {
            app.world()
                .get_resource::<State<EditorState>>()
                .is_some_and(|s| *s.get() == EditorState::Editing)
        },
        MAX_UPDATES,
    );
    assert!(
        reached,
        "the editor never reached EditorState::Editing — its Load pass did not resolve the \
         theme + registries (this is a genuine load failure, not a frame-budget shortfall)",
    );

    // AC1: the theme + the three content registries are all present after the load pass —
    // AND each registry is POPULATED, not an empty default. The editor Load pass falls back
    // to an EMPTY default registry on a failed folder load and `transition_to_editing` gates
    // only on `is_some()`, so a broken loader (wrong dir, RON parse failure, broken
    // success-build branch) would still reach `Editing` with empty registries. These
    // population assertions discriminate that: they FAIL if any registry resolves EMPTY.
    // Value-agnostic — they assert membership / a known authored key resolves, NEVER any
    // authored magnitude. Mirrors the landed real-asset Load-flow bar (load_themes.rs /
    // load_weapons.rs / load_armor.rs).
    let world = app.world();
    assert!(
        world.get_resource::<GdtfTheme>().is_some(),
        "the GdtfTheme must be inserted by the editor Load pass",
    );

    // WeaponRegistry: present, non-empty, and the shipped `stub_pistol` key resolves
    // (assets/content/weapons/stub_pistol.weapon.ron). Mirrors load_weapons.rs.
    let weapons = world.get_resource::<WeaponRegistry>();
    assert!(
        weapons.is_some(),
        "the WeaponRegistry must be inserted by the editor Load pass",
    );
    if let Some(weapons) = weapons {
        assert!(
            !weapons.is_empty(),
            "the editor's WeaponRegistry must be POPULATED from assets/content/weapons/, not \
             an empty default fallback (a broken loader must redden this, not slip through)",
        );
        assert!(
            weapons
                .spec(&WeaponName::new("stub_pistol".to_owned()))
                .is_some(),
            "the WeaponRegistry must hold the shipped `stub_pistol` weapon \
             (keyed by stub_pistol.weapon.ron's stem) — proves a real folder resolve",
        );
    }

    // ArmorRegistry: present, non-empty, and the shipped `flak_vest` key resolves
    // (assets/content/armor/flak_vest.armor.ron). Mirrors load_armor.rs.
    let armor = world.get_resource::<ArmorRegistry>();
    assert!(
        armor.is_some(),
        "the ArmorRegistry must be inserted by the editor Load pass",
    );
    if let Some(armor) = armor {
        assert!(
            !armor.is_empty(),
            "the editor's ArmorRegistry must be POPULATED from assets/content/armor/, not an \
             empty default fallback (a broken loader must redden this, not slip through)",
        );
        assert!(
            armor
                .spec(&ArmorName::new("flak_vest".to_owned()))
                .is_some(),
            "the ArmorRegistry must hold the shipped `flak_vest` armor \
             (keyed by flak_vest.armor.ron's stem) — proves a real folder resolve",
        );
    }

    // ThemeCatalogRegistry: present, non-empty, and the shipped `IndustrialHive` catalog
    // resolves (declared by industrial_hive.theme.ron). Mirrors load_themes.rs.
    let catalogs = world.get_resource::<ThemeCatalogRegistry>();
    assert!(
        catalogs.is_some(),
        "the ThemeCatalogRegistry (GTW-409 theme tile catalog) must be inserted by the editor \
         Load pass",
    );
    if let Some(catalogs) = catalogs {
        assert!(
            !catalogs.is_empty(),
            "the editor's ThemeCatalogRegistry must be POPULATED from assets/content/themes/, \
             not an empty default fallback (a broken loader must redden this, not slip through)",
        );
        assert!(
            catalogs.catalog(LevelTheme::IndustrialHive).is_some(),
            "the ThemeCatalogRegistry must hold the shipped `IndustrialHive` catalog \
             (declared by industrial_hive.theme.ron) — proves a real folder resolve",
        );
    }
}

/// AC2: the FOUR layout regions are spawned as marker-bearing entities once the editor is in
/// `Editing`. Pin-discriminating — dropping any region from `spawn_editor_shell` fails the
/// matching assert.
#[test]
fn editor_spawns_the_four_regions() {
    let mut app = editor_app();

    let reached = advance_until(
        &mut app,
        |app| {
            app.world()
                .get_resource::<State<EditorState>>()
                .is_some_and(|s| *s.get() == EditorState::Editing)
        },
        MAX_UPDATES,
    );
    assert!(reached, "the editor never reached EditorState::Editing");

    // The `OnEnter(Editing)` spawn + its deferred re-parent commands apply across a couple of
    // frames; advance a few more so the region entities are present before we query.
    for _ in 0..4 {
        app.update();
    }

    assert_eq!(
        count::<RightPanelRegion>(&mut app),
        1,
        "exactly one scrollable RIGHT panel region must be spawned",
    );
    assert_eq!(
        count::<LeftPaletteRegion>(&mut app),
        1,
        "exactly one scrollable LEFT palette region must be spawned",
    );
    assert_eq!(
        count::<CanvasRegion>(&mut app),
        1,
        "exactly one central CANVAS region must be spawned",
    );
    assert_eq!(
        count::<StatRegion>(&mut app),
        1,
        "exactly one BOTTOM-RIGHT stat region must be spawned",
    );
}

/// Counts entities carrying the marker component `M` in the app's world.
fn count<M: Component>(app: &mut App) -> usize {
    app.world_mut().query::<&M>().iter(app.world()).count()
}
