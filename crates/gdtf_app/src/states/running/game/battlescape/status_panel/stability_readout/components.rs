use bevy::prelude::*;
use gdtf_battle_sim::stability::ConeMult;
use gdtf_ui::FillFraction;

crate::support_item! {
                                        #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct StabilityBar;
}

#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub(in crate::states::running::game::battlescape::status_panel) struct Steadiness(f32);

impl Steadiness {
                                        #[must_use]
    pub(in crate::states::running::game::battlescape::status_panel) fn from_cone_mult(
        cone_mult: ConeMult,
    ) -> Self {
        Self((1.0 - *cone_mult).clamp(0.0, 1.0))
    }

                #[must_use]
    pub(in crate::states::running::game::battlescape::status_panel) const fn fill_fraction(
        self,
    ) -> FillFraction {
        FillFraction::new(self.0)
    }
}
