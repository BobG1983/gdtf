//! The INJURIES family's authored-content layout vocabulary (GTW-634 C4) — the
//! one-owner folder / extension / per-category-directory spellings every
//! read-side site (the GTW-437 kick-off, the loader registrations, the GTW-582
//! per-file salvage) AND the editor's write-side save path (GTW-654) import, so
//! no site hand-maintains a mirror spelling.

use gdtf_battle_sim::{armor::InjuryCategory, injuries::DamageContext};

/// The injuries content root, relative to the asset source root — one
/// recursive folder carrying the per-injury `<category>/*.injury.ron` defs AND
/// the `weighting/*.weighting.ron` tables (GTW-437).
pub const INJURIES_FOLDER: &str = "content/injuries";

/// The dedicated compound extension the per-injury def loader is registered
/// for (`init_ron_asset_with_extensions::<InjuryDef>`) — keeps the mixed
/// injuries folder's dispatch unambiguous.
pub const INJURY_DEF_EXTENSION: &str = "injury.ron";

/// The dedicated compound extension the per-part weighting-table loader is
/// registered for (`init_ron_asset_with_extensions::<InjuryWeighting>`) —
/// the second asset type sharing the [`INJURIES_FOLDER`] tree.
pub const INJURY_WEIGHTING_EXTENSION: &str = "weighting.ron";

/// The subdirectory of [`INJURIES_FOLDER`] the per-category weighting tables
/// live in (`content/injuries/weighting/<category>.weighting.ron`) — the
/// write-side owner the editor's weighting save resolves against (GTW-654).
/// The LOADER never reads this spelling (its dispatch is extension-based), so
/// only the save path and the shipped layout convention use it.
pub const WEIGHTING_SUBFOLDER: &str = "weighting";

/// The canonical per-category subdirectory name of [`INJURIES_FOLDER`] an
/// [`InjuryCategory`]'s defs live in (`head` / `torso` / `arm` / `leg`,
/// GTW-440) — the ONE owner of those spellings: the loader's organizational
/// subfolder audit derives its needles from this, and the editor's def save
/// (GTW-654) resolves its target directory through it, so a saved def always
/// lands in the subfolder the audit expects.
#[must_use]
pub const fn category_dir(category: InjuryCategory) -> &'static str {
    match category {
        InjuryCategory::Head => "head",
        InjuryCategory::Torso => "torso",
        InjuryCategory::Arm => "arm",
        InjuryCategory::Leg => "leg",
    }
}

/// The per-source file-name INFIX a weighting table's [`DamageContext`] contributes
/// (GTW-452) — the ONE owner of the shipped
/// `weighting/<category>[.<context>].weighting.ron` convention.
///
/// [`Ranged`](DamageContext::Ranged) contributes NOTHING (`head.weighting.ron`) — it is the
/// schema's default context, so the pre-GTW-452 file name keeps naming the ranged table;
/// the other two contribute their lowercase source word (`head.melee.weighting.ron` /
/// `head.fall.weighting.ron`). The loader dispatches on the trailing
/// [`INJURY_WEIGHTING_EXTENSION`] alone (Bevy walks the secondary extensions), so the infix
/// is a naming convention for authors + the editor's save path, never a parse input — the
/// authored `context:` field inside the file is what keys the built bucket.
#[must_use]
pub const fn weighting_context_infix(context: DamageContext) -> Option<&'static str> {
    match context {
        DamageContext::Ranged => None,
        DamageContext::Melee => Some("melee"),
        DamageContext::Fall => Some("fall"),
    }
}
