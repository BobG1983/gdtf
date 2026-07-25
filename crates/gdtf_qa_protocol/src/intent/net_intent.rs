//! [`NetIntent`] — the injectable act vocabulary (GTW-734).

use serde::{Deserialize, Serialize};

use super::{
    payload::{AimNet, FacingNet, MeleeTargetNet, StanceNet},
    raw_input::KeyPressNet,
    ui_stack::UiStackNet,
};
use crate::ids::{
    CellLevelNet, DoorToken, EmplacementToken, FireModeIndex, FocusTargetNet, GangerToken,
    PointerPosNet,
};

/// One battle intent a QA client injects — the wire mirror of the `gdtf_battle_input`
/// act vocabulary (the classic `ActIntent` variants + the eight contextual acts + the
/// GTW-694-ruled `Select`).
///
/// The GTW-694 injection contract: the game side (`net_qa`, T4) maps each variant onto
/// an INTENT-layer push — a `PendingActIntent::push(ActIntent::…)` for the classic
/// acts, a `PendingContextualIntents::<A>::push(target)` for the contextual ones — and
/// never a raw `*Requested` write. The ACTOR is implicit: it is the game's current
/// `SelectedShooter` (a client [`Select`](Self::Select)s first), so the act variants
/// carry only their TARGET / parameters, not an actor token. The sim's own dispatch
/// gate stays the authoritative check (faction / adjacency / TU / state); a rejected
/// contextual offer comes back as an
/// [`InjectReceipt::Rejected`](crate::envelope::InjectReceipt::Rejected).
///
/// # Mirrored vocabulary
///
/// The classic acts mirror the input `ActIntent` (+ the sim `*Requested` payloads):
/// [`Fire`](Self::Fire) (`ActIntent::Fire`), [`Move`](Self::Move) (`ActIntent::Move`),
/// [`SetStance`](Self::SetStance) (`ActIntent::SetStance`),
/// [`SetAiming`](Self::SetAiming) (`ActIntent::AimToggle` — the wire carries the
/// explicit aim flag of `SetAimingRequested`), [`SetFacing`](Self::SetFacing)
/// (`ActIntent::Turn`), [`Reload`](Self::Reload), [`EndTurn`](Self::EndTurn),
/// [`SelectNext`](Self::SelectNext) / [`SelectPrev`](Self::SelectPrev) /
/// [`SelectionClear`](Self::SelectionClear) / [`LevelUp`](Self::LevelUp) /
/// [`LevelDown`](Self::LevelDown). The eight contextual acts mirror the input
/// `ContextualAct` set: [`Melee`](Self::Melee), [`Shove`](Self::Shove),
/// [`Stabilize`](Self::Stabilize), [`Execute`](Self::Execute),
/// [`ThrowGrenade`](Self::ThrowGrenade), [`OpenDoor`](Self::OpenDoor),
/// [`EnterEmplacement`](Self::EnterEmplacement),
/// [`ExitEmplacement`](Self::ExitEmplacement). [`Select`](Self::Select) is the
/// GTW-694-ruled select-by-token intent (its game-side `ActIntent::Select` lands in
/// T2).
///
/// Deliberately NOT mirrored (input variants with no direct wire need — a QA client
/// sets state deterministically, not by blind cycling): `ActIntent::StanceCycle` and
/// `ActIntent::FacingCycle` (superseded by the direct [`SetStance`](Self::SetStance) /
/// [`SetFacing`](Self::SetFacing)), and the presenter-only view toggle
/// `ActIntent::ToggleFullView`.
///
/// # The raw-input family (GTW-783)
///
/// Three variants stand APART from the act mirror above — they drive keyboard-shaped
/// behaviour (focus navigation, hover) that no act-intent covers, so a QA client can
/// capture in-engine evidence of keyboard-driven features over the wire:
/// [`PressKey`](Self::PressKey) taps a key, [`Hover`](Self::Hover) moves the pointer, and
/// [`SetFocus`](Self::SetFocus) points UI input focus at an entity. The game side realises
/// each through the SAME windowing-input path the backend uses (a `KeyboardInput` /
/// `CursorMoved` message, the `InputFocus` resource `sync_hover_to_focus` writes) — never a
/// direct sim mutation, so the one-way input → presenter → sim boundary holds.
///
/// # The DEV UI-stack swap (GTW-816)
///
/// [`SwapUiStack`](Self::SwapUiStack) stands apart from both families above: it drives no
/// act and touches no sim state at all. It names which UI stack the DEV swap harness should
/// make live, so an agent can drive the same swap a developer drives with the harness's
/// keyboard shortcut, and then assert behavioural parity across the swap (the live stack is
/// reported back on [`BattleView::ui_stack`](crate::view::BattleView::ui_stack)). A build
/// without the harness answers it
/// [`Rejected(Unavailable)`](crate::envelope::RejectReason::Unavailable).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NetIntent {
    /// Fire the current selection's weapon in the `mode` at the `target` `(cell, storey)`
    /// (mirrors `ActIntent::Fire` — the sim `FireRequested` carries `target_cell` +
    /// `target_level`). The wire names the fire mode by index into the weapon's list,
    /// never the resolved spec.
    Fire {
        /// The `(cell, storey)` aimed at — 3D, so a client can aim at a target on
        /// another floor (mirrors `FireRequested::target_cell` + `target_level`).
        target: CellLevelNet,
        /// Which authored fire mode to fire.
        mode:   FireModeIndex,
    },
    /// Step the current selection toward the `dest` `(cell, storey)` (mirrors
    /// `ActIntent::Move` — the sim `MoveRequested::dest` is a `CellLevel`).
    Move {
        /// The destination `(cell, storey)` — 3D, so a step can climb/descend a storey
        /// (mirrors `MoveRequested::dest`).
        dest: CellLevelNet,
    },
    /// Set the current selection's stance directly (mirrors `ActIntent::SetStance`).
    SetStance {
        /// The posture to change to.
        stance: StanceNet,
    },
    /// Set the current selection's aim mode (mirrors `ActIntent::AimToggle` via the
    /// explicit aim flag of `SetAimingRequested`).
    SetAiming {
        /// The requested aim mode.
        aim: AimNet,
    },
    /// Turn the current selection to a facing (mirrors `ActIntent::Turn`).
    SetFacing {
        /// The direction to turn to.
        facing: FacingNet,
    },
    /// Reload the current selection's weapon (mirrors `ActIntent::Reload`).
    Reload,
    /// End the active team's turn (mirrors `ActIntent::EndTurn`).
    EndTurn,
    /// Melee-strike a ganger or smash an adjacent structure (mirrors the `Melee`
    /// contextual act).
    Melee {
        /// The ganger-or-structure target.
        target: MeleeTargetNet,
    },
    /// Shove an adjacent opposing ganger (mirrors the `Shove` contextual act).
    Shove {
        /// The ganger to shove.
        target: GangerToken,
    },
    /// Arrest an adjacent downed ally's bleed-out (mirrors the `Stabilize` contextual
    /// act).
    Stabilize {
        /// The downed ally to stabilize.
        target: GangerToken,
    },
    /// Coup-de-grâce an adjacent downed enemy (mirrors the `Execute` contextual act).
    Execute {
        /// The downed enemy to execute.
        target: GangerToken,
    },
    /// Lob a grenade at a target `(cell, storey)` (mirrors the `ThrowGrenade` contextual
    /// act — its `Target` is a `CellLevel`).
    ThrowGrenade {
        /// The `(cell, storey)` to throw at — 3D, so a lob can reach another floor
        /// (mirrors `ThrowGrenadeAct::Target = CellLevel`).
        target: CellLevelNet,
    },
    /// Open an adjacent closed door (mirrors the `OpenDoor` contextual act).
    OpenDoor {
        /// The door to open.
        target: DoorToken,
    },
    /// Man an adjacent vacant emplacement (mirrors the `EnterEmplacement` contextual
    /// act).
    EnterEmplacement {
        /// The emplacement to enter.
        target: EmplacementToken,
    },
    /// Dismount the manned emplacement (mirrors the `ExitEmplacement` contextual act).
    ExitEmplacement {
        /// The emplacement to exit.
        target: EmplacementToken,
    },
    /// Select a ganger by its token (the GTW-694-ruled select-by-id intent; its
    /// game-side `ActIntent::Select` lands in T2).
    Select {
        /// The ganger to select.
        target: GangerToken,
    },
    /// Cycle the selection to the next player ganger (mirrors `ActIntent::SelectNext`).
    SelectNext,
    /// Cycle the selection to the previous player ganger (mirrors
    /// `ActIntent::SelectPrev`).
    SelectPrev,
    /// Clear the current selection (mirrors `ActIntent::SelectionClear`).
    SelectionClear,
    /// Raise the presenter's active level one storey (mirrors `ActIntent::LevelUp`).
    LevelUp,
    /// Lower the presenter's active level one storey (mirrors `ActIntent::LevelDown`).
    LevelDown,
    /// Tap a key — by named physical key or named bound action (GTW-783). The game side
    /// resolves it to a `KeyCode` and writes a real `KeyboardInput` press+release pair
    /// through the windowing-input message stream, exactly as the backend would.
    PressKey {
        /// The key to tap (a physical key, or the key a bound action is currently on).
        key: KeyPressNet,
    },
    /// Move the mouse pointer to a window position (GTW-783). The game side sets the primary
    /// window's cursor position and writes a `CursorMoved` message, so `bevy_ui`'s hover
    /// detection (and the hover-follows-focus bridge) reacts exactly as to a real move.
    Hover {
        /// The window-space logical-pixel position to hover at.
        at: PointerPosNet,
    },
    /// Point UI input focus at an entity directly (GTW-783). The game side sets the
    /// `InputFocus` resource — the same write `sync_hover_to_focus` performs — so a rejected
    /// (stale / malformed) token comes back
    /// [`UnknownEntity`](crate::envelope::RejectReason::UnknownEntity), never a panic.
    SetFocus {
        /// The entity to focus, by its wire token.
        target: FocusTargetNet,
    },
    /// Make a named UI stack the live one in the DEV swap harness (GTW-816) — the wire
    /// twin of the harness's keyboard shortcut, landing in the SAME single-slot swap
    /// latch the key and the on-screen swap buttons write.
    ///
    /// Absolute (never a flip): naming the already-live stack is a no-op, so a script can
    /// put the harness in a known stack without reading the current one first.
    SwapUiStack {
        /// The stack to make live.
        stack: UiStackNet,
    },
}
