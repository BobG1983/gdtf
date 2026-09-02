//! role-table serde pin used to provide), the def-authorable flags, and the typed
use super::TileRole;

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

#[test]
fn def_authorable_excludes_exactly_the_runtime_and_link_roles() {
    let excluded = [TileRole::StairUp, TileRole::StairDown, TileRole::Door];
    for role in TileRole::ALL {
        assert_eq!(
            role.def_authorable(),
            !excluded.contains(&role),
            "{role:?} authorability must match the flag table",
        );
    }
}

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
