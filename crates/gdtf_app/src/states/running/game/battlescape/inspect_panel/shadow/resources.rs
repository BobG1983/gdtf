//! The two cursor-time SHADOW resources the inspect panel reads (GTW-762):
//! [`ShownOccupancyGrid`] and [`ShownCoverLedger`], each HOLDING a snapshot of the sim's
//! live grid / ledger that is promoted only on a caught-up frame.
//!
//! The squad-fog shadow the inspect panel's fog gate reads lives in the presenter
//! ([`ShownSquadVisibility`](gdtf_battle_presenter::ShownSquadVisibility)), shared with
//! `present_fog`; only the occupancy grid and the cover ledger are owned here.

use bevy::prelude::*;
use gdtf_battle_sim::{cover::CoverLedger, prelude::OccupancyGrid};

/// The occupancy grid the inspect panel is CURRENTLY reading — a cursor-time snapshot of the
/// sim's live [`OccupancyGrid`] (GTW-762).
///
/// While closed-gate playback is in progress this stays FROZEN at its last promoted value,
/// so the inspect panel resolves the occupant / terrain the cursor has actually shown — not
/// a move the sim has already applied but the view has not yet played. The instant the
/// cursor catches up it is PROMOTED by
/// [`promote_shown_occupancy`](super::promote_shown_occupancy).
///
/// A presenter-style wrapper HOLDING a value the sim owns (the [`OccupancyGrid`]
/// precedent). Private inner + derived [`Deref`]; the named
/// [`grid`](ShownOccupancyGrid::grid) accessor is the read the panel uses. The [`Default`]
/// is the empty grid (all-Open, no occupants) — fail-safe until the first promote.
#[derive(Resource, Debug, Clone, Default, Deref)]
pub(crate) struct ShownOccupancyGrid(OccupancyGrid);

impl ShownOccupancyGrid {
    /// Overwrite the shadow with a fresh clone of the live occupancy grid — the PROMOTE
    /// step, run only on a caught-up frame.
    pub(crate) fn promote(&mut self, live: &OccupancyGrid) {
        self.0 = live.clone();
    }

    /// The snapshotted occupancy grid — the value the inspect panel reads in place of the
    /// live resource.
    #[must_use]
    pub(crate) const fn grid(&self) -> &OccupancyGrid {
        &self.0
    }
}

/// The cover ledger the inspect panel is CURRENTLY reading — a cursor-time snapshot of the
/// sim's live [`CoverLedger`] (GTW-762).
///
/// Frozen during closed-gate playback and PROMOTED the instant the cursor catches up (by
/// [`promote_shown_cover`](super::promote_shown_cover)), so a hovered object's structural
/// stats reflect the cursor's playback position, not a hit the view has not yet played.
///
/// A wrapper HOLDING the sim-owned [`CoverLedger`]. Private inner + derived [`Deref`]; the
/// named [`ledger`](ShownCoverLedger::ledger) accessor is the read the panel uses. The
/// [`Default`] is the empty ledger (every piece at full HP by lazy-seed).
#[derive(Resource, Debug, Clone, Default, Deref)]
pub(crate) struct ShownCoverLedger(CoverLedger);

impl ShownCoverLedger {
    /// Overwrite the shadow with a fresh clone of the live cover ledger — the PROMOTE step,
    /// run only on a caught-up frame.
    pub(crate) fn promote(&mut self, live: &CoverLedger) {
        self.0 = live.clone();
    }

    /// The snapshotted cover ledger — the value the inspect panel reads in place of the live
    /// resource.
    #[must_use]
    pub(crate) const fn ledger(&self) -> &CoverLedger {
        &self.0
    }
}
