//! The panel-side contextual-act REGISTRAR (GTW-571 C1/P5): the panel's `SystemSet`
//! vocabulary — configured ONCE — plus the compile-time
//! [`add_contextual_act_button`](ContextualPanelActAppExt::add_contextual_act_button)
//! extension that stamps the generic button systems per act.

use bevy::{
    ecs::{schedule::IntoScheduleConfigs, system::ScheduleSystem},
    prelude::*,
};
use gdtf_battle_input::{contextual::ContextualActSystems, pick_hovered_cell};
use gdtf_battle_presenter::playback_caught_up;
use gdtf_battle_sim::prelude::BattleInProgress;

use crate::states::{
    BattleScapeState,
    running::game::battlescape::contextual_panel::{
        seam::{ContextualOffer, ContextualPanelAct},
        systems::{
            press_contextual_button, press_contextual_button_via_key, spawn_contextual_button,
            sync_contextual_button_visibility,
        },
    },
};

/// The contextual panel's per-update band vocabulary (GTW-571 P5) — configured ONCE by
/// [`configure_contextual_panel_sets`], then joined per act by the registrar.
///
/// The chain `Offer -> Toggle -> Rank -> Press` makes a press read the SAME update's
/// freshly-scanned offer (the old detect-before-router edge, now set-shaped) AND the
/// same update's freshly-computed visible-slot ranks (GTW-563 — the digit-key press
/// resolves its key from the button's current rank), and `Press` is additionally ordered
/// `.before` the input layer's [`ContextualActSystems::Drain`] — so a press queued this
/// update is drained (and its `*Requested` sim-consumed) this update: the Q5 same-frame
/// guarantee, all edges explicit (`bevy-traps.md` #3). A framework `SystemSet` label, not
/// a domain value (the no-bare-types framework carve-out).
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::states::running::game::battlescape) enum ContextualPanelSystems {
    /// The per-act OFFER scans — each act's bespoke system writing its
    /// [`ContextualOffer<A>`].
    Offer,
    /// The per-act generic visibility toggles reading the offers.
    Toggle,
    /// The act-agnostic ranking pass numbering the currently-visible buttons 1..N in
    /// slot order (GTW-563 — the keyboard slot binding), read by the digit-key presses.
    Rank,
    /// The per-act generic press routers (mouse + digit key) pushing onto the per-act
    /// intent queues.
    Press,
}

/// The contextual panel's `OnEnter(BattleRunning)` spawn vocabulary (GTW-571 P5) —
/// configured ONCE by [`configure_contextual_panel_sets`].
///
/// The chain `Root -> Buttons -> Order` spawns the panel box first (the explicit edges
/// force `Commands` sync points, so each stage queries the previous stage's spawns),
/// then every act's button, then the deterministic
/// [`order_contextual_buttons`](super::systems::order_contextual_buttons) re-parent. A
/// framework `SystemSet` label, not a domain value.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::states::running::game::battlescape) enum ContextualPanelSpawnSystems {
    /// The bare panel-box root spawn.
    Root,
    /// Every per-act generic button spawn.
    Buttons,
    /// The deterministic slot-order re-parent.
    Order,
}

/// Configures the contextual panel's `SystemSet` vocabulary — ONCE, from
/// [`ContextualPanelPlugin`](super::plugin::ContextualPanelPlugin)'s `build`
/// (`bevy-traps.md` #5: `configure_sets` precedes `.in_set`; GTW-571 P5: the vocabulary
/// is registrar-owned and configured in one place).
///
/// Update-band edges: `Offer -> Toggle -> Rank -> Press` chained (GTW-563 inserts `Rank`
/// so the digit-key press reads the SAME update's freshly-computed visible-slot ranks);
/// `Offer.before(pick_hovered_cell)` (the consumers-read-the-hover-BEFORE-the-picker-
/// rewrites-it idiom — the throw offer reads `InspectTarget`, so the whole offer band
/// takes the deterministic consume-then-resolve order the click decision uses);
/// `Press.before(ContextualActSystems::Drain)` (the Q5 same-frame press -> `*Requested`
/// edge). Spawn-band edges: `Root -> Buttons -> Order` chained.
pub(in crate::states::running::game::battlescape) fn configure_contextual_panel_sets(
    app: &mut App,
) {
    app.configure_sets(
        Update,
        (
            ContextualPanelSystems::Offer,
            ContextualPanelSystems::Toggle,
            ContextualPanelSystems::Rank,
            ContextualPanelSystems::Press,
        )
            .chain(),
    )
    .configure_sets(
        Update,
        ContextualPanelSystems::Offer.before(pick_hovered_cell),
    )
    .configure_sets(
        Update,
        ContextualPanelSystems::Press.before(ContextualActSystems::Drain),
    )
    .configure_sets(
        OnEnter(BattleScapeState::BattleRunning),
        (
            ContextualPanelSpawnSystems::Root,
            ContextualPanelSpawnSystems::Buttons,
            ContextualPanelSpawnSystems::Order,
        )
            .chain(),
    );
}

/// Compile-time contextual-act BUTTON registrar (GTW-571 C1) — the panel layer's
/// one-line-per-act extension, mirroring the input layer's
/// [`add_contextual_act`](gdtf_battle_input::contextual::ContextualActAppExt::add_contextual_act).
///
/// `app.add_contextual_act_button::<A>(offer_a)` wires act `A`'s whole panel-layer
/// slice: the offer resource, the generic spawn / toggle / press systems in the
/// configured set vocabulary, and the act's BESPOKE offer scan (passed in — the scans
/// genuinely vary in what they read, so the descriptor cannot stamp them). NEVER a
/// runtime descriptor table (P4) — the act set is closed at compile time by the
/// registration lines in [`ContextualPanelPlugin`](super::plugin::ContextualPanelPlugin).
pub(in crate::states::running::game::battlescape) trait ContextualPanelActAppExt {
    /// Register contextual act `A`'s button: `init_resource::<ContextualOffer<A>>()`,
    /// the generic [`spawn_contextual_button::<A>`](spawn_contextual_button) in the
    /// spawn band's `Buttons` set, and — gated
    /// `run_if(resource_exists::<BattleInProgress>)` (`bevy-traps.md` #1) — `offer` in
    /// `Offer`, [`sync_contextual_button_visibility::<A>`](sync_contextual_button_visibility)
    /// in `Toggle`, and BOTH press routers in `Press`: the mouse
    /// [`press_contextual_button::<A>`](press_contextual_button) and the digit-key
    /// [`press_contextual_button_via_key::<A>`](press_contextual_button_via_key) (GTW-563),
    /// which share the same per-act intent queue.
    fn add_contextual_act_button<A: ContextualPanelAct, M>(
        &mut self,
        offer: impl IntoScheduleConfigs<ScheduleSystem, M>,
    ) -> &mut Self;
}

impl ContextualPanelActAppExt for App {
    fn add_contextual_act_button<A: ContextualPanelAct, M>(
        &mut self,
        offer: impl IntoScheduleConfigs<ScheduleSystem, M>,
    ) -> &mut Self {
        self.init_resource::<ContextualOffer<A>>()
            .add_systems(
                OnEnter(BattleScapeState::BattleRunning),
                spawn_contextual_button::<A>.in_set(ContextualPanelSpawnSystems::Buttons),
            )
            .add_systems(
                Update,
                (
                    offer.in_set(ContextualPanelSystems::Offer),
                    sync_contextual_button_visibility::<A>.in_set(ContextualPanelSystems::Toggle),
                )
                    .run_if(resource_exists::<BattleInProgress>),
            )
            // GTW-727 C24: the PRESS is blocked at the push site while the presenter is
            // catching up, so a contextual act is never queued against a stale screen. The
            // OFFER and the button's visibility stay live above — the panel keeps showing
            // WHAT is available, it just cannot be pressed until the screen is current.
            // GTW-563: the DIGIT-KEY press rides the same set + gate as the mouse press
            // (both push onto the SAME `PendingContextualIntents<A>` — ADR-0001), chained
            // so their shared `ResMut` on that queue has a deterministic order.
            .add_systems(
                Update,
                (
                    press_contextual_button::<A>,
                    press_contextual_button_via_key::<A>,
                )
                    .chain()
                    .in_set(ContextualPanelSystems::Press)
                    .run_if(resource_exists::<BattleInProgress>.and_then(playback_caught_up)),
            )
    }
}
