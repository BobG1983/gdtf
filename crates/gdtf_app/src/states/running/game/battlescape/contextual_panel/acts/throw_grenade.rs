//! The **Throw Grenade** contextual act's panel-layer module (GTW-546 / GTW-571):
//! marker, descriptor, and offer scan.

use bevy::{ecs::system::SystemParam, prelude::*};
use gdtf_battle_input::{InspectTarget, SelectedShooter, contextual::ThrowGrenadeAct};
use gdtf_battle_sim::{
    CellLevel, TrajectoryStyle,
    ganger::{Faction, Position},
    weapon::{MeleeWeapon, Wields},
};
use gdtf_ui::ButtonLabel;

use crate::states::running::game::battlescape::contextual_panel::seam::{
    ContextualOffer, ContextualPanelAct, PanelSlot,
};

crate::support_item! {
    /// Marks the **Throw** contextual button (GTW-546) — the act that LOBS a grenade at a
    /// target cell at range.
    ///
    /// Spawned [`Visibility::Hidden`](bevy::camera::visibility::Visibility) by the generic
    /// button spawn and revealed IN PLACE by the act's visibility toggle when
    /// [`offer_throw_grenade`] names a target cell — offered when the selected PLAYER actor
    /// wields a [`TrajectoryStyle::Arc`](gdtf_battle_sim::TrajectoryStyle) weapon and a
    /// target cell is hovered (a BLIND lob needs no adjacency / LOS gate — F4 player-only).
    /// A unit marker: presence on an entity is the whole signal (no-bare-types rule).
    #[derive(Component, Clone, Copy, PartialEq, Eq, Debug, Default)]
    struct ThrowGrenadeButton;
}

impl ContextualPanelAct for ThrowGrenadeAct {
    type Marker = ThrowGrenadeButton;

    const SLOT: PanelSlot = PanelSlot::new(7);

    fn label() -> ButtonLabel {
        ButtonLabel::new("Throw")
    }
}

/// The reads the GTW-546 THROW offer needs, bundled into one [`SystemParam`] so
/// [`offer_throw_grenade`] stays under clippy's argument-count gate (the melee module's
/// `LosGrids` precedent).
///
/// A blind throw is offered when the selection wields a [`TrajectoryStyle::Arc`] weapon
/// (resolved `selection -> Wields -> the RANGED weapon entity -> its TrajectoryStyle`,
/// EXCLUDING the melee weapon / fists via `With<MeleeWeapon>`) AND a target cell is
/// hovered ([`InspectTarget::hovered`]). All reads are read-only (the offer layer never
/// mutates the sim / input). [`InspectTarget`] is `Option` so the system stays valid
/// before the picker inserts it (`bevy-traps.md` #1); the throw offer then simply has
/// no target cell.
#[derive(SystemParam)]
pub(in crate::states::running::game::battlescape) struct ThrowReads<'w, 's> {
    /// The live hovered cell (the picker's per-update write) — the BLIND throw's target
    /// CELL. `Option` so the offer stays valid before the picker inserts the resource.
    inspect: Option<Res<'w, InspectTarget>>,
    /// The `selection -> Wields -> weapon entities` relationship read — the throw
    /// resolves the RANGED weapon through this (the sim's `ranged_weapon` idiom).
    wields:  Query<'w, 's, &'static Wields>,
    /// The `With<MeleeWeapon>` filter query backing the `is_melee` closure
    /// `ranged_weapon` takes — so the throw reads the RANGED weapon's trajectory, never
    /// the melee / fists entity.
    melee:   Query<'w, 's, (), With<MeleeWeapon>>,
    /// Each weapon entity's [`TrajectoryStyle`] — the throw is offered only when the
    /// resolved ranged weapon's style is [`Arc`](TrajectoryStyle::Arc).
    styles:  Query<'w, 's, &'static TrajectoryStyle>,
}

impl ThrowReads<'_, '_> {
    /// The target CELL a THROW would lob at (GTW-546), or [`None`] when `actor` wields
    /// no [`Arc`](TrajectoryStyle::Arc) weapon or no cell is hovered.
    ///
    /// Resolves `actor -> Wields -> the RANGED weapon entity` (EXCLUDING the melee /
    /// fists entity via the `With<MeleeWeapon>` filter — the sim's `ranged_weapon`
    /// idiom), reads that weapon's [`TrajectoryStyle`], and offers the hovered cell
    /// only when the style is `Arc`. The throw is BLIND — no adjacency / LOS gate — so
    /// the ONLY offer condition is "wields an `Arc` weapon and the cursor is over a
    /// cell". The sim's `dispatch_throw_grenade` re-gate (loaded round + affords
    /// `ThrowTu`) is the authoritative check; this only decides what to OFFER.
    fn throw_target(&self, actor: Entity) -> Option<CellLevel> {
        let hovered = self.inspect.as_ref()?.hovered()?;
        let wields = self.wields.get(actor).ok()?;
        let weapon = wields.ranged_weapon(|entity| self.melee.get(entity).is_ok())?;
        let style = self.styles.get(weapon).ok()?;
        style.is_arc().then_some(hovered)
    }
}

/// OFFERS the Throw Grenade act: the hovered target cell, when the selection wields a
/// [`TrajectoryStyle::Arc`] weapon and a cell is hovered — or nothing (GTW-546).
///
/// A BLIND lob — no adjacency / LOS gate. F4 is PLAYER-ONLY, and the actor gate mirrors
/// the old shared resolve (the selection must carry `Position` + `Faction` to be an
/// actor at all — a filter-only query). The sim's `dispatch_throw_grenade` re-gate + TU
/// / magazine spend are authoritative; this only decides what to OFFER. Writes
/// [`ContextualOffer`] via `set_if_neq` (change-detection hygiene).
pub(in crate::states::running::game::battlescape) fn offer_throw_grenade(
    selected: Res<SelectedShooter>,
    actors: Query<(), (With<Position>, With<Faction>)>,
    throw: ThrowReads,
    mut offer: ResMut<ContextualOffer<ThrowGrenadeAct>>,
) {
    let target = (**selected)
        .filter(|actor| actors.get(*actor).is_ok())
        .and_then(|actor| throw.throw_target(actor));
    offer.set_if_neq(ContextualOffer::new(target));
}
