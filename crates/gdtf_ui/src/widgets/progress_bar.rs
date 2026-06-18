//! The [`ProgressBar`] widget: a horizontal track with a smooth fill child.
//!
//! A progress bar shows a `current/max` ratio (e.g. TU or HP in the mockup's
//! status panel) as a colored FILL spanning a fraction of a darker TRACK. The two
//! colors — the **remaining** fill and the **lost** track behind it — are pure UI
//! plumbing ([`bevy::Color`]), so a caller pins HP to green/red or any other pair.
//!
//! Per [[ui-mutate-not-respawn]] the bar updates by MUTATING its existing fill
//! node's width — [`set_progress_bar`] writes [`Node::width`](bevy::ui::Node) as a
//! [`Val::Percent`](bevy::ui::Val), never despawning and respawning on a value
//! change, so the fill entity id is stable across updates.

use bevy::{
    prelude::*,
    ui::{BackgroundColor, Node, Val},
};

/// The fraction `0.0..=1.0` of a [`ProgressBar`] track that the fill spans.
///
/// A widget-level quantity (a normalized ratio), NOT a game-domain value: callers
/// pass the inner value of a domain newtype (`current/max` of `Tu`, `Hp`, integrity)
/// in. Wrapped in a named newtype so the fraction is constructed through one clamped
/// constructor — an out-of-range input can never produce a fill wider than the track
/// or a negative width.
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub struct FillFraction(f32);

impl FillFraction {
    /// Builds a [`FillFraction`], clamping the input into `0.0..=1.0`.
    ///
    /// A `current/max` ratio is clamped rather than asserted so a transient
    /// over-full or negative value (e.g. a temporary buff, or a not-yet-initialized
    /// `0/0`) is rendered as a full / empty bar instead of panicking or overflowing
    /// the track.
    #[must_use]
    pub const fn new(fraction: f32) -> Self {
        Self(fraction.clamp(0.0, 1.0))
    }

    /// Builds a [`FillFraction`] from a `current`-of-`max` ratio.
    ///
    /// Guards the `max == 0` degenerate case (an uninitialized `0/0` pool) to an
    /// EMPTY bar rather than dividing by zero, then clamps as [`FillFraction::new`].
    #[must_use]
    pub fn from_ratio(current: f32, max: f32) -> Self {
        if max <= 0.0 {
            Self(0.0)
        } else {
            Self::new(current / max)
        }
    }

    /// The fill width as a [`Val::Percent`](bevy::ui::Val) for the fill node.
    fn as_percent(self) -> Val {
        Val::Percent(self.0 * 100.0)
    }
}

/// Marker on the TRACK root of a [`ProgressBar`].
///
/// The track is the full-width box drawn in the **lost** color; the fill child
/// (marked [`ProgressBarFill`]) is layered over its left edge. The caller attaches
/// its own identity marker alongside this so it can later find the bar to update.
///
/// A unit marker — presence alone is the signal (no-bare-types rule).
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct ProgressBarTrack;

/// Marker on the FILL child of a [`ProgressBar`].
///
/// The fill is the colored bar whose [`Node::width`](bevy::ui::Node)
/// [`set_progress_bar`] mutates to the current fraction. It is a child of the
/// [`ProgressBarTrack`] root, so it inherits the track's height and is clipped to it.
///
/// A unit marker — presence alone is the signal (no-bare-types rule).
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct ProgressBarFill;

/// Spawns a [`ProgressBar`] (a [`ProgressBarTrack`] root with one
/// [`ProgressBarFill`] child) and returns the TRACK [`Entity`].
///
/// `fraction` is the initial fill `0.0..=1.0`; `remaining` is the fill color and
/// `lost` is the track color behind it (the HP table in GTW-278 passes green/red,
/// but the widget itself takes any two [`Color`]s). `marker` is any [`Bundle`] the
/// caller wants on the track root — typically its own identity marker so it can find
/// the bar to update later.
///
/// The fill's width is the only thing [`set_progress_bar`] mutates; everything else
/// (the track box, both colors, the child relationship) is spawned once and reused
/// ([[ui-mutate-not-respawn]]).
pub fn spawn_progress_bar(
    commands: &mut Commands,
    fraction: FillFraction,
    remaining: Color,
    lost: Color,
    marker: impl Bundle,
) -> Entity {
    commands
        .spawn((
            ProgressBarTrack,
            Node {
                width: Val::Percent(100.0),
                height: Val::Vh(BAR_HEIGHT_VH),
                ..default()
            },
            BackgroundColor(lost),
            marker,
        ))
        .with_children(|track| {
            track.spawn((
                ProgressBarFill,
                Node {
                    width: fraction.as_percent(),
                    height: Val::Percent(100.0),
                    ..default()
                },
                BackgroundColor(remaining),
            ));
        })
        .id()
}

/// Sets the fill of the [`ProgressBar`] rooted at `track` to `fraction` by MUTATING
/// the fill child's [`Node::width`](bevy::ui::Node) — never respawning
/// ([[ui-mutate-not-respawn]]).
///
/// Looks up the [`ProgressBarFill`] child of `track` via `children`, then writes its
/// width as a [`Val::Percent`](bevy::ui::Val). Returns `true` if a fill child was
/// found and mutated; `false` if `track` is not a progress bar (no fill child) — so
/// a caller can react to a stale id rather than silently no-op.
///
/// Param-only (a `&Children` read query + a `&mut Node` write query) — no
/// `&mut World` (bevy-traps rule 7).
pub fn set_progress_bar(
    track: Entity,
    fraction: FillFraction,
    children: &Query<&Children>,
    fills: &mut Query<&mut Node, With<ProgressBarFill>>,
) -> bool {
    let Ok(kids) = children.get(track) else {
        return false;
    };
    for &child in kids {
        if let Ok(mut node) = fills.get_mut(child) {
            node.width = fraction.as_percent();
            return true;
        }
    }
    false
}

/// The default track / fill height, in viewport-height units.
///
/// A thin horizontal bar matching the TU / HP bars in the status-panel mockup; the
/// width is always 100% of the bar's container, so only the height is fixed here.
/// Calibrated 10px / 720 * 100 at the default 1280x720 window.
const BAR_HEIGHT_VH: f32 = 1.38889;
