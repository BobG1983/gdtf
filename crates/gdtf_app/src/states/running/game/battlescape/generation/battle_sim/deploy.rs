//! The app-side **roster deployment** glue (GTW-744): after procgen generates the terrain,
//! deploy the authored roster members onto the generated map's deployment zones.
//!
//! The sim ([`gdtf_battle_sim`]) OWNS the deployment MODEL — [`deploy_rosters`] is the
//! deterministic, render-free placement; this app-side glue only DRIVES it (it reads the
//! sim's [`EmittedLevel::zones`] and threads the placed gangers back into the situation the
//! battle is built from). Split out of `procgen.rs` (which owns the terrain-generation drive)
//! so the deployment concern is its own file.

use bevy::prelude::warn;
use gdtf_assets::{ContentFinding, FindingDetail, FindingReferrer};
use gdtf_battle_sim::{
    procgen::{EmittedLevel, deploy_rosters},
    rng::BattleSeed,
    situation::Situation,
};

use super::procgen::{ProcgenOutcome, outcome_from_emitted};

/// Merge the generated terrain over `authored` ([`outcome_from_emitted`]), then DEPLOY the
/// authored roster members onto the generated map's deployment zones — EXTENDING the
/// situation's gangers with the deterministically-placed set (GTW-744).
///
/// [`deploy_rosters`] derives each [`RosterMember`](gdtf_battle_sim::situation::RosterMember)'s
/// spawn `(cell, level)` / facing / stance from the two [`EmittedLevel::zones`] the generation
/// surfaced, deterministically in `seed` (the same [`BattleSeed`] the terrain used). On success
/// the placed gangers are appended to the merged situation (setup then spawns them like any
/// authored ganger). On a fail-closed
/// [`PackingError::DeploymentZoneTooSmall`](gdtf_battle_sim::procgen::PackingError::DeploymentZoneTooSmall)
/// it records a finding + sets [`ProcgenOutcome::deployment_error`] so the caller aborts setup
/// (no under-populated battle). The deploy reads `outcome.situation` (procgen terrain + any
/// authored gangers), so standability + existing-ganger non-overlap are correct.
///
/// `pub(crate)`, not `pub(super)`: the GTW-655 dev-tools stepper (`crate::dev::procgen_stepper`)
/// is a SECOND caller — GTW-765 makes its staged-drive finish DEPLOY the roster through this
/// same function (re-exported from the module `mod.rs` under `dev_tools`), rather than the
/// terrain-only [`outcome_from_emitted`], so a stepper-started battle has gangers on the map.
#[must_use]
pub(crate) fn deploy_over_generated(
    authored: Situation,
    emitted: EmittedLevel,
    seed: BattleSeed,
) -> ProcgenOutcome {
    // Capture the deploy inputs BEFORE `outcome_from_emitted` moves `authored`.
    let zones = emitted.zones;
    let rosters = authored.rosters.clone();
    let player_faction = authored.player_faction;
    let theme = authored.theme;
    let mut outcome = outcome_from_emitted(authored, emitted);
    match deploy_rosters(&zones, &outcome.situation, &rosters, player_faction, seed) {
        Ok(placed) => outcome.situation.gangers.extend(placed),
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
