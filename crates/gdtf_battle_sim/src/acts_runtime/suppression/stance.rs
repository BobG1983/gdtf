//! The **auto-stance drop** [`suppression_auto_stance`] + the pure band→stance mapping
//! [`stance_for_cover_band`] (GTW-526 C5, child of GTW-41).
//!
//! When a ganger is FRESHLY suppressed ([`Added<Suppressed>`](bevy::prelude::Added)) it
//! auto-drops behind the cover it is behind RELATIVE to the suppressor: the cover one
//! cell TOWARD the suppressor (`Direction::from_cells(unit, suppressor)` →
//! [`Direction::cell_step`](crate::ganger::Direction::cell_step)). Its
//! [`HeightBand`](crate::cover::HeightBand) maps to a [`StanceKind`] via
//! [`stance_for_cover_band`], and the ganger's [`Stance`] is written DIRECTLY — no
//! `set_stance`, no TU charged (a suppressed unit ducks reflexively, it does not spend an
//! action). No adjacent cover ⇒ a no-op (stance unchanged; the other suppression effects
//! still apply).

use bevy::prelude::{Added, Query, Res};

use crate::{
    cover::{CoverLedger, HeightBand},
    ganger::{Direction, Position, Stance, StanceKind, Suppressed},
    metric::{Cell, CellLevel, Level},
};

/// Map a cover [`HeightBand`] to the [`StanceKind`] a suppressed unit drops to behind it
/// (GTW-526 C5) — the pure, testable band→posture rule.
///
/// - `Low` → [`Prone`](StanceKind::Prone): the lowest cover only protects a flat body,
///   so the unit goes prone to fit fully behind it.
/// - `Mid` → [`Crouching`](StanceKind::Crouching): mid cover protects a kneeling
///   silhouette (the doc's "kneel").
/// - `High` → [`Crouching`](StanceKind::Crouching): tall cover already protects a
///   standing body, so the reflexive duck settles at a crouch (the steadier of the two
///   non-standing postures) rather than dropping all the way prone. Stairs classify as
///   [`High`](HeightBand::High) in the cover model, so they map the same way.
///
/// Pure, total, no `World` access — unit-tested directly for every band.
#[must_use]
pub const fn stance_for_cover_band(band: HeightBand) -> StanceKind {
    match band {
        HeightBand::Low => StanceKind::Prone,
        HeightBand::Mid | HeightBand::High => StanceKind::Crouching,
    }
}

/// The ground cell `(x, y)` of a [`Position`] (the z storey dropped).
fn pos_cell(position: &Position) -> Cell {
    let key = ***position;
    Cell::new(key.x, key.y)
}

/// The storey [`Level`] of a [`Position`].
fn pos_level(position: &Position) -> Level {
    let key = ***position;
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "z is a storey index in 0..MAX_LEVELS (8) by construction, so the i32 -> \
                  u8 narrowing cannot truncate or sign-flip (the trigger.rs row_level precedent)"
    )]
    let storey = key.z as u8;
    Level::new(storey)
}

/// The `(cell, level)` one Moore-8 step from `unit` TOWARD `suppressor` (same storey), or
/// `None` when the unit and the suppressor share the same ground cell (no direction —
/// `Direction::from_cells` returns `None`).
///
/// This is the cover cell a suppressed unit ducks behind: the cell BETWEEN it and the
/// threat. The step stays on the unit's own storey (the cover it ducks behind is at its
/// level, not the suppressor's).
fn cover_cell_toward(unit: &Position, suppressor: &CellLevel) -> Option<CellLevel> {
    let unit_cell = pos_cell(unit);
    let suppressor_cell = Cell::new(suppressor.x, suppressor.y);
    let dir = Direction::from_cells(unit_cell, suppressor_cell)?;
    let step = dir.cell_step();
    let toward = Cell::new(unit_cell.x + step.x, unit_cell.y + step.y);
    Some(CellLevel::new(toward, pos_level(unit)))
}

/// The **auto-stance drop** — on a FRESH [`Suppressed`], duck the ganger behind its
/// nearest cover, mapping the cover's band to a stance and writing it DIRECTLY (no TU)
/// (GTW-526 C5).
///
/// Runs on the [`Added<Suppressed>`](bevy::prelude::Added) archetype filter — exactly the
/// gangers that gained the [`Suppressed`] component this frame (the producer's fresh
/// application; an idempotent REFRESH re-inserts the same component type, which does NOT
/// re-trip `Added`, so a refresh never re-drops the stance). For each, it:
///
/// 1. reads the [`SuppressorCell`](crate::ganger::SuppressorCell) anchor and finds the
///    cover cell one step TOWARD the suppressor (`cover_cell_toward`);
/// 2. [`peek`](crate::cover::CoverLedger::peek)s the [`CoverLedger`] at that cell — a
///    read-only peek (no lazy seed), so a cell with no registered cover is `None`;
/// 3. if there IS cover, maps its [`HeightBand`](crate::cover::HeightBand) to a
///    [`StanceKind`] ([`stance_for_cover_band`]) and writes `*stance = Stance::new(kind)`
///    DIRECTLY — NOT the TU-charging [`set_stance`](crate::posture::set_stance) verb (a
///    reflexive duck, no action spent).
///
/// No adjacent cover (co-located suppressor, or no ledger entry at the toward-cell) ⇒ a
/// no-op: the stance is left unchanged (the other suppression effects — the reaction-fire
/// lockout — still apply). `set_if_neq`-style: the write only fires when a cover cell
/// resolves, so a no-cover suppression never touches [`Stance`] (no spurious change
/// detection).
///
/// Param-only (`Query` / `Res`) — no `&mut World` (`bevy-traps.md` #7). Reads the
/// [`CoverLedger`] resource directly (battle-lifetime; this system runs in the
/// [`BattleInProgress`](crate::battle::BattleInProgress)-gated band, so the resource is
/// present — `bevy-traps.md` #1).
pub fn suppression_auto_stance(
    mut newly: Query<(&Position, &Suppressed, &mut Stance), Added<Suppressed>>,
    cover: Res<CoverLedger>,
) {
    for (position, suppressed, mut stance) in &mut newly {
        let Some(cover_cell) = cover_cell_toward(position, &suppressed.from) else {
            continue;
        };
        let Some(entry) = cover.peek(&cover_cell) else {
            continue;
        };
        let kind = stance_for_cover_band(entry.height_band);
        *stance = Stance::new(kind);
    }
}
