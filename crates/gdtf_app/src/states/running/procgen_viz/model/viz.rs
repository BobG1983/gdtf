//! The [`ProcgenViz`] resource: the sim build-pipeline projection, the STEP / AUTO
//! reveal mutation, and the test-facing reads. Split out of the monolithic `model.rs`
//! (GTW-583); the model rationale lives on the parent `model` module.

use bevy::prelude::*;
use gdtf_battle_sim::{
    ganger::{GangName, GangRegistry},
    level::{GridSize, PrefabRegistry, SpawnRole, ThemeUuid},
    procgen::{FilledPlacement, ProcgenTuning, assemble_placement, fill_placement},
    rng::{BattleSeed, ProcgenRng},
};

use super::{
    quad::{GangAnnotation, QuadTint, VizQuad},
    reveal::RevealedCount,
    units::{BoardExtent, QuadCellH, QuadCellW, QuadSize},
};

/// The fixed default root seed the visualizer assembles a level from when no
/// [`BattleSeed`] override is injected — a deterministic demo level so the rendered
/// placement is stable across runs.
///
/// A test harness (or a future seed-pick affordance) can pre-insert a `Res<BattleSeed>` to
/// drive a different level; absent that, this constant seeds the demo. Framework-plumbing
/// magnitude (a replay handle), not a domain value the visualizer reasons over.
const DEFAULT_VIZ_SEED: u64 = 0x6764_7466_7669_7A30;

crate::support_item! {
    /// The DEV-ONLY procgen-visualizer model (GTW-434) — the board extent, the ordered
    /// placement sequence, and how many of those quads are currently REVEALED.
    ///
    /// Inserted `OnEnter(DebugProcgenVisualizer)` (built from the live sim placement, or
    /// EMPTY when no registry / situation is present), read by the draw layer, mutated by the
    /// STEP / AUTO controls, and removed `OnExit` (the state-scoped-resource convention). STEP
    /// advances [`revealed`](ProcgenViz::revealed) by one (clamped at the sequence length);
    /// AUTO reveals the whole sequence at once.
    ///
    /// Declared through `crate::support_item!` so it is `pub` under the `test-support`
    /// feature — the headless integration test names it through
    /// [`crate::test_support`](crate::test_support) — and `pub(crate)` in the binary build
    /// (keeping it `unreachable_pub`-clean). Its test-facing methods use the same per-method
    /// flip.
    #[derive(Resource, Clone, PartialEq, Eq, Debug)]
    struct ProcgenViz {
        /// The board's cell extent — the size the dark whole-level quad covers (C2).
        board:    BoardExtent,
        /// The ordered placement sequence (player, enemy, then fill in placement order) the
        /// STEP / AUTO controls reveal.
        quads:    Vec<VizQuad>,
        /// How many leading [`quads`](ProcgenViz::quads) are revealed (`0..=quads.len()`).
        revealed: RevealedCount,
    }
}

impl ProcgenViz {
    /// The board's cell extent (C2).
    #[must_use]
    pub(in crate::states::running::procgen_viz) const fn board(&self) -> BoardExtent {
        self.board
    }

    /// The board's cell extent as a `(width, height)` pair — PURELY test-facing (the C7 size
    /// assertion checks the regenerated board reflects the chosen grid-size width/height), so it
    /// is `#[cfg(feature = "test-support")]`-gated: it exists only in the harness build, never in
    /// the binary (where it would be dead code).
    #[cfg(feature = "test-support")]
    #[must_use]
    pub fn board_dimensions(&self) -> (u32, u32) {
        let extent = self.board.size();
        (*extent.width(), *extent.height())
    }

    /// The DISPLAY label of the quad at `index` (`0` = player, `1` = enemy, then fill), or
    /// `None` if out of range — PURELY test-facing (the C4 assertion checks the player / enemy
    /// quad carries its chosen-gang annotation after Generate). `#[cfg(feature =
    /// "test-support")]`-gated (harness-only, never in the binary).
    #[cfg(feature = "test-support")]
    #[must_use]
    pub fn quad_label_at(&self, index: usize) -> Option<String> {
        self.quads.get(index).map(VizQuad::label_text)
    }

    /// The quad at `index`'s board RECTANGLE as `(min_x, min_y, width, height)` cells, or `None`
    /// if out of range — PURELY test-facing (the C3 determinism assertion fingerprints the
    /// ordered placement RECTANGLES, which change with the seed, without naming a tunable
    /// magnitude). `#[cfg(feature = "test-support")]`-gated (harness-only, never in the binary).
    #[cfg(feature = "test-support")]
    #[must_use]
    pub fn quad_rect_at(&self, index: usize) -> Option<(u32, u32, u32, u32)> {
        self.quads.get(index).map(|quad| {
            let rect = quad.rect();
            (
                *rect.min_x(),
                *rect.min_y(),
                *rect.extent().width(),
                *rect.extent().height(),
            )
        })
    }

    crate::support_item! {
        /// The ordered placement sequence (player, enemy, then fill). Used in-crate by the
        /// screen spawn AND test-facing (the headless assertions read `quads().len()` as the
        /// total and inspect the projected quads through `test_support`).
        #[must_use]
        fn quads(&self) -> &[VizQuad] {
            &self.quads
        }
    }

    crate::support_item! {
        /// How many quads are currently revealed (`0..=quads.len()`) — the value STEP
        /// advances by one and AUTO sets to the sequence length. Used in-crate by the draw
        /// sync AND test-facing (the headless C1 assertion reads it through `test_support`,
        /// deref-ing the `RevealedCount` to a `usize`).
        #[must_use]
        const fn revealed(&self) -> RevealedCount {
            self.revealed
        }
    }

    /// STEP — reveal ONE more quad, clamped at the sequence length (C1). Returns whether a
    /// new quad was revealed (false once every quad is already shown).
    pub(in crate::states::running::procgen_viz) const fn step(&mut self) -> bool {
        if self.revealed.get() < self.quads.len() {
            self.revealed = RevealedCount::new(self.revealed.get() + 1);
            true
        } else {
            false
        }
    }

    /// AUTO — reveal EVERY quad at once, running the placement sequence to completion (C1).
    pub(in crate::states::running::procgen_viz) const fn reveal_all(&mut self) {
        self.revealed = RevealedCount::new(self.quads.len());
    }

    /// Build the visualizer model by running the sim space-packing pipeline against the live
    /// registry + theme + grid-size + seed, projecting the result into the ordered quad
    /// sequence (player, enemy, then fill in placement order) — initially with ZERO revealed.
    ///
    /// This is the no-gang-annotation form (the GTW-434 entry shape preserved for the initial
    /// `OnEnter` build, C6): it delegates to [`build_with_gangs`](ProcgenViz::build_with_gangs)
    /// with no chosen gangs, so the player/enemy quads label by prefab name.
    ///
    /// On a procgen failure (e.g. an EMPTY registry — the no-content harness) OR no registry
    /// at all, it builds an EMPTY model (the board extent only, no quads): the visualizer is
    /// still reachable and tears down cleanly, it just has nothing to reveal. This mirrors the
    /// GTW-433 live-trigger fallback (fail-open, never panic).
    ///
    /// GTW-533: takes the LIVE `tuning` (the hot-reloaded [`ProcgenTuning`] resource) so the
    /// visualizer's fill re-tunes on a `core_tuning/procgen.tuning.ron` edit; `None` ⇒ the
    /// const default.
    #[must_use]
    pub(in crate::states::running::procgen_viz) fn build(
        registry: Option<&PrefabRegistry>,
        theme: ThemeUuid,
        grid_size: GridSize,
        seed: BattleSeed,
        tuning: Option<&ProcgenTuning>,
    ) -> Self {
        Self::build_with_gangs(registry, theme, grid_size, seed, None, None, None, tuning)
    }

    /// Build the visualizer model from the sim space-packing pipeline AND apply chosen-gang
    /// deployment-quad annotations (GTW-498 C4/C5).
    ///
    /// Identical to [`build`](ProcgenViz::build) for the geometry — procgen is TERRAIN-ONLY, so
    /// the chosen `player_gang` / `enemy_gang` do NOT change the generated layout — but the
    /// player quad (index 0) and the enemy quad (index 1) are LABELLED with the occupying gang's
    /// name + member count (resolved against `gangs`). `None` gangs label by prefab name (the
    /// initial / no-gang state). An absent / empty registry yields a `(0)` annotation; nothing
    /// here panics (C4 no-gang fallback). Same seed → identical placement; a different seed →
    /// a different placement (C3/C5 determinism), because the only RNG input is `seed`.
    ///
    /// GTW-533: takes the LIVE `tuning` (the hot-reloaded [`ProcgenTuning`] resource) so an edit
    /// to `core_tuning/procgen.tuning.ron` re-tunes the visualizer's fill on the next Generate;
    /// `None` ⇒ the const default.
    #[must_use]
    #[allow(
        clippy::too_many_arguments,
        reason = "GTW-533 threads the live ProcgenTuning as the final arg alongside the GTW-498 \
                  gang-annotation params; each is a distinct, independent procgen input — \
                  bundling them would obscure more than it saves"
    )]
    pub(in crate::states::running::procgen_viz) fn build_with_gangs(
        registry: Option<&PrefabRegistry>,
        theme: ThemeUuid,
        grid_size: GridSize,
        seed: BattleSeed,
        gangs: Option<&GangRegistry>,
        player_gang: Option<&GangName>,
        enemy_gang: Option<&GangName>,
        tuning: Option<&ProcgenTuning>,
    ) -> Self {
        let board = board_extent(grid_size);
        let Some(filled) = assemble_filled(registry, theme, grid_size, seed, tuning) else {
            // No registry, or procgen failed closed (empty registry, no fitting prefab) —
            // an empty reveal sequence over the board extent. Reachable + tears down cleanly.
            return Self {
                board,
                quads: Vec::new(),
                revealed: RevealedCount::default(),
            };
        };

        let placement = filled.placement();
        let mut quads = Vec::with_capacity(2 + filled.fill().len());
        // Fixed reveal order (C1/C2): player, enemy, then fill in placement order. The player /
        // enemy quads carry their chosen-gang annotation (C4) when a gang is selected.
        let player_label = player_gang.map(|name| GangAnnotation::for_roster(name, gangs));
        let enemy_label = enemy_gang.map(|name| GangAnnotation::for_roster(name, gangs));
        quads.push(VizQuad::from_placed(
            placement.player(),
            QuadTint::Player,
            player_label,
        ));
        quads.push(VizQuad::from_placed(
            placement.enemy(),
            QuadTint::Enemy,
            enemy_label,
        ));
        for placed in filled.fill() {
            quads.push(VizQuad::from_placed(
                placed,
                QuadTint::from_role(SpawnRole::Fill),
                None,
            ));
        }

        Self {
            board,
            quads,
            revealed: RevealedCount::default(),
        }
    }
}

/// The board's cell extent as a [`BoardExtent`] — its width × height ground-plane span
/// (the storey count is dropped; the quads are drawn on the ground plane, C2).
fn board_extent(grid_size: GridSize) -> BoardExtent {
    BoardExtent::new(QuadSize::new(
        QuadCellW::new(u32::from(*grid_size.width())),
        QuadCellH::new(u32::from(*grid_size.height())),
    ))
}

/// Run the sim space-packing pipeline (assemble + fill) against the registry + theme +
/// grid-size + seed, returning the [`FilledPlacement`] — or `None` on a missing registry /
/// any procgen failure (fail-open, never panic; the visualizer then shows an empty board).
///
/// GTW-533: uses the LIVE `tuning` (the hot-reloaded [`ProcgenTuning`] resource, threaded from
/// the caller) when present, else the const RULED [`ProcgenTuning::default`] — so an edit to
/// `core_tuning/procgen.tuning.ron` re-tunes the visualizer's fill on the next Generate, matching
/// the live battle path.
fn assemble_filled(
    registry: Option<&PrefabRegistry>,
    theme: ThemeUuid,
    grid_size: GridSize,
    seed: BattleSeed,
    tuning: Option<&ProcgenTuning>,
) -> Option<FilledPlacement> {
    let registry = registry?;
    let mut rng = ProcgenRng::from_root(seed);
    let tuning = tuning.copied().unwrap_or_default();
    let placement = assemble_placement(registry, theme, grid_size, &mut rng).ok()?;
    fill_placement(placement, registry, theme, grid_size, &tuning, &mut rng).ok()
}

/// The default root seed the visualizer assembles its demo level from — used when no
/// [`BattleSeed`] override resource is present.
#[must_use]
pub(in crate::states::running::procgen_viz) const fn default_viz_seed() -> BattleSeed {
    BattleSeed::new(DEFAULT_VIZ_SEED)
}
