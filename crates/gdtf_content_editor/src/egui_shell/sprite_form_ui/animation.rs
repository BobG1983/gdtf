use bevy_egui::egui;
use gdtf_content_families::sprites::SpriteFps;

use super::{cache::SpritePreviewCache, source_edit::source_editor};
use crate::sprite_form::SpriteDraft;

pub(super) fn animation_section(
    ui: &mut egui::Ui,
    draft: &mut SpriteDraft,
    cache: &mut SpritePreviewCache,
) {
    ui.heading("Animation");
    let mut enabled = draft.is_animated();
    if ui.checkbox(&mut enabled, "Animated").changed() {
        if enabled {
            draft.enable_animation();
        } else {
            draft.disable_animation();
        }
    }
    let Some(animation) = draft.def().animation.clone() else {
        return;
    };

    ui.horizontal(|ui| {
        ui.label("FPS");
        let mut fps = *animation.fps;
        if ui
            .add(
                egui::DragValue::new(&mut fps)
                    .range(SpriteDraft::FPS_RANGE)
                    .speed(0.1),
            )
            .changed()
        {
            draft.set_fps(SpriteFps::new(fps));
        }
    });

    for (index, frame) in animation.frames.iter().enumerate() {
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label(format!("Frame {index}"));
                if ui.add_enabled(index > 0, egui::Button::new("Up")).clicked() {
                    draft.move_frame_up(index);
                }
                if ui
                    .add_enabled(
                        index + 1 < animation.frames.len(),
                        egui::Button::new("Down"),
                    )
                    .clicked()
                {
                    draft.move_frame_down(index);
                }
                if ui
                    .add_enabled(animation.frames.len() > 1, egui::Button::new("Remove"))
                    .clicked()
                {
                    draft.remove_frame(index);
                }
            });
            if let Some(edited) = source_editor(ui, &format!("frame_{index}"), frame, cache) {
                draft.set_frame(index, edited);
            }
        });
    }
    if ui.button("Add frame").clicked() {
        draft.add_frame();
    }
}
