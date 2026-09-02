//! Cross-check theme and emplacement references against terrain and weapon registries.

use bevy::prelude::{Res, ResMut};
use gdtf_assets::{
    ContentFinding, ContentIntegrityReport, FindingFamily, FindingReferrer, FindingTarget,
    ReferenceKeyScheme,
};
use gdtf_battle_sim::{
    level::UuidThemeRegistry,
    terrain::def::{LeavesBehind, TerrainDefRegistry, TerrainSimKind},
    weapon::WeaponRegistry,
};

use crate::sprites::{SpriteDefRegistry, SpriteName};

/// Record dangling theme → terrain UUID references.
pub fn check_theme_terrain_refs(
    themes: Res<UuidThemeRegistry>,
    terrain: Res<TerrainDefRegistry>,
    mut report: ResMut<ContentIntegrityReport>,
) {
    for (key, def) in themes.defs() {
        let referrer = |field: &str| {
            FindingReferrer::new(format!("theme `{}` ({}) {field}", *def.display_name, **key))
        };
        if terrain.def(&def.default_floor).is_none() {
            report.record(ContentFinding::DanglingRef {
                referrer: referrer("default_floor"),
                target:   FindingTarget::new(def.default_floor.to_string()),
                family:   FindingFamily::new("TerrainDefRegistry".to_owned()),
                scheme:   ReferenceKeyScheme::Uuid,
            });
        }
        for piece in &def.terrain {
            if terrain.def(piece).is_none() {
                report.record(ContentFinding::DanglingRef {
                    referrer: referrer("terrain palette"),
                    target:   FindingTarget::new(piece.to_string()),
                    family:   FindingFamily::new("TerrainDefRegistry".to_owned()),
                    scheme:   ReferenceKeyScheme::Uuid,
                });
            }
        }
    }
}

/// Record dangling `leaves_behind` → terrain-def and sprite-def references.
pub fn check_terrain_leaves_behind_refs(
    terrain: Res<TerrainDefRegistry>,
    sprites: Res<SpriteDefRegistry>,
    mut report: ResMut<ContentIntegrityReport>,
) {
    for (key, def) in terrain.defs() {
        let referrer = || {
            FindingReferrer::new(format!(
                "terrain def `{}` ({}) leaves_behind",
                *def.display_name, **key,
            ))
        };
        match &def.leaves_behind {
            LeavesBehind::Nothing => {}
            LeavesBehind::Piece(successor) => {
                if terrain.def(successor).is_none() {
                    report.record(ContentFinding::DanglingRef {
                        referrer: referrer(),
                        target:   FindingTarget::new(successor.to_string()),
                        family:   FindingFamily::new("TerrainDefRegistry".to_owned()),
                        scheme:   ReferenceKeyScheme::Uuid,
                    });
                }
            }
            LeavesBehind::Sprite(graphic) => {
                if !sprites.contains(&SpriteName::new((**graphic).clone())) {
                    report.record(ContentFinding::DanglingRef {
                        referrer: referrer(),
                        target:   FindingTarget::new((**graphic).clone()),
                        family:   FindingFamily::new("SpriteDefRegistry".to_owned()),
                        scheme:   ReferenceKeyScheme::FileStem,
                    });
                }
            }
        }
    }
}

/// Record dangling emplacement mounted-weapon references.
pub fn check_emplacement_weapon_refs(
    terrain: Res<TerrainDefRegistry>,
    weapons: Res<WeaponRegistry>,
    mut report: ResMut<ContentIntegrityReport>,
) {
    for (key, def) in terrain.defs() {
        if let TerrainSimKind::Emplacement { mounted_weapon, .. } = &def.sim_kind
            && weapons.spec(mounted_weapon).is_none()
        {
            report.record(ContentFinding::DanglingRef {
                referrer: FindingReferrer::new(format!(
                    "terrain def `{}` emplacement mounted_weapon",
                    **key,
                )),
                target:   FindingTarget::new((**mounted_weapon).clone()),
                family:   FindingFamily::new("WeaponRegistry".to_owned()),
                scheme:   ReferenceKeyScheme::FileStem,
            });
        }
    }
}
