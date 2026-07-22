//! The combat-log FORWARDERS (GTW-572 C5; moved down from `gdtf_app` beside the
//! vocabulary it feeds in GTW-620): the [`CombatLogSource`] trait, the ONE generic
//! per-source forwarder system, the bespoke turn-boundary forwarder, and the
//! compile-time registrar.
//!
//! Each log source is a sim (or presenter) fact message; its thin forwarder resolves every
//! [`Entity`](bevy::prelude::Entity) to a [`LogName`] AT THIS BOUNDARY (so the shared
//! [`classify_log_event`](super::classify_log_event) stays `World`-free) and writes one
//! buffered [`CombatLogEvent`]; `gdtf_app`'s ONE appender drains that buffer strictly
//! `.after(CombatLogSystems::Forward)` (the exported set — the cross-crate ordering edge)
//! and spawns the lines. Adding a log source is a PRESENTER-ONLY change (GTW-620): one
//! [`CombatLogSource`] impl in `sources.rs`, one [`CombatLogEvent`] variant, and one
//! classify arm — all co-located in this module — plus one
//! [`CombatLogSourceAppExt::add_combat_log_source`] line in the renderer plugin's
//! combat-log registrar. Registration is COMPILE-TIME generic (GTW-572 P4 — no runtime
//! descriptor table), and the registrar NEVER calls `add_message` for a SOURCE buffer:
//! the producers (the sim plugins / this renderer) register their buffers, and a harness
//! that omits one keeps that forwarder inert (`bevy-traps.md` #1 / #4).

use bevy::{
    ecs::{message::Messages, schedule::SystemCondition},
    prelude::{
        App, IntoScheduleConfigs, Message, MessageReader, MessageWriter, Query, Res, SystemSet,
        Update, resource_exists,
    },
};
use gdtf_battle_sim::{
    battle::PlayerFaction, ganger::GangerName, prelude::BattleInProgress, turn::TurnStarted,
};

use super::event::{CombatLogEvent, LogName};
use crate::playback::Played;

/// The combat-log scheduling set (GTW-572 C5): every per-source
/// [`Forward`](Self::Forward)er runs in this set, and `gdtf_app`'s ONE appender orders
/// itself strictly `.after(CombatLogSystems::Forward)` CROSS-CRATE (GTW-620 — the set is
/// a presenter export) — the EXPLICIT ordering (`bevy-traps.md` #3) that lets a sim fact
/// written before an update become a rendered line within that same update
/// (forward → buffered event → append).
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CombatLogSystems {
    /// The thin per-source forwarders (sim fact → resolved [`CombatLogEvent`]).
    Forward,
}

/// A combat-log SOURCE — one sim/presenter fact message the log renders lines from
/// (GTW-572 C5).
///
/// Implemented per source in this module's `sources.rs` (one impl per message — the
/// forwarder half of the add-a-log-source recipe). [`to_event`](Self::to_event) resolves
/// the message into ONE [`CombatLogEvent`] with every [`Entity`](bevy::prelude::Entity)
/// already resolved to a [`LogName`] via `names` — or [`None`] for a message that logs
/// nothing (e.g. a COVER on-death, whose placeholder entity is no ganger).
pub trait CombatLogSource: Message + Clone {
    /// Resolve this fact into its log event, or [`None`] when it yields no line at all.
    fn to_event(&self, names: &Query<&GangerName>) -> Option<CombatLogEvent>;
}

/// Resolve an [`Entity`](bevy::prelude::Entity) to a [`LogName`] via the ganger-name query,
/// falling back to a generic label for an unnamed / unresolvable actor (so a name-less
/// ganger still logs a readable line rather than nothing).
pub(super) fn name_of(entity: bevy::prelude::Entity, names: &Query<&GangerName>) -> LogName {
    names
        .get(entity)
        .map_or_else(|_| LogName::new("Someone"), LogName::from_ganger)
}

/// `Update` (`CombatLogSystems::Forward`): the ONE generic forwarder — drain source `S`,
/// resolve each message's entities to names, and write the resolved [`CombatLogEvent`]s.
///
/// Param-only (`bevy-traps.md` #7): the [`MessageReader`], the read-only name query, and
/// the event [`MessageWriter`] (its `Messages<CombatLogEvent>` buffer is registered
/// unconditionally by the renderer plugin's combat-log registrar — the buffer's owner). The
/// registrar gates it on `BattleInProgress` + `Messages<S>` so the reader param is always
/// valid.
pub fn forward_log_source<S: CombatLogSource>(
    mut source: MessageReader<Played<S>>,
    names: Query<&GangerName>,
    mut events: MessageWriter<CombatLogEvent>,
) {
    for message in source.read() {
        if let Some(event) = message.to_event(&names) {
            events.write(event);
        }
    }
}

/// `Update` ([`CombatLogSystems::Forward`](CombatLogSystems)): the UNPACED forwarder —
/// drain source `S` straight off its own buffer, with no playback wrapper.
///
/// Used for exactly one source: [`ShotImpactResolved`](crate::ShotImpactResolved). That
/// signal is the PRESENTER's own, emitted by the impact animation at the moment a bolt
/// actually lands — and the bolt only spawns when the cursor plays its round, so the signal
/// is already at cursor time by construction. It also could not be replayed even in
/// principle: at the moment the cursor shows the round, the impact has not happened yet.
/// Leaving it unpaced is what makes a shot's outcome line land exactly as its bolt arrives.
pub fn forward_live_log_source<S: CombatLogSource>(
    mut source: MessageReader<S>,
    names: Query<&GangerName>,
    mut events: MessageWriter<CombatLogEvent>,
) {
    for message in source.read() {
        if let Some(event) = message.to_event(&names) {
            events.write(event);
        }
    }
}

/// `Update` (`CombatLogSystems::Forward`): the ONE bespoke forwarder — the turn boundary
/// needs the [`PlayerFaction`] resource (to label Player vs Enemy), which the generic
/// [`CombatLogSource`] shape cannot carry.
///
/// `player` is `Option<Res>` (`bevy-traps.md` #1): with no player faction resolved a turn
/// boundary cannot be labelled, so the message is DRAINED-and-dropped (the reader cursor
/// advances either way — drain-don't-replay, the pre-GTW-572 `.clear()` semantics).
pub fn forward_turn_started(
    mut turns: MessageReader<Played<TurnStarted>>,
    player: Option<Res<PlayerFaction>>,
    mut events: MessageWriter<CombatLogEvent>,
) {
    for turn in turns.read() {
        let Some(player) = player.as_deref() else {
            // Drained (the cursor advanced), not replayed — a boundary with no resolvable
            // Player/Enemy label logs nothing.
            continue;
        };
        events.write(CombatLogEvent::TurnStarted {
            now_active: turn.now_active,
            player:     *player,
        });
    }
}

/// The per-source registrar (GTW-572 C5): `app.add_combat_log_source::<Source>()` is the
/// ONE registration line a log source needs.
pub trait CombatLogSourceAppExt {
    /// Register source `S`'s PACED forwarder in `CombatLogSystems::Forward` — it drains
    /// [`Played<S>`](Played), so its line appears when the presenter SHOWS the fact rather
    /// than when the sim produced it (GTW-727 C28). This is what stops a whole reaction
    /// exchange's lines landing on one frame together.
    ///
    /// Gated on the live-battle witness + the source's own `Messages<S>` buffer (a
    /// [`MessageReader`] panics param validation without its buffer — `bevy-traps.md`
    /// #1 / #4). The gate stays on the SIM buffer `S` deliberately: the reader's
    /// `Messages<Played<S>>` is presenter-owned and always present here, so gating on it
    /// would be trivially true and would quietly retire the inertness convention. NEVER
    /// calls `add_message` for `S`: the producer registers its own buffer.
    fn add_combat_log_source<S: CombatLogSource>(&mut self) -> &mut Self;

    /// Register source `S`'s UNPACED forwarder — for a source that is ALREADY at cursor
    /// time because the presenter itself emits it (see [`forward_live_log_source`]).
    fn add_live_combat_log_source<S: CombatLogSource>(&mut self) -> &mut Self;
}

impl CombatLogSourceAppExt for App {
    fn add_combat_log_source<S: CombatLogSource>(&mut self) -> &mut Self {
        self.add_systems(
            Update,
            forward_log_source::<S>
                .in_set(CombatLogSystems::Forward)
                .run_if(
                    resource_exists::<BattleInProgress>.and_then(resource_exists::<Messages<S>>),
                ),
        );
        self
    }

    fn add_live_combat_log_source<S: CombatLogSource>(&mut self) -> &mut Self {
        self.add_systems(
            Update,
            forward_live_log_source::<S>
                .in_set(CombatLogSystems::Forward)
                .run_if(
                    resource_exists::<BattleInProgress>.and_then(resource_exists::<Messages<S>>),
                ),
        );
        self
    }
}
