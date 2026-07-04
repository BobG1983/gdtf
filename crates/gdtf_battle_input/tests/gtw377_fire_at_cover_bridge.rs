//! GTW-377 C3 / C6b — the CLICK→FIRE→DEPLETE bridge: a left-click on a shootable cover cell
//! fires the REAL `FireRequested` toward that cell through the production input seam, and the
//! WIRED sim fire dispatch + occupancy maintenance resolve it — DEPLETING the cover until the
//! cell is freed.
//!
//! This is the END-TO-END proof the user ruling names: firing at cover must ACTUALLY happen
//! (not a highlight with no fire). It drives the REAL `GdtfBattleInputPlugin` click decision
//! (the FIRE-AT-COVER rung) + `SimActsPlugin`'s `dispatch_fire` (the message PRODUCER that
//! depletes the cover, GTW-364) + `OccupancyMaintenancePlugin`'s `sync_destroyed_cover` (the
//! CONSUMER that frees the smashed cell) — no reimplementation. It mirrors the sim-crate
//! `gtw364_cover_destroyed_bridge.rs` pattern, but the fire is driven by a real mouse CLICK
//! through the input layer, not a hand-written `FireRequested`.
//!
//! Render-free, zero pixels; every `app.world_mut()` mutation is in a TEST BODY
//! (`bevy-traps.md` #7 carve-out (a)) — no helper here takes `&mut World`/`&World`.

use bevy::{input::ButtonInput, platform::collections::HashSet, prelude::*, scene::ScenePlugin};
use gdtf_battle_input::{GdtfBattleInputPlugin, InspectTarget, SelectedFireMode, SelectedShooter};
use gdtf_battle_presenter::{ActiveLevel, FireTargetHighlight, ViewMode};
use gdtf_battle_sim::{
    Accuracy, Aiming, ArmorHardness, ArmorProtection, BaseSpread, BattleSeed, Cell, CellLevel,
    CoverEntry, CoverHp, CoverLedger, DamageProfile, DamageType, Direction, Facing, Faction,
    FatalBias, FireMode, Handedness, HandlingProfile, HeightBand, Hp, InflictedWounds, Kickback,
    Level, LifeState, Luck, Magazine, MagazineSize, OccupancyGrid, OccupancyMaintenancePlugin,
    PlayerFaction, Position, ReloadTu, Shooting, Shove, SquadVisibility, Stable, Stance,
    StanceKind, TerrainKind, Toughness, Tu, TuMax, WeaponBundle, WeaponDamage, WeaponName,
    WeaponPunch, WeaponShred, WieldedBy, Wounds,
    acts::SimActsPlugin,
    test_support::{insert_sim_resources, single_mode},
};

/// The faction the player controls (matches the inserted `PlayerFaction`).
const PLAYER_FACTION: Faction = Faction::new(1);
/// The level all the wiring runs on (the default `ActiveLevel`).
const LEVEL: Level = Level::new(0);

/// The shooter cell — West of the cover, on the ground storey.
fn shooter_cell() -> CellLevel {
    CellLevel::new(Cell::new(2, 5), LEVEL)
}

/// The cover cell — directly East of the shooter, straight ahead of an East facing (in the
/// firing arc), so the central axis aims at the cover's own band.
fn cover_cell() -> CellLevel {
    CellLevel::new(Cell::new(8, 5), LEVEL)
}

/// Build the real-path app: `MinimalPlugins` + `AssetPlugin` + `ScenePlugin` (the input plugin's
/// `update_selection_highlight` spawns its reticle via `spawn_scene`, the GTW-322 requirement) +
/// `GdtfBattleInputPlugin` (the production click decision — the FIRE-AT-COVER rung) +
/// `SimActsPlugin` (the production `dispatch_fire` that depletes the cover) +
/// `OccupancyMaintenancePlugin` (the production `sync_destroyed_cover` that frees the cell), plus
/// the resources `fire()` + the click decision read. The `FireRequested` / `CoverDestroyed`
/// buffers are registered idempotently by the plugins, so the click → fire → free hand-off flows.
fn bridge_app() -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), ScenePlugin))
        .add_plugins(GdtfBattleInputPlugin)
        .add_plugins(SimActsPlugin)
        .add_plugins(OccupancyMaintenancePlugin);
    // The canonical sim-resource litany (GTW-576) the fire path + dispatch band read —
    // the empty grids/ledgers, fog + link graph, the five RNG streams (this suite's own
    // seed: the shot geometry the breach assertions ride), empty injury content,
    // `CombatTuning::default`, and a uniform `FloorCostGrid`. COMPOSED from
    // `gdtf_battle_sim::test_support` instead of mirroring the litany line-by-line; the
    // suite-specific reads + overrides follow.
    insert_sim_resources(&mut app, BattleSeed::new(0xC0BA_17C0));
    // The battle-live gate witness + the click-decision reads.
    app.insert_resource(gdtf_battle_sim::BattleInProgress);
    app.insert_resource(ActiveLevel::new(LEVEL));
    // GTW-521 — `dispatch_act_intents` also mutates the presenter-owned `ViewMode`.
    app.insert_resource(ViewMode::default());
    // OVERRIDE the litany's gang-0 `PlayerFaction` seed: this suite's player gang is 1
    // (`insert_resource` replaces).
    app.insert_resource(PlayerFaction::new(PLAYER_FACTION));
    app.insert_resource(ButtonInput::<MouseButton>::default());
    // The presenter-owned highlight seam the input populate gates on (no renderer plugin here).
    app.insert_resource(FireTargetHighlight::cleared());
    app
}

/// Spawn an armed, alive, loaded, aiming PLAYER-faction shooter at the shooter cell facing East
/// at the cover with a HIGH-damage weapon + tight cone (so a single round lands on the cover and
/// breaches its low HP), place it in the occupancy grid, and return its entity.
fn spawn_shooter(app: &mut App) -> Entity {
    let bundle = WeaponBundle::new(
        WeaponName::new("breacher".to_owned()),
        // A near-zero cone so the round stays on the central axis (onto the cover band).
        BaseSpread::new(0.01),
        Accuracy::new(4.0),
        Kickback::new(0.0),
        FatalBias::new(0.0),
        // High damage + penetration so the single round breaches the low-HP cover.
        DamageProfile::new(
            WeaponDamage::new(200),
            WeaponPunch::new(80),
            WeaponShred::new(40),
            DamageType::Kinetic,
        ),
        HandlingProfile::new(
            Magazine::new(10, MagazineSize::new(30), ReloadTu::new(12)),
            FireMode::new(vec![single_mode(0.2, 1)]),
            Stable::new(true),
            Shove::new(false),
            Handedness::OneHanded,
        ),
    );
    let shooter = app
        .world_mut()
        .spawn((
            Position::new(shooter_cell()),
            Facing::new(Direction::East),
            Stance::new(StanceKind::Standing),
            Aiming::new(true),
            Shooting::new(1.0),
            Tu::new(250),
            TuMax::new(100),
            PLAYER_FACTION,
            (
                Hp::new(50),
                Wounds::new(10),
                LifeState::Alive,
                InflictedWounds::default(),
                Toughness::new(1.0),
                Luck::new(0.0),
            ),
        ))
        .id();
    // GTW-323 slice 2: the weapon rides on a related weapon entity (`Wields`); the `WieldedBy`
    // insert hook populates the ganger's `Wields` synchronously.
    app.world_mut().spawn((WieldedBy::new(shooter), bundle));
    app.world_mut()
        .resource_mut::<OccupancyGrid>()
        .set_occupant(shooter_cell(), Some(shooter));
    shooter
}

/// A low-HP, HIGH-band cover entry — small HP + low armor so a single high-damage round destroys
/// it. HIGH band so a standing shooter's aim lands squarely on it.
const fn low_hp_cover() -> CoverEntry {
    CoverEntry::seeded(
        CoverHp::new(10),
        HeightBand::High,
        ArmorProtection::new(2),
        ArmorHardness::new(1),
    )
}

/// Marks `cell` squad-VISIBLE (so the FIRE-AT-COVER fog gate passes).
fn mark_visible(app: &mut App, cell: CellLevel) {
    let mut visible: HashSet<CellLevel> = app
        .world()
        .get_resource::<SquadVisibility>()
        .map(|fog| fog.visible_cells().copied().collect())
        .unwrap_or_default();
    visible.insert(cell);
    app.world_mut()
        .insert_resource(SquadVisibility::new(visible.clone(), visible));
}

/// C3 / C6b — a left CLICK on a shootable cover cell, through the REAL input FIRE-AT-COVER rung
/// and the `dispatch_fire` / `sync_destroyed_cover` wiring, fires AT the cover and the sim
/// DEPLETES it: the smashed cell ends up in the destroyed-cover set and stops blocking. Drives
/// the production click → fire → free path end-to-end (no hand-written `FireRequested`). Asserts
/// the MECHANISM (the destroyed-cover set and the unblocked cell), never a magnitude.
#[test]
fn clicking_cover_fires_and_the_sim_depletes_it() {
    let mut app = bridge_app();
    // Seed the occupancy grid (with the blocking cover marker) + the cover ledger BEFORE the
    // shooter spawn so `spawn_shooter`'s occupant write lands on the same grid.
    let mut occupancy = OccupancyGrid::new();
    occupancy.set_terrain(cover_cell(), TerrainKind::Cover);
    app.insert_resource(occupancy);
    let mut cover = CoverLedger::new();
    cover.insert(cover_cell(), low_hp_cover());
    app.insert_resource(cover);

    let shooter = spawn_shooter(&mut app);
    // SELECT the shooter + a single-shot fire mode; the cover cell is squad-VISIBLE.
    app.world_mut()
        .insert_resource(SelectedShooter::new(shooter));
    app.world_mut()
        .insert_resource(SelectedFireMode::new(single_mode(0.2, 1)));
    mark_visible(&mut app, cover_cell());

    // BEFORE: the cover blocks (intact), not yet destroyed.
    {
        let grid = app.world().resource::<OccupancyGrid>();
        assert!(
            grid.is_blocked(&cover_cell()),
            "BEFORE: the standing cover cell must block",
        );
        assert!(
            !grid.is_cover_destroyed(&cover_cell()),
            "BEFORE: the cover cell must not yet be in the destroyed-cover set",
        );
    }

    // CLICK the cover cell: the input FIRE-AT-COVER rung emits the real FireRequested toward the
    // cover; `dispatch_fire` resolves the shot (depleting the cover HP + emitting CoverDestroyed);
    // `sync_destroyed_cover` folds the message into the grid's destroyed-cover set. The click
    // systems run `.before(pick_hovered_cell)`, so the injected InspectTarget is read this update
    // before the camera-less picker clobbers it. A couple of updates settle the producer→consumer
    // hand-off (buffered messages persist a frame).
    app.world_mut()
        .insert_resource(InspectTarget::new(Some(cover_cell())));
    app.world_mut()
        .resource_mut::<ButtonInput<MouseButton>>()
        .press(MouseButton::Left);
    app.update();
    app.update();

    // AFTER: the click drove the REAL fire → deplete → free chain. The cell is in the
    // destroyed-cover set, so it no longer blocks.
    let grid = app.world().resource::<OccupancyGrid>();
    assert!(
        grid.is_cover_destroyed(&cover_cell()),
        "AFTER: clicking the cover must have fired a real shot that depleted it — \
         sync_destroyed_cover marks the smashed cell destroyed (the click→fire→free chain); \
         got destroyed-set miss",
    );
    assert!(
        !grid.is_blocked(&cover_cell()),
        "AFTER: a destroyed cover cell must stop blocking (the freed cell)",
    );
}
