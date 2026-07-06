//! The editor's **placement-legality rules** — the SINGLE SHARED predicate both the hover-ghost
//! preview and the click-commit run, plus the vertical auto-handling for multi-level tiles
//! (GTW-430; swept onto the UUID model in GTW-495).
//!
//! ## One source of truth (C3)
//!
//! [`evaluate_placement`] is the ONE legality function. The hover-ghost preview calls it to decide
//! whether to tint the preview RED (an illegal placement) and the click-commit calls it to decide
//! whether to REJECT the paint — there is no second copy of the rule. (The egui viewport that
//! consumes both is the GTW-512 C4 child; the predicate is the shared FOUNDATION it builds on.) Both
//! consume the same `(map, registry, theme, placement)` inputs and the same [`PlacementVerdict`].
//!
//! ## The vertical rules
//!
//! The editor classifies each terrain definition into an [`EditorTileClass`] (see [`classify`]):
//!
//! - A **slab** is identified by the sim's own [`TerrainSimKind::Slab`](gdtf_battle_sim::terrain::def::TerrainSimKind::Slab) — semantics-driven off the
//!   terrain registry, not a magic key.
//! - A **ladder** is a vertical link between storeys ([`LinkKind::Ladder`](gdtf_battle_sim::terrain::vertical::LinkKind) in the sim).
//!   The unified terrain model has no ladder *kind* (its [`TerrainSimKind`](gdtf_battle_sim::terrain::def::TerrainSimKind) is WALL/COVER/SLAB/EMPLACEMENT), so
//!   the editor recognises a ladder by a DATA-driven convention: a terrain def whose
//!   [`TerrainDisplayName`](gdtf_battle_sim::terrain::def::TerrainDisplayName) reads as a ladder
//!   ([`names_a_ladder`]). This is not bound to one specific UUID — any theme that adds a
//!   ladder-named terrain is recognised — and is documented as the chosen recognition because the
//!   sim kind does not model ladders.
//!
//! Two symmetric rules over those classes (`docs/combat/combat.md`: gangers change storeys only over
//! authored stair/ladder links; a slab seals a z-boundary):
//!
//! - **C1 auto-handling — placing a LADDER auto-clears a SLAB directly above it.** A ladder placed
//!   at `(cell, level)` connects up to `(cell, level + 1)`; a slab there would seal the ladder's
//!   destination. The latest authoring intent wins (the [`EditorMap`](crate::editor_map::EditorMap) repaint-overwrites precedent),
//!   so the editor AUTO-CLEARS the slab above rather than rejecting the ladder. The placement is
//!   therefore LEGAL, and [`apply_placement`] performs the clear.
//! - **C2 illegal — placing a SLAB onto a cell whose ladder it would seal is REJECTED** (red tint).
//!   The inverse of C1: a slab dropped where it would seal a ladder is rejected. A slab is illegal
//!   when the ladder it would seal sits at the SAME slot (a ladder rises THROUGH the cell) OR
//!   directly BELOW it (the ladder's destination).
//!
//! Out-of-bounds is also illegal (the [`EditorMap`](crate::editor_map::EditorMap) clamp, surfaced through the verdict so the one
//! predicate covers every rejection).

mod classification;
mod rules;
mod verdict;

#[cfg(test)]
mod tests;

pub use classification::{EditorTileClass, classify, names_a_ladder};
pub use rules::{apply_placement, evaluate_placement};
pub use verdict::{IllegalReason, PlacementVerdict, ProposedPlacement};
