//! C3.5 — egui-layer pure-function tests for [`resolve_autoload`] and [`load_theme_into_form`].
//!
//! These cover the NEW egui-layer fns that have no equivalent in `theme_form/tests.rs` (which
//! covers the model-layer: projection, round-trip, validation, resolution, floor-candidates).
//! The round-trip identity test is NOT duplicated — it is already covered by
//! `theme_def_round_trips_through_the_loader_parser` in `crate::theme_form::tests`.

use gdtf_battle_sim::{
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverHp, HeightBand},
    level::{ThemeDisplayName, ThemeUuid, UuidThemeDef, UuidThemeRegistry},
    slab::SlabHp,
    terrain::{
        def::{
            TerrainDef, TerrainDefRegistry, TerrainDisplayName, TerrainPresenterKind,
            TerrainSimKind, TerrainUuid,
        },
        piece::TerrainGraphicKey,
    },
};

use super::{load_theme_into_form, resolve_autoload};
use crate::theme_form::ThemeDraft;

/// A deterministic [`ThemeUuid`] from a small integer — no random UUIDs in tests.
fn theme_key(n: u128) -> ThemeUuid {
    ThemeUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0001_84c3_0000 + n))
}

/// A deterministic [`TerrainUuid`] from a small integer.
fn terrain_key(n: u128) -> TerrainUuid {
    TerrainUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0001_84c3_1000 + n))
}

/// A minimal Slab [`TerrainDef`] with the given key and display name — constructed with
/// real types, no stubs.
fn slab_def(key: TerrainUuid, name: &str) -> TerrainDef {
    TerrainDef {
        key,
        display_name: TerrainDisplayName::new(name.to_owned()),
        sim_kind: TerrainSimKind::Slab {
            hp:               SlabHp::new(100),
            armor_protection: ArmorProtection::new(4),
            armor_hardness:   ArmorHardness::new(2),
        },
        presenter_kind: TerrainPresenterKind::Slab {
            graphic_name: TerrainGraphicKey::new("slab".to_owned()),
            footfall:     None,
        },
        tags: Vec::new(),
        on_death: None,
    }
}

/// A minimal Wall [`TerrainDef`] with the given key and display name.
fn wall_def(key: TerrainUuid, name: &str) -> TerrainDef {
    TerrainDef {
        key,
        display_name: TerrainDisplayName::new(name.to_owned()),
        sim_kind: TerrainSimKind::Wall {
            hp:               CoverHp::new(60),
            armor_protection: ArmorProtection::new(8),
            armor_hardness:   ArmorHardness::new(4),
            height_band:      HeightBand::High,
        },
        presenter_kind: TerrainPresenterKind::Wall {
            graphic_name: TerrainGraphicKey::new("wall".to_owned()),
        },
        tags: Vec::new(),
        on_death: None,
    }
}

/// A one-theme [`UuidThemeRegistry`] with a fixed display name — gives tests a real registry
/// without touching the filesystem or any global state.
fn single_theme_registry(
    t_key: ThemeUuid,
    terrain: Vec<TerrainUuid>,
    floor: TerrainUuid,
) -> UuidThemeRegistry {
    let def = UuidThemeDef {
        key: t_key,
        display_name: ThemeDisplayName::new("Hive District".to_owned()),
        default_floor: floor,
        terrain,
    };
    UuidThemeRegistry::new([(t_key, def)])
}

// ── C3.5(a) resolve_autoload ──────────────────────────────────────────────────────────────────

/// C3.5(a) — the NIL sentinel returns [`None`] regardless of registry contents. The nil theme
/// signals "no theme selected"; loading it would clobber the in-progress draft with a blank.
#[test]
fn resolve_autoload_nil_returns_none() {
    let slab = terrain_key(1);
    let t_key = theme_key(1);
    let registry = single_theme_registry(t_key, vec![slab], slab);

    let result = resolve_autoload(ThemeUuid::nil(), &registry);
    assert!(
        result.is_none(),
        "resolve_autoload with the nil sentinel must return None \
         (no theme auto-loaded when nothing is selected)",
    );
}

/// C3.5(a) — a non-nil UUID absent from the registry returns [`None`] (the theme was removed
/// or not yet loaded). The caller must not clobber the draft with a missing def.
#[test]
fn resolve_autoload_absent_key_returns_none() {
    let slab = terrain_key(2);
    let registered = theme_key(2);
    let absent = theme_key(99); // intentionally not in the registry
    let registry = single_theme_registry(registered, vec![slab], slab);

    let result = resolve_autoload(absent, &registry);
    assert!(
        result.is_none(),
        "resolve_autoload for a non-nil key absent from the registry must return None",
    );
}

/// C3.5(a) — a non-nil UUID present in the registry returns `Some(&def)` for the correct
/// key. The returned reference is passed straight to `load_theme_into_form` (no double-lookup).
#[test]
fn resolve_autoload_present_key_returns_some_def() {
    let slab = terrain_key(3);
    let wall = terrain_key(4);
    let t_key = theme_key(3);
    let registry = single_theme_registry(t_key, vec![slab, wall], slab);

    let result = resolve_autoload(t_key, &registry);
    assert!(
        result.is_some(),
        "resolve_autoload for a non-nil key present in the registry must return Some(&def)",
    );
    let Some(def) = result else {
        // unreachable — the assert above guards this; avoids unwrap/expect per workspace lints
        return;
    };
    assert_eq!(
        def.key, t_key,
        "the returned def's key must equal the looked-up session theme key",
    );
    assert_eq!(
        &**def.display_name, "Hive District",
        "the returned def carries the correct display name from the registry",
    );
}

// ── C3.5(b) load_theme_into_form ─────────────────────────────────────────────────────────────

/// C3.5(b) — [`load_theme_into_form`] replaces the current draft so that key, display name,
/// terrain palette, and default floor all match the loaded [`UuidThemeDef`]. Verifies the
/// C3.2 "load-existing" affordance: after the call the form reflects the selected theme's live
/// definition, not the previous new-theme blank.
#[test]
fn load_theme_into_form_replaces_draft_with_def_parts() {
    let slab = terrain_key(5);
    let wall = terrain_key(6);
    let t_key = theme_key(4);

    // Confirm the terrain fixture is sound — the test asserts on the DRAFT after load, not on
    // the registry, but building a real TerrainDefRegistry proves the keys are valid.
    let terrain_reg = TerrainDefRegistry::new([
        (slab, slab_def(slab, "Rockcrete Floor")),
        (wall, wall_def(wall, "Tunnel Wall")),
    ]);
    assert!(
        terrain_reg.def(&slab).is_some(),
        "fixture: slab def must be in the terrain registry"
    );
    assert!(
        terrain_reg.def(&wall).is_some(),
        "fixture: wall def must be in the terrain registry"
    );

    let def = UuidThemeDef {
        key:           t_key,
        display_name:  ThemeDisplayName::new("Underhive Sprawl".to_owned()),
        default_floor: slab,
        terrain:       vec![slab, wall],
    };

    // Start from a blank new-theme draft to prove the fn REPLACES it.
    let mut draft = ThemeDraft::new_theme();
    load_theme_into_form(&mut draft, &def);

    assert_eq!(
        draft.key(),
        t_key,
        "after load_theme_into_form the draft's key must match the def's key (C3.2)",
    );
    assert_eq!(
        draft.display_name(),
        "Underhive Sprawl",
        "after load_theme_into_form the draft's display name must match the def's display name",
    );
    assert_eq!(
        draft.terrain(),
        &[slab, wall],
        "after load_theme_into_form the draft's terrain palette must equal the def's terrain \
         list in the same order (no reordering on load)",
    );
    assert_eq!(
        draft.default_floor(),
        Some(slab),
        "after load_theme_into_form the draft's default floor must match the def's default \
         floor (C3.2 / C6)",
    );
}

/// C3.5 round-trip — loads a [`UuidThemeDef`] into the form via [`load_theme_into_form`], then
/// projects + serializes via [`draft_to_theme_def`] / [`serialize_theme_def`] (the same path
/// the save button runs — C3.3), then parses the RON back through the GTW-487 loader's parser
/// (`ron::de::from_str::<UuidThemeDef>`) and asserts structural equality. No magnitudes are
/// pinned — only that the round-trip is identity (no field dropped or changed).
#[test]
fn load_then_save_round_trips_identical() {
    use gdtf_battle_sim::level::UuidThemeDef;

    use crate::theme_form::{draft_to_theme_def, serialize_theme_def};

    let slab = terrain_key(7);
    let wall = terrain_key(8);
    let t_key = theme_key(5);

    let original = UuidThemeDef {
        key:           t_key,
        display_name:  ThemeDisplayName::new("Ash Wastes Outpost".to_owned()),
        default_floor: slab,
        terrain:       vec![slab, wall],
    };

    // Load the def into a fresh draft (the C3.2 form-load path).
    let mut draft = ThemeDraft::new_theme();
    load_theme_into_form(&mut draft, &original);

    // Project + serialize (the C3.3 save path).
    let projected = draft_to_theme_def(&draft, t_key);
    let serialized = serialize_theme_def(&projected);
    assert!(
        serialized.is_ok(),
        "serialize_theme_def must succeed after load_theme_into_form: {:?}",
        serialized.as_ref().err(),
    );
    let Ok(ron_text) = serialized else {
        return;
    };

    // Parse back with the GTW-487 loader's deserializer.
    let reloaded = ron::de::from_str::<UuidThemeDef>(&ron_text);
    assert!(
        reloaded.is_ok(),
        "the serialized def must round-trip through the UuidThemeDef deserializer (the \
         GTW-487 theme loader's parser): {:?}",
        reloaded.as_ref().err(),
    );
    let Ok(reloaded) = reloaded else {
        return;
    };
    assert_eq!(
        reloaded, original,
        "the reloaded UuidThemeDef must be structurally equal to the original def after \
         load → project → serialize → parse (C3.5 round-trip identity)",
    );
}
