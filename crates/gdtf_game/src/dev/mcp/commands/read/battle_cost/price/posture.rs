//! What changing how the actor stands or faces costs.

use gdtf_battle_sim::{
    posture::{afforded_turn_tu_cost, can_set_facing, can_set_stance, stance_tu_cost},
    tuning::CombatTuning,
};

use super::quote::{Quote, afforded};
use crate::dev::mcp::{
    commands::read::battle_cost::reads::ActorRowItem,
    wire::{
        act_payload::{FacingNet, StanceNet},
        cost::CostLegalNet,
    },
};

/// Price taking `stance`.
pub(super) fn stance_quote(
    tuning: &CombatTuning,
    row: &ActorRowItem<'_, '_>,
    stance: StanceNet,
) -> Quote {
    let cost = stance_tu_cost(&tuning.stance_change_tu);
    let allowed = can_set_stance(
        row.stance,
        stance.to_sim(),
        row.tu,
        &tuning.stance_change_tu,
    );
    afforded(*row.tu, cost, CostLegalNet::new(*allowed))
}

/// Price turning toward `facing`, charging only the ring steps the pool affords.
pub(super) fn facing_quote(
    tuning: &CombatTuning,
    row: &ActorRowItem<'_, '_>,
    facing: FacingNet,
) -> Quote {
    let (from, to) = (**row.facing, facing.to_sim());
    let cost = afforded_turn_tu_cost(from, to, row.tu, &tuning.turn_tu);
    let allowed = can_set_facing(from, to, row.tu, &tuning.turn_tu);
    afforded(*row.tu, cost, CostLegalNet::new(*allowed))
}
