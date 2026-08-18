//! Terrain form draft resource.

use bevy::prelude::Resource;
use gdtf_battle_presenter::TileRole;
use gdtf_battle_sim::{
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverHp, HeightBand},
    effects::on_death::OnDeathEffect,
    slab::SlabHp,
    terrain::{
        def::{LosBlocking, TerrainTag, TerrainUuid},
        facing::TerrainFacing,
    },
    weapon::WeaponName,
};

use super::picks::{FootfallChoice, TerrainKindChoice};

/// In-progress terrain def being authored.
#[derive(Resource, Clone, Debug, PartialEq)]
pub struct TerrainDraft {
    display_name:     String,
    kind:             TerrainKindChoice,
    cover_hp:         CoverHp,
    slab_hp:          SlabHp,
    armor_protection: ArmorProtection,
    armor_hardness:   ArmorHardness,
    height_band:      HeightBand,
    graphic:          TileRole,
    footfall:         FootfallChoice,
    mounted_weapon:   Option<WeaponName>,
    entry_sides:      Vec<TerrainFacing>,
    tags:             Vec<TerrainTag>,
    blocks_pathing:   Option<bool>,
    blocks_los:       Option<LosBlocking>,
    on_death:         Option<OnDeathEffect>,
    uuid:             Option<TerrainUuid>,
}

impl TerrainDraft {
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

    /// Presenter graphic role.
    #[must_use]
    pub const fn graphic(&self) -> TileRole {
        self.graphic
    }

    /// Set the graphic role.
    pub const fn set_graphic(&mut self, graphic: TileRole) {
        self.graphic = graphic;
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

    /// On-death effect, if authored.
    #[must_use]
    pub const fn on_death(&self) -> Option<&OnDeathEffect> {
        self.on_death.as_ref()
    }

    /// Set the on-death effect.
    pub fn set_on_death(&mut self, on_death: Option<OnDeathEffect>) {
        self.on_death = on_death;
    }

    /// Mutable on-death effect for the form.
    pub const fn on_death_mut(&mut self) -> &mut Option<OnDeathEffect> {
        &mut self.on_death
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
            graphic:          TileRole::Floor,
            footfall:         FootfallChoice::default(),
            mounted_weapon:   None,
            entry_sides:      Vec::new(),
            tags:             Vec::new(),
            blocks_pathing:   None,
            blocks_los:       None,
            on_death:         None,
            uuid:             None,
        }
    }
}
