//! The injuries family's ONE folder-walk builder — [`build_injury_data`] and its
//! table-fold / audit halves, shared by BOTH hosts' resolve and redrive (GTW-437;
//! moved host-agnostic from the game's `Load` resolve in GTW-654, the GTW-630
//! `validate` precedent) so a live edit, a game boot, and an editor boot all
//! build the resources IDENTICALLY.

use bevy::{
    asset::{AssetServer, Assets, LoadedFolder},
    prelude::warn,
};
use gdtf_assets::RonAsset;
use gdtf_battle_sim::{
    injuries::{
        InjuryDef, InjuryName, InjuryRegistry, InjuryTables, InjuryWeighting, WeightedInjuryEntry,
        WeightedInjuryTable,
    },
    severity::Severity,
};

use super::keying::{injury_key_from_stem, warn_on_subfolder_mismatch};

/// Build BOTH the [`InjuryRegistry`] and the [`InjuryTables`] from a loaded
/// `injuries/` [`LoadedFolder`], or [`None`] if the folder (or any member asset) is
/// not yet in its collection — the bespoke two-resource counterpart of the shared
/// folder walk the GTW-570 content-family seam runs for its single-registry
/// families (injuries stay off the seam by design: one folder resolves into TWO
/// resources).
///
/// Shared by both hosts' one-time resolve AND their GTW-374 live redrive (a hot
/// edit rebuilds through the SAME walk a boot runs):
///
/// - **Partition** the folder's member handles by their compound extension infix: a
///   path ending `.injury.ron` is a `RonAsset<InjuryDef>`, a path ending
///   `.weighting.ron` is a `RonAsset<InjuryWeighting>`. A member with neither infix is
///   skipped defensively.
/// - **Key** each injury by its file STEM minus the `.injury` infix (so
///   `head/lost_eye.injury.ron` keys `lost_eye`), into the [`InjuryRegistry`].
/// - **Validate** each def's authored [`category`](InjuryDef::category) against the owning
///   per-category subfolder name (`injuries/<category>/…`, GTW-440 / GTW-453): a
///   cross-category mismatch only `warn!`s — the
///   injury is STILL loaded using its own `category` field (the def is authoritative,
///   the subfolder is organizational, design fork #10).
/// - **Fold** every weighting file's three severity lists into the [`InjuryTables`],
///   resolving each row's key against the registry: an UNKNOWN key `warn!`s + is
///   skipped (never fails), and the surviving rows are CANONICALLY SORTED (by injury
///   key then weight) so folder-enumeration order can never change the cumulative-weight
///   pick.
/// - **Audit** the missing-weighting rule: any registered injury referenced by NO
///   weighting bucket `warn!`s (it can never be rolled) — a warning, never a failure.
///
/// Returns [`None`] (do NOT publish partial resources) if the folder or any member
/// asset is not yet in its collection — the caller retries next frame.
#[must_use]
pub fn build_injury_data(
    asset_server: &AssetServer,
    folders: &Assets<LoadedFolder>,
    injury_defs: &Assets<RonAsset<InjuryDef>>,
    weightings: &Assets<RonAsset<InjuryWeighting>>,
    folder_handle: &bevy::asset::Handle<LoadedFolder>,
) -> Option<(InjuryRegistry, InjuryTables)> {
    let folder = folders.get(folder_handle)?;

    let mut registry = InjuryRegistry::default();
    let mut authored_weightings: Vec<InjuryWeighting> = Vec::new();

    for untyped in &folder.handles {
        // Resolve the member's asset path so we can both classify it (by extension
        // infix) and key it (by stem). A handle with no resolvable path is skipped
        // defensively (it carries no usable key / type).
        let Some(asset_path) = asset_server.get_path(untyped.id()) else {
            continue;
        };
        let path = asset_path.path();
        let path_str = path.to_string_lossy();

        if path_str.ends_with(".injury.ron") {
            // A per-injury `RonAsset<InjuryDef>`. One-frame race: not yet in the
            // collection — bail so the caller re-polls (do NOT build a partial set).
            let handle = untyped.clone().typed_debug_checked::<RonAsset<InjuryDef>>();
            let def = injury_defs.get(&handle)?;
            let Some(stem) = path
                .file_stem()
                .map(|stem| injury_key_from_stem(&stem.to_string_lossy()))
            else {
                continue;
            };
            // Validate the def's authored category against the owning subfolder
            // (organizational only): a mismatch WARNs but the injury is still loaded.
            warn_on_subfolder_mismatch(&path_str, &stem, def);
            registry.insert(InjuryName::new(stem), (**def).clone());
        } else if path_str.ends_with(".weighting.ron") {
            // A per-part `RonAsset<InjuryWeighting>`. One-frame race as above.
            let handle = untyped
                .clone()
                .typed_debug_checked::<RonAsset<InjuryWeighting>>();
            let weighting = weightings.get(&handle)?;
            authored_weightings.push((**weighting).clone());
        }
        // A member with neither infix is silently skipped (no usable injury type).
    }

    let tables = build_tables(&registry, &authored_weightings);
    audit_unweighted_injuries(&registry, &tables);
    Some((registry, tables))
}

/// Fold the authored per-category [`InjuryWeighting`]s into the [`InjuryTables`], resolving
/// each row's key against the registry (unknown → WARN + skip) and canonically sorting
/// the surviving rows so folder-enumeration order can never change the roll (GTW-437).
pub(super) fn build_tables(
    registry: &InjuryRegistry,
    weightings: &[InjuryWeighting],
) -> InjuryTables {
    let mut tables = InjuryTables::default();
    for weighting in weightings {
        // The three tabled severity buckets (None=graze, Fatal=death are never tabled).
        for (severity, rows) in [
            (Severity::Minor, &weighting.minor),
            (Severity::Major, &weighting.major),
            (Severity::Critical, &weighting.critical),
        ] {
            let mut resolved: Vec<WeightedInjuryEntry> = rows
                .iter()
                .filter(|row| {
                    // A weighting entry naming an UNKNOWN injury key → WARN + skip (no
                    // panic, no load failure — design rule C6).
                    let known = registry.contains(&row.injury);
                    if !known {
                        warn!(
                            "GDTF Load: weighting for {:?} names unknown injury key {:?}; \
                             skipping the row (it resolves to no injury)",
                            severity, &*row.injury,
                        );
                    }
                    known
                })
                .cloned()
                .collect();
            // CANONICAL SORT (by injury key, then weight): the authored Vec order — and
            // hence folder-enumeration order — can never change the built table / the
            // cumulative-weight pick (determinism, design fork #9).
            resolved.sort_by(|a, b| {
                a.injury
                    .cmp(&b.injury)
                    .then_with(|| a.weight.cmp(&b.weight))
            });
            if !resolved.is_empty() {
                tables.insert(
                    weighting.category,
                    severity,
                    WeightedInjuryTable::new(resolved),
                );
            }
        }
    }
    tables
}

/// Audit the missing-weighting rule: a registered injury referenced by NO weighting
/// bucket can never be rolled → `warn!` (a warning, never a load failure — rule C6).
pub(super) fn audit_unweighted_injuries(registry: &InjuryRegistry, tables: &InjuryTables) {
    for (key, def) in registry.iter() {
        let weighted = tables
            .table_for_category(def.category, def.severity)
            .is_some_and(|table| table.iter().any(|row| row.injury == *key));
        if !weighted {
            warn!(
                "GDTF Load: injury {:?} ({:?}/{:?}) is in no weighting table; it can \
                 never be rolled",
                &**key, def.category, def.severity,
            );
        }
    }
}
