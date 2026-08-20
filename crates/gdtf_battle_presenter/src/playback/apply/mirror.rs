//! Mirror an act-log entry into the drawn components.

use bevy::{ecs::component::Mutable, prelude::*};
use gdtf_battle_sim::act_log::{ActDeed, ActEntry};

use super::{
    super::drawn::{DrawnLife, DrawnMagazine, DrawnPose, DrawnPosition, DrawnVitals},
    writers::DrawnWriters,
};

pub(super) fn mirror_drawn(entry: &ActEntry, drawn: &mut DrawnWriters) {
    let actor = entry.actor();
    match entry.deed() {
        ActDeed::PostureChanged { pose } => {
            set_drawn(&mut drawn.poses, actor, DrawnPose::new(*pose));
        }
        ActDeed::Stepped { position, .. } | ActDeed::MovedTo { position } => {
            set_drawn(&mut drawn.positions, actor, DrawnPosition::new(*position));
        }
        ActDeed::MagazineChanged { magazine } => {
            set_drawn(&mut drawn.magazines, actor, DrawnMagazine::new(*magazine));
        }
        ActDeed::VitalsChanged { vitals } => {
            set_drawn(&mut drawn.vitals, actor, DrawnVitals::new(vitals.clone()));
        }
        ActDeed::LifeChanged { to, .. } => {
            set_drawn(&mut drawn.lives, actor, DrawnLife::new(*to));
        }
        ActDeed::TurnBegan { .. }
        | ActDeed::MoveRefused { .. }
        | ActDeed::EnteredView { .. }
        | ActDeed::Fired { .. }
        | ActDeed::RoundResolved { .. }
        | ActDeed::Reloaded { .. }
        | ActDeed::Injured { .. }
        | ActDeed::Fell { .. }
        | ActDeed::Struck { .. }
        | ActDeed::DiedAt { .. }
        | ActDeed::Suppressed { .. }
        | ActDeed::ArmorBroke { .. }
        | ActDeed::DotStarted { .. }
        | ActDeed::FieldStarted { .. }
        | ActDeed::DotTicked { .. }
        | ActDeed::FieldTicked { .. }
        | ActDeed::BleedStarted
        | ActDeed::Bled
        | ActDeed::TerrainPieceSmashed { .. }
        | ActDeed::MeleeLanded { .. }
        | ActDeed::ThrowLanded { .. } => {}
    }
}

// Write a drawn component, leaving change detection alone when it already matches.
fn set_drawn<C: Component<Mutability = Mutable> + PartialEq>(
    drawn: &mut Query<&'static mut C>,
    actor: Entity,
    value: C,
) {
    if let Ok(mut current) = drawn.get_mut(actor) {
        current.set_if_neq(value);
    }
}
