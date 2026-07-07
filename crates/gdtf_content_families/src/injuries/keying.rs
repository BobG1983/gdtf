//! The injuries family's file-KEYING helpers — the stem→key rule and the
//! organizational per-category subfolder audit, shared by the folder walk
//! ([`build_injury_data`](super::build_injury_data)) and the GTW-582 per-file
//! salvage fold ([`salvage`](super::salvage)). Moved host-agnostic from the
//! game's `Load` resolve in GTW-654 (the GTW-630 `validate` precedent) so the
//! content editor's injuries pass keys files IDENTICALLY.

use bevy::prelude::warn;
use gdtf_battle_sim::{armor::InjuryCategory, injuries::InjuryDef};

use super::layout::category_dir;

/// The injury KEY for a loaded injury file's stem — the stem with the dedicated
/// `.injury` infix stripped (GTW-437).
///
/// An injury file is `<key>.injury.ron`; Bevy's `file_stem()` yields `<key>.injury`,
/// so the KEY (the [`InjuryName`](gdtf_battle_sim::injuries::InjuryName) a
/// [`WeightedInjuryEntry`](gdtf_battle_sim::injuries::WeightedInjuryEntry) references)
/// is that stem minus a trailing `.injury`. A stem without the infix is returned
/// unchanged (defensive — keeps a mis-named file's key its plain stem).
pub(super) fn injury_key_from_stem(stem: &str) -> String {
    stem.strip_suffix(".injury").unwrap_or(stem).to_owned()
}

/// `warn!` if a loaded injury's authored [`category`](InjuryDef::category) differs from
/// the [`InjuryCategory`] the per-category subfolder it lives in names
/// (`injuries/<category>/<key>.injury.ron`, GTW-440 / GTW-453).
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

/// The [`InjuryCategory`] a `injuries/<category>/…` path's per-category subfolder
/// names, or [`None`] if the path names no recognised category subfolder (GTW-440).
///
/// The needle spellings derive from the one-owner [`category_dir`] (GTW-654), so the
/// audit can never drift from the save path's target directory. The two arms / the
/// two legs share ONE folder each (the shared-pool restructure), so there is no
/// per-side subfolder anymore.
fn subfolder_injury_category(path_str: &str) -> Option<InjuryCategory> {
    // Normalise to forward slashes so the match works on every platform.
    let normalised = path_str.replace('\\', "/");
    InjuryCategory::ALL.into_iter().find(|category| {
        let needle = format!("/{}/", category_dir(*category));
        normalised.contains(&needle)
    })
}
