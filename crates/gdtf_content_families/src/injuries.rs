//! The INJURIES family's authored-content layout vocabulary (GTW-634 C4).
//!
//! Injuries are one of the two declared GTW-570 seam EXCLUSIONS: ONE folder
//! carries TWO asset types (per-injury defs + per-part weighting tables)
//! resolved into TWO resources, so there is no `ContentFamily` impl to hang
//! the folder / extensions on. Its layout consts therefore live here as plain
//! consts — the same one-owner rule as the seam families' `FOLDER` /
//! `EXTENSION` associated consts — imported by every read-side site (the
//! GTW-437 kick-off, the loader registrations, the GTW-582 per-file salvage),
//! so no site hand-maintains a mirror spelling.

/// The injuries content root, relative to the asset source root — one
/// recursive folder carrying the per-injury `<part>/*.injury.ron` defs AND
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
