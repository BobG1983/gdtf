use bevy::{
    app::App,
    color::Color,
    ecs::{message::Messages, system::RunSystemOnce},
};
use gdtf_battle_presenter::GangerSprites;
use gdtf_battle_sim::{
    battle::{SetupBattleRequested, setup_battle_on_request},
    ganger::{Aiming, Suppressed, SuppressorCell},
    prelude::{Cell, CellLevel, Direction, Level, Stance, StanceKind},
    rng::{BattleSeed, ShotRng},
    situation::{PlacedGanger, Situation},
    test_support::{GangerSpawnBuilder, SituationBuilder, setup_request},
};

use super::{harness::*, probes::*};

const EXTRA_FRAMES: u32 = 8;

struct FrameTint {
    frame: u32,
    color: Option<Color>,
}

struct CaseOutcome {
    trace:          Vec<FrameTint>,
    settled:        Option<Color>,
    plain:          Option<Color>,
    atlas:          Option<usize>,
    expected_atlas: usize,
}

impl CaseOutcome {
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

fn color_of_ganger_at(app: &mut App, at: CellLevel) -> Option<Color> {
    let sim = sim_entity_at(app, at)?;
    let sprite = app
        .world()
        .get_resource::<GangerSprites>()
        .and_then(|m| m.sprite_for(sim))?;
    sprite_color(app, sprite)
}

fn print_trace(label: &str, trace: &[FrameTint]) {
    for reading in trace {
        let tint = reading.color.map_or_else(
            || String::from("sprite not materialized (no Sprite component)"),
            |c| format!("tint = {:?}", c.to_linear()),
        );
        println!("[{label}] frame {:02}: {tint}", reading.frame);
    }
}

fn spawn_trace(
    app: &mut App,
    built: (Situation, Vec<PlacedGanger>),
    subject_at: CellLevel,
    suppress: bool,
) -> Vec<FrameTint> {
    app.world_mut()
        .resource_mut::<Messages<SetupBattleRequested>>()
        .write(setup_request(built, BattleSeed::new(SEED)));
    let mut trace = Vec::new();
    let mut frame: u32 = 0;
    if suppress {
        let ran = app
            .world_mut()
            .run_system_once(setup_battle_on_request)
            .is_ok();
        assert!(ran, "setup_battle_on_request must run in isolation");
        app.world_mut()
            .resource_mut::<Messages<SetupBattleRequested>>()
            .clear();
        let sim = sim_entity_at(app, subject_at);
        assert!(
            sim.is_some(),
            "the subject sim ganger must exist after setup"
        );
        if let Some(sim) = sim {
            let from = CellLevel::new(Cell::new(1, 1), Level::new(0));
            app.world_mut()
                .entity_mut(sim)
                .insert(Suppressed::new(SuppressorCell::new(from)));
        }
    } else {
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
    }
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
    for _ in 0..EXTRA_FRAMES {
        app.update();
        frame += 1;
        let color = color_of_ganger_at(app, subject_at);
        trace.push(FrameTint { frame, color });
    }
    trace
}

fn run_spawn_case(label: &str, stance: StanceKind, aiming: bool, suppress: bool) -> CaseOutcome {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let subject_at = CellLevel::new(Cell::new(5, 6), Level::new(0));
    let control_at = CellLevel::new(Cell::new(9, 8), Level::new(0));
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
