//! Shared decision fixture: the headless decision app, fog / enemy / shooter
//! authoring, and the `SystemState` `decide_left_click` driver.

use bevy::{
    ecs::system::SystemState, input::ButtonInput, math::Vec2, platform::collections::HashSet,
    prelude::*,
};
use gdtf_battle_input::{
    GdtfBattleInputPlugin, InspectTarget, LeftClickOutcome, PathPreviewTarget, SelectedFireMode,
    SelectedShooter, decide_left_click,
    fire_surface::{ShooterFireData, WeaponMagazine},
    selection::LeftClickReads,
};
use gdtf_battle_sim::{
    battle::PlayerFaction,
    ganger::{Aiming, TuMax},
    magazine::{LoadedRounds, Magazine, ReloadTu},
    prelude::{CellLevel, Faction, Level, LifeState, OccupancyGrid, Position, Tu},
    tuning::CombatTuning,
    vertical::VerticalLinkGraph,
    visibility::SquadVisibility,
    weapon::{
        FireMode, FireModeSpec, Handedness, MagazineSize, MeleeWeapon, ModeConeMult, ModeKind,
        ModeShots, ModeTuPercent, WieldedBy, Wields,
    },
};

/// The faction the player controls (matches the inserted `PlayerFaction`).
pub(crate) const PLAYER_FACTION: Faction = Faction::new(0);
/// An ENEMY faction (distinct from [`PLAYER_FACTION`]).
pub(crate) const ENEMY_FACTION: Faction = Faction::new(1);
/// The level all tests run on.
pub(crate) const LEVEL: Level = Level::new(0);
/// The synthetic window / camera render-target size (physical px).
pub(crate) const TARGET_SIZE: Vec2 = Vec2::new(1280.0, 720.0);

/// A `Single`-kind fire-mode spec with a marker `tu_percent` (arbitrary, not pinned
/// tuning).
pub(crate) const fn spec(tu_percent: f32, shots: u16) -> FireModeSpec {
    FireModeSpec::new(
        ModeKind::Single,
        ModeConeMult::new(1.0),
        ModeTuPercent::new(tu_percent),
        ModeShots::new(shots),
    )
}

/// Builds a focused headless decision app — the same `MinimalPlugins` + real
/// [`GdtfBattleInputPlugin`] harness `picking_app()` uses (the landed `selection.rs` house
/// style: carve-out (a), every mutation in a TEST BODY) — and seeds the resources the SHARED
/// [`decide_left_click`] reads beyond what the plugin `init_resource`s.
///
/// The plugin already `init_resource`s `InspectTarget` / `SelectedShooter` / `PendingActIntent`
/// / `SelectedFireMode`; this seeds the battle-scoped reads the AC2 decision also consults
/// (`OccupancyGrid` / `CombatTuning` / `PlayerFaction`), overrides `SelectedFireMode` with the
/// marker spec the FIRE branch needs, and inserts the `ButtonInput<MouseButton>` the
/// `LeftClickReads` bundle's `mouse` field requires for the `SystemState` to validate (the
/// decision never consults it — the bundle is shared with the mouse surface).
pub(crate) fn decision_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(GdtfBattleInputPlugin);
    app.world_mut().insert_resource(OccupancyGrid::default());
    app.world_mut()
        .insert_resource(SelectedFireMode::new(spec(0.2, 1)));
    app.world_mut().insert_resource(CombatTuning::default());
    app.world_mut()
        .insert_resource(PlayerFaction::new(PLAYER_FACTION));
    // GTW-356: the `LeftClickReads` bundle now reads `Res<VerticalLinkGraph>` (the OQ-4
    // link-tile gate), so seed an empty graph for the `SystemState` to validate (no links, so
    // every move target is a non-link tile unless a test adds one).
    app.world_mut()
        .insert_resource(VerticalLinkGraph::default());
    // The `LeftClickReads` bundle reads `ButtonInput<MouseButton>` (the `mouse` field) even
    // though `decide_left_click` does NOT consult it — the bundle is shared with the mouse
    // surface; insert it so the `SystemState` over `LeftClickReads` validates.
    app.world_mut()
        .insert_resource(ButtonInput::<MouseButton>::default());
    app
}

/// Spawns an armed, alive, loaded, affordable PLAYER-faction shooter at `cell` (carrying the
/// firing components `can_fire` reads + a `Position` for turn-to-face), places it in the
/// `OccupancyGrid`, and returns its entity. Takes `&mut App` (not `&mut World`): every spawn /
/// resource write is `app.world_mut()` in a TEST-helper body (carve-out (a), the
/// `selection.rs` `place_player_ganger` / `spawn_ganger` precedent).
pub(crate) fn spawn_player_shooter(app: &mut App, cell: CellLevel) -> Entity {
    let single = spec(0.2, 1);
    // The ganger carries its OWN vitals only — no weapon stat data (GTW-323 slice 3).
    let ganger = app
        .world_mut()
        .spawn((
            PLAYER_FACTION,
            Position::new(cell),
            Aiming::new(false),
            LifeState::Alive,
            Tu::new(255),
            TuMax::new(100),
        ))
        .id();
    // The weapon's FireMode + Magazine ride on a related WEAPON entity (`Wields`); the
    // `WieldedBy` insert hook populates the ganger's `Wields` synchronously in a bare
    // `World` spawn, so the shared fire decision resolves the magazine off the weapon.
    app.world_mut().spawn((
        WieldedBy::new(ganger),
        FireMode::new(vec![single]),
        Magazine::new(
            LoadedRounds::new(10),
            MagazineSize::new(30),
            ReloadTu::new(12),
        ),
        // GTW-443: the fire surface's WeaponMagazine query reads `(&Magazine, &Handedness)`.
        Handedness::OneHanded,
    ));
    app.world_mut()
        .resource_mut::<OccupancyGrid>()
        .set_occupant(cell, Some(ganger));
    ganger
}

/// Spawns an ENEMY-faction occupant at `cell` and returns its entity. Takes `&mut App`
/// (carve-out (a) — `app.world_mut()` in a test-helper body, the `selection.rs` precedent).
pub(crate) fn place_enemy(app: &mut App, cell: CellLevel) -> Entity {
    let enemy = app.world_mut().spawn(ENEMY_FACTION).id();
    app.world_mut()
        .resource_mut::<OccupancyGrid>()
        .set_occupant(cell, Some(enemy));
    enemy
}

/// Inserts a [`SquadVisibility`] with the given VISIBLE + EXPLORED cells (EXPLORED auto-includes
/// VISIBLE per the accrual invariant). The GTW-11 fire-refusal rung reads it through the shared
/// `cell_squad_visible`; absent it, the rung FAILS CLOSED (every enemy cell non-VISIBLE), so the
/// FIRE cases seed a fog that makes the target VISIBLE.
pub(crate) fn seed_fog(app: &mut App, visible: &[CellLevel], explored: &[CellLevel]) {
    let visible: HashSet<CellLevel> = visible.iter().copied().collect();
    let mut explored_set: HashSet<CellLevel> = explored.iter().copied().collect();
    explored_set.extend(visible.iter().copied());
    app.world_mut()
        .insert_resource(SquadVisibility::new(visible, explored_set));
}

/// The `SystemState` param tuple for [`decide_left_click`] (aliased to keep clippy's
/// `type_complexity` happy — the framework-plumbing carve-out for a `SystemState` tuple). The
/// [`InspectTarget`] is a SEPARATE `Res` (GTW-300: it is no longer inside [`LeftClickReads`], so
/// the click systems can hold it as a `ResMut` for the pin write without a B0002 conflict).
type DecideParams<'w, 's> = (
    LeftClickReads<'w>,
    Res<'w, InspectTarget>,
    Res<'w, PathPreviewTarget>,
    Query<'w, 's, &'static Faction>,
    Query<'w, 's, ShooterFireData<'static>>,
    Query<'w, 's, &'static Wields>,
    Query<'w, 's, WeaponMagazine<'static>, With<WieldedBy>>,
    Query<'w, 's, (), With<MeleeWeapon>>,
    Res<'w, SelectedShooter>,
);

/// Calls the SHARED [`decide_left_click`] over the `app`'s world via a `SystemState`
/// constructed in this helper's body (carve-out (a) — `app.world_mut()`, NOT a `&mut World`
/// signature). Exercising the pub decision fn with `Query` params over the real app world.
/// Since GTW-323 slice 3 the fire guard's magazine lives on the related weapon entity, so the
/// decision also takes the `Wields` relationship + the weapon-magazine query; GTW-505 C5 adds the
/// `MeleeWeapon` marker probe so the ranged weapon resolves excluding the melee one.
pub(crate) fn decide(app: &mut App) -> LeftClickOutcome {
    let world = app.world_mut();
    let mut state: SystemState<DecideParams> = SystemState::new(world);
    // `get` now returns a `Result` (Bevy 0.19); these params always validate, so
    // an `Err` is structurally impossible — fall back to the no-op outcome, which
    // would fail the calling assertion loudly rather than panic.
    let Ok((reads, inspect, move_target, factions, shooters, wields, weapons, melee, selected)) =
        state.get(world)
    else {
        return LeftClickOutcome::NoOp;
    };
    decide_left_click(
        &reads,
        &inspect,
        &move_target,
        &factions,
        &shooters,
        &wields,
        &weapons,
        &melee,
        &selected,
    )
}
