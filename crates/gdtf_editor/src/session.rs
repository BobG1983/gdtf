//! The editor's shared **authoring session** — the canonical selection state the right
//! panel (GTW-421) writes and the canvas children (GTW-423 / GTW-426) read.
//!
//! GTW-417 stood up the editor shell with no theme/size selection state; GTW-421 defines it
//! here. [`MapEditorSession`] holds the three selections the right panel drives: the chosen
//! [`LevelTheme`], the active default-floor [`TileKey`] (resolved from the GTW-409
//! [`ThemeCatalogRegistry`] for the chosen theme — C2), and the drawable-area [`GridSize`]
//! (clamped to the sim's `60×60×8` maximum — C3). It is a state-scoped resource: inserted
//! `OnEnter(Editing)` and removed `OnExit(Editing)` (bevy-traps #1), so every reader guards
//! with `Option<Res<…>>` / `run_if(resource_exists::<…>)`.
//!
//! Each selection is a domain value, not a bare type (no-bare-types): the theme is the
//! sim's closed [`LevelTheme`] set, the default floor is the catalog's own [`TileKey`], and
//! the size is the validated [`GridSize`].

use bevy::prelude::*;
use gdtf_battle_sim::level::{GridSize, LevelTheme, TileKey};

/// The editor's shared authoring session — the canonical theme / default-floor / grid-size
/// selection the right panel writes and later canvas children read (GTW-421).
///
/// A state-scoped [`Resource`] (inserted `OnEnter(Editing)`, removed `OnExit(Editing)` —
/// bevy-traps #1). Holds:
///
/// - [`theme`](MapEditorSession::theme) — the selected [`LevelTheme`] (C1/C2). The dropdown
///   pre-selects [`LevelTheme::default`] on open, so this seeds to the same default.
/// - [`default_floor`](MapEditorSession::default_floor) — the active default-floor
///   [`TileKey`] (C2), resolved from the [`ThemeCatalogRegistry`](gdtf_battle_sim::level::ThemeCatalogRegistry)
///   for the selected theme. The GTW-409 catalog keys its default floor by [`TileKey`] (its
///   own catalog-internal key), so the "default-floor tile" the contract names is this
///   `TileKey` — the value `ThemeTileCatalog::default_floor_key` returns. `None` until a
///   catalog resolves it (an empty / absent registry leaves it unset rather than panicking).
/// - [`grid_size`](MapEditorSession::grid_size) — the drawable-area [`GridSize`] (C3),
///   always a validated, in-bounds value (every mutation flows through [`GridSize::new`]).
///
/// Every field is read through an accessor and mutated through a named setter, so the
/// catalog-resolve and clamp invariants live in one place.
#[derive(Resource, Debug, Clone, PartialEq, Eq)]
pub struct MapEditorSession {
    /// The selected level theme (the dropdown's current choice).
    theme:         LevelTheme,
    /// The active default-floor tile key, resolved from the chosen theme's catalog
    /// (`None` until a catalog resolves it).
    default_floor: Option<TileKey>,
    /// The drawable-area dimensions, validated to `1..=60` on x/y and `1..=8` on z.
    grid_size:     GridSize,
}

impl MapEditorSession {
    /// Build a session from an initial theme, default-floor key, and grid size — used by the
    /// `OnEnter(Editing)` seed and by tests.
    #[must_use]
    pub const fn new(
        theme: LevelTheme,
        default_floor: Option<TileKey>,
        grid_size: GridSize,
    ) -> Self {
        Self {
            theme,
            default_floor,
            grid_size,
        }
    }

    /// The selected level theme.
    #[must_use]
    pub const fn theme(&self) -> LevelTheme {
        self.theme
    }

    /// The active default-floor tile key resolved from the selected theme's catalog, or
    /// `None` if no catalog has resolved one yet.
    #[must_use]
    pub const fn default_floor(&self) -> Option<&TileKey> {
        self.default_floor.as_ref()
    }

    /// The drawable-area dimensions (always validated, in-bounds).
    #[must_use]
    pub const fn grid_size(&self) -> GridSize {
        self.grid_size
    }

    /// Set the selected theme and its resolved default-floor key together (C2) — they always
    /// change as a pair, so the setter takes both to keep them consistent.
    pub fn select_theme(&mut self, theme: LevelTheme, default_floor: Option<TileKey>) {
        self.theme = theme;
        self.default_floor = default_floor;
    }

    /// Set the drawable-area dimensions (C3) — the caller has already validated + clamped via
    /// [`GridSize::new`], so this stores the in-bounds value.
    pub const fn set_grid_size(&mut self, grid_size: GridSize) {
        self.grid_size = grid_size;
    }
}

impl Default for MapEditorSession {
    /// The opening session: the default [`LevelTheme`], no resolved default floor yet (the
    /// `OnEnter` seed resolves it from the registry), and the full `60×60×8`
    /// [`GridSize`](GridSize::default) drawable area.
    fn default() -> Self {
        Self {
            theme:         LevelTheme::default(),
            default_floor: None,
            grid_size:     GridSize::default(),
        }
    }
}
