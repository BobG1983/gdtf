//! The GTW-529 dev fall trigger: the `FallAtFrame` parse gate + the headless drive of
//! the REAL `apply_falls` path (the `FallLog` recorder + `spawn_fall_test_app` seeding
//! live here with the test they serve).

// The parse core is not re-exported from `mod.rs` (only `DevCapturePlugin` is, for the
// binary), so reach it through its home submodule.
use super::super::trigger_config::FallAtFrame;

/// GTW-529 — the REAL fall-frame parse ([`FallAtFrame::parse`], the core of `from_env`) is
/// `Some` for a valid `u32` and `None` (trigger inert) for an absent / empty / non-numeric
/// value — the gate the dev fall-trigger keys on. Mirrors the fire-trigger's
/// `fire_at_frame_parses_or_disables`.
#[test]
fn fall_at_frame_parses_or_disables() {
    assert_eq!(FallAtFrame::parse(Some("12")), Some(FallAtFrame::new(12)));
    assert_eq!(FallAtFrame::parse(Some(" 3 ")), Some(FallAtFrame::new(3)));
    for off in [None, Some(""), Some("  "), Some("x"), Some("-1")] {
        assert!(
            FallAtFrame::parse(off).is_none(),
            "{off:?} should leave the fall trigger inert",
        );
    }
}

/// Read a ganger's current storey (its `Position` level `z`) — the fall-drop probe the
/// GTW-529 end-to-end test reads before / after the forced fall. A module-level helper
/// (never after statements). `bevy` paths are fully qualified so it needs no module-top
/// imports (the test's own `use` block is local to the test fn).
fn ganger_level(app: &bevy::app::App, entity: bevy::ecs::entity::Entity) -> u8 {
    use gdtf_battle_sim::prelude::Position;

    let pos = app
        .world()
        .get::<Position>(entity)
        .copied()
        .unwrap_or_else(Position::default);
    // The canonical CellLevel::level accessor through Position's deref (GTW-565).
    *pos.level()
}

/// Every `FallOccurred` observed across the GTW-529 end-to-end run (a `MessageReader` sees
/// only the current + previous update, so recording each into a resource lets the asserts read
/// the full run). A module-level type so the seeding helper can register it.
#[derive(bevy::ecs::resource::Resource, Default)]
struct FallLog(Vec<gdtf_battle_sim::falls::FallOccurred>);

/// Build the minimal app + seed the resources / player ganger the GTW-529 fall end-to-end test
/// drives, returning it wired for a frame-N fall alongside the chosen ganger's `Entity`.
///
/// Extracted from [`trigger_fall_at_frame_drops_a_player_ganger_via_the_real_path`] so that test
/// stays under the `too_many_lines` lint (mirroring how `spawn_dev_menu_buttons` was split out of
/// `spawn_menu`). Registers BOTH the REAL `trigger_fall_at_frame` (ordered `.before(apply_falls)`,
/// exactly as `DevCapturePlugin::build` wires it) AND the production `apply_falls` (via the GTW-523
/// `FallsPlugin`), plus a recorder that drains `FallOccurred` into [`FallLog`]. The grids / tuning
/// / injury content + the two RNG streams `apply_falls` reads are all seeded (the `fall_resolution`
/// seeding, so the fall resolves rather than fail-closing); `CombatTuning::default()` suffices since
/// the test asserts the DROP, never a pinned damage magnitude. The ganger spawns on the GROUND
/// (level 0), exactly where the shipped skirmish places it (no upper-storey placement).
fn spawn_fall_test_app(fall_frame: FallAtFrame) -> (bevy::app::App, bevy::ecs::entity::Entity) {
    use bevy::prelude::*;
    use gdtf_battle_input::{SelectedFireMode, SelectedShooter};
    use gdtf_battle_sim::{
        acts::InjuryInflicted,
        battle::PlayerFaction,
        falls::{FallOccurred, FallsPlugin, apply_falls},
        ganger::{Hp, Luck, Toughness, Wounds},
        inflicted_wound::InflictedWounds,
        injuries::{InjuryRegistry, InjuryTables},
        occupancy_sync::{OccupancyMaintenancePlugin, SlabDestroyed},
        prelude::{Cell, CellLevel, Faction, Level, LifeState, OccupancyGrid, Position},
        rng::{BattleSeed, InjuryRng, SeverityRng},
        surface::SurfaceGrid,
        tuning::CombatTuning,
    };

    use super::super::{plugin::FallConfig, triggers::trigger_fall_at_frame};

    /// An arbitrary fixed seed — determinism is the property, the value is irrelevant.
    const SEED: u64 = 0x0529_FA11_DEAD_BEEF;

    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        // The SlabDestroyed buffer the trigger WRITES + apply_falls READS, and the
        // InjuryInflicted buffer apply_falls writes — registered explicitly (this minimal app
        // omits SimActsPlugin, which normally registers them).
        .add_message::<SlabDestroyed>()
        .add_message::<InjuryInflicted>()
        // The GTW-523 fall wiring: registers apply_falls (+ the FallOccurred buffer). The
        // occupancy maintenance plugin provides sync_moved_gangers / sync_destroyed_slab that
        // apply_falls orders `.after`, so the involuntary-drop Position write settles.
        .add_plugins(OccupancyMaintenancePlugin)
        .add_plugins(FallsPlugin)
        .init_resource::<FallLog>();

    // The grids / tuning / injury content + the two RNG streams apply_falls reads (all present,
    // so the fall resolves — the fall_resolution seeding). CombatTuning::default() suffices:
    // this test asserts the DROP (Position + FallOccurred), never a pinned damage magnitude.
    let root = BattleSeed::new(SEED);
    app.insert_resource(SurfaceGrid::new());
    app.insert_resource(OccupancyGrid::new());
    app.insert_resource(CombatTuning::default());
    app.insert_resource(InjuryTables::default());
    app.insert_resource(InjuryRegistry::default());
    app.insert_resource(SeverityRng::from_root(root));
    app.insert_resource(InjuryRng::from_root(root));

    let player = Faction::new(0);
    app.insert_resource(PlayerFaction::new(player));
    app.insert_resource(SelectedFireMode::default());

    // A player-faction ganger with the full falls-query component set, standing on the GROUND
    // (level 0) — exactly where the shipped skirmish spawns it (no upper-storey placement).
    let ganger = app
        .world_mut()
        .spawn((
            player,
            Position::new(CellLevel::new(Cell::new(4, 4), Level::new(0))),
            Hp::new(1000),
            Wounds::new(200),
            LifeState::Alive,
            InflictedWounds::default(),
            Toughness::new(0.0),
            Luck::new(0.0),
        ))
        .id();
    app.insert_resource(SelectedShooter::new(ganger));

    // Force the fall on the chosen BattleRunning frame, ordered `.before(apply_falls)` — the same
    // wiring DevCapturePlugin::build uses, so the same-frame SlabDestroyed + elevated Position
    // reach apply_falls this frame.
    app.insert_resource(FallConfig::new(fall_frame));
    app.add_systems(
        Update,
        (
            trigger_fall_at_frame.before(apply_falls),
            |mut reader: MessageReader<FallOccurred>, mut log: ResMut<FallLog>| {
                for signal in reader.read() {
                    log.0.push(*signal);
                }
            },
        ),
    );

    (app, ganger)
}

/// GTW-529 — `trigger_fall_at_frame`, at frame N, forces a determinate player ganger to FALL
/// via the REAL GTW-523 path: the chosen ganger's `Position` DROPS to the ground and a
/// `FallOccurred` fires — proving the fall resolves through the wired `apply_falls`, not a
/// fake.
///
/// Drives BOTH the REAL `trigger_fall_at_frame` (ordered `.before(apply_falls)`, exactly as
/// `DevCapturePlugin::build` wires it) AND the production `apply_falls` (via the GTW-523
/// `FallsPlugin`) on a minimal app (built by [`spawn_fall_test_app`]). The chosen player ganger
/// spawns on the GROUND (level 0), as the shipped skirmish places every ganger; the trigger
/// elevates it to storey 1 and smashes the slab under it, so `apply_falls` drops it back to the
/// ground (a 1-storey fall) — the visual (the drop + the GTW-524 impact flash) is
/// Screenshot-QA-covered, this test pins the SIM effect.
///
/// PIN: goes red if the trigger fires on the wrong frame, fails to pick a player ganger,
/// stops elevating-then-smashing on the real `SlabDestroyed` path, or the ordering slips so
/// `apply_falls` misses the same-frame drop.
#[test]
fn trigger_fall_at_frame_drops_a_player_ganger_via_the_real_path() {
    use gdtf_battle_sim::prelude::Level;

    let (mut app, ganger) = spawn_fall_test_app(FallAtFrame::new(3));

    // Frames 1 + 2: the trigger is silent and the ganger stays on the ground.
    app.update();
    app.update();
    assert_eq!(
        ganger_level(&app, ganger),
        0,
        "before the trigger frame the ganger stays on the ground",
    );
    assert!(
        app.world().resource::<FallLog>().0.is_empty(),
        "no fall fires before the target frame",
    );

    // Frame 3: the trigger elevates the ganger to storey 1 and smashes the slab; apply_falls
    // (same frame) drops it back to the ground. Then settle a frame so the buffered signal /
    // Position write are fully observable.
    app.update();
    app.update();

    assert_eq!(
        ganger_level(&app, ganger),
        0,
        "the forced fall drops the elevated ganger back to the ground (storey 0)",
    );
    let falls = &app.world().resource::<FallLog>().0;
    assert_eq!(
        falls.len(),
        1,
        "exactly one FallOccurred fired for the forced fall (the real apply_falls resolved it)",
    );
    let signal = falls[0];
    assert_eq!(
        signal.ganger, ganger,
        "the fall is for the chosen player ganger"
    );
    assert_eq!(
        signal.from_level,
        Level::new(1),
        "elevated to storey 1 before the smash"
    );
    assert_eq!(signal.to_level, Level::new(0), "landed on the ground");
    assert_eq!(*signal.storeys, 1, "storey 1 → 0 is a one-storey fall");

    // The trigger is one-shot: later frames force no further fall.
    app.update();
    app.update();
    assert_eq!(
        app.world().resource::<FallLog>().0.len(),
        1,
        "the fall trigger fires exactly once",
    );
}
