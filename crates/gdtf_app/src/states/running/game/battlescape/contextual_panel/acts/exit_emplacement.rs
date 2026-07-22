//! The **Exit Emplacement** contextual act's panel-layer module (GTW-543 / GTW-571):
//! marker, descriptor, and offer scan.

use bevy::prelude::*;
use gdtf_battle_input::{SelectedShooter, contextual::ExitEmplacementAct};
use gdtf_battle_sim::{
    emplacement::{EmplacementOccupant, EmplacementState},
    ganger::{Faction, Position},
};
use gdtf_ui::ButtonLabel;

use crate::states::running::game::battlescape::contextual_panel::seam::{
    ContextualOffer, ContextualPanelAct, PanelSlot,
};

crate::support_item! {
    /// Marks the **Exit Emplacement** contextual button (GTW-543) — the act that dismounts
    /// the emplacement the selection is manning.
    ///
    /// Spawned [`Visibility::Hidden`](bevy::camera::visibility::Visibility) by the generic
    /// button spawn and revealed IN PLACE by the act's visibility toggle when
    /// `offer_exit_emplacement` names the emplacement whose
    /// [`EmplacementOccupant`](gdtf_battle_sim::emplacement::EmplacementOccupant) IS the current selection
    /// — so Exit is offered ONLY to the occupant (there is NO force-eject; exit is a SEPARATE
    /// TU-costed act). A unit marker: presence on an entity is the whole signal
    /// (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct ExitEmplacementButton;
}

impl ContextualPanelAct for ExitEmplacementAct {
    type Marker = ExitEmplacementButton;

    const SLOT: PanelSlot = PanelSlot::new(6);

    fn label() -> ButtonLabel {
        ButtonLabel::new("Exit")
    }
}

/// OFFERS the Exit Emplacement act: the emplacement whose recorded
/// [`EmplacementOccupant`] IS the selected actor, or nothing (GTW-543).
///
/// Exit is offered ONLY to the ganger currently manning the mount (no force-eject); no
/// adjacency check is needed — the occupant is by definition at the mount. The actor
/// gate mirrors the old shared resolve (the selection must carry `Position` +
/// `Faction` to be an actor at all — a filter-only query, no reads needed). The sim's
/// `dispatch_exit_emplacement` re-gate + TU spend are authoritative; this only decides
/// what to OFFER. Writes [`ContextualOffer`] via `set_if_neq` (change-detection
/// hygiene).
pub(in crate::states::running::game::battlescape) fn offer_exit_emplacement(
    selected: Res<SelectedShooter>,
    actors: Query<(), (With<Position>, With<Faction>)>,
    emplacements: Query<(Entity, &EmplacementOccupant), With<EmplacementState>>,
    mut offer: ResMut<ContextualOffer<ExitEmplacementAct>>,
) {
    let target = (**selected)
        .filter(|actor| actors.get(*actor).is_ok())
        .and_then(|actor| {
            emplacements
                .iter()
                .find(|(_, occupant)| ***occupant == actor)
                .map(|(entity, _)| entity)
        });
    offer.set_if_neq(ContextualOffer::new(target));
}
