//! Sprite source editor for base and facing overrides.
use bevy_egui::egui;
use gdtf_battle_presenter::SheetRole;
use gdtf_content_families::sprites::{SpriteImagePath, SpritePx, SpriteRect, SpriteSource};

use super::cache::SpritePreviewCache;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum SourceKindChoice {
    File,
    Sheet,
}

impl SourceKindChoice {
    const fn of(source: &SpriteSource) -> Self {
        match source {
            SpriteSource::File(_) => Self::File,
            SpriteSource::Sheet { .. } => Self::Sheet,
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::File => "File",
            Self::Sheet => "Sheet",
        }
    }
}

pub(super) fn source_editor(
    ui: &mut egui::Ui,
    id_salt: &str,
    source: &SpriteSource,
    cache: &mut SpritePreviewCache,
) -> Option<SpriteSource> {
    let mut edited = source.clone();
    let mut changed = false;

    let current = SourceKindChoice::of(&edited);
    let mut kind = current;
    ui.horizontal(|ui| {
        ui.label("Source");
        egui::ComboBox::from_id_salt(format!("{id_salt}_source_kind"))
            .selected_text(kind.label())
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    &mut kind,
                    SourceKindChoice::File,
                    SourceKindChoice::File.label(),
                );
                ui.selectable_value(
                    &mut kind,
                    SourceKindChoice::Sheet,
                    SourceKindChoice::Sheet.label(),
                );
            });
    });
    if kind != current {
        edited = converted(edited, kind);
        changed = true;
    }

    match &mut edited {
        SpriteSource::File(path) => {
            changed |= path_field(ui, id_salt, "Image path", path, cache);
        }
        SpriteSource::Sheet { sheet, rect } => {
            changed |= path_field(ui, id_salt, "Sheet path", sheet, cache);
            changed |= rect_fields(ui, rect);
        }
    }

    changed.then_some(edited)
}

fn converted(source: SpriteSource, kind: SourceKindChoice) -> SpriteSource {
    let tile = SheetRole::Terrain.tile_px();
    match (source, kind) {
        (SpriteSource::File(path), SourceKindChoice::Sheet) => SpriteSource::Sheet {
            sheet: path,
            rect:  SpriteRect {
                x: SpritePx::new(0),
                y: SpritePx::new(0),
                w: SpritePx::new(tile),
                h: SpritePx::new(tile),
            },
        },
        (SpriteSource::Sheet { sheet, .. }, SourceKindChoice::File) => SpriteSource::File(sheet),
        (source, _) => source,
    }
}

fn path_field(
    ui: &mut egui::Ui,
    id_salt: &str,
    label: &str,
    path: &mut SpriteImagePath,
    cache: &mut SpritePreviewCache,
) -> bool {
    let mut buffer = path.as_str().to_owned();
    ui.label(label);
    let changed = ui
        .add(egui::TextEdit::singleline(&mut buffer).id_salt(format!("{id_salt}_{label}")))
        .changed();
    if changed {
        *path = SpriteImagePath::new(buffer.clone());
    }
    if buffer.trim().is_empty() {
        ui.colored_label(
            egui::Color32::YELLOW,
            "enter an assets/-relative image path",
        );
    } else if cache.path_exists(path) {
        ui.colored_label(egui::Color32::LIGHT_GREEN, "found under assets/");
    } else {
        ui.colored_label(egui::Color32::LIGHT_RED, "MISSING under assets/");
    }
    changed
}

fn rect_fields(ui: &mut egui::Ui, rect: &mut SpriteRect) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        changed |= px_drag(ui, "x", &mut rect.x);
        changed |= px_drag(ui, "y", &mut rect.y);
        changed |= px_drag(ui, "w", &mut rect.w);
        changed |= px_drag(ui, "h", &mut rect.h);
    });
    changed
}

fn px_drag(ui: &mut egui::Ui, label: &str, px: &mut SpritePx) -> bool {
    let mut value = **px;
    ui.label(label);
    let changed = ui.add(egui::DragValue::new(&mut value)).changed();
    if changed {
        *px = SpritePx::new(value);
    }
    changed
}
