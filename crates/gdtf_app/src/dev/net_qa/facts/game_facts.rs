//! [`GameFacts`] — the per-frame facts every GAME command reads (GTW-942).

use crate::dev::net_qa::wire::AppPhaseNet;

/// Everything about the game's current frame that a command's availability predicate — and
/// a handler that needs no world of its own — is allowed to key on.
///
/// ONE type for the whole host, sampled once a frame by
/// [`GameFactsParam`](super::GameFactsParam) and handed to BOTH the catalogue build and
/// the admission check, which is how "advertised" and "admitted" are computed from one
/// call per command and cannot disagree.
///
/// It is a plain value, not a `Resource` and not a `SystemParam`: a predicate takes
/// `&GameFacts` and is a pure function, unit-testable with no `App` at all. The one read
/// of the world happens in [`read`](super::read), inside the router.
///
/// It carries the app's phase and nothing else, because that is what the game's commands
/// read today. It is the place a further fact JOINS when the first command needs one — a
/// battle-reading command wants "is a battle running", an act-bearing one wants "has the
/// screen caught up" — so a predicate never reaches into the world for itself and the
/// once-a-frame sampling stays a single call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct GameFacts {
    /// Where the app is, at every level of its state machine.
    phase: AppPhaseNet,
}

impl GameFacts {
    /// Build the frame's facts.
    pub(crate) const fn new(phase: AppPhaseNet) -> Self {
        Self { phase }
    }

    /// Where the app is, at every level.
    pub(crate) const fn phase(self) -> AppPhaseNet {
        self.phase
    }
}
