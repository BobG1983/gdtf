//! [`Situation`](crate::situation::Situation)'s authored references against.

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

#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct GangerCount(usize);

impl GangerCount {
        #[must_use]
    pub const fn new(count: usize) -> Self {
        Self(count)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BattleSetup {
        pub occupants: Vec<OccupantPlacement>,
}

impl BattleSetup {
        #[must_use]
    pub const fn ganger_count(&self) -> GangerCount {
        GangerCount::new(self.occupants.len())
    }
}

#[derive(Clone, Copy)]
pub struct BattleRegistries<'a> {
        pub gangs:         &'a GangRegistry,
        pub weapons:       &'a WeaponRegistry,
                pub melee_weapons: &'a MeleeWeaponRegistry,
        pub armor:         &'a ArmorRegistry,
        pub stat_tuning:   &'a GangerStatTuning,
                pub terrain:       Option<&'a TerrainDefRegistry>,
                                    pub fields:        Option<&'a FieldDefRegistry>,
                                        pub attachments:   Option<&'a AttachmentRegistry>,
}

impl<'a> BattleRegistries<'a> {
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

                            #[must_use]
    pub const fn with_field_defs(mut self, fields: &'a FieldDefRegistry) -> Self {
        self.fields = Some(fields);
        self
    }

                            #[must_use]
    pub const fn with_attachments(mut self, attachments: &'a AttachmentRegistry) -> Self {
        self.attachments = Some(attachments);
        self
    }
}
