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
pub struct ActiveLevel(Level);

impl ActiveLevel {
    /// Build the active-storey resource from the [`Level`] the terrain draw renders.
    #[must_use]
    pub const fn new(level: Level) -> Self {
        Self(level)
    }

    /// Whether `storey` lies WITHIN the drawn band — the ground floor up to and including
    /// the active view level (`0..=active`) — the ONE shared band predicate (GTW-520 C4).
    ///
    /// The single successor to the pre-GTW-520 on-active-storey hard cut (`storey ==
    /// active`): a ganger on ANY storey at or below the active view level is now DRAWN (it
    /// peeks through floor-gaps on the lower storeys the terrain draw already renders,
    /// GTW-519), while a ganger strictly ABOVE the active level is culled. It is the SAME
    /// membership the terrain draw's `drawn_band` `0..=active` range expresses, as a
    /// per-storey predicate — so the four ganger-visibility sites
    /// ([`spawn_ganger_sprites`](crate::spawn_ganger_sprites) /
    /// [`move_ganger_sprites`](crate::move_ganger_sprites) /
    /// [`apply_active_level_filter`](crate::apply_active_level_filter) / the fog writer's
    /// `present_actor_fog`) all consult ONE predicate and cannot drift from each other OR from
    /// the terrain band.
    ///
    /// `storey` reads through [`Level`]'s `Deref<Target = u8>`, as does the wrapped active
    /// level; the comparison is on the `u8` storey indices. GTW-521's full-view toggle can
    /// widen this ONE predicate (to the whole occupied stack) without re-touching any of the
    /// four sites.
    #[must_use]
    pub fn draws_storey(&self, storey: Level) -> bool {
        // `self` Derefs `ActiveLevel -> Level -> u8`; `storey` Derefs `Level -> u8`.
        *storey <= ***self
    }
}

impl Default for ActiveLevel {
    /// The default active storey: the ground floor (level 0).
    fn default() -> Self {
        Self::new(Level::new(0))
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
