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

// GTW-512: the THEME form's `bevy_ui` WIDGET MARKERS (the name field, the default-floor picker
// root, the per-terrain library row, the save / new-theme buttons, the read-only key / resolved-stats
// / HP-bar / RON-preview text markers) were DROPPED in the egui swap — they marked `bevy_ui` entities
// that no longer exist. The C3 child (GTW-514) re-creates egui-native equivalents; this file keeps the
// MODEL (the draft + the error enum) and the pure resolution lives in `resolve.rs`.

/// Why a theme save was REJECTED — the handled, no-panic failure of the theme save path
/// (GTW-475). A named domain enum (no-bare-types). `pub` because the `pub`
/// [`serialize_theme_def`](crate::serialize_theme_def) returns it (the C7 tests reuse the
/// projection + serialization path).
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
    /// The shared serialize/write tail failed (GTW-577 C3) — wraps the shared serialize/write path's
    /// [`RonSaveError`](gdtf_assets::RonSaveError), whose `Display` names the failed stage.
    Save(gdtf_assets::RonSaveError),
}

impl From<gdtf_assets::RonSaveError> for SaveThemeError {
    /// The per-type conversion off the shared serialize/write error (GTW-577 C3) — lets the save path
    /// `?` a serialize/write failure straight into the form's error.
    fn from(err: gdtf_assets::RonSaveError) -> Self {
        Self::Save(err)
    }
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
            Self::Save(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for SaveThemeError {}
