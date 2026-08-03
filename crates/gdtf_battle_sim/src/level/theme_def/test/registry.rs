use super::super::{ThemeDisplayName, ThemeUuid, UuidThemeDef, UuidThemeRegistry};
use crate::terrain::def::TerrainUuid;

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

    assert_eq!(
        registry.def(&key),
        Some(&def),
        "a present key resolves to its theme definition",
    );
    assert!(
        registry.def(&ThemeUuid::generate()).is_none(),
        "a fresh, never-inserted key resolves to None",
    );

    assert_eq!(
        registry.default_floor(&key),
        Some(floor),
        "the registry resolves the theme's default-floor terrain UUID",
    );
    assert!(
        registry.default_floor(&ThemeUuid::generate()).is_none(),
        "an absent key resolves to no default floor",
    );

    assert_eq!(
        registry.terrain(&key),
        Some([floor, extra].as_slice()),
        "the registry enumerates the theme's terrain UUID palette",
    );
    assert!(
        registry.terrain(&ThemeUuid::generate()).is_none(),
        "an absent key enumerates no terrain",
    );

    assert_eq!(
        registry.defs().count(),
        1,
        "the registry enumerates its one theme",
    );
}

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
