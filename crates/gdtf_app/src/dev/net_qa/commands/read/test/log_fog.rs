//! Which lines and which identities the log window hands the gang doing the asking.

use bevy::prelude::Entity;
use gdtf_battle_sim::{
    act_log::{
        ActDeed, ActLog, ActProvenance, ActWitnesses, MagazineFacts, RecordedAct, WatchingFactions,
    },
    acts::{MoveRejection, ReloadOutcome, RoundCount},
    magazine::Magazine,
    prelude::{Cell, CellLevel, Level},
    resolve_hit::HpDamage,
    weapon::ModeKind,
};

use super::log_window::{ENEMY, PLAYER, args, watched_by, watched_unnamed};
use crate::dev::net_qa::{
    commands::read::log_read::window,
    wire::{
        deed::{ActDeedKindNet, ReloadOutcomeNet},
        log::ActProvenanceNet,
        token::GangerToken,
    },
};

/// Lines the alternating fixture writes, wide enough that a cap can cut inside it.
const ALTERNATING: u64 = 40;

/// Cap the alternating case asks for, cutting the window part-way through the log.
const CUTTING_CAP: u64 = 10;

/// The cell a deed-anchored deed carries, standing where the asking gang cannot see.
fn a_dark_cell() -> CellLevel {
    CellLevel::new(Cell::new(40, 40), Level::new(0))
}

/// The token an identity arrives as when it reaches the caller.
fn token_of(entity: Entity) -> GangerToken {
    GangerToken::new(entity.to_bits())
}

/// Two gangers with different tokens: one the player commands, one standing in the dark.
fn two_gangers() -> (Entity, Entity) {
    (
        Entity::from_raw_u32(7).unwrap_or(Entity::PLACEHOLDER),
        Entity::from_raw_u32(9).unwrap_or(Entity::PLACEHOLDER),
    )
}

/// A log whose lines alternate between the player watching and only the enemy watching.
fn an_alternating_log() -> ActLog {
    let mut log = ActLog::default();
    for line in 0..ALTERNATING {
        let witnesses = if line % 2 == 0 {
            watched_by(PLAYER)
        } else {
            watched_by(ENEMY)
        };
        log.append(RecordedAct::new(
            Entity::PLACEHOLDER,
            ActProvenance::Clock,
            ActDeed::TurnBegan { now_active: PLAYER },
            witnesses,
        ));
    }
    log
}

#[test]
fn the_cap_cuts_the_window_before_the_fog_filter_thins_it() {
    let log = an_alternating_log();
    let asked = format!("(cap:Some({CUTTING_CAP}))");
    let filtered = window(&log, &args(&asked), Some(PLAYER));
    let unfiltered = window(&log, &args("()"), Some(PLAYER));

    let sequences: Vec<u64> = filtered.entries.iter().map(|entry| *entry.seq).collect();
    assert!(
        sequences.is_sorted(),
        "dropping lines must leave the rest in sequence order: {sequences:?}",
    );
    assert_eq!(
        filtered.head, unfiltered.head,
        "filtering changes which lines come back, never where the log has got to: \
         {filtered:?} against {unfiltered:?}",
    );
    assert_eq!(
        u64::from(*filtered.dropped),
        ALTERNATING.saturating_sub(CUTTING_CAP),
        "`dropped` counts every line the CAP left before the window, kept or not — \
         expected {expected}, got {got} over {ALTERNATING} written with a cap of \
         {CUTTING_CAP}",
        expected = ALTERNATING.saturating_sub(CUTTING_CAP),
        got = *filtered.dropped,
    );
    assert_eq!(
        u64::try_from(filtered.entries.len()).unwrap_or(u64::MAX),
        CUTTING_CAP / 2,
        "half the capped window was watched by the enemy alone, so a filtered reply may \
         hold fewer lines than the cap: {filtered:?}",
    );
}

/// A log holding one deed of each touched-cell key, with the enemy hidden and the player lit.
fn a_log_of_every_key() -> (ActLog, Entity, Entity) {
    let (mine, hidden) = two_gangers();
    let dark = ActWitnesses::new(
        WatchingFactions::new([ENEMY]),
        WatchingFactions::new([ENEMY]),
    );
    let mut log = ActLog::default();

    log.append(RecordedAct::new(
        hidden,
        ActProvenance::AiTurn,
        ActDeed::Reloaded {
            outcome: ReloadOutcome::Reloaded,
        },
        dark.clone(),
    ));
    log.append(RecordedAct::new(
        mine,
        ActProvenance::Commanded,
        ActDeed::MoveRefused {
            reason: MoveRejection::Suppressed,
        },
        watched_by(PLAYER),
    ));
    log.append(RecordedAct::new(
        hidden,
        ActProvenance::AiTurn,
        ActDeed::Struck {
            target:    mine,
            hp_damage: HpDamage::new(3),
        },
        watched_unnamed(PLAYER),
    ));
    log.append(RecordedAct::new(
        hidden,
        ActProvenance::AiTurn,
        ActDeed::MagazineChanged {
            magazine: MagazineFacts::new(Magazine::default()),
        },
        dark.clone(),
    ));
    log.append(RecordedAct::new(
        hidden,
        ActProvenance::AiTurn,
        ActDeed::Suppressed { at: a_dark_cell() },
        dark,
    ));
    log.append(RecordedAct::new(
        hidden,
        ActProvenance::Reaction {
            interrupted: hidden,
        },
        ActDeed::Fired {
            target: Some(mine),
            mode:   ModeKind::Single,
            rounds: RoundCount::new(1),
        },
        watched_unnamed(PLAYER),
    ));
    log.append(RecordedAct::new(
        Entity::PLACEHOLDER,
        ActProvenance::Clock,
        ActDeed::TurnBegan { now_active: PLAYER },
        ActWitnesses::new(
            WatchingFactions::new([PLAYER, ENEMY]),
            WatchingFactions::nobody(),
        ),
    ));
    (log, mine, hidden)
}

#[test]
fn a_read_drops_what_the_asking_gang_could_not_observe() {
    let (log, ..) = a_log_of_every_key();
    let reply = window(&log, &args("()"), Some(PLAYER));

    let kinds: Vec<ActDeedKindNet> = reply.entries.iter().map(|entry| entry.kind).collect();
    assert_eq!(
        kinds.len(),
        4,
        "the three acts the player never observed — the hidden ganger's reload, its \
         magazine change and the suppression on its own unlit cell — must not reach the \
         caller; got {count} lines carrying {kinds:?}",
        count = kinds.len(),
    );
    for absent in [
        ActDeedKindNet::MagazineChanged,
        ActDeedKindNet::Suppressed,
        ActDeedKindNet::Reloaded {
            outcome: ReloadOutcomeNet::Reloaded,
        },
    ] {
        assert!(
            !kinds.contains(&absent),
            "{absent:?} happened where the squad could not see, so no line of it may come \
             back: {kinds:?}",
        );
    }
}

#[test]
fn an_observed_act_by_an_unobserved_actor_carries_no_token() {
    let (log, mine, hidden) = a_log_of_every_key();
    let reply = window(&log, &args("()"), Some(PLAYER));

    for entry in &reply.entries {
        assert_ne!(
            entry.actor,
            Some(token_of(hidden)),
            "the hidden ganger's token {token:?} reached the caller on a {kind:?} line",
            token = token_of(hidden),
            kind = entry.kind,
        );
    }

    let named: Vec<ActDeedKindNet> = reply
        .entries
        .iter()
        .filter(|entry| entry.actor == Some(token_of(mine)))
        .map(|entry| entry.kind)
        .collect();
    assert_eq!(
        named.len(),
        1,
        "exactly one line — the refusal on a cell the squad watches — names the player's \
         own ganger; got {named:?}",
    );
}

#[test]
fn a_reaction_does_not_name_an_interrupted_ganger_the_asking_gang_never_saw() {
    let (log, _mine, hidden) = a_log_of_every_key();
    let reply = window(&log, &args("()"), Some(PLAYER));

    let fired: Vec<_> = reply
        .entries
        .iter()
        .filter(|entry| entry.kind == ActDeedKindNet::Fired)
        .collect();
    assert_eq!(
        fired.len(),
        1,
        "the fixture writes exactly one fire line the player observed; got {fired:?}",
    );
    for entry in fired {
        assert_eq!(
            entry.provenance,
            ActProvenanceNet::Reaction { interrupted: None },
            "the interrupted ganger stood in the dark, so its token {token:?} must not ride \
             along on the provenance: {entry:?}",
            token = token_of(hidden),
        );
    }
}
