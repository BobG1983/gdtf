use bevy::asset::uuid::Uuid;

use crate::{
    level::{
        GridHeight, GridLevels, GridSize, GridWidth, Prefab, PrefabName, PrefabRegistry,
        PrefabSpec, SpawnRole, ThemeUuid,
    },
    procgen::{
        Anchor, Footprint, MinPlayerSide, PackingError, SplitMode, assemble_placement,
        assemble_placement_with,
    },
    rng::{BattleSeed, ProcgenRng},
};

fn size(w: u8, h: u8) -> Option<GridSize> {
    GridSize::new(GridWidth::new(w), GridHeight::new(h), GridLevels::new(1)).ok()
}

fn theme() -> ThemeUuid {
    ThemeUuid::new(Uuid::from_u128(0x0149_2492_0000_0001))
}

fn prefab(theme: ThemeUuid, fp: GridSize, role: SpawnRole, stem: &str) -> Prefab {
    Prefab::new(
        PrefabName::new(stem.to_owned()),
        PrefabSpec::new(theme, fp, role, Vec::new()),
    )
}

fn registry(theme: ThemeUuid, player_fp: GridSize, enemy_fp: GridSize) -> PrefabRegistry {
    let mut r = PrefabRegistry::default();
    r.insert(prefab(theme, player_fp, SpawnRole::Player, "player_pad"));
    r.insert(prefab(theme, enemy_fp, SpawnRole::Enemy, "enemy_pad"));
    r
}

#[test]
fn places_player_and_opposite_enemy() {
    let theme = theme();
    let (Some(board), Some(player_fp), Some(enemy_fp)) = (size(40, 40), size(12, 12), size(12, 12))
    else {
        return;
    };
    let registry = registry(theme, player_fp, enemy_fp);

    let mut rng = ProcgenRng::from_root(BattleSeed::new(11));
    let result = assemble_placement(&registry, theme, board, &mut rng);
    assert!(
        result.is_ok(),
        "expected a valid placement, got {:?}",
        result.as_ref().err()
    );
    let Ok(placement) = result else {
        return;
    };

    let player_anchor = placement.player().anchor();
    assert!(
        Anchor::PLAYER_ANCHORS.contains(&player_anchor),
        "player anchor {player_anchor:?} must be one of the four (C1)",
    );
    assert_eq!(
        placement.enemy().anchor(),
        player_anchor.opposite(),
        "enemy anchor must be the strict opposite of the player anchor (OQ-2)",
    );
    assert!(
        !*placement
            .player()
            .region()
            .intersects(placement.enemy().region()),
        "player and enemy regions must not overlap (C2)",
    );
    assert!(
        Footprint::of(placement.player().prefab().spec().size).min_side()
            >= *MinPlayerSide::DEFAULT.cells(),
        "the placed player fragment must clear the 10-cell minimum (OQ-5)",
    );
}

#[test]
fn placement_is_deterministic_under_a_seed() {
    let theme = theme();
    let (Some(board), Some(player_fp), Some(enemy_fp)) = (size(40, 40), size(12, 12), size(12, 12))
    else {
        return;
    };
    let registry = registry(theme, player_fp, enemy_fp);

    let seed = BattleSeed::new(0xABCD_1234);
    let mut a = ProcgenRng::from_root(seed);
    let mut b = ProcgenRng::from_root(seed);
    let pa = assemble_placement(&registry, theme, board, &mut a);
    let pb = assemble_placement(&registry, theme, board, &mut b);
    assert_eq!(
        pa, pb,
        "the same seed must produce an identical placement (determinism, C3)",
    );
}

#[test]
fn absent_theme_uuid_yields_no_candidate() {
    let authored = theme();
    let absent = ThemeUuid::new(Uuid::from_u128(0x0149_2492_0000_00FF));
    let (Some(board), Some(player_fp), Some(enemy_fp)) = (size(40, 40), size(12, 12), size(12, 12))
    else {
        return;
    };
    let registry = registry(authored, player_fp, enemy_fp);
    let mut rng = ProcgenRng::from_root(BattleSeed::new(42));
    let result = assemble_placement(&registry, absent, board, &mut rng);
    assert!(
        matches!(
            result,
            Err(PackingError::NoPrefabForRole {
                role: SpawnRole::Player,
                ..
            })
        ),
        "an absent ThemeUuid key must yield no candidate (fail-closed NoPrefabForRole), \
         got {result:?}",
    );
    if let Err(PackingError::NoPrefabForRole { theme, .. }) = result {
        assert_eq!(
            theme, absent,
            "the error must name the requested (absent) theme"
        );
    }
}

#[test]
fn undersize_player_footprint_is_rejected_fail_closed() {
    let theme = theme();
    let (Some(board), Some(small), Some(enemy_fp)) = (size(40, 40), size(8, 8), size(12, 12))
    else {
        return;
    };
    let registry = registry(theme, small, enemy_fp);
    let mut rng = ProcgenRng::from_root(BattleSeed::new(3));
    let result = assemble_placement(&registry, theme, board, &mut rng);
    assert!(
        matches!(result, Err(PackingError::PlayerFootprintTooSmall { .. })),
        "expected PlayerFootprintTooSmall, got {result:?}",
    );
    if let Err(PackingError::PlayerFootprintTooSmall { min_side, .. }) = result {
        assert_eq!(min_side, MinPlayerSide::DEFAULT, "the 10-cell floor");
    }
}

#[test]
fn missing_prefab_is_rejected_fail_closed() {
    let theme = theme();
    let Some(board) = size(40, 40) else {
        return;
    };
    let registry = PrefabRegistry::default();
    let mut rng = ProcgenRng::from_root(BattleSeed::new(99));
    let result = assemble_placement(&registry, theme, board, &mut rng);
    assert!(
        matches!(
            result,
            Err(PackingError::NoPrefabForRole {
                role: SpawnRole::Player,
                ..
            })
        ),
        "an empty registry must fail closed with NoPrefabForRole(Player), got {result:?}",
    );
}

#[test]
fn connectivity_holds_under_guillotine_split() {
    let theme = theme();
    let (Some(board), Some(player_fp), Some(enemy_fp)) = (size(40, 40), size(12, 12), size(12, 12))
    else {
        return;
    };
    let registry = registry(theme, player_fp, enemy_fp);
    let mut rng = ProcgenRng::from_root(BattleSeed::new(5));
    let result = assemble_placement_with(
        &registry,
        theme,
        board,
        &mut rng,
        SplitMode::Guillotine,
        MinPlayerSide::DEFAULT,
    );
    assert!(
        result.is_ok(),
        "a valid placement must pass the connectivity assertion under guillotine: {:?}",
        result.err(),
    );
}

#[test]
fn enemy_footprint_that_does_not_fit_is_rejected() {
    let theme = theme();
    let (Some(board), Some(player_fp), Some(enemy_fp)) = (size(20, 20), size(12, 12), size(12, 12))
    else {
        return;
    };
    let registry = registry(theme, player_fp, enemy_fp);
    let mut rng = ProcgenRng::from_root(BattleSeed::new(1));
    let result = assemble_placement(&registry, theme, board, &mut rng);
    assert!(
        matches!(result, Err(PackingError::FootprintDoesNotFit { .. })),
        "an enemy fragment too large for the opposite region must fail closed with \
         FootprintDoesNotFit, got {result:?}",
    );
}
