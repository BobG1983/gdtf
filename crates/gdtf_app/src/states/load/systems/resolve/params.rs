use bevy::{
    asset::LoadedFolder,
    ecs::system::SystemParam,
    prelude::{Assets, Res, ResMut},
};
use gdtf_assets::{ContentIntegrityReport, RonAsset, RonFolderSalvage};
use gdtf_battle_sim::{
    injuries::{InjuryDef, InjuryRegistry, InjuryWeighting},
    level::{PrefabRegistry, PrefabSpec},
};
use gdtf_ui::theme::{GdtfTheme, GdtfThemeSpec};

#[derive(SystemParam)]
pub(in crate::states::load) struct LoadAssetCollections<'w> {
        pub(super) theme:        Option<Res<'w, Assets<RonAsset<GdtfThemeSpec>>>>,
            pub(super) folders:      Option<Res<'w, Assets<LoadedFolder>>>,
        pub(super) injury_defs:  Option<Res<'w, Assets<RonAsset<InjuryDef>>>>,
            pub(super) weightings:   Option<Res<'w, Assets<RonAsset<InjuryWeighting>>>>,
        pub(super) prefab_specs: Option<Res<'w, Assets<RonAsset<PrefabSpec>>>>,
}

#[derive(SystemParam)]
pub(in crate::states::load) struct ResolvedResources<'w> {
        pub(super) theme:    Option<Res<'w, GdtfTheme>>,
                pub(super) injuries: Option<Res<'w, InjuryRegistry>>,
        pub(super) prefabs:  Option<Res<'w, PrefabRegistry>>,
}

#[derive(SystemParam)]
pub(in crate::states::load) struct SalvageStates<'w> {
        pub(super) injury_defs:       Option<Res<'w, RonFolderSalvage<InjuryDef>>>,
        pub(super) injury_weightings: Option<Res<'w, RonFolderSalvage<InjuryWeighting>>>,
        pub(super) prefabs:           Option<Res<'w, RonFolderSalvage<PrefabSpec>>>,
            pub(super) report:            Option<ResMut<'w, ContentIntegrityReport>>,
}
