//! The TERRAIN form's **closed pick enums** (GTW-474; split out of `types.rs` in
//! GTW-574): the structural-kind pick ([`TerrainKindChoice`]), the footfall pick
//! ([`FootfallChoice`]), and the graphic-role pick over the presenter's [`TileRole`]
//! vocabulary ([`offered_graphic_roles`]).
//!
//! Every pick enum is a NAMED closed enum (no-bare-types) so the GTW-410 dropdown /
//! GTW-280 segmented control can be generic over it. Since GTW-574 the kind pick is
//! COMPILER-TIED to the sim's canonical
//! [`TerrainPieceKind`](gdtf_battle_sim::terrain::entity::TerrainPieceKind)
//! discriminant via [`From<TerrainPieceKind>`](TerrainKindChoice#impl-From<TerrainPieceKind>-for-TerrainKindChoice)
//! — a new terrain kind is a compile error HERE, so the editor pick list can never
//! silently drop a kind again (the drift that hid `Emplacement` from authors).

use bevy::prelude::Component;
use gdtf_battle_presenter::TileRole;
use gdtf_battle_sim::terrain::{entity::TerrainPieceKind, piece::FootfallSound};

/// Which structural KIND the authored terrain is — the FOUR sim kinds the form offers
/// (no `Floor` / `Scatter`: those are RETIRED on the UUID model — GTW-476; the
/// `Emplacement` kind joined in GTW-574, closing the drift that made it unauthorable).
///
/// A named closed enum (no-bare-types). The variant ORDER is the segmented-control segment
/// order, so [`from_segment`](TerrainKindChoice::from_segment) maps a chosen index back to a
/// kind. Each maps to a [`TerrainSimKind`](gdtf_battle_sim::terrain::def::TerrainSimKind) variant
/// when the draft projects, and the whole set is compiler-tied to the canonical
/// [`TerrainPieceKind`] via the exhaustive `From` below (GTW-574 C4).
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum TerrainKindChoice {
    /// Solid blocking geometry — projects to [`TerrainSimKind::Wall`](gdtf_battle_sim::terrain::def::TerrainSimKind::Wall).
    #[default]
    Wall,
    /// Chest-high cover / scatter — projects to [`TerrainSimKind::Cover`](gdtf_battle_sim::terrain::def::TerrainSimKind::Cover).
    Cover,
    /// A floor/roof slab — projects to [`TerrainSimKind::Slab`](gdtf_battle_sim::terrain::def::TerrainSimKind::Slab).
    Slab,
    /// A weapon emplacement — projects to
    /// [`TerrainSimKind::Emplacement`](gdtf_battle_sim::terrain::def::TerrainSimKind::Emplacement)
    /// carrying the draft's selected mounted-weapon key (GTW-574 C4/C6).
    Emplacement,
}

impl TerrainKindChoice {
    /// The kind-segment order, left to right — the labels the kind segmented control renders.
    pub const SEGMENT_ORDER: [Self; 4] = [Self::Wall, Self::Cover, Self::Slab, Self::Emplacement];

    /// The kind at segment `index`, or [`None`] if out of range — the inverse of
    /// [`segment_index`](TerrainKindChoice::segment_index).
    #[must_use]
    pub fn from_segment(index: usize) -> Option<Self> {
        Self::SEGMENT_ORDER.get(index).copied()
    }

    /// This kind's segment position (the index the kind control highlights).
    #[must_use]
    pub fn segment_index(self) -> usize {
        Self::SEGMENT_ORDER
            .iter()
            .position(|kind| *kind == self)
            .unwrap_or(0)
    }

    /// This kind's human segment label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Wall => "Wall",
            Self::Cover => "Cover",
            Self::Slab => "Slab",
            Self::Emplacement => "Emplacement",
        }
    }

    /// Whether this kind carries a [`HeightBand`](gdtf_battle_sim::cover::HeightBand) field
    /// (Wall / Cover / Emplacement do — the Emplacement sim variant carries the band its
    /// occluder + cover-ledger entry use; Slab spans the whole z-boundary so it does NOT).
    #[must_use]
    pub const fn has_height_band(self) -> bool {
        matches!(self, Self::Wall | Self::Cover | Self::Emplacement)
    }

    /// Whether this kind OFFERS a footfall sound (ONLY [`Slab`](TerrainKindChoice::Slab) — a slab
    /// is stepped on; walls / cover / emplacements are not) — the C2 footfall-gate rule.
    #[must_use]
    pub const fn offers_footfall(self) -> bool {
        matches!(self, Self::Slab)
    }
}

impl From<TerrainPieceKind> for TerrainKindChoice {
    /// The COMPILER TIE between the canonical terrain-kind discriminant and the editor's
    /// pick list (GTW-574 C4) — exhaustive, no wildcard: a new [`TerrainPieceKind`]
    /// variant fails to compile HERE until the editor offers it, so the pick list can
    /// never silently drop a kind again.
    fn from(kind: TerrainPieceKind) -> Self {
        match kind {
            TerrainPieceKind::Wall => Self::Wall,
            TerrainPieceKind::Cover => Self::Cover,
            TerrainPieceKind::Slab => Self::Slab,
            TerrainPieceKind::Emplacement => Self::Emplacement,
        }
    }
}

/// The graphic roles the TERRAIN form's picker OFFERS, in [`TileRole::ALL`] order —
/// the presenter's vocabulary filtered by [`TileRole::def_authorable`] (GTW-566 C5).
///
/// This DERIVES the pick list from the one shared vocabulary instead of a hand-mirrored
/// editor enum, so a def-authorable role added to the presenter (the GTW-543
/// emplacement, the four GTW-470 oriented stairs) is offered here with NO editor change
/// — and the runtime-swap / link-direction roles the presenter picks itself
/// (`emplacement_occupied`, `slab_destroyed`, `stair_up`, `stair_down`) plus the
/// unoffered plain `door` are excluded by their flags.
#[must_use]
pub fn offered_graphic_roles() -> Vec<TileRole> {
    TileRole::ALL
        .into_iter()
        .filter(|role| role.def_authorable())
        .collect()
}

/// A footfall-sound choice the form offers for a slab's `presenter_kind.footfall` — a closed
/// pick that includes an explicit `None` (C2 gating: a non-slab kind forces `None`).
///
/// A named closed enum (no-bare-types). There is no footfall AUDIO system yet (memory:
/// *guns-only-no-melee-thrown-yet*), so the offered keys are authored placeholders the future
/// footfall pass consumes; the editor only writes the chosen key into the def.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum FootfallChoice {
    /// No footfall sound — the def's `footfall` is `None`. The forced value for any non-slab kind
    /// (C2).
    #[default]
    None,
    /// A metallic deck-plate footfall (`"footfall_metal"`).
    Metal,
    /// A grated-walkway footfall (`"footfall_grate"`).
    Grate,
}

impl FootfallChoice {
    /// Every footfall choice the dropdown offers, in display order (`None` first).
    pub const ALL: [Self; 3] = [Self::None, Self::Metal, Self::Grate];

    /// The dropdown label for this choice.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::None => "None",
            Self::Metal => "Metal",
            Self::Grate => "Grate",
        }
    }

    /// This choice's footfall key, or [`None`] for the [`None`](FootfallChoice::None) choice.
    #[must_use]
    pub fn footfall(self) -> Option<FootfallSound> {
        match self {
            Self::None => Option::None,
            Self::Metal => Some(FootfallSound::new("footfall_metal".to_owned())),
            Self::Grate => Some(FootfallSound::new("footfall_grate".to_owned())),
        }
    }
}
