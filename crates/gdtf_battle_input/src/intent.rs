//! The shared ACT-INTENT data seam (GTW-225 / GTW-48 S8 AC9): the ONE buffered
//! intent surface BOTH input surfaces write — the 222b keyboard systems AND the
//! 222c `gdtf_app` action-bar buttons — drained by ONE dispatch system.
//!
//! # Why a DATA seam, not a shared `fn`
//!
//! "Buttons + keys are parallel surfaces over the SAME act dispatch" must be REAL
//! across the crate boundary. A `SystemParam`-taking dispatch system in
//! `gdtf_battle_input` cannot be CALLED by a `bevy_ui` button system in `gdtf_app`
//! (you cannot invoke one Bevy system from inside another). The only thing that
//! spans the one legal `gdtf_app -> gdtf_battle_input` edge is DATA: both surfaces
//! [`push`](PendingActIntent::push) an [`ActIntent`] into the [`PendingActIntent`]
//! queue, and [`dispatch_act_intents`] drains it. ADR-0001's
//! `input -> presenter -> sim` chain stays acyclic — `gdtf_app` depends on
//! `gdtf_battle_input`, never the reverse.
//!
//! # What 222a owns vs. what 222b (GTW-227) fills
//!
//! 222a owns the no-act intents: [`ActIntent::SelectionClear`] /
//! [`ActIntent::LevelUp`] / [`ActIntent::LevelDown`] — drained here directly
//! (clearing [`SelectedShooter`] / clamping [`ActiveLevel`]). The act-bearing
//! variants ([`ActIntent::StanceCycle`] etc.) were DECLARED there so the seam's shape
//! is fixed from the start; 222b (GTW-227) FILLS their drain arms — emitting the
//! matching `gdtf_battle_sim::acts::*Requested` for the [`SelectedShooter`], reading
//! the actor's CURRENT [`Stance`](gdtf_battle_sim::Stance) /
//! [`Facing`](gdtf_battle_sim::Facing) / [`Aiming`](gdtf_battle_sim::Aiming) /
//! [`FireMode`](gdtf_battle_sim::FireMode) off a query and stepping the authored
//! [`crate::cycle`] order — and adds the [`ActIntent::Fire`] variant the left-click
//! FIRE surface writes (its `can_fire` guard runs at the WRITE site, so the drain
//! just emits the carried [`FireRequested`](gdtf_battle_sim::acts::FireRequested)
//! payload).

use bevy::{ecs::system::SystemParam, prelude::*};
use gdtf_battle_presenter::ActiveLevel;
use gdtf_battle_sim::{
    Aiming, Facing, FireMode, Level, MAX_LEVELS, Stance, StanceKind,
    acts::{
        AimRequest, FireRequested, MoveRequested, SetAimingRequested, SetFacingRequested,
        SetStanceRequested,
    },
};

use crate::{SelectedFireMode, SelectedShooter, cycle, fire_mode::next_fire_mode};

/// One queued battle intent — the act a press (key OR button) asked for.
///
/// A domain enum (no-bare-types: a queued intent is a named act request, not a bare
/// discriminant). Both input surfaces [`push`](PendingActIntent::push) these; the
/// single [`dispatch_act_intents`] drain interprets them. The no-act variants
/// ([`SelectionClear`](Self::SelectionClear) / [`LevelUp`](Self::LevelUp) /
/// [`LevelDown`](Self::LevelDown)) are 222a's; the act-bearing variants
/// ([`StanceCycle`](Self::StanceCycle) / [`AimToggle`](Self::AimToggle) /
/// [`FacingCycle`](Self::FacingCycle) / [`FireModeCycle`](Self::FireModeCycle)) +
/// the [`Fire`](Self::Fire) variant are filled / added by 222b (GTW-227).
///
/// Only [`PartialEq`] (no `Eq` / `Hash`): [`Fire`](Self::Fire) carries an owned
/// [`FireRequested`] whose [`FireModeSpec`](gdtf_battle_sim::FireModeSpec) has `f32`
/// fields, so the enum cannot derive `Eq` / `Hash`
/// (`f32`-field-breaks-container-`Eq`-`Hash`). Nothing keys an `ActIntent` — it is
/// only pushed to / drained from a `Vec` and compared in tests — so `PartialEq`
/// suffices. Not `Copy`: [`Fire`](Self::Fire)'s [`FireRequested`] owns a
/// [`FireModeSpec`](gdtf_battle_sim::FireModeSpec) (which owns a
/// [`ModeName`](gdtf_battle_sim::ModeName)); the enum is `Clone`.
#[derive(Debug, Clone, PartialEq)]
pub enum ActIntent {
    /// Clear the current ganger selection (no sim act — input-layer state only).
    SelectionClear,
    /// Raise the presenter's [`ActiveLevel`] by one storey (clamped to the top).
    LevelUp,
    /// Lower the presenter's [`ActiveLevel`] by one storey (floored at 0).
    LevelDown,
    /// Step the [`SelectedShooter`]'s stance through the authored cycle — drained to
    /// [`SetStanceRequested`] (GTW-227).
    StanceCycle,
    /// Toggle the [`SelectedShooter`]'s aim mode — drained to [`SetAimingRequested`]
    /// (GTW-227).
    AimToggle,
    /// Step the [`SelectedShooter`]'s facing through the authored cycle — drained to
    /// [`SetFacingRequested`] (GTW-227).
    FacingCycle,
    /// Step the [`SelectedFireMode`] through the selected weapon's offered modes
    /// (GTW-227) — mutates [`SelectedFireMode`] in the drain (no sim message; the
    /// chosen mode rides the next [`Fire`](Self::Fire)).
    FireModeCycle,
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

/// The five `*Requested` act [`MessageWriter`]s [`dispatch_act_intents`] emits onto,
/// grouped into ONE [`SystemParam`] so the drain's parameter list stays under
/// clippy's argument-count gate (the sim's `BattleGridsParam` precedent).
///
/// Grouping the cohesive emission writers into one param keeps
/// [`dispatch_act_intents`] at six parameters; the drain arms call
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
/// actor's CURRENT [`Stance`] / [`Facing`] / [`Aiming`] / [`FireMode`] off the
/// `actors` query and stepping the authored [`crate::cycle`] order:
///
/// - [`ActIntent::StanceCycle`] emits [`SetStanceRequested`] for the
///   next-of-cycle [`StanceKind`] ([`cycle::next_stance`]).
/// - [`ActIntent::AimToggle`] emits [`SetAimingRequested`] toggling the actor's
///   current [`Aiming`] flag.
/// - [`ActIntent::FacingCycle`] emits [`SetFacingRequested`] for the next-of-cycle
///   [`Direction`](gdtf_battle_sim::Direction) ([`cycle::next_facing`]).
/// - [`ActIntent::FireModeCycle`] advances [`SelectedFireMode`] among the selected
///   weapon's offered modes ([`next_fire_mode`]) — input-layer state only, no sim
///   message (the chosen mode rides the next [`ActIntent::Fire`]).
/// - [`ActIntent::Fire`] emits the carried [`FireRequested`] verbatim — the `can_fire`
///   guard already ran at the WRITE site.
/// - [`ActIntent::Move`] emits the carried [`MoveRequested`] verbatim onto the move
///   writer (GTW-238) — the destination's terrain TU cost is the sim's `move_ganger`.
/// - [`ActIntent::Turn`] emits the carried [`SetFacingRequested`] verbatim onto the SAME
///   facing writer the [`FacingCycle`](ActIntent::FacingCycle) arm uses (GTW-238) — the
///   per-45deg-step turn TU cost is the sim's facing dispatch.
///
/// With NO [`SelectedShooter`] the cycle intents are no-ops (nothing to act on); a
/// cycle intent for a selected entity that lacks the relevant component is skipped
/// (fail-closed, no panic) via the query lookup. Param-only (`bevy-traps.md` #7):
/// [`ResMut`] over the queue + input-layer state, a read-only `actors` [`Query`], and
/// the [`ActWriters`] message-writer bundle (`bevy-traps.md` #4 — buffered messages).
/// Registered `.after` the intent WRITERS (`bevy-traps.md` #3) so it drains the same
/// update's pushes.
pub fn dispatch_act_intents(
    mut pending: ResMut<PendingActIntent>,
    mut selected: ResMut<SelectedShooter>,
    mut active_level: ResMut<ActiveLevel>,
    mut fire_mode: ResMut<SelectedFireMode>,
    actors: Query<(&Stance, &Facing, &Aiming, &FireMode)>,
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
            ActIntent::AimToggle => {
                let Some(actor) = **selected else { continue };
                let Ok((_, _, aiming, _)) = actors.get(actor) else {
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
            ActIntent::FireModeCycle => {
                let Some(actor) = **selected else { continue };
                let Ok((_, _, _, mode)) = actors.get(actor) else {
                    continue;
                };
                // `FireModeSpec` is no longer `Copy` — clone the current selected mode
                // to read it without moving it out of the `ResMut`.
                *fire_mode = SelectedFireMode::new(next_fire_mode((**fire_mode).clone(), mode));
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
        }
    }
}

/// Which way a level-step intent moves the [`ActiveLevel`].
///
/// A tiny domain enum so [`step_level`] reads `Up` / `Down` rather than a bare
/// sign — the two directions the level-cycle keys drive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LevelStep {
    /// Toward a higher storey (saturating at `MAX_LEVELS - 1`).
    Up,
    /// Toward a lower storey (flooring at `0`).
    Down,
}

/// The [`Level`] after stepping `current` one storey in `direction`, clamped to the
/// valid `0..MAX_LEVELS` storey range.
///
/// `Up` saturates at the top storey (`MAX_LEVELS - 1`) — it never exceeds the grid's
/// storey count; `Down` floors at `0`. Saturating `u8` arithmetic, then a clamp to
/// the top storey, so the level can never wrap or escape the grid (the contract's
/// `0..MAX_LEVELS` clamp). Pure storey math; not a `const fn` because it deref-reads
/// the derived-`Deref` [`Level`] newtype, which is not a const operation.
#[must_use]
pub fn step_level(current: Level, direction: LevelStep) -> Level {
    // The top valid storey index. `MAX_LEVELS` is 8, so `MAX_LEVELS - 1` (= 7) is the
    // highest storey; `saturating_sub` guards the (impossible) `MAX_LEVELS == 0`.
    let top = MAX_LEVELS.saturating_sub(1);
    let raw = *current;
    let stepped = match direction {
        // Saturating add then clamp to the top storey: even if `raw` were already at
        // u8::MAX it could not wrap, and it can never exceed `top`.
        LevelStep::Up => {
            let up = raw.saturating_add(1);
            if up > top { top } else { up }
        }
        // Saturating sub floors at 0.
        LevelStep::Down => raw.saturating_sub(1),
    };
    Level::new(stepped)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// AC6 — level-up steps toward the top and SATURATES at `MAX_LEVELS - 1`; it
    /// never exceeds the grid's storey count.
    #[test]
    fn step_level_up_saturates_at_the_top_storey() {
        // From the ground floor, up moves one storey.
        assert_eq!(step_level(Level::new(0), LevelStep::Up), Level::new(1));
        // One below the top moves to the top.
        assert_eq!(
            step_level(Level::new(MAX_LEVELS - 2), LevelStep::Up),
            Level::new(MAX_LEVELS - 1),
        );
        // At the top, up saturates (stays at the top storey).
        assert_eq!(
            step_level(Level::new(MAX_LEVELS - 1), LevelStep::Up),
            Level::new(MAX_LEVELS - 1),
        );
    }

    /// AC6 — level-down steps toward the ground and FLOORS at `0`.
    #[test]
    fn step_level_down_floors_at_zero() {
        // From an upper storey, down moves one storey.
        assert_eq!(step_level(Level::new(3), LevelStep::Down), Level::new(2));
        // At the ground floor, down floors (stays at 0).
        assert_eq!(step_level(Level::new(0), LevelStep::Down), Level::new(0));
    }
}
