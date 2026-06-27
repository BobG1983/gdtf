//! GTW-205 / E10.3 serde tests — the `Situation` value graph deserializes, and the
//! SHIPPED authored file parses + drives the real setup path. The tests are
//! value-AGNOSTIC on tunables — they assert structural relations (counts, distinct
//! factions, Ok), never a pinned hp/tu/armor magnitude (those are authored data, not
//! pinned by the test).

use bevy::platform::collections::HashSet;

use super::support::*;

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
    // GTW-396: walls/scatter use the new `piece` key schema (no inline stats);
    // slabs use the new `SlabSpawn { at, piece }` schema; `default_floor` and
    // `floors` are the new GTW-396 floor fields.
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
            at: (cell: (x: 1, y: 1), level: 0), piece: \"test-wall\",
        )],
        scatter: [(
            at: (cell: (x: 2, y: 2), level: 0), piece: \"test-cover\",
        )],
        slabs: [
            (at: (cell: (x: 3, y: 3), level: 0), piece: \"test-slab\"),
            (at: (cell: (x: 3, y: 3), level: 1), piece: \"test-slab\"),
        ],
        vertical_links: [(
            from: (cell: (x: 3, y: 3), level: 0),
            to: (cell: (x: 3, y: 3), level: 1),
            kind: Stair(one_way: false),
        )],
        default_floor: \"test-floor\",
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

/// GTW-205 AC3 — the shipped situation file parses back into a `Situation`
/// (value-agnostic round-trip). Asserts only STRUCTURAL relations: gangers
/// non-empty, ≥2 distinct factions present, ≥1 wall, ≥1 scatter, ≥1 slab, ≥1
/// vertical link — NEVER a specific hp/tu/armor magnitude (authored data, not
/// pinned by the test).
#[test]
fn shipped_situation_ron_deserializes_with_required_structure() {
    let Some(situation) = shipped_situation() else {
        return;
    };

    assert!(
        !situation.gangers.is_empty(),
        "the shipped file must author at least one ganger",
    );
    // ≥2 distinct factions present (two gangs face off).
    let distinct_factions: HashSet<_> = situation.gangers.iter().map(|g| g.faction).collect();
    assert!(
        distinct_factions.len() >= 2,
        "the shipped file must author at least two distinct factions, found {}",
        distinct_factions.len(),
    );
    assert!(
        !situation.walls.is_empty(),
        "the shipped file must author at least one wall",
    );
    assert!(
        !situation.scatter.is_empty(),
        "the shipped file must author at least one scatter piece",
    );
    assert!(
        !situation.slabs.is_empty(),
        "the shipped file must author at least one slab",
    );
    assert!(
        !situation.vertical_links.is_empty(),
        "the shipped file must author at least one vertical link",
    );
}

/// GTW-387 (C1) — the shipped `skirmish.ron` has **walkable L1 content beyond the
/// link tile**: at least one authored slab is at level 1 AND is not the stair-link
/// endpoint itself (i.e. NOT at the same `(x, y)` as every vertical link's `from`
/// foot, so a ganger who reaches the stair head has real destinations to walk to on
/// the upper floor). This is a real-path parse (the same `include_str!` path the
/// other shipped-file tests use) — it catches a regression where the platform
/// slabs were stripped back to a dead-end single link tile.
///
/// Pin-discriminating: this fails if the shipped file is missing L1 slabs beyond
/// the link-endpoint cell (the exact pre-GTW-387 bug that stranded gangers at the
/// stair head with nowhere to go).
#[test]
fn shipped_skirmish_has_walkable_l1_content_beyond_link_tile() {
    let Some(situation) = shipped_situation() else {
        return;
    };

    // Collect the (x, y) of every stair head (the L1 arrival endpoint of a
    // vertical link).  We do NOT want to count those cells as "real" L1 platform
    // content, because the pre-GTW-387 bug was that ONLY the link-head cell
    // existed on L1 — a dead-end with no further walkable destinations.
    let link_head_xy: HashSet<(i32, i32)> = situation
        .vertical_links
        .iter()
        .map(|l| (l.to.x, l.to.y))
        .collect();

    // Count L1 slabs that are NOT on a link-head (x, y) — the platform cells.
    let l1_platform_count = situation
        .slabs
        .iter()
        .filter(|s| s.at.z == 1 && !link_head_xy.contains(&(s.at.x, s.at.y)))
        .count();

    assert!(
        l1_platform_count >= 1,
        "the shipped skirmish must have ≥1 L1 slab BEYOND the link-tile (walkable platform) \
         so a ganger who climbs the stair has real destinations; found {l1_platform_count} \
         non-link-head L1 slabs (GTW-387 C1)",
    );
}

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

/// GTW-205 AC4 / GTW-257 AC5 / GTW-269 — the shipped file's vertical links validate AND
/// every authored ganger's weapon key resolves against the SHIPPED weapons registry AND
/// every authored ganger's armor key resolves against the SHIPPED armor registry,
/// proving it is a setup-able, fully-armed, fully-armored situation. Deserialize the
/// shipped file, run `setup_battle` on a `MinimalPlugins` app against the shipped weapon
/// and armor registries, and assert it returns `Ok(BattleSetup)` with `ganger_count()`
/// equal to the authored ganger count AND exactly that many `Wears`-carrying gangers
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
    // GTW-396: the shipped skirmish.ron now references terrain piece keys; supply the
    // shipped terrain registry so cover/slab/floor keys resolve correctly.
    let Some(terrain) = shipped_terrain_registry() else {
        return;
    };
    let authored_ganger_count = situation.gangers.len();

    let Some((mut app, setup)) = run_setup_with(situation, gangs, registry, armor, Some(&terrain))
    else {
        return;
    };

    assert_eq!(
        setup.ganger_count(),
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
/// `skirmish.ron` ↔ `assets/content/gangs/*.gang.ron` ↔ `assets/content/weapons/*.ron`
/// references are consistent, so the setup never hits `WeaponNotFound`.
///
/// GTW-414/415: the weapon key now lives on the gang-ROSTER member, not the placement.
/// Each placed ganger's `(gang, member)` ref is resolved against the shipped gang
/// registry to its [`GangMember`](crate::ganger::GangMember), whose `weapon` key is then
/// checked — proving the full placement → roster → weapon chain resolves.
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
    for placed in &situation.gangers {
        // Resolve placement → roster member → its weapon key against the shipped registry,
        // in one chain. `Some(true)` means the full chain resolved AND the weapon key is
        // present; anything else (member missing, or weapon key absent) is the failure.
        let weapon_resolves = gangs
            .roster(&placed.gang)
            .and_then(|roster| roster.member(&placed.member))
            .map(|member| registry.spec(&member.weapon).is_some());
        assert_eq!(
            weapon_resolves,
            Some(true),
            "shipped placed ganger {:?}/{:?} must resolve its roster member AND that member's \
             weapon key against the shipped registry",
            placed.gang,
            placed.member,
        );
    }
}

/// GTW-269 (companion) — every shipped ganger's armor key is a valid stem present in the
/// shipped-armor registry. A pure-data check (no spawn): proves the `skirmish.ron` ↔
/// `assets/content/gangs/*.gang.ron` ↔ `assets/content/armor/*.armor.ron` references are
/// consistent, so the setup never hits `ArmorNotFound` (the armor mirror of the weapon-key
/// consistency check).
///
/// GTW-414/415: the armor key now lives on the gang-ROSTER member, not the placement — so
/// each placed ganger's `(gang, member)` ref is resolved against the shipped gang registry
/// to its [`GangMember`](crate::ganger::GangMember), whose `armor` key is then checked.
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
    for placed in &situation.gangers {
        // Resolve placement → roster member → its armor key against the shipped registry,
        // in one chain (the armor mirror of the weapon check above).
        let armor_resolves = gangs
            .roster(&placed.gang)
            .and_then(|roster| roster.member(&placed.member))
            .map(|member| armor.spec(&member.armor).is_some());
        assert_eq!(
            armor_resolves,
            Some(true),
            "shipped placed ganger {:?}/{:?} must resolve its roster member AND that member's \
             armor key against the shipped armor registry",
            placed.gang,
            placed.member,
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
