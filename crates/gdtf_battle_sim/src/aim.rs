//! The E4.3 **stability / cone composers** — the two shared methods that compute
//! a shooter's stability read and dispersion-cone WIDTH off ganger state + the
//! MODEL cover ledger (`docs/combat/resolution.md` §1a + §"What's pure math vs
//! sim": "`cone_for` / `stability_for` compose `θ_cone` off ganger state + model
//! cover (the HUD stability readout reads the same method)").
//!
//! These are the **single callable surface** the HUD stability readout (GTW-11 /
//! E6) reads and the E4.5 `fire()` burst loop reuses per round — defined once
//! here so the two never re-derive the math. They **wrap** the landed E2 pipeline,
//! composing nothing new:
//!
//! - [`stability_for`] finds the faced cell via E4.2
//!   [`crate::faced_cell::faced_cell`], peeks the model [`crate::cover::CoverLedger`]
//!   there for the faced [`crate::cover::CoverEntry`] (`None` when no cover is
//!   faced), and calls the landed [`crate::stability::stability`] verbatim — the
//!   §1a brace gate engages exactly when the faced cover's
//!   [`crate::cover::HeightBand`] satisfies the per-stance gate (already inside
//!   `stability`).
//! - [`cone_for`] composes the full §1a chain by calling [`stability_for`] for
//!   the `(cone_mult, recoil_growth)` pair, [`crate::cone::aim_cone_mult`] for the
//!   aim term, then [`crate::cone::cone_angle`] over the five factors verbatim.
//!
//! The composers READ the model cover ledger via
//! [`CoverLedger::peek`](crate::cover::CoverLedger::peek) — it is never rebuilt
//! (the change-driven contract). Angular / dimensionless — **zero pixels**: every
//! output is the abstract stability multiplier or the angular [`ConeAngle`], never
//! a pixel.
//!
//! ## The weapon-intrinsic-stability gap
//!
//! The §1a stability score includes a **weapon-intrinsic** contribution
//! ([`WeaponStability`], resolution.md §1a: "weapon intrinsic + stance + …"). The
//! [`crate::weapon::Weapon`] data record has **no** intrinsic-stability field today
//! (its fields are `base_spread` / `accuracy` / `kickback` / `fatal_bias` /
//! `damage` / `punch` / `shred` / `damage_type` / `magazine_size` / `fire_mode`).
//! So both composers take the [`WeaponStability`] contribution as an **explicit
//! caller-supplied input** — they do NOT add a field to the `Weapon` struct (a
//! separately-filed ticket) and do NOT fabricate or default the value. Once the
//! `Weapon`-field ticket lands, the caller will read it off the weapon instead of
//! passing it here; the composer signatures need not change.

use crate::{
    cone::{ConeAngle, PriorShots, aim_cone_mult, cone_angle},
    cover::CoverLedger,
    faced_cell::faced_cell,
    ganger::{Aiming, Facing, Position, Stance},
    metric::CellLevel,
    stability::{ConeMult, EmplacementStability, RecoilGrowth, WeaponStability, stability},
    tuning::CombatTuning,
    weapon::{FireModeSpec, Weapon},
};

/// The shooter read-state both composers reason over — the ganger components a
/// shot's stability and cone width depend on, bundled into one named record.
///
/// Grouping these four borrowed components keeps [`stability_for`] /
/// [`cone_for`] under clippy's argument-count gate (the
/// [`crate::resolve_coarse::ShotInputs`] bundle precedent), and states the
/// shooter's contribution to a shot as one value rather than four loose params.
/// Every field is a borrowed named ganger [`crate::ganger`] component (no bare
/// primitive); the bundle is a transparent borrow record, not itself a wrapped
/// domain scalar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Shooter<'a> {
    /// The shooter's [`Stance`] — the §1a per-stance stability contribution and
    /// the brace gate's per-stance min-height band.
    pub stance:   &'a Stance,
    /// The shooter's [`Aiming`] flag — selects the §1a aim cone multiplier
    /// (aimed ×0.6 / hip-fired ×1).
    pub aiming:   &'a Aiming,
    /// The shooter's grid [`Position`] — the origin the faced cell is stepped from.
    pub position: &'a Position,
    /// The shooter's [`Facing`] — the direction the faced cell is stepped along.
    pub facing:   &'a Facing,
}

/// Compose a shooter's §1a stability read off its ganger state and the model
/// cover ledger — the shared method the HUD stability readout and E4.5 `fire()`
/// both call (`docs/combat/resolution.md` §"What's pure math vs sim").
///
/// Finds the faced cell via E4.2 [`faced_cell`] (one step along the shooter's
/// [`Facing`] from its [`Position`], on its own storey), peeks the model
/// [`CoverLedger`] there for the faced [`crate::cover::CoverEntry`] (`None` when
/// no cover is faced — the ledger is read via
/// [`CoverLedger::peek`](crate::cover::CoverLedger::peek), never rebuilt), and
/// calls the landed [`stability`] verbatim with that cover, the shooter's
/// [`Stance`], the caller-supplied `weapon_intrinsic` contribution, and the
/// emplacement seam ([`EmplacementStability::none`] — no emplacement entities
/// exist yet). Returns the `(cone_mult, recoil_growth)` pair the §1a cone chain
/// reads.
///
/// This **wraps** [`stability`], re-deriving none of the §1a math: the brace gate
/// engages exactly when the faced cover's [`crate::cover::HeightBand`] satisfies
/// the per-stance gate, already inside `stability`. Angular / dimensionless —
/// **zero pixels**.
///
/// `weapon_intrinsic` is the weapon's §1a intrinsic-stability contribution, a
/// **caller-supplied** input: the [`Weapon`] record carries no such field today,
/// so the caller passes it explicitly pending the `Weapon`-field ticket (see the
/// module docs). Every coefficient and both curves come from `tuning`'s
/// `cone_stability` sub-field; nothing tunable is hardcoded.
#[must_use]
pub fn stability_for(
    shooter: &Shooter,
    weapon_intrinsic: WeaponStability,
    cover: &CoverLedger,
    tuning: &CombatTuning,
) -> (ConeMult, RecoilGrowth) {
    // The faced cell (E4.2) — one horizontal step along the shooter's facing, on
    // its own storey; the cell whose cover the §1a brace gate reads.
    let (cell, level) = faced_cell(shooter.position, shooter.facing);
    // Peek the model ledger for the faced cover (read-only, never rebuilt — the
    // change-driven contract). `None` ⇒ no cover faced ⇒ the brace is withheld.
    let faced = cover.peek(&CellLevel::new(cell, level));
    // Wrap the landed verb verbatim — no emplacements exist yet, so pass the
    // identity emplacement seam.
    stability(
        weapon_intrinsic,
        *shooter.stance,
        faced,
        EmplacementStability::none(),
        &tuning.cone_stability,
    )
}

/// Compose a shooter's §1a dispersion-cone WIDTH `θ_cone` off its ganger state,
/// weapon / fire-mode data, the burst's prior-shot count, and the model cover —
/// the shared method the HUD readout and E4.5 `fire()` both call
/// (`docs/combat/resolution.md` §1a; §"What's pure math vs sim").
///
/// Composes the full §1a chain by **wrapping** the landed verbs, re-deriving none
/// of the math:
///
/// 1. [`stability_for`] for the `(cone_mult, recoil_growth)` pair (which itself
///    finds the faced cell and reads the model cover).
/// 2. [`aim_cone_mult`] over the shooter's [`Aiming`] flag for the aim term
///    (aimed ×0.6 / hip-fired ×1, read from `tuning`).
/// 3. [`cone_angle`] over the five §1a factors: the weapon's [`BaseSpread`], the
///    fire mode's [`ModeConeMult`], the burst's [`PriorShots`], the weapon's
///    [`Kickback`], the `recoil_growth` + `cone_mult` from step 1, and the aim
///    term from step 2.
///
/// Returns the named [`ConeAngle`] (radians — angular, **zero pixels**); every
/// factor magnitude comes from weapon / fire-mode / tuning data, none hardcoded.
///
/// `weapon_intrinsic` is the §1a weapon-intrinsic stability contribution, a
/// **caller-supplied** input (the [`Weapon`] record carries no such field today —
/// see the module docs / [`stability_for`]). `mode` is the selected fire mode's
/// [`FireModeSpec`] (its [`crate::weapon::ModeConeMult`] is the firemode term);
/// `prior_shots` is the count of rounds already fired this action (zero on the
/// first round → the recoil term is the identity ×1).
///
/// [`BaseSpread`]: crate::weapon::BaseSpread
/// [`Kickback`]: crate::weapon::Kickback
/// [`ModeConeMult`]: crate::weapon::ModeConeMult
#[must_use]
pub fn cone_for(
    shooter: &Shooter,
    weapon: &Weapon,
    weapon_intrinsic: WeaponStability,
    mode: &FireModeSpec,
    prior_shots: PriorShots,
    cover: &CoverLedger,
    tuning: &CombatTuning,
) -> ConeAngle {
    // Step 1 — the stability read (faced cell + model cover, inside stability_for).
    let (cone_mult, recoil_growth) = stability_for(shooter, weapon_intrinsic, cover, tuning);
    // Step 2 — the aim term off the shooter's Aiming flag (read from tuning).
    let aim = aim_cone_mult(*shooter.aiming, &tuning.cone_stability);
    // Step 3 — fold the five §1a factors into θ_cone via the landed cone_angle.
    cone_angle(
        weapon.base_spread,
        mode.cone_mult,
        prior_shots,
        weapon.kickback,
        recoil_growth,
        cone_mult,
        aim,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        armor::{ArmorHardness, ArmorProtection},
        cover::{CoverEntry, CoverHp, HeightBand},
        ganger::{Direction, StanceKind},
        metric::{Cell, Level},
        weapon::{
            Accuracy, BaseSpread, DamageType, FatalBias, FireMode, Kickback, MagazineSize,
            ModeConeMult, ModeShots, ModeTuPercent, WeaponDamage, WeaponDamageProfile, WeaponPunch,
            WeaponShred,
        },
    };

    /// Build a shooter read-state from owned components the test holds — the
    /// borrows the [`Shooter`] bundle wants are taken from these locals.
    struct ShooterState {
        stance:   Stance,
        aiming:   Aiming,
        position: Position,
        facing:   Facing,
    }

    impl ShooterState {
        /// An arbitrary shooter at `(x, y, storey)` with the given posture / aim /
        /// facing — NOT shipped magnitudes; only its components matter.
        fn new(x: i32, y: i32, storey: u8, kind: StanceKind, aiming: bool, dir: Direction) -> Self {
            Self {
                stance:   Stance::new(kind),
                aiming:   Aiming::new(aiming),
                position: Position::new(CellLevel::new(Cell::new(x, y), Level::new(storey))),
                facing:   Facing::new(dir),
            }
        }

        /// Borrow the owned components as a [`Shooter`] bundle.
        fn as_shooter(&self) -> Shooter<'_> {
            Shooter {
                stance:   &self.stance,
                aiming:   &self.aiming,
                position: &self.position,
                facing:   &self.facing,
            }
        }
    }

    /// An arbitrary faced-cover entry at `band` — NOT shipped magnitudes; the
    /// stability layer only reads `height_band`, so the HP / armor are filler.
    fn cover_entry(band: HeightBand) -> CoverEntry {
        CoverEntry::seeded(
            CoverHp::new(10),
            band,
            ArmorProtection::new(1),
            ArmorHardness::new(1),
        )
    }

    /// An arbitrary single-mode weapon — NOT shipped magnitudes (there are no
    /// shipped weapons yet); only its `base_spread` / `kickback` / `fire_mode`
    /// flow through the composers.
    fn weapon(base: f32, kick: f32) -> Weapon {
        Weapon::new(
            BaseSpread::new(base),
            Accuracy::new(1.0),
            Kickback::new(kick),
            FatalBias::new(0.0),
            WeaponDamageProfile::new(
                WeaponDamage::new(10),
                WeaponPunch::new(2),
                WeaponShred::new(1),
                DamageType::Kinetic,
            ),
            MagazineSize::new(10),
            FireMode::Single {
                single: FireModeSpec::new(
                    ModeConeMult::new(1.0),
                    ModeTuPercent::new(0.5),
                    ModeShots::new(1),
                ),
            },
        )
    }

    /// Insert `entry` into a fresh ledger at the cell `shooter` faces, so the
    /// composer's `peek` finds it as the faced cover.
    fn ledger_with_faced_cover(shooter: &Shooter, entry: CoverEntry) -> CoverLedger {
        let (cell, level) = faced_cell(shooter.position, shooter.facing);
        let mut ledger = CoverLedger::new();
        ledger.insert(CellLevel::new(cell, level), entry);
        ledger
    }

    /// AC1 — `stability_for` returns the SAME `(ConeMult, RecoilGrowth)` as a
    /// direct [`stability`] call with the same `(weapon_intrinsic, stance, faced
    /// cover, emplacement, tuning)`: bit-equality proving it WRAPS (does not
    /// re-derive) the landed verb. A `CoverEntry` is inserted at the faced cell so
    /// the direct call's `faced` argument is exactly what the composer peeks.
    #[test]
    fn stability_for_bit_equals_a_direct_stability_call() {
        let tuning = CombatTuning::default();
        let state = ShooterState::new(20, 20, 2, StanceKind::Standing, false, Direction::East);
        let shooter = state.as_shooter();
        let intrinsic = WeaponStability::new(0.0);

        // Cover at the faced cell so the composer's peek returns Some(entry).
        let entry = cover_entry(HeightBand::High);
        let ledger = ledger_with_faced_cover(&shooter, entry);

        let (via_composer_cone, via_composer_recoil) =
            stability_for(&shooter, intrinsic, &ledger, &tuning);

        // The direct call with the EXACT same inputs the composer fed the verb.
        let (cell, level) = faced_cell(shooter.position, shooter.facing);
        let faced = ledger.peek(&CellLevel::new(cell, level));
        let (direct_cone, direct_recoil) = stability(
            intrinsic,
            *shooter.stance,
            faced,
            EmplacementStability::none(),
            &tuning.cone_stability,
        );

        assert_eq!(
            (*via_composer_cone).to_bits(),
            (*direct_cone).to_bits(),
            "stability_for must bit-equal a direct stability call (cone_mult)",
        );
        assert_eq!(
            (*via_composer_recoil).to_bits(),
            (*direct_recoil).to_bits(),
            "stability_for must bit-equal a direct stability call (recoil_growth)",
        );
    }

    /// AC2 — a faced cell whose cover band satisfies the per-stance brace gate
    /// yields a strictly STEADIER (smaller `ConeMult`) `stability_for` than the
    /// same shooter facing an EMPTY cell — RELATION, never a pinned score. A
    /// standing shooter's gate is HIGH (resolution.md §1a), so a HIGH faced wall
    /// braces; an empty ledger (no faced cover) withholds the brace.
    #[test]
    fn brace_at_faced_cell_is_steadier_than_an_empty_cell() {
        let tuning = CombatTuning::default();
        let state = ShooterState::new(15, 15, 1, StanceKind::Standing, false, Direction::South);
        let shooter = state.as_shooter();
        let intrinsic = WeaponStability::new(0.0);

        // Braced: a HIGH wall at the faced cell satisfies the standing gate.
        let braced_ledger = ledger_with_faced_cover(&shooter, cover_entry(HeightBand::High));
        let (braced_cone, _) = stability_for(&shooter, intrinsic, &braced_ledger, &tuning);

        // Unbraced: an empty ledger — no cover at the faced cell.
        let empty = CoverLedger::new();
        let (empty_cone, _) = stability_for(&shooter, intrinsic, &empty, &tuning);

        assert!(
            *braced_cone < *empty_cone,
            "a satisfying faced band must brace (steadier, lower cone_mult) vs an empty cell: \
             braced {} vs empty {}",
            *braced_cone,
            *empty_cone,
        );
    }

    /// AC3 — `cone_for` returns a `ConeAngle` bit-equal to a hand-composed
    /// [`cone_angle`] over the same five §1a factors (the weapon's `base_spread`,
    /// the mode's `cone_mult`, the burst's `prior_shots`, the weapon's `kickback`,
    /// the `recoil_growth` and `cone_mult` from `stability_for`, and the aim mult):
    /// value-agnostic on the inputs, proving the composition WRAPS.
    #[test]
    fn cone_for_bit_equals_a_hand_composed_cone_angle() {
        let tuning = CombatTuning::default();
        let state = ShooterState::new(8, 8, 0, StanceKind::Crouching, true, Direction::West);
        let shooter = state.as_shooter();
        let intrinsic = WeaponStability::new(0.0);
        let wpn = weapon(0.2, 0.1);
        let mode = wpn.fire_mode.single();
        let prior = PriorShots::new(2);

        // Cover at the faced cell (a MID wall — kneel's gate is MID) so the
        // stability term reflects a real faced lookup, not just an empty path.
        let ledger = ledger_with_faced_cover(&shooter, cover_entry(HeightBand::Mid));

        let via_composer = cone_for(&shooter, &wpn, intrinsic, &mode, prior, &ledger, &tuning);

        // Hand-compose: the same stability_for pair + the same aim term + cone_angle.
        let (cone_mult, recoil_growth) = stability_for(&shooter, intrinsic, &ledger, &tuning);
        let aim = aim_cone_mult(*shooter.aiming, &tuning.cone_stability);
        let hand = cone_angle(
            wpn.base_spread,
            mode.cone_mult,
            prior,
            wpn.kickback,
            recoil_growth,
            cone_mult,
            aim,
        );

        assert_eq!(
            (*via_composer).to_bits(),
            (*hand).to_bits(),
            "cone_for must bit-equal a hand-composed cone_angle over the five §1a factors",
        );
    }

    /// AC4 — aimed fire (`Aiming(true)`) yields a strictly NARROWER `cone_for`
    /// than hip-fire (`Aiming(false)`), all else equal — the ×0.6 via
    /// [`aim_cone_mult`], asserted by RELATION (aimed < hip), never the literal
    /// 0.6. Two shooters differ ONLY in the aim flag.
    #[test]
    fn aimed_cone_for_is_strictly_narrower_than_hip_fire() {
        let tuning = CombatTuning::default();
        let wpn = weapon(0.2, 0.1);
        let mode = wpn.fire_mode.single();
        let prior = PriorShots::first();
        let intrinsic = WeaponStability::new(0.0);

        // Same posture / position / facing — only the aim flag differs.
        let aimed_state =
            ShooterState::new(10, 10, 0, StanceKind::Standing, true, Direction::North);
        let hip_state = ShooterState::new(10, 10, 0, StanceKind::Standing, false, Direction::North);
        let aimed = aimed_state.as_shooter();
        let hip = hip_state.as_shooter();

        // Same (empty) cover for both — the only difference is aim.
        let ledger = CoverLedger::new();

        let aimed_cone = cone_for(&aimed, &wpn, intrinsic, &mode, prior, &ledger, &tuning);
        let hip_cone = cone_for(&hip, &wpn, intrinsic, &mode, prior, &ledger, &tuning);

        assert!(
            *aimed_cone < *hip_cone,
            "aimed fire must be strictly narrower than hip-fire: aimed {} vs hip {}",
            *aimed_cone,
            *hip_cone,
        );
    }

    /// AC5 — each additional prior shot widens `cone_for` monotonically for a
    /// positive-kickback weapon (`recoil = 1 + prior_shots × kickback ×
    /// recoil_growth`): sweep `prior_shots` and assert `θ_cone` is strictly
    /// non-decreasing (the per-round widening E4.5's burst loop relies on).
    #[test]
    fn each_prior_shot_widens_cone_for_monotonically() {
        let tuning = CombatTuning::default();
        let state = ShooterState::new(5, 5, 0, StanceKind::Standing, false, Direction::East);
        let shooter = state.as_shooter();
        let intrinsic = WeaponStability::new(0.0);
        let wpn = weapon(0.2, 0.15); // positive kickback so recoil widens
        let mode = wpn.fire_mode.single();
        let ledger = CoverLedger::new();

        let mut prev = f32::NEG_INFINITY;
        for shots in 0u16..6 {
            let theta = cone_for(
                &shooter,
                &wpn,
                intrinsic,
                &mode,
                PriorShots::new(shots),
                &ledger,
                &tuning,
            );
            assert!(
                *theta >= prev,
                "θ_cone must be non-decreasing across prior shots: {} after {prev} at {shots}",
                *theta,
            );
            // Strictly increasing for positive kickback after the first round
            // (the standing/un-braced score gives a positive recoil_growth).
            if shots > 0 {
                assert!(
                    *theta > prev,
                    "a positive-kickback weapon must widen strictly per prior shot",
                );
            }
            prev = *theta;
        }
    }

    /// AC6 — both composers are the public model methods (the HUD-shared callable
    /// surface) and carry zero pixels: exercise them through the public crate API
    /// as a library caller would, asserting finite angular / dimensionless outputs
    /// (a `ConeAngle` is radians, a `ConeMult` dimensionless — never a pixel).
    #[test]
    fn composers_are_the_public_library_surface_with_zero_pixels() {
        // Reach them via the crate's public re-exports, exactly as the HUD / fire()
        // caller would (proving they are the shared public surface).
        use crate::{Shooter as PubShooter, cone_for as pub_cone_for, stability_for as pub_stab};

        let tuning = CombatTuning::default();
        let state = ShooterState::new(12, 12, 0, StanceKind::Prone, true, Direction::North);
        let stance = state.stance;
        let aiming = state.aiming;
        let position = state.position;
        let facing = state.facing;
        let shooter = PubShooter {
            stance:   &stance,
            aiming:   &aiming,
            position: &position,
            facing:   &facing,
        };
        let intrinsic = WeaponStability::new(0.0);
        let wpn = weapon(0.2, 0.1);
        let mode = wpn.fire_mode.single();
        let ledger = CoverLedger::new();

        let (cone_mult, recoil_growth) = pub_stab(&shooter, intrinsic, &ledger, &tuning);
        let theta = pub_cone_for(
            &shooter,
            &wpn,
            intrinsic,
            &mode,
            PriorShots::first(),
            &ledger,
            &tuning,
        );

        // Angular / dimensionless outputs, all finite — no pixel anywhere.
        assert!(
            (*cone_mult).is_finite(),
            "cone_mult is dimensionless, finite"
        );
        assert!(
            (*recoil_growth).is_finite(),
            "recoil_growth is dimensionless, finite"
        );
        assert!((*theta).is_finite(), "θ_cone is an angle (radians), finite");
    }
}
