//! C3 — the [`UuidThemeRegistry`] holds a [`UuidThemeDef`] inserted by-key, looks it up
//! by [`ThemeUuid`], RESOLVES its `default_floor` [`TerrainUuid`], and ENUMERATES its
//! terrain UUID list — exercised through the REAL registry, not unreachable dead code.

use super::super::{ThemeDisplayName, ThemeUuid, UuidThemeDef, UuidThemeRegistry};
use crate::terrain::def::TerrainUuid;

/// Build a fixture theme definition with the given key, default floor, and palette.
fn theme_def(
    key: ThemeUuid,
    default_floor: TerrainUuid,
    terrain: Vec<TerrainUuid>,
) -> UuidThemeDef {
    UuidThemeDef {
        key,
        display_name: ThemeDisplayName::new("Industrial Hive".to_owned()),
        default_floor,
        terrain,
    }
}

/// C3 — insert a theme under its [`ThemeUuid`] key, look it up, resolve its default-floor
/// [`TerrainUuid`], and enumerate its terrain UUID list. Built directly from
/// [`UuidThemeRegistry::insert`] (the sim-unit shape — no `AssetServer`). No magnitude
/// assertions — key routing / resolve / enumeration only.
#[test]
fn registry_inserts_resolves_floor_and_enumerates_terrain() {
    let key = ThemeUuid::generate();
    let floor = TerrainUuid::generate();
    let extra = TerrainUuid::generate();
    let def = theme_def(key, floor, vec![floor, extra]);

    let mut registry = UuidThemeRegistry::default();
    assert!(registry.is_empty(), "a fresh registry is empty");

    let previous = registry.insert(key, def.clone());
    assert!(
        previous.is_none(),
        "the first insert under a key has no predecessor",
    );
    assert_eq!(
        registry.len(),
        1,
        "the registry holds the one inserted theme",
    );
    assert!(!registry.is_empty(), "a one-theme registry is non-empty");

    // Look up by ThemeUuid.
    assert_eq!(
        registry.def(&key),
        Some(&def),
        "a present key resolves to its theme definition",
    );
    assert!(
        registry.def(&ThemeUuid::generate()).is_none(),
        "a fresh, never-inserted key resolves to None",
    );

    // RESOLVE the default-floor TerrainUuid.
    assert_eq!(
        registry.default_floor(&key),
        Some(floor),
        "the registry resolves the theme's default-floor terrain UUID",
    );
    assert!(
        registry.default_floor(&ThemeUuid::generate()).is_none(),
        "an absent key resolves to no default floor",
    );

    // ENUMERATE the terrain UUID list.
    assert_eq!(
        registry.terrain(&key),
        Some([floor, extra].as_slice()),
        "the registry enumerates the theme's terrain UUID palette",
    );
    assert!(
        registry.terrain(&ThemeUuid::generate()).is_none(),
        "an absent key enumerates no terrain",
    );

    // The registry-wide enumeration sees the one theme.
    assert_eq!(
        registry.defs().count(),
        1,
        "the registry enumerates its one theme",
    );
}

/// C3 — the `(key, def)` constructor builds a registry keyed by [`ThemeUuid`] (the loader
/// shape) and resolves the inserted key.
#[test]
fn registry_new_keys_by_uuid() {
    let key = ThemeUuid::generate();
    let floor = TerrainUuid::generate();
    let def = theme_def(key, floor, vec![floor]);

    let registry = UuidThemeRegistry::new([(key, def.clone())]);
    assert_eq!(registry.len(), 1, "the constructor holds the one theme");
    assert_eq!(
        registry.def(&key),
        Some(&def),
        "the constructed registry resolves the inserted key",
    );
    assert_eq!(
        registry.default_floor(&key),
        Some(floor),
        "the constructed registry resolves the default floor",
    );
}
