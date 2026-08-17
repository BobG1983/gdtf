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

use super::{autoload::resolve_autoload, load_theme_into_form};
use crate::theme_form::ThemeDraft;


fn theme_key(n: u128) -> ThemeUuid {
    ThemeUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0001_84c3_0000 + n))
}

fn terrain_key(n: u128) -> TerrainUuid {
    TerrainUuid::new(bevy::asset::uuid::Uuid::from_u128(0x0001_84c3_1000 + n))
}

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

        blocks_pathing: None,
        blocks_los: None,
    }
}

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

        blocks_pathing: None,
        blocks_los: None,
    }
}

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

#[test]
fn resolve_autoload_absent_key_returns_none() {
    let slab = terrain_key(2);
    let registered = theme_key(2);
    let absent = theme_key(99);
    let registry = single_theme_registry(registered, vec![slab], slab);

    let result = resolve_autoload(absent, &registry);
    assert!(
        result.is_none(),
        "resolve_autoload for a non-nil key absent from the registry must return None",
    );
}

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

#[test]
fn load_theme_into_form_replaces_draft_with_def_parts() {
    let slab = terrain_key(5);
    let wall = terrain_key(6);
    let t_key = theme_key(4);

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

    let mut draft = ThemeDraft::new_theme();
    load_theme_into_form(&mut draft, &original);

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

    let reloaded = ron::de::from_str::<UuidThemeDef>(&ron_text);
    assert!(
        reloaded.is_ok(),
        "the serialized def must round-trip through the UuidThemeDef deserializer (the \
          theme loader's parser): {:?}",
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
