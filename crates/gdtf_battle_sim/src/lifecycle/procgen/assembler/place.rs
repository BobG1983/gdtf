//! The two assemble PLACEMENTS — [`place_player`] then [`place_enemy`] — plus the placement
//! value types ([`PlacedPrefab`] / [`Placement`]) and the ruled-defaults composition
//! [`assemble_placement`] / [`assemble_placement_with`] (GTW-424; GTW-732 split into two
//! single-placement steps the unified step primitive drives).

use super::{
    super::{
        anchor::Anchor,
        error::PackingError,
        geometry::{Footprint, Margin, MinPlayerSide, RegionRect},
        packer::{MaxRectsPacker, SplitMode},
    },
    pick::{pick_fitting_prefab, pick_player_prefab},
};
use crate::{
    level::{GridSize, Prefab, PrefabRegistry, SpawnRole, ThemeUuid},
    rng::ProcgenRng,
};

/// One prefab the assembler has PLACED — the chosen [`Prefab`], the [`Anchor`] it sits at,
/// and its placed [`RegionRect`] on the board (GTW-424; GTW-492 switched it onto the
/// UUID-keyed [`Prefab`]).
///
/// A named struct (no-bare-types: a placement is a domain value, not a bare tuple). The
/// GTW-427 fill pass reads the region to stamp interior fill; the GTW-431 emit step reads
/// the prefab + origin to pour the fragment's UUID-keyed placements into the
/// [`Situation`](crate::situation::Situation). Holds the prefab BY VALUE (it is `Clone`),
/// so a placement survives the registry borrow ending.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlacedPrefab {
    /// The chosen prefab (player-spawn or enemy-spawn fragment).
    prefab: Prefab,
    /// The anchor it was placed flush against.
    anchor: Anchor,
    /// Its placed footprint on the board (min-corner origin + extent).
    region: RegionRect,
}

impl PlacedPrefab {
    /// Build a placed-prefab record.
    #[must_use]
    pub const fn new(prefab: Prefab, anchor: Anchor, region: RegionRect) -> Self {
        Self {
            prefab,
            anchor,
            region,
        }
    }

    /// The chosen prefab.
    #[must_use]
    pub const fn prefab(&self) -> &Prefab {
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
    /// Build a placement from its player + enemy halves — the composition point the unified
    /// step primitive uses after [`place_player`] and [`place_enemy`] have each run.
    #[must_use]
    pub(in crate::lifecycle::procgen) const fn new(
        player: PlacedPrefab,
        enemy: PlacedPrefab,
    ) -> Self {
        Self { player, enemy }
    }

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

/// Place the PLAYER-spawn prefab — the FIRST assemble step (GTW-424 C1).
///
/// Builds the packer over the whole board, chooses the player [`Anchor`] (the ONLY RNG draw
/// in the assemble stage), picks the largest fitting `>= MinPlayerSide` player-spawn prefab,
/// commits it (with a fail-closed re-guard), and returns the placed player AND the packer with
/// the player region already carved — so the enemy step ([`place_enemy`]) picks against the
/// SAME free space the one-shot [`assemble_placement_with`] left after the player commit
/// (GTW-732: carrying the packer between the two steps reproduces the one-shot free space
/// exactly).
///
/// # Errors
///
/// - [`PackingError::NoPrefabForRole`] if the registry has no player-spawn prefab for
///   `(theme, size, role)`.
/// - [`PackingError::PlayerFootprintTooSmall`] if every player prefab is below
///   [`MinPlayerSide`].
/// - [`PackingError::FootprintDoesNotFit`] if the chosen player prefab (plus seam) does not
///   fit at the player anchor.
pub(in crate::lifecycle::procgen) fn place_player(
    registry: &PrefabRegistry,
    theme: ThemeUuid,
    grid_size: GridSize,
    rng: &mut ProcgenRng,
    split: SplitMode,
    min_player_side: MinPlayerSide,
) -> Result<(PlacedPrefab, MaxRectsPacker), PackingError> {
    let board = RegionRect::board(grid_size);
    let mut packer = MaxRectsPacker::new(board, split, Margin::DEFAULT);

    // C1: choose the player anchor — the ONLY RNG draw in the assemble stage.
    let player_anchor = Anchor::choose(rng);

    // C1: pick the player-spawn prefab — the largest registered fragment that clears the OQ-5
    // minimum AND fits (with its seam) flush at the player anchor.
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
    if !*packer.place(player_region) {
        return Err(PackingError::FootprintDoesNotFit {
            anchor:    player_anchor,
            footprint: player_footprint,
            region:    board,
        });
    }

    Ok((
        PlacedPrefab::new(player_prefab, player_anchor, player_region),
        packer,
    ))
}

/// Place the ENEMY-spawn prefab at the strict-opposite anchor — the SECOND assemble step
/// (GTW-424 C2).
///
/// The enemy anchor is the zero-draw strict [`opposite`](Anchor::opposite) of the player's
/// anchor (OQ-2 — fairness is structural). Picks the largest enemy fragment that FITS the
/// live free space `packer` holds after the player commit, commits it, and re-guards
/// fail-closed. Connectivity is by-construction via the 1-cell seam every placement reserves
/// (GTW-497 removed the old flood).
///
/// # Errors
///
/// - [`PackingError::NoPrefabForRole`] if the registry has no enemy-spawn prefab for
///   `(theme, size, role)`.
/// - [`PackingError::FootprintDoesNotFit`] if every enemy prefab is too large for the
///   remaining space at the opposite anchor.
pub(in crate::lifecycle::procgen) fn place_enemy(
    registry: &PrefabRegistry,
    theme: ThemeUuid,
    player: &PlacedPrefab,
    packer: &mut MaxRectsPacker,
    board: RegionRect,
) -> Result<PlacedPrefab, PackingError> {
    let enemy_anchor = player.anchor().opposite();
    let enemy_prefab = pick_fitting_prefab(
        registry,
        theme,
        SpawnRole::Enemy,
        packer,
        board,
        enemy_anchor,
    )?;
    let enemy_footprint = Footprint::of(enemy_prefab.spec().size);
    let enemy_region = board.place_at_anchor(enemy_anchor, enemy_footprint);
    if !*packer.place(enemy_region) {
        return Err(PackingError::FootprintDoesNotFit {
            anchor:    enemy_anchor,
            footprint: enemy_footprint,
            region:    board,
        });
    }
    Ok(PlacedPrefab::new(enemy_prefab, enemy_anchor, enemy_region))
}

/// Assemble the GTW-424 first-half placement with the RULED defaults — choose a player anchor,
/// place a `>= 10x10` player-spawn prefab there, place an enemy-spawn prefab at the strict
/// opposite, and reserve the 1-cell seam around both. Connectivity is by-construction via that
/// seam lattice — no flood, no assertion (GTW-497).
///
/// # Errors
///
/// Same as `place_player` / `place_enemy`.
pub fn assemble_placement(
    registry: &PrefabRegistry,
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
/// A thin composition of the two single-placement steps (`place_player` then
/// `place_enemy`) — the SAME steps the unified step primitive drives one at a time
/// (GTW-732), so a one-shot assemble and a two-step assemble draw identically. Exposed so the
/// A/B comparison (`MaxRects` vs Guillotine) and the unit tests can drive the packer's split
/// strategy without changing the shipped default.
///
/// # Errors
///
/// Same as [`assemble_placement`].
pub fn assemble_placement_with(
    registry: &PrefabRegistry,
    theme: ThemeUuid,
    grid_size: GridSize,
    rng: &mut ProcgenRng,
    split: SplitMode,
    min_player_side: MinPlayerSide,
) -> Result<Placement, PackingError> {
    let (player, mut packer) =
        place_player(registry, theme, grid_size, rng, split, min_player_side)?;
    let board = RegionRect::board(grid_size);
    let enemy = place_enemy(registry, theme, &player, &mut packer, board)?;
    Ok(Placement::new(player, enemy))
}
