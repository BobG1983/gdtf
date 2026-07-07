//! In-crate tests for the GTW-566 tile-role vocabulary: the key round-trip, the
//! vocabulary↔seeded-sprite-catalog lockstep pin (GTW-665 — the anchor the retired
//! role-table serde pin used to provide), the def-authorable flags, and the typed
//! stair counterpart pairing.

use std::path::PathBuf;

use super::TileRole;

/// The shipped `assets/content/sprites/` catalog directory — the sprite-def
/// members the presenter resolves graphic keys against (GTW-665).
fn shipped_sprites_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("assets")
        .join("content")
        .join("sprites")
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

/// GTW-566 AC2 as re-anchored by GTW-665 — vocabulary↔catalog lockstep: every
/// [`TileRole`] key names a shipped `content/sprites/<key>.spritedef.ron` member
/// (the presenter-owned role fallbacks — floor / rubble / the stair-link tiles —
/// must resolve to REAL defs, never the missing-marker). Before GTW-665 this pin
/// compared the vocabulary against the retired `TileRoles` serde key set; the
/// seeded sprite catalog is the successor truth.
#[test]
fn every_role_key_names_a_shipped_sprite_def() {
    let dir = shipped_sprites_dir();
    for role in TileRole::ALL {
        let member = dir.join(format!("{}.spritedef.ron", role.as_key()));
        assert!(
            member.is_file(),
            "the vocabulary key `{}` must name a shipped sprite-def member at {member:?} — a \
             renamed/removed seed breaks the presenter's role fallback resolution",
            role.as_key(),
        );
    }
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
