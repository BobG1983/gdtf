//! The per-ganger card projection (GTW-738, the T5 view path).
//!
//! [`ganger_views`] walks the [`SnapshotWorld`]'s ganger query, drops the DEAD (a
//! `GangerView` is minted per LIVING ganger — [`Alive`](gdtf_battle_sim::ganger::LifeState::Alive)
//! or [`Downed`](gdtf_battle_sim::ganger::LifeState::Downed)), and projects each into a
//! [`GangerView`] card. The weapon read ([`weapon_view`]) keys the ganger's RANGED weapon
//! (excluding the melee weapon) exactly as `NetIntent::Fire`'s resolver does, so the
//! indexed fire-mode list a client reads is the one it fires by. The injury projection
//! ([`injury_summary_net`]) mirrors the durable ledger, dropping the frozen effect
//! magnitudes (balance data never crosses the wire).

use gdtf_battle_sim::{
    ganger::LifeState,
    injuries::InflictedInjuries,
    weapon::{FireMode, WeaponName},
};
use gdtf_qa_protocol::{
    ids::{FireModeIndex, GangerToken},
    intent::AimNet,
    view::{
        FireModeLabel, FireModeView, GangerNameNet, GangerView, HpMaxNet, HpNet, InjuryEntryNet,
        InjuryNameNet, InjurySummaryNet, TuMaxNet, TuNet, WeaponNameNet, WeaponView, WoundsMaxNet,
        WoundsNet,
    },
};

use super::{
    map::{
        body_part_net, cell_level_net, facing_net, faction_net, life_net, severity_net, stance_net,
    },
    read::SnapshotWorld,
};

/// Project every LIVING ganger into its [`GangerView`] card (the DEAD are dropped — a
/// corpse is off the living roster; its [`LifeState`] still rides the card for the
/// Downed-but-alive case).
pub(super) fn ganger_views(world: &SnapshotWorld) -> Vec<GangerView> {
    world
        .gangers
        .iter()
        .filter(|row| !matches!(row.life, LifeState::Dead))
        .map(|row| {
            // Resolve the RANGED weapon (excluding the melee weapon) exactly as
            // `NetIntent::Fire`'s resolver does, so the indexed list the client reads is
            // the list it fires by. An unarmed / not-yet-related ganger (no `Wields`)
            // resolves to `None` → an empty weapon view.
            let ranged = row.wields.and_then(|wields| {
                wields.ranged_weapon(|entity| world.melee_marker.get(entity).is_ok())
            });
            let weapon_parts = ranged.and_then(|entity| world.weapons.get(entity).ok());
            GangerView {
                token:      GangerToken::new(row.entity.to_bits()),
                name:       GangerNameNet::new((**row.name).clone()),
                faction:    faction_net(*row.faction),
                position:   cell_level_net(**row.position),
                facing:     facing_net(**row.facing),
                aiming:     AimNet::new(**row.aiming),
                stance:     stance_net(**row.stance),
                life:       life_net(*row.life),
                hp:         HpNet::new(**row.hp),
                hp_max:     HpMaxNet::new(**row.hp_max),
                wounds:     WoundsNet::new(**row.wounds),
                wounds_max: WoundsMaxNet::new(**row.wounds_max),
                tu:         TuNet::new(**row.tu),
                tu_max:     TuMaxNet::new(**row.tu_max),
                injuries:   injury_summary_net(row.injuries),
                weapon:     weapon_view(weapon_parts),
            }
        })
        .collect()
}

/// Project a resolved wielded ranged weapon into its [`WeaponView`] — the name plus the
/// authored fire modes flattened to `(index, label)` pairs.
///
/// The `index` is the position in the authored mode list — exactly the
/// [`FireModeIndex`] a [`Fire`](gdtf_qa_protocol::intent::NetIntent::Fire) selects; the
/// label is the mode's [`Display`](std::fmt::Display) string. An UNARMED ganger (no ranged
/// weapon resolved) yields an empty view (empty name, no modes) — the card is never
/// optional.
fn weapon_view(parts: Option<(&WeaponName, &FireMode)>) -> WeaponView {
    let Some((name, fire_mode)) = parts else {
        return WeaponView::new(WeaponNameNet::new(String::new()), Vec::new());
    };
    let modes = fire_mode
        .iter()
        .enumerate()
        .map(|(index, spec)| {
            FireModeView::new(
                FireModeIndex::new(u32::try_from(index).unwrap_or(u32::MAX)),
                FireModeLabel::new(spec.kind.to_string()),
            )
        })
        .collect();
    WeaponView::new(WeaponNameNet::new((**name).clone()), modes)
}

/// Project a ganger's durable injury ledger into its [`InjurySummaryNet`] — the ordered
/// name / struck-part / severity entries, dropping the frozen effect magnitudes (balance
/// data never crosses the wire).
fn injury_summary_net(injuries: &InflictedInjuries) -> InjurySummaryNet {
    let entries = injuries
        .gained()
        .iter()
        .map(|gained| {
            InjuryEntryNet::new(
                InjuryNameNet::new((*gained.name).clone()),
                body_part_net(gained.part),
                severity_net(gained.severity),
            )
        })
        .collect();
    InjurySummaryNet::new(entries)
}
