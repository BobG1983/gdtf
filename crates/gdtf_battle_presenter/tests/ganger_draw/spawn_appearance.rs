//! GTW-642: the spawn-NON-NEUTRAL appearance pin — a ganger ENTERING battle already
//! Prone / Aiming / Suppressed materializes its MODULATED tint, with NO plain-tinted
//! frame gap anywhere across the deferred `spawn_scene` materialization window.
//!
//! The twice-disputed mechanism this suite ends: the presenter sprite materializes via
//! the DEFERRED `spawn_scene` (GTW-322) one update AFTER `Added<Position>`, while the
//! change-gated appearance writer fires on the add frame, misses the not-yet-existing
//! sprite, and CONSUMES the change tick — so a spawn seed that does not already carry
//! the full classifier verdict leaves the sprite plain-tinted until the ganger's state
//! next changes (the pre-GTW-631 shape: the seed was the bare faction/life tint).
//! GTW-631 seeds the spawn through the ONE classifier (`ganger_sprite_appearance` over
//! Stance/Aiming/Suppressed/LifeState/faction), so the sprite's `Sprite` component is
//! BORN modulated. This suite pins that permanently, per the C1 probe shape: the tint is
//! read on EVERY update from the setup request to past materialization (the per-frame
//! gap probe — a plain reading on ANY frame fails), and the settled tint + atlas index
//! are compared against the expected non-neutral verdict.

use bevy::{app::App, color::Color, ecs::message::Messages};
use gdtf_battle_presenter::GangerSprites;
use gdtf_battle_sim::{
    battle::SetupBattleRequested,
    ganger::{Aiming, Suppressed, SuppressorCell},
    prelude::{Cell, CellLevel, Direction, Level, Stance, StanceKind},
    rng::{BattleSeed, ShotRng},
    situation::Situation,
    test_support::{GangerSpawnBuilder, SituationBuilder},
};

use super::{harness::*, probes::*};

/// Post-materialization probe window: enough further updates that a LATE plain re-stamp
/// (e.g. a writer overwriting the seed with the bare faction tint a frame after the scene
/// materializes) lands inside the recorded trace and fails the never-plain assert.
const EXTRA_FRAMES: u32 = 8;

/// One per-frame tint reading of the subject ganger's presenter sprite — `None` until the
/// deferred `spawn_scene` lands the `Sprite` component (no sprite exists to be plain).
struct FrameTint {
    /// The 1-based update count since the `SetupBattleRequested` write.
    frame: u32,
    /// The sprite's `Sprite.color` this frame, if the sprite has materialized.
    color: Option<Color>,
}

/// Everything a spawn-case test asserts on: the full per-frame trace, the settled
/// subject readings, the observed PLAIN reference tint, and the expected atlas index.
struct CaseOutcome {
    /// Per-frame tint readings, one per update from the setup request to settle.
    trace:          Vec<FrameTint>,
    /// The subject's settled tint (the trace's last reading).
    settled:        Option<Color>,
    /// The PLAIN reference tint, OBSERVED on the real path: the settled tint of a
    /// same-faction control ganger spawned neutral (Standing, not aiming, unsuppressed)
    /// in the same battle — for a neutral state the classifier's verdict IS the bare
    /// faction tint, i.e. exactly what the pre-GTW-631 seed stamped on every spawn.
    plain:          Option<Color>,
    /// The subject's settled atlas index.
    atlas:          Option<usize>,
    /// The expected atlas index, read structurally from the roles table + the 8->4 map.
    expected_atlas: usize,
}

impl CaseOutcome {
    /// The C1 per-frame gap probe: NO recorded frame may show the subject's sprite
    /// wearing the PLAIN tint — the sprite is either not yet materialized (`None`) or
    /// already modulated. A single plain frame is the disputed one-frame gap.
    fn assert_never_plain(&self, label: &str) {
        for reading in &self.trace {
            assert!(
                !same_color(reading.color, self.plain),
                "[{label}] frame {}: the sprite rendered PLAIN-tinted ({:?}) — the \
                 disputed spawn-window gap exists",
                reading.frame,
                reading.color,
            );
        }
    }
}

/// The `Sprite.color` of the presenter sprite mirroring the sim ganger at `at`, or
/// `None` while the sim entity, the map entry, or the sprite's deferred-scene
/// components are still absent.
fn color_of_ganger_at(app: &mut App, at: CellLevel) -> Option<Color> {
    let sim = sim_entity_at(app, at)?;
    let sprite = app
        .world()
        .get_resource::<GangerSprites>()
        .and_then(|m| m.sprite_for(sim))?;
    sprite_color(app, sprite)
}

/// Print the per-frame trace (the C1 evidence lines — run with `--nocapture` to see them).
fn print_trace(label: &str, trace: &[FrameTint]) {
    for reading in trace {
        let tint = reading.color.map_or_else(
            || String::from("sprite not materialized (no Sprite component)"),
            |c| format!("tint = {:?}", c.to_linear()),
        );
        println!("[{label}] frame {:02}: {tint}", reading.frame);
    }
}

/// Drive the REAL setup/spawn path while recording the subject's tint on EVERY update —
/// the C1 repro: `SetupBattleRequested` -> `setup_battle_on_request` -> the presenter's
/// `Added<Position>` spawn -> the deferred `spawn_scene` materialization -> settle.
///
/// With `suppress`, a [`Suppressed`] component is inserted on the subject at the setup
/// flush — AFTER the sim entity exists, BEFORE the presenter's `Added<Position>` spawn
/// system first observes the ganger — so the presenter meets a ganger that is ALREADY
/// suppressed (the situation schema cannot author `Suppressed`; the sim inserts it,
/// GTW-526).
fn spawn_trace(
    app: &mut App,
    situation: Situation,
    subject_at: CellLevel,
    suppress: bool,
) -> Vec<FrameTint> {
    app.world_mut()
        .resource_mut::<Messages<SetupBattleRequested>>()
        .write(SetupBattleRequested::new(situation, BattleSeed::new(SEED)));
    let mut trace = Vec::new();
    let mut frame: u32 = 0;
    // Phase 1: run the setup to its flush. ShotRng is inserted on the Ok setup path; it
    // and the ganger spawns land in the same end-of-update command flush, so once it is
    // visible the sim entity exists but the presenter has NOT yet observed the ganger
    // (its Added<Position> fires on the next update).
    let mut setup_done = false;
    for _ in 0..MAX_UPDATES {
        app.update();
        frame += 1;
        let color = color_of_ganger_at(app, subject_at);
        trace.push(FrameTint { frame, color });
        if app.world().get_resource::<ShotRng>().is_some() {
            setup_done = true;
            break;
        }
    }
    assert!(setup_done, "setup_battle must complete");
    if suppress {
        let sim = sim_entity_at(app, subject_at);
        assert!(
            sim.is_some(),
            "the subject sim ganger must exist at the setup flush"
        );
        if let Some(sim) = sim {
            let from = CellLevel::new(Cell::new(1, 1), Level::new(0));
            app.world_mut()
                .entity_mut(sim)
                .insert(Suppressed::new(SuppressorCell::new(from)));
        }
    }
    // Phase 2: the per-frame probe across the deferred materialization window — record
    // every update until the Sprite component exists.
    let mut materialized = false;
    for _ in 0..MAX_UPDATES {
        app.update();
        frame += 1;
        let color = color_of_ganger_at(app, subject_at);
        let done = color.is_some();
        trace.push(FrameTint { frame, color });
        if done {
            materialized = true;
            break;
        }
    }
    assert!(
        materialized,
        "the subject sprite must materialize within the settle budget"
    );
    // Phase 3: the post-materialization window (a LATE plain re-stamp would land here).
    for _ in 0..EXTRA_FRAMES {
        app.update();
        frame += 1;
        let color = color_of_ganger_at(app, subject_at);
        trace.push(FrameTint { frame, color });
    }
    trace
}

/// Run one full spawn-non-neutral case: a subject authored with the given stance / aim
/// (optionally suppressed at the flush) plus a NEUTRAL same-faction control, poured
/// through the real setup; returns the recorded trace + settled readings.
fn run_spawn_case(label: &str, stance: StanceKind, aiming: bool, suppress: bool) -> CaseOutcome {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let subject_at = CellLevel::new(Cell::new(5, 6), Level::new(0));
    let control_at = CellLevel::new(Cell::new(9, 8), Level::new(0));
    // The subject: the builder's default "Test Ganger" member (registered in the
    // canonical test_gang_registry for faction 0), non-neutral via the authored
    // stance/aiming. The control: the harness's neutral fixture (Standing, not aiming).
    let situation = SituationBuilder::new()
        .with_ganger(
            GangerSpawnBuilder::new()
                .at(subject_at)
                .stance(Stance::new(stance))
                .aiming(Aiming::new(aiming))
                .build(),
        )
        .with_ganger(ganger_at(control_at, 0, Direction::East))
        .build();

    let trace = spawn_trace(&mut app, situation, subject_at, suppress);
    print_trace(label, &trace);

    // The observed PLAIN reference: the neutral control's settled tint.
    let control_sim = sim_entity_at(&mut app, control_at);
    assert!(
        settle_actor(&mut app, control_sim),
        "the control ganger's sprite must settle"
    );
    let plain = color_of_ganger_at(&mut app, control_at);
    assert!(
        plain.is_some(),
        "the control ganger must have a settled tint"
    );
    println!(
        "[{label}] plain reference tint = {:?}",
        plain.map(|c| c.to_linear())
    );

    let settled = trace.last().and_then(|reading| reading.color);
    assert!(settled.is_some(), "the subject must have a settled tint");

    let subject_sim = sim_entity_at(&mut app, subject_at);
    let atlas = subject_sim.and_then(|sim| atlas_index_of_sim(&mut app, sim));
    let roles = character_roles(&app);
    assert!(roles.is_some(), "CharacterRoles must be resident");
    let expected_atlas = roles.map_or(0, |r| expected_index(&r, 0, Direction::East));

    CaseOutcome {
        trace,
        settled,
        plain,
        atlas,
        expected_atlas,
    }
}

/// A ganger ENTERING battle already Prone materializes DIMMED (the classifier's prone
/// verdict), with the correct atlas index and no plain-tinted frame anywhere in the
/// spawn window.
#[test]
fn prone_spawn_materializes_dimmed_with_no_plain_tinted_frame() {
    let out = run_spawn_case("prone", StanceKind::Prone, false, false);
    out.assert_never_plain("prone");
    assert!(
        luminance(out.settled) < luminance(out.plain),
        "a spawned-Prone ganger settles DIMMER than the neutral tint (settled {:?} vs \
         plain {:?})",
        out.settled,
        out.plain,
    );
    assert_eq!(
        out.atlas,
        Some(out.expected_atlas),
        "the spawned-Prone ganger's atlas index matches the classifier's structural \
         faction-base + facing-frame sum",
    );
}

/// A ganger ENTERING battle already Aiming materializes BRIGHTENED, with no plain-tinted
/// frame anywhere in the spawn window.
#[test]
fn aiming_spawn_materializes_brightened_with_no_plain_tinted_frame() {
    let out = run_spawn_case("aiming", StanceKind::Standing, true, false);
    out.assert_never_plain("aiming");
    assert!(
        luminance(out.settled) > luminance(out.plain),
        "a spawned-Aiming ganger settles BRIGHTER than the neutral tint (settled {:?} vs \
         plain {:?})",
        out.settled,
        out.plain,
    );
}

/// A ganger that is ALREADY Suppressed when the presenter first observes it materializes
/// DESATURATED + DARKENED (the GTW-526 pinned look), with no plain-tinted frame anywhere
/// in the spawn window.
#[test]
fn suppressed_spawn_materializes_desaturated_with_no_plain_tinted_frame() {
    let out = run_spawn_case("suppressed", StanceKind::Standing, false, true);
    out.assert_never_plain("suppressed");
    assert!(
        saturation(out.settled) < saturation(out.plain),
        "an already-suppressed spawn settles DESATURATED (settled saturation {:?} vs \
         plain {:?})",
        saturation(out.settled),
        saturation(out.plain),
    );
    assert!(
        luminance(out.settled) < luminance(out.plain),
        "an already-suppressed spawn settles DARKER than the neutral tint (settled {:?} \
         vs plain {:?})",
        out.settled,
        out.plain,
    );
}
