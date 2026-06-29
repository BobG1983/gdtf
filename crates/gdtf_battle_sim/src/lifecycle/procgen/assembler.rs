//! The **first-half assembler** — anchor selection + strict-opposite enemy placement
//! (GTW-424).
//!
//! This is the GTW-424 slice of the staged assembler (424 placement -> 427 fill -> 431
//! emit/trigger): it picks a player anchor from [`ProcgenRng`](crate::rng::ProcgenRng)
//! (C1), places a `>= 10x10` player-spawn prefab there (C1/OQ-5), places an enemy-spawn
//! prefab at the STRICT geometric opposite (C2/OQ-2), and reserves the 1-cell seam around
//! both (OQ-3). Connectivity is by-construction via that seam lattice — GTW-497 removed the
//! old OQ-4 fail-closed connectivity flood / rejection (there is nothing to assert or
//! repair: the seam guarantees every open cell is reachable). It returns the two
//! [`PlacedPrefab`]s; the GTW-427 fill pass and the GTW-431 emit-to-`Situation` step build
//! on top. NOTHING here wires `BattleScapeState` (a later ticket).
//!
//! GTW-492 (child T07b of the GTW-476 data-model refactor): the assembler reads the
//! UUID-keyed [`PrefabRegistry2`] of [`Prefab2`] fragments, keyed by a stable
//! [`ThemeUuid`] (GTW-485 / GTW-488).

use super::{
    anchor::Anchor,
    error::PackingError,
    geometry::{Footprint, MinPlayerSide, RegionRect},
    packer::{MaxRectsPacker, SplitMode},
};
use crate::{
    level::{GridSize, Prefab2, PrefabKey2, PrefabRegistry2, SpawnRole, ThemeUuid},
    rng::ProcgenRng,
};

/// One prefab the assembler has PLACED — the chosen [`Prefab2`], the [`Anchor`] it sits at,
/// and its placed [`RegionRect`] on the board (GTW-424; GTW-492 switched it onto the
/// UUID-keyed [`Prefab2`]).
///
/// A named struct (no-bare-types: a placement is a domain value, not a bare tuple). The
/// GTW-427 fill pass reads the region to stamp interior fill; the GTW-431 emit step reads
/// the prefab + origin to pour the fragment's UUID-keyed placements into the
/// [`Situation`](crate::situation::Situation). Holds the prefab BY VALUE (it is `Clone`),
/// so a placement survives the registry borrow ending.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlacedPrefab {
    /// The chosen prefab (player-spawn or enemy-spawn fragment).
    prefab: Prefab2,
    /// The anchor it was placed flush against.
    anchor: Anchor,
    /// Its placed footprint on the board (min-corner origin + extent).
    region: RegionRect,
}

impl PlacedPrefab {
    /// Build a placed-prefab record.
    #[must_use]
    pub const fn new(prefab: Prefab2, anchor: Anchor, region: RegionRect) -> Self {
        Self {
            prefab,
            anchor,
            region,
        }
    }

    /// The chosen prefab.
    #[must_use]
    pub const fn prefab(&self) -> &Prefab2 {
        &self.prefab
    }

    /// The anchor it sits at.
    #[must_use]
    pub const fn anchor(&self) -> Anchor {
        self.anchor
    }

    /// Its placed region on the board.
    #[must_use]
    pub const fn region(&self) -> RegionRect {
        self.region
    }
}

/// The first-half placement RESULT — the player-spawn and enemy-spawn prefabs and where
/// they landed (GTW-424).
///
/// A named struct (no-bare-types: the placement outcome is a domain value). The C1/C2
/// deliverable: a `>= 10x10` player-spawn at a deterministically chosen anchor and an
/// enemy-spawn at its strict opposite, both seam-separated and connectivity-asserted. The
/// GTW-427 fill pass + GTW-431 emit step consume this.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Placement {
    /// The placed player-spawn prefab (at the RNG-chosen anchor).
    player: PlacedPrefab,
    /// The placed enemy-spawn prefab (at the strict-opposite anchor).
    enemy:  PlacedPrefab,
}

impl Placement {
    /// The placed player-spawn prefab.
    #[must_use]
    pub const fn player(&self) -> &PlacedPrefab {
        &self.player
    }

    /// The placed enemy-spawn prefab.
    #[must_use]
    pub const fn enemy(&self) -> &PlacedPrefab {
        &self.enemy
    }
}

/// Assemble the GTW-424 first-half placement: choose a player anchor, place a `>= 10x10`
/// player-spawn prefab there, place an enemy-spawn prefab at the strict opposite, and
/// reserve the 1-cell seam around both. Connectivity is by-construction via that seam
/// lattice — no flood, no assertion (GTW-497).
///
/// The ONLY RNG draw is [`Anchor::choose`] (the player anchor, C1); the enemy anchor is
/// the zero-draw strict [`opposite`](Anchor::opposite) (OQ-2). Prefab CHOICE within a
/// `(theme, size, role)` bucket is the FIRST candidate (deterministic order — no RNG draw
/// in GTW-424; richer prefab selection is GTW-427's concern), so the same seed always
/// produces the same placement (the determinism contract).
///
/// # Errors
///
/// - [`PackingError::NoPrefabForRole`] if the registry has no player- or enemy-spawn
///   prefab for `(theme, size, role)`.
/// - [`PackingError::PlayerFootprintTooSmall`] if the chosen player prefab's footprint is
///   below [`MinPlayerSide`] (OQ-5 runtime backstop; load-time rejection is preferred).
/// - [`PackingError::FootprintDoesNotFit`] if a chosen prefab's footprint (plus seam) does
///   not fit its region (C2 — the player anchor region, or the strict-opposite enemy
///   region).
pub fn assemble_placement(
    registry: &PrefabRegistry2,
    theme: ThemeUuid,
    grid_size: GridSize,
    rng: &mut ProcgenRng,
) -> Result<Placement, PackingError> {
    assemble_placement_with(
        registry,
        theme,
        grid_size,
        rng,
        SplitMode::default(),
        MinPlayerSide::DEFAULT,
    )
}

/// The full assembler with explicit [`SplitMode`] (the OQ-7 A/B flag) and minimum
/// player-side floor — [`assemble_placement`] is the RULED-defaults wrapper.
///
/// Exposed so the A/B comparison (`MaxRects` vs Guillotine) and the unit tests can drive
/// the packer's split strategy without changing the shipped default.
///
/// # Errors
///
/// Same as [`assemble_placement`].
pub fn assemble_placement_with(
    registry: &PrefabRegistry2,
    theme: ThemeUuid,
    grid_size: GridSize,
    rng: &mut ProcgenRng,
    split: SplitMode,
    min_player_side: MinPlayerSide,
) -> Result<Placement, PackingError> {
    let board = RegionRect::board(grid_size);
    let mut packer = MaxRectsPacker::new(board, split, super::geometry::Margin::DEFAULT);

    // C1: choose the player anchor — the ONLY RNG draw.
    let player_anchor = Anchor::choose(rng);
    // OQ-2: the enemy anchor is the zero-draw strict opposite.
    let enemy_anchor = player_anchor.opposite();

    // C1: pick the player-spawn prefab — the largest registered fragment that clears the
    // OQ-5 minimum AND fits (with its seam) flush at the player anchor. OQ-5's cap on the
    // player footprint is realised here: a fragment too large for the board (so the
    // opposite enemy region could not also fit) is skipped, never chosen.
    let player_prefab = pick_player_prefab(
        registry,
        theme,
        &packer,
        board,
        player_anchor,
        min_player_side,
    )?;
    let player_footprint = Footprint::of(player_prefab.spec().size);
    let player_region = board.place_at_anchor(player_anchor, player_footprint);
    // The fit was already checked in selection, but commit it (and re-guard fail-closed).
    if !packer.place(player_region) {
        return Err(PackingError::FootprintDoesNotFit {
            anchor:    player_anchor,
            footprint: player_footprint,
            region:    board,
        });
    }

    // C2: pick + place the enemy-spawn prefab at the strict-opposite anchor — the largest
    // registered enemy fragment that FITS the remaining space at the opposite anchor.
    let enemy_prefab = pick_fitting_prefab(
        registry,
        theme,
        SpawnRole::Enemy,
        &packer,
        board,
        enemy_anchor,
    )?;
    let enemy_footprint = Footprint::of(enemy_prefab.spec().size);
    let enemy_region = board.place_at_anchor(enemy_anchor, enemy_footprint);
    if !packer.place(enemy_region) {
        return Err(PackingError::FootprintDoesNotFit {
            anchor:    enemy_anchor,
            footprint: enemy_footprint,
            region:    board,
        });
    }

    // OQ-4: connectivity is BY CONSTRUCTION — the 1-cell `default_floor` seam reserved
    // around both regions (and around every later fill placement) leaves a walkable corridor
    // lattice that joins every open cell, so the two placed regions are always reachable. No
    // flood / assertion is needed (GTW-497 removed the old fail-closed connectivity check).
    Ok(Placement {
        player: PlacedPrefab::new(player_prefab, player_anchor, player_region),
        enemy:  PlacedPrefab::new(enemy_prefab, enemy_anchor, enemy_region),
    })
}

/// Every `(theme, role)` prefab the registry holds, in a DETERMINISTIC order (sorted by
/// footprint area DESC then by name, so the iteration order does not depend on the
/// registry's unordered `HashMap`) — the candidate list both pickers scan.
///
/// Prefabs are level FRAGMENTS smaller than the board: their registry `size` key is the
/// fragment footprint, NOT the board. So the assembler enumerates ACROSS sizes for a
/// `(theme, role)` and picks a FITTING one — it never assumes a fragment fills the board.
///
/// GTW-492: keyed on the stable [`ThemeUuid`] (the [`PrefabKey2::theme`] field) and the
/// [`PrefabKey2::role`] field. An absent `theme` (no key matches) yields an EMPTY list — the
/// pickers turn that into a fail-closed [`PackingError::NoPrefabForRole`].
fn candidates(registry: &PrefabRegistry2, theme: ThemeUuid, role: SpawnRole) -> Vec<Prefab2> {
    let mut out: Vec<Prefab2> = registry
        .keys()
        .filter(|k| k.theme == theme && k.role == role)
        .flat_map(|k: &PrefabKey2| registry.prefabs_for(k).iter().cloned())
        .collect();
    // Deterministic order: largest fragment first (prefer the densest deployment zone that
    // fits), ties broken by name so the order is total and seed-independent.
    out.sort_by(|a, b| {
        let area = |p: &Prefab2| {
            let f = Footprint::of(p.spec().size);
            i64::from(f.width()) * i64::from(f.height())
        };
        area(b)
            .cmp(&area(a))
            .then_with(|| (**a.name()).cmp(&**b.name()))
    });
    out
}

/// Pick the PLAYER-spawn prefab: the first candidate (largest-first, deterministic) that
/// clears the OQ-5 minimum side AND fits — with its seam — flush at the player anchor.
///
/// OQ-5's cap is realised here: a fragment too large to leave room for the opposite enemy
/// region is simply not chosen (it fails the packer fit). Fails closed with
/// [`PackingError::NoPrefabForRole`] if no player prefab exists at all, or
/// [`PackingError::PlayerFootprintTooSmall`] if EVERY candidate is below the minimum side,
/// or [`PackingError::FootprintDoesNotFit`] if every (large-enough) candidate is too large
/// to fit the board with its seam.
fn pick_player_prefab(
    registry: &PrefabRegistry2,
    theme: ThemeUuid,
    packer: &MaxRectsPacker,
    board: RegionRect,
    anchor: Anchor,
    min_player_side: MinPlayerSide,
) -> Result<Prefab2, PackingError> {
    let candidates = candidates(registry, theme, SpawnRole::Player);
    if candidates.is_empty() {
        return Err(PackingError::NoPrefabForRole {
            theme,
            role: SpawnRole::Player,
        });
    }

    // Track the best diagnostic error: a too-small one only matters if NO candidate
    // clears the minimum; a does-not-fit one if none of the big-enough ones fit.
    let mut last_too_small: Option<PackingError> = None;
    let mut last_no_fit: Option<PackingError> = None;
    for prefab in candidates {
        let footprint = Footprint::of(prefab.spec().size);
        if footprint.min_side() < min_player_side.cells() {
            last_too_small = Some(PackingError::PlayerFootprintTooSmall {
                footprint,
                min_side: min_player_side,
            });
            continue;
        }
        let region = board.place_at_anchor(anchor, footprint);
        if packer.fits(region) {
            return Ok(prefab);
        }
        last_no_fit = Some(PackingError::FootprintDoesNotFit {
            anchor,
            footprint,
            region: board,
        });
    }
    // Prefer the "does not fit" diagnostic (a big-enough candidate existed but did not
    // fit) over "too small" (no candidate even reached the minimum).
    Err(last_no_fit
        .or(last_too_small)
        .unwrap_or(PackingError::NoPrefabForRole {
            theme,
            role: SpawnRole::Player,
        }))
}

/// Pick a prefab of `role` that FITS — with its seam — flush at `anchor` against the
/// current free space (the enemy-spawn picker, C2). Largest fitting fragment first.
///
/// Fails closed with [`PackingError::NoPrefabForRole`] if none exists, or
/// [`PackingError::FootprintDoesNotFit`] if every candidate is too large for the remaining
/// space at the opposite anchor.
fn pick_fitting_prefab(
    registry: &PrefabRegistry2,
    theme: ThemeUuid,
    role: SpawnRole,
    packer: &MaxRectsPacker,
    board: RegionRect,
    anchor: Anchor,
) -> Result<Prefab2, PackingError> {
    let candidates = candidates(registry, theme, role);
    if candidates.is_empty() {
        return Err(PackingError::NoPrefabForRole { theme, role });
    }
    let mut last_no_fit: Option<PackingError> = None;
    for prefab in candidates {
        let footprint = Footprint::of(prefab.spec().size);
        let region = board.place_at_anchor(anchor, footprint);
        if packer.fits(region) {
            return Ok(prefab);
        }
        last_no_fit = Some(PackingError::FootprintDoesNotFit {
            anchor,
            footprint,
            region: board,
        });
    }
    Err(last_no_fit.unwrap_or(PackingError::NoPrefabForRole { theme, role }))
}
