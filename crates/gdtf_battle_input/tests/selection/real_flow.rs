//! The real setup-flow harness: same-update auto-select + the highlight tracks
//! the selected ganger (play-test wave 3, A2 + B).

use bevy::{asset::AssetPlugin, input::ButtonInput, prelude::*, scene::ScenePlugin};
use gdtf_battle_input::GdtfBattleInputPlugin;
use gdtf_battle_presenter::{ActiveLevel, ViewMode, cell_to_world};
use gdtf_battle_sim::{
    BattleInProgress, BattleSimPlugin, Cell, CellLevel, Faction, Level, PlayerFaction,
};
// ---------------------------------------------------------------------------------
// GTW play-test wave 3 (A2 + B) — the REAL battle-setup flow: a `SetupBattleRequested`
// (NOT a hand-inserted BattleInProgress/PlayerFaction/ganger) auto-selects a player
// ganger, and the selection highlight lands on THAT ganger's cell (never an empty cell).
//
// These drive the FULL runtime — `BattleSimPlugin` (which registers
// `setup_battle_on_request`, the system whose ordering vs the input band was the A2 bug)
// AND `GdtfBattleInputPlugin` (which registers the GTW-255 auto-select) together — so the
// ordering edge `InputSystems::Gather.after(setup_battle_on_request)` (the A2 fix) is
// exercised. The pre-existing GTW-255 tests above seed BattleInProgress + PlayerFaction +
// gangers DIRECTLY, bypassing setup — which is exactly the gap that let A2 ship.
// ---------------------------------------------------------------------------------
use gdtf_battle_sim::{
    SetupBattleRequested,
    armor::{
        ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorName, ArmorPiece, ArmorProtection,
        ArmorRegistry, ArmorSpec, ArmorType,
    },
    ganger::{Aim, Aiming, Direction, Facing, GangerName, Luck, Stance, StanceKind, Toughness},
    rng::BattleSeed,
    situation::{GangerSpawn, Situation},
    test_support::test_weapon_spec,
    tuning::CombatTuning,
    weapon::{WeaponName, WeaponRegistry},
};
use gdtf_test_utils::press_left;

use super::harness::*;

/// The weapon KEY every fixture ganger references — present in [`real_flow_registry`] so
/// the real `setup_battle_on_request` arms each spawned ganger (GTW-257).
const REAL_FLOW_WEAPON_KEY: &str = "test-weapon";

/// The armor KEY every fixture ganger references — present in
/// [`real_flow_armor_registry`] so the real `setup_battle_on_request` armors each spawned
/// ganger (GTW-269).
const REAL_FLOW_ARMOR_KEY: &str = "test-armor";

/// The test [`WeaponRegistry`] — the one [`REAL_FLOW_WEAPON_KEY`] weapon the fixture
/// gangers reference, standing in for the app's `Load`-built registry.
fn real_flow_registry() -> WeaponRegistry {
    WeaponRegistry::new([(
        WeaponName::new(REAL_FLOW_WEAPON_KEY.to_owned()),
        test_weapon_spec(),
    )])
}

/// An arbitrary armor SPEC (mechanism only) — the suit the [`REAL_FLOW_ARMOR_KEY`]
/// resolves to in [`real_flow_armor_registry`], so the real setup copies it into each
/// ganger's `WornArmor` (GTW-269).
const fn real_flow_armor(base: i32) -> ArmorSpec {
    ArmorSpec::uniform(ArmorPiece::new(
        ArmorFloor::new(base),
        ArmorProtection::new(base + 1),
        ArmorIntegrity::new(base + 2),
        ArmorHardness::new(base + 3),
        ArmorType::DEFAULT,
    ))
}

/// The test [`ArmorRegistry`] — the one [`REAL_FLOW_ARMOR_KEY`] armor suit the fixture
/// gangers reference, standing in for the app's `Load`-built registry (GTW-269).
fn real_flow_armor_registry() -> ArmorRegistry {
    ArmorRegistry::new([(
        ArmorName::new(REAL_FLOW_ARMOR_KEY.to_owned()),
        real_flow_armor(1),
    )])
}

/// Build an authored [`GangerSpawn`] at `at` (faction `faction`) with arbitrary-but-valid
/// component values, referencing the one [`REAL_FLOW_WEAPON_KEY`].
///
/// Routed through the canonical shared [`GangerSpawnBuilder`] (GTW-324). The builder's
/// default TEST armor/weapon keys are the same `"test-weapon"` / `"test-armor"` strings as
/// [`REAL_FLOW_WEAPON_KEY`] / [`REAL_FLOW_ARMOR_KEY`], so the existing `real_flow_registry`
/// / `real_flow_armor_registry` still resolve. The original fixture overrides — the
/// per-faction `"Ganger {faction}"` name, the per-faction Aim/Toughness/Luck attribute
/// offsets (the auto-select tests read no values off them, but they keep the fixture's
/// identity — GTW-384: the per-ganger DATA the derived stats compute from),
/// and the Crouching stance — are applied explicitly so the migration is value-for-value
/// identical to the prior struct literal.
fn real_flow_ganger(at: CellLevel, faction: u8) -> GangerSpawn {
    use gdtf_battle_sim::test_support::GangerSpawnBuilder;
    GangerSpawnBuilder::new()
        .at(at)
        .name(GangerName::new(format!("Ganger {faction}")))
        .faction(Faction::new(faction))
        .facing(Facing::new(Direction::East))
        .stance(Stance::new(StanceKind::Crouching))
        .aiming(Aiming::new(true))
        .aim(Aim::new(f32::from(faction) + 2.0))
        .toughness(Toughness::new(f32::from(faction) + 3.0))
        .luck(Luck::new(f32::from(faction) + 1.0))
        .build()
}

/// A two-ganger fixture (link-free → validates trivially): a PLAYER ganger (faction 0,
/// the default `player_faction`) and an ENEMY ganger (faction 1). The auto-select must
/// pick the player ganger, never the enemy.
fn real_flow_situation() -> Situation {
    use gdtf_battle_sim::test_support::SituationBuilder;
    let level = Level::new(0);
    // Built via the canonical [`SituationBuilder`] (GTW-324) — value-for-value identical to
    // the prior struct literal: the two `real_flow_ganger` gangers, no terrain/links.
    SituationBuilder::new()
        .with_gangers(vec![
            real_flow_ganger(CellLevel::new(Cell::new(5, 6), level), 0),
            real_flow_ganger(CellLevel::new(Cell::new(7, 8), level), 1),
        ])
        .build()
}

/// Builds the REAL-FLOW app: `MinimalPlugins` + BOTH the input plugin AND the sim's
/// `BattleSimPlugin`, plus the persistent `Load` resources a real app has before a battle
/// (`CombatTuning` + the `WeaponRegistry` the setup arms gangers from) and the presenter's
/// `ActiveLevel`. It deliberately does NOT insert `BattleInProgress` / `PlayerFaction` /
/// `OccupancyGrid` / any ganger — `setup_battle_on_request` creates all of those from the
/// `SetupBattleRequested` the test sends, so this exercises the production setup path.
fn real_flow_app() -> App {
    let mut app = App::new();
    // `AssetPlugin` + `ScenePlugin` are required: `setup_battle` now spawns each ganger
    // as a `bsn!` Scene via `commands.spawn_scene` (GTW-322). `Commands::spawn_scene`
    // applies the scene SYNCHRONOUSLY at the command-flush sync point (it calls
    // `apply_scene`, not the deferred `SpawnScene`-schedule path), so the ganger
    // components still materialize the SAME frame the setup runs (preserving the A2
    // same-update auto-select) — but `apply_scene` reads the `AssetServer`, which panics
    // under bare `MinimalPlugins`. In the real app both ride `DefaultPlugins`.
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), ScenePlugin))
        .add_plugins(GdtfBattleInputPlugin)
        .add_plugins(BattleSimPlugin);
    app.world_mut()
        .insert_resource(ActiveLevel::new(Level::new(0)));
    // GTW-521 — `dispatch_act_intents` also mutates the presenter-owned `ViewMode`.
    app.world_mut().insert_resource(ViewMode::default());
    app.world_mut().insert_resource(CombatTuning::default());
    app.world_mut().insert_resource(real_flow_registry());
    // GTW-505: the MeleeWeaponRegistry (with the `fists` default) so the real setup arms
    // each spawned ganger's melee weapon — the placed gangers author none, so each resolves
    // to `fists`; without it setup_battle_on_request fails closed and no ganger spawns.
    app.world_mut()
        .insert_resource(gdtf_battle_sim::test_support::test_melee_weapon_registry());
    // The Load-built ArmorRegistry (GTW-269) so the real setup armors each spawned
    // ganger (its key is present in this registry); without it setup fails closed.
    app.world_mut().insert_resource(real_flow_armor_registry());
    // GTW-414/415: the GangRegistry the v2 setup_battle resolves each placed ganger's
    // (gang, member) ref against. `real_flow_situation` references "Ganger {faction}"
    // members (the `ganger_at` convention) carrying the test weapon/armor keys, which the
    // canonical `test_gang_registry` holds — without it setup fails closed (GangNotFound)
    // and no ganger spawns to auto-select.
    app.world_mut()
        .insert_resource(gdtf_battle_sim::test_support::test_gang_registry());
    // An empty mouse buffer so the input band's click systems pass param validation.
    app.world_mut()
        .insert_resource(ButtonInput::<MouseButton>::default());
    app
}

/// Sends the real `SetupBattleRequested` (the same message `gdtf_app`'s Generation scene
/// emits) carrying `situation` and an arbitrary seed.
fn request_setup(app: &mut App, situation: Situation) {
    app.world_mut().write_message(SetupBattleRequested::new(
        situation,
        BattleSeed::new(0x5A1C),
    ));
}

/// The faction of the entity currently selected, if any (looked up via a world query).
fn selected_faction(app: &mut App) -> Option<Faction> {
    let entity = selected(app)?;
    let world = app.world_mut();
    let mut query = world.query::<&Faction>();
    query.get(world, entity).ok().copied()
}

/// A2 (the mandatory regression test) — driving the REAL battle-setup flow auto-selects a
/// PLAYER-faction ganger ON THE SAME UPDATE the setup pours the battle in. A
/// `SetupBattleRequested` (NOT a hand-inserted BattleInProgress/PlayerFaction/ganger)
/// pours the battle in via `setup_battle_on_request`; after EXACTLY ONE `app.update()`
/// `SelectedShooter` is `Some` and the selected entity is a PLAYER-faction ganger.
///
/// PIN (the A2 ordering fix, the ONE-FRAME race the bug was):
/// `setup_battle_on_request` and the GTW-255 auto-select were both merely
/// `.before(SimSystems::Simulate)` with NO order between them. Without the A2 edge
/// (`InputSystems::Gather.after(setup_battle_on_request)`) there is no apply-deferred sync
/// point between setup's `Commands` (the `PlayerFaction` insert + the ganger spawns) and
/// the input band, so on the setup update the auto-select's
/// `run_if(resource_exists::<PlayerFaction>)` is FALSE (the insert is not applied yet) —
/// nothing is selected that frame (the in-engine "No ganger selected"). WITH the edge the
/// sync point applies setup's commands before the input band runs, so the selection lands
/// the SAME update. Asserting after EXACTLY ONE update discriminates: it FAILS without the
/// edge (selection still `None` after one update — it would not appear until update 2) and
/// PASSES with it.
///
/// This is also the gap the old GTW-255 tests missed: they seeded the gate + faction +
/// gangers directly, bypassing setup's ordering vs the input band. The test does NOT seed
/// `SelectedShooter`.
#[test]
fn real_setup_flow_auto_selects_a_player_ganger_same_update() {
    let mut app = real_flow_app();

    assert_eq!(
        selected(&app),
        None,
        "precondition: nothing selected before the battle is set up",
    );

    request_setup(&mut app, real_flow_situation());

    // EXACTLY ONE update: setup processes the message + (with the A2 edge) its Commands are
    // applied at the sync point BEFORE the input band's auto-select runs the same frame.
    app.update();

    // The setup actually ran this update (its gate witness + the player faction are present).
    assert!(
        app.world().get_resource::<BattleInProgress>().is_some(),
        "the real setup must have inserted BattleInProgress from the SetupBattleRequested",
    );
    assert_eq!(
        app.world().get_resource::<PlayerFaction>().map(|p| **p),
        Some(Faction::new(0)),
        "the real setup must have seeded PlayerFaction(0) from the situation default",
    );

    // A ganger IS auto-selected THIS update, and it is the PLAYER's (faction 0), never the
    // enemy. WITHOUT the A2 edge this is still `None` after one update (the regression).
    assert!(
        selected(&app).is_some(),
        "a player ganger must be auto-selected on the SAME update the real setup runs — NOT \
         left 'No ganger selected' (A2 regression: the missing setup->Gather sync point)",
    );
    assert_eq!(
        selected_faction(&mut app),
        Some(Faction::new(0)),
        "the auto-selected ganger must be a PLAYER-faction ganger, never the enemy",
    );
}

/// B — with a ganger auto-selected via the real flow, the ONE selection-highlight sprite
/// sits on the SELECTED GANGER's cell (its real `Position`/occupancy cell), and clicking
/// BARE FLOOR does NOT move the marker onto an empty cell.
///
/// The selection highlight scans the `OccupancyGrid` for the SELECTED entity's cell, so it
/// can only ever land on an occupied (the selected ganger's) cell. A bare-floor left-click
/// with a player selection is a MOVE under GTW-238 (no selection change), so the marker
/// stays on the ganger's cell — never the clicked empty cell.
#[test]
fn selection_highlight_tracks_selected_ganger_not_empty_cells() {
    let level = Level::new(0);
    let mut app = real_flow_app();
    request_setup(&mut app, real_flow_situation());
    for _ in 0..4 {
        app.update();
    }

    // A player ganger was auto-selected (A2); its cell is the player spawn (5, 6).
    let Some(selected_entity) = selected(&app) else {
        unreachable!("A2 must have auto-selected a player ganger before checking the highlight");
    };
    let ganger_cell = Cell::new(5, 6);

    // The ONE highlight sprite sits on the SELECTED ganger's cell, visible.
    assert_eq!(
        highlight_count(&mut app),
        1,
        "exactly one selection-highlight sprite exists once a ganger is selected",
    );
    assert_eq!(
        highlight_state(&mut app),
        Some((cell_to_world(ganger_cell, level), Visibility::Visible)),
        "the selection highlight must sit on the SELECTED ganger's cell (5, 6), not elsewhere",
    );

    // Click BARE FLOOR (an empty, in-bounds, unblocked cell). Under GTW-238 a player
    // selection + an empty cell is a MOVE, NOT a re-placement of the selection marker —
    // the marker must NOT jump to the clicked empty cell.
    let empty_cell = CellLevel::new(Cell::new(20, 20), level);
    set_hovered(&mut app, Some(empty_cell));
    press_left(&mut app);
    app.update();

    // The selection is unchanged (a MOVE does not touch SelectedShooter), so the marker
    // still tracks the selected ganger's occupancy cell — not the empty clicked cell.
    assert_eq!(
        selected(&app),
        Some(selected_entity),
        "a bare-floor click must not change the selection (GTW-238 MOVE, not re-select)",
    );
    let world_at_empty = cell_to_world(Cell::new(20, 20), level);
    let highlight = highlight_state(&mut app);
    assert!(
        highlight.is_some_and(|(t, _)| t != world_at_empty),
        "the selection marker must NOT move onto the clicked bare-floor cell (20, 20) — \
         it tracks the selected ganger, never an empty cell (B)",
    );
}
