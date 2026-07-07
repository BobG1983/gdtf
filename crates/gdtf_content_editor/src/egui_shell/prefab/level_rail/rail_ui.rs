//! The rail's **egui draw + input** (GTW-595 C1) — the vertical per-storey strip in the
//! PREFAB right panel that replaced the blind `Level n / m` paging: one row per storey
//! (TOP storey first), each with its occupancy thumbnail + painted count, the ACTIVE row
//! strongly highlighted (THE "editing Ln" affordance — it superseded the GTW-594
//! viewport badge), click-to-jump, drag-to-scrub across rows, and wheel-scrub.
//!
//! Compact/collapsible (C2): the strip lives in a [`egui::CollapsingHeader`] whose title
//! carries the `editing Ln` readout, so the level indicator survives a collapse. Every
//! mutation is set-to-target over the EXISTING clamped [`CurrentEditLevel`] model —
//! idempotent under the egui multipass re-run (bevy-traps #8 fact (b)); the wheel fold
//! is pass-safe because egui takes the frame input after the first pass.

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

/// The on-screen width of a row's occupancy thumbnail, in egui points — sized for the
/// crowded right panel (C2); the height follows the grid's aspect. NEAREST-filtered, so
/// the per-cell texels stay crisp. A framework layout const, not a domain value.
const THUMB_WIDTH: f32 = 72.0;

/// The borrows the level rail draws + scrubs from — bundled so the controls panel
/// threads one struct (the `ViewportCtx` pattern). Not a domain value: a plumbing
/// aggregate of borrows the shell already holds.
pub(crate) struct RailCtx<'a> {
    /// The sparse paintable map the occupancy sweep reads.
    pub(crate) map:        &'a EditorMap,
    /// The authoring session (grid size — row count + thumbnail extent).
    pub(crate) session:    &'a MapEditorSession,
    /// The EXISTING clamped storey selector the rows jump/scrub (no new level state).
    pub(crate) edit_level: &'a mut CurrentEditLevel,
    /// The terrain registry (the uuid → graphic-role hue resolve); [`None`] until the
    /// Load pass resolves — painted cells then draw the fallback hue.
    pub(crate) registry:   Option<&'a TerrainDefRegistry>,
    /// The rail's retained view state (thumbnail cache + scrub remainder).
    pub(crate) state:      &'a mut RailUiState,
}

/// The rail's storeys in ROW ORDER — TOP storey first (C1: the rail reads like the
/// building it scrubs, ceiling at the top). The click→level mapping's pure half.
pub(super) fn rail_rows_top_first(levels: u8) -> impl DoubleEndedIterator<Item = Level> {
    (0..levels).rev().map(Level::new)
}

/// Draw the per-storey LEVEL RAIL into the PREFAB right panel (GTW-595 C1) and fold its
/// input — row clicks, drag-scrub, and wheel-scrub — into the clamped
/// [`CurrentEditLevel`].
pub(crate) fn level_rail(ui: &mut egui::Ui, rail: &mut RailCtx<'_>) {
    let size = rail.session.grid_size();
    let current = rail.edit_level.level();
    rail.state.thumbs.prune(size);

    // The collapsible header IS the surviving level readout (1-based, like the old
    // `Level n / m` label) — collapsing the rail never hides which storey is edited.
    let title = format!("Storeys — editing L{}", one_based(current));
    egui::CollapsingHeader::new(title)
        .default_open(true)
        .show(ui, |ui| rail_rows(ui, rail, size, current));
}

/// Draw the rail's rows (top storey first) and resolve this frame's row interactions
/// into the selector — split from [`level_rail`] so the closure body stays readable.
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

    // DRAG-SCRUB across rows: egui latches a drag to its origin row, so map the
    // pointer's CURRENT position back to the row under it — set-to-target, idempotent
    // under a multipass re-run.
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

    // WHEEL-SCRUB while hovering the rail: fold the scroll points into whole storey
    // steps through the kept clamp (wheel-up = up a storey). Pass-safe: egui takes the
    // frame input after the first pass, so a re-run folds zero points.
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

/// Draw ONE storey row — thumbnail + `L{n}` + painted count, framed, the ACTIVE row
/// strongly highlighted (selection fill + stroke + a `— editing` suffix: the rail's
/// level affordance) — and return its click/drag response.
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
                    // The change-keyed thumbnail (C2): rebuilt only when this storey's
                    // signature moved; otherwise the retained texture redraws as-is.
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

/// A storey index shown 1-based, matching the old `Level n / m` readout the rail
/// replaced (the stored [`Level`] stays 0-based).
fn one_based(storey: Level) -> u16 {
    u16::from(*storey).saturating_add(1)
}
