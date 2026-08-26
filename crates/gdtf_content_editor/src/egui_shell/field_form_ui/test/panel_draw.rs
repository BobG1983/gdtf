use bevy_egui::egui;

use crate::{
    egui_shell::field_form_ui::{def_panel, field_stack},
    field_form::FieldDraft,
    save_record::LastSaveRecord,
};

// The pane the test hands egui, big enough that nothing is clipped away.
fn screen() -> egui::Rect {
    egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1200.0, 900.0))
}

// Every string egui painted this frame, gathered out of the shapes it emitted.
fn drawn_text(shape: &egui::epaint::Shape, into: &mut Vec<String>) {
    match shape {
        egui::epaint::Shape::Vec(shapes) => {
            for child in shapes {
                drawn_text(child, into);
            }
        }
        egui::epaint::Shape::Text(text) => into.push(text.galley.text().to_owned()),
        _ => {}
    }
}

// Draw both Field panels over a bare context and read back what they painted.
fn draw(draft: &mut FieldDraft) -> Vec<String> {
    let ctx = egui::Context::default();
    let input = egui::RawInput {
        screen_rect: Some(screen()),
        ..Default::default()
    };
    let mut last_save = LastSaveRecord::default();
    ctx.begin_pass(input);
    {
        let mut ui = egui::Ui::new(
            ctx.clone(),
            egui::Id::new("field_panel_draw"),
            egui::UiBuilder::new()
                .layer_id(egui::LayerId::background())
                .max_rect(screen()),
        );
        field_stack(&mut ui, draft, None, &mut last_save);
        def_panel(&mut ui, draft);
    }
    let output = ctx.end_pass();
    let mut text = Vec::new();
    for shape in &output.shapes {
        drawn_text(&shape.shape, &mut text);
    }
    text
}

#[test]
fn both_field_panels_draw_the_key_the_stats_and_every_immunity_box() {
    let mut draft = FieldDraft::new_field();
    let text = draw(&mut draft);

    for wanted in [
        "Field",
        "Field definition",
        "Damage per tick",
        "Damage type",
        "Duration",
        "Permanent",
        "New field",
    ] {
        assert!(
            text.iter().any(|line| line == wanted),
            "the Field tab draws `{wanted}` across its right rail and its central panel: \
             {text:?}",
        );
    }
    for armor_type in gdtf_battle_sim::armor::ArmorType::ALL {
        let label = format!("{armor_type:?}");
        assert!(
            text.contains(&label),
            "the immune list is a tick box per armor type, so `{label}` must be drawn: {text:?}",
        );
    }
}

#[test]
fn the_turns_drag_is_drawn_only_while_the_duration_is_finite() {
    let mut permanent = FieldDraft::new_field();
    let text = draw(&mut permanent);
    assert_eq!(
        text.iter().filter(|line| *line == "Turns").count(),
        1,
        "a Permanent draft draws the Turns choice and no turn-count row: {text:?}",
    );

    let mut finite = FieldDraft::new_field();
    let Some(count) = std::num::NonZeroU8::new(3) else {
        unreachable!("3 is not zero");
    };
    finite.set_duration(gdtf_battle_sim::effects::fields::FieldDuration::Turns(
        gdtf_battle_sim::effects::fields::FieldTurns::new(count),
    ));
    let text = draw(&mut finite);
    assert_eq!(
        text.iter().filter(|line| *line == "Turns").count(),
        2,
        "a finite draft draws the Turns choice AND the turn-count row beneath it: {text:?}",
    );
}
