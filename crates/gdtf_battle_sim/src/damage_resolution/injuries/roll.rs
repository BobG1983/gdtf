//! Pick an injury from the weighted tables for a body part, context, and severity.

use bevy::log::warn_once;

use super::{DamageContext, InjuryTables, RolledInjury, WeightedInjuryTable};
use crate::{armor::BodyPart, injuries::InjuryRegistry, rng::InjuryRng, severity::Severity};

/// Roll one injury, or `None` when severity is None/Fatal or the table is empty.
#[must_use]
pub fn roll_injury(
    part: BodyPart,
    severity: Severity,
    context: DamageContext,
    tables: &InjuryTables,
    registry: &InjuryRegistry,
    rng: &mut InjuryRng,
) -> Option<RolledInjury> {
    match severity {
        Severity::None | Severity::Fatal => return None,
        Severity::Minor | Severity::Major | Severity::Critical => {}
    }

    let table = tables.table(part, context, severity);

    let pick = pick_key(table, rng);

    let Some(picked) = pick else {
        warn_once!(
            "injury roll for ({part:?}, {context:?}, {severity:?}) found no rollable content \
             (empty/missing weighting bucket); the InjuryRng draw was taken and discarded"
        );
        return None;
    };

    let Some(def) = registry.def(&picked) else {
        warn_once!(
            "injury roll picked key `{}` for ({part:?}, {context:?}, {severity:?}) but it is \
             not in the InjuryRegistry; no injury inflicted (the draw was taken)",
            &*picked
        );
        return None;
    };

    Some(RolledInjury::new(
        def.name.clone(),
        part,
        def.severity,
        def.effects.clone(),
        def.popup_text.clone(),
        def.log_text.clone(),
        def.inspect_text.clone(),
    ))
}

fn pick_key(table: Option<&WeightedInjuryTable>, rng: &mut InjuryRng) -> Option<super::InjuryName> {
    let rows: &[super::WeightedInjuryEntry] = table.map_or(&[], |t| t);
    let total: u64 = rows.iter().map(|row| u64::from(*row.weight)).sum();

    let upper = total.saturating_sub(1);
    let draw: u64 = rng.random_range(0..=upper);

    if total == 0 {
        return None;
    }

    let mut cumulative: u64 = 0;
    for row in rows {
        cumulative += u64::from(*row.weight);
        if draw < cumulative {
            return Some(row.injury.clone());
        }
    }

    None
}
