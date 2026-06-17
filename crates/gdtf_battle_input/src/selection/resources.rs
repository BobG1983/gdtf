//! Selection state (GTW-225 / GTW-227 / GTW-254): the [`SelectedShooter`] resource, the
//! [`WorldClickSuppressed`] click-through guard, and the [`SelectionHighlight`] sprite marker.

use bevy::prelude::*;

/// Whether a `gdtf_app`-owned modal (the GTW-254 fire-mode popup picker) is currently
/// capturing the pointer, so the WORLD click surfaces must NOT also act on this press.
///
/// A named bool newtype (no-bare-types: a suppression flag is a domain value; [`Resource`] is
/// the framework carve-out) [`Deref`]ing to its inner `bool`. The INPUT-owned half of the
/// cross-crate click-through seam (GTW-254 clause 6b): the world click decision READS it and
/// is inert while it is `true`, so a click on a picker button (or its scrim) does not ALSO
/// move/fire the ganger behind the modal. SET by `gdtf_app` while the picker is open and
/// CLEARED when it closes — `gdtf_app` depends on `gdtf_battle_input`, so the flag is OWNED
/// here and only written across the legal `gdtf_app -> gdtf_battle_input` edge (never a reverse
/// edge — the ADR-0001 / GTW-251 constraint). `init_resource`-d by
/// [`GdtfBattleInputPlugin`](crate::GdtfBattleInputPlugin) to the not-suppressed default
/// (`false`).
#[derive(Resource, Deref, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct WorldClickSuppressed(pub bool);

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
pub struct SelectedShooter(pub Option<Entity>);

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
