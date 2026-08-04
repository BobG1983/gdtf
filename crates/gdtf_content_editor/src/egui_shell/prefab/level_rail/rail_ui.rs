use bevy_egui::egui;
use gdtf_battle_sim::{level::GridSize, metric::Level, terrain::def::TerrainDefRegistry};

use super::{
    cache::RailUiState,
    occupancy::{storey_image, storey_key},
};
use crate::{
    canvas::{CurrentEditLevel, LevelStep},
    editor_map::EditorMap,
    session::MapEditorSession,
};

const THUMB_WIDTH: f32 = 72.0;

pub(crate) struct RailCtx<'a> {
    pub(crate) map:        &'a EditorMap,
    pub(crate) session:    &'a MapEditorSession,
    pub(crate) edit_level: &'a mut CurrentEditLevel,
    pub(crate) registry:   Option<&'a TerrainDefRegistry>,
    pub(crate) state:      &'a mut RailUiState,
}

pub(super) fn rail_rows_top_first(levels: u8) -> impl DoubleEndedIterator<Item = Level> {
    (0..levels).rev().map(Level::new)
}

pub(crate) fn level_rail(ui: &mut egui::Ui, rail: &mut RailCtx<'_>) {
    let size = rail.session.grid_size();
    let current = rail.edit_level.level();
    rail.state.thumbs.prune(size);

    let title = format!("Storeys — editing L{}", one_based(current));
    egui::CollapsingHeader::new(title)
        .default_open(true)
        .show(ui, |ui| rail_rows(ui, rail, size, current));
}

fn rail_rows(ui: &mut egui::Ui, rail: &mut RailCtx<'_>, size: GridSize, current: Level) {
    let mut rects: Vec<(egui::Rect, Level)> = Vec::new();
    let mut jump: Option<Level> = None;
    let mut drag_pos: Option<egui::Pos2> = None;
    let mut hovered = false;

    for storey in rail_rows_top_first(*size.levels()) {
        let response = rail_row(ui, rail, size, storey, storey == current);
        if response.clicked() {
            jump = Some(storey);
        }
        if response.dragged_by(egui::PointerButton::Primary)
            && let Some(pos) = response.interact_pointer_pos()
        {
            drag_pos = Some(pos);
        }
        hovered |= response.hovered();
        rects.push((response.rect, storey));
    }

    if let Some(pos) = drag_pos
        && let Some((_, storey)) = rects.iter().find(|(rect, _)| rect.contains(pos))
    {
        jump = Some(*storey);
    }

    if let Some(target) = jump {
        let next = CurrentEditLevel::jumped(target, size);
        if next != *rail.edit_level {
            *rail.edit_level = next;
        }
    }

    if hovered {
        let points = ui.ctx().input(|i| i.smooth_scroll_delta.y);
        let steps = rail.state.scrub.fold(points);
        if steps != 0 {
            let step = if steps > 0 {
                LevelStep::up()
            } else {
                LevelStep::down()
            };
            let mut next = *rail.edit_level;
            for _ in 0..steps.unsigned_abs() {
                next = next.stepped(step, size);
            }
            if next != *rail.edit_level {
                *rail.edit_level = next;
            }
        }
    }
}

fn rail_row(
    ui: &mut egui::Ui,
    rail: &mut RailCtx<'_>,
    size: GridSize,
    storey: Level,
    active: bool,
) -> egui::Response {
    let map = rail.map;
    let registry = rail.registry;
    let state = &mut *rail.state;
    let (signature, count) = storey_key(map, registry, size, storey);
    let (fill, stroke) = if active {
        (
            ui.visuals().selection.bg_fill,
            ui.visuals().selection.stroke,
        )
    } else {
        (ui.visuals().faint_bg_color, egui::Stroke::NONE)
    };
    let inner = ui.push_id(*storey, |ui| {
        egui::Frame::new()
            .fill(fill)
            .stroke(stroke)
            .inner_margin(egui::Margin::same(3))
            .corner_radius(3)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let thumb = state
                        .thumbs
                        .refresh(ui.ctx(), storey, signature, count, || {
                            storey_image(map, registry, size, storey)
                        });
                    if let Some(thumb) = thumb {
                        let aspect = f32::from(*size.height()) / f32::from(*size.width()).max(1.0);
                        let display = egui::Vec2::new(THUMB_WIDTH, THUMB_WIDTH * aspect);
                        ui.add(egui::Image::new(egui::load::SizedTexture::new(
                            thumb.texture.id(),
                            display,
                        )));
                    }
                    ui.vertical(|ui| {
                        let n = one_based(storey);
                        if active {
                            ui.strong(format!("L{n} — editing"));
                        } else {
                            ui.label(format!("L{n}"));
                        }
                        ui.label(format!("{} painted", *count));
                    });
                });
            })
    });
    inner.inner.response.interact(egui::Sense::click_and_drag())
}

fn one_based(storey: Level) -> u16 {
    u16::from(*storey).saturating_add(1)
}
