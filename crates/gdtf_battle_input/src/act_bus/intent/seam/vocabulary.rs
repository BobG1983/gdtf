//! The classic [`ActIntent`] vocabulary — it changes when a press gains or loses an intent
//! variant.

use gdtf_battle_sim::{
    StanceKind,
    acts::{FireRequested, MoveRequested, SetFacingRequested},
};

/// One queued battle intent — the CLASSIC (non-contextual) act a press (key OR button)
/// asked for.
///
/// A domain enum (no-bare-types: a queued intent is a named act request, not a bare
/// discriminant). Both input surfaces [`push`](super::PendingActIntent::push) these; the
/// single [`dispatch_act_intents`](super::dispatch_act_intents) drain interprets them. The CONTEXTUAL acts (Execute
/// / Stabilize / Melee / Shove / Open Door / Enter / Exit Emplacement / Throw Grenade)
/// do NOT ride this enum — each is a [`ContextualAct`](crate::contextual::ContextualAct)
/// descriptor with its own buffered queue + generic drain (GTW-571; see
/// [`crate::contextual`]), so adding one never edits this vocabulary. The no-act variants
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
    /// Raise the presenter's [`ActiveLevel`](gdtf_battle_presenter::ActiveLevel) by one storey (clamped to the top).
    LevelUp,
    /// Lower the presenter's [`ActiveLevel`](gdtf_battle_presenter::ActiveLevel) by one storey (floored at 0).
    LevelDown,
    /// Toggle the presenter's [`ViewMode`](gdtf_battle_presenter::ViewMode) between
    /// `DownToActive` (draw `0..=active`, the default) and `FullView` (draw ALL storeys —
    /// the UFO full-stack view) (GTW-521). A no-act, presenter-view intent like
    /// [`LevelUp`](Self::LevelUp) / [`LevelDown`](Self::LevelDown): it does NOT touch
    /// [`ActiveLevel`](gdtf_battle_presenter::ActiveLevel), only flips the view mode; the drain toggles the presenter-owned
    /// [`ViewMode`](gdtf_battle_presenter::ViewMode) resource, honoring the one-way
    /// `input -> presenter` edge (input never reads the presenter's internals). The bound
    /// full-view key pushes it; the terrain draw + ganger visibility re-run on the resulting
    /// [`ViewMode`](gdtf_battle_presenter::ViewMode) change.
    ToggleFullView,
    /// Step the [`SelectedShooter`](crate::SelectedShooter)'s stance through the authored cycle — drained to
    /// [`SetStanceRequested`](gdtf_battle_sim::acts::SetStanceRequested) (GTW-227). The KEYBOARD stance-cycle key still pushes this
    /// (a blind step through the [`crate::cycle`] order); the GTW-267 action-bar replaced
    /// its BLIND-cycle BUTTON with three direct-set toggles that push
    /// [`SetStance`](Self::SetStance) instead.
    StanceCycle,
    /// Set the [`SelectedShooter`](crate::SelectedShooter)'s stance DIRECTLY to the carried [`StanceKind`] —
    /// drained to [`SetStanceRequested`](gdtf_battle_sim::acts::SetStanceRequested) for that exact posture (GTW-267). The action-bar
    /// Stance 3-toggle sub-panel pushes this (Stand / Kneel / Prone each set their
    /// posture directly, NOT a cycle step). With no selection it is a no-op in the drain.
    SetStance(StanceKind),
    /// Toggle the [`SelectedShooter`](crate::SelectedShooter)'s aim mode — drained to [`SetAimingRequested`](gdtf_battle_sim::acts::SetAimingRequested)
    /// (GTW-227).
    AimToggle,
    /// Step the [`SelectedShooter`](crate::SelectedShooter)'s facing through the authored cycle — drained to
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
    /// sim's `dispatch_move` / committed-walk concern, not this layer's).
    Move(MoveRequested),
    /// TURN the carried request — drained 1:1 to a [`SetFacingRequested`] (GTW-238). The
    /// right-click turn-to-face surface pushes this with the
    /// [`Direction`](gdtf_battle_sim::Direction) computed from the actor's cell toward
    /// the hovered cell; the drain emits it onto the SAME facing writer the
    /// [`FacingCycle`](Self::FacingCycle) intent uses (the per-45deg-step turn TU cost is
    /// the sim's facing dispatch, not this layer's).
    Turn(SetFacingRequested),
    /// RELOAD the [`SelectedShooter`](crate::SelectedShooter)'s weapon — drained to [`ReloadRequested`](gdtf_battle_sim::acts::ReloadRequested) for the
    /// selection (GTW-275). The weapon panel's Reload button pushes this; the drain
    /// emits [`ReloadRequested::new(selected)`](gdtf_battle_sim::acts::ReloadRequested::new) ONLY when a shooter
    /// is selected (a no-op with no selection). The per-weapon `reload_tu` cost is the
    /// sim's reload dispatch, not this layer's.
    Reload,
    /// END the active team's turn — drained 1:1 to a fieldless [`EndTurnRequested`](gdtf_battle_sim::acts::EndTurnRequested)
    /// (GTW-309). A GLOBAL turn signal like [`SelectionClear`](Self::SelectionClear) /
    /// [`LevelUp`](Self::LevelUp), NOT a per-ganger act: which team's turn is ending lives
    /// in the sim's [`ActiveFaction`](gdtf_battle_sim::ActiveFaction) resource, so it needs
    /// NO [`SelectedShooter`](crate::SelectedShooter) and carries no payload. The action-bar's End-Turn button
    /// pushes this; the drain emits the unit [`EndTurnRequested`](gdtf_battle_sim::acts::EndTurnRequested) unconditionally (the
    /// sim's [`dispatch_end_turn`](gdtf_battle_sim::dispatch_end_turn) advances the cycle
    /// and runs the next team's turn-start TU regen).
    EndTurn,
    /// CYCLE the [`SelectedShooter`](crate::SelectedShooter) to the NEXT player ganger in `(z, y, x)` order, wrapping
    /// (GTW-458). A SELECTION-layer intent (no sim act), drained directly in
    /// [`dispatch_act_intents`](super::dispatch_act_intents) via [`cycle_player_selection`](crate::selection::cycle_player_selection): it collects the player-faction
    /// gangers (enemies excluded), sorts them by the SAME [`cell_order_key`](crate::selection::cell_order_key) the battle-start
    /// auto-select uses, finds the current selection's index, and `set_selection`s the wrapping
    /// `(i + 1) % n` neighbour. With NO selection it makes the FIRST; an EMPTY player gang is a
    /// no-op. Both the `Tab` key and the on-bar Next button push this through the ONE seam
    /// (ADR-0001 — keys + buttons share one dispatch).
    SelectNext,
    /// CYCLE the [`SelectedShooter`](crate::SelectedShooter) to the PREVIOUS player ganger in `(z, y, x)` order,
    /// wrapping (GTW-458). The [`SelectNext`](Self::SelectNext) twin in reverse: it
    /// `set_selection`s the wrapping `(i + n - 1) % n` neighbour; with NO selection it makes
    /// the LAST; an EMPTY player gang is a no-op. `Shift+Tab` and the on-bar Prev button push
    /// this through the ONE seam.
    SelectPrev,
}
