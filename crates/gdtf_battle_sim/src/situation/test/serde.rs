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
    let authored = "(
        gangers: [(
            at: (cell: (x: 0, y: 0), level: 0),
            name: \"Test Ganger\",
            faction: 0, facing: North, stance: Standing, aiming: false,
            hp: 10, hp_max: 10, wounds: 2, wounds_max: 2, tu: 30, tu_max: 30, life_state: Alive,
            shooting: 1.0, toughness: 1.0, luck: 0.0,
            armor: \"flak\",
            weapon: \"autogun\",
        )],
        walls: [(
            at: (cell: (x: 1, y: 1), level: 0), terrain: Wall, cover_hp: 50,
            height_band: High, armor_protection: 4, armor_hardness: 2,
        )],
        scatter: [(
            at: (cell: (x: 2, y: 2), level: 0), terrain: Cover, cover_hp: 10,
            height_band: Low, armor_protection: 1, armor_hardness: 0,
        )],
        slabs: [
            (cell: (x: 3, y: 3), level: 0),
            (cell: (x: 3, y: 3), level: 1),
        ],
        vertical_links: [(
            from: (cell: (x: 3, y: 3), level: 0),
            to: (cell: (x: 3, y: 3), level: 1),
            kind: Stair(one_way: false),
        )],
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
/// equal to the authored ganger count AND exactly that many `WornArmor`-carrying
/// entities AND exactly that many ARMED (`Weapon`-marked) entities. Count-equality plus
/// Ok proves the links validate, the file drives the real setup path, each ganger ends
/// up armed from the shipped weapon files, and each ganger ends up armored from the
/// shipped armor files (the GTW-257 AC5 + GTW-269 sim-side proof).
#[test]
fn shipped_situation_ron_drives_the_real_setup_path() {
    let Some(situation) = shipped_situation() else {
        return;
    };
    let Some(registry) = shipped_registry() else {
        return;
    };
    let Some(armor) = shipped_armor_registry() else {
        return;
    };
    let authored_ganger_count = situation.gangers.len();

    let Some((mut app, setup)) = run_setup_with(situation, registry, armor) else {
        return;
    };

    assert_eq!(
        setup.ganger_count(),
        authored_ganger_count,
        "setup must spawn exactly the authored ganger count from the shipped file",
    );
    // Exactly that many entities carry the seeded worn armor — proving the file
    // poured through the real spawn path (and that the vertical links validated,
    // since setup aborts before spawning on a bad link).
    let world: &mut World = app.world_mut();
    let mut armor_query = world.query::<&WornArmor>();
    assert_eq!(
        armor_query.iter(world).count(),
        authored_ganger_count,
        "the world must hold exactly the authored ganger count of WornArmor entities",
    );
    // GTW-257 AC5 — and exactly that many ARMED entities: each shipped ganger
    // resolved its weapon key and carries the Weapon marker.
    let mut armed_query = world.query::<&Weapon>();
    assert_eq!(
        armed_query.iter(world).count(),
        authored_ganger_count,
        "each shipped ganger must end up armed (the Weapon marker landed via the registry)",
    );
}

/// GTW-257 AC5 (companion) — every shipped ganger's authored `weapon` key is a valid
/// stem present in the shipped-weapons registry. A pure-data check (no spawn): proves
/// the `skirmish.ron` ↔ `assets/weapons/*.ron` references are consistent, so the
/// setup never hits `WeaponNotFound`.
#[test]
fn every_shipped_ganger_references_a_loaded_weapon() {
    let Some(situation) = shipped_situation() else {
        return;
    };
    let Some(registry) = shipped_registry() else {
        return;
    };
    for ganger in &situation.gangers {
        assert!(
            registry.spec(&ganger.weapon).is_some(),
            "shipped ganger weapon key {:?} must resolve against the shipped registry",
            ganger.weapon,
        );
    }
}

/// GTW-269 (companion) — every shipped ganger's authored `armor` key is a valid stem
/// present in the shipped-armor registry. A pure-data check (no spawn): proves the
/// `skirmish.ron` ↔ `assets/armor/*.armor.ron` references are consistent, so the setup
/// never hits `ArmorNotFound` (the armor mirror of the weapon-key consistency check).
#[test]
fn every_shipped_ganger_references_a_loaded_armor() {
    let Some(situation) = shipped_situation() else {
        return;
    };
    let Some(armor) = shipped_armor_registry() else {
        return;
    };
    for ganger in &situation.gangers {
        assert!(
            armor.spec(&ganger.armor).is_some(),
            "shipped ganger armor key {:?} must resolve against the shipped armor registry",
            ganger.armor,
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
