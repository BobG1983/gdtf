//! The registry every test fixture hands out, holding all named test terrain defs.

use super::{
    super::situation::test_pieces,
    defs::{
        test_cover, test_emplacement, test_emplacement_leaving_wall, test_floor,
        test_low_vision_cover, test_path_slab, test_slab, test_vision_slab, test_wall,
    },
    view_defs::{
        test_door, test_facing_wall, test_rubble_cover, test_stair, test_successor_floor,
        test_unresolvable,
    },
};
use crate::terrain::def::TerrainDefRegistry;

/// Registry of all named test terrain pieces.
#[must_use]
pub fn test_terrain_registry() -> TerrainDefRegistry {
    TerrainDefRegistry::new([
        (test_pieces::WALL, test_wall()),
        (test_pieces::SLAB, test_slab()),
        (test_pieces::COVER, test_cover()),
        (test_pieces::FLOOR, test_floor()),
        (test_pieces::VISION_SLAB, test_vision_slab()),
        (test_pieces::PATH_SLAB, test_path_slab()),
        (test_pieces::LOW_VISION_COVER, test_low_vision_cover()),
        (test_pieces::EMPLACEMENT, test_emplacement()),
        (
            test_pieces::EMPLACEMENT_LEAVING_WALL,
            test_emplacement_leaving_wall(),
        ),
        (test_pieces::DOOR, test_door()),
        (test_pieces::FACING_WALL, test_facing_wall()),
        (test_pieces::STAIR, test_stair()),
        (test_pieces::RUBBLE_COVER, test_rubble_cover()),
        (test_pieces::SUCCESSOR_FLOOR, test_successor_floor()),
        (test_pieces::UNRESOLVABLE, test_unresolvable()),
    ])
}

#[cfg(test)]
mod test {
    use super::test_terrain_registry;
    use crate::{
        terrain::def::{TerrainSimKind, owed_views},
        test_support::{registries::test_weapon_registry, situation::test_pieces},
        weapon::WeaponName,
    };

    #[test]
    fn every_test_emplacement_names_a_weapon_the_test_registry_holds() {
        let registry = test_terrain_registry();
        let mounts: Vec<&WeaponName> = registry
            .defs()
            .filter_map(|(_, def)| match &def.sim_kind {
                TerrainSimKind::Emplacement { mounted_weapon, .. } => Some(mounted_weapon),
                TerrainSimKind::Wall { .. }
                | TerrainSimKind::Cover { .. }
                | TerrainSimKind::Slab { .. } => None,
            })
            .collect();
        assert!(
            !mounts.is_empty(),
            "test_terrain_registry must hold at least one Emplacement def, or the mounted-weapon \
             check below passes without checking anything",
        );

        let weapons = test_weapon_registry();
        for key in mounts {
            assert!(
                weapons.spec(key).is_some(),
                "test_weapon_registry must resolve `{}`, the mounted weapon a test emplacement \
                 def names — the two registries are handed out as a pair",
                key.as_str(),
            );
        }
    }

    #[test]
    fn the_view_authoring_defs_answer_every_view_they_owe() {
        let registry = test_terrain_registry();
        let authored = [
            test_pieces::DOOR,
            test_pieces::FACING_WALL,
            test_pieces::STAIR,
            test_pieces::SLAB,
            test_pieces::WALL,
            test_pieces::COVER,
            test_pieces::FLOOR,
            test_pieces::EMPLACEMENT,
            test_pieces::EMPLACEMENT_LEAVING_WALL,
            test_pieces::RUBBLE_COVER,
            test_pieces::SUCCESSOR_FLOOR,
            test_pieces::UNRESOLVABLE,
        ];
        for key in authored {
            let Some(def) = registry.def(&key) else {
                unreachable!("test_terrain_registry holds every def named here");
            };
            let missing: Vec<String> = owed_views(def)
                .iter()
                .filter(|view| def.views.sprite(**view).is_none())
                .map(|view| format!("{view:?}"))
                .collect();
            assert!(
                missing.is_empty(),
                "`{}` must author every view owed_views names for it, or a resolver test \
                 measures a missing row rather than the rule it pins — missing {missing:?}",
                *def.display_name,
            );
        }
    }
}
