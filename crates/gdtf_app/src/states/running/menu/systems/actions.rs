//! Maps an enabled menu button's *activation* to a [`RunningState`] transition
//! (GTW-122).
//!
//! This is the pure interaction→state layer: it touches no colors or fonts. Each
//! enabled menu button, when activated, requests the matching
//! [`NextState<RunningState>`] — Battlescape → [`RunningState::Game`], Options →
//! [`RunningState::Options`], Quit → [`RunningState::Quit`]. The disabled
//! `HiveScape` placeholder produces **no** transition under any input: every
//! query here is filtered `Without<DisabledButton>`, so it is invisible to the
//! action layer (whatever the campaign layer eventually does is out of scope).
//!
//! ## Two activation paths, one mapping
//!
//! A button can be activated two ways, and both drive the *same* per-marker
//! mapping:
//!
//! 1. **Mouse** — a [`bevy::ui::Interaction`] that became
//!    [`Pressed`](bevy::ui::Interaction::Pressed) this frame
//!    ([`mouse_button_actions`]). The query is filtered `Changed<Interaction>` so
//!    only the frame the press *lands* fires, never a held press re-firing every
//!    frame. (Real pointer production of `Pressed` is GTW-141; this ticket only
//!    consumes it — tests inject it.)
//! 2. **Keyboard / gamepad** — a [`FocusActivated`] message raised by the GTW-119
//!    focus-nav bridge on `Enter` / gamepad South while the button holds focus
//!    ([`focus_activated_actions`]). The activated [`Entity`] is mapped back to
//!    its enabled marker and the same transition is requested.
//!
//! ## Ordering (bevy-traps rule 3)
//!
//! [`focus_activated_actions`] is ordered `.after(FocusNavSystems::Bridge)` (the
//! set that *writes* [`FocusActivated`]) in [`MenuScenePlugin`](super::super::plugin),
//! so an activation raised this frame is consumed the same frame rather than one
//! frame late. Both systems run only under
//! `run_if(in_state(RunningState::Menu))`.

use bevy::{prelude::*, ui::Interaction};
use gdtf_ui::{DisabledButton, focus_nav::FocusActivated};

use crate::states::{
    RunningState,
    running::menu::components::{BattlescapeButton, OptionsButton, QuitButton},
};

/// The [`RunningState`] a menu button activation transitions to.
///
/// A named newtype over the target state rather than a bare [`RunningState`] in
/// the system internals (no-bare-types rule): it names the value's role — "the
/// screen this menu action opens" — and is the single place the marker→state
/// mapping is expressed, shared by both activation paths.
#[derive(Deref, Clone, Copy, PartialEq, Eq, Debug)]
struct MenuActionTarget(RunningState);

impl MenuActionTarget {
    /// Battlescape launches a mission → [`RunningState::Game`].
    const BATTLESCAPE: Self = Self(RunningState::Game);
    /// Options opens the options screen → [`RunningState::Options`].
    const OPTIONS: Self = Self(RunningState::Options);
    /// Quit exits the game → [`RunningState::Quit`].
    const QUIT: Self = Self(RunningState::Quit);
    /// DEV-ONLY: the Gang Editor opens the in-app editor → [`RunningState::DebugEditor`]
    /// (GTW-420). `cfg(debug_assertions)`-gated so this mapping never compiles into a release
    /// binary.
    #[cfg(debug_assertions)]
    const GANG_EDITOR: Self = Self(RunningState::DebugEditor);
}

/// Query filter selecting the enabled button carrying marker `M` whose
/// [`bevy::ui::Interaction`] became a press this frame.
///
/// Factored into a named alias both to keep [`mouse_button_actions`]'s signature
/// legible (clippy `type_complexity`) and to make the exclusion explicit:
/// `Without<DisabledButton>` is what skips the disabled `HiveScape`, and
/// `Changed<Interaction>` limits each query to the frame a press lands.
type PressedButton<M> = (Changed<Interaction>, With<M>, Without<DisabledButton>);

/// Whether a [`bevy::ui::Interaction`] is a fresh press to act on.
///
/// Centralizes the "an activation happened" test so each mouse-path query reads
/// it identically. Only [`Pressed`](Interaction::Pressed) counts as an
/// activation. Takes [`Interaction`] by value (it is a one-byte `Copy` enum).
const fn is_press(interaction: Interaction) -> bool {
    matches!(interaction, Interaction::Pressed)
}

/// Drives [`RunningState`] transitions from **mouse** button presses.
///
/// For each enabled menu button whose [`Interaction`](bevy::ui::Interaction)
/// changed to [`Pressed`](bevy::ui::Interaction::Pressed) this frame, sets
/// [`NextState<RunningState>`] to the button's mapped target. The three marker
/// queries are disjoint (each filtered to one role marker and
/// `Without<DisabledButton>`), so they never conflict; the disabled `HiveScape`
/// is excluded by the `Without<DisabledButton>` filter and so produces no
/// transition under a mouse press.
///
/// `Changed<Interaction>` limits each query to the frame a press *lands*, so a
/// held button does not re-request the transition every frame. Registered under
/// `run_if(in_state(RunningState::Menu))` by
/// [`MenuScenePlugin`](super::super::plugin).
pub(in crate::states::running::menu) fn mouse_button_actions(
    mut next: ResMut<NextState<RunningState>>,
    battlescape: Query<&Interaction, PressedButton<BattlescapeButton>>,
    options: Query<&Interaction, PressedButton<OptionsButton>>,
    quit: Query<&Interaction, PressedButton<QuitButton>>,
    #[cfg(debug_assertions)] gang_editor: Query<
        &Interaction,
        PressedButton<super::super::components::GangEditorButton>,
    >,
) {
    if battlescape.iter().copied().any(is_press) {
        next.set(*MenuActionTarget::BATTLESCAPE);
    }
    if options.iter().copied().any(is_press) {
        next.set(*MenuActionTarget::OPTIONS);
    }
    if quit.iter().copied().any(is_press) {
        next.set(*MenuActionTarget::QUIT);
    }
    // DEV-ONLY: the Gang Editor button (GTW-420), cfg-gated so it never compiles into release.
    #[cfg(debug_assertions)]
    if gang_editor.iter().copied().any(is_press) {
        next.set(*MenuActionTarget::GANG_EDITOR);
    }
}

/// Drives [`RunningState`] transitions from **keyboard / gamepad** activation.
///
/// Drains the [`FocusActivated`] messages the GTW-119 focus-nav bridge raises on
/// `Enter` / gamepad South while a button holds focus, and for each activated
/// [`Entity`] requests the mapped transition. The activated entity is mapped to
/// its role by testing the three enabled marker queries (each filtered
/// `Without<DisabledButton>`) with [`Query::contains`]: an activation aimed at
/// the disabled `HiveScape` matches none of them and is therefore a no-op.
///
/// Read with [`MessageReader`] because [`FocusActivated`] is a Bevy 0.18
/// [`Message`](bevy::ecs::message::Message), not an observer event (bevy-traps
/// rule 4). Ordered `.after(FocusNavSystems::Bridge)` by
/// [`MenuScenePlugin`](super::super::plugin) so a same-frame activation is
/// consumed the frame it is raised (bevy-traps rule 3), and gated by
/// `run_if(in_state(RunningState::Menu))`.
pub(in crate::states::running::menu) fn focus_activated_actions(
    mut activations: MessageReader<FocusActivated>,
    mut next: ResMut<NextState<RunningState>>,
    battlescape: Query<(), (With<BattlescapeButton>, Without<DisabledButton>)>,
    options: Query<(), (With<OptionsButton>, Without<DisabledButton>)>,
    quit: Query<(), (With<QuitButton>, Without<DisabledButton>)>,
    #[cfg(debug_assertions)] gang_editor: Query<
        (),
        (
            With<super::super::components::GangEditorButton>,
            Without<DisabledButton>,
        ),
    >,
) {
    for activated in activations.read() {
        let entity = **activated;
        if battlescape.contains(entity) {
            next.set(*MenuActionTarget::BATTLESCAPE);
        } else if options.contains(entity) {
            next.set(*MenuActionTarget::OPTIONS);
        } else if quit.contains(entity) {
            next.set(*MenuActionTarget::QUIT);
        }
        // DEV-ONLY: the Gang Editor button (GTW-420), cfg-gated so it never compiles into
        // release. Checked after the enabled buttons; an activation aimed at it requests the
        // editor transition.
        #[cfg(debug_assertions)]
        if gang_editor.contains(entity) {
            next.set(*MenuActionTarget::GANG_EDITOR);
        }
        // Any other entity (notably the disabled `HiveScape`, which carries
        // `DisabledButton` and so matches none of the filtered queries) is
        // intentionally ignored — no transition.
    }
}
