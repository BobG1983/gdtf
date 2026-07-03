//! The TERRAIN form's **in-progress authoring draft** (GTW-474; split out of
//! `types.rs` in GTW-574): the state-scoped [`TerrainDraft`] resource the form's
//! controls write and the save reads + projects into a
//! [`TerrainDef`](gdtf_battle_sim::terrain::def::TerrainDef).

use bevy::prelude::Resource;
use gdtf_battle_presenter::TileRole;
use gdtf_battle_sim::{
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverHp, HeightBand},
    slab::SlabHp,
    terrain::def::{TerrainTag, TerrainUuid},
    weapon::WeaponName,
};

use super::picks::{FootfallChoice, TerrainKindChoice};

/// The in-progress TERRAIN-mode authoring DRAFT — the state-scoped resource the form's controls
/// write and the save reads + projects into a [`TerrainDef`](gdtf_battle_sim::terrain::def::TerrainDef)
/// (GTW-474 C2).
///
/// A state-scoped [`Resource`] (inserted `OnEnter(Editing)`, removed `OnExit(Editing)` —
/// bevy-traps #1). Every field is a domain value (no-bare-types): the stats reuse the sim
/// newtypes ([`CoverHp`] / [`SlabHp`] / [`ArmorProtection`] / [`ArmorHardness`] / [`HeightBand`]),
/// the kind / footfall are the form's closed pick enums, the graphic is the presenter's
/// [`TileRole`] directly (GTW-566 C5), the mounted weapon is the sim's [`WeaponName`]
/// registry-key newtype (GTW-574 C5 — Emplacement only), the tags are sim
/// [`TerrainTag`]s, and the UUID is a [`TerrainUuid`] (generated on first save — [`uuid`]).
///
/// [`uuid`]: TerrainDraft::uuid
#[derive(Resource, Clone, Debug, PartialEq, Eq)]
pub struct TerrainDraft {
    /// The display name (the text field). Folded to a file stem on save.
    display_name:     String,
    /// The chosen structural kind — drives which stat fields show + footfall enablement (C2).
    kind:             TerrainKindChoice,
    /// The HP magnitude for a Wall / Cover / Emplacement kind ([`CoverHp`]).
    cover_hp:         CoverHp,
    /// The HP magnitude for a Slab kind ([`SlabHp`], the distinct slab pool).
    slab_hp:          SlabHp,
    /// The armor-protection stat (shared across kinds).
    armor_protection: ArmorProtection,
    /// The armor-hardness stat (shared across kinds).
    armor_hardness:   ArmorHardness,
    /// The clearance band (Wall / Cover / Emplacement only; a Slab carries none).
    height_band:      HeightBand,
    /// The chosen graphic role (the sprite picker) — a def-authorable [`TileRole`].
    graphic:          TileRole,
    /// The chosen footfall sound — OFFERED only for a Slab kind; forced to
    /// [`None`](FootfallChoice::None) otherwise (C2).
    footfall:         FootfallChoice,
    /// The selected mounted-weapon registry key (GTW-574 C5) — OFFERED only for the
    /// Emplacement kind (the [`WeaponRegistry`](gdtf_battle_sim::weapon::WeaponRegistry)-backed
    /// dropdown, Q1); [`None`] until the author picks one, and cleared when the kind
    /// leaves Emplacement (the footfall-gate precedent). The save FAILS CLOSED on an
    /// Emplacement draft with no selection (C6).
    mounted_weapon:   Option<WeaponName>,
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
    /// [`None`](FootfallChoice::None) when the new kind does not offer footfall (C2 fail-closed)
    /// and clears the mounted weapon when the new kind is not Emplacement (GTW-574 C5 — the
    /// same fail-closed gate, so a stale selection never survives a kind switch).
    pub fn set_kind(&mut self, kind: TerrainKindChoice) {
        self.kind = kind;
        if !kind.offers_footfall() {
            self.footfall = FootfallChoice::None;
        }
        if !matches!(kind, TerrainKindChoice::Emplacement) {
            self.mounted_weapon = None;
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

    /// The selected mounted-weapon registry key, or [`None`] when unset / not an
    /// Emplacement kind (GTW-574 C5).
    #[must_use]
    pub const fn mounted_weapon(&self) -> Option<&WeaponName> {
        self.mounted_weapon.as_ref()
    }

    /// Set the mounted-weapon selection (from the registry-backed dropdown) — IGNORED
    /// when the current kind is not Emplacement, so the Emplacement-only rule holds
    /// fail-closed even if a stale dropdown commit arrives (the C2 footfall precedent).
    pub fn set_mounted_weapon(&mut self, weapon: Option<WeaponName>) {
        if matches!(self.kind, TerrainKindChoice::Emplacement) {
            self.mounted_weapon = weapon;
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
    /// footfall, no mounted weapon, no tags, and no UUID (minted on first save).
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
            mounted_weapon:   None,
            tags:             Vec::new(),
            uuid:             None,
        }
    }
}
