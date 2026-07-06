//! GTW-582: the **terrain/theme defs'** outbound reference edges — a theme's
//! `default_floor` + palette UUIDs (the nil-sentinel `default_floor` fallback's
//! root cause, C3(d)) and an emplacement def's mounted-weapon key (found on the
//! C1 verify-first walk; a dangling mount leaves the emplacement weaponless at
//! setup). Both edges span only families the content editor loads too, so the
//! editor registers both (GTW-630).

use bevy::prelude::{Res, ResMut};
use gdtf_assets::{
    ContentFinding, ContentIntegrityReport, FindingFamily, FindingReferrer, FindingTarget,
    ReferenceKeyScheme,
};
use gdtf_battle_sim::{
    level::UuidThemeRegistry,
    terrain::def::{TerrainDefRegistry, TerrainSimKind},
    weapon::WeaponRegistry,
};

/// `Check`: every theme def's `default_floor` and terrain-palette UUIDs resolve
/// in the [`TerrainDefRegistry`]. A dangling `default_floor` is what the
/// procgen emit degrades to the nil sentinel over (C3(d)) — reported HERE at
/// its authoring root, and again at generation time if the pour actually
/// degrades.
///
/// Plain `Res` params by contract: the registering HOST's `Check`-set window
/// condition must have verified them present (`bevy-traps.md` #1, guarded once
/// at the host's set — see the [module doc](super)).
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

/// `Check`: every EMPLACEMENT terrain def's `mounted_weapon` key resolves in
/// the [`WeaponRegistry`] — the terrain → weapon edge the C1 walk surfaced
/// (`TerrainSimKind::Emplacement` mounts a ranged weapon by file stem,
/// GTW-543).
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
