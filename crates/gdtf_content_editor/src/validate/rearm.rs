//! Re-arm authoring-time validation when watched content registries change.
use bevy::{
    ecs::system::SystemParam,
    prelude::{Commands, DetectChanges, Res},
};
use gdtf_assets::{ContentChecksComplete, ContentIntegrityReport, ContentValidationDone};
use gdtf_battle_sim::{
    armor::ArmorRegistry,
    equipment::attachments::AttachmentRegistry,
    ganger::GangRegistry,
    injuries::{InjuryRegistry, InjuryTables},
    level::{PrefabRegistry, UuidThemeRegistry},
    terrain::def::TerrainDefRegistry,
    weapon::{MeleeWeaponRegistry, WeaponRegistry},
};
use gdtf_content_families::sprites::SpriteDefRegistry;

/// Watched content registries for change detection.
#[derive(SystemParam)]
pub(super) struct WatchedRegistries<'w> {
    weapons:       Option<Res<'w, WeaponRegistry>>,
    melee_weapons: Option<Res<'w, MeleeWeaponRegistry>>,
    armor:         Option<Res<'w, ArmorRegistry>>,
    gangs:         Option<Res<'w, GangRegistry>>,
    terrain:       Option<Res<'w, TerrainDefRegistry>>,
    themes:        Option<Res<'w, UuidThemeRegistry>>,
    injuries:      Option<Res<'w, InjuryRegistry>>,
    sprite_defs:   Option<Res<'w, SpriteDefRegistry>>,
    attachments:   Option<Res<'w, AttachmentRegistry>>,
    prefabs:       Option<Res<'w, PrefabRegistry>>,
    injury_tables: Option<Res<'w, InjuryTables>>,
}

impl WatchedRegistries<'_> {
    fn any_changed(&self) -> bool {
        self.weapons.as_ref().is_some_and(DetectChanges::is_changed)
            || self
                .melee_weapons
                .as_ref()
                .is_some_and(DetectChanges::is_changed)
            || self.armor.as_ref().is_some_and(DetectChanges::is_changed)
            || self.gangs.as_ref().is_some_and(DetectChanges::is_changed)
            || self.terrain.as_ref().is_some_and(DetectChanges::is_changed)
            || self.themes.as_ref().is_some_and(DetectChanges::is_changed)
            || self
                .injuries
                .as_ref()
                .is_some_and(DetectChanges::is_changed)
            || self
                .sprite_defs
                .as_ref()
                .is_some_and(DetectChanges::is_changed)
            || self
                .attachments
                .as_ref()
                .is_some_and(DetectChanges::is_changed)
            || self.prefabs.as_ref().is_some_and(DetectChanges::is_changed)
            || self
                .injury_tables
                .as_ref()
                .is_some_and(DetectChanges::is_changed)
    }
}

pub(super) fn rearm_validation_on_content_change(
    done: Option<Res<ContentValidationDone>>,
    watched: WatchedRegistries,
    mut commands: Commands,
) {
    if done.is_none() {
        return;
    }
    if watched.any_changed() {
        commands.insert_resource(ContentIntegrityReport::default());
        commands.remove_resource::<ContentChecksComplete>();
        commands.remove_resource::<ContentValidationDone>();
    }
}
