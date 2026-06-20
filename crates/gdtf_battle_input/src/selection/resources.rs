//! Selection state (GTW-225 / GTW-227): the [`SelectedShooter`] resource and the
//! [`SelectionHighlight`] sprite marker.

use bevy::prelude::*;

/// The currently SELECTED shooter — the ganger a left-click picked, or `None`.
///
/// A named newtype over `Option<Entity>` (no-bare-types: the selection is a domain value;
/// [`Entity`] is the framework carve-out) that [`Deref`]s to its inner [`Option`] so a reader
/// matches it directly. `init_resource`-d by
/// [`GdtfBattleInputPlugin`](crate::GdtfBattleInputPlugin) — so its [`Default`] is the empty
/// selection (`None`). [`left_click_act`](crate::left_click_act) writes it; the
/// [`update_selection_highlight`](crate::update_selection_highlight) sprite + the act surfaces
/// read it.
#[derive(Resource, Deref, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SelectedShooter(Option<Entity>);

impl SelectedShooter {
    /// Build a selection holding `entity`.
    #[must_use]
    pub const fn new(entity: Entity) -> Self {
        Self(Some(entity))
    }

    /// The empty (no-ganger) selection — what the CLEAR branch of
    /// [`left_click_act`](crate::left_click_act), or a
    /// [`SelectionClear`](crate::ActIntent::SelectionClear) intent, sets.
    #[must_use]
    pub const fn cleared() -> Self {
        Self(None)
    }
}

/// Marker for the single selection-highlight [`Sprite`].
///
/// Plumbing around the framework sprite (the no-bare-types framework carve-out, the same
/// justification the presenter's `WorldCamera` marker uses):
/// [`update_selection_highlight`](crate::update_selection_highlight) queries
/// `With<SelectionHighlight>` to find and MOVE the one existing highlight rather than spawning a
/// duplicate each update. Distinct from the hover highlight so the two reticles coexist.
#[derive(Component, Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct SelectionHighlight;

/// The translucent tint of the selection-highlight sprite.
///
/// Framework plumbing — a literal [`Color`] handed straight to a [`Sprite`], not a domain
/// quantity (the `CELL_PX`-class const carve-out). A cooler cyan at moderate alpha so the
/// SELECTED cell reads distinctly from the warm-white hover reticle when both are on the same
/// cell.
pub(super) const SELECTION_TINT: Color = Color::srgba(0.4, 0.85, 1.0, 0.5);

/// Writes `next` into `selected` only on a real change (change-detection hygiene), so a no-op
/// CLEAR/SELECT does not spuriously trip `Changed<SelectedShooter>` (which
/// `sync_fire_mode_on_select` keys off).
pub(super) fn set_selection(selected: &mut ResMut<SelectedShooter>, next: SelectedShooter) {
    if **selected != next {
        **selected = next;
    }
}

/// Widens a `usize` grid extent to `i32` for the selection-highlight scan bounds.
///
/// `as i32` on a `usize` trips `cast_possible_wrap` (`-D`); [`i32::try_from`] is the
/// no-`unwrap` cast. An unrepresentable extent saturates to [`i32::MAX`] (only widens the scan,
/// never narrows it).
pub(super) fn grid_extent_i32(extent: usize) -> i32 {
    i32::try_from(extent).unwrap_or(i32::MAX)
}
