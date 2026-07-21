//! GTW-205 / E10.3 serde tests — the `Situation` value graph deserializes, and the
//! SHIPPED authored file parses + drives the real setup path. The tests are
//! value-AGNOSTIC on tunables — they assert structural relations (counts, distinct
//! factions, Ok), never a pinned hp/tu/armor magnitude (those are authored data, not
//! pinned by the test).

use bevy::platform::collections::HashSet;

use super::support::*;
use crate::{
    metric::Cell,
    procgen::{Anchor, DeploymentZone, DeploymentZones, Footprint, RegionRect, deploy_rosters},
    rng::BattleSeed,
};

/// GTW-205 AC1 — an inline RON `Situation` containing ≥1 ganger, ≥1 wall, ≥1
/// scatter piece, ≥1 slab, and ≥1 vertical link deserializes to `Ok`, and the
/// resulting value's list lengths equal the authored counts. Count-equality, not
/// a tunable magnitude — proving the whole spawn-struct value graph is
/// serde-deserializable through the landed newtype/enum derives.
#[test]
fn situation_deserializes_from_inline_ron_with_each_section() {
    // A minimal-but-complete authored situation: one of every section. The
    // single vertical link's endpoints are authored slabs on different storeys
    // (non-dangling, cross-storey), so the value is setup-able.
    // GTW-491 schema: walls/scatter/slabs/floors `piece` keys + `default_floor` are now
    // UUID-keyed `TerrainUuid` strings (the migration from filename-stem `TerrainName`).
    // GTW-414 schema v2: a placed ganger is a REFERENCE (gang + member) plus placement +
    // faction — the roster (identity / attributes / weapon / armor) lives in a gang file,
    // resolved against the GangRegistry at setup (not authored inline here).
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
    // Count-equality with the authored sections — never a magnitude.
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

/// GTW-205 AC3 / GTW-433 C1 / GTW-744 — the shipped situation file parses back into a
/// `Situation` (value-agnostic round-trip). Since the GTW-433 theme+size procgen migration the
/// shipped `skirmish.ron` authors NO inline terrain, and since GTW-744 it authors its combatants
/// as `rosters` (gang + member + faction refs, NO placement cells — procgen derives them) and
/// ZERO placed `gangers`. So the structural assertions are: rosters non-empty, ≥2 distinct
/// factions present among the rosters, ZERO placed gangers, and ZERO inline terrain entries —
/// NEVER a pinned magnitude (authored data, not pinned by the test).
#[test]
fn shipped_situation_ron_deserializes_with_required_structure() {
    let Some(situation) = shipped_situation() else {
        return;
    };

    assert!(
        !situation.rosters.is_empty(),
        "the shipped file must author at least one roster member",
    );
    // GTW-744: the shipped file authors NO placement cells — the deploy step derives them.
    assert!(
        situation.gangers.is_empty(),
        "the shipped file must author NO placed gangers (GTW-744: procgen derives the cells); \
         found {}",
        situation.gangers.len(),
    );
    // ≥2 distinct factions present (two gangs face off).
    let distinct_factions: HashSet<_> = situation.rosters.iter().map(|m| m.faction).collect();
    assert!(
        distinct_factions.len() >= 2,
        "the shipped file must author at least two distinct factions, found {}",
        distinct_factions.len(),
    );
    // GTW-433 C1: the shipped file authors NO inline terrain — it is procgen-generated from
    // theme + grid_size at Generation. Each terrain list must be empty (un-migrating the file
    // by re-authoring any terrain entry turns this red).
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

// GTW-387 (C1)'s `shipped_skirmish_has_walkable_l1_content_beyond_link_tile` test was
// REMOVED by the GTW-433 theme+size procgen migration: the shipped `skirmish.ron` no longer
// authors ANY terrain (slabs / vertical links included) — the level (and thus its multi-
// storey walkable platforms) is now PROCGEN-generated at Generation, so an authored-L1-
// content invariant no longer applies to the situation file. The "no inline terrain"
// invariant the migration DOES require is asserted by
// `shipped_situation_ron_deserializes_with_required_structure` above (it pins every terrain
// list, slabs included, to empty). Ensuring procgen-generated multi-storey levels have
// reachable upper-floor destinations is the procgen pipeline's connectivity concern (the
// GTW-431 emit-step OQ-4 assertion), not the authored file's.

/// GTW-226 AC6 — the shipped `skirmish.ron` omits `player_faction`, so the
/// struct-level `#[serde(default)]` supplies [`Faction::default`] = `Faction(0)`
/// (gang 0 = the player, by convention). Documents the data-driven serde seam: no
/// `.ron` edit is needed for this slice, and the omitted field defaults cleanly.
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

/// GTW-205 AC4 / GTW-257 AC5 / GTW-269 / GTW-744 — the shipped file's vertical links validate
/// AND every shipped ROSTER member's weapon key resolves against the SHIPPED weapons registry
/// AND its armor key resolves against the SHIPPED armor registry, proving it is a setup-able,
/// fully-armed, fully-armored situation. Deserialize the shipped file, DEPLOY its rosters into
/// placed gangers via the REAL [`deploy_rosters`](crate::procgen::deploy_rosters)
/// (`deploy_shipped_rosters`), run `setup_battle` on a `MinimalPlugins` app against the shipped
/// weapon and armor registries, and assert it returns `Ok(BattleSetup)` with `ganger_count()`
/// equal to the roster count AND exactly that many `Wears`-carrying gangers
/// (the armor relationship) AND exactly that many ARMED (wielded-weapon) entities.
/// Count-equality plus Ok proves the links validate, the file drives the real setup
/// path, each ganger ends up armed from the shipped weapon files, and each ganger ends
/// up armored from the shipped armor files (the GTW-257 AC5 + GTW-269 sim-side proof).
#[test]
fn shipped_situation_ron_drives_the_real_setup_path() {
    let Some(situation) = shipped_situation() else {
        return;
    };
    // GTW-414/415: the shipped skirmish.ron now REFERENCES gangs (gang_0 / gang_1) by
    // name; supply the gang registry built from the shipped `assets/content/gangs/`
    // files so each placed ganger's (gang, member) ref resolves to its roster.
    let Some(gangs) = shipped_gang_registry() else {
        return;
    };
    let Some(registry) = shipped_registry() else {
        return;
    };
    let Some(armor) = shipped_armor_registry() else {
        return;
    };
    // GTW-491: the shipped skirmish.ron authors NO inline terrain, so the (empty) UUID-keyed
    // terrain registry is never actually resolved against; supply it to satisfy the setup
    // signature.
    let terrain = shipped_terrain_registry();
    // GTW-744: the shipped file authors `rosters` (no placement cells), so DEPLOY them into
    // placed gangers via the REAL deploy_rosters before setup — the count to spawn is the
    // roster count.
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
    // Exactly that many gangers carry the `Wears` armor relationship — proving the file
    // poured through the real spawn path (and that the vertical links validated, since
    // setup aborts before spawning on a bad link). Since GTW-323 slice 3 (ADR-0004) the
    // armor stats live on related piece entities, NOT the ganger, so the ganger carries
    // the `Wears` collection (one per ganger), not a `WornArmor` component.
    let world: &mut World = app.world_mut();
    let mut armor_query = world.query::<&Wears>();
    assert_eq!(
        armor_query.iter(world).count(),
        authored_ganger_count,
        "the world must hold exactly the authored ganger count of gangers wearing armor (Wears)",
    );
    // GTW-257 AC5 — and exactly that many ARMED entities: each shipped ganger resolved
    // its weapon key and carries the Weapon marker. Since GTW-323 (ADR-0004) the weapon
    // is the related WEAPON entity (one per ganger, `WieldedBy` + Weapon), so the armed
    // count is the count of weapon entities.
    let mut armed_query = world.query::<(&Weapon, &WieldedBy)>();
    assert_eq!(
        armed_query.iter(world).count(),
        authored_ganger_count,
        "each shipped ganger must wield a weapon ENTITY (the Weapon marker landed via the \
         registry on its related weapon)",
    );
}

/// GTW-257 AC5 (companion) — every shipped ganger's weapon key is a valid stem present
/// in the shipped-weapons registry. A pure-data check (no spawn): proves the
/// `skirmish.ron` ↔ `assets/content/gangs/*.gang.ron` ↔ `assets/content/weapons/ranged/*.ron`
/// references are consistent, so the setup never hits `WeaponNotFound`.
///
/// GTW-414/415 + GTW-744: the weapon key lives on the gang-ROSTER member; each shipped
/// [`RosterMember`](crate::situation::RosterMember)'s `(gang, member)` ref is resolved
/// against the shipped gang registry to its [`GangMember`](crate::ganger::GangMember),
/// whose `weapon` key is then checked — proving the full roster → member → weapon chain
/// resolves.
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
        // Resolve roster ref → roster member → its weapon key against the shipped registry,
        // in one chain. `Some(true)` means the full chain resolved AND the weapon key is
        // present; anything else (member missing, or weapon key absent) is the failure.
        let weapon_resolves = gangs
            .roster(&roster.gang)
            .and_then(|gang_roster| gang_roster.member(&roster.member))
            .map(|member| registry.spec(&member.weapon).is_some());
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

/// GTW-269 (companion) — every shipped ganger's armor key is a valid stem present in the
/// shipped-armor registry. A pure-data check (no spawn): proves the `skirmish.ron` ↔
/// `assets/content/gangs/*.gang.ron` ↔ `assets/content/armor/*.armor.ron` references are
/// consistent, so the setup never hits `ArmorNotFound` (the armor mirror of the weapon-key
/// consistency check).
///
/// GTW-414/415 + GTW-744: the armor key lives on the gang-ROSTER member — so each shipped
/// [`RosterMember`](crate::situation::RosterMember)'s `(gang, member)` ref is resolved against
/// the shipped gang registry to its [`GangMember`](crate::ganger::GangMember), whose `armor`
/// key is then checked.
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
        // Resolve roster ref → roster member → its armor key against the shipped registry,
        // in one chain (the armor mirror of the weapon check above).
        let armor_resolves = gangs
            .roster(&roster.gang)
            .and_then(|gang_roster| gang_roster.member(&roster.member))
            .map(|member| armor.spec(&member.armor).is_some());
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

/// GTW-205 AC5 — every value-bearing field in the authored `.ron` carries a
/// per-line explanatory comment (the tuning `.ron` convention). Reads the shipped
/// file text and asserts that every line carrying an authored LEAF value (a
/// scalar/variant field or a list element) also carries a `//` annotation. This
/// guards the canon-comment convention without pinning any value.
#[test]
fn shipped_situation_ron_is_per_line_commented() {
    for raw in SHIPPED_SITUATION_RON.lines() {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            continue;
        }
        // A full-line comment (header / section banner) is fine as-is.
        if trimmed.starts_with("//") {
            continue;
        }
        // Split off any trailing comment; the code portion is what precedes `//`.
        let code = trimmed.split("//").next().unwrap_or("").trim();
        if code.is_empty() {
            continue;
        }
        // A line that only OPENS or CLOSES a block (its code ends with a bare
        // bracket, e.g. `gangers: [`, `armor: (`, `(`, `),`, `],`) is structural:
        // the section-comment banner above it documents the block, so it needs no
        // per-line annotation. Every other code line carries an authored LEAF
        // value and MUST be annotated.
        let opens_block = code.ends_with('(') || code.ends_with('[') || code.ends_with('{');
        let closes_block = code
            .chars()
            .all(|c| matches!(c, '(' | ')' | '[' | ']' | '{' | '}' | ','));
        if opens_block || closes_block {
            continue;
        }
        // A value-bearing leaf line: it MUST carry a `//` annotation somewhere.
        assert!(
            trimmed.contains("//"),
            "every value-bearing line must carry a `//` comment; bare line: {trimmed:?}",
        );
    }
}

/// Deploy the shipped situation's GTW-744 ROSTER members into placed gangers via the REAL
/// [`deploy_rosters`](crate::procgen::deploy_rosters) over hand-built deployment zones — so
/// [`shipped_situation_ron_drives_the_real_setup_path`] exercises `setup_battle` on the DEPLOYED
/// gangers (the shipped file now authors `rosters`, not placed `gangers`).
///
/// The zones are two non-overlapping 10×10 corners of the shipped 30×30 board (ample standable
/// room for the four-member roster). This drives the REAL deploy function (not a shadow);
/// deployment CORRECTNESS over the real `generate_level` zones is separately pinned by
/// `procgen::test::deploy`. Deterministic (fixed seed). Returns the situation with its `gangers`
/// extended by the deployed set (an empty roster leaves it unchanged). Local to this file — its
/// sole consumer — per the module-layout single-consumer-helper rule.
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
