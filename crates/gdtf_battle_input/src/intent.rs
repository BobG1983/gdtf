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
//! # What THIS slice owns vs. what 222b fills
//!
//! This slice owns the no-act intents: [`ActIntent::SelectionClear`] /
//! [`ActIntent::LevelUp`] / [`ActIntent::LevelDown`] — drained here directly
//! (clearing [`SelectedShooter`] / clamping [`ActiveLevel`]). The act-bearing
//! variants ([`ActIntent::StanceCycle`] etc.) are DECLARED so the seam's shape is
//! fixed from the start; their drain arms — emitting the matching
//! `gdtf_battle_sim::acts::*Requested` — are added in 222b (this slice emits NO act,
//! per its scope). The cyclic ORDER those arms step is the authored
//! [`crate::cycle`] data.

use bevy::prelude::*;
use gdtf_battle_presenter::ActiveLevel;
use gdtf_battle_sim::{Level, MAX_LEVELS};

use crate::SelectedShooter;

/// One queued battle intent — the act a press (key OR button) asked for.
///
/// A domain enum (no-bare-types: a queued intent is a named act request, not a bare
/// discriminant). Both input surfaces [`push`](PendingActIntent::push) these; the
/// single [`dispatch_act_intents`] drain interprets them. The no-act variants are
/// handled THIS slice; the act-bearing variants are declared here and their drain
/// arms filled in 222b (this slice emits no act).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActIntent {
    /// Clear the current ganger selection (no sim act — input-layer state only).
    SelectionClear,
    /// Raise the presenter's [`ActiveLevel`] by one storey (clamped to the top).
    LevelUp,
    /// Lower the presenter's [`ActiveLevel`] by one storey (floored at 0).
    LevelDown,
    /// Step the selected ganger's stance through the authored cycle (filled in 222b).
    StanceCycle,
    /// Toggle the selected ganger's aim mode (filled in 222b).
    AimToggle,
    /// Step the selected ganger's facing through the authored cycle (filled in 222b).
    FacingCycle,
    /// Step the selected ganger's fire mode through the authored cycle (filled in 222b).
    FireModeCycle,
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

/// **Dispatch** the queued [`ActIntent`]s — the ONE drain system over the shared seam.
///
/// Drains the [`PendingActIntent`] queue every update (gated on
/// `BattleInProgress` by the plugin) and interprets each intent. THIS slice handles
/// the no-act intents directly:
///
/// - [`ActIntent::SelectionClear`] clears [`SelectedShooter`] to `None`.
/// - [`ActIntent::LevelUp`] / [`ActIntent::LevelDown`] step [`ActiveLevel`], clamped
///   to `0..MAX_LEVELS` ([`step_level`]): up saturates at `MAX_LEVELS - 1`, down
///   floors at `0`.
///
/// The act-bearing variants ([`ActIntent::StanceCycle`] / [`ActIntent::AimToggle`] /
/// [`ActIntent::FacingCycle`] / [`ActIntent::FireModeCycle`]) are NO-OPs here — this
/// slice emits no `*Requested` act (its scope is selection + level + the substrate).
/// 222b extends these arms to emit the matching `gdtf_battle_sim::acts::*Requested`,
/// reading the selected shooter off [`SelectedShooter`] and stepping the authored
/// [`crate::cycle`] order. Draining them now (rather than leaving them queued) keeps
/// the queue from growing before 222b lands; 222b does not push them until its arms
/// exist.
///
/// Param-only (`bevy-traps.md` #7): [`ResMut`] over the queue + the two pieces of
/// input-layer state it mutates. Registered `.after` the intent WRITERS
/// (`bevy-traps.md` #3) so it drains the same update's pushes.
pub fn dispatch_act_intents(
    mut pending: ResMut<PendingActIntent>,
    mut selected: ResMut<SelectedShooter>,
    mut active_level: ResMut<ActiveLevel>,
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
            // The act-bearing intents (222b): no act emitted in this slice. See the
            // doc-comment — 222b adds the `*Requested` emission arms here.
            ActIntent::StanceCycle
            | ActIntent::AimToggle
            | ActIntent::FacingCycle
            | ActIntent::FireModeCycle => {}
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
