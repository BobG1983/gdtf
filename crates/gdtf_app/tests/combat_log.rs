//! GTW-328 (slice 3) — the battlescape combat-text LOG (bottom-left, ABOVE the weapon panel),
//! driven through the REAL app stack.
//!
//! These headless `GdtfTestAppBuilder` integration tests drive the genuine state machine down
//! to `BattleScapeState::BattleRunning`, where the real combat-log plugin spawns its container
//! and its update system drains the sim combat-event messages into fading UI text lines. They
//! cover:
//!
//! - **Lines from events** — writing the sim's `MovementOccurred` / `TurnStarted` combat-event
//!   messages makes the log gain line entities whose rendered `Text` matches the shared
//!   `classify_log_event` classifier output (the names resolved from `GangerName`).
//! - **FIFO overflow** — once more lines than the tuned `max_visible_lines` are appended, the
//!   OLDEST visible lines are despawned so the visible count is capped (the newest survive).

use bevy::{
    color::Alpha,
    ecs::entity::Entity,
    prelude::*,
    state::state::State,
    text::{FontSize, LineHeight, TextColor, TextFont},
    ui::Node,
};
use gdtf_app::test_support::{
    AppState, BattleScapeState, CombatLogLine, CombatLogRoot, RunningState,
};
use gdtf_battle_presenter::{ShotImpactResolved, severity_color};
use gdtf_battle_sim::{
    AppliedDamage, BodyPart, Cell, GainedInjury, GangerName, HitReport, HitResult, HpDamage,
    InjuryInflicted, InjuryName, InspectText, IntegrityWear, LifeState, LogText, Matchup,
    MovementOccurred, PenetratingDamage, PopupText, Severity, ShotFired, ShotKind, TurnStarted,
    injuries::InjuryRegistry, terrain::piece::TerrainRegistry, tuning::CombatTuning,
};
use gdtf_test_utils::{GdtfTestAppBuilder, advance_until};
use gdtf_ui::theme::default_theme;

/// A budget large enough to drive the deep walk into the battlescape, bounded so a machine that
/// never reaches the predicate fails instead of hanging.
const BUDGET: u32 = 96;

/// The shipped default visible-line cap (`combat_log.ron` / `MaxVisibleLines::DEFAULT`) — the
/// FIFO trim target this test asserts against. Mirrors the tuning default so the overflow test
/// is independent of the (unloaded, defaulted) RON in the headless harness.
const DEFAULT_MAX_VISIBLE: usize = 6;

/// The same cap as an `i32` for cell-coordinate arithmetic (the movement events' `y` is an `i32`
/// cell coord), so the overflow test needs no `usize`→`i32` cast (clippy `cast_possible_wrap`).
const DEFAULT_MAX_VISIBLE_I32: i32 = 6;

/// Reads the current [`BattleScapeState`] if active.
fn battlescape_state(app: &App) -> Option<BattleScapeState> {
    app.world()
        .get_resource::<State<BattleScapeState>>()
        .map(|state| *state.get())
}

/// Reads the current [`RunningState`] if active.
fn running_state(app: &App) -> Option<RunningState> {
    app.world()
        .get_resource::<State<RunningState>>()
        .map(|state| *state.get())
}

/// Drives the real stack to `BattleScapeState::BattleRunning`, where the combat log is live.
fn battle_running_app() -> App {
    let mut app = GdtfTestAppBuilder::new_with_scene_support()
        .starting_in(AppState::Running)
        .build();
    app.world_mut().insert_resource(default_theme());
    app.world_mut().insert_resource(CombatTuning::default());
    app.world_mut()
        .insert_resource(gdtf_battle_sim::weapon::WeaponRegistry::default());
    // GTW-269: setup_battle_on_request armors each ganger from an ArmorRegistry, failing closed
    // without one. This log harness builds a ganger-free default battle, so an empty registry
    // suffices — it just must be present for the setup to reach BattleRunning.
    app.world_mut()
        .insert_resource(gdtf_battle_sim::armor::ArmorRegistry::default());
    // GTW-394: the Load gate also requires a TerrainRegistry; empty clears it.
    app.world_mut().insert_resource(TerrainRegistry::default());
    app.world_mut().insert_resource(InjuryRegistry::default());

    let at_menu = advance_until(
        &mut app,
        |app| running_state(app) == Some(RunningState::Menu),
        BUDGET,
    );
    assert!(at_menu, "the walk should reach RunningState::Menu");
    app.world_mut()
        .resource_mut::<NextState<RunningState>>()
        .set(RunningState::Game);
    let at_battle = advance_until(
        &mut app,
        |app| battlescape_state(app) == Some(BattleScapeState::BattleRunning),
        BUDGET,
    );
    assert!(
        at_battle,
        "the walk should reach BattleScapeState::BattleRunning; last was {:?}",
        battlescape_state(&app),
    );
    app
}

/// All entities carrying marker `M`.
fn all_with<M: Component>(app: &mut App) -> Vec<Entity> {
    let mut q = app.world_mut().query_filtered::<Entity, With<M>>();
    q.iter(app.world()).collect()
}

/// The rendered `Text` strings of every entity carrying marker `M`.
fn line_texts<M: Component>(app: &mut App) -> Vec<String> {
    let entities = all_with::<M>(app);
    entities
        .into_iter()
        .filter_map(|e| app.world().get::<Text>(e).map(|t| t.as_str().to_owned()))
        .collect()
}

/// The (rendered `Text`, `TextColor`) of every combat-log line. The line spawns at `alpha 0`
/// (its fade-in ramps it on), so callers compare the COLOR alpha-agnostically (RGB only).
fn line_texts_and_colors(app: &mut App) -> Vec<(String, Color)> {
    let entities = all_with::<CombatLogLine>(app);
    entities
        .into_iter()
        .filter_map(|e| {
            let text = app.world().get::<Text>(e)?.as_str().to_owned();
            let color = app.world().get::<TextColor>(e)?.0;
            Some((text, color))
        })
        .collect()
}

/// Whether some combat-log line reads exactly `text` AND is drawn in `color` (RGB-only, since a
/// freshly appended line spawns transparent and fades in — the hue, not the alpha, is the signal).
fn has_log_line(lines: &[(String, Color)], text: &str, color: Color) -> bool {
    let want = color.to_srgba();
    lines.iter().any(|(t, c)| {
        let got = c.to_srgba();
        t == text
            && (got.red - want.red).abs() < 0.001
            && (got.green - want.green).abs() < 0.001
            && (got.blue - want.blue).abs() < 0.001
    })
}

/// The greatest rendered alpha across every combat-log line — the brightest line on screen.
/// `0.0` when there are no lines. Used to prove the GTW-328 slice-B fade-IN (a fresh line starts
/// transparent and ramps up) without depending on the exact per-update delta.
fn max_line_alpha(app: &mut App) -> f32 {
    let entities = all_with::<CombatLogLine>(app);
    entities
        .into_iter()
        .filter_map(|e| app.world().get::<TextColor>(e).map(|c| c.0.alpha()))
        .fold(0.0_f32, f32::max)
}

/// Spawns a NAMED ganger (so the log can resolve its `Entity` to a `GangerName`) and returns its
/// entity. The log only reads `&GangerName` off the entity — no other components are needed for
/// the movement / turn lines this test drives.
fn spawn_named(app: &mut App, name: &str) -> Entity {
    app.world_mut().spawn(GangerName::new(name.to_owned())).id()
}

/// A connecting ganger-hit `HitReport` dealing `hp` HP to `struck`'s torso — the verdict a
/// `ShotImpactResolved` (or `ShotFired`) carries for a shot that landed (classifies to a
/// damage line). A no-effect / `None` report would read as a miss; this proves the connecting path.
const fn connecting_report(struck: Entity, hp: i32) -> HitReport {
    HitReport {
        kind:            ShotKind::Ganger(struck),
        part:            Some(BodyPart::Torso),
        applied:         Some(AppliedDamage {
            matchup:    Matchup::Neutral,
            hit:        HitResult {
                penetrating: PenetratingDamage::new(0),
                hp_damage:   HpDamage::new(hp),
                wear:        IntegrityWear::new(0),
            },
            severity:   Severity::None,
            life_after: LifeState::Alive,
            broken:     None,
            worn:       None,
        }),
        cover_destroyed: None,
        slab_destroyed:  None,
        ground_accrued:  None,
        injury:          None,
    }
}

/// The number of shot-OUTCOME log lines currently visible — the lines whose text is a
/// `classify_report` pop (here the `"-N"` HP-loss line a connecting hit yields). Distinguishes the
/// staggered outcome lines from any event-driven fire-declaration / movement / turn lines.
fn outcome_line_count(app: &mut App, hp_text: &str) -> usize {
    line_texts::<CombatLogLine>(app)
        .into_iter()
        .filter(|t| t == hp_text)
        .count()
}

// ---------------------------------------------------------------------------------
// Lines from events — the log gains lines whose text matches the classifier output.
// ---------------------------------------------------------------------------------

/// A `MovementOccurred` message makes the log gain one line reading
/// `"<name> moved <from> -> <to>"` — the resolved name + the shared classifier phrasing.
#[test]
fn a_movement_message_appends_a_line_with_the_classified_text() {
    let mut app = battle_running_app();
    let ganger = spawn_named(&mut app, "Vex");
    app.update();

    // The log container exists (spawned on BattleRunning enter).
    assert_eq!(
        all_with::<CombatLogRoot>(&mut app).len(),
        1,
        "the combat-log container is spawned in BattleRunning",
    );
    // No lines yet (no events drained).
    assert!(
        all_with::<CombatLogLine>(&mut app).is_empty(),
        "the log starts empty",
    );

    app.world_mut().write_message(MovementOccurred::new(
        ganger,
        Cell::new(3, 4),
        Cell::new(3, 6),
    ));
    app.update();

    let texts = line_texts::<CombatLogLine>(&mut app);
    assert_eq!(
        texts.len(),
        1,
        "one movement event => one log line, got {texts:?}"
    );
    assert_eq!(
        texts[0], "Vex moved (3, 4) -> (3, 6)",
        "the line carries the resolved name + the classified movement phrasing",
    );
}

/// GTW-439, QA-gap remediation — the REAL system path: a genuine `InjuryInflicted` MESSAGE
/// written to the live buffer drives the registered `update_combat_log` system to APPEND one
/// combat-log line reading `"<name> <log_text>"` (the wounded target resolved to its
/// `GangerName`, the authored log clause as the predicate) in the severity-scaled wound amber
/// (`severity_color`). NOT the pure `classify_log_event` classifier (covered by its own unit
/// test) — this drives the actual `CombatLogReaders.injury` drain in a live battle and asserts
/// the appended line entity.
///
/// Pin-discriminating: it FAILS if the `InjuryInflicted` → `CombatLogEvent::InjuryInflicted`
/// arm were removed from `update_combat_log` (no line would be appended, failing the content
/// assertion), and it FAILS if the line were drawn a flat (non-severity) color, since the
/// assertion pins the EXACT `severity_color(Critical)` swatch (RGB) — distinct from a milder
/// tier's swatch.
#[test]
fn an_injury_message_appends_a_line_in_the_severity_colour() {
    let mut app = battle_running_app();
    let ganger = spawn_named(&mut app, "Vex");
    app.update();

    // The log starts empty (no events drained yet).
    assert!(
        all_with::<CombatLogLine>(&mut app).is_empty(),
        "the log starts empty",
    );

    // A REAL InjuryInflicted on the named ganger (the buffer is registered by the sim's acts
    // plugin in a live battle). The combat log reads only the target (→ LogName), log_text, and
    // severity; the gained ledger + popup / inspect texts are filler (they drive other surfaces).
    let name = InjuryName::new("Lost Eye".to_owned());
    app.world_mut().write_message(InjuryInflicted {
        target: ganger,
        gained: GainedInjury::new(
            name.clone(),
            BodyPart::Head,
            Severity::Critical,
            Vec::new(),
            InspectText::new("Lost Eye -- -2 Aim".to_owned()),
        ),
        name,
        part: BodyPart::Head,
        severity: Severity::Critical,
        popup_text: PopupText::new("LOST EYE".to_owned()),
        log_text: LogText::new("loses an eye".to_owned()),
        inspect_text: InspectText::new("Lost Eye -- -2 Aim".to_owned()),
    });
    app.update();

    // POSITIVE assertion: the registered drain appended one line reading "<name> <log_text>" in
    // the Critical severity_color (RGB-only — the line spawns transparent and fades in).
    let lines = line_texts_and_colors(&mut app);
    assert!(
        has_log_line(
            &lines,
            "Vex loses an eye",
            severity_color(Severity::Critical)
        ),
        "an InjuryInflicted must append a \"Vex loses an eye\" line in the Critical \
         severity_color, got {lines:?}",
    );
}

/// GTW-328 slice B: a freshly appended line does NOT snap to full opacity — it FADES IN. The
/// frame it spawns its alpha is well below full (it spawned transparent and the fade system has
/// only ramped it a sliver), and over subsequent updates it climbs toward full as the hold phase
/// begins. (The unit test in `components.rs` proves the exact three-phase curve; this proves the
/// real spawned line is driven by it through the app stack.)
#[test]
fn a_fresh_line_fades_in_rather_than_snapping_to_full_opacity() {
    let mut app = battle_running_app();
    let ganger = spawn_named(&mut app, "Vex");
    app.update();

    app.world_mut().write_message(MovementOccurred::new(
        ganger,
        Cell::new(3, 4),
        Cell::new(3, 6),
    ));
    // The drain frame: the line spawns (transparent) and the fade system ramps it a sliver.
    app.update();
    assert_eq!(
        all_with::<CombatLogLine>(&mut app).len(),
        1,
        "the movement event appended exactly one line",
    );
    let just_appeared = max_line_alpha(&mut app);
    assert!(
        just_appeared < 0.95,
        "a freshly appeared line must be FADING IN (alpha below full), got {just_appeared}",
    );

    // Drive enough updates to clear the (short, sub-second) fade-in window — the line reaches the
    // hold phase at (near) full opacity, proving the ramp climbs rather than staying dim or off.
    for _ in 0..64 {
        app.update();
    }
    let after_hold = max_line_alpha(&mut app);
    assert!(
        after_hold > just_appeared,
        "after the fade-in window the line's alpha must have climbed (fade-in ramp), \
         {after_hold} must exceed {just_appeared}",
    );
}

/// The shipped default combat-log line size (`combat_log.ron` / `LineFontPt::DEFAULT`) — the size
/// the headless harness (no RON, defaulted tuning) draws each line at. Mirrors the tuning default
/// so the readability test is independent of the (unloaded) RON.
const DEFAULT_LINE_FONT_PT: f32 = 20.0;

/// The theme's body text size (`assets/theme/grimdark.ron` / fallback) — the size the log USED to
/// (too-small-ly) render at before the fix. The log line size must be clearly LARGER than this.
const THEME_BODY_FONT_PT: f32 = 18.0;

/// The font size of the first combat-log line (its [`TextFont`] is set on the line entity itself).
/// `None` if there are no lines or the line carries no `TextFont` / non-`Px` size.
fn first_line_font_px(app: &mut App) -> Option<f32> {
    let entity = *all_with::<CombatLogLine>(app).first()?;
    match app.world().get::<TextFont>(entity)?.font_size {
        FontSize::Px(px) => Some(px),
        _ => None,
    }
}

/// GTW combat-log readability fix: a log line draws at the tuned `line_font_pt` (the log's OWN
/// readable size, `20.0` pt by default), NOT the theme's smaller `18.0` pt body text — so a line
/// like "Alex Mercer moved (15, 11) -> (14, 12)" is legible at a glance rather than rendering too
/// small.
#[test]
fn a_log_line_draws_at_the_larger_tuned_size_not_the_body_text() {
    let mut app = battle_running_app();
    let ganger = spawn_named(&mut app, "Alex Mercer");
    app.update();

    app.world_mut().write_message(MovementOccurred::new(
        ganger,
        Cell::new(15, 11),
        Cell::new(14, 12),
    ));
    app.update();

    let size = first_line_font_px(&mut app);
    assert_eq!(
        size,
        Some(DEFAULT_LINE_FONT_PT),
        "a normal-emphasis log line draws at the tuned line_font_pt ({DEFAULT_LINE_FONT_PT}), got \
         {size:?}",
    );
    let size = size.unwrap_or_default();
    assert!(
        size > THEME_BODY_FONT_PT,
        "the log line size ({size}) must be clearly LARGER than the {THEME_BODY_FONT_PT}pt theme \
         body text (the readability fix), so the log is legible",
    );
}

/// GTW combat-log cut-off fix: each spawned line reserves its FULL glyph box — an explicit
/// [`LineHeight::Px`] AND a matching `min_height` at least as tall as the font size — so the panel
/// clip never shaves a settled line's ascenders/descenders. (The slide-in reveal from below still
/// happens; this only guarantees the line box is fully reserved so a resting line is uncut.)
#[test]
fn a_log_line_reserves_its_full_glyph_box_so_it_is_not_clipped() {
    let mut app = battle_running_app();
    let ganger = spawn_named(&mut app, "Alex Mercer");
    app.update();

    app.world_mut().write_message(MovementOccurred::new(
        ganger,
        Cell::new(15, 11),
        Cell::new(14, 12),
    ));
    app.update();

    let lines = all_with::<CombatLogLine>(&mut app);
    assert_eq!(
        lines.len(),
        1,
        "the movement event appends exactly one line"
    );
    // `first()` is `Some` (len == 1 asserted above); bind without `unwrap` (denied even in tests).
    let Some(&entity) = lines.first() else {
        return;
    };

    // An explicit Px line height (not bevy's default RelativeToFont) at least the font size, so the
    // glyph box is reserved against the clip rather than collapsing. `is_some_and` asserts the
    // Px box AND its size in one go (no bind-then-panic — restriction lints deny `panic!` in tests).
    let line_height = app.world().get::<LineHeight>(entity).copied();
    assert!(
        line_height.is_some_and(|h| matches!(h, LineHeight::Px(px) if px >= DEFAULT_LINE_FONT_PT)),
        "a combat-log line must carry an explicit Px LineHeight box reserving at least the full \
         font height ({DEFAULT_LINE_FONT_PT}px) plus leading so descenders are not clipped; was \
         {line_height:?}",
    );

    // And the line Node's min_height matches that box, so the laid-out line never collapses shorter
    // than its glyphs (and the clipped panel's summed height accounts for it).
    let min_height = app.world().get::<Node>(entity).map(|n| n.min_height);
    assert!(
        min_height.is_some_and(|m| matches!(m, Val::Px(px) if px >= DEFAULT_LINE_FONT_PT)),
        "the line Node must reserve a Px min_height for the full glyph box ({DEFAULT_LINE_FONT_PT}px) \
         so a settled line is fully visible under the clip; was {min_height:?}",
    );
}

/// A `TurnStarted` message for the player's faction makes the log gain the `"— Player turn —"`
/// boundary line (the classifier compares `now_active` to the `PlayerFaction`).
#[test]
fn a_turn_message_appends_the_player_turn_boundary_line() {
    let mut app = battle_running_app();
    app.update();

    // The live battle resolves a PlayerFaction — drive a turn-start for it so the boundary reads
    // "Player". Assert it is present (a live battle always has it) before reading.
    let player = app
        .world()
        .get_resource::<gdtf_battle_sim::PlayerFaction>()
        .copied();
    assert!(
        player.is_some(),
        "the live battle must have a PlayerFaction resolved"
    );
    let player = player.unwrap_or_default();
    app.world_mut().write_message(TurnStarted::new(*player));
    app.update();

    let texts = line_texts::<CombatLogLine>(&mut app);
    assert!(
        texts.iter().any(|t| t == "— Player turn —"),
        "a player-faction turn boundary logs \"— Player turn —\", got {texts:?}",
    );
}

// ---------------------------------------------------------------------------------
// FIFO overflow — over the tuned cap, the OLDEST lines despawn; the newest survive.
// ---------------------------------------------------------------------------------

/// Appending MORE lines than the tuned `max_visible_lines` (default 6) FIFO-despawns the OLDEST,
/// so the visible count is capped and the most-recent lines are the survivors.
#[test]
fn overflow_fifo_despawns_the_oldest_lines() {
    let mut app = battle_running_app();
    let ganger = spawn_named(&mut app, "Vex");
    app.update();

    // Write more movement events than the cap. Each `to` cell's y is the event index (an i32 cell
    // coord), so the rendered text is uniquely identifiable per line — the survivors are the
    // highest indices.
    let total: i32 = DEFAULT_MAX_VISIBLE_I32 + 3;
    for y in 0..total {
        app.world_mut().write_message(MovementOccurred::new(
            ganger,
            Cell::new(0, 0),
            Cell::new(0, y),
        ));
    }
    // Two updates: the first drains all events + appends + trims; a second settles any deferred
    // despawns from the trim so the visible set is stable for the assert.
    app.update();
    app.update();

    let texts = line_texts::<CombatLogLine>(&mut app);
    assert_eq!(
        texts.len(),
        DEFAULT_MAX_VISIBLE,
        "the visible line count is capped at max_visible_lines ({DEFAULT_MAX_VISIBLE}), got {} \
         lines: {texts:?}",
        texts.len(),
    );
    // The OLDEST (lowest y) are gone; the newest (the last cap-many indices) survive.
    let survivors_start = total - DEFAULT_MAX_VISIBLE_I32;
    for i in survivors_start..total {
        let expected = format!("Vex moved (0, 0) -> (0, {i})");
        assert!(
            texts.contains(&expected),
            "the newest line {expected:?} must survive the FIFO trim, got {texts:?}",
        );
    }
    // The very first (oldest) line must be gone.
    let oldest = "Vex moved (0, 0) -> (0, 0)".to_owned();
    assert!(
        !texts.contains(&oldest),
        "the OLDEST line {oldest:?} must have been FIFO-despawned, got {texts:?}",
    );
}

// ---------------------------------------------------------------------------------
// GTW-328 slice A — shot-outcome lines key off the presenter's per-shot
// `ShotImpactResolved` signal (staggered per impact), NOT the fire-frame `ShotFired` drain.
// ---------------------------------------------------------------------------------

/// A `ShotImpactResolved` (the presenter's per-shot impact signal) makes the log gain ONE
/// shot-outcome line — the `classify_report` `"-N"` HP-loss line — with the shooter resolved to its
/// `GangerName`. This is the rewired drain (the log used to drain `ShotFired`).
#[test]
fn a_shot_impact_resolved_appends_the_outcome_line() {
    let mut app = battle_running_app();
    let shooter = spawn_named(&mut app, "Vex");
    let struck = spawn_named(&mut app, "Skar");
    app.update();
    assert!(
        all_with::<CombatLogLine>(&mut app).is_empty(),
        "the log starts empty",
    );

    app.world_mut().write_message(ShotImpactResolved {
        shooter,
        report: Some(connecting_report(struck, 7)),
    });
    app.update();

    assert_eq!(
        outcome_line_count(&mut app, "-7"),
        1,
        "a ShotImpactResolved carrying a 7-HP connecting hit must append one \"-7\" outcome line, \
         got {:?}",
        line_texts::<CombatLogLine>(&mut app),
    );
}

/// The drain SWITCHED (GTW-328 clause 2): writing a `ShotFired` directly produces NO shot-outcome
/// line — the log no longer drains `ShotFired` for outcomes (it drains `ShotImpactResolved`). Only
/// the per-shot impact signal yields an outcome line.
#[test]
fn a_shot_fired_no_longer_appends_an_outcome_line() {
    let mut app = battle_running_app();
    let shooter = spawn_named(&mut app, "Vex");
    let struck = spawn_named(&mut app, "Skar");
    app.update();

    // Write a ShotFired carrying a connecting report — the OLD code would have logged its outcome
    // here, on the drain frame. The buffer is registered by the sim plugins in a live battle.
    app.world_mut().write_message(ShotFired {
        shooter,
        muzzle: gdtf_battle_sim::SimPos::new(1.0, 1.0, 0.0),
        trajectory: gdtf_battle_sim::ShotDir::from_direction(Vec3::new(1.0, 0.0, 0.0)),
        impact_cell: Cell::new(2, 1),
        impact_level: gdtf_battle_sim::Level::new(0),
        kind: ShotKind::Ganger(struck),
        damage: gdtf_battle_sim::DamageType::Kinetic,
        report: Some(connecting_report(struck, 7)),
    });
    app.update();
    app.update();

    assert_eq!(
        outcome_line_count(&mut app, "-7"),
        0,
        "a ShotFired must NOT append a shot-outcome line — the log keys outcomes off \
         ShotImpactResolved now, got {:?}",
        line_texts::<CombatLogLine>(&mut app),
    );
}

/// A MULTI-ROUND volley's shot-outcome lines appear STAGGERED — ONE PER `ShotImpactResolved`, as
/// each shot's impact resolves — NOT all at once. At the drain frame (no signal yet) the log has
/// ZERO outcome lines; each subsequent staggered `ShotImpactResolved` grows the outcome-line count
/// monotonically. (The presenter's `fx_draw` test proves the SIGNALS themselves fire staggered;
/// this proves the log turns each into exactly one outcome line, in cadence.)
#[test]
fn outcome_lines_appear_staggered_one_per_impact_not_all_at_once() {
    let mut app = battle_running_app();
    let shooter = spawn_named(&mut app, "Vex");
    let struck = spawn_named(&mut app, "Skar");
    app.update();

    // The drain frame: no ShotImpactResolved written -> zero outcome lines (the bug was a whole
    // volley's lines dumping here when the log drained ShotFired).
    app.update();
    assert_eq!(
        outcome_line_count(&mut app, "-3"),
        0,
        "at the drain frame (no impact resolved yet) there must be ZERO shot-outcome lines",
    );

    // First shot's impact resolves -> exactly one outcome line.
    app.world_mut().write_message(ShotImpactResolved {
        shooter,
        report: Some(connecting_report(struck, 3)),
    });
    app.update();
    let after_first = outcome_line_count(&mut app, "-3");
    assert_eq!(
        after_first,
        1,
        "after the first shot's ShotImpactResolved exactly one \"-3\" outcome line must exist, \
         got {:?}",
        line_texts::<CombatLogLine>(&mut app),
    );

    // Second shot's impact resolves later -> the outcome-line count STRICTLY GROWS (shot-by-shot).
    app.world_mut().write_message(ShotImpactResolved {
        shooter,
        report: Some(connecting_report(struck, 3)),
    });
    app.update();
    let after_second = outcome_line_count(&mut app, "-3");
    assert!(
        after_second > after_first,
        "after the second shot's staggered impact MORE \"-3\" outcome lines must exist \
         ({after_second} must exceed {after_first}) — the lines appeared one per impact, not at once",
    );
}
