//! The THEME-mode form's **type vocabulary** (GTW-475): the in-progress [`ThemeDraft`]
//! resource, the identity / field markers the form's controls carry, and the
//! [`SaveThemeError`] failure enum.
//!
//! The draft captures the [`UuidThemeDef`](gdtf_battle_sim::level::UuidThemeDef) shape by
//! REFERENCE only — a display name, a multi-selected set of terrain [`TerrainUuid`]s (chosen
//! from the loaded terrain library), and one of those as the `default_floor`. The theme never
//! inlines terrain stats (C3): it stores UUIDs the registry resolves. The [`ThemeUuid`] is
//! editor-minted (auto-generated for a new theme, or loaded from the registry when editing —
//! C4) and shown read-only.

use bevy::prelude::*;
use gdtf_battle_sim::{level::ThemeUuid, terrain::def::TerrainUuid};

/// The in-progress THEME-mode authoring DRAFT — the state-scoped resource the form's controls
/// write and the save reads + projects into a
/// [`UuidThemeDef`](gdtf_battle_sim::level::UuidThemeDef) (GTW-475 C2).
///
/// A state-scoped [`Resource`] (inserted `OnEnter(Editing)`, removed `OnExit(Editing)` —
/// bevy-traps #1). Every field is a domain value (no-bare-types): the key is a [`ThemeUuid`],
/// the selected terrain + the default floor are [`TerrainUuid`]s. The draft holds REFERENCES
/// only — it never inlines a terrain's stats (C3). The display name is a bare [`String`] only
/// as the text-field buffer; it folds into a [`ThemeDisplayName`](gdtf_battle_sim::level::ThemeDisplayName)
/// on projection (the same exception the TERRAIN form's `display_name` buffer is).
#[derive(Resource, Clone, Debug, PartialEq, Eq)]
pub struct ThemeDraft {
    /// The display name (the text-field buffer). Folds to a file slug on save + a
    /// [`ThemeDisplayName`](gdtf_battle_sim::level::ThemeDisplayName) on projection.
    display_name:  String,
    /// The multi-selected terrain palette — the [`TerrainUuid`]s this theme draws from (C2).
    /// A set: deduped, insertion-order-stable so the saved `terrain` list is deterministic.
    terrain:       Vec<TerrainUuid>,
    /// The chosen default-floor terrain — MUST be one of [`terrain`](ThemeDraft::terrain) on
    /// save (C6). `None` until a terrain is selected to be the floor.
    default_floor: Option<TerrainUuid>,
    /// The theme's UUID key — minted on a NEW theme ([`ThemeUuid::generate`]) or loaded from
    /// the registry when EDITING an existing theme (C4). Shown read-only.
    key:           ThemeUuid,
}

impl ThemeDraft {
    /// A fresh draft for a NEW theme: an empty name, no terrain, no floor, and a freshly minted
    /// [`ThemeUuid`] (C4 — the New-theme affordance mints + clears the form).
    #[must_use]
    pub fn new_theme() -> Self {
        Self {
            display_name:  String::new(),
            terrain:       Vec::new(),
            default_floor: None,
            key:           ThemeUuid::generate(),
        }
    }

    /// Build a draft from a loaded theme's parts (C4 — editing an existing theme): its key,
    /// display name, terrain palette, and default floor.
    #[must_use]
    pub const fn from_parts(
        key: ThemeUuid,
        display_name: String,
        terrain: Vec<TerrainUuid>,
        default_floor: TerrainUuid,
    ) -> Self {
        Self {
            display_name,
            terrain,
            default_floor: Some(default_floor),
            key,
        }
    }

    /// The display name.
    #[must_use]
    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    /// Set the display name (committed from the text field).
    pub fn set_display_name(&mut self, name: String) {
        self.display_name = name;
    }

    /// The selected terrain palette (the multi-select set).
    #[must_use]
    pub fn terrain(&self) -> &[TerrainUuid] {
        &self.terrain
    }

    /// Whether a terrain UUID is currently in the selected palette.
    #[must_use]
    pub fn has_terrain(&self, terrain: TerrainUuid) -> bool {
        self.terrain.contains(&terrain)
    }

    /// Toggle a terrain UUID in the multi-select palette — add it if absent, remove it if
    /// present (C2). Removing the current default floor clears the floor (it must stay one of
    /// the selected terrain — C6 fail-closed).
    pub fn toggle_terrain(&mut self, terrain: TerrainUuid) {
        if let Some(pos) = self.terrain.iter().position(|t| *t == terrain) {
            self.terrain.remove(pos);
            if self.default_floor == Some(terrain) {
                self.default_floor = None;
            }
        } else {
            self.terrain.push(terrain);
        }
    }

    /// The chosen default-floor terrain, or [`None`] if none is chosen yet.
    #[must_use]
    pub const fn default_floor(&self) -> Option<TerrainUuid> {
        self.default_floor
    }

    /// Set the default-floor terrain — IGNORED unless the terrain is in the selected palette,
    /// so the C6 "default floor is one of the theme's own terrain" rule holds fail-closed even
    /// if a stale dropdown commit arrives.
    pub fn set_default_floor(&mut self, terrain: TerrainUuid) {
        if self.terrain.contains(&terrain) {
            self.default_floor = Some(terrain);
        }
    }

    /// The theme's UUID key (minted for a new theme; loaded when editing — C4).
    #[must_use]
    pub const fn key(&self) -> ThemeUuid {
        self.key
    }
}

impl Default for ThemeDraft {
    /// A fresh NEW-theme draft (the `OnEnter(Editing)` seed) — defers to
    /// [`new_theme`](ThemeDraft::new_theme) so the opening draft already carries a minted key.
    fn default() -> Self {
        Self::new_theme()
    }
}

/// Identity marker on the THEME form's display-name text field (no-bare-types unit marker).
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct ThemeNameField;

/// Identity marker on the THEME form's default-floor dropdown root (no-bare-types unit marker).
/// `pub` (re-exported) so the C-tests can assert the picker exists / drive a selection.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub struct ThemeDefaultFloorPicker;

/// Identity marker on ONE terrain-library multi-select row, carrying which [`TerrainUuid`] it
/// toggles (C2). NOT a bare `Uuid` — the key is the sim domain value (no-bare-types). `pub`
/// (re-exported) so the integration test can find / count the library rows.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub struct ThemeTerrainRow {
    /// The terrain UUID this row toggles in/out of the theme palette.
    terrain: TerrainUuid,
}

impl ThemeTerrainRow {
    /// Build a terrain-library row marker for a terrain key.
    #[must_use]
    pub(crate) const fn new(terrain: TerrainUuid) -> Self {
        Self { terrain }
    }

    /// The terrain UUID this row toggles.
    #[must_use]
    pub(crate) const fn terrain(self) -> TerrainUuid {
        self.terrain
    }
}

/// Identity marker on the "Save theme" button (no-bare-types unit marker).
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct SaveThemeButton;

/// Identity marker on the "New theme" button (no-bare-types unit marker) — the C4 affordance
/// that mints a fresh key + clears the form.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct NewThemeButton;

/// Marker on the THEME form's read-only KEY text node — rewritten with the draft's key on a
/// load / new-theme reset (C4). A unit marker (no-bare-types).
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct ThemeKeyText;

/// Marker on the THEME form's RESOLVED-STATS readout text node — rewritten when the
/// default-floor selection changes, showing the resolved terrain's sim kind + HP/armor
/// (C3 — single-source-of-truth proof: the theme stores a UUID, the stats are RESOLVED). A
/// unit marker (no-bare-types). `pub` (re-exported) so the integration test can read it.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub struct ThemeResolvedStatsText;

/// Marker on the THEME form's resolved-stats HP [`ProgressBar`](gdtf_ui::ProgressBarTrack) track
/// — the bar's fill is mutated to the resolved terrain's HP fraction (C3 — reuse the
/// progress-bar widget the way the TERRAIN STAT preview does). A unit marker (no-bare-types).
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct ThemeResolvedHpBar;

/// Marker on the THEME form's read-only `.terrain_theme.ron` PREVIEW text node — rewritten each
/// draft change with the serialized theme def (the live preview, C3). A unit marker.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct ThemeRonPreview;

/// Why a theme save was REJECTED — the handled, no-panic failure of the theme save path
/// (GTW-475). A named domain enum (no-bare-types). `pub` because the `pub`
/// [`serialize_theme_def`](crate::serialize_theme_def) returns it (the C7 tests reuse the
/// projection + serialization seam).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaveThemeError {
    /// The author entered no display name (an empty / whitespace-only field) — there is no
    /// file slug to write to.
    EmptyName,
    /// The theme has no terrain selected — a theme with an empty palette has nothing to draw
    /// from (and no candidate for the default floor — C6).
    NoTerrain,
    /// No default floor is chosen, or the chosen one is not in the theme's own terrain palette
    /// (C6 — a saved theme's default floor MUST be one of its terrain UUIDs).
    DefaultFloorNotInTerrain,
    /// Serializing the built [`UuidThemeDef`](gdtf_battle_sim::level::UuidThemeDef) to RON
    /// failed.
    Serialize(String),
    /// Writing the serialized def to disk failed (a missing dir / permissions error / io).
    Write(String),
}

impl std::fmt::Display for SaveThemeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyName => write!(f, "no theme name entered — nothing to save"),
            Self::NoTerrain => write!(
                f,
                "the theme has no terrain selected — nothing to draw from"
            ),
            Self::DefaultFloorNotInTerrain => {
                write!(
                    f,
                    "the default floor must be one of the theme's own terrain (C6)"
                )
            }
            Self::Serialize(err) => write!(f, "failed to serialize the theme def: {err}"),
            Self::Write(err) => write!(f, "failed to write the theme-def file: {err}"),
        }
    }
}

impl std::error::Error for SaveThemeError {}
