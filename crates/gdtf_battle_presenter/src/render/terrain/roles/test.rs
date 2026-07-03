//! In-crate tests for the GTW-566 tile-role vocabulary: the key round-trip, the
//! vocabulary-completeness pin against the serialized [`TileRoles`] key set, the
//! def-authorable flags, and the typed stair counterpart pairing.

use super::{TileRole, TileRoles};

/// The shipped `tile_roles.spritedef.ron`, parsed into a [`TileRoles`] — the real
/// authored table (the GTW-373 tests in `render/terrain/test.rs` pin its magnitudes;
/// these tests only use it as a live vocabulary fixture). `None` surfaces a parse
/// failure to the calling assert.
fn shipped_roles() -> Option<TileRoles> {
    const SHIPPED: &str = include_str!("../../../../../../assets/sprites/tile_roles.spritedef.ron");
    ron::de::from_str(SHIPPED).ok()
}

/// The field-key list of a serialized [`TileRoles`] RON body — `(floor:6,wall:0,…)`
/// split into its `key:` names. Every value is a `#[serde(transparent)]` integer, so
/// a flat comma/colon split is exact (no nested separators).
fn serialized_keys(text: &str) -> Vec<String> {
    text.trim()
        .trim_start_matches('(')
        .trim_end_matches(')')
        .split(',')
        .filter_map(|pair| pair.split(':').next())
        .map(str::trim)
        .filter(|key| !key.is_empty())
        .map(str::to_owned)
        .collect()
}

/// GTW-566 AC1 — the key round-trip: every [`TileRole`] survives
/// `from_key(as_key(v)) == Some(v)`, and an out-of-vocabulary string classifies to
/// [`None`] (never a phantom role).
#[test]
fn every_role_key_round_trips() {
    for role in TileRole::ALL {
        assert_eq!(
            TileRole::from_key(role.as_key()),
            Some(role),
            "from_key(as_key({role:?})) must round-trip to the same role",
        );
    }
    assert_eq!(
        TileRole::from_key("not_a_role"),
        None,
        "an out-of-vocabulary key must classify to None",
    );
}

/// GTW-566 AC2 — vocabulary completeness: the serialized key set of a [`TileRoles`]
/// value equals the [`TileRole::ALL`] `as_key` set, in both size and membership. Adding
/// a `TileRoles` field without a matching variant (or renaming a key on either side)
/// FAILS here; a variant without a field already fails `index_in` at compile time.
#[test]
fn vocabulary_matches_the_serialized_tile_roles_key_set() {
    let roles = shipped_roles();
    assert!(
        roles.is_some(),
        "the shipped tile_roles.spritedef.ron must parse into TileRoles",
    );
    let Some(roles) = roles else { return };
    let serialized = ron::ser::to_string(&roles);
    assert!(
        serialized.is_ok(),
        "TileRoles must serialize to RON: {:?}",
        serialized.as_ref().err(),
    );
    let Ok(serialized) = serialized else { return };

    let mut struct_keys = serialized_keys(&serialized);
    let mut vocab_keys: Vec<String> = TileRole::ALL
        .into_iter()
        .map(|role| role.as_key().to_owned())
        .collect();
    struct_keys.sort();
    vocab_keys.sort();
    assert_eq!(
        struct_keys, vocab_keys,
        "the TileRoles serde key set must equal the TileRole vocabulary — a field \
         added/renamed without its enum variant (or vice versa) fails this pin",
    );
}

/// GTW-566 C2 — `index_for_key` routes through the vocabulary: for every role the
/// public seam resolves the SAME index the typed `index_in` read yields, and an
/// out-of-vocabulary key resolves to [`None`].
#[test]
fn index_for_key_routes_through_the_vocabulary() {
    let roles = shipped_roles();
    assert!(
        roles.is_some(),
        "the shipped tile_roles.spritedef.ron must parse into TileRoles",
    );
    let Some(roles) = roles else { return };
    for role in TileRole::ALL {
        assert_eq!(
            roles.index_for_key(role.as_key()),
            Some(role.index_in(&roles)),
            "index_for_key({:?}) must resolve through the vocabulary to index_in",
            role.as_key(),
        );
    }
    assert_eq!(
        roles.index_for_key("not_a_role"),
        None,
        "an out-of-vocabulary key must resolve to None (the caller falls back)",
    );
}

/// GTW-566 C5 — the def-authorable flags: exactly the runtime-swap roles
/// (`emplacement_occupied` / `slab_destroyed`), the link-direction roles
/// (`stair_up` / `stair_down`), and the unoffered plain `door` are NOT authorable;
/// every other role (including the GTW-543 emplacement and the four GTW-470 oriented
/// stairs) is.
#[test]
fn def_authorable_excludes_exactly_the_runtime_and_link_roles() {
    let excluded = [
        TileRole::EmplacementOccupied,
        TileRole::SlabDestroyed,
        TileRole::StairUp,
        TileRole::StairDown,
        TileRole::Door,
    ];
    for role in TileRole::ALL {
        assert_eq!(
            role.def_authorable(),
            !excluded.contains(&role),
            "{role:?} authorability must match the GTW-566 C5 flag table",
        );
    }
}

/// GTW-566 C6 — the typed stair pairing: the three stair pairs are bidirectional
/// counterparts with exactly one ASCEND end each, and every unpaired role (ladder,
/// doors, floors, …) is fail-closed [`None`].
#[test]
fn stair_counterparts_pair_bidirectionally_and_fail_closed() {
    let pairs = [
        (TileRole::StairUp, TileRole::StairDown),
        (TileRole::StairNsUp, TileRole::StairNsDown),
        (TileRole::StairEwUp, TileRole::StairEwDown),
    ];
    for (up, down) in pairs {
        assert_eq!(
            up.counterpart(),
            Some(down),
            "{up:?} pairs down to {down:?}"
        );
        assert_eq!(down.counterpart(), Some(up), "{down:?} pairs up to {up:?}");
        assert!(up.is_up_connector(), "{up:?} is the ascend end");
        assert!(!down.is_up_connector(), "{down:?} is not an ascend end");
    }
    for role in TileRole::ALL {
        let paired = pairs.iter().any(|(up, down)| role == *up || role == *down);
        if !paired {
            assert_eq!(
                role.counterpart(),
                None,
                "unpaired role {role:?} must have no counterpart (fail-closed)",
            );
            assert!(
                !role.is_up_connector(),
                "unpaired role {role:?} must not classify as an up connector",
            );
        }
    }
}
