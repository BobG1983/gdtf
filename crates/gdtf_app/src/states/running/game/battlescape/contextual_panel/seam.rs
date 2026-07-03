//! The panel-side contextual-act descriptor seam (GTW-571): the [`ContextualPanelAct`]
//! trait, the per-act [`ContextualOffer`] resource the offer scans write, the
//! value-carrying [`ContextualActButton`] component every act button wears, and the
//! [`PanelSlot`] ordering key.
//!
//! Together with the generic button systems
//! ([`buttons`](super::systems::spawn_contextual_button)) and the
//! [`registrar`](super::registrar), this dissolves the old per-marker lattice: no
//! per-act `*VisFilter` aliases, no `PanelVisibility` bundle, no per-marker press
//! queries, no per-marker spawn stanzas — adding a contextual act adds ONE module under
//! [`acts`](super::acts) plus ONE registration line in the plugin (see
//! `docs/authoring/contextual-act-recipe.md`).

use bevy::prelude::*;
use gdtf_battle_input::contextual::ContextualAct;
use gdtf_ui::ButtonLabel;

/// A CONTEXTUAL act's panel-layer descriptor (GTW-571) — what the eight acts vary in on
/// the button side: the marker component, the button label, and the button's stable
/// slot in the panel column. One impl per act, on the act's input-layer
/// [`ContextualAct`] token (the vertical act module for this crate layer lives under
/// [`acts`](super::acts)).
///
/// The generic spawn / visibility / press systems are stamped over this trait by
/// [`add_contextual_act_button`](super::registrar::ContextualPanelActAppExt::add_contextual_act_button)
/// — compile-time generic registration, never a runtime descriptor table (P4). The
/// descriptor carries NO keybind field: contextual acts are button-only (GTW-571 Q8,
/// ruled).
pub(in crate::states::running::game::battlescape) trait ContextualPanelAct:
    ContextualAct
{
    /// The act button's unit MARKER component (declared through `crate::support_item!`
    /// in the act's module so the external AC tests can name it). `Default` so the
    /// generic spawn system can attach it.
    type Marker: Component + Default;

    /// The act button's stable top-to-bottom slot in the panel column — slots must be
    /// UNIQUE across the registered acts (the recipe's pick-an-unused-slot station).
    const SLOT: PanelSlot;

    /// The act button's themed label.
    fn label() -> ButtonLabel;
}

/// A contextual-act button's stable position in the panel's top-to-bottom column
/// (GTW-571).
///
/// A named newtype over the raw ordinal (no-bare-types) with a private inner: per-act
/// spawn systems run in nondeterministic order, so child order alone would shuffle the
/// buttons between runs — the `order_contextual_buttons` pass sorts the spawned buttons
/// by this key instead, making the column deterministic. `Ord` so it IS the sort key.
#[derive(Deref, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub(in crate::states::running::game::battlescape) struct PanelSlot(u8);

impl PanelSlot {
    /// Wrap a raw column ordinal (0 = topmost button).
    #[must_use]
    pub(in crate::states::running::game::battlescape) const fn new(slot: u8) -> Self {
        Self(slot)
    }
}

/// The value-carrying component EVERY contextual-act button wears (GTW-571 C2) —
/// carries the act's [`PanelSlot`] and tags the entity as a contextual-act button.
///
/// The act-agnostic pieces read it: `order_contextual_buttons` sorts the panel's
/// children by the carried slot, and `sync_panel_root_visibility` shows the root iff
/// ANY entity wearing this is visible — both act-count-independent (no per-act
/// disjointness filters anywhere). A named domain component, never a bare `u8`.
#[derive(Component, Deref, Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::states::running::game::battlescape) struct ContextualActButton(PanelSlot);

impl ContextualActButton {
    /// Tag a button with its act's [`PanelSlot`].
    #[must_use]
    pub(in crate::states::running::game::battlescape) const fn new(slot: PanelSlot) -> Self {
        Self(slot)
    }
}

/// Act `A`'s current OFFER — the target its button press would act on, or [`None`] when
/// the act is not offered (GTW-571; the per-act replacement for the old nine-field
/// `ContextualTargets` struct).
///
/// Written each update by the act's bespoke offer scan (the system passed to
/// [`add_contextual_act_button`](super::registrar::ContextualPanelActAppExt::add_contextual_act_button))
/// via `set_if_neq` (change-detection hygiene — an unchanged offer never spuriously
/// trips `Changed<ContextualOffer<A>>`), read by the act's generic visibility toggle +
/// press system. A named newtype over the optional target (no-bare-types; the inner
/// stays private — [`new`](Self::new) / [`target`](Self::target) are the only
/// touch-points). `PartialEq` is hand-written over the inner target (a derive would
/// wrongly bound the act TOKEN `A: PartialEq` — the GTW-567 generic-derive lesson).
#[derive(Resource, Debug)]
pub(in crate::states::running::game::battlescape) struct ContextualOffer<A: ContextualAct>(
    Option<A::Target>,
);

impl<A: ContextualAct> Default for ContextualOffer<A> {
    fn default() -> Self {
        Self(None)
    }
}

impl<A: ContextualAct> PartialEq for ContextualOffer<A> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<A: ContextualAct> ContextualOffer<A> {
    /// Wrap an offer — the value an offer scan hands to `set_if_neq`.
    #[must_use]
    pub(in crate::states::running::game::battlescape) const fn new(
        target: Option<A::Target>,
    ) -> Self {
        Self(target)
    }

    /// The offered target, or [`None`] when the act is not offered.
    #[must_use]
    pub(in crate::states::running::game::battlescape) const fn target(&self) -> Option<A::Target> {
        self.0
    }

    /// Whether the act is currently offered (the button-visibility predicate).
    #[must_use]
    pub(in crate::states::running::game::battlescape) const fn is_offered(&self) -> bool {
        self.0.is_some()
    }
}
