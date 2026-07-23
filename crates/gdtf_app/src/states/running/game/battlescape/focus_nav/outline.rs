//! GTW-782: the focus-highlight — an [`Outline`] ring on whichever battlescape panel
//! button currently holds keyboard focus, so keyboard / gamepad navigation is VISIBLE.
//!
//! Bevy's [`InputFocus`] is a purely logical resource — moving it paints nothing. Mouse
//! hover already has a visual (the theme's hover fill), but a keyboard Tab step only moves
//! the logical focus, so without this the navigation would be invisible. This ring reads
//! the focus and draws a distinct outline on the focused button, composing cleanly with
//! the existing fills (an [`Outline`] is independent of `BackgroundColor`, so it never
//! fights the theme's hover / active / disabled paints).

use bevy::{input_focus::InputFocus, prelude::*};
use gdtf_battle_input::PanelNavOrder;

/// The focus ring's fixed stroke geometry, in device px.
///
/// A named newtype over the raw px (no-bare-types), not a responsive layout share: a
/// focus ring is a crisp fixed-width hairline at any window size (the border-width idiom),
/// unlike the responsive `Vw`/`Vh` gaps the panels use for layout.
#[derive(Deref, Clone, Copy, Debug)]
struct FocusRingPx(f32);

impl FocusRingPx {
    /// The ring stroke width — a 2 px hairline.
    const WIDTH: Self = Self(2.0);
    /// The gap between the button edge and the ring — a 1 px inset so the ring reads as a
    /// halo, not a second border flush against the button.
    const OFFSET: Self = Self(1.0);
}

/// The focus ring colour — a bright amber halo that stands clear of the dark themed panels.
const FOCUS_RING_COLOR: Color = Color::srgb(1.0, 0.82, 0.2);

/// Paints an [`Outline`] ring on the panel button that currently holds keyboard focus, and
/// removes it from the rest (GTW-782).
///
/// For every [`PanelNavOrder`] button it inserts the ring on the one matching
/// [`InputFocus`] and removes any stale ring from the others — guarded on `Has<Outline>`
/// so it only touches a button the frame its focus state actually flips (no per-frame
/// component churn on steady frames). Registered in `Update` gated on the live-battle
/// witness. The ring is the ONLY focus visual; a Tab step moving [`InputFocus`] moves the
/// ring the next frame.
pub(in crate::states::running::game::battlescape) fn paint_focus_outline(
    mut commands: Commands,
    focus: Res<InputFocus>,
    buttons: Query<(Entity, Has<Outline>), With<PanelNavOrder>>,
) {
    let focused = focus.get();
    for (entity, has_ring) in &buttons {
        match (Some(entity) == focused, has_ring) {
            // Newly focused → add the ring.
            (true, false) => {
                commands.entity(entity).insert(Outline {
                    width:  Val::Px(*FocusRingPx::WIDTH),
                    offset: Val::Px(*FocusRingPx::OFFSET),
                    color:  FOCUS_RING_COLOR,
                });
            }
            // No longer focused → drop the stale ring.
            (false, true) => {
                commands.entity(entity).remove::<Outline>();
            }
            // Unchanged focus state → nothing to do.
            _ => {}
        }
    }
}
