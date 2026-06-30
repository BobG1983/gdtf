//! The TERRAIN-mode form's **type vocabulary** (GTW-474): the in-progress
//! [`TerrainDraft`] resource, the closed pick enums the form's controls are generic over
//! (kind / graphic-role / footfall), the field/widget identity markers, and the
//! [`SaveTerrainError`] failure enum.
//!
//! Every pick enum is a NAMED closed enum (no-bare-types) so the GTW-410 dropdown / GTW-280
//! segmented control can be generic over it. The kind/graphic/footfall choices map onto the
//! sim's real [`TerrainSimKind`](gdtf_battle_sim::terrain::def::TerrainSimKind) /
//! [`TerrainGraphicKey`](gdtf_battle_sim::terrain::piece::TerrainGraphicKey) /
//! [`FootfallSound`](gdtf_battle_sim::terrain::piece::FootfallSound) when the draft projects to a
//! [`TerrainDef`].

use bevy::prelude::*;
use gdtf_battle_sim::{
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverHp, HeightBand},
    slab::SlabHp,
    terrain::{
        def::{TerrainTag, TerrainUuid},
        piece::{FootfallSound, TerrainGraphicKey},
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

/// A graphic-role KEY the form offers for `presenter_kind.graphic_name` — a closed pick over the
/// presenter's `TileRoles` vocabulary (NOT a raw atlas index).
///
/// A named closed enum (no-bare-types) so the picker is generic over it. Each variant's
/// [`key`](TerrainGraphicChoice::key) is the exact `tile_roles.ron` role string the presenter's
/// [`TileRoles::index_for_key`](gdtf_battle_presenter::TileRoles::index_for_key) resolves, so an
/// authored def draws the same sprite the battlescape does. The set mirrors the resolvable role
/// keys.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum TerrainGraphicChoice {
    /// The default walkable-ground tile (`"floor"`).
    #[default]
    Floor,
    /// The bolted-panel floor alternate (`"floor_alt_panel"`).
    FloorAltPanel,
    /// The north-south wall tile (`"wall"`).
    Wall,
    /// The east-west wall tile (`"wall_ew"`).
    WallEw,
    /// The chest-high cover tile (`"cover"`).
    Cover,
    /// The elevated-deck slab tile (`"slab"`).
    Slab,
    /// The broken-debris rubble tile (`"rubble"`).
    Rubble,
    /// The north-south door tile (`"door_ns"`).
    DoorNs,
    /// The east-west door tile (`"door_ew"`).
    DoorEw,
    /// The ladder cell tile (`"ladder"`).
    Ladder,
}

impl TerrainGraphicChoice {
    /// Every graphic-role choice the picker offers, in display order.
    pub const ALL: [Self; 10] = [
        Self::Floor,
        Self::FloorAltPanel,
        Self::Wall,
        Self::WallEw,
        Self::Cover,
        Self::Slab,
        Self::Rubble,
        Self::DoorNs,
        Self::DoorEw,
        Self::Ladder,
    ];

    /// The `tile_roles.ron` role-key STRING this choice resolves to — the exact key the
    /// presenter's `index_for_key` matches (so the editor + battlescape draw the same sprite).
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Floor => "floor",
            Self::FloorAltPanel => "floor_alt_panel",
            Self::Wall => "wall",
            Self::WallEw => "wall_ew",
            Self::Cover => "cover",
            Self::Slab => "slab",
            Self::Rubble => "rubble",
            Self::DoorNs => "door_ns",
            Self::DoorEw => "door_ew",
            Self::Ladder => "ladder",
        }
    }

    /// This choice's [`TerrainGraphicKey`] (the sim newtype the projected
    /// [`TerrainDef`](gdtf_battle_sim::terrain::def::TerrainDef) carries).
    #[must_use]
    pub fn graphic_key(self) -> TerrainGraphicKey {
        TerrainGraphicKey::new(self.key().to_owned())
    }
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
/// the kind / graphic / footfall are the form's closed pick enums, the tags are sim
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
    /// The chosen graphic-role key (the sprite picker).
    graphic:          TerrainGraphicChoice,
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

    /// The chosen graphic-role choice.
    #[must_use]
    pub const fn graphic(&self) -> TerrainGraphicChoice {
        self.graphic
    }

    /// Set the chosen graphic-role choice (from the sprite picker).
    pub const fn set_graphic(&mut self, graphic: TerrainGraphicChoice) {
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
            graphic:          TerrainGraphicChoice::default(),
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

/// Identity marker on the TERRAIN form's KIND segmented control root (no-bare-types unit
/// marker), so the form's drive system filters a [`SegmentSelected`](gdtf_ui::SegmentSelected) to
/// the kind control. `pub` (re-exported) so the C4 integration test can drive a kind
/// `SegmentSelected` to it on the real code path.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub struct TerrainKindTabs;

/// Identity marker on the TERRAIN form's HEIGHT-BAND segmented control root (no-bare-types unit
/// marker).
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct TerrainBandTabs;

/// Identity marker on the TERRAIN form's display-name text field (no-bare-types unit marker).
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct TerrainNameField;

/// Identity marker on the TERRAIN form's HP numeric field (no-bare-types unit marker).
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct TerrainHpField;

/// Identity marker on the TERRAIN form's armor-protection numeric field (no-bare-types unit
/// marker).
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct TerrainArmorProtField;

/// Identity marker on the TERRAIN form's armor-hardness numeric field (no-bare-types unit
/// marker).
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct TerrainArmorHardField;

/// Identity marker on the TERRAIN form's graphic-role dropdown root (no-bare-types unit marker).
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct TerrainGraphicPicker;

/// Identity marker on the TERRAIN form's footfall dropdown root (no-bare-types unit marker). The
/// footfall gate adds/removes `DisabledButton` on it (C2 Slab-only). `pub` (re-exported) so the
/// C4 integration test can assert the `DisabledButton` gate flips with the kind.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub struct TerrainFootfallPicker;

/// Identity marker on ONE tag toggle button, carrying which [`TerrainTag`] it toggles (C2
/// multi-select). NOT a bare tag — the tag is the sim domain value (no-bare-types).
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct TerrainTagToggle {
    /// The tag this button toggles.
    tag: TerrainTag,
}

impl TerrainTagToggle {
    /// Build a tag-toggle marker for a tag.
    #[must_use]
    pub(crate) const fn new(tag: TerrainTag) -> Self {
        Self { tag }
    }

    /// The tag this button toggles.
    #[must_use]
    pub(crate) const fn tag(self) -> TerrainTag {
        self.tag
    }
}

/// Identity marker on the "Save terrain" button (no-bare-types unit marker). A PLAIN marker
/// (`pub(crate)` — no external test names it).
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct SaveTerrainButton;

/// Marker on the TERRAIN form's read-only UUID text node — rewritten with the minted UUID after
/// the first save (C2). A unit marker (no-bare-types).
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct TerrainUuidText;

/// Marker on the TERRAIN form's read-only `.terrain_def.ron` PREVIEW text node — rewritten each
/// draft change with the serialized def (the live preview). A unit marker (no-bare-types).
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct TerrainRonPreview;

/// Why a terrain save was REJECTED — the handled, no-panic failure of the terrain save path
/// (GTW-474). A named domain enum (no-bare-types). `pub` because the `pub`
/// [`serialize_terrain_def`](crate::serialize_terrain_def) returns it (the C4 test reuses the
/// projection + serialization seam).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaveTerrainError {
    /// The author entered no display name (an empty / whitespace-only field) — there is no file
    /// stem to write to.
    EmptyName,
    /// Serializing the built [`TerrainDef`](gdtf_battle_sim::terrain::def::TerrainDef) to RON
    /// failed.
    Serialize(String),
    /// Writing the serialized def to disk failed (a missing dir / permissions error / io).
    Write(String),
}

impl std::fmt::Display for SaveTerrainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyName => write!(f, "no terrain name entered — nothing to save"),
            Self::Serialize(err) => write!(f, "failed to serialize the terrain def: {err}"),
            Self::Write(err) => write!(f, "failed to write the terrain-def file: {err}"),
        }
    }
}

impl std::error::Error for SaveTerrainError {}
