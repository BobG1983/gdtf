//! The presenter-owned active storey resource and the shared draw-ordering set.

use bevy::prelude::*;
use gdtf_battle_sim::Level;

/// The presenter-owned ACTIVE storey — the single [`Level`] the terrain draw renders.
///
/// A named newtype over [`Level`] (no-bare-types) that [`Deref`]s to it. OWNED BY THE
/// PRESENTER CRATE so the `input -> presenter -> sim` direction holds: S5's ganger draw
/// READS it; S8's level-cycling input (in the input crate, which depends on the
/// presenter) MUTATES it. Present for the whole battle span (the
/// [`TopDownRendererPlugin`](crate::TopDownRendererPlugin) inserts the
/// [`Default`] — level 0 — on build) so the later input slice has a resource to mutate.
#[derive(Resource, Deref, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ActiveLevel(pub Level);

impl Default for ActiveLevel {
    /// The default active storey: the ground floor (level 0).
    fn default() -> Self {
        Self(Level::new(0))
    }
}

/// Presenter draw ordering anchor — the named `Update`-schedule band the terrain draw
/// (and S5's ganger draw) registers into.
///
/// Defined ONCE via `configure_sets` (`bevy-traps.md` #5), then `.in_set`. The draw
/// band is ordered `.after(SimSystems::Simulate)` so a draw reads the sim AFTER its
/// world mutations this update (`bevy-traps.md` #3). This is the shared anchor S5's
/// ganger draw also registers into; S4 (the slice that lands first) introduces it.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PresenterSystems {
    /// The band holding the presenter's per-`(cell, level)` draw systems — ordered
    /// after the sim's world mutations so it observes a settled sim state.
    Draw,
}
