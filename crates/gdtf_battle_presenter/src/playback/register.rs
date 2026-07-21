//! [`register_playback`] — the playback cursor's registration (GTW-727 C15 / C18 / C36).

use bevy::prelude::*;

use super::{
    advance::advance_playback,
    cursor::PlaybackCursor,
    dwell::{PlaybackTuning, register_playback_tuning_hot_ron},
    emit::register_played_messages,
    seed::seed_drawn_state,
};
use crate::PresenterSystems;

/// Wire the playback cursor into a Bevy [`App`]: its two resources, every `Played<M>`
/// buffer, the dwell table's hot-reload chain, and the two `Replay`-stage systems.
///
/// ## Everything here is renderer-only, structurally
///
/// In production this is called ONLY from `TopDownRendererPlugin::build`. An app that does
/// not add that plugin — a headless sim test, an autobattle run, a focused harness —
/// registers no cursor, no dwell table, no `Played<M>` buffer and no `Drawn*` insertion.
/// There is no runtime flag to get wrong: the pacing simply does not exist there, which is
/// why the input gate cannot deadlock a presenter-less app.
///
/// It is `pub` so a focused test can wire the cursor WITHOUT the whole render stack (the
/// dwell chain self-gates on an `AssetServer`, so it is a no-op there and the shipped
/// defaults stand). That does not weaken the property above: what makes a presenter-less
/// app safe is that nothing there CALLS this.
///
/// The two resources are `init_resource`'d at PLUGIN BUILD, not on a state boundary, so
/// they live for the process. That is deliberate: it means no `gdtf_app` state boundary has
/// to own a presenter resource, and per-battle reset is self-healing off the act log's own
/// absence (`advance_playback` resets the cursor whenever there is no log).
pub fn register_playback(app: &mut App) {
    app.init_resource::<PlaybackCursor>()
        .init_resource::<PlaybackTuning>();
    register_played_messages(app);
    register_playback_tuning_hot_ron(app);
    app.add_systems(
        Update,
        (seed_drawn_state, advance_playback)
            .chain()
            .in_set(PresenterSystems::Replay),
    );
}
