//! Melee against cover / structure cells.

use super::{
    MeleeCombatants, MeleeOutcomes, MeleeWorld,
    cost::{MeleeReach, can_melee, structure_stands},
    snapshot::AttackerSnapshot,
};
use crate::{
    acts::request::MeleeResolved, cover::CoverEvent, melee::resolve_structural_melee,
    metric::CellLevel, occupancy::TerrainKind, occupancy_sync::TerrainPieceDestroyed, tu::spend_tu,
};

/// Spend TU and smash adjacent cover/structure.
pub(super) fn resolve_structure_melee(
    attacker: &AttackerSnapshot<'_>,
    at: CellLevel,
    combatants: &mut MeleeCombatants,
    world: &mut MeleeWorld,
    outcomes: &mut MeleeOutcomes,
) {
    let standing = structure_stands(&world.cover, &world.occupancy, at);
    if !*can_melee(attacker.reach(), MeleeReach::structure(at, standing)) {
        return;
    }

    let prototype = match world.cover.peek(&at).copied() {
        Some(entry) => entry,
        // Empty ground holds nothing to smash, so the fallback must not mint an entry for it.
        None if world.occupancy.terrain(&at) == TerrainKind::Open => return,
        None => STRUCTURE_SMASH_FALLBACK,
    };

    let Ok(mut attacker_tu) = combatants.tu.get_mut(attacker.entity) else {
        return;
    };
    if spend_tu(&mut attacker_tu, attacker.tu_cost).is_err() {
        return;
    }

    let event = resolve_structural_melee(
        attacker.weapon,
        &prototype,
        at,
        &mut world.cover,
        &world.tuning,
    );

    if let CoverEvent::Destroyed(cell) = event {
        outcomes
            .cover
            .write(TerrainPieceDestroyed::new(cell, prototype.kind));
        outcomes
            .deaths
            .write(crate::effects::on_death::OnDeathOccurred::cover(cell));
    }
    outcomes
        .resolved
        .write(MeleeResolved::new(at, attacker.strike_damage_type));
}

const STRUCTURE_SMASH_FALLBACK: crate::cover::CoverEntry = crate::cover::CoverEntry::seeded(
    crate::cover::CoverHp::new(1),
    crate::cover::HeightBand::Low,
    crate::armor::ArmorProtection::new(0),
    crate::armor::ArmorHardness::new(0),
    crate::terrain::entity::TerrainPieceKind::Cover,
);
