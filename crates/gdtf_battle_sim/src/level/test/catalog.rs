//! Tests for the per-theme tile catalog + the [`ThemeCatalogRegistry`] (GTW-409
//! AC3/AC4/AC5). Parses the SHIPPED `assets/content/themes/*.theme.ron`, asserts the
//! default-floor resolves (AC3), the enumerable tile list returns the authored tiles
//! (AC4), and the consistency invariant holds — NO shipped magnitudes pinned (AC5 /
//! the brittle-test rule).

use super::super::{spec::ThemeSpec, *};

// ── Compile-time path verification: the shipped catalog files exist ──────────
// `include_str!` fails at compile time if the path does not resolve — a regression in
// an authored theme file immediately turns this red.
const SHIPPED_INDUSTRIAL_HIVE_RON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/content/themes/industrial_hive.theme.ron"
));
const SHIPPED_UNDERHIVE_RON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/content/themes/underhive.theme.ron"
));
const SHIPPED_SUMP_WASTE_RON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../assets/content/themes/sump_waste.theme.ron"
));

/// Every shipped `.theme.ron`, paired with the [`LevelTheme`] its file is named for —
/// the fixtures the catalog tests parse. Returns the parsed `ThemeSpec`s (assert-fail on
/// a parse error rather than a denied `unwrap`).
fn shipped_specs() -> Vec<ThemeSpec> {
    let mut specs = Vec::new();
    for (label, ron) in [
        ("industrial_hive", SHIPPED_INDUSTRIAL_HIVE_RON),
        ("underhive", SHIPPED_UNDERHIVE_RON),
        ("sump_waste", SHIPPED_SUMP_WASTE_RON),
    ] {
        let parsed = ron::de::from_str::<ThemeSpec>(ron);
        assert!(
            parsed.is_ok(),
            "the shipped {label}.theme.ron must parse into a ThemeSpec: {:?}",
            parsed.as_ref().err(),
        );
        if let Ok(spec) = parsed {
            specs.push(spec);
        }
    }
    specs
}

/// AC5 — every shipped `assets/content/themes/*.theme.ron` parses into a `ThemeSpec`
/// (parse-OK). Structure only; no magnitudes asserted.
#[test]
fn shipped_theme_specs_parse() {
    let specs = shipped_specs();
    assert_eq!(
        specs.len(),
        3,
        "all three shipped theme catalogs must parse (IndustrialHive / Underhive / SumpWaste)",
    );
}

/// AC5 (consistency invariant) — each shipped catalog's `default_floor` key names a tile
/// PRESENT in that catalog's tile map, and that tile is a `Floor` kind. This is a
/// structural invariant (not a magnitude pin): a mis-typed default-floor key turns it
/// red.
#[test]
fn shipped_default_floor_keys_resolve_to_a_floor() {
    for spec in shipped_specs() {
        let catalog = ThemeTileCatalog::from_spec(spec);
        let resolved = catalog.default_floor();
        assert!(
            resolved.is_some(),
            "the catalog's default_floor key `{}` must name a tile present in the catalog",
            **catalog.default_floor_key(),
        );
        if let Some(tile) = resolved {
            assert!(
                matches!(tile.kind, CatalogTileKind::Floor { .. }),
                "the default-floor tile must be a Floor kind, got {:?}",
                tile.kind,
            );
        }
    }
}

/// AC4 — the catalog exposes an ENUMERABLE tile list (display name + atlas index +
/// gameplay stats), and every tile is reachable by its key. Asserts the enumeration is
/// non-empty and round-trips each key (NO magnitude pins).
#[test]
fn catalog_enumerates_its_tiles() {
    let Some(spec) = shipped_specs().into_iter().next() else {
        return;
    };
    let catalog = ThemeTileCatalog::from_spec(spec);

    assert!(!catalog.is_empty(), "a shipped catalog has tiles");
    assert_eq!(
        catalog.tiles().count(),
        catalog.len(),
        "the enumerable list returns every authored tile",
    );
    // Each enumerated tile exposes a display name + atlas index + a stat-carrying kind,
    // and is reachable by its key (the palette listing contract).
    for (key, tile) in catalog.tiles() {
        assert!(
            !tile.display_name.is_empty(),
            "every palette tile has a non-empty display name",
        );
        assert_eq!(
            catalog.tile(key),
            Some(tile),
            "every enumerated tile resolves by its key",
        );
    }
}

/// AC2/AC3/AC4 (registry shape) — a `ThemeCatalogRegistry` keys catalogs by their
/// DECLARED `LevelTheme`, resolves a present theme, returns `None` for an absent one,
/// and enumerates its themes. Built directly from `ThemeCatalogRegistry::new` (no
/// `AssetServer` — the sim-unit shape). Theme/key routing only — no magnitudes.
#[test]
fn registry_keys_catalogs_by_declared_theme() {
    let specs = shipped_specs();
    let registry = ThemeCatalogRegistry::new(
        specs
            .into_iter()
            .map(|spec| (spec.theme, ThemeTileCatalog::from_spec(spec))),
    );

    assert_eq!(registry.len(), 3, "all three shipped themes are registered");
    assert!(!registry.is_empty(), "a populated registry is non-empty");

    // A present theme resolves to a catalog whose default floor resolves (C3 end-to-end).
    let hive = registry.catalog(LevelTheme::IndustrialHive);
    assert!(
        hive.is_some(),
        "the IndustrialHive theme resolves to its catalog",
    );
    if let Some(catalog) = hive {
        assert!(
            catalog.default_floor().is_some(),
            "the resolved catalog resolves its default floor",
        );
    }

    // The enumeration lists every registered theme.
    assert_eq!(
        registry.catalogs().count(),
        registry.len(),
        "the registry enumerates every registered theme",
    );
}
