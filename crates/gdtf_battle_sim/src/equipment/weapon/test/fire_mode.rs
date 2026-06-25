//! Relocated tests for the fire-mode model — `ModeKind` + `Display`,
//! `FireModeSpec`, and the `FireMode` selector (GTW-201; moved VERBATIM).

use super::support::*;

/// AC2 — a [`FireMode`] is a `Vec` of modes: `.iter()` yields the authored modes
/// in order, `.len()` is the count, and `single()` returns the `Single`-kind
/// spec. Built from arbitrary literals (mechanism, not magnitude).
#[test]
fn fire_mode_is_a_vec_of_modes_in_authored_order() {
    // single only
    let single_only = FireMode::new(vec![spec(1.0, 0.5, 1)]);
    assert_eq!(single_only.len(), 1, "a single-only weapon offers one mode");
    assert_eq!(single_only.single().kind, ModeKind::Single);
    assert_eq!(*single_only.single().shots, 1u16);

    // single + burst + full-auto, in authored order
    let full = FireMode::new(vec![
        kind_spec(ModeKind::Single, 1.0, 0.5, 1),
        kind_spec(ModeKind::Burst, 1.2, 0.8, 3),
        kind_spec(ModeKind::Full, 1.6, 1.0, 8),
    ]);
    assert_eq!(full.len(), 3, "a three-mode weapon offers three modes");
    // `.iter()` yields the authored modes in order.
    let kinds: Vec<ModeKind> = full.iter().map(|spec| spec.kind).collect();
    assert_eq!(
        kinds,
        vec![ModeKind::Single, ModeKind::Burst, ModeKind::Full],
        "the modes iterate in authored order",
    );
    // `single()` finds the Single-kind spec regardless of list length.
    assert_eq!(full.single().kind, ModeKind::Single);
    assert_eq!(*full.single().shots, 1u16);

    // single() falls back to the FIRST mode when no Single-kind is authored
    // (defensive — never panics on a mis-authored weapon).
    let no_single = FireMode::new(vec![kind_spec(ModeKind::Burst, 1.2, 0.8, 3)]);
    assert_eq!(
        no_single.single().kind,
        ModeKind::Burst,
        "single() falls back to the first authored mode",
    );

    // single() falls back to a structural default on an EMPTY selector.
    let empty = FireMode::new(vec![]);
    assert_eq!(
        empty.single().kind,
        ModeKind::Single,
        "single() returns a structural Single default for an empty selector",
    );
    assert_eq!(*empty.single().shots, 1u16);
}

/// AC1 — [`ModeKind`] renders its canonical human labels via [`Display`]:
/// `Single` → `"single"`, `Burst` → `"burst"`, `Full` → `"full-auto"`.
#[test]
fn mode_kind_display_labels() {
    assert_eq!(ModeKind::Single.to_string(), "single");
    assert_eq!(ModeKind::Burst.to_string(), "burst");
    assert_eq!(ModeKind::Full.to_string(), "full-auto");
}

/// AC1 — [`ModeKind`] round-trips as the closed enum through RON (parses each
/// variant by name).
#[test]
fn mode_kind_round_trips_through_ron() {
    assert_eq!(
        ron::from_str::<ModeKind>("Single").ok(),
        Some(ModeKind::Single)
    );
    assert_eq!(
        ron::from_str::<ModeKind>("Burst").ok(),
        Some(ModeKind::Burst)
    );
    assert_eq!(ron::from_str::<ModeKind>("Full").ok(), Some(ModeKind::Full));
}

/// AC2 / C5 — the [`FireMode`] selector deserializes from a **bare RON list** of
/// mode entries (each `( kind: …, cone_mult: …, tu_percent: …, shots: … )`):
/// parses a hand-written list and asserts the per-mode KINDS in order (the thing
/// under test), NOT the tunable cone/TU magnitudes (brittle-test rule).
#[test]
fn fire_mode_parses_from_a_bare_ron_list() {
    let ron = r"[
        ( kind: Single, cone_mult: 1.0, tu_percent: 0.5, shots: 1),
        ( kind: Burst,  cone_mult: 1.3, tu_percent: 0.8, shots: 3),
        ( kind: Full,   cone_mult: 1.7, tu_percent: 1.0, shots: 10),
    ]";
    let parsed = ron::from_str::<FireMode>(ron);
    assert!(
        parsed.is_ok(),
        "a FireMode selector must deserialize from a bare RON list: {parsed:?}",
    );
    let Ok(fire_mode) = parsed else {
        return;
    };
    // The authored per-mode KINDS round-trip onto each list entry, in order.
    let kinds: Vec<ModeKind> = fire_mode.iter().map(|spec| spec.kind).collect();
    assert_eq!(
        kinds,
        vec![ModeKind::Single, ModeKind::Burst, ModeKind::Full],
    );
}

/// AC2 — a [`FireModeSpec`] round-trips through serialize → deserialize unchanged
/// (`deserialize(serialize(x)) == x`): the per-mode kind and numbers survive a
/// RON serialize and re-parse. Value-equality of the whole spec, exercising the
/// serde mechanism (not a pinned tunable).
#[test]
fn fire_mode_spec_ron_round_trip_is_identity() {
    let spec = kind_spec(ModeKind::Burst, 1.3, 0.8, 3);
    let Ok(serialized) = ron::to_string(&spec) else {
        return;
    };
    let parsed = ron::from_str::<FireModeSpec>(&serialized);
    assert_eq!(
        parsed.ok(),
        Some(spec),
        "deserialize(serialize(spec)) must equal the original FireModeSpec",
    );
}

/// AC1/AC4 — a [`ModeKind`] rides on each [`FireModeSpec`] (read back via the
/// spec's `kind` field) and its human label is `kind.to_string()` (no stored
/// name string — the GTW-256 name-string newtype is gone).
#[test]
fn mode_kind_rides_on_fire_mode_spec_and_labels_via_display() {
    let spec = kind_spec(ModeKind::Full, 1.7, 1.0, 10);
    assert_eq!(
        spec.kind,
        ModeKind::Full,
        "ModeKind is a FireModeSpec field"
    );
    assert_eq!(
        spec.kind.to_string(),
        "full-auto",
        "the label derives from ModeKind's Display",
    );
}
