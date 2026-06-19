//! The buffered intent queue + the ONE drain system both input surfaces feed.

use bevy::{ecs::system::SystemParam, prelude::*};
use gdtf_battle_presenter::ActiveLevel;
use gdtf_battle_sim::{
    Aiming, Facing, Stance, StanceKind,
    acts::{
        AimRequest, EndTurnRequested, FireRequested, MoveRequested, ReloadRequested,
        SetAimingRequested, SetFacingRequested, SetStanceRequested,
    },
};

use crate::{
    SelectedShooter, cycle,
    intent::level::{LevelStep, step_level},
};

/// One queued battle intent — the act a press (key OR button) asked for.
///
/// A domain enum (no-bare-types: a queued intent is a named act request, not a bare
/// discriminant). Both input surfaces [`push`](PendingActIntent::push) these; the
/// single [`dispatch_act_intents`] drain interprets them. The no-act variants
/// ([`SelectionClear`](Self::SelectionClear) / [`LevelUp`](Self::LevelUp) /
/// [`LevelDown`](Self::LevelDown)) are 222a's; the act-bearing variants
/// ([`StanceCycle`](Self::StanceCycle) / [`SetStance`](Self::SetStance) /
/// [`AimToggle`](Self::AimToggle) / [`FacingCycle`](Self::FacingCycle)) + the
/// [`Fire`](Self::Fire) variant are filled / added by 222b (GTW-227). (The fire-mode
/// `FireModeCycle` blind-cycle variant was REMOVED in GTW-254; the GTW-265 action-bar
/// then replaced the popup picker with a 3-toggle Mode sub-panel that sets
/// [`SelectedFireMode`](crate::SelectedFireMode) directly — no intent variant.)
///
/// Only [`PartialEq`] (no `Eq` / `Hash`): [`Fire`](Self::Fire) carries an owned
/// [`FireRequested`] whose [`FireModeSpec`](gdtf_battle_sim::FireModeSpec) has `f32`
/// fields, so the enum cannot derive `Eq` / `Hash`
/// (`f32`-field-breaks-container-`Eq`-`Hash`). Nothing keys an `ActIntent` — it is
/// only pushed to / drained from a `Vec` and compared in tests — so `PartialEq`
/// suffices. Not `Copy`: [`Move`](Self::Move) / [`Turn`](Self::Turn) and
/// [`Fire`](Self::Fire) carry owned request payloads, and the enum stays `Clone`
/// (no `Copy`) to keep the variant set uniform.
#[derive(Debug, Clone, PartialEq)]
pub enum ActIntent {
    /// Clear the current ganger selection (no sim act — input-layer state only).
    SelectionClear,
    /// Raise the presenter's [`ActiveLevel`] by one storey (clamped to the top).
    LevelUp,
    /// Lower the presenter's [`ActiveLevel`] by one storey (floored at 0).
    LevelDown,
    /// Step the [`SelectedShooter`]'s stance through the authored cycle — drained to
    /// [`SetStanceRequested`] (GTW-227). The KEYBOARD stance-cycle key still pushes this
    /// (a blind step through the [`crate::cycle`] order); the GTW-267 action-bar replaced
    /// its BLIND-cycle BUTTON with three direct-set toggles that push
    /// [`SetStance`](Self::SetStance) instead.
    StanceCycle,
    /// Set the [`SelectedShooter`]'s stance DIRECTLY to the carried [`StanceKind`] —
    /// drained to [`SetStanceRequested`] for that exact posture (GTW-267). The action-bar
    /// Stance 3-toggle sub-panel pushes this (Stand / Kneel / Prone each set their
    /// posture directly, NOT a cycle step). With no selection it is a no-op in the drain.
    SetStance(StanceKind),
    /// Toggle the [`SelectedShooter`]'s aim mode — drained to [`SetAimingRequested`]
    /// (GTW-227).
    AimToggle,
    /// Step the [`SelectedShooter`]'s facing through the authored cycle — drained to
    /// [`SetFacingRequested`] (GTW-227).
    FacingCycle,
    /// FIRE the carried request — drained 1:1 to a [`FireRequested`] (GTW-227). The
    /// left-click FIRE surface runs the shared `can_fire` guard at the WRITE site and
    /// only pushes this when it passes, so the drain emits the payload unconditionally.
    Fire(FireRequested),
    /// MOVE the carried request — drained 1:1 to a [`MoveRequested`] (GTW-238). The
    /// unified left-click surface pushes this when the MOVE branch wins (a player-faction
    /// selection + an empty, in-bounds, unblocked hovered cell); the drain emits the
    /// payload verbatim onto the move writer (the destination's terrain TU cost is the
    /// sim's [`move_ganger`](gdtf_battle_sim::move_ganger), not this layer's).
    Move(MoveRequested),
    /// TURN the carried request — drained 1:1 to a [`SetFacingRequested`] (GTW-238). The
    /// right-click turn-to-face surface pushes this with the
    /// [`Direction`](gdtf_battle_sim::Direction) computed from the actor's cell toward
    /// the hovered cell; the drain emits it onto the SAME facing writer the
    /// [`FacingCycle`](Self::FacingCycle) intent uses (the per-45deg-step turn TU cost is
    /// the sim's facing dispatch, not this layer's).
    Turn(SetFacingRequested),
    /// RELOAD the [`SelectedShooter`]'s weapon — drained to [`ReloadRequested`] for the
    /// selection (GTW-275). The weapon panel's Reload button pushes this; the drain
    /// emits [`ReloadRequested::new(selected)`](ReloadRequested::new) ONLY when a shooter
    /// is selected (a no-op with no selection). The per-weapon `reload_tu` cost is the
    /// sim's reload dispatch, not this layer's.
    Reload,
    /// END the active team's turn — drained 1:1 to a fieldless [`EndTurnRequested`]
    /// (GTW-309). A GLOBAL turn signal like [`SelectionClear`](Self::SelectionClear) /
    /// [`LevelUp`](Self::LevelUp), NOT a per-ganger act: which team's turn is ending lives
    /// in the sim's [`ActiveFaction`](gdtf_battle_sim::ActiveFaction) resource, so it needs
    /// NO [`SelectedShooter`] and carries no payload. The action-bar's End-Turn button
    /// pushes this; the drain emits the unit [`EndTurnRequested`] unconditionally (the
    /// sim's [`dispatch_end_turn`](gdtf_battle_sim::dispatch_end_turn) advances the cycle
    /// and runs the next team's turn-start TU regen).
    EndTurn,
}

/// The shared intent QUEUE — the buffered seam both input surfaces write.
///
/// A named newtype over a `Vec<ActIntent>` (no-bare-types: a pending-intent queue is
/// a domain value; the inner `Vec` is the collection-of-domain-values carve-out),
/// owned by `gdtf_battle_input` and made `pub` so the 222b keyboard systems AND the
/// 222c `gdtf_app` buttons both reach it across the legal `gdtf_app ->
/// gdtf_battle_input` edge. `init_resource`-d by
/// [`GdtfBattleInputPlugin`](crate::GdtfBattleInputPlugin) (its [`Default`] is the
/// empty queue) and DRAINED every update by [`dispatch_act_intents`], so a buffered
/// intent is acted on exactly once.
#[derive(Resource, Debug, Default)]
pub struct PendingActIntent(Vec<ActIntent>);

impl PendingActIntent {
    /// Queue `intent` to be drained by [`dispatch_act_intents`] next time it runs.
    ///
    /// The single write-point both surfaces call — a key system or a `gdtf_app`
    /// button system pushes the intent the press maps to. Buffered (not applied
    /// inline) so the ONE drain system is the only place an intent takes effect.
    pub fn push(&mut self, intent: ActIntent) {
        self.0.push(intent);
    }

    /// Take and clear every queued intent — the drain's read.
    ///
    /// Returns the buffered intents in push order and leaves the queue empty, so an
    /// intent is acted on exactly once. `pub(crate)` — only the in-crate drain
    /// consumes the queue; external surfaces only [`push`](Self::push).
    pub(crate) fn drain(&mut self) -> Vec<ActIntent> {
        core::mem::take(&mut self.0)
    }

    /// Whether the queue currently holds no intents (test/inspection helper).
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// The six `*Requested` act [`MessageWriter`]s [`dispatch_act_intents`] emits onto,
/// grouped into ONE [`SystemParam`] so the drain's parameter list stays under
/// clippy's argument-count gate (the sim's `BattleGridsParam` precedent).
///
/// Grouping the cohesive emission writers into one param keeps
/// [`dispatch_act_intents`] at five parameters; the drain arms call
/// `acts.fire.write(..)` / `acts.stance.write(..)` etc. A transparent system-param
/// bundle of framework [`MessageWriter`]s — not itself a wrapped domain scalar. The
/// [`SetFacingRequested`] writer ([`facing`](Self::facing)) is REUSED by BOTH the
/// keyboard [`ActIntent::FacingCycle`] arm and the right-click
/// [`ActIntent::Turn`] arm (one set-facing message type, one sim dispatch).
#[derive(SystemParam)]
pub struct ActWriters<'w> {
    /// The FIRE-act writer — the left-click FIRE surface's drained message.
    fire:     MessageWriter<'w, FireRequested>,
    /// The MOVE-act writer — the left-click MOVE branch's drained message (GTW-238).
    movement: MessageWriter<'w, MoveRequested>,
    /// The set-stance writer — the stance-cycle key's drained message.
    stance:   MessageWriter<'w, SetStanceRequested>,
    /// The set-aiming writer — the aim-toggle key's drained message.
    aiming:   MessageWriter<'w, SetAimingRequested>,
    /// The set-facing writer — the facing-cycle key's AND the right-click turn-to-face
    /// surface's drained message (GTW-238 reuses it for [`ActIntent::Turn`]).
    facing:   MessageWriter<'w, SetFacingRequested>,
    /// The reload-act writer — the weapon panel Reload button's drained message
    /// (GTW-275).
    reload:   MessageWriter<'w, ReloadRequested>,
    /// The end-turn writer — the action-bar End-Turn button's drained message (GTW-309).
    /// A fieldless turn signal: the drain emits the unit [`EndTurnRequested`] verbatim.
    end_turn: MessageWriter<'w, EndTurnRequested>,
}

/// **Dispatch** the queued [`ActIntent`]s — the ONE drain system over the shared seam.
///
/// Drains the [`PendingActIntent`] queue every update (gated on `BattleInProgress` by
/// the plugin) and interprets each intent. The no-act intents are handled directly:
///
/// - [`ActIntent::SelectionClear`] clears [`SelectedShooter`] to `None`.
/// - [`ActIntent::LevelUp`] / [`ActIntent::LevelDown`] step [`ActiveLevel`], clamped
///   to `0..MAX_LEVELS` ([`step_level`]): up saturates at `MAX_LEVELS - 1`, down
///   floors at `0`.
///
/// The act-bearing intents (GTW-227) emit the matching
/// `gdtf_battle_sim::acts::*Requested` for the [`SelectedShooter`], reading the
/// actor's CURRENT [`Stance`] / [`Facing`] / [`Aiming`] off the `actors` query and
/// stepping the authored [`crate::cycle`] order:
///
/// - [`ActIntent::StanceCycle`] emits [`SetStanceRequested`] for the
///   next-of-cycle [`StanceKind`] ([`cycle::next_stance`]).
/// - [`ActIntent::SetStance`] emits [`SetStanceRequested`] for the CARRIED
///   [`StanceKind`] directly (GTW-267 — the action-bar 3-toggle stance set).
/// - [`ActIntent::AimToggle`] emits [`SetAimingRequested`] toggling the actor's
///   current [`Aiming`] flag.
/// - [`ActIntent::FacingCycle`] emits [`SetFacingRequested`] for the next-of-cycle
///   [`Direction`](gdtf_battle_sim::Direction) ([`cycle::next_facing`]).
/// - [`ActIntent::Fire`] emits the carried [`FireRequested`] verbatim — the `can_fire`
///   guard already ran at the WRITE site.
/// - [`ActIntent::Move`] emits the carried [`MoveRequested`] verbatim onto the move
///   writer (GTW-238) — the destination's terrain TU cost is the sim's `move_ganger`.
/// - [`ActIntent::Turn`] emits the carried [`SetFacingRequested`] verbatim onto the SAME
///   facing writer the [`FacingCycle`](ActIntent::FacingCycle) arm uses (GTW-238) — the
///   per-45deg-step turn TU cost is the sim's facing dispatch.
/// - [`ActIntent::Reload`] emits [`ReloadRequested`] for the [`SelectedShooter`]
///   (GTW-275) — a no-op with no selection; the per-weapon `reload_tu` cost is the sim's
///   reload dispatch.
/// - [`ActIntent::EndTurn`] emits the fieldless [`EndTurnRequested`] unconditionally
///   (GTW-309) — a GLOBAL turn signal needing no selection; the sim's `dispatch_end_turn`
///   advances the turn cycle and runs the next team's turn-start TU regen.
///
/// With NO [`SelectedShooter`] the cycle intents are no-ops (nothing to act on); a
/// cycle intent for a selected entity that lacks the relevant component is skipped
/// (fail-closed, no panic) via the query lookup. Param-only (`bevy-traps.md` #7):
/// [`ResMut`] over the queue + the presenter [`ActiveLevel`], a read-only `actors`
/// [`Query`], and the [`ActWriters`] message-writer bundle (`bevy-traps.md` #4 —
/// buffered messages). Registered `.after` the intent WRITERS (`bevy-traps.md` #3) so
/// it drains the same update's pushes.
pub fn dispatch_act_intents(
    mut pending: ResMut<PendingActIntent>,
    mut selected: ResMut<SelectedShooter>,
    mut active_level: ResMut<ActiveLevel>,
    actors: Query<(&Stance, &Facing, &Aiming)>,
    mut acts: ActWriters,
) {
    for intent in pending.drain() {
        match intent {
            ActIntent::SelectionClear => {
                if selected.is_some() {
                    *selected = SelectedShooter::cleared();
                }
            }
            ActIntent::LevelUp => {
                let next = step_level(**active_level, LevelStep::Up);
                if next != **active_level {
                    *active_level = ActiveLevel(next);
                }
            }
            ActIntent::LevelDown => {
                let next = step_level(**active_level, LevelStep::Down);
                if next != **active_level {
                    *active_level = ActiveLevel(next);
                }
            }
            ActIntent::StanceCycle => {
                let Some(actor) = **selected else { continue };
                let Ok((stance, ..)) = actors.get(actor) else {
                    continue;
                };
                let next: StanceKind = cycle::next_stance(**stance);
                acts.stance.write(SetStanceRequested::new(actor, next));
            }
            ActIntent::SetStance(kind) => {
                // Direct set (GTW-267): emit the carried posture for the selection. No
                // need to read the current stance — the action-bar toggle named the exact
                // target posture. With no selection there is nothing to act on.
                let Some(actor) = **selected else { continue };
                acts.stance.write(SetStanceRequested::new(actor, kind));
            }
            ActIntent::AimToggle => {
                let Some(actor) = **selected else { continue };
                let Ok((_, _, aiming)) = actors.get(actor) else {
                    continue;
                };
                acts.aiming
                    .write(SetAimingRequested::new(actor, AimRequest::new(!**aiming)));
            }
            ActIntent::FacingCycle => {
                let Some(actor) = **selected else { continue };
                let Ok((_, facing, ..)) = actors.get(actor) else {
                    continue;
                };
                let next = cycle::next_facing(**facing);
                acts.facing.write(SetFacingRequested::new(actor, next));
            }
            ActIntent::Fire(request) => {
                acts.fire.write(request);
            }
            ActIntent::Move(request) => {
                acts.movement.write(request);
            }
            ActIntent::Turn(request) => {
                acts.facing.write(request);
            }
            ActIntent::Reload => {
                // Emit a reload for the selection only — mirror the SetStance arm
                // (GTW-275). With no selection there is nothing to reload.
                let Some(actor) = **selected else { continue };
                acts.reload.write(ReloadRequested::new(actor));
            }
            ActIntent::EndTurn => {
                // Emit the fieldless turn signal (GTW-309). A GLOBAL act like SelectionClear
                // / LevelUp — no selection needed; the sim's ActiveFaction tracks whose turn
                // is ending, so the drain just writes the unit message unconditionally.
                acts.end_turn.write(EndTurnRequested);
            }
        }
    }
}
