//! Situation outbound reference edges — gang, theme, terrain, and field keys.

use bevy::prelude::{Res, ResMut};
use gdtf_assets::{
    ContentFinding, ContentIntegrityReport, ContentMemberKey, FindingFamily, FindingReferrer,
    FindingTarget, ReferenceField, ReferenceKeyScheme, ReferringRecord,
};
use gdtf_battle_sim::{
    effects::fields::FieldDefRegistry,
    ganger::GangRegistry,
    level::UuidThemeRegistry,
    terrain::def::{TerrainDefRegistry, TerrainUuid},
};

use crate::situation::{LoadedSituation, SITUATION_RON_PATH};

// The situation loads by hardcoded path, so its referring record is that path.
fn situation_record(field: &str) -> ReferringRecord {
    ReferringRecord::new(
        FindingFamily::new("LoadedSituation".to_owned()),
        ContentMemberKey::new(SITUATION_RON_PATH.to_owned()),
        ReferenceField::new(field.to_owned()),
    )
}

/// Record dangling situation → gang name and situation → roster member references.
pub fn check_situation_gang_refs(
    situation: Res<LoadedSituation>,
    gangs: Res<GangRegistry>,
    mut report: ResMut<ContentIntegrityReport>,
) {
    let combatant_refs = situation
        .combatants
        .rosters
        .iter()
        .map(|member| (&member.gang, &member.member));
    for (gang, member) in combatant_refs {
        let Some(roster) = gangs.roster(gang) else {
            report.record(ContentFinding::DanglingRef {
                referrer:         FindingReferrer::new(format!(
                    "{SITUATION_RON_PATH}: situation ganger `{}`",
                    **member,
                )),
                referring_record: situation_record("rosters[].gang"),
                target:           FindingTarget::new((**gang).clone()),
                family:           FindingFamily::new("GangRegistry".to_owned()),
                scheme:           ReferenceKeyScheme::FileStem,
            });
            continue;
        };
        if roster.member(member).is_none() {
            report.record(ContentFinding::DanglingRef {
                referrer:         FindingReferrer::new(format!(
                    "{SITUATION_RON_PATH}: situation ganger of gang `{}`",
                    **gang,
                )),
                referring_record: situation_record("rosters[].member"),
                target:           FindingTarget::new((**member).clone()),
                family:           FindingFamily::new(format!("GangRegistry roster `{}`", **gang)),
                scheme:           ReferenceKeyScheme::DisplayName,
            });
        }
    }
}

/// Record a dangling situation → theme UUID reference.
///
/// Nil theme is allowed (serde default); skip when unset.
pub fn check_situation_theme_ref(
    situation: Res<LoadedSituation>,
    themes: Res<UuidThemeRegistry>,
    mut report: ResMut<ContentIntegrityReport>,
) {
    let theme = situation.map.theme;
    if *theme.is_nil() {
        return;
    }
    if themes.def(&theme).is_none() {
        report.record(ContentFinding::DanglingRef {
            referrer:         FindingReferrer::new(format!("{SITUATION_RON_PATH}: theme")),
            referring_record: situation_record("theme"),
            target:           FindingTarget::new(theme.to_string()),
            family:           FindingFamily::new("UuidThemeRegistry".to_owned()),
            scheme:           ReferenceKeyScheme::Uuid,
        });
    }
}

/// Record dangling situation → placed terrain UUID and default-floor references.
pub fn check_situation_terrain_refs(
    situation: Res<LoadedSituation>,
    terrain: Res<TerrainDefRegistry>,
    mut report: ResMut<ContentIntegrityReport>,
) {
    let mut seen: Vec<TerrainUuid> = Vec::new();
    let map = &situation.map;
    let pieces = map
        .walls
        .iter()
        .map(|spawn| ("walls", spawn.piece))
        .chain(map.scatter.iter().map(|spawn| ("scatter", spawn.piece)))
        .chain(map.slabs.iter().map(|spawn| ("slabs", spawn.piece)))
        .chain(map.floors.iter().map(|spawn| ("floors", spawn.piece)));
    for (list, piece) in pieces {
        if seen.contains(&piece) {
            continue;
        }
        seen.push(piece);
        if terrain.def(&piece).is_none() {
            report.record(ContentFinding::DanglingRef {
                referrer:         FindingReferrer::new(format!("{SITUATION_RON_PATH}: {list}")),
                referring_record: situation_record(&format!("{list}[].piece")),
                target:           FindingTarget::new(piece.to_string()),
                family:           FindingFamily::new("TerrainDefRegistry".to_owned()),
                scheme:           ReferenceKeyScheme::Uuid,
            });
        }
    }
    if !*map.default_floor.is_nil() && terrain.def(&map.default_floor).is_none() {
        report.record(ContentFinding::DanglingRef {
            referrer:         FindingReferrer::new(format!("{SITUATION_RON_PATH}: default_floor")),
            referring_record: situation_record("default_floor"),
            target:           FindingTarget::new(map.default_floor.to_string()),
            family:           FindingFamily::new("TerrainDefRegistry".to_owned()),
            scheme:           ReferenceKeyScheme::Uuid,
        });
    }
}

/// Record dangling situation → field def key references.
pub fn check_situation_field_refs(
    situation: Res<LoadedSituation>,
    fields: Res<FieldDefRegistry>,
    mut report: ResMut<ContentIntegrityReport>,
) {
    for spawn in &situation.map.fields {
        if fields.def(&spawn.field).is_none() {
            report.record(ContentFinding::DanglingRef {
                referrer:         FindingReferrer::new(format!("{SITUATION_RON_PATH}: fields")),
                referring_record: situation_record("fields[].field"),
                target:           FindingTarget::new((*spawn.field).clone()),
                family:           FindingFamily::new("FieldDefRegistry".to_owned()),
                scheme:           ReferenceKeyScheme::FileStem,
            });
        }
    }
}
