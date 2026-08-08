use bevy::prelude::*;
use gdtf_battle_input::contextual::{ContextualAct, SlotRank};
use gdtf_ui::ButtonLabel;

pub(in crate::states::running::game::battlescape) trait ContextualPanelAct:
    ContextualAct
{
    type Marker: Component + Default;

    const SLOT: PanelSlot;

    fn label() -> ButtonLabel;
}

#[derive(Deref, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub(in crate::states::running::game::battlescape) struct PanelSlot(u8);

impl PanelSlot {
    #[must_use]
    pub(in crate::states::running::game::battlescape) const fn new(slot: u8) -> Self {
        Self(slot)
    }
}

#[derive(Component, Deref, Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::states::running::game::battlescape) struct ContextualActButton(PanelSlot);

impl ContextualActButton {
    #[must_use]
    pub(in crate::states::running::game::battlescape) const fn new(slot: PanelSlot) -> Self {
        Self(slot)
    }
}

#[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub(in crate::states::running::game::battlescape) struct VisibleSlotRank(Option<SlotRank>);

impl VisibleSlotRank {
    #[must_use]
    pub(in crate::states::running::game::battlescape) const fn unranked() -> Self {
        Self(None)
    }

    #[must_use]
    pub(in crate::states::running::game::battlescape) const fn new(rank: Option<SlotRank>) -> Self {
        Self(rank)
    }

    #[must_use]
    pub(in crate::states::running::game::battlescape) const fn rank(self) -> Option<SlotRank> {
        self.0
    }
}

/// Whether the panel shows an offered button as pressable rather than greyed out.
#[derive(Deref, Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct OfferPressable(bool);

impl OfferPressable {
    #[must_use]
    pub(crate) const fn new(pressable: bool) -> Self {
        Self(pressable)
    }
}

#[derive(Resource, Debug)]
pub(crate) struct ContextualOffer<A: ContextualAct> {
    target:    Option<A::Target>,
    pressable: OfferPressable,
}

impl<A: ContextualAct> Default for ContextualOffer<A> {
    fn default() -> Self {
        Self {
            target:    None,
            pressable: OfferPressable::new(true),
        }
    }
}

impl<A: ContextualAct> PartialEq for ContextualOffer<A> {
    fn eq(&self, other: &Self) -> bool {
        self.target == other.target && self.pressable == other.pressable
    }
}

impl<A: ContextualAct> ContextualOffer<A> {
    /// An offer the panel shows as pressable.
    #[must_use]
    pub(crate) const fn new(target: Option<A::Target>) -> Self {
        Self {
            target,
            pressable: OfferPressable::new(true),
        }
    }

    /// The same offer, shown greyed out when `pressable` says the actor cannot afford it.
    #[must_use]
    pub(crate) const fn with_pressable(self, pressable: OfferPressable) -> Self {
        Self {
            target: self.target,
            pressable,
        }
    }

    #[must_use]
    pub(crate) const fn target(&self) -> Option<A::Target> {
        self.target
    }

    #[must_use]
    pub(crate) const fn pressable(&self) -> OfferPressable {
        self.pressable
    }

    #[must_use]
    pub(crate) const fn is_offered(&self) -> bool {
        self.target.is_some()
    }
}
