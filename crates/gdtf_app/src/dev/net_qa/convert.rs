//! The wildcard-free [`NetIntent`] classification (GTW-737, the GTW-694 architecture's
//! T4 exhaustive wire-to-local mapping).
//!
//! [`classify`] is a PURE, EXHAUSTIVE `match` from the wire [`NetIntent`] vocabulary onto
//! the local intent vocabulary — no catch-all arm, so adding a `NetIntent` (or a local
//! [`ActIntent`] / contextual act) variant forces a compile-time decision HERE (the
//! GTW-694 "convert.rs = wildcard-free exhaustive match + per-act contextual registry").
//! The three outcome enums split by what resolution each intent still needs:
//!
//! - [`Classified::Classic`] — a self-contained [`ActIntent`], ready to push verbatim.
//! - [`Classified::Actor`] — a classic act whose payload needs the current
//!   `SelectedShooter` (Move / Turn / Fire / aim) to finish building (see
//!   [`ActorIntent`]).
//! - [`Classified::Select`] — the by-token actor pick (its [`GangerToken`] still to
//!   resolve).
//! - [`Classified::Contextual`] — one of the eight contextual acts, its wire target still
//!   to resolve + offer-gate ([`ContextualIntent`] — the per-act registry).
//!
//! This module is deliberately BEVY-STATE-free: it never touches the world, so the
//! resolution that DOES (token liveness, fire-mode lookup, the offer gate) lives in
//! [`resolve`](super::resolve) / [`inject`](super::inject). Only the pure wire→sim value
//! mappers ([`cell_level`] / [`direction`] / [`stance_kind`]) live here.

use bevy::input::keyboard::KeyCode;
use gdtf_battle_input::ActIntent;
use gdtf_battle_sim::prelude::{Cell, CellLevel, Direction, Level, StanceKind};
use gdtf_qa_protocol::{
    ids::{
        CellLevelNet, DoorToken, EmplacementToken, FireModeIndex, FocusTargetNet, GangerToken,
        PointerPosNet,
    },
    intent::{AimNet, FacingNet, KeyNet, KeyPressNet, MeleeTargetNet, NetIntent, StanceNet},
};

/// The exhaustive outcome of classifying a [`NetIntent`] — split by the resolution it
/// still needs (see the module doc).
pub(super) enum Classified {
    /// A self-contained classic intent, ready to push onto `PendingActIntent` verbatim.
    Classic(ActIntent),
    /// A classic act whose payload needs the current `SelectedShooter` actor.
    Actor(ActorIntent),
    /// A by-token actor pick — the [`GangerToken`] still to resolve to a live ganger.
    Select(GangerToken),
    /// One of the eight contextual acts — its wire target still to resolve + offer-gate.
    Contextual(ContextualIntent),
    /// A GTW-783 raw-input intent (keypress / hover / focus-set) — realised through the
    /// windowing-input path, NOT an act queue (see [`RawInputIntent`]).
    RawInput(RawInputIntent),
}

/// A GTW-783 raw-input intent still awaiting its windowing-input realisation — the keypress
/// key still to resolve (a physical key is pure, a bound action needs the live `Keybinds`),
/// the hover position, or the focus token still to resolve to a live entity. Handled by
/// [`push_raw_input`](super::inject::push_raw_input), NOT an act queue.
pub(super) enum RawInputIntent {
    /// Tap a key (→ a real `KeyboardInput` press+release pair).
    Key(KeyPressNet),
    /// Move the pointer to a window position (→ the primary window cursor + a `CursorMoved`).
    Hover(PointerPosNet),
    /// Point UI input focus at an entity (→ the `InputFocus` resource), token still to resolve.
    Focus(FocusTargetNet),
}

/// A classic act still awaiting the current `SelectedShooter` to finish building its
/// payload (the actor is implicit — the game's selection — per the GTW-694 injection
/// contract, so the wire carries only the parameters).
pub(super) enum ActorIntent {
    /// Step the selection toward a `(cell, storey)` (→ `ActIntent::Move`).
    Move(CellLevelNet),
    /// Turn the selection to a facing (→ `ActIntent::Turn`).
    Turn(FacingNet),
    /// Fire the selection's weapon in a mode at a `(cell, storey)` (→ `ActIntent::Fire`);
    /// the mode index still to resolve against the weapon's authored list.
    Fire {
        /// The `(cell, storey)` aimed at.
        target: CellLevelNet,
        /// Which authored fire mode to fire.
        mode:   FireModeIndex,
    },
    /// Set the selection's aim mode (→ `ActIntent::AimToggle`, mapped from the explicit
    /// wire aim flag against the actor's current aim).
    Aim(AimNet),
}

/// The per-act CONTEXTUAL registry (GTW-694 "per-act contextual registry") — one variant
/// per contextual act, carrying its still-to-resolve WIRE target. EXHAUSTIVE: adding a
/// contextual act forces a variant here AND a resolve+offer-gate arm in
/// [`push_contextual`](super::inject::push_contextual), so no contextual intent can be
/// silently dropped.
pub(super) enum ContextualIntent {
    /// Strike a ganger or smash a structure (`MeleeAct`).
    Melee(MeleeTargetNet),
    /// Shove an opposing ganger (`ShoveAct`).
    Shove(GangerToken),
    /// Arrest a downed ally's bleed-out (`StabilizeAct`).
    Stabilize(GangerToken),
    /// Coup-de-grâce a downed enemy (`ExecuteAct`).
    Execute(GangerToken),
    /// Lob a grenade at a cell (`ThrowGrenadeAct`).
    ThrowGrenade(CellLevelNet),
    /// Open a closed door (`OpenDoorAct`).
    OpenDoor(DoorToken),
    /// Man a vacant emplacement (`EnterEmplacementAct`).
    EnterEmplacement(EmplacementToken),
    /// Dismount the manned emplacement (`ExitEmplacementAct`).
    ExitEmplacement(EmplacementToken),
}

/// Classify a wire [`NetIntent`] into the local vocabulary — the wildcard-free
/// exhaustive `match` (GTW-737 clause 6).
///
/// Every one of the 21 `NetIntent` variants has an explicit arm; there is NO catch-all,
/// so a new wire variant fails to compile until it is classified here. The classic acts
/// map onto the [`ActIntent`] vocabulary the local keyboard / button surfaces push, the
/// contextual acts onto the [`ContextualIntent`] registry — the SAME two write-points
/// (never a raw `*Requested`), per the injection contract.
pub(super) const fn classify(intent: NetIntent) -> Classified {
    match intent {
        // Self-contained classic intents — pushed verbatim.
        NetIntent::SelectionClear => Classified::Classic(ActIntent::SelectionClear),
        NetIntent::LevelUp => Classified::Classic(ActIntent::LevelUp),
        NetIntent::LevelDown => Classified::Classic(ActIntent::LevelDown),
        NetIntent::Reload => Classified::Classic(ActIntent::Reload),
        NetIntent::EndTurn => Classified::Classic(ActIntent::EndTurn),
        NetIntent::SelectNext => Classified::Classic(ActIntent::SelectNext),
        NetIntent::SelectPrev => Classified::Classic(ActIntent::SelectPrev),
        NetIntent::SetStance { stance } => {
            Classified::Classic(ActIntent::SetStance(stance_kind(stance)))
        }
        // Classic acts needing the current selection to finish.
        NetIntent::Move { dest } => Classified::Actor(ActorIntent::Move(dest)),
        NetIntent::SetFacing { facing } => Classified::Actor(ActorIntent::Turn(facing)),
        NetIntent::SetAiming { aim } => Classified::Actor(ActorIntent::Aim(aim)),
        NetIntent::Fire { target, mode } => Classified::Actor(ActorIntent::Fire { target, mode }),
        // The by-token actor pick.
        NetIntent::Select { target } => Classified::Select(target),
        // The eight contextual acts — target still to resolve + offer-gate.
        NetIntent::Melee { target } => Classified::Contextual(ContextualIntent::Melee(target)),
        NetIntent::Shove { target } => Classified::Contextual(ContextualIntent::Shove(target)),
        NetIntent::Stabilize { target } => {
            Classified::Contextual(ContextualIntent::Stabilize(target))
        }
        NetIntent::Execute { target } => Classified::Contextual(ContextualIntent::Execute(target)),
        NetIntent::ThrowGrenade { target } => {
            Classified::Contextual(ContextualIntent::ThrowGrenade(target))
        }
        NetIntent::OpenDoor { target } => {
            Classified::Contextual(ContextualIntent::OpenDoor(target))
        }
        NetIntent::EnterEmplacement { target } => {
            Classified::Contextual(ContextualIntent::EnterEmplacement(target))
        }
        NetIntent::ExitEmplacement { target } => {
            Classified::Contextual(ContextualIntent::ExitEmplacement(target))
        }
        // The GTW-783 raw-input family — realised through the windowing-input path.
        NetIntent::PressKey { key } => Classified::RawInput(RawInputIntent::Key(key)),
        NetIntent::Hover { at } => Classified::RawInput(RawInputIntent::Hover(at)),
        NetIntent::SetFocus { target } => Classified::RawInput(RawInputIntent::Focus(target)),
    }
}

/// Map a wire [`CellLevelNet`] onto the sim [`CellLevel`] `(cell, storey)` key.
pub(super) fn cell_level(net: CellLevelNet) -> CellLevel {
    CellLevel::new(Cell::new(*net.cell.x, *net.cell.y), Level::new(*net.level))
}

/// Map a wire [`FacingNet`] onto the sim 8-way [`Direction`] (same ring order).
pub(super) const fn direction(net: FacingNet) -> Direction {
    match net {
        FacingNet::North => Direction::North,
        FacingNet::NorthEast => Direction::NorthEast,
        FacingNet::East => Direction::East,
        FacingNet::SouthEast => Direction::SouthEast,
        FacingNet::South => Direction::South,
        FacingNet::SouthWest => Direction::SouthWest,
        FacingNet::West => Direction::West,
        FacingNet::NorthWest => Direction::NorthWest,
    }
}

/// Map a wire [`StanceNet`] onto the sim [`StanceKind`].
pub(super) const fn stance_kind(net: StanceNet) -> StanceKind {
    match net {
        StanceNet::Standing => StanceKind::Standing,
        StanceNet::Crouching => StanceKind::Crouching,
        StanceNet::Prone => StanceKind::Prone,
    }
}

/// Map a wire [`KeyNet`] physical key onto the Bevy [`KeyCode`] the game reads — the pure,
/// state-free half of keypress resolution (a bound-action keypress needs the live
/// `Keybinds`, so it resolves in [`resolve`](super::resolve) instead).
pub(super) const fn key_code_of(key: KeyNet) -> KeyCode {
    match key {
        KeyNet::Escape => KeyCode::Escape,
        KeyNet::KeyQ => KeyCode::KeyQ,
        KeyNet::KeyE => KeyCode::KeyE,
        KeyNet::KeyC => KeyCode::KeyC,
        KeyNet::KeyF => KeyCode::KeyF,
        KeyNet::KeyR => KeyCode::KeyR,
        KeyNet::KeyV => KeyCode::KeyV,
        KeyNet::Tab => KeyCode::Tab,
        KeyNet::PageUp => KeyCode::PageUp,
        KeyNet::PageDown => KeyCode::PageDown,
        KeyNet::BracketLeft => KeyCode::BracketLeft,
        KeyNet::BracketRight => KeyCode::BracketRight,
        KeyNet::Digit1 => KeyCode::Digit1,
        KeyNet::Digit2 => KeyCode::Digit2,
        KeyNet::Digit3 => KeyCode::Digit3,
        KeyNet::Digit4 => KeyCode::Digit4,
        KeyNet::Digit5 => KeyCode::Digit5,
        KeyNet::Digit6 => KeyCode::Digit6,
        KeyNet::Digit7 => KeyCode::Digit7,
        KeyNet::Digit8 => KeyCode::Digit8,
        KeyNet::Digit9 => KeyCode::Digit9,
        KeyNet::ArrowUp => KeyCode::ArrowUp,
        KeyNet::ArrowDown => KeyCode::ArrowDown,
        KeyNet::ArrowLeft => KeyCode::ArrowLeft,
        KeyNet::ArrowRight => KeyCode::ArrowRight,
    }
}
