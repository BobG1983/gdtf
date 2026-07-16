//! [`AppFlowView`] — the top-level app-lifecycle snapshot (GTW-734).

use bevy_derive::Deref;
use serde::{Deserialize, Serialize};

/// The app's top-level lifecycle **state** — the wire mirror of the game `AppState`.
///
/// Tells a QA client where the app is in its lifecycle so it can wait for
/// [`Running`](Self::Running) before driving a battle. An independent serde enum
/// mirroring the five `AppState` variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AppStateNet {
    /// One-shot bootstrap before any scene runs (the game's `Init`).
    Init,
    /// Asset / scene loading hand-off (the game's `Load`).
    Load,
    /// Intro / splash scene (the game's `Intro`).
    Intro,
    /// The running game (the game's `Running`).
    Running,
    /// Final teardown before exit (the game's `Teardown`).
    Teardown,
}

/// Whether a battle is currently in progress — the wire mirror of the sim
/// `BattleInProgress` witness.
///
/// A private-inner newtype over `bool` (no-bare-types), serde-transparent: a QA client
/// gates injection / snapshot requests on this (a `NetIntent` off-battle is rejected
/// `NoBattle`).
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BattleActiveNet(bool);

impl BattleActiveNet {
    /// Build a battle-active answer — `true` when a battle is running.
    #[must_use]
    pub const fn new(active: bool) -> Self {
        Self(active)
    }
}

/// The app-flow **snapshot** — the lifecycle state plus whether a battle is running.
///
/// The reply to a [`GetAppFlow`](crate::envelope::QaRequest::GetAppFlow). Serde default
/// shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AppFlowView {
    /// The app's lifecycle state.
    pub state:         AppStateNet,
    /// Whether a battle is currently in progress.
    pub battle_active: BattleActiveNet,
}

impl AppFlowView {
    /// Build an app-flow snapshot from the lifecycle state and battle-active flag.
    #[must_use]
    pub const fn new(state: AppStateNet, battle_active: BattleActiveNet) -> Self {
        Self {
            state,
            battle_active,
        }
    }
}
