//! The DEV-ONLY procgen-visualizer model (GTW-434) — the ordered placement sequence the
//! STEP / AUTO controls reveal, plus the domain newtypes the visualizer reasons in.
//!
//! The visualizer is a pure VIEW onto the sim's space-packing placement (the one-way
//! sim→presenter boundary): it RUNS the sim's
//! [`assemble_placement`](gdtf_battle_sim::procgen::assemble_placement) and
//! [`fill_placement`](gdtf_battle_sim::procgen::fill_placement) against the loaded UUID-keyed
//! [`PrefabRegistry`](gdtf_battle_sim::PrefabRegistry) for a theme + grid-size + seed, then
//! projects the resulting [`FilledPlacement`](gdtf_battle_sim::FilledPlacement) into an
//! ORDERED list of [`VizQuad`]s — player, enemy, then every fill prefab in placement order.
//! It never mutates combat/sim state; it only reads the placement.
//!
//! It is inserted as a [`Resource`](bevy::prelude::Resource) `OnEnter(DebugProcgenVisualizer)`
//! (built from the live registry / situation, or EMPTY when none is present) and removed
//! `OnExit`, per the project's state-scoped-resource convention (`bevy-traps.md` #1) — so
//! every system reading it guards with `run_if(resource_exists::<ProcgenViz>)` /
//! `Option<Res<…>>`.
//!
//! The whole module is `#[cfg(debug_assertions)]`-gated by its parent (`procgen_viz`), so it
//! compiles out of release (C4).

use bevy::prelude::*;
use gdtf_battle_sim::{
    FilledPlacement, GangName, GangRegistry, GridSize, PlacedPrefab, PrefabName, PrefabRegistry,
    ProcgenRng, ProcgenTuning, SpawnRole, ThemeUuid, assemble_placement, fill_placement,
    rng::BattleSeed,
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
    /// Which deployment ROLE a placed prefab plays — and therefore which tint its quad draws
    /// with (C3): the player-spawn prefab is GREEN, the enemy-spawn prefab is RED, every other
    /// (fill) prefab is the NEUTRAL light tint.
    ///
    /// A named domain enum (no-bare-types: a quad's tint role is a domain value, not a bare
    /// `Color`/`u8`) so the draw layer never confuses "which role" with "which raw color". It
    /// is projected from the sim's [`SpawnRole`] for the spawn prefabs and is always
    /// [`Neutral`](QuadTint::Neutral) for fill. Declared through `crate::support_item!` so
    /// the headless test can read a quad entity's tint through [`crate::test_support`].
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    enum QuadTint {
        /// The player-spawn prefab quad — tinted GREEN (C3).
        Player,
        /// The enemy-spawn prefab quad — tinted RED (C3).
        Enemy,
        /// Every other (fill) prefab quad — the neutral LIGHT tint (C2/C3).
        Neutral,
    }
}

impl QuadTint {
    /// The light tinted [`Color`] a quad of this role draws with (C2/C3): a light GREEN for
    /// the player spawn, a light RED for the enemy spawn, a neutral light grey otherwise.
    ///
    /// Each is a translucent light fill so the dark board quad reads THROUGH it (the quads
    /// are "light tinted quads over a single dark whole-level quad", C2). Pure UI plumbing
    /// (a resolved `Color`), produced from the domain [`QuadTint`].
    #[must_use]
    pub(in crate::states::running::procgen_viz) const fn color(self) -> Color {
        match self {
            // A light, slightly translucent green / red / grey: the dark board reads through.
            Self::Player => Color::srgba(0.45, 0.85, 0.50, 0.78),
            Self::Enemy => Color::srgba(0.90, 0.40, 0.40, 0.78),
            Self::Neutral => Color::srgba(0.82, 0.82, 0.86, 0.62),
        }
    }

    /// Project a placed-prefab [`SpawnRole`] to its quad tint: the deployment roles map to
    /// their colored tints, generic [`Fill`](SpawnRole::Fill) to the neutral one.
    #[must_use]
    const fn from_role(role: SpawnRole) -> Self {
        match role {
            SpawnRole::Player => Self::Player,
            SpawnRole::Enemy => Self::Enemy,
            SpawnRole::Fill => Self::Neutral,
        }
    }
}

/// A gang ANNOTATION on a deployment quad — the occupying gang's name plus its roster member
/// count, shown as the quad's label so the player/enemy deployment zones read as "who is here"
/// (GTW-498 C4).
///
/// A named newtype over [`String`] (no-bare-types rule 1: a label is a domain value, not a bare
/// `String`). Procgen is TERRAIN-ONLY — choosing a gang does NOT change the generated layout;
/// the annotation only LABELS the already-placed player/enemy deployment quad (display, not
/// geometry). Built through [`for_roster`](GangAnnotation::for_roster) from a resolved roster,
/// read through the derived [`Deref`]; private inner.
#[derive(Deref, Clone, PartialEq, Eq, Debug)]
pub(in crate::states::running::procgen_viz) struct GangAnnotation(String);

impl GangAnnotation {
    /// Build a deployment-quad annotation from a gang's [`GangName`] and its roster's member
    /// count — e.g. `goliaths (4)`. The count is resolved against the [`GangRegistry`]; an
    /// absent / empty roster annotates with `(0)` (never panics — C4 no-gang fallback).
    #[must_use]
    pub(in crate::states::running::procgen_viz) fn for_roster(
        name: &GangName,
        registry: Option<&GangRegistry>,
    ) -> Self {
        let members = registry
            .and_then(|registry| registry.roster(name))
            .map_or(0, |roster| roster.members.len());
        Self(format!("{} ({members})", name.as_str()))
    }
}

/// A quad's WIDTH in board cells (its x span).
///
/// A viz-local newtype over [`u32`] (no-bare-types rule 1: a cell extent is a domain value,
/// not a bare integer). The sim's [`GridWidth`](gdtf_battle_sim::GridWidth) wraps a `u8` coarse
/// grid span — a prefab footprint width comes off a signed [`Footprint`] cast to `u32`, a
/// distinct concept and inner type — so the visualizer mints its own. Private inner + derived
/// [`Deref`]; built through [`new`](QuadCellW::new).
#[derive(Deref, Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::states::running::procgen_viz) struct QuadCellW(u32);

impl QuadCellW {
    /// Build a quad width from its cell span.
    #[must_use]
    pub(in crate::states::running::procgen_viz) const fn new(cells: u32) -> Self {
        Self(cells)
    }
}

/// A quad's HEIGHT in board cells (its y span).
///
/// A viz-local newtype over [`u32`] (no-bare-types rule 3: distinct from [`QuadCellW`] even
/// over the same inner — a width is never a height). Private inner + derived [`Deref`]; built
/// through [`new`](QuadCellH::new).
#[derive(Deref, Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::states::running::procgen_viz) struct QuadCellH(u32);

impl QuadCellH {
    /// Build a quad height from its cell span.
    #[must_use]
    pub(in crate::states::running::procgen_viz) const fn new(cells: u32) -> Self {
        Self(cells)
    }
}

/// The footprint extent (width × height in cells) of one placed prefab quad — the size the
/// quad's label reports and the size its rectangle covers on the board.
///
/// A named newtype (no-bare-types: a quad's cell extent is a domain value, not a bare pair)
/// with [`QuadCellW`] / [`QuadCellH`] leaves. Read through the named
/// [`width`](QuadSize::width) / [`height`](QuadSize::height) accessors.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::states::running::procgen_viz) struct QuadSize {
    /// The quad's width in cells.
    width:  QuadCellW,
    /// The quad's height in cells.
    height: QuadCellH,
}

impl QuadSize {
    /// Build a quad size from its width × height in cells.
    #[must_use]
    pub(in crate::states::running::procgen_viz) const fn new(
        width: QuadCellW,
        height: QuadCellH,
    ) -> Self {
        Self { width, height }
    }

    /// The quad's width in cells.
    #[must_use]
    pub(in crate::states::running::procgen_viz) const fn width(self) -> QuadCellW {
        self.width
    }

    /// The quad's height in cells.
    #[must_use]
    pub(in crate::states::running::procgen_viz) const fn height(self) -> QuadCellH {
        self.height
    }
}

/// The min-corner X cell coordinate of a quad's board rectangle (`>= 0`).
///
/// A viz-local newtype over [`u32`] (no-bare-types rule 1: a board cell coordinate is a domain
/// value). The sim's [`Cell`](gdtf_battle_sim::Cell) wraps a *signed* `IVec2` pair; the packer
/// never produces a negative origin, so the visualizer projects the min-corner into a
/// non-negative `u32` coordinate it owns. Private inner + derived [`Deref`]; built through
/// [`new`](QuadCellX::new).
#[derive(Deref, Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::states::running::procgen_viz) struct QuadCellX(u32);

impl QuadCellX {
    /// Build a min-corner X cell coordinate.
    #[must_use]
    pub(in crate::states::running::procgen_viz) const fn new(cell: u32) -> Self {
        Self(cell)
    }
}

/// The min-corner Y cell coordinate of a quad's board rectangle (`>= 0`).
///
/// A viz-local newtype over [`u32`] (no-bare-types rule 3: distinct from [`QuadCellX`] even
/// over the same inner — an x is never a y). Private inner + derived [`Deref`]; built through
/// [`new`](QuadCellY::new).
#[derive(Deref, Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::states::running::procgen_viz) struct QuadCellY(u32);

impl QuadCellY {
    /// Build a min-corner Y cell coordinate.
    #[must_use]
    pub(in crate::states::running::procgen_viz) const fn new(cell: u32) -> Self {
        Self(cell)
    }
}

/// A cell rectangle on the board — a min-corner cell plus a cell extent — projected from a
/// placed prefab's region so the draw layer can position + size its quad WITHOUT depending on
/// the sim's `RegionRect` (whose inner is private).
///
/// A named struct (no-bare-types: a board placement rectangle is a domain value, not a bare
/// origin+size tuple). All coords are non-negative board cells (the packer never produces a
/// negative origin) — a [`QuadCellX`] / [`QuadCellY`] min-corner plus a [`QuadSize`] extent.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::states::running::procgen_viz) struct QuadRect {
    /// The min-corner cell x of the rectangle (`>= 0`).
    min_x:  QuadCellX,
    /// The min-corner cell y of the rectangle (`>= 0`).
    min_y:  QuadCellY,
    /// The rectangle's width × height in cells.
    extent: QuadSize,
}

impl QuadRect {
    /// Build a quad rectangle from its min-corner cell + cell extent.
    #[must_use]
    pub(in crate::states::running::procgen_viz) const fn new(
        min_x: QuadCellX,
        min_y: QuadCellY,
        extent: QuadSize,
    ) -> Self {
        Self {
            min_x,
            min_y,
            extent,
        }
    }

    /// The min-corner cell x.
    #[must_use]
    pub(in crate::states::running::procgen_viz) const fn min_x(self) -> QuadCellX {
        self.min_x
    }

    /// The min-corner cell y.
    #[must_use]
    pub(in crate::states::running::procgen_viz) const fn min_y(self) -> QuadCellY {
        self.min_y
    }

    /// The rectangle's cell extent.
    #[must_use]
    pub(in crate::states::running::procgen_viz) const fn extent(self) -> QuadSize {
        self.extent
    }
}

crate::support_item! {
    /// One quad in the visualizer's ordered reveal sequence — a placed prefab's name, footprint
    /// size, board rectangle, and the [`QuadTint`] it draws with (C2/C3).
    ///
    /// A named struct (no-bare-types: a visualized placement is a domain value). The STEP /
    /// AUTO controls reveal these in order; the draw layer turns each REVEALED quad into a light
    /// tinted [`Node`](bevy::ui::Node) sized to its rectangle with a name + size label. Declared
    /// through `crate::support_item!` so it is at least as public as
    /// [`ProcgenViz::quads`](ProcgenViz::quads) (which returns `&[VizQuad]`) under `test-support`.
    #[derive(Clone, PartialEq, Eq, Debug)]
    struct VizQuad {
        /// The placed prefab's name (the base label text).
        name: PrefabName,
        /// The quad's footprint size in cells (the label reports `WxH`).
        size: QuadSize,
        /// The quad's rectangle on the board (origin + extent — where + how big to draw it).
        rect: QuadRect,
        /// The tint role this quad draws with (player = green, enemy = red, fill = neutral, C3).
        tint: QuadTint,
        /// The OCCUPYING gang annotation (C4) — present only on the player / enemy deployment
        /// quads when a gang is chosen, [`None`] on fill quads. When present it REPLACES the
        /// prefab name in the quad's label so the deployment zone reads as the gang in it.
        gang: Option<GangAnnotation>,
    }
}

impl VizQuad {
    /// The quad's footprint size in cells.
    #[must_use]
    pub(in crate::states::running::procgen_viz) const fn size(&self) -> QuadSize {
        self.size
    }

    /// The quad's board rectangle (origin + extent).
    #[must_use]
    pub(in crate::states::running::procgen_viz) const fn rect(&self) -> QuadRect {
        self.rect
    }

    /// The quad's tint role.
    #[must_use]
    pub(in crate::states::running::procgen_viz) const fn tint(&self) -> QuadTint {
        self.tint
    }

    /// The quad's DISPLAY label text — the occupying gang annotation (C4) when present
    /// (player / enemy deployment quads), else the placed prefab's name (fill quads). The draw
    /// layer renders this plus the `WxH` size.
    #[must_use]
    pub(in crate::states::running::procgen_viz) fn label_text(&self) -> String {
        match &self.gang {
            Some(gang) => (**gang).clone(),
            None => self.name.as_str().to_owned(),
        }
    }

    /// Project one placed prefab (with an explicit tint role + optional gang annotation, C4)
    /// into a visualizer quad — reading its name, footprint size, and region rectangle from the
    /// sim placement. A `gang` annotation REPLACES the prefab name in the quad's label.
    fn from_placed(placed: &PlacedPrefab, tint: QuadTint, gang: Option<GangAnnotation>) -> Self {
        let prefab = placed.prefab();
        let region = placed.region();
        let footprint = region.footprint();
        let origin = region.origin();
        // The packer never produces a negative origin / extent (board cells are `>= 0`); the
        // `.max(0)` clamp is a fail-safe so a stray negative never wraps the cast.
        let size = QuadSize::new(
            QuadCellW::new(u32::try_from(footprint.width().max(0)).unwrap_or(0)),
            QuadCellH::new(u32::try_from(footprint.height().max(0)).unwrap_or(0)),
        );
        let rect = QuadRect::new(
            QuadCellX::new(u32::try_from(origin.x.max(0)).unwrap_or(0)),
            QuadCellY::new(u32::try_from(origin.y.max(0)).unwrap_or(0)),
            size,
        );
        Self {
            name: prefab.name().clone(),
            size,
            rect,
            tint,
            gang,
        }
    }
}

/// The whole board's cell extent — the size the single DARK whole-level quad covers (C2).
///
/// A named newtype over [`QuadSize`] (no-bare-types: the board extent is a distinct domain
/// value from a prefab footprint, even over the same shape). The draw layer scales every
/// per-prefab quad's board rectangle against this extent so the quads sit correctly within
/// the dark board quad.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::states::running::procgen_viz) struct BoardExtent(QuadSize);

impl BoardExtent {
    /// Build a board extent from its cell width × height.
    #[must_use]
    pub(in crate::states::running::procgen_viz) const fn new(extent: QuadSize) -> Self {
        Self(extent)
    }

    /// The board's cell extent.
    #[must_use]
    pub(in crate::states::running::procgen_viz) const fn size(self) -> QuadSize {
        self.0
    }
}

crate::support_item! {
    /// How many leading quads of the placement sequence are currently REVEALED (`0..=len`).
    ///
    /// A viz-local newtype over [`usize`] (no-bare-types rule 1: a reveal count is a domain
    /// value, not a bare index). STEP advances it by one; AUTO sets it to the sequence length.
    /// Private inner + derived [`Deref`] (so a count reads as a `usize`); built through
    /// [`new`](RevealedCount::new). `Default` (zero revealed) is the starting state. Declared
    /// through `crate::support_item!` so it is at least as public as the
    /// [`ProcgenViz::revealed`] accessor that returns it (which widens under `test-support`);
    /// the external test reads it through `Deref` rather than naming it.
    #[derive(Deref, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Debug)]
    struct RevealedCount(usize);
}

impl RevealedCount {
    /// Build a reveal count from its number of revealed quads.
    #[must_use]
    pub(in crate::states::running::procgen_viz) const fn new(revealed: usize) -> Self {
        Self(revealed)
    }

    /// The number of revealed quads (the const-context read — [`Deref`] is not const).
    #[must_use]
    pub(in crate::states::running::procgen_viz) const fn get(self) -> usize {
        self.0
    }
}

crate::support_item! {
    /// A quad's REVEAL position in the placement sequence — its index (`0` = player, `1` =
    /// enemy, then fill in placement order) the STEP / AUTO reveal count is measured against.
    ///
    /// A viz-local newtype over [`usize`] (no-bare-types rule 1: a reveal position is a domain
    /// value, not a bare index). Distinct from [`RevealedCount`] (rule 3: a position is not a
    /// count) — a quad is shown once the count EXCEEDS its index
    /// ([`is_revealed_within`](RevealIndex::is_revealed_within)). Private inner + derived
    /// [`Deref`]; built through [`new`](RevealIndex::new). Declared through
    /// `crate::support_item!` so it is at least as public as the [`PrefabQuad::index`]
    /// accessor that returns it (which widens under `test-support`); the external test reads it
    /// through `Deref` rather than naming it.
    ///
    /// [`PrefabQuad::index`]: super::components::PrefabQuad::index
    #[derive(Deref, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
    struct RevealIndex(usize);
}

impl RevealIndex {
    /// Build a reveal index from its position in the placement sequence.
    #[must_use]
    pub(in crate::states::running::procgen_viz) const fn new(index: usize) -> Self {
        Self(index)
    }

    /// Whether a quad at this index is REVEALED for the given reveal count — true once the
    /// count exceeds the index (`index < revealed`), the draw layer's show/hide predicate (C2).
    #[must_use]
    pub(in crate::states::running::procgen_viz) const fn is_revealed_within(
        self,
        revealed: RevealedCount,
    ) -> bool {
        self.0 < revealed.get()
    }
}

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
