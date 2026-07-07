//! The capture drive's **parsed force-value newtypes** — each wraps one env-var-selected
//! override the [`plugin`](super::plugin) inserts and the [`drive`](super::drive) systems
//! apply. All named wrappers (no-bare-types) so a forced value reads as a domain value
//! rather than colliding with the editor's own resources.

use bevy::prelude::{Deref, Resource};

use crate::{EditorMode, canvas::CanvasZoom, terrain_form::TerrainKindChoice};

/// The capture's FORCED [`EditorMode`] (C2.4 — from `GDTF_EDITOR_MODE`), inserted only when the
/// env var named a recognized mode.
#[derive(Resource, Clone, Copy, Deref)]
pub(super) struct ForcedMode(EditorMode);

impl ForcedMode {
    /// Parse a `GDTF_EDITOR_MODE` value (case-insensitive `terrain` | `theme` | `prefab` |
    /// `gang` | `armor`) into a forced mode, or [`None`] for an unset / unrecognized value (the
    /// capture keeps the editor's default mode).
    pub(super) fn from_env_value(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "terrain" => Some(Self(EditorMode::Terrain)),
            "theme" => Some(Self(EditorMode::Theme)),
            "prefab" => Some(Self(EditorMode::Prefab)),
            "gang" => Some(Self(EditorMode::Gang)),
            "armor" => Some(Self(EditorMode::Armor)),
            _ => None,
        }
    }
}

/// The capture's FORCED TERRAIN-form kind segment (GTW-574 — from
/// `GDTF_EDITOR_TERRAIN_KIND`), inserted only when the env var named a recognized kind.
/// The drive applies it to the live [`TerrainDraft`](crate::terrain_form::TerrainDraft)
/// through the SAME `set_kind` setter the egui segmented row commits — and, for the
/// Emplacement kind, pre-selects the first sorted
/// [`WeaponRegistry`](gdtf_battle_sim::weapon::WeaponRegistry) key so the mounted-weapon
/// dropdown captures POPULATED (the AC-7 positive content).
#[derive(Resource, Clone, Copy, Deref)]
pub(super) struct ForcedTerrainKind(TerrainKindChoice);

impl ForcedTerrainKind {
    /// Parse a `GDTF_EDITOR_TERRAIN_KIND` value (case-insensitive `wall` | `cover` | `slab` |
    /// `emplacement`) into a forced kind, or [`None`] for an unset / unrecognized value (the
    /// capture keeps the draft's default kind).
    pub(super) fn from_env_value(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "wall" => Some(Self(TerrainKindChoice::Wall)),
            "cover" => Some(Self(TerrainKindChoice::Cover)),
            "slab" => Some(Self(TerrainKindChoice::Slab)),
            "emplacement" => Some(Self(TerrainKindChoice::Emplacement)),
            _ => None,
        }
    }
}

/// The capture's FORCED preview zoom (C4.11 — from `GDTF_EDITOR_ZOOM`), inserted only when the env
/// var parsed a finite float. The wrapped [`CanvasZoom`] clamps it into `[0.25, 4.0]` on
/// construction.
#[derive(Resource, Clone, Copy, Deref)]
pub(super) struct ForcedZoom(CanvasZoom);

impl ForcedZoom {
    /// Parse a `GDTF_EDITOR_ZOOM` value (a float) into a forced zoom, or [`None`] for an unset /
    /// unparseable / non-finite value (the capture keeps the identity `1.0` scale). The parsed
    /// factor is applied through [`CanvasZoom::scaled`] over the identity, so it is clamped into
    /// `[0.25, 4.0]`.
    pub(super) fn from_env_value(value: &str) -> Option<Self> {
        let factor = value.trim().parse::<f32>().ok().filter(|f| f.is_finite())?;
        Some(Self(CanvasZoom::identity().scaled(factor)))
    }
}

/// The capture's FORCED prefab storey view (GTW-532 / GTW-594 — from `GDTF_EDITOR_VIEW`),
/// inserted only when the env var selected a stageable variant. Either way the drive
/// stages a 2-storey grid with a distinct upper-storey block, so the captured view
/// visibly differs from the plain default shot.
#[derive(Resource, Clone, Copy, PartialEq, Eq)]
pub(super) enum ForcedView {
    /// `GDTF_EDITOR_VIEW=full` — force
    /// [`ViewMode::FullView`](gdtf_battle_presenter::ViewMode::FullView) AND
    /// [`IsolateView::Off`](gdtf_battle_presenter::IsolateView::Off) (Isolate wins over
    /// the two-state mode, so the GTW-521 whole-stack capture must lift it — GTW-594 C3).
    Full,
    /// `GDTF_EDITOR_VIEW=isolate` — assert the GTW-594 editor default (Isolate ON, one
    /// onion below) AND lift the edit storey to the painted UPPER storey, so the shot
    /// shows the three categorical classes (authored-here / exists-below / empty) at once.
    Isolate,
}

impl ForcedView {
    /// Parse a `GDTF_EDITOR_VIEW` value into a forced view, or [`None`] for an unset /
    /// unrecognized value (the capture keeps the editor's defaults). `full`
    /// (case-insensitive) selects the GTW-521 whole-stack variant; `isolate` the GTW-594
    /// three-class variant.
    pub(super) fn from_env_value(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "full" | "fullview" | "full_view" => Some(Self::Full),
            "isolate" => Some(Self::Isolate),
            _ => None,
        }
    }
}
