//! Catalog handles and the result of battle setup.

use bevy::prelude::Deref;

use crate::{
    armor::ArmorRegistry,
    effects::fields::FieldDefRegistry,
    equipment::attachments::AttachmentRegistry,
    ganger::GangRegistry,
    occupancy::OccupantPlacement,
    terrain::def::TerrainDefRegistry,
    tuning::GangerStatTuning,
    weapon::{MeleeWeaponRegistry, WeaponRegistry},
};

/// Number of gangers spawned.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct GangerCount(usize);

impl GangerCount {
    /// Wrap a count.
    #[must_use]
    pub const fn new(count: usize) -> Self {
        Self(count)
    }
}

/// Result of a successful battle setup.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BattleSetup {
    /// Occupant placements written into the occupancy grid.
    pub occupants: Vec<OccupantPlacement>,
}

impl BattleSetup {
    /// How many gangers were spawned.
    #[must_use]
    pub const fn ganger_count(&self) -> GangerCount {
        GangerCount::new(self.occupants.len())
    }
}

/// Borrowed catalogs needed to resolve a situation.
#[derive(Clone, Copy)]
pub struct BattleRegistries<'a> {
    /// Gang rosters.
    pub gangs: &'a GangRegistry,
    /// Ranged weapons.
    pub weapons: &'a WeaponRegistry,
    /// Melee weapons.
    pub melee_weapons: &'a MeleeWeaponRegistry,
    /// Armor pieces.
    pub armor: &'a ArmorRegistry,
    /// Ganger stat defaults.
    pub stat_tuning: &'a GangerStatTuning,
    /// Terrain piece catalog.
    pub terrain: Option<&'a TerrainDefRegistry>,
    /// Area-damage field catalog.
    pub fields: Option<&'a FieldDefRegistry>,
    /// Attachment catalog.
    pub attachments: Option<&'a AttachmentRegistry>,
}

impl<'a> BattleRegistries<'a> {
    /// Core catalogs without optional field/attachment registries.
    #[must_use]
    pub const fn new(
        gangs: &'a GangRegistry,
        weapons: &'a WeaponRegistry,
        melee_weapons: &'a MeleeWeaponRegistry,
        armor: &'a ArmorRegistry,
        stat_tuning: &'a GangerStatTuning,
        terrain: Option<&'a TerrainDefRegistry>,
    ) -> Self {
        Self {
            gangs,
            weapons,
            melee_weapons,
            armor,
            stat_tuning,
            terrain,
            fields: None,
            attachments: None,
        }
    }

    /// Attach a field catalog.
    #[must_use]
    pub const fn with_field_defs(mut self, fields: &'a FieldDefRegistry) -> Self {
        self.fields = Some(fields);
        self
    }

    /// Attach an attachment catalog.
    #[must_use]
    pub const fn with_attachments(mut self, attachments: &'a AttachmentRegistry) -> Self {
        self.attachments = Some(attachments);
        self
    }
}
