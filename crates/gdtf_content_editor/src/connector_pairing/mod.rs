//! The prefab-editor **vertical-connector auto-pairing** rule (GTW-531) — placing an UP connector
//! at `(x, y, N)` ALSO places its paired DOWN connector at `(x, y, N+1)`, so authoring a
//! two-ended vertical connector (stair / ladder) is ONE placement, not two.
//!
//! ## Why this lives here (a prefab-editor placement rule ONLY)
//!
//! A vertical connector is two-ended: the UP endpoint on storey `N` is walkable-up to its DOWN
//! endpoint on storey `N+1`. The author should draw one endpoint and get both. This module is that
//! convenience — it does NOT change the sim terrain model or any runtime logic (the sim still sees
//! two independent placed terrains, exactly as if the author had painted both by hand).
//!
//! ## The up↔down map — how it is resolved (C1), TYPED through the role vocabulary (GTW-566 C6)
//!
//! The unified terrain model ([`TerrainDef`](gdtf_battle_sim::terrain::def::TerrainDef)) does NOT
//! cleanly express an up↔down pairing: a stair's [`sim_kind`](gdtf_battle_sim::terrain::def::TerrainSimKind)
//! is `Slab` (GTW-470), there is NO up/down or NS/EW *direction* field, and the up-vs-down + NS/EW
//! distinction lives ONLY in the presenter-side [`graphic_name`](gdtf_battle_sim::terrain::def::TerrainPresenterKind)
//! (e.g. `stair_ns_up` / `stair_ns_down`, `stair_ew_up` / `stair_ew_down`, and the generic
//! `stair_up` / `stair_down`). So the pairing CANNOT be read from a model field.
//!
//! GTW-531 shipped a graphic-name SUFFIX pairing (`…_up` string-swapped to `…_down`); GTW-566 C6
//! replaces that string surgery with the presenter's TYPED role vocabulary: a def's graphic name
//! classifies through [`TileRole::from_key`](gdtf_battle_presenter::TileRole::from_key), [`TileRole::is_up_connector`](gdtf_battle_presenter::TileRole::is_up_connector) recognises the ASCEND
//! end, and [`TileRole::counterpart`](gdtf_battle_presenter::TileRole::counterpart) names the DOWN role, whose
//! [`as_key`](gdtf_battle_presenter::TileRole::as_key) is then resolved against the loaded
//! [`TerrainDefRegistry`](gdtf_battle_sim::terrain::def::TerrainDefRegistry) at placement time
//! (find the def whose graphic name equals the counterpart key). Behaviour is identical for every
//! in-vocabulary name — `stair_ns_up`↔`stair_ns_down`, `stair_ew_up`↔`stair_ew_down`,
//! `stair_up`↔`stair_down` — and stays keyed off the typed
//! [`TerrainGraphicKey`](gdtf_battle_sim::terrain::piece::TerrainGraphicKey) (no bare string
//! escapes the recognition boundary). **FAIL-CLOSED:** an OUT-OF-VOCABULARY graphic name ending in
//! `_up` (a typo, or a theme inventing e.g. `ladder_up` outside the vocabulary) no longer
//! phantom-pairs — it classifies to no role and places as a plain single tile; extending the
//! pairing means extending the [`TileRole`](gdtf_battle_presenter::TileRole) vocabulary, not naming files.
//!
//! ## The placement behaviour (C2 / C4)
//!
//! [`apply_placement_with_pairing`] REUSES the shared GTW-430
//! [`apply_placement`](crate::placement::apply_placement) predicate VERBATIM — it never
//! re-implements placement or the legality rules. It:
//!
//! 1. Places the requested tile at `(x, y, N)` through `apply_placement` (respecting the existing
//!    out-of-bounds / slab-seals-ladder / ladder-auto-clear rules).
//! 2. IF that landed AND the tile is an UP connector whose DOWN counterpart resolves in the
//!    registry, ALSO places that DOWN counterpart at `(x, y, N+1)` through `apply_placement`
//!    (again, the shared legality rules apply to the pair placement — a conflict there does NOT
//!    silently corrupt: `apply_placement` rejects or auto-clears per its own contract).
//! 3. FAIL-CLOSED at the top: if `N` is the top storey (no `N+1` inside the prefab's level range),
//!    the pair is SKIPPED (log-and-continue) — only the up connector is placed, never above the top.
//!
//! Pairing is **one-way (C4)**: this places both endpoints from one UP placement, but REMOVING the
//! up connector does NOT auto-remove the paired down connector — removal stays manual. (Linked
//! removal is a possible later refinement; shipped default is one-way.)

mod pairing;
mod resolve;

#[cfg(test)]
mod tests;

pub use pairing::{PairingOutcome, apply_placement_with_pairing};
pub use resolve::{is_up_connector, resolve_down_counterpart};
