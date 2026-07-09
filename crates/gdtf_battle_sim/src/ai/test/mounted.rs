//! GTW-673 — the AI ENGAGE gate resolves the weapon it would actually FIRE through the ONE
//! shared preference rule ([`Wields::firing_weapon`](crate::weapon::Wields::firing_weapon),
//! mounted-first, GTW-660): a mounted enemy engages on the MOUNT, not its carried gun.
//!
//! Driven through the REAL brain in a headless [`SimActsPlugin`](crate::acts::SimActsPlugin)
//! app (no mocks), with the seeded per-test RNG the [`support`](super::support) harness wires.

use bevy::prelude::{Entity, World};

use super::support::{
    ENEMY, PLAYER, active_of, brain_app, drain_fires, ground, place_occupant, spawn_combatant,
    tu_of,
};
use crate::{
    ganger::Direction,
    magazine::{Magazine, ReloadTu},
    metric::Cell,
    weapon::{
        Accuracy, BaseSpread, DamageProfile, DamageType, FatalBias, FireMode, FireModeSpec,
        Handedness, HandlingProfile, Kickback, MagazineSize, ModeConeMult, ModeKind, ModeShots,
        ModeTuPercent, MountedWeapon, Shove, Stable, WeaponBundle, WeaponDamage, WeaponName,
        WeaponPunch, WeaponShred, WieldedBy,
    },
};

/// A generous frame cap — the enemy turn is finite (TU + ammo are finite), so it returns
/// control to the player well within this (the `brain.rs` scenarios' cap).
const FRAME_CAP: usize = 80;

/// Relate a LOADED, [`MountedWeapon`]-marked weapon entity to an already-spawned `ganger` —
/// the emplacement's bolted-down gun a seated ganger would fire (GTW-543). The
/// [`WieldedBy`] hook populates the ganger's `Wields` synchronously in a bare-world spawn
/// (the `support` harness note), so the very next brain tick resolves it; because the
/// carried weapon is related FIRST (by `spawn_combatant`) and this mount SECOND, the mount
/// is the LATER entry in the insertion-ordered `Wields` collection — the OLD ranged-only
/// gate (first non-melee wielded entity) therefore resolves the carried gun, not this mount.
///
/// Single consumer (the [`ai_mounted_enemy_engages_on_the_mount`] test), so it lives local to
/// its consumer (module-layout rule 6). Arbitrary magnitudes — never shipped tuning.
fn man_loaded_mount(world: &mut World, ganger: Entity, ammo: u16) {
    let mode = FireModeSpec::new(
        ModeKind::Single,
        ModeConeMult::new(1.0),
        ModeTuPercent::new(0.2),
        ModeShots::new(1),
    );
    let bundle = WeaponBundle::new(
        WeaponName::new("test-mount".to_owned()),
        BaseSpread::new(0.05),
        Accuracy::new(2.0),
        Kickback::new(0.2),
        FatalBias::new(0.0),
        DamageProfile::new(
            WeaponDamage::new(40),
            WeaponPunch::new(20),
            WeaponShred::new(10),
            DamageType::Kinetic,
        ),
        HandlingProfile::new(
            Magazine::new(ammo, MagazineSize::new(30), ReloadTu::new(12)),
            FireMode::new(vec![mode]),
            Stable::new(true),
            Shove::new(false),
            Handedness::OneHanded,
        ),
    );
    world.spawn((WieldedBy::new(ganger), bundle, MountedWeapon));
}

/// GTW-673 C2 — an AI ganger seated on an armed emplacement, whose CARRIED gun cannot fire
/// (an empty magazine), must ENGAGE on the MOUNT (the weapon `dispatch_fire` would actually
/// resolve) — never fall silent on the unfireable carried gun.
///
/// The enemy at (2,5) faces East at a player occupant at (8,5) — clear LOS, in view range,
/// in arc, the IDENTICAL geometry that fires in `brain.rs` scenario 1 — but carries an EMPTY
/// gun (`ammo = 0`) AND wields a LOADED mount. The engage gate must route through
/// [`Wields::firing_weapon`](crate::weapon::Wields::firing_weapon) (mounted-first), resolve
/// the loaded mount, and emit a real [`FireRequested`](crate::acts::FireRequested) at the
/// player.
///
/// Pin-discriminating against the OLD ranged-only gate: it resolved the FIRST non-melee
/// wielded entity — the empty carried gun related first — whose `can_fire` fails on the empty
/// magazine, so the brain would NEVER engage (no fire at the player). The assertion below
/// therefore flips red against the pre-fix code and green once the gate prefers the mount.
/// Because the only fireable weapon is the mount, a `FireRequested` at the player IS proof the
/// gate resolved the mount.
#[test]
fn ai_mounted_enemy_engages_on_the_mount() {
    let mut app = brain_app();
    let player_at = ground(8, 5);
    // The enemy carries an EMPTY gun (ammo = 0) — the OLD ranged-only gate would resolve this
    // first-related, unfireable weapon and refuse to engage.
    let enemy = spawn_combatant(
        app.world_mut(),
        ground(2, 5),
        ENEMY,
        Direction::East,
        100,
        0,
    );
    // Man the emplacement: a LOADED mount, related SECOND (later in Wields), the weapon the
    // mounted-first preference resolves and dispatch would fire.
    man_loaded_mount(app.world_mut(), enemy, 6);
    let player = spawn_combatant(app.world_mut(), player_at, PLAYER, Direction::West, 100, 2);
    place_occupant(&mut app, player_at, player);

    let mut fires = Vec::new();
    for _ in 0..FRAME_CAP {
        app.update();
        fires.extend(drain_fires(&mut app));
        if active_of(&app) == PLAYER {
            break;
        }
    }

    // ENGAGE-ON-THE-MOUNT: the enemy emitted a real FireRequested at the player's exact
    // (8,5,0) cell — only possible by resolving the LOADED mount (the carried gun is empty).
    assert!(
        fires.iter().any(|f| f.shooter == enemy
            && f.target_cell == Cell::new(8, 5)
            && *f.target_level == 0),
        "a mounted AI enemy with an empty carried gun must engage on the loaded mount at the \
         player's (8,5,0) cell: {fires:?}",
    );
    assert!(
        tu_of(&app, enemy) < 100,
        "the enemy spent TU engaging on the mount: {}",
        tu_of(&app, enemy),
    );
}
