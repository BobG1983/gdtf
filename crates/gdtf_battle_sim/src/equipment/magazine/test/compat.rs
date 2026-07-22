//! The GTW-775 ammo-type COMPATIBILITY gate proofs — the [`ammo_compatible`]
//! identity rule, [`Magazine::load`] rejecting incompatible ammo (magazine left
//! entirely unchanged) / accepting compatible ammo (loaded class set + refilled),
//! and the real loader→spawn path (`ron` deserialize + `into_bundle`) threading a
//! weapon's accepted class into the spawned magazine, all gated by it.

use crate::{
    magazine::{Magazine, ReloadTu, ammo_compatible},
    weapon::{AmmoType, MagazineSize, WeaponName, WeaponSpec},
};

/// The gate rule is class IDENTITY: `ammo_compatible(accepted, candidate)` is
/// `true` iff the candidate class equals the accepted class. Exhaustive over
/// [`AmmoType::ALL`] × [`AmmoType::ALL`], so a rule change (or a new variant that
/// slips the identity) surfaces.
#[test]
fn ammo_compatible_is_ammo_class_identity() {
    for &accepted in &AmmoType::ALL {
        for &candidate in &AmmoType::ALL {
            let expected = accepted == candidate;
            assert_eq!(
                *ammo_compatible(accepted, candidate),
                expected,
                "ammo_compatible must be class identity: candidate {candidate:?} vs accepted {accepted:?}",
            );
        }
    }
}

/// An INCOMPATIBLE load is REJECTED through the real [`Magazine::load`] path: the
/// magazine is left entirely unchanged — neither the loaded class nor the round
/// count moves (so the reject never sneaks a refill in). A `Cell`-accepting
/// magazine refuses a `Slug` candidate.
#[test]
fn load_rejects_incompatible_ammo_and_leaves_magazine_unchanged() {
    // A Cell-accepting weapon's magazine, partially spent so an unwanted refill would show.
    let mut mag = Magazine::loaded_with(MagazineSize::new(12), ReloadTu::new(10), AmmoType::Cell);
    mag.spend_round();
    mag.spend_round();
    let rounds_before = mag.rounds();

    // Load ammo the weapon does NOT accept (Slug into a Cell weapon).
    let compatible = mag.load(AmmoType::Slug, AmmoType::Cell);

    assert!(
        !*compatible,
        "loading ammo the weapon does not accept is rejected by the compatibility gate",
    );
    assert_eq!(
        mag.loaded_ammo(),
        AmmoType::Cell,
        "the loaded class is UNCHANGED on a rejected load",
    );
    assert_eq!(
        *mag.rounds(),
        *rounds_before,
        "a rejected load does NOT refill — the round count is unchanged",
    );
}

/// A COMPATIBLE load succeeds through the real [`Magazine::load`] path: the loaded
/// class stays the accepted one and the magazine refills to full. A
/// `Cell`-accepting magazine takes a `Cell` candidate.
#[test]
fn load_accepts_compatible_ammo_and_refills() {
    let size = MagazineSize::new(12);
    let mut mag = Magazine::loaded_with(size, ReloadTu::new(10), AmmoType::Cell);
    mag.spend_round();
    mag.spend_round();
    assert!(!*mag.is_full(), "precondition: the magazine is depleted");

    let compatible = mag.load(AmmoType::Cell, AmmoType::Cell);

    assert!(
        *compatible,
        "loading the accepted class is compatible and succeeds",
    );
    assert_eq!(
        mag.loaded_ammo(),
        AmmoType::Cell,
        "the loaded class remains the accepted one",
    );
    assert_eq!(*mag.rounds(), *size, "a compatible load refills to full");
    assert!(*mag.is_full(), "and the magazine reads full");
}

/// The REAL loader→spawn path: a weapon authored `accepts: Cell` is deserialized
/// by the same `ron` parse the folder loader runs, resolved through the real
/// [`WeaponSpec::into_bundle`](crate::weapon::WeaponSpec::into_bundle) spawn path
/// into a [`Magazine`] loaded with the accepted class, and the compatibility gate
/// keys off that weapon's accepted class — an off-class `Slug` load is rejected, a
/// same-class `Cell` load refills. Proves the gate exercises the real magazine
/// spawned from a real spec, not a hand-built stub.
#[test]
fn spawned_magazine_loads_the_weapons_accepted_class_and_gates_by_it() {
    // The REAL loader deserialize (the same `ron` parse the folder loader runs) of a
    // weapon that ACCEPTS Cell — value-agnostic on the tunable magnitudes.
    let authored = r"(
        base_spread: 0.1, accuracy: 1.0, kickback: 0.1, fatal_bias: 1.0,
        damage: 6, punch: 2, shred: 1, damage_type: Las,
        accepts: Cell,
        magazine: ( size: 12, reload_tu: 10 ),
        fire_mode: [ ( kind: Single, cone_mult: 1.0, tu_percent: 0.3, shots: 1 ) ],
        stable: false,
        handedness: TwoHanded,
    )";
    let parsed = ron::de::from_str::<WeaponSpec>(authored);
    assert!(
        parsed.is_ok(),
        "the authored Cell-accepting weapon must parse: {parsed:?}",
    );
    let Ok(spec) = parsed else {
        return;
    };
    assert_eq!(
        spec.accepts,
        AmmoType::Cell,
        "the loader deserializes the authored `accepts: Cell` class",
    );
    // Capture the accepted class before `into_bundle` consumes the spec (it is `Copy`).
    let accepted = spec.accepts;

    // The REAL spawn resolution loads the accepted class into the spawned magazine, FULL.
    let bundle = spec.into_bundle(WeaponName::new("cell-gun".to_owned())).0;
    let mut mag = bundle.magazine;
    assert_eq!(
        mag.loaded_ammo(),
        AmmoType::Cell,
        "into_bundle spawns the magazine loaded with the weapon's ACCEPTED class",
    );
    assert_eq!(
        *mag.rounds(),
        *mag.size(),
        "into_bundle spawns the magazine full (loaded == size)",
    );

    // The gate keys off the weapon's accepted class: an off-class load is rejected
    // (no refill), a same-class load refills.
    mag.spend_round();
    assert!(
        !*mag.load(AmmoType::Slug, accepted),
        "a Slug load into a Cell-accepting weapon is rejected by the compatibility gate",
    );
    assert!(
        !*mag.is_full(),
        "the rejected load did NOT refill the magazine"
    );
    assert!(
        *mag.load(AmmoType::Cell, accepted),
        "loading the accepted Cell class succeeds",
    );
    assert!(*mag.is_full(), "the compatible load refilled the magazine");
}
