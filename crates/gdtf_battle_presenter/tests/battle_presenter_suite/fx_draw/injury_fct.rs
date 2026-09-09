use bevy::transform::components::Transform;
use gdtf_battle_presenter::{FloatingCombatText, cell_to_world, severity_color};
use gdtf_battle_sim::{
    acts::InjuryInflicted,
    armor::BodyPart,
    injuries::{GainedInjury, InjuryName, InspectText, LogText, PopupText},
    prelude::{BattleInProgress, Cell, CellLevel, Level, Position},
    severity::Severity,
};

use super::{harness::*, probes::*};

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

#[test]
fn injury_inflicted_message_spawns_the_severity_coloured_fct_pop() {
    let mut app = headless_renderer_app();
    app.add_message::<InjuryInflicted>();
    settle_resources(&mut app);
    app.world_mut().insert_resource(BattleInProgress);

    let cell = Cell::new(7, 5);
    let level = Level::new(0);
    let target = app
        .world_mut()
        .spawn(Position::new(CellLevel::new(cell, level)))
        .id();

    play(
        &mut app,
        injury_message(target, "LOST EYE", Severity::Critical),
    );
    app.update();

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
