//! `Update` (during [`EditorState::Load`](crate::EditorState)): poll the in-flight loads
//! and insert the resolved resources.
//!
//! A slim mirror of `gdtf_app`'s `poll_and_resolve`, trimmed to the editor's four loads:
//! the [`GdtfTheme`] and the [`WeaponRegistry`] / [`ArmorRegistry`] /
//! [`ThemeCatalogRegistry`]. Each branch re-gates on its OWN resource's absence so none
//! starves another (`bevy-traps.md` #3), and EVERY branch is fail-safe: a failed asset
//! falls back to a const default (the ADR-0003 error-path safety-net) so the editor never
//! hangs in `Load` on a bad asset folder. Once all four resources exist, the plugin's
//! transition leaves `Load` for [`Editing`](crate::EditorState::Editing).

use bevy::{
    asset::{AssetServer, Assets, LoadState, LoadedFolder, RecursiveDependencyLoadState},
    ecs::system::SystemParam,
    prelude::*,
};
use gdtf_assets::RonAsset;
use gdtf_battle_sim::{
    armor::{ArmorName, ArmorRegistry, ArmorSpec},
    level::{ThemeCatalogRegistry, ThemeSpec, ThemeTileCatalog},
    weapon::{WeaponName, WeaponRegistry, WeaponSpec},
};
use gdtf_ui::theme::{ActiveThemeHandle, GdtfTheme, GdtfThemeSpec, default_theme};

use crate::load::handles::EditorLoadHandles;

/// The loaded RON asset collections [`poll_and_resolve_editor`] reads, bundled into one
/// [`SystemParam`] so the system's parameter list stays under clippy's argument-count gate
/// (the game's `LoadAssetCollections` grouping precedent).
///
/// Each is `Option<Res<…>>` because a headless app without the asset stack has no
/// `AssetServer` (and so no `Assets<…>` collections); the system early-returns when any is
/// absent, so it never panics on a missing collection (`bevy-traps.md` #1).
#[derive(SystemParam)]
pub(crate) struct EditorLoadCollections<'w> {
    /// The loaded theme-spec RON collection (`core_tuning/ui_theme.tuning.ron`).
    theme_specs:   Option<Res<'w, Assets<RonAsset<GdtfThemeSpec>>>>,
    /// The loaded `LoadedFolder` collection — used to read each content folder's members.
    folders:       Option<Res<'w, Assets<LoadedFolder>>>,
    /// The loaded per-weapon RON collection (`content/weapons/*.weapon.ron`).
    weapon_specs:  Option<Res<'w, Assets<RonAsset<WeaponSpec>>>>,
    /// The loaded per-armor RON collection (`content/armor/*.armor.ron`).
    armor_specs:   Option<Res<'w, Assets<RonAsset<ArmorSpec>>>>,
    /// The loaded per-theme RON collection (`content/themes/*.theme.ron`).
    catalog_specs: Option<Res<'w, Assets<RonAsset<ThemeSpec>>>>,
}

/// The four persistent resources [`poll_and_resolve_editor`] resolves, each as an
/// `Option<Res<…>>` presence-probe, bundled into one [`SystemParam`] so the system's
/// parameter list stays under clippy's argument-count gate (the game's `ResolvedResources`
/// grouping precedent). Each branch resolves on its OWN resource's absence so none starves
/// another (`bevy-traps.md` #3).
#[derive(SystemParam)]
pub(crate) struct EditorResolved<'w> {
    /// Whether the resolved [`GdtfTheme`] is already inserted.
    theme:    Option<Res<'w, GdtfTheme>>,
    /// Whether the resolved [`WeaponRegistry`] is already inserted.
    weapons:  Option<Res<'w, WeaponRegistry>>,
    /// Whether the resolved [`ArmorRegistry`] is already inserted.
    armor:    Option<Res<'w, ArmorRegistry>>,
    /// Whether the resolved [`ThemeCatalogRegistry`] is already inserted.
    catalogs: Option<Res<'w, ThemeCatalogRegistry>>,
}

/// Polls the editor's in-flight loads and inserts each resolved resource on its OWN
/// absence guard.
///
/// Runs while [`EditorLoadHandles`] exists and ANY of the four target resources
/// ([`GdtfTheme`], [`WeaponRegistry`], [`ArmorRegistry`], [`ThemeCatalogRegistry`]) is
/// still missing. Each branch resolves independently and falls back to a const default on
/// a failed load, so a slow/bad asset never strands the editor. Takes its borrows as
/// `Option<…>` so a headless app without the asset stack no-ops rather than panics
/// (`bevy-traps.md` #1); under `DefaultPlugins` (the real editor) they are always present.
pub(crate) fn poll_and_resolve_editor(
    mut commands: Commands,
    asset_server: Option<Res<AssetServer>>,
    collections: EditorLoadCollections,
    resolved: EditorResolved,
    handles: Option<Res<EditorLoadHandles>>,
) {
    let (theme_done, weapons_done, armor_done, catalogs_done) = (
        resolved.theme.is_some(),
        resolved.weapons.is_some(),
        resolved.armor.is_some(),
        resolved.catalogs.is_some(),
    );
    let (
        Some(asset_server),
        Some(theme_assets),
        Some(folders),
        Some(weapon_specs),
        Some(armor_specs),
        Some(catalog_specs),
        Some(handles),
    ) = (
        asset_server,
        collections.theme_specs,
        collections.folders,
        collections.weapon_specs,
        collections.armor_specs,
        collections.catalog_specs,
        handles,
    )
    else {
        return;
    };

    if !theme_done {
        resolve_theme(&mut commands, &asset_server, &theme_assets, &handles);
    }
    if !weapons_done {
        resolve_weapons(
            &mut commands,
            &asset_server,
            &folders,
            &weapon_specs,
            &handles,
        );
    }
    if !armor_done {
        resolve_armor(
            &mut commands,
            &asset_server,
            &folders,
            &armor_specs,
            &handles,
        );
    }
    if !catalogs_done {
        resolve_themes(
            &mut commands,
            &asset_server,
            &folders,
            &catalog_specs,
            &handles,
        );
    }
}

/// Resolve the loaded theme RON into a [`GdtfTheme`], or fall back to the const default
/// theme on a failed load — mirrors the game's `poll_and_resolve` theme branch. Inserts
/// the persistent [`ActiveThemeHandle`] on both paths so a later file-watcher reload can
/// recover (the editor reuses the same theming seam as the game).
fn resolve_theme(
    commands: &mut Commands,
    asset_server: &AssetServer,
    theme_assets: &Assets<RonAsset<GdtfThemeSpec>>,
    handles: &EditorLoadHandles,
) {
    let theme_state = asset_server.load_state(&*handles.theme);

    if theme_state.is_failed() {
        warn!(
            "GDTF editor Load: theme `core_tuning/ui_theme.tuning.ron` failed to load; falling \
             back to the const default theme",
        );
        commands.insert_resource(default_theme());
        commands.insert_resource(ActiveThemeHandle::new((*handles.theme).clone()));
        return;
    }

    if matches!(theme_state, LoadState::Loaded) {
        let Some(spec) = theme_assets.get(&*handles.theme) else {
            // Loaded-but-not-yet-in-collection: a transient one-frame state; retry next frame.
            return;
        };
        // Resolve each font key on demand (idempotent). The editor does not preload a fonts
        // folder like the game — `load::<Font>` returns the same handle whether or not it was
        // preloaded, so the theme's fonts load lazily here.
        let theme: GdtfTheme = (**spec)
            .clone()
            .resolve(|key| asset_server.load::<Font>(key.to_owned()));
        commands.insert_resource(theme);
        commands.insert_resource(ActiveThemeHandle::new((*handles.theme).clone()));
    }
}

/// Resolve the loaded `content/weapons/` folder into the name-keyed [`WeaponRegistry`], or
/// fall back to an empty registry on a failed folder — the editor mirror of the game's
/// `resolve_weapons`.
fn resolve_weapons(
    commands: &mut Commands,
    asset_server: &AssetServer,
    folders: &Assets<LoadedFolder>,
    weapon_specs: &Assets<RonAsset<WeaponSpec>>,
    handles: &EditorLoadHandles,
) {
    let folder_state = asset_server.recursive_dependency_load_state(&*handles.weapons);

    if matches!(folder_state, RecursiveDependencyLoadState::Failed(_)) {
        warn!(
            "GDTF editor Load: the `weapons` folder failed to load; inserting an empty \
             WeaponRegistry",
        );
        commands.insert_resource(WeaponRegistry::default());
        return;
    }

    if matches!(folder_state, RecursiveDependencyLoadState::Loaded) {
        let Some(folder) = folders.get(&*handles.weapons) else {
            return;
        };
        let mut registry = WeaponRegistry::default();
        for untyped in &folder.handles {
            let handle = untyped
                .clone()
                .typed_debug_checked::<RonAsset<WeaponSpec>>();
            let Some(spec) = weapon_specs.get(&handle) else {
                // A member spec not yet in its collection: retry next frame (do NOT publish
                // a partial registry).
                return;
            };
            let Some(key) = asset_server.get_path(untyped.id()).and_then(|path| {
                path.path()
                    .file_stem()
                    .map(|stem| key_from_stem(&stem.to_string_lossy(), ".weapon"))
            }) else {
                continue;
            };
            registry.insert(WeaponName::new(key), (**spec).clone());
        }
        commands.insert_resource(registry);
    }
}

/// Resolve the loaded `content/armor/` folder into the name-keyed [`ArmorRegistry`], or
/// fall back to an empty registry on a failed folder — the editor mirror of the game's
/// `resolve_armor`.
fn resolve_armor(
    commands: &mut Commands,
    asset_server: &AssetServer,
    folders: &Assets<LoadedFolder>,
    armor_specs: &Assets<RonAsset<ArmorSpec>>,
    handles: &EditorLoadHandles,
) {
    let folder_state = asset_server.recursive_dependency_load_state(&*handles.armor);

    if matches!(folder_state, RecursiveDependencyLoadState::Failed(_)) {
        warn!(
            "GDTF editor Load: the `armor` folder failed to load; inserting an empty \
             ArmorRegistry",
        );
        commands.insert_resource(ArmorRegistry::default());
        return;
    }

    if matches!(folder_state, RecursiveDependencyLoadState::Loaded) {
        let Some(folder) = folders.get(&*handles.armor) else {
            return;
        };
        let mut registry = ArmorRegistry::default();
        for untyped in &folder.handles {
            let handle = untyped.clone().typed_debug_checked::<RonAsset<ArmorSpec>>();
            let Some(spec) = armor_specs.get(&handle) else {
                return;
            };
            let Some(key) = asset_server.get_path(untyped.id()).and_then(|path| {
                path.path()
                    .file_stem()
                    .map(|stem| key_from_stem(&stem.to_string_lossy(), ".armor"))
            }) else {
                continue;
            };
            registry.insert(ArmorName::new(key), **spec);
        }
        commands.insert_resource(registry);
    }
}

/// Resolve the loaded `content/themes/` folder into the theme-keyed
/// [`ThemeCatalogRegistry`], or fall back to an empty registry on a failed folder — the
/// editor mirror of the game's `resolve_themes`. Keyed by each spec's DECLARED
/// [`LevelTheme`](gdtf_battle_sim::level::LevelTheme), not the filename stem.
fn resolve_themes(
    commands: &mut Commands,
    asset_server: &AssetServer,
    folders: &Assets<LoadedFolder>,
    theme_specs: &Assets<RonAsset<ThemeSpec>>,
    handles: &EditorLoadHandles,
) {
    let folder_state = asset_server.recursive_dependency_load_state(&*handles.themes);

    if matches!(folder_state, RecursiveDependencyLoadState::Failed(_)) {
        warn!(
            "GDTF editor Load: the `themes` folder failed to load; inserting an empty \
             ThemeCatalogRegistry",
        );
        commands.insert_resource(ThemeCatalogRegistry::default());
        return;
    }

    if matches!(folder_state, RecursiveDependencyLoadState::Loaded) {
        let Some(folder) = folders.get(&*handles.themes) else {
            return;
        };
        let mut registry = ThemeCatalogRegistry::default();
        for untyped in &folder.handles {
            let handle = untyped.clone().typed_debug_checked::<RonAsset<ThemeSpec>>();
            let Some(spec) = theme_specs.get(&handle) else {
                return;
            };
            // Key by the spec's DECLARED LevelTheme (the editor/procgen lookup key), not the
            // filename stem — the game's resolve_themes contract.
            let spec = (**spec).clone();
            registry.insert(spec.theme, ThemeTileCatalog::from_spec(spec));
        }
        commands.insert_resource(registry);
    }
}

/// Strip a dedicated compound-extension infix (e.g. `.weapon`, `.armor`) off an asset
/// file STEM to recover its registry KEY: `stub_pistol.weapon.ron`'s file stem is
/// `stub_pistol.weapon`, whose key is `stub_pistol`. A stem without the infix is returned
/// unchanged (defensive). Mirrors the game's `weapon_key_from_stem` / `armor_key_from_stem`.
fn key_from_stem(stem: &str, infix: &str) -> String {
    stem.strip_suffix(infix).unwrap_or(stem).to_owned()
}
