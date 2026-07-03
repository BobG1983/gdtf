//! End-to-end assembler tests (GTW-424 C1/C2/C3; GTW-492 v2 model): anchor selection,
//! opposite-side fit, determinism, the OQ-5 minimum size, and the fail-closed errors — all
//! over the UUID-keyed [`PrefabRegistry`] of [`Prefab`], keyed by a stable [`ThemeUuid`]
//! (GTW-492 C2/C5). Connectivity is by-construction via the 1-cell `default_floor` seam
//! (GTW-497 removed the old connectivity flood / fail-closed assertion), so the
//! by-construction invariant is exercised end-to-end by the emit test
//! (`emitted_level_is_in_bounds_and_fully_connected`), not by a flood here.

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

/// A footprint / board `GridSize` (clamped; `None` returns early — never panics).
fn size(w: u8, h: u8) -> Option<GridSize> {
    GridSize::new(GridWidth::new(w), GridHeight::new(h), GridLevels::new(1)).ok()
}

/// The canonical test theme key — a fixed `from_u128` [`ThemeUuid`] the test prefabs author
/// (so the assembler's theme-keyed candidate lookup resolves them).
fn theme() -> ThemeUuid {
    ThemeUuid::new(Uuid::from_u128(0x0149_2492_0000_0001))
}

/// A v2 prefab of `role` at footprint `fp` authoring NO placements (the assembler cares only
/// about footprint + role + theme — the geometry is the emit step's concern). `None` if the
/// size is invalid (it cannot be, but the no-panic contract is honoured upstream).
fn prefab(theme: ThemeUuid, fp: GridSize, role: SpawnRole, stem: &str) -> Prefab {
    Prefab::new(
        PrefabName::new(stem.to_owned()),
        PrefabSpec::new(theme, fp, role, Vec::new()),
    )
}

/// A registry with one player + one enemy prefab at the given fragment footprints under
/// `theme`. `None` if any size is invalid.
fn registry(theme: ThemeUuid, player_fp: GridSize, enemy_fp: GridSize) -> PrefabRegistry {
    let mut r = PrefabRegistry::default();
    r.insert(prefab(theme, player_fp, SpawnRole::Player, "player_pad"));
    r.insert(prefab(theme, enemy_fp, SpawnRole::Enemy, "enemy_pad"));
    r
}

/// C1/C2: a valid registry places a `>= 10x10` player-spawn at one of the four anchors and
/// an enemy-spawn at the strict opposite that FITS, connectivity assertion holding.
#[test]
fn places_player_and_opposite_enemy() {
    let theme = theme();
    // A 40x40 board with 12x12 deployment fragments — both fit at opposite anchors with a
    // seam, with plenty of floor between.
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
        !placement
            .player()
            .region()
            .intersects(placement.enemy().region()),
        "player and enemy regions must not overlap (C2)",
    );
    // The chosen player fragment cleared the OQ-5 minimum side.
    assert!(
        Footprint::of(placement.player().prefab().spec().size).min_side()
            >= MinPlayerSide::DEFAULT.cells(),
        "the placed player fragment must clear the 10-cell minimum (OQ-5)",
    );
}

/// C3 (determinism): the same seed produces an identical placement. The RNG draw order is
/// fixed (one player-anchor draw; no draw for the enemy side or prefab choice).
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

/// C2 (pin-discriminating, GTW-492): an ABSENT `ThemeUuid` key yields NO candidate — the
/// theme-keyed candidate lookup matches nothing, so `assemble_placement` fails closed with
/// `NoPrefabForRole(Player)`.
///
/// Discriminating: the SAME registry under the AUTHORED theme places a valid level
/// (`places_player_and_opposite_enemy`); switching ONLY the requested `ThemeUuid` to an
/// unregistered key turns that into a no-candidate fail-closed — so the test pins that the
/// assembler keys on the `ThemeUuid` (a lookup ignoring the theme would still find the
/// player prefab and succeed).
#[test]
fn absent_theme_uuid_yields_no_candidate() {
    let authored = theme();
    let absent = ThemeUuid::new(Uuid::from_u128(0x0149_2492_0000_00FF));
    let (Some(board), Some(player_fp), Some(enemy_fp)) = (size(40, 40), size(12, 12), size(12, 12))
    else {
        return;
    };
    // The registry's prefabs are authored under `authored`; we request `absent`.
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

/// OQ-5: with ONLY an undersize player prefab (below `~10x10`), the assembler rejects fail
/// closed with the typed error.
///
/// Discriminating: an 8x8 player footprint (below the 10 floor) must error; the 12x12 in
/// the success test above is accepted. The pair pins the floor both ways.
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

/// Fail-closed: an empty registry (no player prefab) errors with `NoPrefabForRole(Player)`
/// rather than panicking.
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

/// C3 (connectivity holds, A/B): a valid placement passes the OQ-4 connectivity assertion
/// under the guillotine split too (the assertion is split-mode independent).
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

/// C2 (opposite fit fail-closed): an enemy fragment too large for the strict-opposite
/// region (once the player fragment + seam are placed) is rejected with the typed
/// `FootprintDoesNotFit` rather than placed overlapping or panicking.
///
/// A 20x20 board with a 12x12 player fragment leaves a strip too narrow for a 12x12 enemy
/// fragment at the opposite anchor (12 + seam + 12 > 20), so the enemy fails to fit.
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
