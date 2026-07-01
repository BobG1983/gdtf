//! The presenter-owned active storey resource and the shared draw-ordering set.

use bevy::prelude::*;
use gdtf_battle_sim::{Level, MAX_LEVELS};

/// The presenter-owned VIEW MODE — how the terrain draw + ganger visibility bound the
/// drawn storey stack from ABOVE (GTW-521, the UFO "full view" toggle).
///
/// A named domain enum (no-bare-types: the two view modes are a named presentation choice,
/// NOT a bare `bool`), so it reads at every site as [`DownToActive`](Self::DownToActive) /
/// [`FullView`](Self::FullView) rather than a nameless flag. OWNED BY THE PRESENTER CRATE so
/// the `input -> presenter -> sim` direction holds exactly as [`ActiveLevel`] does: the
/// terrain draw + the ganger visibility filter READ it (through the shared
/// [`drawn_band`](super::draw::drawn_band) band helper); the input crate's
/// `ToggleFullView` intent — in `gdtf_battle_input`, which depends on the presenter —
/// MUTATES it. `init_resource`-d by the
/// [`TopDownRendererPlugin`](crate::TopDownRendererPlugin) on build (its [`Default`] is
/// [`DownToActive`](Self::DownToActive) — exactly the current GTW-519/520 behaviour) so the
/// later input toggle has a resource to flip.
///
/// The two modes ONLY differ in the UPPER bound the [`drawn_band`](super::draw::drawn_band) helper
/// returns; every other draw / visibility rule (per-storey Z occlusion, the peek-through
/// floor-gap reveal, the GTW-520 fog hard-cut) is untouched.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ViewMode {
    /// Draw the storey stack from the ground floor UP TO AND INCLUDING the active view
    /// level (`0..=active`) and CULL everything strictly above it — the DEFAULT, exactly the
    /// GTW-519/520 behaviour.
    #[default]
    DownToActive,
    /// Draw ALL storeys `0..=MAX_LEVELS - 1` regardless of the active level — the UFO
    /// full-stack view. The upper roofs / floors re-appear and hide the storeys / units
    /// beneath them by per-storey Z; empty upper cells still emit nothing (peek-through), so
    /// it stays cheap.
    FullView,
}

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

    /// Whether `storey` lies WITHIN the drawn band under `view` — the ONE shared band
    /// predicate (GTW-520 C4, widened for the GTW-521 view toggle).
    ///
    /// The single successor to the pre-GTW-520 on-active-storey hard cut (`storey ==
    /// active`): a ganger on ANY storey within the drawn band is DRAWN (it peeks through
    /// floor-gaps on the lower storeys the terrain draw already renders, GTW-519), while a
    /// ganger strictly ABOVE the band ceiling is culled. It is the SAME membership the
    /// terrain draw's [`drawn_band`](super::draw::drawn_band) range expresses, as a per-storey
    /// predicate — so the four ganger-visibility sites
    /// ([`spawn_ganger_sprites`](crate::spawn_ganger_sprites) /
    /// [`move_ganger_sprites`](crate::move_ganger_sprites) /
    /// [`apply_active_level_filter`](crate::apply_active_level_filter) / the fog writer's
    /// `present_actor_fog`) all consult ONE predicate and cannot drift from each other OR from
    /// the terrain band.
    ///
    /// The `view` chooses the CEILING exactly as [`drawn_band`](super::draw::drawn_band) does
    /// (GTW-521 C2 — units on all storeys are shown in [`ViewMode::FullView`], still subject
    /// to the GTW-520 fog hard-cut applied downstream):
    ///
    /// - [`ViewMode::DownToActive`] (default) → `storey <= active` (unchanged GTW-520).
    /// - [`ViewMode::FullView`] → every storey `0..=MAX_LEVELS - 1` is drawn.
    ///
    /// `storey` reads through [`Level`]'s `Deref<Target = u8>`, as does the wrapped active
    /// level; the comparison is on the `u8` storey indices.
    #[must_use]
    pub fn draws_storey(&self, storey: Level, view: ViewMode) -> bool {
        // `self` Derefs `ActiveLevel -> Level -> u8`; `storey` Derefs `Level -> u8`. The
        // ceiling is the active level (DownToActive) or the top storey (FullView), matching
        // `drawn_band`.
        let ceiling = match view {
            ViewMode::DownToActive => ***self,
            ViewMode::FullView => MAX_LEVELS.saturating_sub(1),
        };
        *storey <= ceiling
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
