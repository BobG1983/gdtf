//! The terrain draft's own fields and their accessors.

use bevy::prelude::Resource;
use gdtf_assets::ContentSourcePath;
use gdtf_battle_sim::{
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverHp, HeightBand},
    effects::on_death::OnDeathEffect,
    slab::SlabHp,
    terrain::{
        def::{
            LeavesBehind, LosBlocking, TerrainTag, TerrainUuid, TerrainView, TerrainViewArt,
            TerrainViews,
        },
        facing::TerrainFacing,
        piece::TerrainGraphicKey,
    },
    weapon::WeaponName,
};

use crate::terrain_form::picks::{FootfallChoice, TerrainKindChoice};

/// In-progress terrain def being authored.
#[derive(Resource, Clone, Debug, PartialEq)]
pub struct TerrainDraft {
    display_name:        String,
    kind:                TerrainKindChoice,
    cover_hp:            CoverHp,
    slab_hp:             SlabHp,
    armor_protection:    ArmorProtection,
    armor_hardness:      ArmorHardness,
    height_band:         HeightBand,
    views:               TerrainViews,
    footfall:            FootfallChoice,
    mounted_weapon:      Option<WeaponName>,
    entry_sides:         Vec<TerrainFacing>,
    tags:                Vec<TerrainTag>,
    blocks_pathing:      Option<bool>,
    blocks_los:          Option<LosBlocking>,
    leaves_behind:       LeavesBehind,
    pub(super) on_death: Vec<OnDeathEffect>,
    uuid:                Option<TerrainUuid>,
    source:              Option<ContentSourcePath>,
}

impl TerrainDraft {
    /// Range a cover or slab HP input offers.
    pub const HP_RANGE: core::ops::RangeInclusive<u32> = 0..=1000;

    /// Range an armor protection or hardness input offers.
    pub const ARMOR_RANGE: core::ops::RangeInclusive<i32> = 0..=100;

    /// Display name.
    #[must_use]
    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    /// Set the display name.
    pub fn set_display_name(&mut self, name: String) {
        self.display_name = name;
    }

    /// Terrain kind choice.
    #[must_use]
    pub const fn kind(&self) -> TerrainKindChoice {
        self.kind
    }

    /// Set kind; clears footfall / mounted weapon / entry sides when they no longer apply.
    pub fn set_kind(&mut self, kind: TerrainKindChoice) {
        self.kind = kind;
        if !kind.offers_footfall() {
            self.footfall = FootfallChoice::None;
        }
        if !matches!(kind, TerrainKindChoice::Emplacement) {
            self.mounted_weapon = None;
            self.entry_sides = Vec::new();
        }
    }

    /// The per-view art rows the draft holds.
    #[must_use]
    pub const fn views(&self) -> &TerrainViews {
        &self.views
    }

    /// Replace every art row, the way loading an existing def fills the draft.
    pub fn replace_views(&mut self, views: TerrainViews) {
        self.views = views;
    }

    /// Name the sprite one view is drawn with, replacing any row that view already has.
    pub fn set_view(&mut self, view: TerrainView, sprite: TerrainGraphicKey) {
        let row = TerrainViewArt { view, sprite };
        let mut rows = (*self.views).clone();
        match rows.iter_mut().find(|held| held.view == view) {
            Some(held) => *held = row,
            None => rows.push(row),
        }
        self.views = TerrainViews::new(rows);
    }

    /// Footfall choice for slabs.
    #[must_use]
    pub const fn footfall(&self) -> FootfallChoice {
        self.footfall
    }

    /// Set footfall when the kind supports it.
    pub const fn set_footfall(&mut self, footfall: FootfallChoice) {
        if self.kind.offers_footfall() {
            self.footfall = footfall;
        }
    }

    /// Mounted weapon for emplacements.
    #[must_use]
    pub const fn mounted_weapon(&self) -> Option<&WeaponName> {
        self.mounted_weapon.as_ref()
    }

    /// Set mounted weapon when the kind is emplacement.
    pub fn set_mounted_weapon(&mut self, weapon: Option<WeaponName>) {
        if matches!(self.kind, TerrainKindChoice::Emplacement) {
            self.mounted_weapon = weapon;
        }
    }

    /// Sides an emplacement can be entered from, unrotated.
    #[must_use]
    pub fn entry_sides(&self) -> &[TerrainFacing] {
        &self.entry_sides
    }

    /// Set the entry sides when the kind is emplacement.
    pub fn set_entry_sides(&mut self, sides: Vec<TerrainFacing>) {
        if matches!(self.kind, TerrainKindChoice::Emplacement) {
            self.entry_sides = sides;
        }
    }

    /// Cover / wall HP.
    #[must_use]
    pub const fn cover_hp(&self) -> CoverHp {
        self.cover_hp
    }

    /// Set cover HP.
    pub const fn set_cover_hp(&mut self, hp: CoverHp) {
        self.cover_hp = hp;
    }

    /// Slab HP.
    #[must_use]
    pub const fn slab_hp(&self) -> SlabHp {
        self.slab_hp
    }

    /// Set slab HP.
    pub const fn set_slab_hp(&mut self, hp: SlabHp) {
        self.slab_hp = hp;
    }

    /// Armor protection.
    #[must_use]
    pub const fn armor_protection(&self) -> ArmorProtection {
        self.armor_protection
    }

    /// Set armor protection.
    pub const fn set_armor_protection(&mut self, protection: ArmorProtection) {
        self.armor_protection = protection;
    }

    /// Armor hardness.
    #[must_use]
    pub const fn armor_hardness(&self) -> ArmorHardness {
        self.armor_hardness
    }

    /// Set armor hardness.
    pub const fn set_armor_hardness(&mut self, hardness: ArmorHardness) {
        self.armor_hardness = hardness;
    }

    /// Height band for cover/wall.
    #[must_use]
    pub const fn height_band(&self) -> HeightBand {
        self.height_band
    }

    /// Set height band.
    pub const fn set_height_band(&mut self, band: HeightBand) {
        self.height_band = band;
    }

    /// Terrain tags.
    #[must_use]
    pub fn tags(&self) -> &[TerrainTag] {
        &self.tags
    }

    /// Whether a tag is selected.
    #[must_use]
    pub fn has_tag(&self, tag: TerrainTag) -> bool {
        self.tags.contains(&tag)
    }

    /// Toggle a tag on or off.
    pub fn toggle_tag(&mut self, tag: TerrainTag) {
        if let Some(pos) = self.tags.iter().position(|t| *t == tag) {
            self.tags.remove(pos);
        } else {
            self.tags.push(tag);
        }
    }

    /// Replace the selected tags.
    pub fn replace_tags(&mut self, tags: Vec<TerrainTag>) {
        self.tags = tags;
    }

    /// Optional pathing override.
    #[must_use]
    pub const fn blocks_pathing(&self) -> Option<bool> {
        self.blocks_pathing
    }

    /// Set pathing override.
    pub const fn set_blocks_pathing(&mut self, blocks_pathing: Option<bool>) {
        self.blocks_pathing = blocks_pathing;
    }

    /// Optional LOS override.
    #[must_use]
    pub const fn blocks_los(&self) -> Option<LosBlocking> {
        self.blocks_los
    }

    /// Set LOS override.
    pub const fn set_blocks_los(&mut self, blocks_los: Option<LosBlocking>) {
        self.blocks_los = blocks_los;
    }

    /// What destroying this piece leaves standing in its cell.
    #[must_use]
    pub const fn leaves_behind(&self) -> &LeavesBehind {
        &self.leaves_behind
    }

    /// Set what destroying this piece leaves behind.
    pub fn set_leaves_behind(&mut self, leaves_behind: LeavesBehind) {
        self.leaves_behind = leaves_behind;
    }

    /// Assigned terrain uuid, if any.
    #[must_use]
    pub const fn uuid(&self) -> Option<TerrainUuid> {
        self.uuid
    }

    /// Assign a terrain uuid.
    pub const fn set_uuid(&mut self, uuid: Option<TerrainUuid>) {
        self.uuid = uuid;
    }

    /// Ensure a uuid exists and return it.
    pub fn ensure_uuid(&mut self) -> TerrainUuid {
        *self.uuid.get_or_insert_with(TerrainUuid::generate)
    }

    /// The file this draft was opened from, when it was opened from one.
    #[must_use]
    pub const fn source(&self) -> Option<&ContentSourcePath> {
        self.source.as_ref()
    }

    /// Record the file this draft was opened from, so the next save writes it.
    pub fn set_source(&mut self, source: Option<ContentSourcePath>) {
        self.source = source;
    }
}

impl Default for TerrainDraft {
    fn default() -> Self {
        Self {
            display_name:     String::new(),
            kind:             TerrainKindChoice::default(),
            cover_hp:         CoverHp::new(40),
            slab_hp:          SlabHp::new(50),
            armor_protection: ArmorProtection::new(4),
            armor_hardness:   ArmorHardness::new(2),
            height_band:      HeightBand::High,
            views:            TerrainViews::new(Vec::new()),
            footfall:         FootfallChoice::default(),
            mounted_weapon:   None,
            entry_sides:      Vec::new(),
            tags:             Vec::new(),
            blocks_pathing:   None,
            blocks_los:       None,
            leaves_behind:    LeavesBehind::Nothing,
            on_death:         Vec::new(),
            uuid:             None,
            source:           None,
        }
    }
}
