use std::path::PathBuf;

use bevy_egui::egui;

use crate::{
    egui_shell::injury_form_ui::panels::{InjuryPanelsCtx, injury_panels},
    injury_form::{InjuryDraft, WeightingDraft},
    mode::InjurySubTab,
    save_record::LastSaveRecord,
};

// The pane the test hands egui, big enough that neither panel is clipped away.
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

// Draw one sub-tab over a bare context and read back what it painted.
fn draw(sub_tab: InjurySubTab) -> Vec<String> {
    let ctx = egui::Context::default();
    let mut sub_tab = sub_tab;
    let mut draft = InjuryDraft::default();
    let mut weighting = WeightingDraft::default();
    let mut last_save = LastSaveRecord::default();
    // The root `MapEditorPlugin` seeds, so the Tables sub-tab draws the save button it ships with.
    let root: PathBuf = std::env::temp_dir();
    let mut text = Vec::new();
    // Two passes: a scroll area paints its whole content only once it has measured it.
    for _ in 0..2 {
        let input = egui::RawInput {
            screen_rect: Some(screen()),
            ..Default::default()
        };
        ctx.begin_pass(input);
        {
            let mut ui = egui::Ui::new(
                ctx.clone(),
                egui::Id::new("injury_sub_tab_draw"),
                egui::UiBuilder::new()
                    .layer_id(egui::LayerId::background())
                    .max_rect(screen()),
            );
            let mut panels = InjuryPanelsCtx {
                sub_tab:   &mut sub_tab,
                draft:     Some(&mut draft),
                weighting: Some(&mut weighting),
                injuries:  None,
                tables:    None,
                last_save: &mut last_save,
                root:      Some(&root),
            };
            injury_panels(&mut ui, &mut panels);
        }
        let output = ctx.end_pass();
        text.clear();
        for shape in &output.shapes {
            drawn_text(&shape.shape, &mut text);
        }
    }
    text
}

#[test]
fn the_def_sub_tab_draws_the_def_form_and_not_the_weighting_table() {
    let text = draw(InjurySubTab::Def);
    assert!(
        text.iter().any(|line| line == "Injury definition"),
        "the Def sub-tab draws the def form's own heading: {text:?}",
    );
    assert!(
        !text.iter().any(|line| line == "Weighting"),
        "the Def sub-tab draws only the def form, so the weighting heading must not be painted \
         in the same pass: {text:?}",
    );
}

#[test]
fn the_tables_sub_tab_draws_the_weighting_table_and_not_the_def_form() {
    let text = draw(InjurySubTab::Tables);
    assert!(
        text.iter().any(|line| line == "Weighting"),
        "the Tables sub-tab draws the weighting table's own heading: {text:?}",
    );
    assert!(
        !text.iter().any(|line| line == "Injury definition"),
        "the Tables sub-tab draws only the weighting table, so the def heading must not be \
         painted in the same pass: {text:?}",
    );
}

#[test]
fn both_sub_tabs_draw_the_sub_tab_row() {
    for sub_tab in InjurySubTab::TAB_ORDER {
        let text = draw(sub_tab);
        for label in InjurySubTab::TAB_ORDER.map(InjurySubTab::tab_label) {
            assert!(
                text.iter().any(|line| line == label),
                "the sub-tab row is how an author reaches the other sub-tab, so `{label}` must \
                 be drawn on {sub_tab:?}: {text:?}",
            );
        }
    }
}

#[test]
#[cfg(feature = "mcp")]
fn the_tables_sub_tab_draws_the_save_button_the_one_writer_sits_behind() {
    let text = draw(InjurySubTab::Tables);
    assert!(
        text.iter().any(|line| line == "Save weighting"),
        "the Tables sub-tab draws its save button whenever the editor holds a root to write \
         under, which it always does: {text:?}",
    );
}
