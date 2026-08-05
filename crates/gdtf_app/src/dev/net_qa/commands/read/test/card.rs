//! Which world value lands in which roster-card field, and that the drawn values win.

use bevy::prelude::*;
use gdtf_battle_presenter::DrawnVitals;
use gdtf_battle_sim::{
    act_log::VitalsFacts,
    armor::BodyPart,
    ganger::{GangerName, Hp, HpMax, Stance, StanceKind, Tu, TuMax, Wounds, WoundsMax},
    inflicted_wound::{InflictedWound, InflictedWounds},
    injuries::InflictedInjuries,
    prelude::Faction,
    severity::Severity,
};

use crate::{
    dev::net_qa::{
        commands::read::shown::ganger_card,
        wire::{
            act_payload::StanceNet,
            cell::{CellLevelNet, CellNet, CellXNet, CellYNet, LevelNet},
            roster::{FactionNet, GangerCardNet, GangerNameNet},
            token::GangerToken,
            vitals::{HpMaxNet, HpNet, TuMaxNet, TuNet, WoundsMaxNet, WoundsNet},
        },
    },
    states::running::game::battlescape::stat_block::StatBlockData,
};

/// The cell the caller says the sprite is drawn on, distinct in both axes and off level zero.
const DRAWN_AT: CellLevelNet = CellLevelNet::new(
    CellNet::new(CellXNet::new(13), CellYNet::new(4)),
    LevelNet::new(2),
);

/// Live values on the sim components, all distinct so a swapped field is visible.
const LIVE_TU: Tu = Tu::new(11);
const LIVE_HP: Hp = Hp::new(22);
const LIVE_WOUNDS: Wounds = Wounds::new(3);

/// Drawn values, distinct from every live one so "reports the drawn value" is provable.
const DRAWN_TU: Tu = Tu::new(44);
const DRAWN_HP: Hp = Hp::new(55);
const DRAWN_WOUNDS: Wounds = Wounds::new(6);

const TU_MAX: TuMax = TuMax::new(77);
const HP_MAX: HpMax = HpMax::new(88);
const WOUNDS_MAX: WoundsMax = WoundsMax::new(9);

const GANG: Faction = Faction::new(2);

/// A ganger carrying the live values above, plus whatever `extra` adds.
fn a_ganger(world: &mut World, extra: impl Bundle) -> Entity {
    world
        .spawn((
            GangerName::new("Vex".to_owned()),
            GANG,
            Stance::new(StanceKind::Crouching),
            LIVE_TU,
            TU_MAX,
            LIVE_HP,
            HP_MAX,
            LIVE_WOUNDS,
            WOUNDS_MAX,
            extra,
        ))
        .id()
}

/// Build the card the same way both handlers do: from a `StatBlockData` query row.
fn card_for(world: &mut World, entity: Entity) -> GangerCardNet {
    let mut rows = world.query::<StatBlockData>();
    let Ok(row) = rows.get(world, entity) else {
        unreachable!("the ganger this test just spawned matches the stat block's own query");
    };
    ganger_card(entity, DRAWN_AT, &row)
}

fn drawn(tu: Tu, hp: Hp, wounds: Wounds, inflicted: InflictedWounds) -> DrawnVitals {
    DrawnVitals::new(VitalsFacts::new(
        tu,
        hp,
        wounds,
        inflicted,
        InflictedInjuries::default(),
    ))
}

#[test]
fn a_card_carries_the_identity_the_world_holds() {
    let mut world = World::new();
    let entity = a_ganger(&mut world, InflictedWounds::default());
    let card = card_for(&mut world, entity);

    assert_eq!(
        card.token,
        GangerToken::new(entity.to_bits()),
        "the card names the ganger it was built from: {card:?}",
    );
    assert_eq!(
        card.name,
        Some(GangerNameNet::new("Vex".to_owned())),
        "the card carries the name the stat block would draw: {card:?}",
    );
    assert_eq!(
        card.faction,
        FactionNet::from_sim(GANG),
        "the card carries the gang the ganger fights for: {card:?}",
    );
    assert_eq!(
        card.at, DRAWN_AT,
        "the card says where the sprite stands, so a reader can place it on the map without \
         asking a second command: {card:?}",
    );
    assert_eq!(
        card.stance,
        StanceNet::Crouching,
        "the card carries the posture, not a default: {card:?}",
    );
    assert_eq!(
        card.tu_max,
        TuMaxNet::new(*TU_MAX),
        "TuMax lands in tu_max: {card:?}",
    );
    assert_eq!(
        card.hp_max,
        HpMaxNet::new(*HP_MAX),
        "HpMax lands in hp_max: {card:?}",
    );
    assert_eq!(
        card.wounds_max,
        WoundsMaxNet::new(*WOUNDS_MAX),
        "WoundsMax lands in wounds_max: {card:?}",
    );
}

#[test]
fn a_ganger_with_nothing_drawn_yet_reports_its_live_vitals() {
    let mut world = World::new();
    let entity = a_ganger(&mut world, InflictedWounds::default());
    let card = card_for(&mut world, entity);

    assert_eq!(
        (card.tu, card.hp, card.wounds),
        (
            TuNet::new(*LIVE_TU),
            HpNet::new(*LIVE_HP),
            WoundsNet::new(*LIVE_WOUNDS),
        ),
        "before the presenter has drawn anything the card falls back to the sim's own \
         components, each in its own field: {card:?}",
    );
}

#[test]
fn a_card_reports_the_drawn_vitals_the_stat_block_is_showing() {
    let mut world = World::new();
    let mut taken = InflictedWounds::default();
    taken.record(InflictedWound::new(Severity::Major, BodyPart::LeftLeg));
    let entity = a_ganger(
        &mut world,
        (
            InflictedWounds::default(),
            drawn(DRAWN_TU, DRAWN_HP, DRAWN_WOUNDS, taken),
        ),
    );
    let card = card_for(&mut world, entity);

    assert_eq!(
        (card.tu, card.hp, card.wounds),
        (
            TuNet::new(*DRAWN_TU),
            HpNet::new(*DRAWN_HP),
            WoundsNet::new(*DRAWN_WOUNDS),
        ),
        "while an act plays out the card reports what the stat block is drawing, not the sim \
         values ahead of it: {card:?}",
    );
    assert_eq!(
        card.wounds_taken.len(),
        1,
        "the wound list comes from the drawn vitals too, so it lags with the rest of the \
         card: {card:?}",
    );
}

#[test]
fn a_ganger_missing_its_maxima_reports_its_current_values_as_full() {
    let mut world = World::new();
    let entity = world
        .spawn((
            GANG,
            Stance::new(StanceKind::Standing),
            LIVE_TU,
            TU_MAX,
            LIVE_HP,
            LIVE_WOUNDS,
        ))
        .id();
    let card = card_for(&mut world, entity);

    assert_eq!(card.name, None, "an unnamed ganger has no name: {card:?}");
    assert_eq!(
        (card.hp_max, card.wounds_max),
        (HpMaxNet::new(*LIVE_HP), WoundsMaxNet::new(*LIVE_WOUNDS),),
        "with no HpMax or WoundsMax the card reads full, matching the bar the stat block \
         draws: {card:?}",
    );
}
