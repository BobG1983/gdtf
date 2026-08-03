//! GTW-582: the authored **situation's** outbound reference edges — gang/member
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

pub(super) fn check_situation_gang_refs(
    situation: Res<LoadedSituation>,
    gangs: Res<GangRegistry>,
    mut report: ResMut<ContentIntegrityReport>,
) {
    let combatant_refs = situation
        .gangers
        .iter()
        .map(|placed| (&placed.gang, &placed.member))
        .chain(
            situation
                .rosters
                .iter()
                .map(|member| (&member.gang, &member.member)),
        );
    for (gang, member) in combatant_refs {
        let Some(roster) = gangs.roster(gang) else {
            report.record(ContentFinding::DanglingRef {
                referrer: FindingReferrer::new(format!(
                    "{SITUATION_RON_PATH}: situation ganger `{}`",
                    **member,
                )),
                target:   FindingTarget::new((**gang).clone()),
                family:   FindingFamily::new("GangRegistry".to_owned()),
                scheme:   ReferenceKeyScheme::FileStem,
            });
            continue;
        };
        if roster.member(member).is_none() {
            report.record(ContentFinding::DanglingRef {
                referrer: FindingReferrer::new(format!(
                    "{SITUATION_RON_PATH}: situation ganger of gang `{}`",
                    **gang,
                )),
                target:   FindingTarget::new((**member).clone()),
                family:   FindingFamily::new(format!("GangRegistry roster `{}`", **gang)),
                scheme:   ReferenceKeyScheme::DisplayName,
            });
        }
    }
}

/// documented `#[serde(default)]`), so it is skipped — a nil theme is an
pub(super) fn check_situation_theme_ref(
    situation: Res<LoadedSituation>,
    themes: Res<UuidThemeRegistry>,
    mut report: ResMut<ContentIntegrityReport>,
) {
    if *situation.theme.is_nil() {
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
    if !*situation.default_floor.is_nil() && terrain.def(&situation.default_floor).is_none() {
        report.record(ContentFinding::DanglingRef {
            referrer: FindingReferrer::new(format!("{SITUATION_RON_PATH}: default_floor")),
            target:   FindingTarget::new(situation.default_floor.to_string()),
            family:   FindingFamily::new("TerrainDefRegistry".to_owned()),
            scheme:   ReferenceKeyScheme::Uuid,
        });
    }
}

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
