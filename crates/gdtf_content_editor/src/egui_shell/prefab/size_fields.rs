//! The PREFAB-mode grid-SIZE fields (GTW-515 C4.6) + their explicit VIEW MODEL (GTW-464).
//!
//! ## The two-way sync, modelled (GTW-464)
//!
//! The right-panel W / H / Levels [`egui::DragValue`]s are driven by [`SizeFieldSpans`] — the
//! per-pass view model of what the three fields DISPLAY:
//!
//! - **session → fields (C1, the reverse sync)**: [`SizeFieldSpans::from_session`] derives the
//!   displayed spans FRESH from [`MapEditorSession::grid_size`] on EVERY egui pass — nothing is
//!   buffered crate-side across frames, so a PROGRAMMATIC session change (the dev-capture drive's
//!   `drive_capture_grid_size` today; loaded situation/prefab data later per GTW-433) is displayed
//!   on the next pass instead of a stale seeded `60/60/8`. A pure read is trivially idempotent
//!   under the egui multipass re-run (bevy-traps #8).
//! - **fields → session (the kept forward commit)**: [`SizeFieldSpans::commit`] folds the edited
//!   spans through [`clamp_to_grid_size`] into [`MapEditorSession::set_grid_size`] and re-clamps
//!   the [`CurrentEditLevel`] (the kept level-nav clamp). The draw fires it only on a
//!   [`changed`](bevy_egui::egui::Response::changed) response — a real edit event, never a
//!   per-pass side effect — and the commit is set-to-target, so re-committing an identical value
//!   is a no-op.
//!
//! ## Mid-edit safety (C2) — egui-owned, not modelled here
//!
//! While a [`DragValue`](egui::DragValue) has keyboard focus, egui displays its OWN temp edit
//! `String` (stored in egui memory keyed by the widget id — egui 0.35 `drag_value.rs`, the
//! `is_kb_editing` branch re-inserts it every pass) and only re-formats the fed-in value once
//! focus is gone. Feeding the fresh session-derived span each pass therefore never clobbers an
//! in-progress edit — there is no crate-side in-focus buffer to sync or to fight.

use bevy_egui::egui;
use gdtf_battle_sim::level::{GridHeight, GridLevels, GridSize, GridWidth, MAX_GRID_SPAN};

use crate::{canvas::CurrentEditLevel, right_panel::GridSpanInput, session::MapEditorSession};

/// The inclusive width/height [`DragValue`](egui::DragValue) range — the sim's fixed
/// `1..=`[`MAX_GRID_SPAN`] system
/// constant (the ticket fixes this bound, so it is a system constant, not a tunable).
const SPAN_RANGE: core::ops::RangeInclusive<u8> = 1..=MAX_GRID_SPAN;

/// The inclusive levels [`DragValue`](egui::DragValue) range — the sim's fixed `1..=`[`MAX_LEVELS`]
/// system constant.
///
/// [`MAX_LEVELS`]: gdtf_battle_sim::metric::MAX_LEVELS
const LEVELS_RANGE: core::ops::RangeInclusive<u8> = 1..=gdtf_battle_sim::metric::MAX_LEVELS;

/// The three axis spans the right-panel W / H / Levels size fields DISPLAY and EDIT this pass —
/// the size fields' VIEW MODEL (GTW-464).
///
/// Derived FRESH from the session every egui pass via [`from_session`](SizeFieldSpans::from_session)
/// (the session → fields reverse sync, C1) and folded back through
/// [`commit`](SizeFieldSpans::commit) only on a real edit (the kept fields → session forward path).
/// Because the model is re-derived per pass and never stored crate-side, a programmatic
/// [`MapEditorSession::set_grid_size`] shows up on the next pass, and the derivation is idempotent
/// under the egui multipass re-run (bevy-traps #8). Each span is a [`GridSpanInput`]
/// (no-bare-types — the kept GTW-421 field-value newtype).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SizeFieldSpans {
    /// The displayed/edited width span (x cells).
    width:  GridSpanInput,
    /// The displayed/edited height span (y cells).
    height: GridSpanInput,
    /// The displayed/edited storey count (z levels).
    levels: GridSpanInput,
}

impl SizeFieldSpans {
    /// Build the model from three (possibly user-edited) spans — the draw reassembles the model
    /// from the [`DragValue`](egui::DragValue)-edited locals before a [`commit`](Self::commit).
    #[must_use]
    pub const fn new(width: GridSpanInput, height: GridSpanInput, levels: GridSpanInput) -> Self {
        Self {
            width,
            height,
            levels,
        }
    }

    /// Derive the spans the size fields DISPLAY from the session's current
    /// [`grid_size`](MapEditorSession::grid_size) — the session → fields reverse sync (GTW-464 C1).
    ///
    /// Called on every egui pass, so a programmatic session change (dev-capture, loaded prefab
    /// data) is reflected on the next pass; a pure read, so multipass-idempotent (bevy-traps #8).
    #[must_use]
    pub fn from_session(session: &MapEditorSession) -> Self {
        let size = session.grid_size();
        Self::new(
            GridSpanInput::new(*size.width()),
            GridSpanInput::new(*size.height()),
            GridSpanInput::new(*size.levels()),
        )
    }

    /// The displayed/edited width span.
    #[must_use]
    pub const fn width(&self) -> GridSpanInput {
        self.width
    }

    /// The displayed/edited height span.
    #[must_use]
    pub const fn height(&self) -> GridSpanInput {
        self.height
    }

    /// The displayed/edited storey count.
    #[must_use]
    pub const fn levels(&self) -> GridSpanInput {
        self.levels
    }

    /// Fold the (edited) spans back into the session — the KEPT fields → session forward commit
    /// (GTW-515 C4.6, preserved by GTW-464 C1).
    ///
    /// Clamps through `clamp_to_grid_size` into [`MapEditorSession::set_grid_size`], then
    /// re-clamps the [`CurrentEditLevel`] so a shrunk volume never leaves it pointing past the new
    /// extent (the kept level-nav clamp — [`CurrentEditLevel::clamped`]). Set-to-target: committing
    /// spans equal to the session's current size changes nothing.
    pub fn commit(self, session: &mut MapEditorSession, edit_level: &mut CurrentEditLevel) {
        let new_size =
            clamp_to_grid_size(*self.width, *self.height, *self.levels, session.grid_size());
        session.set_grid_size(new_size);
        *edit_level = edit_level.clamped(new_size);
    }
}

/// Clamp three raw axis spans into a validated [`GridSize`] (GTW-515 C4.6) — the PURE size-field
/// clamp the unit test exercises on the real path (C4.13).
///
/// Each axis is clamped to its fixed system-constant band (`1..=`[`MAX_GRID_SPAN`] on x/y,
/// `1..=`[`MAX_LEVELS`](gdtf_battle_sim::metric::MAX_LEVELS) on z), wrapped in its [`GridWidth`] /
/// [`GridHeight`] / [`GridLevels`]
/// newtype (no bare numeric), and validated through [`GridSize::new`]. Because every axis is
/// pre-clamped into range, `GridSize::new` never rejects — but a defensive fall-back to `previous`
/// keeps the function total + panic-free if the sim's bounds ever tighten below the clamp.
#[must_use]
pub(crate) fn clamp_to_grid_size(w: u8, h: u8, levels: u8, previous: GridSize) -> GridSize {
    let width = GridWidth::new(w.clamp(*SPAN_RANGE.start(), *SPAN_RANGE.end()));
    let height = GridHeight::new(h.clamp(*SPAN_RANGE.start(), *SPAN_RANGE.end()));
    let levels = GridLevels::new(levels.clamp(*LEVELS_RANGE.start(), *LEVELS_RANGE.end()));
    GridSize::new(width, height, levels).unwrap_or(previous)
}

/// The W / H / levels size fields (GTW-515 C4.6) — three [`egui::DragValue`]s clamped to the sim
/// bands, displaying the per-pass [`SizeFieldSpans`] view model (the session → fields reverse sync,
/// GTW-464 C1) and committing edits through [`SizeFieldSpans::commit`] (the kept forward path),
/// with the edit level re-clamped on change (the kept clamp).
pub(crate) fn size_fields(
    ui: &mut egui::Ui,
    session: &mut MapEditorSession,
    edit_level: &mut CurrentEditLevel,
) {
    // Session → fields: re-derive the displayed spans from the session THIS pass (C1). The locals
    // below feed the DragValues; an in-focus field ignores the fed value in favor of egui's own
    // temp edit buffer, so a mid-edit programmatic change is never clobbering (C2).
    let spans = SizeFieldSpans::from_session(session);
    let mut w = *spans.width();
    let mut h = *spans.height();
    let mut levels = *spans.levels();
    let mut changed = false;

    ui.label("Grid size (cells)");
    ui.horizontal(|ui| {
        ui.label("W");
        changed |= ui
            .add(egui::DragValue::new(&mut w).range(SPAN_RANGE))
            .changed();
        ui.label("H");
        changed |= ui
            .add(egui::DragValue::new(&mut h).range(SPAN_RANGE))
            .changed();
        ui.label("Levels");
        changed |= ui
            .add(egui::DragValue::new(&mut levels).range(LEVELS_RANGE))
            .changed();
    });

    // Fields → session: commit only on a REAL edit response (never a per-pass side effect —
    // multipass-idempotent, bevy-traps #8).
    if changed {
        SizeFieldSpans::new(
            GridSpanInput::new(w),
            GridSpanInput::new(h),
            GridSpanInput::new(levels),
        )
        .commit(session, edit_level);
    }
}

#[cfg(test)]
mod tests {
    use gdtf_battle_sim::level::{
        GridHeight, GridLevels, GridSize, GridWidth, MAX_GRID_SPAN, ThemeUuid,
    };

    use super::{SizeFieldSpans, clamp_to_grid_size};
    use crate::session::MapEditorSession;

    /// C4.13 — the size-field clamp folds an over-max width/height to `MAX_GRID_SPAN` and an
    /// over-max level count to `MAX_LEVELS` (the ticket fixes these bounds, so pinning THEM is
    /// allowed — C4.13). A zero span clamps UP to 1 (never the empty grid `GridSize::new` rejects).
    #[test]
    fn clamp_folds_into_the_system_constant_bands() {
        let previous = GridSize::default();
        // Over-max on every axis → each axis saturates at its ceiling.
        let clamped = clamp_to_grid_size(200, 200, 99, previous);
        assert_eq!(
            *clamped.width(),
            MAX_GRID_SPAN,
            "width clamps to MAX_GRID_SPAN (60)"
        );
        assert_eq!(
            *clamped.height(),
            MAX_GRID_SPAN,
            "height clamps to MAX_GRID_SPAN (60)"
        );
        assert_eq!(
            *clamped.levels(),
            gdtf_battle_sim::metric::MAX_LEVELS,
            "levels clamps to MAX_LEVELS (8)",
        );
        // Zero span → clamps UP to 1 on every axis (a valid 1x1x1, not the rejected empty grid).
        let floored = clamp_to_grid_size(0, 0, 0, previous);
        assert_eq!(*floored.width(), 1, "zero width clamps up to 1");
        assert_eq!(*floored.height(), 1, "zero height clamps up to 1");
        assert_eq!(*floored.levels(), 1, "zero levels clamps up to 1");
    }

    /// C4.13 — an in-range size passes through unchanged (the clamp only bites at the bounds).
    #[test]
    fn clamp_passes_in_range_sizes_through() {
        let previous = GridSize::default();
        let clamped = clamp_to_grid_size(16, 24, 3, previous);
        assert_eq!(*clamped.width(), 16);
        assert_eq!(*clamped.height(), 24);
        assert_eq!(*clamped.levels(), 3);
    }

    /// GTW-464 C1 (unit) — the view model mirrors the session's grid size EACH derivation: after a
    /// programmatic [`MapEditorSession::set_grid_size`] (the same call the dev-capture drive
    /// makes), `from_session` yields the NEW spans, not the seed's. Nothing is buffered across
    /// derivations — the reverse sync is the derivation itself.
    #[test]
    fn from_session_tracks_a_programmatic_grid_change() {
        let mut session = MapEditorSession::new(ThemeUuid::nil(), None, GridSize::default());
        let seeded = SizeFieldSpans::from_session(&session);
        assert_eq!(*seeded.width(), *GridSize::default().width());

        let Ok(shrunk) = GridSize::new(GridWidth::new(16), GridHeight::new(16), GridLevels::new(1))
        else {
            unreachable!("a 16×16×1 grid is valid");
        };
        session.set_grid_size(shrunk);
        let displayed = SizeFieldSpans::from_session(&session);
        assert_eq!(
            *displayed.width(),
            16,
            "width tracks the programmatic change"
        );
        assert_eq!(
            *displayed.height(),
            16,
            "height tracks the programmatic change"
        );
        assert_eq!(
            *displayed.levels(),
            1,
            "levels tracks the programmatic change"
        );
    }
}
