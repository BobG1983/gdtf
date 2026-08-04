//! Prefab grid size field state and UI.

use bevy_egui::egui;
use gdtf_battle_sim::level::{GridHeight, GridLevels, GridSize, GridWidth, MAX_GRID_SPAN};

use crate::{canvas::CurrentEditLevel, right_panel::GridSpanInput, session::MapEditorSession};

const SPAN_RANGE: core::ops::RangeInclusive<u8> = 1..=MAX_GRID_SPAN;

const LEVELS_RANGE: core::ops::RangeInclusive<u8> = 1..=gdtf_battle_sim::metric::MAX_LEVELS;

/// Editable width, height, and level spans for the prefab grid.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SizeFieldSpans {
    width:  GridSpanInput,
    height: GridSpanInput,
    levels: GridSpanInput,
}

impl SizeFieldSpans {
    /// Build from explicit span inputs.
    #[must_use]
    pub const fn new(width: GridSpanInput, height: GridSpanInput, levels: GridSpanInput) -> Self {
        Self {
            width,
            height,
            levels,
        }
    }

    /// Seed from the current session grid size.
    #[must_use]
    pub fn from_session(session: &MapEditorSession) -> Self {
        let size = session.grid_size();
        Self::new(
            GridSpanInput::new(*size.width()),
            GridSpanInput::new(*size.height()),
            GridSpanInput::new(*size.levels()),
        )
    }

    /// Width span.
    #[must_use]
    pub const fn width(&self) -> GridSpanInput {
        self.width
    }

    /// Height span.
    #[must_use]
    pub const fn height(&self) -> GridSpanInput {
        self.height
    }

    /// Level count span.
    #[must_use]
    pub const fn levels(&self) -> GridSpanInput {
        self.levels
    }

    /// Write clamped size into the session and clamp the edit level.
    pub fn commit(self, session: &mut MapEditorSession, edit_level: &mut CurrentEditLevel) {
        let new_size =
            clamp_to_grid_size(*self.width, *self.height, *self.levels, session.grid_size());
        session.set_grid_size(new_size);
        *edit_level = edit_level.clamped(new_size);
    }
}

#[must_use]
pub(crate) fn clamp_to_grid_size(w: u8, h: u8, levels: u8, previous: GridSize) -> GridSize {
    let width = GridWidth::new(w.clamp(*SPAN_RANGE.start(), *SPAN_RANGE.end()));
    let height = GridHeight::new(h.clamp(*SPAN_RANGE.start(), *SPAN_RANGE.end()));
    let levels = GridLevels::new(levels.clamp(*LEVELS_RANGE.start(), *LEVELS_RANGE.end()));
    GridSize::new(width, height, levels).unwrap_or(previous)
}

pub(crate) fn size_fields(
    ui: &mut egui::Ui,
    session: &mut MapEditorSession,
    edit_level: &mut CurrentEditLevel,
) {
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

    #[test]
    fn clamp_folds_into_the_system_constant_bands() {
        let previous = GridSize::default();
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
        let floored = clamp_to_grid_size(0, 0, 0, previous);
        assert_eq!(*floored.width(), 1, "zero width clamps up to 1");
        assert_eq!(*floored.height(), 1, "zero height clamps up to 1");
        assert_eq!(*floored.levels(), 1, "zero levels clamps up to 1");
    }

    #[test]
    fn clamp_passes_in_range_sizes_through() {
        let previous = GridSize::default();
        let clamped = clamp_to_grid_size(16, 24, 3, previous);
        assert_eq!(*clamped.width(), 16);
        assert_eq!(*clamped.height(), 24);
        assert_eq!(*clamped.levels(), 3);
    }

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
