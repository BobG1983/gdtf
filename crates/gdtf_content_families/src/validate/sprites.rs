//! GTW-663: the **terrain → sprite-def** edge — every terrain def's
//! presenter-half `graphic_name` is a FOREIGN KEY by name into the
//! [`SpriteDefRegistry`] (the GTW-600 ruling), so a key that resolves no
//! `content/sprites/<name>.spritedef.ron` member is reported at load AND
//! authoring time. Both families load in both hosts, so both register this
//! check (GTW-630).

use bevy::prelude::{Res, ResMut};
use gdtf_assets::{
    ContentFinding, ContentIntegrityReport, FindingFamily, FindingReferrer, FindingTarget,
    ReferenceKeyScheme,
};
use gdtf_battle_sim::terrain::def::{TerrainDefRegistry, TerrainPresenterKind};

use crate::sprites::{SpriteDefRegistry, SpriteName};

/// `Check`: every terrain def's `graphic_name` (every
/// [`TerrainPresenterKind`] variant carries one) resolves in the
/// [`SpriteDefRegistry`]. Green on shipped content because GTW-663 seeded a
/// def for every role-table name; the renderer keeps drawing through the role
/// table until GTW-665, so a dangling key today is an authoring mistake
/// surfaced early, not a draw failure.
///
/// Plain `Res` params by contract: the registering HOST's `Check`-set window
/// condition must have verified them present (`bevy-traps.md` #1, guarded
/// once at the host's set — see the [module doc](super)).
pub fn check_terrain_graphic_refs(
    terrain: Res<TerrainDefRegistry>,
    sprites: Res<SpriteDefRegistry>,
    mut report: ResMut<ContentIntegrityReport>,
) {
    for (key, def) in terrain.defs() {
        let graphic_name = match &def.presenter_kind {
            TerrainPresenterKind::Wall { graphic_name }
            | TerrainPresenterKind::Cover { graphic_name }
            | TerrainPresenterKind::Emplacement { graphic_name }
            | TerrainPresenterKind::Slab { graphic_name, .. } => graphic_name,
        };
        if !sprites.contains(&SpriteName::new((**graphic_name).clone())) {
            report.record(ContentFinding::DanglingRef {
                referrer: FindingReferrer::new(format!(
                    "terrain def `{}` ({}) graphic_name",
                    *def.display_name, **key,
                )),
                target:   FindingTarget::new((**graphic_name).clone()),
                family:   FindingFamily::new("SpriteDefRegistry".to_owned()),
                scheme:   ReferenceKeyScheme::FileStem,
            });
        }
    }
}
