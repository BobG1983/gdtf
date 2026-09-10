//! Merge generated terrain with the authored situation's theme, size and fields.
use bevy::prelude::warn;
use gdtf_assets::{ContentFinding, FindingDetail, FindingReferrer};
use gdtf_battle_sim::{
    procgen::{EmittedLevel, PackingError, ProcgenFinding},
    situation::{BattleMap, PlacedGanger, Situation, SituationCombatants},
};

pub(crate) struct ProcgenOutcome {
    pub situation:        Situation,
    pub placements:       Vec<PlacedGanger>,
    pub findings:         Vec<ContentFinding>,
    pub deployment_error: Option<PackingError>,
}

#[must_use]
pub(crate) fn outcome_from_emitted(authored: Situation, emitted: EmittedLevel) -> ProcgenOutcome {
    let findings = emitted
        .findings
        .iter()
        .map(|finding| convert_procgen_finding(*finding))
        .collect();
    let situation = Situation {
        map:        merge_procgen_terrain(authored.map, emitted.map),
        combatants: SituationCombatants {
            rosters:        Vec::new(),
            player_faction: authored.combatants.player_faction,
        },
    };
    ProcgenOutcome {
        situation,
        placements: Vec::new(),
        findings,
        deployment_error: None,
    }
}

#[must_use]
pub(crate) fn outcome_from_packing_error(
    authored: Situation,
    err: &PackingError,
) -> ProcgenOutcome {
    warn!(
        "procgen could not assemble a level ({err}); using the authored situation's terrain \
         instead"
    );
    let finding = ContentFinding::DegradedFallback {
        context: FindingReferrer::new(format!(
            "procgen for theme {} (empty-board fallback)",
            *authored.map.theme,
        )),
        detail:  FindingDetail::new(format!(
            "could not assemble a level ({err}); the authored situation's terrain was used \
             instead"
        )),
    };
    ProcgenOutcome {
        situation:        authored,
        placements:       Vec::new(),
        findings:         vec![finding],
        deployment_error: None,
    }
}

fn convert_procgen_finding(finding: ProcgenFinding) -> ContentFinding {
    match finding {
        ProcgenFinding::MissingThemeDefaultFloor { theme } => {
            warn!(
                "procgen: theme {} resolves no default floor in the UuidThemeRegistry; the \
                 level was poured with the nil-sentinel floor (playable but degraded)",
                *theme,
            );
            ContentFinding::DegradedFallback {
                context: FindingReferrer::new(format!("procgen for theme {}", *theme)),
                detail:  FindingDetail::new(
                    "the theme resolves no default floor; the level was poured with the \
                     nil-sentinel floor"
                        .to_owned(),
                ),
            }
        }
        ProcgenFinding::UnresolvedTerrainPiece { piece } => {
            warn!(
                "procgen: placed terrain piece {} resolves no TerrainDef; poured fail-open \
                 into the walls list (it will surface as TerrainNotFound at setup)",
                *piece,
            );
            ContentFinding::DegradedFallback {
                context: FindingReferrer::new(format!("procgen placed terrain piece {}", *piece)),
                detail:  FindingDetail::new(
                    "the piece resolves no TerrainDef; poured fail-open into the walls list \
                     (it will surface as TerrainNotFound at setup)"
                        .to_owned(),
                ),
            }
        }
    }
}

fn merge_procgen_terrain(authored: BattleMap, generated: BattleMap) -> BattleMap {
    BattleMap {
        fields:         authored.fields,
        theme:          authored.theme,
        grid_size:      authored.grid_size,
        default_floor:  generated.default_floor,
        walls:          generated.walls,
        scatter:        generated.scatter,
        slabs:          generated.slabs,
        floors:         generated.floors,
        vertical_links: generated.vertical_links,
    }
}
