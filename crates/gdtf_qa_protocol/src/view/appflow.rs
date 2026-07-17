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

/// A request **kind** — the discriminant of a [`QaRequest`](crate::envelope::QaRequest)
/// without its payload.
///
/// One variant per `QaRequest` variant, kept in lock-step with it by
/// [`QaRequest::kind`](crate::envelope::QaRequest::kind) (a wildcard-free map that fails
/// to compile until a newly-added request grows a matching kind). This is the vocabulary
/// [`AppFlowView::available`] advertises: the request kinds the server will service in the
/// current app state, so a QA client can read "where am I, what can I send you" from one
/// snapshot instead of probing each request. An independent serde enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RequestKindNet {
    /// The [`Hello`](crate::envelope::QaRequest::Hello) handshake.
    Hello,
    /// The [`GetAppFlow`](crate::envelope::QaRequest::GetAppFlow) lifecycle read.
    GetAppFlow,
    /// The [`GetBattleState`](crate::envelope::QaRequest::GetBattleState) snapshot read.
    GetBattleState,
    /// An [`Inject`](crate::envelope::QaRequest::Inject) of one battle intent.
    Inject,
    /// A [`TakeScreenshot`](crate::envelope::QaRequest::TakeScreenshot) capture.
    TakeScreenshot,
    /// A [`GetOutput`](crate::envelope::QaRequest::GetOutput) combat-event drain.
    GetOutput,
    /// A [`StartBattle`](crate::envelope::QaRequest::StartBattle) navigation.
    StartBattle,
}

impl RequestKindNet {
    /// Every request kind, in [`QaRequest`](crate::envelope::QaRequest) declaration order.
    ///
    /// The canonical list the game server filters to build [`AppFlowView::available`], and
    /// the list the round-trip suite walks to prove each kind round-trips. The per-variant
    /// round-trip witness keeps this array complete — a new kind that is not listed here
    /// fails that test.
    pub const ALL: [Self; 7] = [
        Self::Hello,
        Self::GetAppFlow,
        Self::GetBattleState,
        Self::Inject,
        Self::TakeScreenshot,
        Self::GetOutput,
        Self::StartBattle,
    ];
}

/// The app-flow **snapshot** — the lifecycle state, whether a battle is running, and the
/// requests the server will service right now.
///
/// The reply to a [`GetAppFlow`](crate::envelope::QaRequest::GetAppFlow). A QA client reads
/// [`available`](Self::available) to learn which requests are valid in the current state
/// (the trio of battle-only reads is absent until a battle is running); it is meant to be
/// polled first, before acting on what it lists. Serde default shape.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AppFlowView {
    /// The app's lifecycle state.
    pub state:         AppStateNet,
    /// Whether a battle is currently in progress.
    pub battle_active: BattleActiveNet,
    /// The request kinds the server will service in this state — the affordances a QA
    /// client can act on right now.
    pub available:     Vec<RequestKindNet>,
}

impl AppFlowView {
    /// Build an app-flow snapshot from the lifecycle state, the battle-active flag, and the
    /// request kinds serviceable right now.
    #[must_use]
    pub const fn new(
        state: AppStateNet,
        battle_active: BattleActiveNet,
        available: Vec<RequestKindNet>,
    ) -> Self {
        Self {
            state,
            battle_active,
            available,
        }
    }
}
