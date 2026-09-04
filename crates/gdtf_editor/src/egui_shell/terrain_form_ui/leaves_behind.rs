//! The control that authors what a destroyed piece leaves standing in its cell.

use bevy_egui::egui;
use gdtf_battle_sim::terrain::{
    def::{LeavesBehind, TerrainDefRegistry, TerrainUuid},
    piece::TerrainGraphicKey,
};
use gdtf_content_families::sprites::{SpriteDefRegistry, SpriteName};

use crate::terrain_form::TerrainDraft;

/// Which of the three choices the draft is on, so the row can be drawn without a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LeavesChoice {
    Nothing,
    Piece,
    Sprite,
}

impl LeavesChoice {
    const ORDER: [Self; 3] = [Self::Nothing, Self::Piece, Self::Sprite];

    const fn of(leaves: &LeavesBehind) -> Self {
        match leaves {
            LeavesBehind::Nothing => Self::Nothing,
            LeavesBehind::Piece(_) => Self::Piece,
            LeavesBehind::Sprite(_) => Self::Sprite,
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::Nothing => "Nothing",
            Self::Piece => "Piece",
            Self::Sprite => "Sprite",
        }
    }
}

/// Draw the leaves-behind row: the choice, and the def or sprite it names.
pub(super) fn leaves_behind_row(
    ui: &mut egui::Ui,
    draft: &mut TerrainDraft,
    terrain: Option<&TerrainDefRegistry>,
    sprites: Option<&SpriteDefRegistry>,
) {
    ui.label("Leaves behind");
    choice_row(ui, draft);
    match draft.leaves_behind() {
        LeavesBehind::Nothing => {}
        LeavesBehind::Piece(_) => piece_combo(ui, draft, terrain),
        LeavesBehind::Sprite(_) => sprite_combo(ui, draft, sprites),
    }
}

fn choice_row(ui: &mut egui::Ui, draft: &mut TerrainDraft) {
    ui.horizontal(|ui| {
        let before = LeavesChoice::of(draft.leaves_behind());
        let mut chosen = before;
        for option in LeavesChoice::ORDER {
            ui.selectable_value(&mut chosen, option, option.label());
        }
        if chosen != before {
            draft.set_leaves_behind(match chosen {
                LeavesChoice::Nothing => LeavesBehind::Nothing,
                LeavesChoice::Piece => LeavesBehind::Piece(TerrainUuid::nil()),
                LeavesChoice::Sprite => LeavesBehind::Sprite(TerrainGraphicKey::new(String::new())),
            });
        }
    });
}

fn piece_combo(ui: &mut egui::Ui, draft: &mut TerrainDraft, terrain: Option<&TerrainDefRegistry>) {
    let Some(registry) = terrain else {
        ui.label("(loading…)");
        return;
    };
    let mut rows: Vec<(TerrainUuid, String)> = registry
        .defs()
        .map(|(key, def)| (*key, (*def.display_name).clone()))
        .collect();
    rows.sort_by(|left, right| left.1.cmp(&right.1));

    let current = match draft.leaves_behind() {
        LeavesBehind::Piece(key) => Some(*key),
        LeavesBehind::Nothing | LeavesBehind::Sprite(_) => None,
    };
    let preview = current
        .and_then(|key| rows.iter().find(|(row, _)| *row == key))
        .map_or_else(|| "(select…)".to_owned(), |(_, label)| label.clone());
    let mut picked: Option<TerrainUuid> = None;
    egui::ComboBox::from_id_salt("terrain_leaves_behind_piece_combo")
        .selected_text(preview)
        .show_ui(ui, |ui| {
            for (key, label) in &rows {
                if ui.selectable_label(current == Some(*key), label).clicked() {
                    picked = Some(*key);
                }
            }
        });
    if let Some(key) = picked {
        draft.set_leaves_behind(LeavesBehind::Piece(key));
    }
}

fn sprite_combo(ui: &mut egui::Ui, draft: &mut TerrainDraft, sprites: Option<&SpriteDefRegistry>) {
    let Some(registry) = sprites else {
        ui.label("(loading…)");
        return;
    };
    let mut names: Vec<&SpriteName> = registry.keys().collect();
    names.sort_by(|left, right| (**left).cmp(right));

    let current = match draft.leaves_behind() {
        LeavesBehind::Sprite(graphic) => Some((**graphic).clone()),
        LeavesBehind::Nothing | LeavesBehind::Piece(_) => None,
    };
    let preview = current
        .clone()
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "(select…)".to_owned());
    let mut picked: Option<String> = None;
    egui::ComboBox::from_id_salt("terrain_leaves_behind_sprite_combo")
        .selected_text(preview)
        .show_ui(ui, |ui| {
            for name in names {
                let selected = current.as_deref() == Some(&**name);
                if ui.selectable_label(selected, &**name).clicked() {
                    picked = Some((**name).clone());
                }
            }
        });
    if let Some(name) = picked {
        draft.set_leaves_behind(LeavesBehind::Sprite(TerrainGraphicKey::new(name)));
    }
}
