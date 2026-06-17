//! Shared test fixtures + the weapon-symbol re-exports for the `weapon` tests.
//!
//! The concern test files (`components` / `fire_mode` / `bundle` / `spec_registry`)
//! each glob `use super::support::*` to reach these helpers plus every weapon type
//! they exercise — the single glob the flat module's `use super::*` (resolving the
//! `weapon/mod.rs` re-exports) used to provide before the GTW-201 dir-split. The
//! fixtures are RELOCATED VERBATIM (same construction, same literals); no test logic
//! changed.

// Re-exported so each concern file's `use super::support::*` reaches the headless
// `World` harness type + every public `weapon` item the tests touch.
pub(super) use bevy::prelude::World;

pub(super) use super::super::*;

/// Build an arbitrary `Single`-kind fire-mode spec from raw literals — NOT
/// shipped magnitudes (these only exercise the type surface). For an explicit
/// kind, use [`kind_spec`].
pub(super) const fn spec(cone: f32, tu: f32, shots: u16) -> FireModeSpec {
    kind_spec(ModeKind::Single, cone, tu, shots)
}

/// Build a fire-mode spec with an explicit [`ModeKind`] — for the kind / label
/// round-trip tests (the kind is the thing under test).
pub(super) const fn kind_spec(kind: ModeKind, cone: f32, tu: f32, shots: u16) -> FireModeSpec {
    FireModeSpec::new(
        kind,
        ModeConeMult::new(cone),
        ModeTuPercent::new(tu),
        ModeShots::new(shots),
    )
}

/// Build an arbitrary damage block from raw literals — NOT shipped magnitudes.
/// Pins the damage-leaf mechanism only.
pub(super) fn profile(
    damage: i32,
    punch: i32,
    shred: i32,
    damage_type: DamageType,
) -> DamageProfile {
    DamageProfile::new(
        WeaponDamage::new(damage),
        WeaponPunch::new(punch),
        WeaponShred::new(shred),
        damage_type,
    )
}

/// Build an arbitrary handling block (magazine + single-mode selector +
/// `stable` tag) from raw literals — NOT shipped magnitudes.
pub(super) fn handling(mag: u16, stable: bool) -> HandlingProfile {
    HandlingProfile::new(
        MagazineSize::new(mag),
        FireMode::new(vec![spec(1.0, 0.5, 1)]),
        Stable::new(stable),
    )
}

/// Build an arbitrary armed-entity bundle from the four §1/§6 cone/severity
/// numbers plus a [`DamageProfile`] / [`HandlingProfile`] — NOT shipped
/// magnitudes. The construction path every weapon test fixture migrates to;
/// takes the cohesive groups so the helper stays under the argument-count gate.
pub(super) fn weapon_bundle(
    base: f32,
    accuracy: f32,
    kick: f32,
    bias: f32,
    damage: DamageProfile,
    handling: HandlingProfile,
) -> WeaponBundle {
    WeaponBundle::new(
        WeaponName::new("test-weapon".to_owned()),
        BaseSpread::new(base),
        Accuracy::new(accuracy),
        Kickback::new(kick),
        FatalBias::new(bias),
        damage,
        handling,
    )
}
