//! Melee against cover / structure cells.

use bevy::prelude::{MessageWriter, Query};

use super::{MeleeWorld, snapshot::AttackerSnapshot};
use crate::{
    acts::{downed::is_8_adjacent, request::MeleeResolved},
    cover::CoverEvent,
    ganger::{Position, Tu},
    melee::resolve_structural_melee,
    metric::CellLevel,
    occupancy_sync::CoverDestroyed,
    tu::spend_tu,
};

/// Spend TU and smash adjacent cover/structure.
pub(super) fn resolve_structure_melee(
    attacker: &AttackerSnapshot<'_>,
    at: CellLevel,
    tu_q: &mut Query<&mut Tu>,
    world: &mut MeleeWorld,
    resolved: &mut MessageWriter<MeleeResolved>,
    cover_destroyed: &mut MessageWriter<CoverDestroyed>,
    deaths: &mut MessageWriter<crate::effects::on_death::OnDeathOccurred>,
) {
    if !*is_8_adjacent(attacker.position, Position::new(at)) {
        return;
    }

    let prototype = world
        .cover
        .peek(&at)
        .copied()
        .unwrap_or(STRUCTURE_SMASH_FALLBACK);

    let Ok(mut attacker_tu) = tu_q.get_mut(attacker.entity) else {
        return;
    };
    spend_tu(&mut attacker_tu, attacker.tu_cost);

    let event = resolve_structural_melee(
        attacker.weapon,
        &prototype,
        at,
        &mut world.cover,
        &world.tuning,
    );

    if let CoverEvent::Destroyed(cell) = event {
        cover_destroyed.write(CoverDestroyed::new(cell));
        deaths.write(crate::effects::on_death::OnDeathOccurred::cover(cell));
    }
    resolved.write(MeleeResolved::new(at, attacker.strike_damage_type));
}

const STRUCTURE_SMASH_FALLBACK: crate::cover::CoverEntry = crate::cover::CoverEntry::seeded(
    crate::cover::CoverHp::new(1),
    crate::cover::HeightBand::Low,
    crate::armor::ArmorProtection::new(0),
    crate::armor::ArmorHardness::new(0),
);
