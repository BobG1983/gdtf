//! The injuries branch's file-KEYING helpers — the stem→key rule and the
//! organizational per-category subfolder audit, shared by the folder walk
//! ([`build_injury_data`](super::build_injury_data)) and the GTW-582 per-file
//! salvage fold (`injuries/salvage.rs`).

use bevy::prelude::warn;
use gdtf_battle_sim::injuries::InjuryDef;

/// The injury KEY for a loaded injury file's stem — the stem with the dedicated
/// `.injury` infix stripped (GTW-437).
///
/// An injury file is `<key>.injury.ron`; Bevy's `file_stem()` yields `<key>.injury`,
/// so the KEY (the [`InjuryName`] a
/// [`WeightedInjuryEntry`](gdtf_battle_sim::injuries::WeightedInjuryEntry) references)
/// is that stem minus a trailing `.injury`. A stem without the infix is returned
/// unchanged (defensive — keeps a mis-named file's key its plain stem).
pub(super) fn injury_key_from_stem(stem: &str) -> String {
    stem.strip_suffix(".injury").unwrap_or(stem).to_owned()
}

/// `warn!` if a loaded injury's authored [`category`](InjuryDef::category) differs from
/// the [`InjuryCategory`](gdtf_battle_sim::armor::InjuryCategory) the per-category
/// subfolder it lives in names (`injuries/<category>/<key>.injury.ron`, GTW-440 / GTW-453).
///
/// The subfolder is ORGANIZATIONAL only — the def's own `category` field is authoritative
/// (design fork #10) — so a mismatch is a content-authoring smell worth a warning, NEVER a
/// load failure: the injury is still loaded under its own field. Since GTW-440 the folders
/// are per-CATEGORY (`head` / `torso` / `arm` / `leg`); GTW-453 authors the category
/// DIRECTLY, so the comparison is the def's `category` against the subfolder's — an
/// `Arm`-declared injury under `arm/` is consistent, only a cross-category misfile WARNs.
pub(super) fn warn_on_subfolder_mismatch(path_str: &str, key: &str, def: &InjuryDef) {
    let Some(subfolder) = subfolder_injury_category(path_str) else {
        // No recognised per-category subfolder (e.g. a flat layout) — nothing to compare.
        return;
    };
    if subfolder != def.category {
        warn!(
            "GDTF Load: injury {key:?} declares category {:?} but lives in the {:?} \
             subfolder; loading it under its authoritative field ({:?})",
            def.category, subfolder, def.category,
        );
    }
}

/// The [`InjuryCategory`](gdtf_battle_sim::armor::InjuryCategory) a
/// `injuries/<category>/…` path's per-category subfolder names, or [`None`] if the path
/// names no recognised category subfolder (GTW-440).
///
/// Maps the canonical per-category subfolder names (`head` / `torso` / `arm` / `leg`) to
/// their [`InjuryCategory`](gdtf_battle_sim::armor::InjuryCategory). The two arms / the
/// two legs share ONE folder each (the shared-pool restructure), so there is no
/// per-side subfolder anymore.
fn subfolder_injury_category(path_str: &str) -> Option<gdtf_battle_sim::armor::InjuryCategory> {
    use gdtf_battle_sim::armor::InjuryCategory;
    // Normalise to forward slashes so the match works on every platform.
    let normalised = path_str.replace('\\', "/");
    [
        ("/head/", InjuryCategory::Head),
        ("/torso/", InjuryCategory::Torso),
        ("/arm/", InjuryCategory::Arm),
        ("/leg/", InjuryCategory::Leg),
    ]
    .into_iter()
    .find_map(|(needle, category)| normalised.contains(needle).then_some(category))
}
