//! GTW-298 authoritative layout in one place (one change-reason: the layout tune).
use crate::states::running::game::battlescape::bottom_bar::{BOTTOM_BAR_H_VH, BOTTOM_BAR_PAD_Y_VH};

pub(super) const PANEL_W_VW: f32 = 22.0;

pub(super) const STANCE_W_VW: f32 = 8.0;

pub(super) const STANCE_LEFT_VW: f32 = PANEL_W_VW + 0.5;

pub(super) const PANEL_Z: i32 = 11;

pub(super) const PANEL_H_VH: f32 = BOTTOM_BAR_H_VH - 2.0 * BOTTOM_BAR_PAD_Y_VH;

pub(super) const TOP_CELL_PCT: f32 = 65.0;

pub(super) const BOTTOM_CELL_PCT: f32 = 35.0;

pub(super) const LEFT_COL_PCT: f32 = 75.0;

pub(super) const RIGHT_COL_PCT: f32 = 25.0;

pub(super) const INFO_ROW_H_PCT: f32 = 50.0;

pub(super) const INFO_TEXT_H_PCT: f32 = 65.0;

pub(super) const INFO_RELOAD_H_PCT: f32 = 35.0;

pub(super) const CONTENT_MIN_H_VH: f32 = 6.0;

pub(super) const GAP_VH: f32 = 0.55556;

pub(super) const GAP_VW: f32 = 0.3125;
