//! The SPRITE form's **animation section** (GTW-664 C2) — the optional
//! `animation {fps, frames}`: an on/off checkbox, the fps drag, and the ORDERED frame
//! list with add / remove / reorder rows.

use bevy_egui::egui;
use gdtf_content_families::sprites::SpriteFps;

use super::{cache::SpritePreviewCache, source_edit::source_editor};
use crate::sprite_form::SpriteDraft;

/// Draw the ANIMATION section. Enabling seeds one base-source frame (the model's
/// idempotent [`enable_animation`](SpriteDraft::enable_animation)); each frame row
/// offers Up / Down (reorder), Remove (disabled at one frame — turn the OPTIONAL
/// animation off to author "no animation"), and the shared source editor; Add frame
/// appends. Every commit routes through a named draft mutator and the checkbox is
/// set-to-target, so the section is multipass-idempotent (bevy-traps #8).
pub(super) fn animation_section(
    ui: &mut egui::Ui,
    draft: &mut SpriteDraft,
    cache: &mut SpritePreviewCache,
) {
    ui.heading("Animation");
    let mut enabled = draft.def().animation.is_some();
    if ui.checkbox(&mut enabled, "Animated").changed() {
        if enabled {
            draft.enable_animation();
        } else {
            draft.disable_animation();
        }
    }
    // Iterate a CLONE of the current animation (immediate-mode read) while the button /
    // editor commits mutate the draft — at most one commit lands per pass.
    let Some(animation) = draft.def().animation.clone() else {
        return;
    };

    ui.horizontal(|ui| {
        ui.label("FPS");
        let mut fps = *animation.fps;
        // A playback RATE is non-negative by definition (frames PER SECOND); no tighter
        // documented bound exists, so the ceiling stays the type's (the injury
        // payload-drag precedent).
        if ui
            .add(
                egui::DragValue::new(&mut fps)
                    .range(0.0..=f32::MAX)
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
