use bevy::asset::uuid::Uuid;
use bevy_egui::egui;
use gdtf_battle_sim::{
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverHp, HeightBand},
    level::{GridSize, ThemeUuid},
    terrain::def::{
        LeavesBehind, TerrainDef, TerrainDefRegistry, TerrainDisplayName, TerrainPresenterKind,
        TerrainSimKind, TerrainUuid, TerrainViews,
    },
};

use super::super::fields::{TerrainSaveContext, field_stack};
use crate::{
    save_record::LastSaveRecord,
    session::MapEditorSession,
    terrain_form::{TerrainDraft, load_candidates},
};

// The display name both fixture defs hold, so their picker labels need the key to differ.
const SHARED_NAME: &str = "Drum";

// The pane the test hands egui, wide enough that the combo's preview is not truncated.
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

fn cover_def(low: u128) -> TerrainDef {
    TerrainDef {
        key:            TerrainUuid::new(Uuid::from_u128(low)),
        display_name:   TerrainDisplayName::new(SHARED_NAME.to_owned()),
        sim_kind:       TerrainSimKind::Cover {
            hp:               CoverHp::new(20),
            armor_protection: ArmorProtection::new(3),
            armor_hardness:   ArmorHardness::new(1),
            height_band:      HeightBand::Low,
        },
        presenter_kind: TerrainPresenterKind::Cover,
        views:          TerrainViews::new(Vec::new()),
        tags:           Vec::new(),
        on_death:       Vec::new(),
        blocks_pathing: None,
        blocks_los:     None,
        leaves_behind:  LeavesBehind::Nothing,
    }
}

// Two defs under one display name, so a label wired to the name alone reads the same twice.
fn registry() -> TerrainDefRegistry {
    TerrainDefRegistry::new([0x0184_0bcd_9001, 0x0184_0bcd_9002].into_iter().map(|low| {
        let def = cover_def(low);
        (def.key, def)
    }))
}

// Draw the Terrain field stack over a bare context and read back what it painted.
fn draw(draft: &mut TerrainDraft, terrain: &TerrainDefRegistry) -> Vec<String> {
    let ctx = egui::Context::default();
    let input = egui::RawInput {
        screen_rect: Some(screen()),
        ..Default::default()
    };
    let session = MapEditorSession::new(ThemeUuid::nil(), None, GridSize::default());
    let mut last_save = LastSaveRecord::default();
    ctx.begin_pass(input);
    {
        let mut ui = egui::Ui::new(
            ctx.clone(),
            egui::Id::new("terrain_load_combo_draw"),
            egui::UiBuilder::new()
                .layer_id(egui::LayerId::background())
                .max_rect(screen()),
        );
        let save = TerrainSaveContext {
            session:   &session,
            themes:    None,
            last_save: &mut last_save,
        };
        field_stack(&mut ui, draft, save, None, Some(terrain), None);
    }
    let output = ctx.end_pass();
    let mut text = Vec::new();
    for shape in &output.shapes {
        drawn_text(&shape.shape, &mut text);
    }
    text
}

#[test]
fn the_load_combo_is_drawn_above_the_display_name_box() {
    let terrain = registry();
    let mut draft = TerrainDraft::default();
    let text = draw(&mut draft, &terrain);

    let combo = text.iter().position(|line| line == "Load terrain");
    let name = text.iter().position(|line| line == "Display name");
    let (Some(combo), Some(name)) = (combo, name) else {
        unreachable!("the Terrain form draws both `Load terrain` and `Display name`: {text:?}");
    };
    assert!(
        combo < name,
        "the load picker opens the form, so it is painted before the name box: {text:?}",
    );
}

#[test]
fn the_preview_names_the_row_of_the_def_the_draft_was_loaded_from() {
    let terrain = registry();
    let candidates = load_candidates(&terrain);
    let Some((key, label)) = candidates.first().cloned() else {
        unreachable!("a two-def registry answers two picker rows: {candidates:?}");
    };

    let mut draft = TerrainDraft::default();
    draft.set_uuid(Some(key));
    let text = draw(&mut draft, &terrain);

    assert!(
        text.contains(&label),
        "the preview is the picker row of the def whose key the draft carries. Both fixture defs \
         are named `{SHARED_NAME}`, so a preview wired to the display name cannot say which one \
         is open: {text:?}",
    );
}

#[test]
fn a_draft_with_no_key_previews_the_empty_choice() {
    let terrain = registry();
    let mut draft = TerrainDraft::default();
    assert_eq!(draft.uuid(), None, "a fresh draft carries no key");
    let text = draw(&mut draft, &terrain);

    assert!(
        text.iter().any(|line| line == "(select…)"),
        "a draft that was never loaded matches no candidate, so the combo says nothing is \
         picked: {text:?}",
    );
}
