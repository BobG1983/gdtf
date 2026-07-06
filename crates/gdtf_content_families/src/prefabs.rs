//! The PREFABS family's authored-content layout vocabulary (GTW-634 C4).
//!
//! Prefabs are one of the two declared GTW-570 seam EXCLUSIONS: the fragments
//! live in a NESTED `<theme>/<size>/` tree and resolve into a bucketed
//! multimap registry, so there is no `ContentFamily` impl to hang the folder /
//! extension on. Its layout consts therefore live here as plain consts — the
//! same one-owner rule as the seam families' `FOLDER` / `EXTENSION` associated
//! consts — imported by BOTH the game's bespoke Load chain (kick-off, resolve,
//! salvage, loader registration) and the map editor's save path, so the write
//! side can never drift from the read side (the GTW-621 gang-extension bug
//! class).

/// The prefab fragments' content root, relative to the asset source root —
/// the `<theme>/<size>/` subfolders nest under this (GTW-489; moved under
/// `content/` by GTW-556). The GTW-562 change-class (move a content root) is
/// a ONE-site edit: here.
pub const PREFABS_FOLDER: &str = "content/maps";

/// The dedicated compound extension the prefab loader is registered for
/// (`init_ron_asset_with_extensions::<PrefabSpec>`), keeping Bevy's
/// extension-based folder dispatch unambiguous among GDTF's many `.ron`
/// loaders (the GTW-257 precedent). A saved prefab MUST use it or the loader
/// never picks the file up; the loader strips the trailing `.prefab` infix
/// from the stem to recover the prefab name.
pub const PREFAB_EXTENSION: &str = "prefab.ron";
