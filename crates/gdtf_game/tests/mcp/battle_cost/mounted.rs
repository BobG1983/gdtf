//! Which weapon a `battle.cost {Fire}` quote is priced against while a mount is wielded.

use bevy::{app::App, ecs::entity::Entity, prelude::World};
use gdtf_battle_sim::{
    magazine::{LoadedRounds, Magazine, ReloadTu},
    weapon::{
        FireMode, FireModeSpec, Handedness, MagazineSize, MeleeWeapon, ModeConeMult, ModeKind,
        ModeShots, ModeTuPercent, MountedWeapon, TrajectoryStyle, WieldedBy, Wields,
    },
};
use gdtf_game::qa_wire::{
    cell::CellLevelNet,
    cost::{CostActNet, CostRefusalNet},
    misc::ModeKindNet,
    vitals::TuNet,
};

use super::support::{CostBody, cost_body, cost_calls, fire_cost, settle};
use crate::{
    battle_reads::a_player_ganger,
    command_exchange::exchange_in_battle,
    socket_support::{TestError, TestResult},
};

/// A mount magazine that takes thirty rounds and takes twelve TU to refill.
const MOUNT_MAGAZINE: fn(u16) -> Magazine = |rounds| {
    Magazine::new(
        LoadedRounds::new(rounds),
        MagazineSize::new(30),
        ReloadTu::new(12),
    )
};

/// One fire mode of `kind` charging `tu_percent` of the shooter's pool.
const fn mode(kind: ModeKind, tu_percent: f32) -> FireModeSpec {
    FireModeSpec::new(
        kind,
        ModeConeMult::new(1.0),
        ModeTuPercent::new(tu_percent),
        ModeShots::new(1),
    )
}

/// The actor, where it stands, and the specs the two weapons were given.
struct Bench {
    actor:   Entity,
    at:      CellLevelNet,
    carried: FireModeSpec,
    mount:   FireModeSpec,
}

/// The gun in the actor's hands, found the way `Wields::ranged_weapon` finds it.
fn carried_gun(world: &World, actor: Entity) -> Option<Entity> {
    world
        .get_entity(actor)
        .ok()?
        .get::<Wields>()?
        .ranged_weapon(|entity| {
            world
                .get_entity(entity)
                .is_ok_and(|row| row.contains::<MeleeWeapon>())
        })
}

/// Wield a mounted weapon on `actor`, as manning an emplacement does.
fn mount_on(app: &mut App, actor: Entity, modes: FireMode, rounds: u16) -> Entity {
    app.world_mut()
        .spawn((
            WieldedBy::new(actor),
            MountedWeapon,
            modes,
            MOUNT_MAGAZINE(rounds),
            Handedness::OneHanded,
            TrajectoryStyle::Straight,
        ))
        .id()
}

/// Give the actor a carried gun offering `modes`, and a mount holding `rounds` offering `mount`.
///
/// The two mode sets are deliberately different, so which weapon answered is readable off a quote.
fn bench(
    app: &mut App,
    carried: &[FireModeSpec],
    mount: FireModeSpec,
    rounds: u16,
) -> Option<Bench> {
    let (actor, at) = a_player_ganger(app)?;
    let gun = carried_gun(app.world(), actor)?;
    app.world_mut()
        .entity_mut(gun)
        .insert(FireMode::new(carried.to_vec()));
    mount_on(app, actor, FireMode::new(vec![mount]), rounds);
    Some(Bench {
        actor,
        at,
        carried: *carried.first()?,
        mount,
    })
}

/// One shot at `at` per mode kind, in the order the kinds are listed.
fn fire_at(at: CellLevelNet, kinds: &[ModeKind]) -> Vec<CostActNet> {
    kinds
        .iter()
        .map(|kind| CostActNet::Fire {
            target: at,
            mode:   ModeKindNet::from_sim(*kind),
        })
        .collect()
}

/// The bodies of the replies, one per act asked.
fn bodies(
    replies: Vec<cobalt_mcp_protocol::message::McpResponse>,
) -> Result<Vec<CostBody>, TestError> {
    replies.into_iter().map(cost_body).collect()
}

#[test]
fn a_fire_quote_is_refused_when_the_mount_is_dry_and_the_carried_gun_is_loaded() -> TestResult {
    let mut planned: Option<Bench> = None;
    let (app, replies) = exchange_in_battle(|app| {
        settle(app);
        let Some(set) = bench(
            app,
            &[mode(ModeKind::Single, 0.2)],
            mode(ModeKind::Single, 0.6),
            0,
        ) else {
            return Vec::new();
        };
        let calls = cost_calls(set.actor, &fire_at(set.at, &[ModeKind::Single]));
        planned = Some(set);
        calls
    })?;
    let Some(set) = planned else {
        return Err("a running battle must field a player ganger holding a ranged weapon".into());
    };
    let [body] = bodies(replies)?.try_into().map_err(|found| {
        TestError::from(format!(
            "the one act asked must come back with one reply: {found:?}"
        ))
    })?;

    assert!(
        carried_rounds(&app, set.actor).is_some_and(|rounds| *rounds > 0),
        "precondition: the gun in the actor's hands is loaded, so only the dry mount can refuse",
    );
    assert_eq!(
        body.refusal,
        Some(CostRefusalNet::ActNotAllowed),
        "a dry mount must refuse the quote even with a loaded gun in the actor's hands: {body:?}",
    );
    assert!(!*body.legal, "a refused act is not legal: {body:?}");

    let Some(priced_at) = fire_cost(&app, set.actor, &set.mount) else {
        return Err("the same world must still price the mount's own mode".into());
    };
    assert_eq!(
        body.cost,
        Some(TuNet::new(*priced_at)),
        "the refused quote is still priced off the MOUNT's spec, not the carried gun's: {body:?}",
    );
    Ok(())
}

#[test]
fn a_fire_quote_follows_the_mounted_weapon_not_the_carried_one() -> TestResult {
    let mut planned: Option<Bench> = None;
    let (app, replies) = exchange_in_battle(|app| {
        settle(app);
        let Some(set) = bench(
            app,
            &[mode(ModeKind::Single, 0.2), mode(ModeKind::Burst, 0.4)],
            mode(ModeKind::Single, 0.7),
            30,
        ) else {
            return Vec::new();
        };
        let calls = cost_calls(
            set.actor,
            &fire_at(set.at, &[ModeKind::Single, ModeKind::Burst]),
        );
        planned = Some(set);
        calls
    })?;
    let Some(set) = planned else {
        return Err("a running battle must field a player ganger holding a ranged weapon".into());
    };
    let [single, burst] = bodies(replies)?.try_into().map_err(|found| {
        TestError::from(format!(
            "both acts asked must come back with a reply each: {found:?}"
        ))
    })?;

    let (Some(by_mount), Some(by_carried)) = (
        fire_cost(&app, set.actor, &set.mount),
        fire_cost(&app, set.actor, &set.carried),
    ) else {
        return Err("the same world must still price both weapons' Single modes".into());
    };
    assert_ne!(
        by_mount, by_carried,
        "the case only means something while the two weapons charge different TU for Single",
    );
    assert_eq!(
        single.cost,
        Some(TuNet::new(*by_mount)),
        "the Single quote must be the MOUNT's price, not the carried gun's: {single:?}",
    );

    assert_eq!(
        burst.refusal,
        Some(CostRefusalNet::ActNotAllowed),
        "Burst is offered by the carried gun and NOT by the mount, so the quote must refuse it: \
         {burst:?}",
    );
    assert_eq!(
        burst.cost, None,
        "a mode the fired weapon does not offer carries no price: {burst:?}",
    );
    Ok(())
}

/// How many rounds the gun in the actor's hands is holding, off the live world.
fn carried_rounds(app: &App, actor: Entity) -> Option<LoadedRounds> {
    let world = app.world();
    let gun = carried_gun(world, actor)?;
    world
        .get_entity(gun)
        .ok()?
        .get::<Magazine>()
        .map(Magazine::rounds)
}
