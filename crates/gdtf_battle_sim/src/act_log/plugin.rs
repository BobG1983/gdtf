//! [`wire_act_log`] — the act log's registration (GTW-727 C12 / C14).

use bevy::prelude::*;

use super::{log::ActLog, record::record_acts};
use crate::{
    acts::{
        FireDeclaration, InjuryInflicted, MeleeResolved, MeleeStruck, MoveRejected,
        MovementOccurred, ReloadResult, ThrowResolved,
    },
    armor_wear::ArmorBroken,
    effects::{
        bleed::{BleedStarted, Bleeding},
        dot::{DotAfflicted, DotTicked},
        fields::{FieldAfflicted, FieldTicked},
        on_death::OnDeathOccurred,
    },
    falls::FallOccurred,
    occupancy_sync::{CoverDestroyed, SimSystems},
    reaction::InterruptDeclared,
    shot_fired::ShotFired,
    suppression::SuppressionApplied,
    turn::TurnStarted,
};

/// Wire the act log into a Bevy [`App`]: the reaction-exposure buffer, every source buffer
/// the recorder reads, and the ONE registered writer.
///
/// Called from [`BattleSimPlugin`](crate::battle::BattleSimPlugin), which already bundles
/// every producer, so the buffer registrations below are IDEMPOTENT with the producers'
/// own — the same both-sides registration `OccupancyMaintenancePlugin` already documents
/// for `SlabDestroyed` / `GroundAccrued`. Registering them here is what makes
/// [`record_acts`]'s [`MessageReader`] params valid unconditionally, so a focused harness
/// that omits a producer keeps that source inert rather than failing param validation
/// (`bevy-traps.md` #1 / #4). Every buffer named is SIM-owned — this is the producer crate
/// registering its own.
///
/// [`record_acts`] joins [`SimSystems::Record`], whose `.after(Simulate)` edge is
/// configured by `OccupancyMaintenancePlugin` (the band's owner) and whose live-battle gate
/// is configured beside `Simulate`'s in `BattleSimPlugin` — a sibling set variant inherits
/// NEITHER, so both are stated explicitly at their owning sites. The system additionally
/// takes its own `resource_exists::<ActLog>` gate, so a harness that raises
/// [`BattleInProgress`](crate::battle::BattleInProgress) by hand without seeding a log is
/// inert rather than a panic.
pub fn wire_act_log(app: &mut App) {
    app.add_message::<InterruptDeclared>()
        .add_message::<TurnStarted>()
        .add_message::<MovementOccurred>()
        .add_message::<MoveRejected>()
        .add_message::<FireDeclaration>()
        .add_message::<ShotFired>()
        .add_message::<ReloadResult>()
        .add_message::<InjuryInflicted>()
        .add_message::<FallOccurred>()
        .add_message::<MeleeStruck>()
        .add_message::<OnDeathOccurred>()
        .add_message::<SuppressionApplied>()
        .add_message::<ArmorBroken>()
        .add_message::<DotAfflicted>()
        .add_message::<DotTicked>()
        .add_message::<FieldAfflicted>()
        .add_message::<FieldTicked>()
        .add_message::<BleedStarted>()
        .add_message::<Bleeding>()
        .add_message::<CoverDestroyed>()
        .add_message::<MeleeResolved>()
        .add_message::<ThrowResolved>()
        .add_systems(
            Update,
            record_acts
                .in_set(SimSystems::Record)
                .run_if(resource_exists::<ActLog>),
        );
}
