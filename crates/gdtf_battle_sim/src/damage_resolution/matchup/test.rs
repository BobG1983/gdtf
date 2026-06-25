//! Relocated unit tests for the matchup wheel (GTW-201 wave 22 — moved verbatim from
//! the former inline `#[cfg(test)] mod tests`).

use crate::{
    armor::ArmorType,
    matchup::{Matchup, WheelNode, matchup, matchup_multiplier, wheel::WHEEL_NODE_COUNT},
    tuning::CombatTuning,
    weapon::DamageType,
};

/// AC1 — sweep all 49 (`DamageType` × `ArmorType`) pairs and assert that
/// EXACTLY the 7 mirror-node pairs (node `i` weapon vs node `i` armor) are
/// [`Matchup::Neutral`], and no others.
#[test]
fn neutral_iff_same_wheel_node() {
    let mut neutral_count = 0u32;
    for weapon in DamageType::ALL {
        for armor in ArmorType::ALL {
            let same_node = *weapon.node() == *armor.node();
            let result = matchup(weapon, armor);
            if same_node {
                assert_eq!(
                    result,
                    Matchup::Neutral,
                    "mirror pair {weapon:?} vs {armor:?} must be Neutral",
                );
                neutral_count += 1;
            } else {
                assert_ne!(
                    result,
                    Matchup::Neutral,
                    "non-mirror pair {weapon:?} vs {armor:?} must NOT be Neutral",
                );
            }
        }
    }
    // Exactly the 7 mirror pairs are Neutral.
    assert_eq!(neutral_count, 7, "exactly 7 mirror pairs must be Neutral");
}

/// AC2 — balanced regular tournament: for each of the 7 `DamageType`s, count
/// Favorable across the 7 `ArmorType`s == 3 and Resisted == 3 (and Neutral == 1,
/// its own mirror).
#[test]
fn each_type_favorable_three_resisted_three() {
    for weapon in DamageType::ALL {
        let mut favorable = 0u32;
        let mut neutral = 0u32;
        let mut resisted = 0u32;
        for armor in ArmorType::ALL {
            match matchup(weapon, armor) {
                Matchup::Favorable => favorable += 1,
                Matchup::Neutral => neutral += 1,
                Matchup::Resisted => resisted += 1,
            }
        }
        assert_eq!(favorable, 3, "{weapon:?} must be Favorable vs exactly 3");
        assert_eq!(resisted, 3, "{weapon:?} must be Resisted vs exactly 3");
        assert_eq!(
            neutral, 1,
            "{weapon:?} must be Neutral vs exactly 1 (mirror)"
        );
    }
}

/// AC3 — tournament antisymmetry: whenever node X (weapon) is Favorable against
/// node Y (armor), the mirror direction — the `DamageType` at Y's node vs the
/// `ArmorType` at X's node — is Resisted (one self-converse wheel drives both
/// sides). Crosses between the two vocabularies via the node index.
#[test]
fn favorable_implies_mirror_resisted() {
    for weapon in DamageType::ALL {
        for armor in ArmorType::ALL {
            if matchup(weapon, armor) != Matchup::Favorable {
                continue;
            }
            // Cross vocabularies at the same wheel nodes: the weapon now sits
            // at the armor's node, the armor at the weapon's node.
            let mirror_weapon = DamageType::ALL[*armor.node() as usize];
            let mirror_armor = ArmorType::ALL[*weapon.node() as usize];
            assert_eq!(
                matchup(mirror_weapon, mirror_armor),
                Matchup::Resisted,
                "if {weapon:?} is Favorable vs {armor:?}, the mirror direction \
                 ({mirror_weapon:?} vs {mirror_armor:?}) must be Resisted",
            );
        }
    }
}

/// AC5 — load the default tuning multipliers and assert
/// `resisted < neutral < favorable` as an ORDERING (never a magnitude). The
/// resisted penalty is the heavier swing, the favorable bonus the lighter —
/// the asymmetry of `docs/combat/matchup.md` §"Modifier never auto-win".
#[test]
fn multiplier_ordering_resisted_lt_neutral_lt_favorable() {
    let tuning = CombatTuning::default();
    let resisted = *matchup_multiplier(Matchup::Resisted, &tuning);
    let neutral = *matchup_multiplier(Matchup::Neutral, &tuning);
    let favorable = *matchup_multiplier(Matchup::Favorable, &tuning);

    assert!(
        resisted < neutral,
        "resisted multiplier must be < neutral (got {resisted} vs {neutral})",
    );
    assert!(
        neutral < favorable,
        "neutral multiplier must be < favorable (got {neutral} vs {favorable})",
    );
}

/// The `node()` accessor agrees with `ALL` ordering for both vocabularies, and
/// the two are mirrors (node `i` of one matches node `i` of the other). Pins
/// the bridge from the dual vocabulary onto the shared wheel.
#[test]
fn node_accessor_agrees_with_all_ordering() {
    for (i, weapon) in DamageType::ALL.into_iter().enumerate() {
        assert_eq!(
            *weapon.node(),
            i as u8,
            "{weapon:?} node must be its ALL index"
        );
    }
    for (i, armor) in ArmorType::ALL.into_iter().enumerate() {
        assert_eq!(
            *armor.node(),
            i as u8,
            "{armor:?} node must be its ALL index"
        );
    }
    // Mirror parity across the two vocabularies (matchup.md Table 1).
    for i in 0..7usize {
        assert_eq!(*DamageType::ALL[i].node(), *ArmorType::ALL[i].node());
    }
}

/// `WheelNode::strong_against` returns the quadratic non-residues
/// `{(n+3)%7, (n+5)%7, (n+6)%7}` for every node, all reduced into `0..=6` and
/// none equal to the node itself (a node is never strong against its mirror).
#[test]
fn strong_against_is_the_non_residue_offsets() {
    for n in 0..WHEEL_NODE_COUNT {
        let node = WheelNode::new(n);
        let strong = node.strong_against();
        let expected = [(n + 3) % 7, (n + 5) % 7, (n + 6) % 7];
        for (got, want) in strong.into_iter().zip(expected) {
            assert_eq!(*got, want, "node {n} strong-against offset drifted");
        }
        assert!(
            !strong.contains(&node),
            "node {n} must not be strong against its own mirror",
        );
    }
}
