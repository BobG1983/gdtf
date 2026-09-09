//! deploy the authored roster members onto the generated map's deployment zones.
use bevy::prelude::warn;
use gdtf_assets::{ContentFinding, FindingDetail, FindingReferrer};
use gdtf_battle_sim::{
    procgen::{EmittedLevel, deploy_rosters},
    rng::BattleSeed,
    situation::Situation,
};

use super::procgen::{ProcgenOutcome, outcome_from_emitted};

#[must_use]
pub(crate) fn deploy_over_generated(
    authored: Situation,
    emitted: EmittedLevel,
    seed: BattleSeed,
) -> ProcgenOutcome {
    let zones = emitted.zones;
    let rosters = authored.combatants.rosters.clone();
    let player_faction = authored.combatants.player_faction;
    let theme = authored.map.theme;
    let mut outcome = outcome_from_emitted(authored, emitted);
    match deploy_rosters(
        &zones,
        &outcome.situation.map,
        &rosters,
        player_faction,
        seed,
    ) {
        Ok(placed) => outcome.placements.extend(placed),
        Err(err) => {
            warn!(
                "procgen could not deploy the roster into its zone ({err}); the battle will not \
                 set up (staying in Generation)"
            );
            outcome.findings.push(ContentFinding::DegradedFallback {
                context: FindingReferrer::new(format!(
                    "procgen roster deployment for theme {}",
                    *theme,
                )),
                detail:  FindingDetail::new(format!(
                    "a deployment zone could not stand its roster ({err}); the battle will not \
                     set up"
                )),
            });
            outcome.deployment_error = Some(err);
        }
    }
    outcome
}
