use std::time::Duration;

use bevy::{app::App, ecs::message::Messages, time::TimeUpdateStrategy};
use gdtf_battle_presenter::{PlaybackCursor, PlaybackTuning, StampedGraphic};
use gdtf_battle_sim::{
    act_log::{ActDeed, ActLog, ActProvenance, ActSeq, RecordedAct},
    battle::BattleReady,
    cover::CoverLedger,
    entity::TerrainPieceKind,
    occupancy::{TerrainKind, TerrainPlacement},
    occupancy_sync::TerrainPieceDestroyed,
    prelude::{BattleInProgress, Cell, CellLevel, Level},
    surface::{SlabState, SurfaceGrid},
};

use super::harness::*;

// How many manual frames the bounded advance loop is allowed before it gives up.
const MAX_FRAMES: usize = 16;

fn shown(app: &App) -> ActSeq {
    app.world().resource::<PlaybackCursor>().shown()
}

fn holding(app: &App) -> bool {
    app.world().resource::<PlaybackCursor>().is_holding()
}

fn raw_destroyed_present(app: &App) -> bool {
    app.world()
        .contains_resource::<Messages<TerrainPieceDestroyed>>()
}

// One manual frame, longer than the shortest dwell.
fn hold_step(app: &App) -> Duration {
    let minor = *app.world().resource::<PlaybackTuning>().minor_seconds;
    Duration::from_secs_f32(minor.max(0.0) + 0.05)
}

// An act log holding a detaining minor deed then the smash, whose sequence it returns.
fn detained_smash_log(app: &mut App, at: CellLevel, kind: TerrainPieceKind) -> ActSeq {
    let actor = app.world_mut().spawn_empty().id();
    let mut log = ActLog::default();
    log.append(RecordedAct::new(
        actor,
        ActProvenance::Commanded,
        ActDeed::BleedStarted,
    ));
    let smash = log.append(RecordedAct::new(
        actor,
        ActProvenance::Commanded,
        ActDeed::TerrainPieceSmashed { at, kind },
    ));
    app.world_mut().insert_resource(log);
    smash
}

// Steps the manual clock until the cursor plays `seq`, running `each_frame` after each update.
fn play_past(app: &mut App, seq: ActSeq, each_frame: impl Fn(&App)) {
    let step = hold_step(app);
    app.insert_resource(TimeUpdateStrategy::ManualDuration(step));
    for _ in 0..MAX_FRAMES {
        app.update();
        each_frame(app);
        if shown(app) > seq {
            return;
        }
    }
    assert!(
        shown(app) > seq,
        "the cursor never played the smash entry {seq:?} within {MAX_FRAMES} manual frames of \
         {step:?} — it is still at {:?}",
        shown(app),
    );
}

fn draw_two_slabs(app: &mut App, slab_key: CellLevel, other_slab_key: CellLevel) {
    insert_occupancy(app, Vec::new());
    app.world_mut().insert_resource(CoverLedger::new());
    let mut surface = SurfaceGrid::new();
    surface.set_slab(slab_key, SlabState::Present);
    surface.set_slab(other_slab_key, SlabState::Present);
    app.world_mut().insert_resource(surface);
    app.world_mut().insert_resource(BattleInProgress);
    app.world_mut()
        .resource_mut::<Messages<BattleReady>>()
        .write(BattleReady);
    app.update();
}

#[test]
fn the_slab_swap_waits_until_the_cursor_plays_the_smash() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let l0 = Level::new(0);
    let slab_key = CellLevel::new(Cell::new(4, 5), l0);
    let other_slab_key = CellLevel::new(Cell::new(6, 7), l0);
    draw_two_slabs(&mut app, slab_key, other_slab_key);

    let defs = sprite_defs(&app);
    assert!(
        defs.is_some(),
        "the SpriteDefRegistry must be resident after settle"
    );
    let Some(defs) = defs else { return };

    assert_ne!(
        def_rect(&defs, "slab_destroyed"),
        def_rect(&defs, "slab"),
        "the slab_destroyed def rect must differ from the intact slab rect (a real swap)",
    );
    assert_eq!(
        sprite_rect_at(&mut app, slab_key),
        def_rect(&defs, "slab"),
        "the slab cell's sprite must start on the intact slab def's rect",
    );
    let entity_before = sprite_entity_at(&mut app, slab_key);
    assert!(
        entity_before.is_some(),
        "the slab cell's terrain sprite must exist before the destruction",
    );
    let other_rect_before = sprite_rect_at(&mut app, other_slab_key);

    // What the fire path emits at resolve time: the smash deed and the raw message, one step.
    let smash = detained_smash_log(&mut app, slab_key, TerrainPieceKind::Slab);
    app.world_mut()
        .resource_mut::<Messages<TerrainPieceDestroyed>>()
        .write(TerrainPieceDestroyed::new(slab_key, TerrainPieceKind::Slab));

    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::ZERO));
    app.update();

    assert!(
        holding(&app),
        "the cursor must be holding on the detaining minor deed, or nothing keeps the smash \
         unplayed and this case measures no timing at all",
    );
    assert_eq!(
        shown(&app),
        smash,
        "the cursor must be waiting ON the smash entry, not past it",
    );
    assert_eq!(
        sprite_rect_at(&mut app, slab_key),
        def_rect(&defs, "slab"),
        "the slab must still draw INTACT while the cursor holds short of the smash — the raw \
         message the sim wrote at resolve time must draw nothing on its own — found {:?}",
        sprite_rect_at(&mut app, slab_key),
    );

    play_past(&mut app, smash, |_| {});

    assert_eq!(
        sprite_rect_at(&mut app, slab_key),
        def_rect(&defs, "slab_destroyed"),
        "once the cursor plays the smash, the slab cell's sprite must carry the \
         `slab_destroyed` def's rect — found {:?}",
        sprite_rect_at(&mut app, slab_key),
    );
    assert_eq!(
        stamped_graphic_at(&mut app, slab_key),
        Some(StampedGraphic::from_key("slab_destroyed")),
        "a played Slab-kind destruction leaves the cell stamped with the destroyed-slab role, \
         not rubble — the shipped slab_destroyed and rubble defs share a rect, so this is what \
         names which swap stamped the cell — found {:?}",
        stamped_graphic_at(&mut app, slab_key),
    );
    assert_eq!(
        sprite_entity_at(&mut app, slab_key),
        entity_before,
        "the destroyed slab cell must be the SAME Entity after the swap (no despawn/respawn)",
    );
    assert_eq!(
        sprite_rect_at(&mut app, other_slab_key),
        other_rect_before,
        "the other (intact) slab sprite must be untouched by the slab destruction",
    );
}

#[test]
fn a_played_cover_smash_leaves_the_cover_cell_alone() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let cover_key = CellLevel::new(Cell::new(9, 8), Level::new(0));

    insert_occupancy(
        &mut app,
        vec![TerrainPlacement::new(cover_key, TerrainKind::Cover)],
    );
    let mut cover_ledger = CoverLedger::new();
    cover_ledger.insert(cover_key, low_cover_entry());
    app.world_mut().insert_resource(cover_ledger);
    app.world_mut().insert_resource(SurfaceGrid::new());
    app.world_mut().insert_resource(BattleInProgress);
    app.world_mut()
        .resource_mut::<Messages<BattleReady>>()
        .write(BattleReady);
    app.update();

    let defs = sprite_defs(&app);
    assert!(
        defs.is_some(),
        "the SpriteDefRegistry must be resident after settle"
    );
    let Some(defs) = defs else { return };

    assert_ne!(
        def_rect(&defs, "cover"),
        def_rect(&defs, "slab_destroyed"),
        "the cover and slab_destroyed def rects must differ, or a dropped kind filter would be \
         invisible on this cell",
    );
    assert_eq!(
        sprite_rect_at(&mut app, cover_key),
        def_rect(&defs, "cover"),
        "the cover cell's sprite must start on the `cover` def's rect",
    );

    // This cell gets no raw destroyed message, only the played fact.
    let smash = detained_smash_log(&mut app, cover_key, TerrainPieceKind::Cover);
    play_past(&mut app, smash, |_| {});

    assert_eq!(
        sprite_rect_at(&mut app, cover_key),
        def_rect(&defs, "cover"),
        "a played Cover-kind smash must leave the cover cell's rect alone — swap_destroyed_slab \
         acts on Slab and nothing else — found {:?}",
        sprite_rect_at(&mut app, cover_key),
    );
    assert_eq!(
        stamped_graphic_at(&mut app, cover_key),
        Some(StampedGraphic::from_key("cover")),
        "the cover cell must still be stamped with the cover role — a slab-destroyed stamp here \
         is the kind filter dropped from the played reader — found {:?}",
        stamped_graphic_at(&mut app, cover_key),
    );
}

#[test]
fn the_slab_swap_runs_with_only_the_played_destroyed_buffer_in_the_app() {
    let mut app = headless_renderer_app_without_raw_destroyed();
    settle_resources(&mut app);

    let l0 = Level::new(0);
    let slab_key = CellLevel::new(Cell::new(4, 5), l0);
    let other_slab_key = CellLevel::new(Cell::new(6, 7), l0);
    draw_two_slabs(&mut app, slab_key, other_slab_key);

    let defs = sprite_defs(&app);
    assert!(
        defs.is_some(),
        "the SpriteDefRegistry must be resident after settle"
    );
    let Some(defs) = defs else { return };
    assert_eq!(
        sprite_rect_at(&mut app, slab_key),
        def_rect(&defs, "slab"),
        "the slab cell's sprite must start on the intact slab def's rect",
    );
    assert!(
        !raw_destroyed_present(&app),
        "this case's app must never register the raw TerrainPieceDestroyed buffer",
    );

    let smash = detained_smash_log(&mut app, slab_key, TerrainPieceKind::Slab);
    play_past(&mut app, smash, |app| {
        assert!(
            !raw_destroyed_present(app),
            "the raw TerrainPieceDestroyed buffer must stay absent across every update, or this \
             case cannot tell a gate on the raw buffer from a gate on the played one",
        );
    });

    assert_eq!(
        sprite_rect_at(&mut app, slab_key),
        def_rect(&defs, "slab_destroyed"),
        "the log alone must drive the slab swap: a gate left on the raw TerrainPieceDestroyed \
         buffer never runs swap_destroyed_slab in this app, leaving the cell intact — found {:?}",
        sprite_rect_at(&mut app, slab_key),
    );
    assert_eq!(
        stamped_graphic_at(&mut app, slab_key),
        Some(StampedGraphic::from_key("slab_destroyed")),
        "the cell must be stamped with the destroyed-slab role with no raw buffer in the app — \
         found {:?}",
        stamped_graphic_at(&mut app, slab_key),
    );
}
