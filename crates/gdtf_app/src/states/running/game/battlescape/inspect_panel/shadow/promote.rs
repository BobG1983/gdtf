//! The two promote systems (GTW-762) that keep the inspect panel's grid / ledger shadows
//! frozen during closed-gate playback and refresh them the instant the cursor catches up.
//!
//! Both key on the ONE catch-up predicate the input gate uses
//! ([`PlaybackGate::is_open`](gdtf_battle_presenter::PlaybackGate)) — the same crate-boundary
//! read `action_bar` / `contextual_panel` / `weapon_panel` already consume — so the shadow
//! tracks live exactly when the player may act on what is shown, and freezes exactly while
//! the view is still catching up.

use bevy::prelude::*;
use gdtf_battle_presenter::PlaybackGate;
use gdtf_battle_sim::{cover::CoverLedger, prelude::OccupancyGrid};

use super::resources::{ShownCoverLedger, ShownOccupancyGrid};

/// `Update`: PROMOTE the occupancy-grid shadow to the live grid whenever the playback cursor
/// is caught up, leaving it FROZEN while closed-gate playback is in progress (GTW-762).
///
/// The clone is gated on the live grid's own change detection COMBINED with the catch-up
/// state (GTW-762 clause c) — the grid is a large flat buffer, so an idle caught-up frame
/// where nothing changed skips the clone. `was_caught_up` (a [`Local`]) tracks the previous
/// frame's catch-up state so the promote ALSO fires on the catch-up EDGE: during closed-gate
/// playback the sim is idle, so the live grid is not `is_changed()` on the frame the cursor
/// finally catches up — the edge is what makes that frame promote anyway.
pub(crate) fn promote_shown_occupancy(
    live: Option<Res<OccupancyGrid>>,
    mut shadow: ResMut<ShownOccupancyGrid>,
    playback: PlaybackGate,
    mut was_caught_up: Local<bool>,
) {
    let caught_up = playback.is_open();
    let just_caught_up = caught_up && !*was_caught_up;
    *was_caught_up = caught_up;
    let Some(live) = live else {
        return;
    };
    if caught_up && (live.is_changed() || just_caught_up) {
        shadow.promote(&live);
    }
}

/// `Update`: PROMOTE the cover-ledger shadow to the live ledger whenever the playback cursor
/// is caught up, leaving it FROZEN while closed-gate playback is in progress (GTW-762).
///
/// The ledger is a small, lazily-populated map, so — unlike the occupancy grid — this
/// promotes on EVERY caught-up frame without a change-detection guard; the clone is cheap
/// and always tracking live is the simpler correct behaviour.
pub(crate) fn promote_shown_cover(
    live: Option<Res<CoverLedger>>,
    mut shadow: ResMut<ShownCoverLedger>,
    playback: PlaybackGate,
) {
    if playback.is_open()
        && let Some(live) = live
    {
        shadow.promote(&live);
    }
}
