//! Combat-log FORWARDER registration (GTW-620): one registrar line per log source plus
//! the bespoke turn-boundary forwarder — the sim-fact → [`CombatLogEvent`] half of the
//! combat log, registered by the same plugin that registers the FCT drains over the same
//! sim facts.

use bevy::{
    ecs::{message::Messages, schedule::SystemCondition},
    prelude::*,
};
use gdtf_battle_sim::{
    acts::{
        FireDeclaration, InjuryInflicted, MeleeStruck, MoveRejected, MovementOccurred, ReloadResult,
    },
    armor_wear::ArmorBroken,
    effects::{
        bleed::BleedStarted, dot::DotAfflicted, fields::FieldAfflicted, on_death::OnDeathOccurred,
    },
    falls::FallOccurred,
    prelude::BattleInProgress,
    suppression::SuppressionApplied,
    turn::TurnStarted,
};

use crate::{
    CombatLogEvent, CombatLogSourceAppExt, CombatLogSystems, ShotImpactResolved,
    forward_turn_started,
};

/// Registers the GTW-572 C5 forwarder seam (moved down from `gdtf_app` in GTW-620): the
/// buffered [`CombatLogEvent`] the forwarders write, one
/// [`add_combat_log_source`](CombatLogSourceAppExt::add_combat_log_source) registrar line
/// per log source, and the bespoke turn-boundary forwarder.
///
/// Each per-source forwarder joins [`CombatLogSystems::Forward`]; `gdtf_app`'s ONE
/// appender orders itself `.after(CombatLogSystems::Forward)` CROSS-CRATE (the exported
/// set — `bevy-traps.md` #3), so a sim fact written before an update becomes a rendered
/// line within that same update (forward → buffered event → append). Every forwarder is
/// gated on the live-battle witness + its source's own `Messages<S>` buffer, and the
/// registrar never `add_message`s a SOURCE buffer (the GTW-572 C4 / GTW-623 C4
/// convention): the sim plugins / this renderer register those, and a focused harness
/// that omits one keeps that forwarder inert (`bevy-traps.md` #1 / #4).
pub(super) fn register_combat_log_forwarders(app: &mut App) {
    // GTW-620: the presenter OWNS the resolved-event seam — the forwarders write the
    // buffered CombatLogEvent, so its buffer is registered unconditionally here (the
    // producer registering its own buffer, the ShotImpactResolved precedent in `fx.rs`).
    // The downstream appender in gdtf_app gates on this buffer EXISTING instead of
    // registering it, so an app harness built without this renderer stays inert rather
    // than failing param validation (`bevy-traps.md` #1 / #4).
    app.add_message::<CombatLogEvent>();

    // The log sources — one registrar line each (GTW-572 C5; adding a source = one
    // CombatLogSource impl + one CombatLogEvent variant + one classify arm in
    // `actors/fx/fct/log_event/` + one line here — ONE crate, GTW-620). The GTW-328
    // combat events first, then the GTW-572 C6 state changes (the Q2 ruling: falls,
    // melee damage, on-death kills, suppression, armor-broken, and the three
    // once-at-start afflictions — their per-tick signals have no source impl at all).
    app.add_combat_log_source::<FireDeclaration>()
        .add_combat_log_source::<MovementOccurred>()
        .add_combat_log_source::<MoveRejected>()
        .add_combat_log_source::<ShotImpactResolved>()
        .add_combat_log_source::<ReloadResult>()
        .add_combat_log_source::<InjuryInflicted>()
        .add_combat_log_source::<FallOccurred>()
        .add_combat_log_source::<MeleeStruck>()
        .add_combat_log_source::<OnDeathOccurred>()
        .add_combat_log_source::<SuppressionApplied>()
        .add_combat_log_source::<ArmorBroken>()
        .add_combat_log_source::<DotAfflicted>()
        .add_combat_log_source::<FieldAfflicted>()
        .add_combat_log_source::<BleedStarted>();

    // The turn boundary is the one bespoke forwarder (it reads PlayerFaction to label
    // Player vs Enemy, which the generic CombatLogSource shape cannot carry) — same set,
    // same gates.
    app.add_systems(
        Update,
        forward_turn_started
            .in_set(CombatLogSystems::Forward)
            .run_if(
                resource_exists::<BattleInProgress>
                    .and_then(resource_exists::<Messages<TurnStarted>>),
            ),
    );
}
