//! SHIPPED authored file parses + drives the real setup path. The tests are
//! factions, Ok), never a pinned hp/tu/armor magnitude (those are authored data, not
use bevy::platform::collections::HashSet;

use super::support::*;
use crate::{
    metric::Cell,
    procgen::{Anchor, DeploymentZone, DeploymentZones, Footprint, RegionRect, deploy_rosters},
    rng::BattleSeed,
};

#[test]
fn situation_deserializes_from_inline_ron_with_each_section() {
    let authored = "(
        gangers: [(
            gang: \"gang_0\",
            member: \"Test Ganger\",
            at: (cell: (x: 0, y: 0), level: 0),
            faction: 0, facing: North, stance: Standing, aiming: false, life_state: Alive,
        )],
        walls: [(
            at: (cell: (x: 1, y: 1), level: 0), piece: \"01491491-0000-0001-0000-000000000000\",
        )],
        scatter: [(
            at: (cell: (x: 2, y: 2), level: 0), piece: \"01491491-0000-0003-0000-000000000000\",
        )],
        slabs: [
            (at: (cell: (x: 3, y: 3), level: 0), piece: \"01491491-0000-0002-0000-000000000000\"),
            (at: (cell: (x: 3, y: 3), level: 1), piece: \"01491491-0000-0002-0000-000000000000\"),
        ],
        vertical_links: [(
            from: (cell: (x: 3, y: 3), level: 0),
            to: (cell: (x: 3, y: 3), level: 1),
            kind: Stair(one_way: false),
        )],
        default_floor: \"01491491-0000-0004-0000-000000000000\",
        floors: [],
    )";

    let parsed = ron::de::from_str::<Situation>(authored);
    assert!(
        parsed.is_ok(),
        "inline Situation RON must parse: {parsed:?}"
    );
    let Ok(situation) = parsed else {
        return;
    };
    assert_eq!(situation.gangers.len(), 1, "one authored ganger");
    assert_eq!(situation.walls.len(), 1, "one authored wall");
    assert_eq!(situation.scatter.len(), 1, "one authored scatter piece");
    assert_eq!(situation.slabs.len(), 2, "two authored slabs");
    assert_eq!(
        situation.vertical_links.len(),
        1,
        "one authored vertical link",
    );
}

#[test]
fn shipped_situation_ron_deserializes_with_required_structure() {
    let Some(situation) = shipped_situation() else {
        return;
    };

    assert!(
        !situation.rosters.is_empty(),
        "the shipped file must author at least one roster member",
    );
    assert!(
        situation.gangers.is_empty(),
        "the shipped file must author NO placed gangers ( procgen derives the cells); \
         found {}",
        situation.gangers.len(),
    );
    let distinct_factions: HashSet<_> = situation.rosters.iter().map(|m| m.faction).collect();
    assert!(
        distinct_factions.len() >= 2,
        "the shipped file must author at least two distinct factions, found {}",
        distinct_factions.len(),
    );
    assert!(
        situation.walls.is_empty(),
        "the migrated shipped file must author NO walls (procgen generates them); found {}",
        situation.walls.len(),
    );
    assert!(
        situation.scatter.is_empty(),
        "the migrated shipped file must author NO scatter (procgen generates it); found {}",
        situation.scatter.len(),
    );
    assert!(
        situation.slabs.is_empty(),
        "the migrated shipped file must author NO slabs (procgen generates them); found {}",
        situation.slabs.len(),
    );
    assert!(
        situation.vertical_links.is_empty(),
        "the migrated shipped file must author NO vertical links (terrain — procgen generates \
         them); found {}",
        situation.vertical_links.len(),
    );
    assert!(
        situation.floors.is_empty(),
        "the migrated shipped file must author NO floor overrides (procgen generates them); \
         found {}",
        situation.floors.len(),
    );
    assert!(
        *situation.default_floor.is_nil(),
        "the migrated shipped file must author NO default_floor (procgen supplies it)",
    );
}

/// struct-level `#[serde(default)]` supplies [`Faction::default`] = `Faction(0)`
#[test]
fn shipped_situation_player_faction_defaults_to_gang_zero() {
    let Some(situation) = shipped_situation() else {
        return;
    };
    assert_eq!(
        situation.player_faction,
        Faction::new(0),
        "the shipped file omits player_faction, so it defaults to gang 0 (the player)",
    );
}

#[test]
fn shipped_situation_ron_drives_the_real_setup_path() {
    let Some(situation) = shipped_situation() else {
        return;
    };
    let Some(gangs) = shipped_gang_registry() else {
        return;
    };
    let Some(registry) = shipped_registry() else {
        return;
    };
    let Some(armor) = shipped_armor_registry() else {
        return;
    };
    let terrain = shipped_terrain_registry();
    let authored_ganger_count = situation.rosters.len();
    let situation = deploy_shipped_rosters(situation);

    let Some((mut app, setup)) = run_setup_with(situation, gangs, registry, armor, Some(&terrain))
    else {
        return;
    };

    assert_eq!(
        *setup.ganger_count(),
        authored_ganger_count,
        "setup must spawn exactly the authored ganger count from the shipped file",
    );
    let world: &mut World = app.world_mut();
    let mut armor_query = world.query::<&Wears>();
    assert_eq!(
        armor_query.iter(world).count(),
        authored_ganger_count,
        "the world must hold exactly the authored ganger count of gangers wearing armor (Wears)",
    );
    let mut armed_query = world.query::<(&Weapon, &WieldedBy)>();
    assert_eq!(
        armed_query.iter(world).count(),
        authored_ganger_count,
        "each shipped ganger must wield a weapon ENTITY (the Weapon marker landed via the \
         registry on its related weapon)",
    );
}

#[test]
fn every_shipped_ganger_references_a_loaded_weapon() {
    let Some(situation) = shipped_situation() else {
        return;
    };
    let Some(gangs) = shipped_gang_registry() else {
        return;
    };
    let Some(registry) = shipped_registry() else {
        return;
    };
    for roster in &situation.rosters {
        let weapon_resolves = gangs
            .roster(&roster.gang)
            .and_then(|gang_roster| gang_roster.member(&roster.member))
            .map(|member| {
                member
                    .weapon
                    .as_ref()
                    .is_some_and(|key| registry.spec(key).is_some())
            });
        assert_eq!(
            weapon_resolves,
            Some(true),
            "shipped roster member {:?}/{:?} must resolve its gang member AND that member's \
             weapon key against the shipped registry",
            roster.gang,
            roster.member,
        );
    }
}

#[test]
fn every_shipped_ganger_references_a_loaded_armor() {
    let Some(situation) = shipped_situation() else {
        return;
    };
    let Some(gangs) = shipped_gang_registry() else {
        return;
    };
    let Some(armor) = shipped_armor_registry() else {
        return;
    };
    for roster in &situation.rosters {
        let armor_resolves = gangs
            .roster(&roster.gang)
            .and_then(|gang_roster| gang_roster.member(&roster.member))
            .map(|member| {
                member
                    .armor
                    .as_ref()
                    .is_some_and(|key| armor.spec(key).is_some())
            });
        assert_eq!(
            armor_resolves,
            Some(true),
            "shipped roster member {:?}/{:?} must resolve its gang member AND that member's \
             armor key against the shipped armor registry",
            roster.gang,
            roster.member,
        );
    }
}

#[test]
fn shipped_situation_ron_is_per_line_commented() {
    for raw in SHIPPED_SITUATION_RON.lines() {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.starts_with("//") {
            continue;
        }
        let code = trimmed.split("//").next().unwrap_or("").trim();
        if code.is_empty() {
            continue;
        }
        let opens_block = code.ends_with('(') || code.ends_with('[') || code.ends_with('{');
        let closes_block = code
            .chars()
            .all(|c| matches!(c, '(' | ')' | '[' | ']' | '{' | '}' | ','));
        if opens_block || closes_block {
            continue;
        }
        assert!(
            trimmed.contains("//"),
            "every value-bearing line must carry a `//` comment; bare line: {trimmed:?}",
        );
    }
}

#[test]
fn a_roster_member_writing_no_loadout_keys_parses_with_both_reading_none() {
    let authored = "(
        members: [(
            name: \"Bare\",
            speed: 4, aim: 4, strength: 4, toughness: 4,
            reflexes: 4, cool: 4, grit: 4, luck: 4,
        )],
    )";

    let parsed = ron::de::from_str::<GangRoster>(authored);
    assert!(
        parsed.is_ok(),
        "a member writing neither armor nor weapon must parse: {parsed:?}",
    );
    let Ok(roster) = parsed else {
        return;
    };
    assert_eq!(roster.members.len(), 1, "the roster authors one member");
    let Some(member) = roster.members.first() else {
        return;
    };
    assert_eq!(member.armor, None, "an omitted armor key reads None");
    assert_eq!(member.weapon, None, "an omitted weapon key reads None");
}

fn deploy_shipped_rosters(mut situation: Situation) -> Situation {
    let zones = DeploymentZones::new(
        DeploymentZone::new(
            Anchor::BottomLeft,
            RegionRect::new(Cell::new(0, 0), Footprint::new(10, 10)),
        ),
        DeploymentZone::new(
            Anchor::TopRight,
            RegionRect::new(Cell::new(20, 20), Footprint::new(10, 10)),
        ),
    );
    if let Ok(placed) = deploy_rosters(
        &zones,
        &situation,
        &situation.rosters,
        situation.player_faction,
        BattleSeed::new(0x0744),
    ) {
        situation.gangers.extend(placed);
    }
    situation
}
