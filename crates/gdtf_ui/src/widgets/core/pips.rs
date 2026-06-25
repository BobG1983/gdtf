//! The [`Pips`] widget: a row of N small circle nodes, M-of-N filled.
//!
//! Pips show a small discrete count (the Wounds row in the status-panel mockup) as
//! a row of rounded nodes: the first `M` carry the **remaining** (filled) color, the
//! rest the **lost** (empty) color. The two colors are pure UI plumbing
//! ([`bevy::Color`]) — Wounds default yellow filled / panel-bg empty, but the widget
//! takes any pair.
//!
//! Per [[ui-mutate-not-respawn]] an update MUTATES each existing pip's
//! [`BackgroundColor`](bevy::ui::BackgroundColor) by the M-of-N split —
//! [`set_pips`] never despawns or respawns a pip on a value change, so the pip entity
//! ids are stable across updates.

use bevy::{
    prelude::*,
    scene::{CommandsSceneExt, bsn, template_value},
    ui::{BackgroundColor, BorderRadius, Node, Val},
};

/// How many pips of a [`Pips`] row are FILLED (carry the remaining color).
///
/// A widget-level count, NOT a game-domain value: a caller passes the inner value of
/// a domain newtype (e.g. wounds taken) in. Wrapped so the count flows through one
/// type; the M-of-N split saturates against the spawned pip total, so an out-of-range
/// `filled` simply fills all (or none of) the pips.
#[derive(Deref, Clone, Copy, PartialEq, Eq, Debug)]
pub struct FilledPips(usize);

impl FilledPips {
    /// Wraps a filled-pip count.
    #[must_use]
    pub const fn new(filled: usize) -> Self {
        Self(filled)
    }
}

/// Marker on the ROW root of a [`Pips`] widget.
///
/// The row is a horizontal flex container; each pip is a child marked [`Pip`]. The
/// caller attaches its own identity marker alongside this so it can later find the
/// row to update.
///
/// A unit marker — presence alone is the signal (no-bare-types rule).
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct PipsRow;

/// Marker on one circular pip node (a child of a [`PipsRow`]).
///
/// [`set_pips`] mutates each pip's [`BackgroundColor`](bevy::ui::BackgroundColor) to
/// the remaining or lost color by its index within the row.
///
/// A unit marker — presence alone is the signal (no-bare-types rule).
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Pip;

/// Spawns a [`Pips`] row of `total` circular pips (the first `filled` in the
/// `remaining` color, the rest in `lost`) and returns the ROW [`Entity`].
///
/// `remaining` is the filled-pip color (Wounds default yellow) and `lost` the
/// empty-pip color (Wounds default panel-bg), both override-able. `marker` is any
/// [`Bundle`] the caller wants on the row root — typically its own identity marker so
/// it can find the row to update later.
///
/// The pip entities are spawned ONCE here; [`set_pips`] only re-colors them
/// ([[ui-mutate-not-respawn]]). The children are spawned left-to-right, so child
/// order matches pip index.
pub fn spawn_pips(
    commands: &mut Commands,
    total: usize,
    filled: FilledPips,
    remaining: Color,
    lost: Color,
    marker: impl Bundle,
) -> Entity {
    // The row + per-pip layout nodes are runtime values (no `bsn!` value-grammar form),
    // bridged with `template_value`. The pip COUNT is a runtime `total`, so the N pip
    // child scenes cannot be a compile-time `Children [ .. ]` list; they are built as a
    // runtime `Vec<Scene>` (a `SceneList`) and spliced into the relationship list via the
    // `{ expr }` scene-list-include grammar. The caller's generic `marker` is `.insert`ed
    // after (GTW-322).
    let row_node = Node {
        column_gap: Val::Vw(PIP_GAP_VW),
        ..default()
    };
    let pip_node = Node {
        width: Val::Vw(PIP_DIAMETER_VW),
        height: Val::Vw(PIP_DIAMETER_VW),
        // A fully rounded square is a circle.
        border_radius: BorderRadius::all(Val::Percent(50.0)),
        ..default()
    };
    let pips: Vec<_> = (0..total)
        .map(|index| {
            let color = if index < *filled { remaining } else { lost };
            let node = pip_node.clone();
            bsn! {
                Pip
                BackgroundColor(color)
                template_value(node)
            }
        })
        .collect();
    commands
        .spawn_scene((
            bsn! {
                PipsRow
                Children [ { pips } ]
            },
            template_value(row_node),
        ))
        .insert(marker)
        .id()
}

/// Re-colors the pips of the [`Pips`] row rooted at `row` to the new M-of-N split by
/// MUTATING each existing pip's [`BackgroundColor`](bevy::ui::BackgroundColor) —
/// never respawning ([[ui-mutate-not-respawn]]).
///
/// Walks the row's [`Children`] in order: pips before index `filled` get `remaining`,
/// the rest `lost`. Returns the number of pips re-colored (`0` if `row` is not a pips
/// widget / has no pip children), so a caller can detect a stale id.
///
/// Param-only (a `&Children` read query + a `&mut BackgroundColor` write query) — no
/// `&mut World` (bevy-traps rule 7).
pub fn set_pips(
    row: Entity,
    filled: FilledPips,
    remaining: Color,
    lost: Color,
    children: &Query<&Children>,
    pips: &mut Query<&mut BackgroundColor, With<Pip>>,
) -> usize {
    let Ok(kids) = children.get(row) else {
        return 0;
    };
    let mut recolored = 0usize;
    for (index, child) in kids.iter().enumerate() {
        if let Ok(mut background) = pips.get_mut(child) {
            background.0 = if index < *filled { remaining } else { lost };
            recolored += 1;
        }
    }
    recolored
}

/// The diameter of one pip, in viewport-width units (applied to both width +
/// height; the sub-pixel 16:9 skew on a ~12px pip is invisible). Calibrated
/// 12px / 1280 * 100 at the default 1280x720 window.
const PIP_DIAMETER_VW: f32 = 0.9375;

/// The horizontal gap between adjacent pips (the row `column_gap`), in
/// viewport-width units. Calibrated 4px / 1280 * 100.
const PIP_GAP_VW: f32 = 0.3125;
