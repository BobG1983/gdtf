//! The TERRAIN-mode form's **type vocabulary** (GTW-474): the in-progress
//! [`TerrainDraft`] resource, the closed pick enums the form's controls are generic over
//! (kind / footfall), the graphic-role pick over the presenter's [`TileRole`] vocabulary,
//! and the [`SaveTerrainError`] failure enum.
//!
//! Every pick enum is a NAMED closed enum (no-bare-types) so the GTW-410 dropdown / GTW-280
//! segmented control can be generic over it. The kind/footfall choices map onto the
//! sim's real [`TerrainSimKind`](gdtf_battle_sim::terrain::def::TerrainSimKind) /
//! [`FootfallSound`](gdtf_battle_sim::terrain::piece::FootfallSound) when the draft projects to a
//! [`TerrainDef`]. The graphic pick is the presenter's [`TileRole`] DIRECTLY (GTW-566 C5)
//! — the editor no longer maintains a hand-mirrored role enum with its own key strings;
//! the picker offers [`offered_graphic_roles`] (the vocabulary filtered by
//! [`TileRole::def_authorable`]), and the draft projects the chosen role's
//! [`as_key`](TileRole::as_key) into the def's `graphic_name`.

use bevy::prelude::*;
use gdtf_battle_presenter::TileRole;
use gdtf_battle_sim::{
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverHp, HeightBand},
    slab::SlabHp,
    terrain::{
        def::{TerrainTag, TerrainUuid},
        piece::FootfallSound,
    },
};

/// Which structural KIND the authored terrain is — the THREE sim kinds the form offers
/// (no `Floor` / `Scatter`: those are RETIRED on the UUID model — GTW-476).
///
/// A named closed enum (no-bare-types). The variant ORDER is the segmented-control segment
/// order, so [`from_segment`](TerrainKindChoice::from_segment) maps a chosen index back to a
/// kind. Each maps to a [`TerrainSimKind`](gdtf_battle_sim::terrain::def::TerrainSimKind) variant
/// when the draft projects.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum TerrainKindChoice {
    /// Solid blocking geometry — projects to [`TerrainSimKind::Wall`](gdtf_battle_sim::terrain::def::TerrainSimKind::Wall).
    #[default]
    Wall,
    /// Chest-high cover / scatter — projects to [`TerrainSimKind::Cover`](gdtf_battle_sim::terrain::def::TerrainSimKind::Cover).
    Cover,
    /// A floor/roof slab — projects to [`TerrainSimKind::Slab`](gdtf_battle_sim::terrain::def::TerrainSimKind::Slab).
    Slab,
}

impl TerrainKindChoice {
    /// The kind-segment order, left to right — the labels the kind segmented control renders.
    pub const SEGMENT_ORDER: [Self; 3] = [Self::Wall, Self::Cover, Self::Slab];

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
        }
    }

    /// Whether this kind carries a [`HeightBand`] field (Wall / Cover do; Slab spans the whole
    /// z-boundary so it does NOT).
    #[must_use]
    pub const fn has_height_band(self) -> bool {
        matches!(self, Self::Wall | Self::Cover)
    }

    /// Whether this kind OFFERS a footfall sound (ONLY [`Slab`](TerrainKindChoice::Slab) — a slab
    /// is stepped on; walls / cover are not) — the C2 footfall-gate rule.
    #[must_use]
    pub const fn offers_footfall(self) -> bool {
        matches!(self, Self::Slab)
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

/// The in-progress TERRAIN-mode authoring DRAFT — the state-scoped resource the form's controls
/// write and the save reads + projects into a [`TerrainDef`](gdtf_battle_sim::terrain::def::TerrainDef)
/// (GTW-474 C2).
///
/// A state-scoped [`Resource`] (inserted `OnEnter(Editing)`, removed `OnExit(Editing)` —
/// bevy-traps #1). Every field is a domain value (no-bare-types): the stats reuse the sim
/// newtypes ([`CoverHp`] / [`SlabHp`] / [`ArmorProtection`] / [`ArmorHardness`] / [`HeightBand`]),
/// the kind / footfall are the form's closed pick enums, the graphic is the presenter's
/// [`TileRole`] directly (GTW-566 C5), the tags are sim
/// [`TerrainTag`]s, and the UUID is a [`TerrainUuid`] (generated on first save — [`uuid`]).
///
/// [`uuid`]: TerrainDraft::uuid
#[derive(Resource, Clone, Debug, PartialEq, Eq)]
pub struct TerrainDraft {
    /// The display name (the text field). Folded to a file stem on save.
    display_name:     String,
    /// The chosen structural kind — drives which stat fields show + footfall enablement (C2).
    kind:             TerrainKindChoice,
    /// The HP magnitude for a Wall / Cover kind ([`CoverHp`]).
    cover_hp:         CoverHp,
    /// The HP magnitude for a Slab kind ([`SlabHp`], the distinct slab pool).
    slab_hp:          SlabHp,
    /// The armor-protection stat (shared across kinds).
    armor_protection: ArmorProtection,
    /// The armor-hardness stat (shared across kinds).
    armor_hardness:   ArmorHardness,
    /// The clearance band (Wall / Cover only; a Slab carries none).
    height_band:      HeightBand,
    /// The chosen graphic role (the sprite picker) — a def-authorable [`TileRole`].
    graphic:          TileRole,
    /// The chosen footfall sound — OFFERED only for a Slab kind; forced to
    /// [`None`](FootfallChoice::None) otherwise (C2).
    footfall:         FootfallChoice,
    /// The multi-selected tags — a set (deduped, order-stable on save).
    tags:             Vec<TerrainTag>,
    /// The auto-generated UUID — `None` until the FIRST save MINTS it (C2: editor-generated,
    /// shown read-only). A re-save reuses the minted key so the def keeps a stable identity.
    uuid:             Option<TerrainUuid>,
}

impl TerrainDraft {
    /// The display name.
    #[must_use]
    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    /// Set the display name (committed from the text field).
    pub fn set_display_name(&mut self, name: String) {
        self.display_name = name;
    }

    /// The chosen kind.
    #[must_use]
    pub const fn kind(&self) -> TerrainKindChoice {
        self.kind
    }

    /// Set the chosen kind (from the kind segmented control). Forces footfall to
    /// [`None`](FootfallChoice::None) when the new kind does not offer footfall (C2 fail-closed).
    pub const fn set_kind(&mut self, kind: TerrainKindChoice) {
        self.kind = kind;
        if !kind.offers_footfall() {
            self.footfall = FootfallChoice::None;
        }
    }

    /// The chosen graphic role.
    #[must_use]
    pub const fn graphic(&self) -> TileRole {
        self.graphic
    }

    /// Set the chosen graphic role (from the sprite picker).
    pub const fn set_graphic(&mut self, graphic: TileRole) {
        self.graphic = graphic;
    }

    /// The chosen footfall choice (always [`None`](FootfallChoice::None) for a non-slab kind).
    #[must_use]
    pub const fn footfall(&self) -> FootfallChoice {
        self.footfall
    }

    /// Set the footfall choice — IGNORED (kept [`None`](FootfallChoice::None)) when the current
    /// kind does not offer footfall, so the C2 Slab-only rule holds fail-closed even if a stale
    /// dropdown commit arrives.
    pub const fn set_footfall(&mut self, footfall: FootfallChoice) {
        if self.kind.offers_footfall() {
            self.footfall = footfall;
        }
    }

    /// The cover/wall HP magnitude.
    #[must_use]
    pub const fn cover_hp(&self) -> CoverHp {
        self.cover_hp
    }

    /// Set the cover/wall HP magnitude (committed from the HP numeric field).
    pub const fn set_cover_hp(&mut self, hp: CoverHp) {
        self.cover_hp = hp;
    }

    /// The slab HP magnitude.
    #[must_use]
    pub const fn slab_hp(&self) -> SlabHp {
        self.slab_hp
    }

    /// Set the slab HP magnitude (committed from the slab-HP numeric field).
    pub const fn set_slab_hp(&mut self, hp: SlabHp) {
        self.slab_hp = hp;
    }

    /// The armor-protection stat.
    #[must_use]
    pub const fn armor_protection(&self) -> ArmorProtection {
        self.armor_protection
    }

    /// Set the armor-protection stat (committed from its numeric field).
    pub const fn set_armor_protection(&mut self, protection: ArmorProtection) {
        self.armor_protection = protection;
    }

    /// The armor-hardness stat.
    #[must_use]
    pub const fn armor_hardness(&self) -> ArmorHardness {
        self.armor_hardness
    }

    /// Set the armor-hardness stat (committed from its numeric field).
    pub const fn set_armor_hardness(&mut self, hardness: ArmorHardness) {
        self.armor_hardness = hardness;
    }

    /// The clearance band.
    #[must_use]
    pub const fn height_band(&self) -> HeightBand {
        self.height_band
    }

    /// Set the clearance band (from the band segmented control).
    pub const fn set_height_band(&mut self, band: HeightBand) {
        self.height_band = band;
    }

    /// The selected tags.
    #[must_use]
    pub fn tags(&self) -> &[TerrainTag] {
        &self.tags
    }

    /// Whether a tag is currently selected.
    #[must_use]
    pub fn has_tag(&self, tag: TerrainTag) -> bool {
        self.tags.contains(&tag)
    }

    /// Toggle a tag in the multi-select set — add it if absent, remove it if present (C2).
    pub fn toggle_tag(&mut self, tag: TerrainTag) {
        if let Some(pos) = self.tags.iter().position(|t| *t == tag) {
            self.tags.remove(pos);
        } else {
            self.tags.push(tag);
        }
    }

    /// The auto-generated UUID, or [`None`] until the first save mints it.
    #[must_use]
    pub const fn uuid(&self) -> Option<TerrainUuid> {
        self.uuid
    }

    /// MINT the UUID if not yet set (the first-save auto-generate — C2), returning the key.
    /// Idempotent: a re-save reuses the existing key so the def keeps a stable identity.
    pub fn ensure_uuid(&mut self) -> TerrainUuid {
        *self.uuid.get_or_insert_with(TerrainUuid::generate)
    }
}

impl Default for TerrainDraft {
    /// A fresh draft: an empty name, a Wall kind, modest default stats, the `floor` graphic, no
    /// footfall, no tags, and no UUID (minted on first save).
    fn default() -> Self {
        Self {
            display_name:     String::new(),
            kind:             TerrainKindChoice::default(),
            cover_hp:         CoverHp::new(40),
            slab_hp:          SlabHp::new(50),
            armor_protection: ArmorProtection::new(4),
            armor_hardness:   ArmorHardness::new(2),
            height_band:      HeightBand::High,
            graphic:          TileRole::Floor,
            footfall:         FootfallChoice::default(),
            tags:             Vec::new(),
            uuid:             Option::None,
        }
    }
}

/// An HP magnitude the TERRAIN form's HP numeric field edits, clamps, and commits — the
/// [`NumericValue`](gdtf_ui::NumericValue) generic the field is built over (GTW-474).
///
/// A named newtype over [`u32`] (no-bare-types rule 1: a numeric-field generic is a domain value,
/// never a bare `u32`; the sim's [`CoverHp`] / [`SlabHp`] do not impl `Display` / `FromStr`, so
/// this thin input newtype satisfies the [`NumericValue`](gdtf_ui::NumericValue) bound and is
/// converted into the right HP newtype on commit). Private inner + derived [`Deref`].
#[derive(Deref, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct HpInput(u32);

impl HpInput {
    /// Wrap an HP magnitude.
    #[must_use]
    pub const fn new(hp: u32) -> Self {
        Self(hp)
    }
}

impl core::fmt::Display for HpInput {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl core::str::FromStr for HpInput {
    type Err = core::num::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<u32>().map(Self)
    }
}

/// An armor magnitude the TERRAIN form's armor numeric fields edit, clamp, and commit — the
/// [`NumericValue`](gdtf_ui::NumericValue) generic the protection + hardness fields share
/// (GTW-474).
///
/// A named newtype over [`i32`] (no-bare-types rule 1; the sim's [`ArmorProtection`] /
/// [`ArmorHardness`] do not impl `Display` / `FromStr`). Private inner + derived [`Deref`];
/// converted into the right armor newtype on commit.
#[derive(Deref, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct ArmorInput(i32);

impl ArmorInput {
    /// Wrap an armor magnitude.
    #[must_use]
    pub const fn new(armor: i32) -> Self {
        Self(armor)
    }
}

impl core::fmt::Display for ArmorInput {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl core::str::FromStr for ArmorInput {
    type Err = core::num::ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<i32>().map(Self)
    }
}

// GTW-512: the TERRAIN form's `bevy_ui` WIDGET MARKERS (the kind/band segmented-control roots, the
// name/HP/armor field markers, the graphic/footfall picker roots, the per-tag toggle, the save
// button, the read-only UUID + RON-preview text markers) were DROPPED in the egui swap — they
// marked `bevy_ui` entities that no longer exist. The C2 child (GTW-513) re-creates egui-native
// equivalents; this file keeps the MODEL (the draft, the kind/graphic/footfall pick enums, the
// input newtypes, the error enum).

/// Why a terrain save was REJECTED — the handled, no-panic failure of the terrain save path
/// (GTW-474). A named domain enum (no-bare-types). `pub` because the `pub`
/// [`serialize_terrain_def`](crate::serialize_terrain_def) returns it (the C4 test reuses the
/// projection + serialization seam). The domain-validation variant stays bespoke (GTW-577
/// P9); the serialize/write tail collapsed onto the shared
/// [`RonSaveError`](gdtf_assets::RonSaveError), wrapped by [`Save`](Self::Save).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaveTerrainError {
    /// The author entered no display name (an empty / whitespace-only field) — there is no file
    /// stem to write to.
    EmptyName,
    /// The shared serialize/write tail failed (GTW-577 C3) — wraps the seam's
    /// [`RonSaveError`](gdtf_assets::RonSaveError), whose `Display` names the failed stage.
    Save(gdtf_assets::RonSaveError),
}

impl From<gdtf_assets::RonSaveError> for SaveTerrainError {
    /// The per-type conversion off the shared seam error (GTW-577 C3) — lets the save path
    /// `?` a seam failure straight into the form's error.
    fn from(err: gdtf_assets::RonSaveError) -> Self {
        Self::Save(err)
    }
}

impl std::fmt::Display for SaveTerrainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyName => write!(f, "no terrain name entered — nothing to save"),
            Self::Save(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for SaveTerrainError {}
