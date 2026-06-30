//! The map-editor **session-drive helpers** kept across the egui swap (GTW-421 right panel;
//! egui-swept GTW-512).
//!
//! GTW-421 stood up a `bevy_ui` right panel: a theme dropdown + a three-field prefab SIZE selector,
//! each driving the shared [`MapEditorSession`]. The GTW-512 egui swap DELETED that `bevy_ui` spawn
//! (`spawn_right_panel_controls`) and the `gdtf_ui`-widget commit drives (`apply_size_commit`); the
//! egui shell will grow the size selector back in C4 (the PREFAB form). This module retains:
//!
//! - the model TYPES the size selector + theme dropdown are built over ([`GridSpanInput`] /
//!   [`SizeFieldAxis`] / [`ThemeDropdown`]) — re-exported for the children that rebuild the controls,
//! - [`seed_default_theme`] — the UI-agnostic `Update` system that seeds the session theme to the
//!   registry's first theme once it resolves (still wired).
//!
//! The pre-egui `apply_theme_selection` (which read a `gdtf_ui` `DropdownSelectionChanged<ThemeUuid>`
//! and folded it into the session) is GONE: the egui `ComboBox` in
//! [`editor_egui_ui`](crate::egui_shell::editor_egui_ui) performs its VERBATIM body inline
//! (resolve the chosen theme's default-floor + [`MapEditorSession::select_theme`]), so there is no
//! separate drive system and no inert message reader.

use bevy::prelude::*;
use gdtf_battle_sim::level::{ThemeUuid, UuidThemeRegistry};

use crate::session::MapEditorSession;

/// One drawable-area dimension's edited span, in cells — the numeric-field value the size fields
/// edit, clamp, and commit (GTW-421).
///
/// A named newtype over [`u8`] (no-bare-types rule 1). Private inner + derived [`Deref`]; built
/// through [`new`](GridSpanInput::new). It implements the
/// [`NumericValue`](gdtf_ui::NumericValue) bound set (`Copy + PartialOrd + FromStr + Display`) so the
/// children's size fields can be `spawn_numeric_field::<GridSpanInput>` once they rebuild the
/// selector. Converted into the axis newtypes ([`GridWidth`](gdtf_battle_sim::level::GridWidth) /
/// [`GridHeight`](gdtf_battle_sim::level::GridHeight) /
/// [`GridLevels`](gdtf_battle_sim::level::GridLevels)) when a commit is folded into the session's
/// [`GridSize`](gdtf_battle_sim::level::GridSize).
#[derive(Deref, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct GridSpanInput(u8);

impl GridSpanInput {
    /// Wrap a cell span.
    #[must_use]
    pub const fn new(cells: u8) -> Self {
        Self(cells)
    }
}

impl core::fmt::Display for GridSpanInput {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl core::str::FromStr for GridSpanInput {
    type Err = core::num::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<u8>().map(Self)
    }
}

/// Which drawable-area axis a size field drives — the identity marker the children attach to each
/// size field so the commit drive maps a commit to the right
/// [`GridSize`](gdtf_battle_sim::level::GridSize) axis (GTW-421).
///
/// A named marker enum (no-bare-types) rather than three separate unit markers.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub enum SizeFieldAxis {
    /// The width field (x span).
    Width,
    /// The height field (y span).
    Height,
    /// The levels field (z span, clamped to the sim's `MAX_LEVELS` storey ceiling).
    Levels,
}

/// Marker on a theme dropdown's closed-control root — the identity marker a child's theme-selection
/// drive uses to filter a selection back to THIS editor's theme control (no-bare-types unit marker).
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub struct ThemeDropdown;

/// `Update` (in `Editing`): seed the session theme to the registry's first theme (by display name)
/// once the [`UuidThemeRegistry`] resolves (C1).
///
/// The session opens with the [`ThemeUuid::nil`] sentinel theme; this picks the first theme (by
/// display name — the SAME order the egui `ComboBox` lists) and resolves its default-floor key,
/// writing both into the session. Runs while the session theme is still nil (it seeds exactly once),
/// guarded on the optional registry / session (bevy-traps #1). After the seed the egui status line
/// + the children's palette/canvas populate from a real theme rather than the empty nil theme.
pub(crate) fn seed_default_theme(
    themes: Option<Res<UuidThemeRegistry>>,
    session: Option<ResMut<MapEditorSession>>,
) {
    let (Some(themes), Some(mut session)) = (themes, session) else {
        return;
    };
    if !session.theme().is_nil() {
        return;
    }
    let mut entries: Vec<(String, ThemeUuid)> = themes
        .defs()
        .map(|(key, def)| ((*def.display_name).clone(), *key))
        .collect();
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    let Some((_, theme)) = entries.into_iter().next() else {
        return;
    };
    let default_floor = themes.default_floor(&theme);
    session.select_theme(theme, default_floor);
}
