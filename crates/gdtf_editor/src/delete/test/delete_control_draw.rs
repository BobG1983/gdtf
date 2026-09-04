//! Each mode that offers a delete draws the button; a mode with no entry draws none.

use bevy_egui::egui;
use gdtf_assets::ContentMemberKey;

use super::offered::registry;
use crate::{
    delete::control::{DELETE_BUTTON_LABEL, delete_control},
    mode::EditorMode,
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

// Draw the control for one mode over a bare context and read back what it painted.
fn draw(mode: EditorMode) -> Vec<String> {
    let ctx = egui::Context::default();
    let input = egui::RawInput {
        screen_rect: Some(screen()),
        ..Default::default()
    };
    let registry = registry();
    let key = ContentMemberKey::new("a_record_the_form_holds".to_owned());
    ctx.begin_pass(input);
    {
        let mut ui = egui::Ui::new(
            ctx.clone(),
            egui::Id::new("delete_control_draw"),
            egui::UiBuilder::new()
                .layer_id(egui::LayerId::background())
                .max_rect(screen()),
        );
        delete_control(&mut ui, &registry, (mode, None), Some(&key));
    }
    let output = ctx.end_pass();
    let mut text = Vec::new();
    for shape in &output.shapes {
        drawn_text(&shape.shape, &mut text);
    }
    text
}

#[test]
fn every_mode_this_child_makes_deletable_draws_the_delete_button() {
    for mode in [
        EditorMode::Terrain,
        EditorMode::Theme,
        EditorMode::Gang,
        EditorMode::Weapon,
    ] {
        let text = draw(mode);
        assert!(
            text.iter().any(|line| line == DELETE_BUTTON_LABEL),
            "the {mode:?} panel draws `{DELETE_BUTTON_LABEL}` for the record its form holds: \
             {text:?}",
        );
    }
}

#[test]
fn the_sprite_panel_draws_no_delete_button() {
    let text = draw(EditorMode::Sprite);
    assert!(
        !text.iter().any(|line| line == DELETE_BUTTON_LABEL),
        "no entry names the Sprite screen, so its panel paints no delete button: {text:?}",
    );
}

#[test]
fn a_form_holding_no_key_draws_no_delete_button() {
    let ctx = egui::Context::default();
    let input = egui::RawInput {
        screen_rect: Some(screen()),
        ..Default::default()
    };
    let registry = registry();
    ctx.begin_pass(input);
    {
        let mut ui = egui::Ui::new(
            ctx.clone(),
            egui::Id::new("delete_control_no_key"),
            egui::UiBuilder::new()
                .layer_id(egui::LayerId::background())
                .max_rect(screen()),
        );
        delete_control(&mut ui, &registry, (EditorMode::Terrain, None), None);
    }
    let output = ctx.end_pass();
    let mut text = Vec::new();
    for shape in &output.shapes {
        drawn_text(&shape.shape, &mut text);
    }
    assert!(
        !text.iter().any(|line| line == DELETE_BUTTON_LABEL),
        "a form holding no key names no file to delete, so the control paints nothing: {text:?}",
    );
}
