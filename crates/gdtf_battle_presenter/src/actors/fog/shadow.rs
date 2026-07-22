//! The cursor-time squad-fog SHADOW (GTW-762): [`ShownSquadVisibility`] holds the
//! [`SquadVisibility`] the presenter is CURRENTLY SHOWING, and [`promote_shown_fog`] keeps
//! it frozen while closed-gate playback runs, promoting it to the live fog the instant the
//! cursor catches up.
//!
//! ## Why a shadow resource, and not a `Drawn*` component (GTW-727 C30)
//!
//! The [`Drawn*`](crate::DrawnPosition) mirrors restore per-entity change detection for a
//! ganger's shown state. Fog is not per-entity: [`SquadVisibility`] is one grid-shaped
//! [`Resource`] the sim replaces wholesale each recompute, so there is no entity to hang a
//! mirror on. The cursor-time equivalent is one resource that HOLDS a snapshot and is
//! overwritten (PROMOTED) only on a caught-up frame — the terrain fog then reads the shadow
//! instead of the live fog, so it shows what the cursor's playback position shows, never a
//! reveal that has not been played yet.
//!
//! ## The ganger-sprite arm stays LIVE, deliberately
//!
//! Only the TERRAIN fog reads this shadow. The ganger-sprite visibility resolver
//! ([`resolve_ganger_visibility`](crate::resolve_ganger_visibility)) keeps reading the LIVE
//! [`SquadVisibility`] — see the exemption documented on that system.

use bevy::prelude::*;
use gdtf_battle_sim::visibility::SquadVisibility;

use crate::playback::PlaybackGate;

/// The squad fog the presenter is CURRENTLY SHOWING — a cursor-time snapshot of the sim's
/// live [`SquadVisibility`] (GTW-762).
///
/// While closed-gate playback is in progress (the cursor is behind the act log) this stays
/// FROZEN at its last promoted value, so [`present_fog`](super::present_fog) draws the
/// terrain fog for the cursor's playback position — not for a reveal the sim has already
/// reached but the view has not yet played. The instant the cursor catches up it is
/// PROMOTED (overwritten with a fresh clone of the live fog) by [`promote_shown_fog`].
///
/// The precedent it follows is [`DrawnVitals`](crate::DrawnVitals): a presenter-owned
/// wrapper HOLDING a value the sim owns, written only by the playback machinery. Private
/// inner + derived [`Deref`]; the named [`visibility`](ShownSquadVisibility::visibility)
/// accessor is the read the fog uses. The [`Default`] is the empty fog (everything UNSEEN)
/// — fail-closed until the first promote.
#[derive(Resource, Debug, Clone, Default, Deref)]
pub struct ShownSquadVisibility(SquadVisibility);

impl ShownSquadVisibility {
    /// Overwrite the shadow with a fresh clone of the live squad fog — the PROMOTE step,
    /// run only on a caught-up frame by [`promote_shown_fog`].
    pub fn promote(&mut self, live: &SquadVisibility) {
        self.0 = live.clone();
    }

    /// The snapshotted squad fog — the value [`present_fog`](super::present_fog) and the
    /// inspect panel read in place of the live resource.
    #[must_use]
    pub const fn visibility(&self) -> &SquadVisibility {
        &self.0
    }
}

/// `Update` ([`PresenterSystems::Compose`](crate::PresenterSystems), ordered `.before`
/// [`present_fog`](super::present_fog)): PROMOTE the squad-fog shadow to the live fog
/// whenever the playback cursor is caught up, leaving it FROZEN at its last value while
/// closed-gate playback is in progress (GTW-762).
///
/// The caught-up test is the ONE predicate the input gate keys on
/// ([`PlaybackGate::is_open`]): the shadow tracks live exactly when the player may act on
/// what is shown, and stops tracking exactly while the view is still catching up. It reads
/// the live [`SquadVisibility`] (guarded by the sibling registration's
/// `resource_exists::<SquadVisibility>` run condition), so a frame without a battle fog
/// never reaches it.
pub fn promote_shown_fog(
    live: Res<SquadVisibility>,
    mut shadow: ResMut<ShownSquadVisibility>,
    playback: PlaybackGate,
) {
    if playback.is_open() {
        shadow.promote(&live);
    }
}
