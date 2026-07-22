//! The input-populated [`PathPreview`] read-side resource (E7 · GTW-12j).
//!
//! # The C6 read path (input → presenter → sim)
//!
//! The PRESENTER owns the read-side resource ([`PathPreview`], the route [`CellLevel`] list +
//! the previewed [`Tu`] cost) plus the draw system; the INPUT crate (which alone may
//! read `SelectedShooter` + the new `PathPreviewTarget`, and which builds the GTW-353
//! [`PlanningView`](gdtf_battle_sim::pathfinder::PlanningView) from the sim's `SquadVisibility`)
//! calls [`find_path`](gdtf_battle_sim::pathfinder::find_path) and POPULATES this resource (clearing
//! it when there is no target, or the target is unreachable / only-through-UNSEEN).
//! Selection + target are NEVER pushed into the authoritative sim model, and the
//! dependency direction stays `input → presenter → sim` — the SAME shape as the
//! [`HighlightRequest`](crate::HighlightRequest)
//! message (the presenter DEFINES the type; the input crate WRITES it). The route the
//! input crate computes uses the SAME [`find_path`](gdtf_battle_sim::pathfinder::find_path) +
//! `PlanningView` the move dispatch ([`dispatch_move`](gdtf_battle_sim::acts::dispatch_move))
//! uses, so the previewed route + its cost EXACTLY match what a commit will accept
//! (GTW-354/GTW-355 dependency) and the route NEVER enters an UNSEEN cell.
//!
//! # §48 cost identity (visibility.md "Pay-per-step TUs")
//!
//! The previewed cost is `path.total()` — the SAME §48 bit-identity number GTW-355's
//! `advance_walk` charges (the summed per-step entry costs), so the price the player sees
//! before committing equals what the walk spends by construction.

use bevy::prelude::*;
use gdtf_battle_sim::prelude::{CellLevel, Tu};

/// The presenter-owned route path-preview read-side resource — the cells of the
/// [`find_path`](gdtf_battle_sim::pathfinder::find_path) route from the SELECTED ganger to the target,
/// plus the previewed TU cost (C1 / C6).
///
/// A named domain value (no-bare-types: the previewed route + its cost is a domain value),
/// holding the route [`CellLevel`] list (`start..=goal` in step order, exactly
/// [`Path::cells`](gdtf_battle_sim::pathfinder::Path::cells)) and the previewed [`Tu`] cost (exactly
/// [`Path::total`](gdtf_battle_sim::pathfinder::Path::total) — the §48 bit-identity GTW-355 charges).
/// OWNED BY THE PRESENTER so the `input → presenter → sim` direction holds: the draw system
/// READS it; the input crate's `populate_path_preview` POPULATES it (the
/// [`HighlightRequest`](crate::HighlightRequest) precedent). `init_resource`-d by the
/// [`TopDownRendererPlugin`](crate::TopDownRendererPlugin), so its [`Default`] is the empty
/// preview (no target → nothing drawn).
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct PathPreview {
    /// The route cells, `start..=goal` in step order — exactly
    /// [`Path::cells`](gdtf_battle_sim::pathfinder::Path::cells). Empty when there is no preview.
    cells: Vec<CellLevel>,
    /// The previewed total cost — exactly [`Path::total`](gdtf_battle_sim::pathfinder::Path::total),
    /// the §48 bit-identity sum GTW-355 charges. [`Tu::ZERO`](gdtf_battle_sim::ganger::Tu) for an
    /// empty preview.
    cost:  Tu,
}

impl PathPreview {
    /// Build a path preview from a found route's cells + total cost — the `(cells, total)`
    /// of a [`find_path`](gdtf_battle_sim::pathfinder::find_path) [`Path`](gdtf_battle_sim::pathfinder::Path).
    #[must_use]
    pub const fn new(cells: Vec<CellLevel>, cost: Tu) -> Self {
        Self { cells, cost }
    }

    /// The empty (no-route / no-target) preview — what the input populate system writes when
    /// there is no target, or the target is unreachable / only reachable through UNSEEN.
    #[must_use]
    pub const fn cleared() -> Self {
        Self {
            cells: Vec::new(),
            cost:  Tu::new(0),
        }
    }

    /// The previewed route cells, `start..=goal` in step order — read-only.
    #[must_use]
    pub fn cells(&self) -> &[CellLevel] {
        &self.cells
    }

    /// The previewed total cost — the §48 bit-identity sum
    /// ([`Path::total`](gdtf_battle_sim::pathfinder::Path::total)) GTW-355 charges.
    #[must_use]
    pub const fn cost(&self) -> Tu {
        self.cost
    }

    /// Whether the preview is empty (no route drawn).
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }
}
