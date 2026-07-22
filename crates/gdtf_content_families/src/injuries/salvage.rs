//! The injuries family's GTW-582 C4 **per-file salvage** halves — the salvage
//! MACHINERY itself lives ONCE in `gdtf_assets`; these two fns are
//! only the injuries-shaped fold over it (one folder → two asset types → two
//! resources, settled atomically). Moved host-agnostic from the game's `Load`
//! resolve in GTW-654 (the GTW-630 `validate` precedent) so the content
//! editor's injuries pass salvages a broken folder IDENTICALLY.

use bevy::{
    asset::{AssetServer, Assets},
    prelude::{Commands, warn},
};
use gdtf_assets::{
    ContentIntegrityReport, FindingFamily, RonAsset, RonFolderSalvage, RonSalvagePoll,
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

/// Settle the injuries family's TWO per-file salvages (defs + weightings) —
/// when BOTH have settled, fold the loaded defs into the [`InjuryRegistry`],
/// build the [`InjuryTables`] from the loaded weightings (the same
/// `build_tables` + audit the folder path runs), report every malformed member
/// (loudly), and insert both resources atomically. While EITHER salvage is
/// still pending, nothing is published (the caller re-polls next frame).
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

/// Begin the injuries family's per-file salvage on a `Failed` folder walk
/// (GTW-582 C4): one salvage per asset type over the SAME `content/injuries`
/// tree. When at least one member file enumerates, both salvage resources are
/// inserted (the settle above waits for both); a folder that cannot be
/// enumerated at all still fails closed to the EMPTY resources, so the host's
/// load always exits with both present (the roll then fails closed — no injury
/// rolled — rather than crashing).
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
