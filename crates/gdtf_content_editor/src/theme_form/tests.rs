//! In-crate tests for the THEME form's pure projection, the C7 RON round-trip, the C6
//! default-floor rule (Slab-preference + must-be-own-terrain), and the C3 resolved-stats
//! resolution against a `TerrainDefRegistry` fixture (GTW-475).
//!
//! The C7c 3-way toggle + the in-engine evidence live in the crate's integration test
//! (`tests/theme_mode.rs`); these unit-test the pure halves. Panic / expect-free per the
//! workspace lints (`assert!` + `let … else`).

use gdtf_battle_sim::{
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverHp, HeightBand},
    level::{ThemeUuid, UuidThemeDef},
    slab::SlabHp,
    terrain::{
        def::{
            TerrainDef, TerrainDefRegistry, TerrainDisplayName, TerrainPresenterKind,
            TerrainSimKind, TerrainUuid,
        },
        piece::TerrainGraphicKey,
    },
};

use super::{
    resolve::{floor_candidates, resolved_stats, slab_floor_candidates},
    save::{draft_to_theme_def, serialize_theme_def, validate_for_save},
    types::{SaveThemeError, ThemeDraft},
};

/// A terrain UUID from a small constant (a deterministic test key).
fn terrain_key(n: u128) -> TerrainUuid {
    TerrainUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0184_0bcd_0000 + n))
}

/// A theme UUID from a small constant.
fn theme_key() -> ThemeUuid {
    ThemeUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0184_0bcd_9000))
}

/// Build a Slab terrain def (the floor candidate) with a given display name.
fn slab_def(key: TerrainUuid, name: &str) -> TerrainDef {
    TerrainDef {
        key,
        display_name: TerrainDisplayName::new(name.to_owned()),
        sim_kind: TerrainSimKind::Slab {
            hp:               SlabHp::new(120),
            armor_protection: ArmorProtection::new(6),
            armor_hardness:   ArmorHardness::new(3),
        },
        presenter_kind: TerrainPresenterKind::Slab {
            graphic_name: TerrainGraphicKey::new("slab".to_owned()),
            footfall:     None,
        },
        tags: Vec::new(),
    }
}

/// Build a Wall terrain def with a given display name.
fn wall_def(key: TerrainUuid, name: &str) -> TerrainDef {
    TerrainDef {
        key,
        display_name: TerrainDisplayName::new(name.to_owned()),
        sim_kind: TerrainSimKind::Wall {
            hp:               CoverHp::new(80),
            armor_protection: ArmorProtection::new(10),
            armor_hardness:   ArmorHardness::new(5),
            height_band:      HeightBand::High,
        },
        presenter_kind: TerrainPresenterKind::Wall {
            graphic_name: TerrainGraphicKey::new("wall".to_owned()),
        },
        tags: Vec::new(),
    }
}

/// Build a Cover terrain def with a given display name — the third structural kind, used by the
/// GTW-530 C3 mixed-kind fixture to prove the Slab-only default-floor filter drops Cover too.
fn cover_def(key: TerrainUuid, name: &str) -> TerrainDef {
    TerrainDef {
        key,
        display_name: TerrainDisplayName::new(name.to_owned()),
        sim_kind: TerrainSimKind::Cover {
            hp:               CoverHp::new(40),
            armor_protection: ArmorProtection::new(4),
            armor_hardness:   ArmorHardness::new(2),
            height_band:      HeightBand::Low,
        },
        presenter_kind: TerrainPresenterKind::Cover {
            graphic_name: TerrainGraphicKey::new("cover".to_owned()),
        },
        tags: Vec::new(),
    }
}

/// A two-terrain registry fixture: a Wall + a Slab.
fn fixture() -> (TerrainDefRegistry, TerrainUuid, TerrainUuid) {
    let wall = terrain_key(1);
    let slab = terrain_key(2);
    let registry = TerrainDefRegistry::new([
        (wall, wall_def(wall, "Tunnel Wall")),
        (slab, slab_def(slab, "Rockcrete Floor")),
    ]);
    (registry, wall, slab)
}

/// C2/C5 — `draft_to_theme_def` projects the draft by REFERENCE: the theme carries the selected
/// terrain UUIDs + the chosen default floor, NO inlined stats (C3). Pin-discriminating on every
/// projected field.
#[test]
fn draft_projects_to_theme_def_by_reference() {
    let (_, wall, slab) = fixture();
    let mut draft = ThemeDraft::new_theme();
    draft.set_display_name("Industrial Hive".to_owned());
    draft.toggle_terrain(wall);
    draft.toggle_terrain(slab);
    draft.set_default_floor(slab);

    let def = draft_to_theme_def(&draft, theme_key());
    assert_eq!(
        def.key,
        theme_key(),
        "the projected def carries the supplied key"
    );
    assert_eq!(
        &**def.display_name, "Industrial Hive",
        "the display name is projected (trimmed) onto the def",
    );
    assert_eq!(
        def.default_floor, slab,
        "the chosen default floor is projected (C6)"
    );
    assert_eq!(
        def.terrain,
        vec![wall, slab],
        "the selected terrain palette is projected BY REFERENCE — UUIDs only, no inlined stats \
         (C2/C3)",
    );
}

/// C7b — the projected def RON round-trips through the SAME parser the GTW-487 theme loader
/// (`resolve_theme_defs`) uses (`ron::de::from_str::<UuidThemeDef>`): the reloaded def EQUALS the
/// saved one. Pin-discriminating — any dropped field flips the `assert_eq!`. Tunable magnitudes
/// are NOT pinned (the theme references UUIDs only — no stats in the file).
#[test]
fn theme_def_round_trips_through_the_loader_parser() {
    let (_, wall, slab) = fixture();
    let mut draft = ThemeDraft::new_theme();
    draft.set_display_name("Industrial Hive".to_owned());
    draft.toggle_terrain(wall);
    draft.toggle_terrain(slab);
    draft.set_default_floor(slab);

    let def = draft_to_theme_def(&draft, theme_key());
    let serialized = serialize_theme_def(&def);
    assert!(
        serialized.is_ok(),
        "serializing the theme def must succeed: {:?}",
        serialized.as_ref().err(),
    );
    let Ok(serialized) = serialized else { return };

    let reloaded = ron::de::from_str::<UuidThemeDef>(&serialized);
    assert!(
        reloaded.is_ok(),
        "the serialized def must round-trip through the UuidThemeDef deserializer (the GTW-487 \
         theme loader's parser): {:?}",
        reloaded.as_ref().err(),
    );
    let Ok(reloaded) = reloaded else { return };
    assert_eq!(
        reloaded, def,
        "the reloaded UuidThemeDef must equal the saved one — every field survives (C7b)",
    );
}

/// C6 — `validate_for_save` enforces the default-floor rule: a saved theme's default floor must be
/// one of its OWN terrain. A valid draft passes; a missing floor, a floor not in the palette, an
/// empty palette, and an empty name each fail with the matching typed error.
#[test]
fn validate_enforces_default_floor_in_terrain() {
    let (_, wall, slab) = fixture();

    // A complete, valid draft passes.
    let mut draft = ThemeDraft::new_theme();
    draft.set_display_name("Industrial Hive".to_owned());
    draft.toggle_terrain(slab);
    draft.set_default_floor(slab);
    assert_eq!(
        validate_for_save(&draft),
        Ok(()),
        "a complete draft validates (C6)"
    );

    // No default floor → rejected.
    let mut no_floor = ThemeDraft::new_theme();
    no_floor.set_display_name("Hive".to_owned());
    no_floor.toggle_terrain(slab);
    assert_eq!(
        validate_for_save(&no_floor),
        Err(SaveThemeError::DefaultFloorNotInTerrain),
        "a theme with no default floor is rejected (C6)",
    );

    // A default floor that is NOT in the palette → rejected (the setter ignored it, so the floor
    // stays unset — proving the C6 fail-closed guard).
    let mut stray = ThemeDraft::new_theme();
    stray.set_display_name("Hive".to_owned());
    stray.toggle_terrain(slab);
    stray.set_default_floor(wall); // wall is NOT in the palette — ignored by the setter
    assert_eq!(
        stray.default_floor(),
        None,
        "setting a default floor outside the palette is ignored (C6 fail-closed)",
    );
    assert_eq!(
        validate_for_save(&stray),
        Err(SaveThemeError::DefaultFloorNotInTerrain),
        "a theme whose default floor is not in its terrain is rejected (C6)",
    );

    // An empty palette → rejected.
    let mut empty = ThemeDraft::new_theme();
    empty.set_display_name("Hive".to_owned());
    assert_eq!(
        validate_for_save(&empty),
        Err(SaveThemeError::NoTerrain),
        "a theme with no terrain is rejected (C6)",
    );

    // An empty name → rejected.
    let mut nameless = ThemeDraft::new_theme();
    nameless.toggle_terrain(slab);
    nameless.set_default_floor(slab);
    assert_eq!(
        validate_for_save(&nameless),
        Err(SaveThemeError::EmptyName),
        "a theme with no name is rejected",
    );
}

/// C6 — `floor_candidates` offers the theme's own terrain with the SLAB kind FIRST (Slab = the
/// walkable floor; `Floor` was retired in GTW-476). With a Wall selected before a Slab, the Slab
/// still leads the list.
#[test]
fn floor_candidates_prefer_slab() {
    let (registry, wall, slab) = fixture();
    let mut draft = ThemeDraft::new_theme();
    // Select the WALL first, then the SLAB — insertion order is wall-then-slab.
    draft.toggle_terrain(wall);
    draft.toggle_terrain(slab);

    let candidates = floor_candidates(&draft, &registry);
    assert_eq!(candidates.len(), 2, "both selected terrain are candidates");
    assert_eq!(
        candidates.first().map(|(key, _)| *key),
        Some(slab),
        "the Slab-kind terrain is offered FIRST even though it was selected second (C6 — Slab \
         preferred as the walkable floor)",
    );
    assert_eq!(
        candidates.get(1).map(|(key, _)| *key),
        Some(wall),
        "the non-Slab terrain follows the Slab in the candidate list (C6)",
    );
}

/// GTW-530 C3 — `slab_floor_candidates` offers ONLY Slab-kind terrain for the default-floor
/// picker: given a draft selecting a mix of Slab / Wall / Cover terrain, EVERY returned candidate
/// resolves to a Slab and NO Wall / Cover appears. The single-source-of-truth proof that the
/// picker can never offer a non-floor kind as the default floor. Pin-discriminating: it asserts
/// both the exact surviving key set (the two slabs, neither the wall nor the cover) and that each
/// survivor is Slab-kind in the registry.
#[test]
fn slab_floor_candidates_are_slab_only() {
    // A four-terrain registry mixing all three kinds: two Slabs, one Wall, one Cover.
    let slab_a = terrain_key(10);
    let slab_b = terrain_key(11);
    let wall = terrain_key(12);
    let cover = terrain_key(13);
    let registry = TerrainDefRegistry::new([
        (slab_a, slab_def(slab_a, "Rockcrete Floor")),
        (slab_b, slab_def(slab_b, "Grate Floor")),
        (wall, wall_def(wall, "Tunnel Wall")),
        (cover, cover_def(cover, "Ammo Crate")),
    ]);

    // Select ALL four into the draft (interleaved so ordering can't accidentally exclude a kind).
    let mut draft = ThemeDraft::new_theme();
    draft.toggle_terrain(wall);
    draft.toggle_terrain(slab_a);
    draft.toggle_terrain(cover);
    draft.toggle_terrain(slab_b);

    let candidates = slab_floor_candidates(&draft, &registry);
    let keys: Vec<TerrainUuid> = candidates.iter().map(|(key, _)| *key).collect();

    assert_eq!(
        candidates.len(),
        2,
        "only the two Slab terrain survive the Slab-only filter (C3); the Wall + Cover are dropped",
    );
    assert!(
        keys.contains(&slab_a) && keys.contains(&slab_b),
        "both Slab terrain are offered as default-floor candidates (C3): {keys:?}",
    );
    assert!(
        !keys.contains(&wall),
        "the Wall terrain is NOT offered as a default-floor candidate (C3 — Slab only): {keys:?}",
    );
    assert!(
        !keys.contains(&cover),
        "the Cover terrain is NOT offered as a default-floor candidate (C3 — Slab only): {keys:?}",
    );
    // Every survivor is genuinely Slab-kind in the registry (the filter dropped by KIND, not by
    // accident of key ordering).
    for key in &keys {
        let Some(def) = registry.def(key) else {
            unreachable!("a candidate key must resolve in the fixture registry")
        };
        assert!(
            matches!(def.sim_kind, TerrainSimKind::Slab { .. }),
            "every Slab-only candidate resolves to a Slab-kind terrain (C3): {:?}",
            def.sim_kind,
        );
    }
}

/// C3 — `resolved_stats` resolves a terrain UUID against the `TerrainDefRegistry` (the
/// single-source-of-truth proof): it returns the resolved def's kind + HP + armor and a non-empty
/// HP fraction. Pin-discriminating on the resolved KIND + a non-zero fill.
#[test]
fn resolved_stats_resolves_against_registry() {
    let (registry, _wall, slab) = fixture();
    let Some(def) = registry.def(&slab) else {
        unreachable!("the slab def must be in the fixture registry")
    };
    let (summary, fraction) = resolved_stats(def);
    assert!(
        summary.contains("Slab"),
        "the resolved readout names the resolved sim kind (Slab) — C3: {summary}",
    );
    assert!(
        summary.contains("Rockcrete Floor"),
        "the resolved readout names the resolved def — C3: {summary}",
    );
    assert!(
        fraction > 0.0,
        "a non-zero HP pool resolves to a non-zero bar fill (C3): {fraction}",
    );
}
