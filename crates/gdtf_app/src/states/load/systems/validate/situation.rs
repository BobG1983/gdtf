//! GTW-582: the authored **situation's** outbound reference edges — gang/member
//! refs (the TWO gang-path key schemes), the theme UUID, the terrain UUIDs, and
//! the field keys. Each is ALSO still guarded abort-first at battle-request
//! time (`setup_battle`'s typed errors, C3(a) — the runtime guard stays); these
//! checks surface the same mistakes at `Load`, when they are cheap to fix.

use bevy::prelude::{Res, ResMut};
use gdtf_assets::{
    ContentFinding, ContentIntegrityReport, FindingFamily, FindingReferrer, FindingTarget,
    ReferenceKeyScheme,
};
use gdtf_battle_sim::{
    effects::fields::FieldDefRegistry, ganger::GangRegistry, level::UuidThemeRegistry,
    terrain::def::TerrainDefRegistry,
};

use crate::states::load::{plugin::SITUATION_RON_PATH, resources::LoadedSituation};

/// `Check`: every placed ganger's `gang` ref resolves in the [`GangRegistry`]
/// (by file STEM) and its `member` ref resolves in that gang's roster (by
/// roster DISPLAY-NAME) — the two coexisting key schemes on the gang path, each
/// finding naming WHICH scheme failed (GTW-582 C2).
///
/// Plain `Res` params are safe here: the `Check` set's window condition
/// (`reference_graph_ready`) verified every one of them present (bevy-traps #1,
/// guarded once at the set).
pub(super) fn check_situation_gang_refs(
    situation: Res<LoadedSituation>,
    gangs: Res<GangRegistry>,
    mut report: ResMut<ContentIntegrityReport>,
) {
    for placed in &situation.gangers {
        let Some(roster) = gangs.roster(&placed.gang) else {
            report.record(ContentFinding::DanglingRef {
                referrer: FindingReferrer::new(format!(
                    "{SITUATION_RON_PATH}: placed ganger `{}`",
                    *placed.member,
                )),
                target:   FindingTarget::new((*placed.gang).clone()),
                family:   FindingFamily::new("GangRegistry".to_owned()),
                scheme:   ReferenceKeyScheme::FileStem,
            });
            continue;
        };
        if roster.member(&placed.member).is_none() {
            report.record(ContentFinding::DanglingRef {
                referrer: FindingReferrer::new(format!(
                    "{SITUATION_RON_PATH}: placed ganger of gang `{}`",
                    *placed.gang,
                )),
                target:   FindingTarget::new((*placed.member).clone()),
                family:   FindingFamily::new(format!("GangRegistry roster `{}`", *placed.gang)),
                scheme:   ReferenceKeyScheme::DisplayName,
            });
        }
    }
}

/// `Check`: the situation's authored `theme` UUID resolves in the
/// [`UuidThemeRegistry`]. The NIL sentinel means "no theme authored" (the
/// documented `#[serde(default)]`), so it is skipped — a nil theme is an
/// authoring CHOICE, not a dangling reference.
pub(super) fn check_situation_theme_ref(
    situation: Res<LoadedSituation>,
    themes: Res<UuidThemeRegistry>,
    mut report: ResMut<ContentIntegrityReport>,
) {
    if situation.theme.is_nil() {
        return;
    }
    if themes.def(&situation.theme).is_none() {
        report.record(ContentFinding::DanglingRef {
            referrer: FindingReferrer::new(format!("{SITUATION_RON_PATH}: theme")),
            target:   FindingTarget::new(situation.theme.to_string()),
            family:   FindingFamily::new("UuidThemeRegistry".to_owned()),
            scheme:   ReferenceKeyScheme::Uuid,
        });
    }
}

/// `Check`: every terrain UUID the situation authors (walls / scatter / slabs /
/// floor overrides / the non-nil `default_floor`) resolves in the
/// [`TerrainDefRegistry`]. Deduplicated per UUID (a wall piece repeats per
/// cell; one dangling def is ONE finding).
pub(super) fn check_situation_terrain_refs(
    situation: Res<LoadedSituation>,
    terrain: Res<TerrainDefRegistry>,
    mut report: ResMut<ContentIntegrityReport>,
) {
    let mut seen: Vec<gdtf_battle_sim::terrain::def::TerrainUuid> = Vec::new();
    let pieces = situation
        .walls
        .iter()
        .map(|spawn| ("walls", spawn.piece))
        .chain(
            situation
                .scatter
                .iter()
                .map(|spawn| ("scatter", spawn.piece)),
        )
        .chain(situation.slabs.iter().map(|spawn| ("slabs", spawn.piece)))
        .chain(situation.floors.iter().map(|spawn| ("floors", spawn.piece)));
    for (list, piece) in pieces {
        if seen.contains(&piece) {
            continue;
        }
        seen.push(piece);
        if terrain.def(&piece).is_none() {
            report.record(ContentFinding::DanglingRef {
                referrer: FindingReferrer::new(format!("{SITUATION_RON_PATH}: {list}")),
                target:   FindingTarget::new(piece.to_string()),
                family:   FindingFamily::new("TerrainDefRegistry".to_owned()),
                scheme:   ReferenceKeyScheme::Uuid,
            });
        }
    }
    // The level-wide default floor (nil = "not authored", the documented default).
    if !situation.default_floor.is_nil() && terrain.def(&situation.default_floor).is_none() {
        report.record(ContentFinding::DanglingRef {
            referrer: FindingReferrer::new(format!("{SITUATION_RON_PATH}: default_floor")),
            target:   FindingTarget::new(situation.default_floor.to_string()),
            family:   FindingFamily::new("TerrainDefRegistry".to_owned()),
            scheme:   ReferenceKeyScheme::Uuid,
        });
    }
}

/// `Check`: every authored field placement's key resolves in the
/// [`FieldDefRegistry`] catalog.
pub(super) fn check_situation_field_refs(
    situation: Res<LoadedSituation>,
    fields: Res<FieldDefRegistry>,
    mut report: ResMut<ContentIntegrityReport>,
) {
    for spawn in &situation.fields {
        if fields.def(&spawn.field).is_none() {
            report.record(ContentFinding::DanglingRef {
                referrer: FindingReferrer::new(format!("{SITUATION_RON_PATH}: fields")),
                target:   FindingTarget::new((*spawn.field).clone()),
                family:   FindingFamily::new("FieldDefRegistry".to_owned()),
                scheme:   ReferenceKeyScheme::FileStem,
            });
        }
    }
}
