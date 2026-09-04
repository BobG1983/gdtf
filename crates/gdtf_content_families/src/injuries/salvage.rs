//! Per-file salvage when the injuries folder fails to load as a whole.

use bevy::{
    asset::{AssetServer, Assets},
    prelude::{Commands, warn},
};
use cobalt_ron_assets::RonAsset;
use gdtf_assets::{
    ContentIntegrityReport, FindingFamily, RonFolderSalvage, RonSalvagePoll,
    begin_ron_folder_salvage, poll_ron_folder_salvage, report_malformed_members,
};
use gdtf_battle_sim::injuries::{
    InjuryDef, InjuryName, InjuryRegistry, InjuryTables, InjuryWeighting,
};

use super::{
    build::{audit_unweighted_injuries, build_tables},
    keying::{injury_key_from_stem, warn_on_subfolder_mismatch},
    layout::{INJURIES_FOLDER, INJURY_DEF_EXTENSION, INJURY_WEIGHTING_EXTENSION},
};

/// When both salvage polls settle, insert the registry and tables.
pub fn settle_injuries_salvage(
    commands: &mut Commands,
    asset_server: &AssetServer,
    injury_defs: &Assets<RonAsset<InjuryDef>>,
    weightings: &Assets<RonAsset<InjuryWeighting>>,
    def_salvage: &RonFolderSalvage<InjuryDef>,
    weighting_salvage: &RonFolderSalvage<InjuryWeighting>,
    report: Option<&mut ContentIntegrityReport>,
) {
    let def_poll = poll_ron_folder_salvage(def_salvage, asset_server, injury_defs);
    let weighting_poll = poll_ron_folder_salvage(weighting_salvage, asset_server, weightings);
    if let (
        RonSalvagePoll::Settled {
            loaded: loaded_defs,
            malformed: malformed_defs,
        },
        RonSalvagePoll::Settled {
            loaded: loaded_weightings,
            malformed: malformed_weightings,
        },
    ) = (def_poll, weighting_poll)
    {
        let mut registry = InjuryRegistry::default();
        for member in &loaded_defs {
            let Some(stem) = std::path::Path::new(member.path.as_str())
                .file_stem()
                .map(|stem| injury_key_from_stem(&stem.to_string_lossy()))
            else {
                continue;
            };
            warn_on_subfolder_mismatch(member.path.as_str(), &stem, member.spec);
            registry.insert(InjuryName::new(stem), (**member.spec).clone());
        }
        let authored: Vec<InjuryWeighting> = loaded_weightings
            .iter()
            .map(|member| (**member.spec).clone())
            .collect();
        let tables = build_tables(&registry, &authored);
        audit_unweighted_injuries(&registry, &tables);
        let mut report = report;
        report_malformed_members(
            report.as_deref_mut(),
            &FindingFamily::new("InjuryRegistry".to_owned()),
            malformed_defs,
        );
        report_malformed_members(
            report,
            &FindingFamily::new("InjuryTables".to_owned()),
            malformed_weightings,
        );
        commands.insert_resource(registry);
        commands.insert_resource(tables);
    }
}

/// Start per-file salvage of injury defs and weightings after a folder load failure.
pub fn begin_injuries_salvage(commands: &mut Commands, asset_server: &AssetServer) {
    let def_salvage =
        begin_ron_folder_salvage::<InjuryDef>(asset_server, INJURIES_FOLDER, INJURY_DEF_EXTENSION);
    let weighting_salvage = begin_ron_folder_salvage::<InjuryWeighting>(
        asset_server,
        INJURIES_FOLDER,
        INJURY_WEIGHTING_EXTENSION,
    );
    match (def_salvage, weighting_salvage) {
        (Ok(defs), Ok(weightings)) if !(defs.is_empty() && weightings.is_empty()) => {
            warn!(
                "GDTF Load: the `injuries` folder failed to load; salvaging its members \
                 per-file into the InjuryRegistry + InjuryTables",
            );
            commands.insert_resource(defs);
            commands.insert_resource(weightings);
        }
        _ => {
            warn!(
                "GDTF Load: the `injuries` folder failed to load; inserting an empty \
                 InjuryRegistry + InjuryTables (the injury roll will find no table and \
                 inflict no injury)",
            );
            commands.insert_resource(InjuryRegistry::default());
            commands.insert_resource(InjuryTables::default());
        }
    }
}
