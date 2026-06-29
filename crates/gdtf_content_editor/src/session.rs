//! The editor's shared **authoring session** — the canonical selection state the right
//! panel (GTW-421) writes and the canvas children (GTW-423 / GTW-426) read.
//!
//! GTW-417 stood up the editor shell with no theme/size selection state; GTW-421 defined it
//! here. GTW-495 (T09) swept it onto the NEW UUID-keyed terrain/theme model: the theme is a
//! [`ThemeUuid`], the default floor and the active paint tile are [`TerrainUuid`]s (resolved
//! against the GTW-487 [`UuidThemeRegistry`](gdtf_battle_sim::level::UuidThemeRegistry) /
//! [`TerrainDefRegistry`](gdtf_battle_sim::terrain::def::TerrainDefRegistry)), and the
//! drawable-area [`GridSize`] is unchanged. It is a state-scoped resource: inserted
//! `OnEnter(Editing)` and removed `OnExit(Editing)` (bevy-traps #1), so every reader guards
//! with `Option<Res<…>>` / `run_if(resource_exists::<…>)`.
//!
//! Each selection is a domain value, not a bare type (no-bare-types): the theme is the sim's
//! [`ThemeUuid`], the default floor and the paint tile are [`TerrainUuid`]s, and the size is
//! the validated [`GridSize`].

use bevy::prelude::*;
use gdtf_battle_sim::{
    level::{GridSize, ThemeUuid},
    terrain::def::TerrainUuid,
};

/// The editor's shared authoring session — the canonical theme / default-floor / grid-size /
/// paint-tile selection the right panel writes and the canvas children read (GTW-421; swept
/// onto the UUID model in GTW-495).
///
/// A state-scoped [`Resource`] (inserted `OnEnter(Editing)`, removed `OnExit(Editing)` —
/// bevy-traps #1). Holds:
///
/// - [`theme`](MapEditorSession::theme) — the selected [`ThemeUuid`] (C1). On open this seeds
///   to the [`ThemeUuid::nil`] sentinel; the right-panel dropdown's first selection (or the
///   eager `OnEnter` seed once the registries resolve) sets it to a real theme key.
/// - [`default_floor`](MapEditorSession::default_floor) — the active default-floor
///   [`TerrainUuid`], resolved from the [`UuidThemeRegistry`](gdtf_battle_sim::level::UuidThemeRegistry)
///   for the selected theme. `None` until a theme with a registered default floor resolves it.
/// - [`grid_size`](MapEditorSession::grid_size) — the drawable-area [`GridSize`] (C3), always a
///   validated, in-bounds value (every mutation flows through [`GridSize::new`]).
/// - [`selected_tile`](MapEditorSession::selected_tile) — the active PAINT tile's
///   [`TerrainUuid`] (GTW-422 C2), the palette row the author last clicked. `None` until a row
///   is selected; the canvas paints with it and the bottom-right stat region shows its
///   [`TerrainDef`](gdtf_battle_sim::terrain::def::TerrainDef) stats (C3).
///
/// Every field is read through an accessor and mutated through a named setter, so the
/// registry-resolve and clamp invariants live in one place.
#[derive(Resource, Debug, Clone, PartialEq, Eq)]
pub struct MapEditorSession {
    /// The selected level theme key (the dropdown's current choice).
    theme:         ThemeUuid,
    /// The active default-floor terrain key, resolved from the chosen theme's registry entry
    /// (`None` until a theme with a registered default floor resolves it).
    default_floor: Option<TerrainUuid>,
    /// The drawable-area dimensions, validated to `1..=60` on x/y and `1..=8` on z.
    grid_size:     GridSize,
    /// The active PAINT tile — the [`TerrainUuid`] of the palette row the author last clicked
    /// (GTW-422 C2). `None` until a palette row is selected; the canvas paints with this tile
    /// and the bottom-right stat region shows its terrain-def stats (C3).
    selected_tile: Option<TerrainUuid>,
}

impl MapEditorSession {
    /// Build a session from an initial theme, default-floor key, and grid size — used by the
    /// `OnEnter(Editing)` seed and by tests. The active paint tile starts unset (no palette
    /// row has been clicked yet).
    #[must_use]
    pub const fn new(
        theme: ThemeUuid,
        default_floor: Option<TerrainUuid>,
        grid_size: GridSize,
    ) -> Self {
        Self {
            theme,
            default_floor,
            grid_size,
            selected_tile: None,
        }
    }

    /// The selected level theme key.
    #[must_use]
    pub const fn theme(&self) -> ThemeUuid {
        self.theme
    }

    /// The active default-floor terrain key resolved from the selected theme's registry entry,
    /// or `None` if no theme has resolved one yet.
    #[must_use]
    pub const fn default_floor(&self) -> Option<TerrainUuid> {
        self.default_floor
    }

    /// The drawable-area dimensions (always validated, in-bounds).
    #[must_use]
    pub const fn grid_size(&self) -> GridSize {
        self.grid_size
    }

    /// Set the selected theme and its resolved default-floor key together (C2) — they always
    /// change as a pair, so the setter takes both to keep them consistent.
    pub const fn select_theme(&mut self, theme: ThemeUuid, default_floor: Option<TerrainUuid>) {
        self.theme = theme;
        self.default_floor = default_floor;
    }

    /// Set the drawable-area dimensions (C3) — the caller has already validated + clamped via
    /// [`GridSize::new`], so this stores the in-bounds value.
    pub const fn set_grid_size(&mut self, grid_size: GridSize) {
        self.grid_size = grid_size;
    }

    /// The active PAINT tile's [`TerrainUuid`] — the palette row the author last selected (C2),
    /// or `None` if no row has been clicked yet.
    #[must_use]
    pub const fn selected_tile(&self) -> Option<TerrainUuid> {
        self.selected_tile
    }

    /// Set the active paint tile to the clicked palette row's [`TerrainUuid`] (GTW-422 C2). The
    /// canvas paints with this tile and the bottom-right stat region shows its stats (C3).
    pub const fn select_tile(&mut self, tile: TerrainUuid) {
        self.selected_tile = Some(tile);
    }

    /// Clear the active paint tile (no tile selected) — the hover ghost then hides and a canvas
    /// click paints nothing (GTW-426 C1). The deselect counterpart of
    /// [`select_tile`](MapEditorSession::select_tile).
    pub const fn clear_selected_tile(&mut self) {
        self.selected_tile = None;
    }
}

impl Default for MapEditorSession {
    /// The opening session: the [`ThemeUuid::nil`] sentinel theme (the `OnEnter` seed / the
    /// dropdown's first selection resolves a real one), no resolved default floor yet, and the
    /// full `60×60×8` [`GridSize`](GridSize::default) drawable area.
    fn default() -> Self {
        Self {
            theme:         ThemeUuid::nil(),
            default_floor: None,
            grid_size:     GridSize::default(),
            selected_tile: None,
        }
    }
}
