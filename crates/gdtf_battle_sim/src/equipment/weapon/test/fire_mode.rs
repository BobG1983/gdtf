use super::support::*;

/// AC2 — a [`FireMode`] is a `Vec` of modes: `.iter()` yields the authored modes
#[test]
fn fire_mode_is_a_vec_of_modes_in_authored_order() {
    let single_only = FireMode::new(vec![spec(1.0, 0.5, 1)]);
    assert_eq!(single_only.len(), 1, "a single-only weapon offers one mode");
    assert_eq!(single_only.single().kind, ModeKind::Single);
    assert_eq!(*single_only.single().shots, 1u16);

    let full = FireMode::new(vec![
        kind_spec(ModeKind::Single, 1.0, 0.5, 1),
        kind_spec(ModeKind::Burst, 1.2, 0.8, 3),
        kind_spec(ModeKind::Full, 1.6, 1.0, 8),
    ]);
    assert_eq!(full.len(), 3, "a three-mode weapon offers three modes");
    let kinds: Vec<ModeKind> = full.iter().map(|spec| spec.kind).collect();
    assert_eq!(
        kinds,
        vec![ModeKind::Single, ModeKind::Burst, ModeKind::Full],
        "the modes iterate in authored order",
    );
    assert_eq!(full.single().kind, ModeKind::Single);
    assert_eq!(*full.single().shots, 1u16);

    let no_single = FireMode::new(vec![kind_spec(ModeKind::Burst, 1.2, 0.8, 3)]);
    assert_eq!(
        no_single.single().kind,
        ModeKind::Burst,
        "single() falls back to the first authored mode",
    );

    let empty = FireMode::new(vec![]);
    assert_eq!(
        empty.single().kind,
        ModeKind::Single,
        "single() returns a structural Single default for an empty selector",
    );
    assert_eq!(*empty.single().shots, 1u16);
}

#[test]
fn mode_kind_display_labels() {
    assert_eq!(ModeKind::Single.to_string(), "single");
    assert_eq!(ModeKind::Burst.to_string(), "burst");
    assert_eq!(ModeKind::Full.to_string(), "full-auto");
}

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
    let kinds: Vec<ModeKind> = fire_mode.iter().map(|spec| spec.kind).collect();
    assert_eq!(
        kinds,
        vec![ModeKind::Single, ModeKind::Burst, ModeKind::Full],
    );
}

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
