//! [`sync_stance_buttons_active`] — drive the three Stance toggles' active (toggled-on)
//! look from the selected ganger's [`Stance`] state (GTW-267).
//!
//! Play-test bug #7: the blind `StanceCycleButton` had no current-state indicator, so the
//! player could not tell which posture the selected ganger held. The fix (decided 2026-06-17)
//! replaced it with THREE mutually-exclusive toggle buttons (Stand / Kneel / Prone); this
//! system supplies the missing VISUAL feedback — it does NOT change how the toggles set the
//! stance (each press pushes a direct
//! [`ActIntent::SetStance`](gdtf_battle_input::ActIntent::SetStance) →
//! [`SetStanceRequested`](gdtf_battle_sim::acts::SetStanceRequested) → sim-sets-[`Stance`]).
//! It mirrors the sim's [`Stance`] onto the `gdtf_ui`
//! [`ActiveButton`](gdtf_ui::ActiveButton) paint primitive, exactly as
//! [`sync_aim_button_active`](super::aim_active::sync_aim_button_active) does for Aim.
//!
//! ## The sim → button mapping
//!
//! Each battle frame it reads [`Res<SelectedShooter>`](gdtf_battle_input::SelectedShooter)
//! → the selected entity → its [`Stance`] component, then marks exactly the matching toggle
//! [`ActiveButton`](gdtf_ui::ActiveButton) and removes it from the other two:
//!
//! - [`StanceKind::Standing`](gdtf_battle_sim::StanceKind::Standing) → [`StanceStandingButton`];
//! - [`StanceKind::Crouching`](gdtf_battle_sim::StanceKind::Crouching) → [`StanceKneelingButton`];
//! - [`StanceKind::Prone`](gdtf_battle_sim::StanceKind::Prone) → [`StanceProneButton`];
//! - no selection, or a selected entity with no [`Stance`] → none active.
//!
//! The result is mutually exclusive (exactly one toggle is active at a time, or none), and
//! the active toggle reads as toggled-on (the GTW-266 sticky `ActiveButton` paint, so the
//! active fill is not overridden by hover/press feedback).
//!
//! ## Gating + ordering (`bevy-traps.md` #1 / #3 / #7)
//!
//! Registered `run_if(resource_exists::<BattleInProgress>)` by the action-bar plugin — the
//! SAME live-battle witness the other action-bar systems gate on. It uses [`Commands`] for
//! the insert/remove (NOT `&mut World`). The look reflects the CURRENT [`Stance`]; because
//! the sim applies the set in its own dispatch, the button look may lag a press by at most
//! one frame, which is acceptable for a visual indicator.

use bevy::prelude::*;
use gdtf_battle_input::SelectedShooter;
use gdtf_battle_sim::{Stance, StanceKind};
use gdtf_ui::ActiveButton;

use crate::scenes::running::game::battlescape::action_bar::components::{
    StanceKneelingButton, StanceProneButton, StanceStandingButton,
};

/// Syncs the [`ActiveButton`](gdtf_ui::ActiveButton) marker across the three Stance
/// toggles to the selected ganger's [`Stance`] (GTW-267).
///
/// Reads [`Res<SelectedShooter>`](gdtf_battle_input::SelectedShooter); if it holds an
/// entity with a [`Stance`], the toggle for that posture gets
/// [`ActiveButton`](gdtf_ui::ActiveButton) inserted and the other two have it removed (so
/// exactly one reads as toggled-on). With no selection — or a selected entity that carries
/// no [`Stance`] — none of the three is active. Insert/remove are idempotent, so this runs
/// every frame.
///
/// This is VISUAL-ONLY: the marker does not affect interaction (`gdtf_ui` paints
/// `ActiveButton` but never filters it out of the press path), so the toggles stay
/// clickable to set a different stance.
///
/// Param-only (`bevy-traps.md` #7): [`Commands`] for the insert/remove, a
/// `Res<`[`SelectedShooter`](gdtf_battle_input::SelectedShooter)`>` read, a read-only
/// `Query<&`[`Stance`]`>`, and one `Query<`[`Entity`]`, With<…>>` per stance toggle — no
/// `&mut World`.
pub(in crate::scenes::running::game::battlescape) fn sync_stance_buttons_active(
    mut commands: Commands,
    selected: Res<SelectedShooter>,
    stances: Query<&Stance>,
    stand_buttons: Query<Entity, With<StanceStandingButton>>,
    kneel_buttons: Query<Entity, With<StanceKneelingButton>>,
    prone_buttons: Query<Entity, With<StanceProneButton>>,
) {
    // The selected ganger's posture, if any (no selection / no `Stance` → none active).
    let current: Option<StanceKind> = (**selected)
        .and_then(|entity| stances.get(entity).ok())
        .map(|stance| **stance);

    set_active(
        &mut commands,
        &stand_buttons,
        current == Some(StanceKind::Standing),
    );
    set_active(
        &mut commands,
        &kneel_buttons,
        current == Some(StanceKind::Crouching),
    );
    set_active(
        &mut commands,
        &prone_buttons,
        current == Some(StanceKind::Prone),
    );
}

/// Inserts or removes [`ActiveButton`](gdtf_ui::ActiveButton) on every button matched by
/// `buttons`, by whether that toggle is the `active` one this frame.
///
/// Factored out so each of the three stance toggles applies the same idempotent
/// insert-or-remove (the `aim_active.rs` per-button loop, generalized to three toggles).
fn set_active<M: Component>(
    commands: &mut Commands,
    buttons: &Query<Entity, With<M>>,
    active: bool,
) {
    for button in buttons {
        let mut entity = commands.entity(button);
        if active {
            entity.insert(ActiveButton);
        } else {
            entity.remove::<ActiveButton>();
        }
    }
}
