use super::support::*;
use crate::weapon::WieldedBy;

/// The HP the neighbour of a corpse carrying its own effect starts on.
const NEIGHBOUR_HP: u16 = 10;

/// The flat drain that corpse's authored blast deals.
const BLAST_DAMAGE: u16 = 5;

#[test]
fn a_victims_own_on_death_fires_with_no_weapon_to_read_it_off() {
    let mut app = resolver_app();
    let dead = dead_body_with_own_on_death(
        &mut app,
        OnDeathEffect::Explode {
            hit_type:    HitType::Blast {
                radius: BlastRadius::new(1),
            },
            damage:      ExplodeDamage::new(BLAST_DAMAGE),
            damage_type: DamageType::Blast,
        },
    );
    let neighbour = app
        .world_mut()
        .spawn((
            Hp::new(NEIGHBOUR_HP),
            LifeState::Alive,
            Position::new(ground(6, 5)),
        ))
        .id();
    grid_with_occupant(&mut app, ground(6, 5), neighbour);

    app.world_mut()
        .write_message(OnDeathOccurred::new(dead, ground(5, 5)));
    app.update();

    assert_eq!(
        hp_of(&app, neighbour),
        NEIGHBOUR_HP - BLAST_DAMAGE,
        "the effect on the dying entity itself fans, so the neighbour loses the authored \
         {BLAST_DAMAGE}; its HP reads {}",
        hp_of(&app, neighbour),
    );
}

#[test]
fn explode_on_death_damages_an_adjacent_ganger() {
    let mut app = resolver_app();
    let dead = dead_ganger_with_on_death(
        &mut app,
        OnDeathEffect::Explode {
            hit_type:    HitType::Blast {
                radius: BlastRadius::new(1),
            },
            damage:      ExplodeDamage::new(5),
            damage_type: DamageType::Blast,
        },
    );
    let victim = app
        .world_mut()
        .spawn((Hp::new(10), LifeState::Alive, Position::new(ground(6, 5))))
        .id();
    grid_with_occupant(&mut app, ground(6, 5), victim);

    app.world_mut()
        .write_message(OnDeathOccurred::new(dead, ground(5, 5)));
    app.update();

    assert_eq!(
        hp_of(&app, victim),
        5,
        "the adjacent victim took the blast's flat 5 HP"
    );
    assert_eq!(
        life_of(&app, victim),
        LifeState::Alive,
        "a non-lethal blast leaves the victim alive"
    );
}

#[test]
fn explode_out_of_radius_ganger_is_untouched() {
    let mut app = resolver_app();
    let dead = dead_ganger_with_on_death(
        &mut app,
        OnDeathEffect::Explode {
            hit_type:    HitType::Blast {
                radius: BlastRadius::new(1),
            },
            damage:      ExplodeDamage::new(5),
            damage_type: DamageType::Blast,
        },
    );
    let far = app
        .world_mut()
        .spawn((Hp::new(10), LifeState::Alive, Position::new(ground(10, 10))))
        .id();
    grid_with_occupant(&mut app, ground(10, 10), far);

    app.world_mut()
        .write_message(OnDeathOccurred::new(dead, ground(5, 5)));
    app.update();

    assert_eq!(
        hp_of(&app, far),
        10,
        "a ganger outside the radius is untouched"
    );
}

#[test]
fn explode_chain_reaction_kills_then_terminates() {
    let mut app = resolver_app();
    let a = dead_ganger_with_on_death(
        &mut app,
        OnDeathEffect::Explode {
            hit_type:    HitType::Blast {
                radius: BlastRadius::new(1),
            },
            damage:      ExplodeDamage::new(100),
            damage_type: DamageType::Blast,
        },
    );
    let b_ganger = app
        .world_mut()
        .spawn((Hp::new(10), LifeState::Alive, Position::new(ground(6, 5))))
        .id();
    app.world_mut().spawn((
        Weapon,
        WieldedBy::new(b_ganger),
        OnDeath::new(vec![OnDeathEffect::Explode {
            hit_type:    HitType::Blast {
                radius: BlastRadius::new(1),
            },
            damage:      ExplodeDamage::new(100),
            damage_type: DamageType::Blast,
        }]),
    ));
    grid_with_occupant(&mut app, ground(6, 5), b_ganger);

    app.world_mut()
        .write_message(OnDeathOccurred::new(a, ground(5, 5)));
    app.update();

    assert_eq!(
        life_of(&app, b_ganger),
        LifeState::Dead,
        "the cascade killed B"
    );
    assert_eq!(hp_of(&app, b_ganger), 0, "B's HP was emptied by A's blast");
}
