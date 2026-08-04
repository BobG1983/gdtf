//! Place player and enemy spawn prefabs on the board.

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

/// A prefab placed at an anchor region.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlacedPrefab {
    prefab: Prefab,
    anchor: Anchor,
    region: RegionRect,
}

impl PlacedPrefab {
    /// Build a placed prefab.
    #[must_use]
    pub const fn new(prefab: Prefab, anchor: Anchor, region: RegionRect) -> Self {
        Self {
            prefab,
            anchor,
            region,
        }
    }

    /// The prefab.
    #[must_use]
    pub const fn prefab(&self) -> &Prefab {
        &self.prefab
    }

    /// Anchor used.
    #[must_use]
    pub const fn anchor(&self) -> Anchor {
        self.anchor
    }

    /// Board region occupied.
    #[must_use]
    pub const fn region(&self) -> RegionRect {
        self.region
    }
}

/// Player and enemy spawn placements.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Placement {
    player: PlacedPrefab,
    enemy:  PlacedPrefab,
}

impl Placement {
    /// Pair of placements.
    #[must_use]
    pub(in crate::lifecycle::procgen) const fn new(
        player: PlacedPrefab,
        enemy: PlacedPrefab,
    ) -> Self {
        Self { player, enemy }
    }

    /// Player placement.
    #[must_use]
    pub const fn player(&self) -> &PlacedPrefab {
        &self.player
    }

    /// Enemy placement.
    #[must_use]
    pub const fn enemy(&self) -> &PlacedPrefab {
        &self.enemy
    }
}

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

    let player_anchor = Anchor::choose(rng);

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

/// Place player and enemy spawn prefabs on the board (default packer settings).
///
/// # Errors
///
/// Returns [`PackingError`] when no fitting prefab exists for a role, a footprint does not fit its anchor region, or the player footprint is below the minimum side.
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

/// Place player and enemy spawn prefabs with explicit packer split and min player side.
///
/// # Errors
///
/// Same failure modes as [`assemble_placement`].
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
