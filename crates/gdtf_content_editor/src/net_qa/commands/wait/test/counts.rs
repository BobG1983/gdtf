use super::super::probe::RegistryChangeCounts;
use crate::net_qa::wire::ContentFamilyNet;

#[test]
fn a_fresh_tally_stands_at_zero_for_every_family() {
    let counts = RegistryChangeCounts::default();
    for family in ContentFamilyNet::ALL {
        assert_eq!(
            *counts.count_of(family),
            0,
            "a wait parked before anything has happened must see {family:?} at zero, or it \
             would answer on a change that never came",
        );
    }
}

#[test]
fn counting_one_family_leaves_every_other_family_alone() {
    for counted in ContentFamilyNet::ALL {
        let mut counts = RegistryChangeCounts::default();
        counts.count_one(counted);
        for family in ContentFamilyNet::ALL {
            let expected = u64::from(family == counted);
            assert_eq!(
                *counts.count_of(family),
                expected,
                "a change to {counted:?} must move that family's tally and no other's — a wait \
                 parked on {family:?} would otherwise answer on a registry it never named",
            );
        }
    }
}
