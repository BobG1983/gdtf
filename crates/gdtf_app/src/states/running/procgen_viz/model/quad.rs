//! The reveal-sequence quad: the tint role a placed prefab draws with, the occupying
//! gang annotation, and the [`VizQuad`] projection off the sim placement. Split out of
//! the monolithic `model.rs` (GTW-583); the model rationale lives on the parent
//! `model` module.

use bevy::prelude::*;
use gdtf_battle_sim::{
    ganger::{GangName, GangRegistry},
    level::{PrefabName, SpawnRole},
    procgen::PlacedPrefab,
};

use super::units::{QuadCellH, QuadCellW, QuadCellX, QuadCellY, QuadRect, QuadSize};

crate::support_item! {
    /// Which deployment ROLE a placed prefab plays — and therefore which tint its quad draws
    /// with (C3): the player-spawn prefab is GREEN, the enemy-spawn prefab is RED, every other
    /// (fill) prefab is the NEUTRAL light tint.
    ///
    /// A named domain enum (no-bare-types: a quad's tint role is a domain value, not a bare
    /// `Color`/`u8`) so the draw layer never confuses "which role" with "which raw color". It
    /// is projected from the sim's [`SpawnRole`] for the spawn prefabs and is always
    /// [`Neutral`](QuadTint::Neutral) for fill. Declared through `crate::support_item!` so
    /// the headless test can read a quad entity's tint through [`crate::test_support`].
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    enum QuadTint {
        /// The player-spawn prefab quad — tinted GREEN (C3).
        Player,
        /// The enemy-spawn prefab quad — tinted RED (C3).
        Enemy,
        /// Every other (fill) prefab quad — the neutral LIGHT tint (C2/C3).
        Neutral,
    }
}

impl QuadTint {
    /// The light tinted [`Color`] a quad of this role draws with (C2/C3): a light GREEN for
    /// the player spawn, a light RED for the enemy spawn, a neutral light grey otherwise.
    ///
    /// Each is a translucent light fill so the dark board quad reads THROUGH it (the quads
    /// are "light tinted quads over a single dark whole-level quad", C2). Pure UI plumbing
    /// (a resolved `Color`), produced from the domain [`QuadTint`].
    #[must_use]
    pub(in crate::states::running::procgen_viz) const fn color(self) -> Color {
        match self {
            // A light, slightly translucent green / red / grey: the dark board reads through.
            Self::Player => Color::srgba(0.45, 0.85, 0.50, 0.78),
            Self::Enemy => Color::srgba(0.90, 0.40, 0.40, 0.78),
            Self::Neutral => Color::srgba(0.82, 0.82, 0.86, 0.62),
        }
    }

    /// Project a placed-prefab [`SpawnRole`] to its quad tint: the deployment roles map to
    /// their colored tints, generic [`Fill`](SpawnRole::Fill) to the neutral one.
    #[must_use]
    pub(super) const fn from_role(role: SpawnRole) -> Self {
        match role {
            SpawnRole::Player => Self::Player,
            SpawnRole::Enemy => Self::Enemy,
            SpawnRole::Fill => Self::Neutral,
        }
    }
}

/// A gang ANNOTATION on a deployment quad — the occupying gang's name plus its roster member
/// count, shown as the quad's label so the player/enemy deployment zones read as "who is here"
/// (GTW-498 C4).
///
/// A named newtype over [`String`] (no-bare-types rule 1: a label is a domain value, not a bare
/// `String`). Procgen is TERRAIN-ONLY — choosing a gang does NOT change the generated layout;
/// the annotation only LABELS the already-placed player/enemy deployment quad (display, not
/// geometry). Built through [`for_roster`](GangAnnotation::for_roster) from a resolved roster,
/// read through the derived [`Deref`]; private inner.
#[derive(Deref, Clone, PartialEq, Eq, Debug)]
pub(in crate::states::running::procgen_viz) struct GangAnnotation(String);

impl GangAnnotation {
    /// Build a deployment-quad annotation from a gang's [`GangName`] and its roster's member
    /// count — e.g. `goliaths (4)`. The count is resolved against the [`GangRegistry`]; an
    /// absent / empty roster annotates with `(0)` (never panics — C4 no-gang fallback).
    #[must_use]
    pub(in crate::states::running::procgen_viz) fn for_roster(
        name: &GangName,
        registry: Option<&GangRegistry>,
    ) -> Self {
        let members = registry
            .and_then(|registry| registry.roster(name))
            .map_or(0, |roster| roster.members.len());
        Self(format!("{} ({members})", name.as_str()))
    }
}

crate::support_item! {
    /// One quad in the visualizer's ordered reveal sequence — a placed prefab's name, footprint
    /// size, board rectangle, and the [`QuadTint`] it draws with (C2/C3).
    ///
    /// A named struct (no-bare-types: a visualized placement is a domain value). The STEP /
    /// AUTO controls reveal these in order; the draw layer turns each REVEALED quad into a light
    /// tinted [`Node`](bevy::ui::Node) sized to its rectangle with a name + size label. Declared
    /// through `crate::support_item!` so it is at least as public as
    /// [`ProcgenViz::quads`](super::viz::ProcgenViz::quads) (which returns `&[VizQuad]`) under `test-support`.
    #[derive(Clone, PartialEq, Eq, Debug)]
    struct VizQuad {
        /// The placed prefab's name (the base label text).
        name: PrefabName,
        /// The quad's footprint size in cells (the label reports `WxH`).
        size: QuadSize,
        /// The quad's rectangle on the board (origin + extent — where + how big to draw it).
        rect: QuadRect,
        /// The tint role this quad draws with (player = green, enemy = red, fill = neutral, C3).
        tint: QuadTint,
        /// The OCCUPYING gang annotation (C4) — present only on the player / enemy deployment
        /// quads when a gang is chosen, [`None`] on fill quads. When present it REPLACES the
        /// prefab name in the quad's label so the deployment zone reads as the gang in it.
        gang: Option<GangAnnotation>,
    }
}

impl VizQuad {
    /// The quad's footprint size in cells.
    #[must_use]
    pub(in crate::states::running::procgen_viz) const fn size(&self) -> QuadSize {
        self.size
    }

    /// The quad's board rectangle (origin + extent).
    #[must_use]
    pub(in crate::states::running::procgen_viz) const fn rect(&self) -> QuadRect {
        self.rect
    }

    /// The quad's tint role.
    #[must_use]
    pub(in crate::states::running::procgen_viz) const fn tint(&self) -> QuadTint {
        self.tint
    }

    /// The quad's DISPLAY label text — the occupying gang annotation (C4) when present
    /// (player / enemy deployment quads), else the placed prefab's name (fill quads). The draw
    /// layer renders this plus the `WxH` size.
    #[must_use]
    pub(in crate::states::running::procgen_viz) fn label_text(&self) -> String {
        match &self.gang {
            Some(gang) => (**gang).clone(),
            None => self.name.as_str().to_owned(),
        }
    }

    /// Project one placed prefab (with an explicit tint role + optional gang annotation, C4)
    /// into a visualizer quad — reading its name, footprint size, and region rectangle from the
    /// sim placement. A `gang` annotation REPLACES the prefab name in the quad's label.
    pub(super) fn from_placed(
        placed: &PlacedPrefab,
        tint: QuadTint,
        gang: Option<GangAnnotation>,
    ) -> Self {
        let prefab = placed.prefab();
        let region = placed.region();
        let footprint = region.footprint();
        let origin = region.origin();
        // The packer never produces a negative origin / extent (board cells are `>= 0`); the
        // `.max(0)` clamp is a fail-safe so a stray negative never wraps the cast.
        let size = QuadSize::new(
            QuadCellW::new(u32::try_from(footprint.width().max(0)).unwrap_or(0)),
            QuadCellH::new(u32::try_from(footprint.height().max(0)).unwrap_or(0)),
        );
        let rect = QuadRect::new(
            QuadCellX::new(u32::try_from(origin.x.max(0)).unwrap_or(0)),
            QuadCellY::new(u32::try_from(origin.y.max(0)).unwrap_or(0)),
            size,
        );
        Self {
            name: prefab.name().clone(),
            size,
            rect,
            tint,
            gang,
        }
    }
}
