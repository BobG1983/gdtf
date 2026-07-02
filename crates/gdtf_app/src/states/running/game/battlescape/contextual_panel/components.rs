//! Markers, layout consts, and the target resource for the battlescape CONTEXTUAL PANEL
//! (GTW-294).
//!
//! The contextual panel is the bottom-RIGHT cluster of the HUD (per the
//! `docs/ui_mockups/battlescape_mockup.png` bottom-right corner — "Contextual Buttons go
//! Here", to the right of the Stance column). It hosts the situational acts a selected ganger
//! can take on a neighbour — **Execute** / **Stabilize** on a DOWNED neighbour, **Melee**
//! (GTW-507) on an in-LOS alive enemy, **Shove** (GTW-525) on any alive opposing neighbour —
//! plus a deferred **Open Door** act. The panel and its buttons spawn
//! [`Visibility::Hidden`](bevy::camera::visibility::Visibility):
//! the spawn system only builds the tree. The live detection system fills [`ContextualTargets`]
//! each update and toggles the Execute / Stabilize buttons' `Visibility` IN PLACE when a valid
//! downed neighbour is in reach, and the press router routes a press onto the shared act-intent
//! seam. Open Door stays hidden (no sim verb yet — a deferred follow-up).
//!
//! ## Visibility (the GTW-145 test-only-surface convention)
//!
//! Each marker is declared through [`crate::support_item!`], which widens it to `pub` under the
//! `test-support` feature — so the external integration tests can name it through
//! [`crate::test_support`] — and keeps it `pub(crate)` otherwise, so it stays internal in the
//! production `grimdark_turfwar` binary (which compiles `gdtf_app` WITHOUT `test-support`) and
//! satisfies `unreachable_pub`. This mirrors the action-bar / bottom-bar marker shape.

use bevy::prelude::*;
use gdtf_battle_sim::CellLevel;

crate::support_item! {
    /// Marks the **root** node of the contextual panel box (the bottom-right cluster holding the
    /// Execute / Stabilize / Open Door buttons — GTW-294).
    ///
    /// It is a BARE [`spawn_panel`](gdtf_ui::spawn_panel) themed box (the bottom-bar precedent: a
    /// bare absolute root resolves the UI camera fine, no wrapper needed), spawned
    /// [`Visibility::Hidden`](bevy::camera::visibility::Visibility) and revealed IN PLACE by
    /// `detect_contextual_targets`. It carries [`GlobalZIndex`](bevy::ui::GlobalZIndex)`(`
    /// [`CONTEXTUAL_PANEL_Z`]`)` so the whole panel subtree (this box + the buttons) stacks ABOVE
    /// the opaque bottom bar it overlaps — without it the bar (a higher-z opaque panel) painted
    /// over the panel and nothing rendered (the GTW-294 occlusion bug). The
    /// `OnExit(BattleRunning)` despawn tears the whole subtree down by THIS marker. Widened
    /// through `support_item!` so the AC tests can assert the box's presence. A unit marker:
    /// presence on an entity is the whole signal (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct ContextualPanelRoot;
}

/// The contextual panel's stacking order ([`GlobalZIndex`](bevy::ui::GlobalZIndex) — the higher,
/// the nearer the viewer), set strictly ABOVE the bottom bar so the panel draws ON TOP of it.
///
/// A framework-plumbing `const` fed straight to a [`GlobalZIndex`](bevy::ui::GlobalZIndex) (the
/// framework carve-out, not a domain value). The contextual panel is anchored bottom-RIGHT,
/// landing INSIDE the bottom bar's full-width opaque footprint; the bottom bar carries
/// `GlobalZIndex(10)` (`BOTTOM_BAR_Z`) and the weapon + stance cluster that sits on the bar carries
/// `GlobalZIndex(11)` (`PANEL_Z`). With NO `GlobalZIndex` (default `0`) the contextual panel drew
/// BEHIND the opaque bar and rendered nothing (the GTW-294 occlusion bug, proven by a runtime
/// probe: correct camera / parent / transform / size / visibility, yet zero pixels). This value is
/// `20` — comfortably above both the bar (`10`) and the on-bar cluster (`11`), with headroom — so
/// the whole panel subtree (root box + Execute / Stabilize buttons) draws on top of the bar.
pub(in crate::states::running::game::battlescape) const CONTEXTUAL_PANEL_Z: i32 = 20;

crate::support_item! {
    /// Marks the **Execute** contextual button (GTW-294) — the coup-de-grâce act on a downed
    /// neighbour.
    ///
    /// Spawned [`Visibility::Hidden`](bevy::camera::visibility::Visibility) and revealed IN PLACE by the
    /// detection system (which also wires its press to the `ExecuteDownedRequested` act) when
    /// [`ContextualTargets::execute`] names a target. A unit marker: presence on an entity is the
    /// whole signal (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct ExecuteButton;
}

crate::support_item! {
    /// Marks the **Stabilize** contextual button (GTW-294) — the act that arrests a downed
    /// neighbour's bleed-out.
    ///
    /// Spawned [`Visibility::Hidden`](bevy::camera::visibility::Visibility) and revealed IN PLACE by the
    /// detection system (which also wires its press to the `StabilizeDownedRequested` act) when
    /// [`ContextualTargets::stabilize`] names a target. A unit marker: presence on an entity is the
    /// whole signal (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct StabilizeButton;
}

crate::support_item! {
    /// Marks the **Open Door** contextual button (GTW-294 scaffold; GTW-315 live) — the act that
    /// opens an 8-adjacent CLOSED door.
    ///
    /// Spawned [`Visibility::Hidden`](bevy::camera::visibility::Visibility) and revealed IN PLACE by the
    /// detection system (which also wires its press to the `OpenDoorRequested` act) when
    /// [`ContextualTargets::open_door`] names a target — the first 8-adjacent openable terrain entity in
    /// the [`OpenState::Closed`](gdtf_battle_sim::OpenState) state (the button always OPENS; closing is
    /// not offered, and F4 is PLAYER-ONLY). A unit marker: presence on an entity is the whole signal
    /// (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct OpenDoorButton;
}

crate::support_item! {
    /// Marks the **Melee** contextual button (GTW-507) — the close-combat strike act on an
    /// 8-adjacent, alive, in-LOS ENEMY ganger (`docs/combat/resolution.md` §7).
    ///
    /// A DEDICATED action-bar button (the GTW-507 D1 ruling — NOT a left-click overload),
    /// mirroring [`ExecuteButton`] / [`StabilizeButton`]. Spawned
    /// [`Visibility::Hidden`](bevy::camera::visibility::Visibility) and revealed IN PLACE by the
    /// detection system (which also wires its press to the `MeleeRequested` act) when
    /// [`ContextualTargets::melee`] names a target. A unit marker: presence on an entity is the
    /// whole signal (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct MeleeButton;
}

crate::support_item! {
    /// Marks the **Shove** contextual button (GTW-525) — the deliberate knock-back act on an
    /// 8-adjacent, ALIVE, opposing ganger.
    ///
    /// A UNIVERSAL act available to EVERY ganger (NO weapon requirement — a pure-displacement
    /// shove, not a weapon strike), mirroring the DEDICATED [`MeleeButton`] (GTW-507) /
    /// [`ExecuteButton`] / [`StabilizeButton`]. Spawned
    /// [`Visibility::Hidden`](bevy::camera::visibility::Visibility) and revealed IN PLACE by the
    /// detection system (which also wires its press to the `ShoveRequested` act) when
    /// [`ContextualTargets::shove`] names a target — a WEAKER gate than Melee's (no LOS required:
    /// a shove is contact, not a sighted strike). A unit marker: presence on an entity is the whole
    /// signal (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct ShoveButton;
}

crate::support_item! {
    /// Marks the **Enter Emplacement** contextual button (GTW-543) — the act that mans an
    /// 8-adjacent VACANT weapon emplacement.
    ///
    /// A DEDICATED contextual button, mirroring [`OpenDoorButton`] / [`MeleeButton`]. Spawned
    /// [`Visibility::Hidden`](bevy::camera::visibility::Visibility) and revealed IN PLACE by the
    /// detection system (which also wires its press to the `EnterEmplacementRequested` act) when
    /// [`ContextualTargets::enter_emplacement`] names a VACANT emplacement the selected PLAYER actor
    /// is 8-adjacent to (F4 player-only). A unit marker: presence on an entity is the whole signal
    /// (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct EnterEmplacementButton;
}

crate::support_item! {
    /// Marks the **Exit Emplacement** contextual button (GTW-543) — the act that dismounts the
    /// emplacement the selection is manning.
    ///
    /// A DEDICATED contextual button, mirroring [`EnterEmplacementButton`]. Spawned
    /// [`Visibility::Hidden`](bevy::camera::visibility::Visibility) and revealed IN PLACE by the
    /// detection system (which also wires its press to the `ExitEmplacementRequested` act) when
    /// [`ContextualTargets::exit_emplacement`] names the emplacement whose
    /// [`EmplacementOccupant`](gdtf_battle_sim::EmplacementOccupant) IS the current selection — so
    /// Exit is offered ONLY to the occupant (there is NO force-eject; exit is a SEPARATE TU-costed
    /// act). A unit marker: presence on an entity is the whole signal (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct ExitEmplacementButton;
}

/// The neighbours the contextual panel can act on; written each update by the detection system
/// (GTW-294 live slice; extended by GTW-507 / GTW-508 / GTW-525 / GTW-315).
///
/// Every field defaults to [`None`] (no target in reach). The detection system
/// [`detect_contextual_targets`](super::systems::detect_contextual_targets) fills
/// [`execute`](Self::execute) / [`stabilize`](Self::stabilize) / [`melee`](Self::melee) /
/// [`melee_structure`](Self::melee_structure) / [`shove`](Self::shove) /
/// [`open_door`](Self::open_door) / [`enter_emplacement`](Self::enter_emplacement) /
/// [`exit_emplacement`](Self::exit_emplacement) with the neighbour each act targets (via a direct
/// struct literal), and reveals the matching button only when its field is [`Some`]; the press
/// router
/// [`contextual_button_intents`](super::systems::contextual_button_intents) reads these to route a
/// press to the carried target. A [`Resource`] inserted by
/// [`ContextualPanelPlugin`](super::plugin::ContextualPanelPlugin)'s `init_resource`.
#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(in crate::states::running::game::battlescape) struct ContextualTargets {
    /// The downed neighbour the **Execute** act would target, or [`None`] when no downed ENEMY
    /// is in reach. Written each update by the detection system.
    pub(in crate::states::running::game::battlescape) execute:           Option<Entity>,
    /// The downed neighbour the **Stabilize** act would target, or [`None`] when no
    /// not-yet-stabilized downed ALLY is in reach. Written each update by the detection system.
    pub(in crate::states::running::game::battlescape) stabilize:         Option<Entity>,
    /// The opposing ganger the **Melee** act would strike (GTW-507), or [`None`] when no
    /// 8-adjacent, ALIVE, in-LOS ENEMY is in reach. A STRONGER gate than Execute's
    /// downed-adjacency (alive + LOS, not downed). Written each update by the detection system.
    pub(in crate::states::running::game::battlescape) melee:             Option<Entity>,
    /// The adjacent inert STRUCTURE cell the **Melee** act would SMASH (GTW-508), or [`None`]
    /// when no 8-adjacent intact Cover / Wall cell is in reach. Offered ONLY when no meleeable
    /// ganger [`melee`](Self::melee) target is in reach (a ganger target takes priority), so the
    /// ONE Melee button routes to a ganger strike or a cover-smash, never both. Written each
    /// update by the detection system.
    pub(in crate::states::running::game::battlescape) melee_structure:   Option<CellLevel>,
    /// The opposing ganger the **Shove** act would knock back (GTW-525), or [`None`] when no
    /// 8-adjacent, ALIVE, opposing ganger is in reach. A WEAKER gate than Melee's — NO LOS
    /// required (a shove is contact, not a sighted strike) and NO weapon required (any ganger can
    /// shove). Written each update by the detection system.
    pub(in crate::states::running::game::battlescape) shove:             Option<Entity>,
    /// The adjacent CLOSED door the **Open Door** act would open (GTW-315), or [`None`] when no
    /// 8-adjacent openable terrain entity in the [`OpenState::Closed`](gdtf_battle_sim::OpenState)
    /// state is in reach. The button always OPENS — an already-open door is NOT offered (closing is
    /// not a contextual act), and F4 is PLAYER-ONLY. Written each update by the detection system.
    pub(in crate::states::running::game::battlescape) open_door:         Option<Entity>,
    /// The adjacent VACANT emplacement the **Enter Emplacement** act would man (GTW-543), or
    /// [`None`] when no 8-adjacent
    /// [`EmplacementState::Vacant`](gdtf_battle_sim::EmplacementState) emplacement is in reach. F4 is
    /// PLAYER-ONLY (this offer runs only for a selected player-faction actor). Written each update by
    /// the detection system.
    pub(in crate::states::running::game::battlescape) enter_emplacement: Option<Entity>,
    /// The emplacement the **Exit Emplacement** act would dismount (GTW-543), or [`None`] when the
    /// selection is not manning any emplacement — the emplacement whose
    /// [`EmplacementOccupant`](gdtf_battle_sim::EmplacementOccupant) IS the current selection.
    /// Offered ONLY to the occupant (there is NO force-eject). Written each update by the detection
    /// system.
    pub(in crate::states::running::game::battlescape) exit_emplacement:  Option<Entity>,
}

impl ContextualTargets {
    /// The **Execute** target — the downed ENEMY a press would execute, or [`None`].
    ///
    /// Read by [`contextual_button_intents`](super::systems::contextual_button_intents) to
    /// route an Execute press to the carried target (the `Some`-guard keeps a stale press
    /// safe).
    #[must_use]
    pub(in crate::states::running::game::battlescape) const fn execute(&self) -> Option<Entity> {
        self.execute
    }

    /// The **Stabilize** target — the not-yet-stabilized downed ALLY a press would stabilize,
    /// or [`None`].
    ///
    /// Read by [`contextual_button_intents`](super::systems::contextual_button_intents) to
    /// route a Stabilize press to the carried target.
    #[must_use]
    pub(in crate::states::running::game::battlescape) const fn stabilize(&self) -> Option<Entity> {
        self.stabilize
    }

    /// The **Melee** target — the 8-adjacent, alive, in-LOS ENEMY a press would strike, or
    /// [`None`] (GTW-507).
    ///
    /// Read by [`contextual_button_intents`](super::systems::contextual_button_intents) to
    /// route a Melee press to the carried target (the `Some`-guard keeps a stale press safe).
    #[must_use]
    pub(in crate::states::running::game::battlescape) const fn melee(&self) -> Option<Entity> {
        self.melee
    }

    /// The **Melee-structure** target — the 8-adjacent intact Cover / Wall cell a press would
    /// SMASH, or [`None`] (GTW-508).
    ///
    /// Read by [`contextual_button_intents`](super::systems::contextual_button_intents) to route
    /// a Melee press to a cover-smash when no ganger [`melee`](Self::melee) target is in reach
    /// (the ganger target takes priority — the router checks it first).
    #[must_use]
    pub(in crate::states::running::game::battlescape) const fn melee_structure(
        &self,
    ) -> Option<CellLevel> {
        self.melee_structure
    }

    /// The **Shove** target — the 8-adjacent, alive, opposing ganger a press would knock back, or
    /// [`None`] (GTW-525).
    ///
    /// Read by [`contextual_button_intents`](super::systems::contextual_button_intents) to route a
    /// Shove press to the carried target (the `Some`-guard keeps a stale press safe).
    #[must_use]
    pub(in crate::states::running::game::battlescape) const fn shove(&self) -> Option<Entity> {
        self.shove
    }

    /// The **Open Door** target — the 8-adjacent CLOSED door a press would open, or [`None`]
    /// (GTW-315).
    ///
    /// Read by [`contextual_button_intents`](super::systems::contextual_button_intents) to route
    /// an Open-Door press to the carried door (the `Some`-guard keeps a stale press safe).
    #[must_use]
    pub(in crate::states::running::game::battlescape) const fn open_door(&self) -> Option<Entity> {
        self.open_door
    }

    /// The **Enter Emplacement** target — the 8-adjacent VACANT emplacement a press would man, or
    /// [`None`] (GTW-543).
    ///
    /// Read by [`contextual_button_intents`](super::systems::contextual_button_intents) to route an
    /// Enter press to the carried emplacement (the `Some`-guard keeps a stale press safe).
    #[must_use]
    pub(in crate::states::running::game::battlescape) const fn enter_emplacement(
        &self,
    ) -> Option<Entity> {
        self.enter_emplacement
    }

    /// The **Exit Emplacement** target — the emplacement the selection is manning that a press would
    /// dismount, or [`None`] (GTW-543).
    ///
    /// Read by [`contextual_button_intents`](super::systems::contextual_button_intents) to route an
    /// Exit press to the carried emplacement (the `Some`-guard keeps a stale press safe).
    #[must_use]
    pub(in crate::states::running::game::battlescape) const fn exit_emplacement(
        &self,
    ) -> Option<Entity> {
        self.exit_emplacement
    }
}

/// The contextual panel's INSET from the window's RIGHT edge, as a fraction of the window WIDTH
/// ([`Val::Vw`](bevy::ui::Val) — the responsive-units ruling: relative units only, NO fixed px,
/// so it scales with the window on resize).
///
/// A `const`, layout plumbing fed straight to a [`Node`](bevy::ui::Node)'s `right` (the
/// `CELL_PX`-class carve-out, not a domain value). Anchors the panel off the window's right side
/// so it sits in the bottom-right corner, matching the mockup's contextual cluster to the right
/// of the Stance column.
pub(in crate::states::running::game::battlescape) const CONTEXTUAL_PANEL_RIGHT_VW: f32 = 1.5;

/// The contextual panel's INSET from the window's BOTTOM edge, as a fraction of the window HEIGHT
/// ([`Val::Vh`](bevy::ui::Val) — the responsive-units ruling: relative units only, NO fixed px,
/// so it scales with the window on resize).
///
/// A `const`, layout plumbing fed straight to a [`Node`](bevy::ui::Node)'s `bottom` (the
/// `CELL_PX`-class carve-out, not a domain value). Lifts the panel off the window's bottom edge
/// so its buttons sit inside the bottom HUD band rather than flush against the window bottom,
/// matching the mockup.
pub(in crate::states::running::game::battlescape) const CONTEXTUAL_PANEL_BOTTOM_VH: f32 = 3.0;

/// The contextual panel's WIDTH, as a fraction of the window WIDTH ([`Val::Vw`](bevy::ui::Val) —
/// the responsive-units ruling: relative units only, NO fixed px, so it scales with the window
/// on resize).
///
/// A `const`, layout plumbing fed straight to a [`Node`](bevy::ui::Node)'s `width` (the
/// `CELL_PX`-class carve-out, not a domain value). Sized to the bottom-right contextual cluster
/// region of the mockup (the empty space to the right of the Stance column).
pub(in crate::states::running::game::battlescape) const CONTEXTUAL_PANEL_WIDTH_VW: f32 = 22.0;

/// The vertical gap between the contextual panel's stacked buttons, as a fraction of the window
/// HEIGHT ([`Val::Vh`](bevy::ui::Val) — the responsive-units ruling: relative units only, NO
/// fixed px, so it scales with the window on resize).
///
/// A `const`, layout plumbing fed straight to a [`Node`](bevy::ui::Node)'s `row_gap` (the
/// `CELL_PX`-class carve-out, not a domain value). Separates the Execute / Stabilize / Open Door
/// buttons stacked in the panel's column.
pub(in crate::states::running::game::battlescape) const CONTEXTUAL_PANEL_ROW_GAP_VH: f32 = 1.0;
