use bevy::{
    asset::{AssetEvent, AssetServer, LoadedFolder},
    ecs::system::SystemParam,
    prelude::{Assets, Deref, MessageReader, Res, ResMut},
};
use cobalt_ron_assets::RonAsset;
use gdtf_assets::{ContentIntegrityReport, RonFolderSalvage};
use gdtf_battle_sim::{
    injuries::{InjuryDef, InjuryRegistry, InjuryTables, InjuryWeighting},
    level::{PrefabRegistry, PrefabSpec},
};
use gdtf_content_families::injuries::build_injury_data;
use gdtf_ui::theme::{GdtfTheme, GdtfThemeSpec};

use crate::states::load::resources::ActiveInjuriesFolderHandle;

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

/// The asset edits that could make the injury tables stale.
#[derive(SystemParam)]
pub(in crate::states::load) struct InjuryAssetEdits<'w, 's> {
    defs:       MessageReader<'w, 's, AssetEvent<RonAsset<InjuryDef>>>,
    weightings: MessageReader<'w, 's, AssetEvent<RonAsset<InjuryWeighting>>>,
}

/// Whether an injury def or weighting was rewritten on disk.
#[derive(Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct InjuriesRewritten(bool);

impl InjuriesRewritten {
    /// Wrap the asset-event answer.
    const fn new(rewritten: bool) -> Self {
        Self(rewritten)
    }
}

impl InjuryAssetEdits<'_, '_> {
    /// Whether an injury def or weighting was rewritten on disk.
    pub(super) fn any_modified(&mut self) -> InjuriesRewritten {
        let defs = self
            .defs
            .read()
            .any(|event| matches!(event, AssetEvent::Modified { .. }));
        let weightings = self
            .weightings
            .read()
            .any(|event| matches!(event, AssetEvent::Modified { .. }));
        InjuriesRewritten::new(defs || weightings)
    }

    /// Drop the pending edits without acting on them.
    pub(super) fn discard(&mut self) {
        self.defs.clear();
        self.weightings.clear();
    }
}

/// What a rebuild of the injury tables reads.
#[derive(SystemParam)]
pub(in crate::states::load) struct InjuryRebuildSources<'w> {
    asset_server: Option<Res<'w, AssetServer>>,
    folder:       Option<Res<'w, ActiveInjuriesFolderHandle>>,
    folders:      Option<Res<'w, Assets<LoadedFolder>>>,
    defs:         Option<Res<'w, Assets<RonAsset<InjuryDef>>>>,
    weightings:   Option<Res<'w, Assets<RonAsset<InjuryWeighting>>>>,
}

impl InjuryRebuildSources<'_> {
    /// Rebuild the registry and tables, or `None` while a source is missing.
    pub(super) fn rebuild(&self) -> Option<(InjuryRegistry, InjuryTables)> {
        build_injury_data(
            self.asset_server.as_deref()?,
            self.folders.as_deref()?,
            self.defs.as_deref()?,
            self.weightings.as_deref()?,
            self.folder.as_deref()?,
        )
    }
}
