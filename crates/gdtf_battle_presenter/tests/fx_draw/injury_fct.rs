//! Injury-inflicted severity-coloured FCT pop (GTW-439 C1).

use bevy::{ecs::message::Messages, transform::components::Transform};
use gdtf_battle_presenter::{FloatingCombatText, cell_to_world, severity_color};
use gdtf_battle_sim::{
    acts::InjuryInflicted,
    armor::BodyPart,
    injuries::{GainedInjury, InjuryName, InspectText, LogText, PopupText},
    prelude::{BattleInProgress, Cell, CellLevel, Level, Position},
    severity::Severity,
};

use super::{harness::*, probes::*};

/// Builds an [`InjuryInflicted`] for `target` carrying `popup` (the FCT line) at `severity`,
/// with sane filler for the durable ledger + the log / inspect texts the FCT reader ignores.
///
/// The injury family classify reads ONLY the message's `target`, `popup_text`, and
/// `severity`; the gained ledger entry + log / inspect texts are along for the ride (they
/// drive the applier + the combat log + the inspect panel, exercised by their own tests), so
/// they are filled with self-consistent placeholders here.
fn injury_message(
    target: bevy::ecs::entity::Entity,
    popup: &str,
    severity: Severity,
) -> InjuryInflicted {
    let name = InjuryName::new("Lost Eye".to_owned());
    InjuryInflicted {
        target,
        gained: GainedInjury::new(
            name.clone(),
            BodyPart::Head,
            severity,
            Vec::new(),
            InspectText::new("Lost Eye -- -2 Aim".to_owned()),
        ),
        name,
        part: BodyPart::Head,
        severity,
        popup_text: PopupText::new(popup.to_owned()),
        log_text: LogText::new("loses an eye".to_owned()),
        inspect_text: InspectText::new("Lost Eye -- -2 Aim".to_owned()),
    }
}

/// GTW-439 (slice C1), QA-gap remediation — the REAL system path: a genuine `InjuryInflicted`
/// MESSAGE written to the live buffer drives the REGISTERED `read_consequence_fct::<InjuryFct>` family reader (GTW-572) to SPAWN
/// one floating-combat-text pop over the wounded ganger's cell, carrying the injury's
/// `popup_text` verbatim, drawn in the severity-scaled `severity_color`. NOT the pure
/// `injury_pop` classifier (covered in the module unit test) — this drives the actual message
/// consumer registered by `TopDownRendererPlugin` and asserts the rendered entity.
///
/// Pin-discriminating: it FAILS if the injury family reader stopped consuming `InjuryInflicted` (or
/// stopped spawning a pop) — the world would then carry ZERO `FloatingCombatText`, failing the
/// content assertion — and it FAILS if the pop were drawn a flat (non-severity) color, since the
/// assertion pins the EXACT `severity_color(Critical)` swatch (RGB), distinct from the `Minor`
/// swatch the ramp would yield for a milder tier.
#[test]
fn injury_inflicted_message_spawns_the_severity_coloured_fct_pop() {
    let mut app = headless_renderer_app();
    // The FCT injury reader is gated on its buffer existing (bevy-traps.md #4 — a MessageReader
    // param panics validation without it); the sim's acts plugin registers it in a live battle,
    // this focused harness adds only the buffers its readers need.
    app.add_message::<InjuryInflicted>();
    settle_resources(&mut app);
    app.world_mut().insert_resource(BattleInProgress);

    // A wounded ganger carrying a Position (the reader anchors the pop at its cell, fail-closed).
    let cell = Cell::new(7, 5);
    let level = Level::new(0);
    let target = app
        .world_mut()
        .spawn(Position::new(CellLevel::new(cell, level)))
        .id();

    // Write a REAL InjuryInflicted to the live buffer and run one update — the registered
    // the injury family reader drains it and spawns the pop on this frame's SpawnScene schedule.
    app.world_mut()
        .resource_mut::<Messages<InjuryInflicted>>()
        .write(injury_message(target, "LOST EYE", Severity::Critical));
    app.update();

    // POSITIVE assertion: the system spawned a FloatingCombatText pop reading the popup_text
    // verbatim, in the severity-scaled wound amber for the rolled Critical tier.
    let pops = fct_pops(&mut app);
    assert_eq!(
        pop_count_for(&pops, "LOST EYE"),
        1,
        "the registered injury family reader must spawn exactly one \"LOST EYE\" pop for one \
         InjuryInflicted, got {pops:?}",
    );
    assert!(
        has_fct_pop(&pops, "LOST EYE", severity_color(Severity::Critical)),
        "the injury pop must render the popup_text in the Critical severity_color, got {pops:?}",
    );
    // The pop is anchored at the wounded ganger's cell (planar x of cell_to_world; the FCT z is
    // the Highlight band, distinct from the cell z).
    let anchor = cell_to_world(cell, level);
    let mut q = app.world_mut().query::<(&FloatingCombatText, &Transform)>();
    let any_at_cell = q
        .iter(app.world())
        .any(|(_, transform)| (transform.translation.x - anchor.x).abs() < 0.001);
    assert!(
        any_at_cell,
        "the injury pop must anchor at the wounded ganger's cell x ({})",
        anchor.x,
    );
}
