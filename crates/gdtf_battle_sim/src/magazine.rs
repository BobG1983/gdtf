//! The E4.4 **ammo state + shared `can_fire` guard set** — the two pieces the HUD
//! fire button and the E4.5 `fire()` act both reason over.
//!
//! Two things land here (`docs/combat/resolution.md` §"What's pure math vs sim":
//! "ammo clamp"; §9 `can_stabilize` precedent: "button and act share one guard
//! set"):
//!
//! 1. The current-rounds AMMO state — a [`Magazine`] Component carrying the rounds
//!    currently loaded, **clamped at construction** by the weapon's
//!    [`MagazineSize`](crate::weapon::MagazineSize) (a weapon NUMBER; the current
//!    rounds are battle-local state). A **saturating** per-round decrement
//!    ([`Magazine::spend_round`], floors at `0`, never underflows) and the
//!    burst-clamp primitive [`clamp_burst`] (the round count `fire()` may actually
//!    loop = `min(mode shots, rounds left)`). **Reload boundary:** this slice ships
//!    ONLY the ammo state + the per-round decrement + the burst clamp — a standalone
//!    `reload()` act with the `reload_tu` refill (resolution.md L166 names
//!    `reload_tu` tunable, but no `reload_tu` tuning leaf exists yet and a reload
//!    ACT is not in GTW-9's scope) is OUT of E4, noted here, not built.
//!
//! 2. The [`can_fire`] GUARD SET — the validation the HUD button and `fire()`
//!    SHARE (one guard set, the §9 `can_stabilize` precedent). It returns `true`
//!    iff ALL hold: the shooter is [`LifeState::Alive`](crate::ganger::LifeState::Alive);
//!    affords the selected mode's TU charge ([`mode_tu_cost`], checked via E4.0
//!    [`can_spend_tu`](crate::tu::can_spend_tu)); has ammo
//!    ([`Magazine`] rounds ≥ 1); and the target `(cell, level)` is
//!    [`in_bounds`]. **LOS/fog is explicitly NOT a `can_fire` input** — the
//!    has-LOS / fog gate is PLAYER POLICY in the presenter (resolution.md §"What's
//!    pure math vs sim": fog "never enters the shared act"); `can_fire` validates
//!    only alive + TU + ammo + in-bounds, taking no `has_los`/visibility argument.
//!
//! The TU charge ([`mode_tu_cost`]) is the **single source** both `can_fire` and
//! the E4.5 `fire()` debit read, so the affordability check and the actual charge
//! can never diverge. Pure math, no [`World`](bevy::ecs::world::World) access —
//! render-free, **zero pixels**.

use bevy::prelude::{Component, Deref};

use crate::{
    ganger::{Aiming, LifeState, Tu, TuMax},
    metric::{Cell, Level, MAX_LEVELS},
    occupancy::{GRID_HEIGHT, GRID_WIDTH},
    tu::can_spend_tu,
    tuning::CombatTuning,
    weapon::{FireModeSpec, MagazineSize, ModeShots},
};

/// A ganger's **magazine** — the rounds currently loaded in their weapon.
///
/// The battle-local AMMO state (resolution.md §"What's pure math vs sim": the
/// "ammo clamp" `fire()` honors): the magazine's *capacity* is a weapon NUMBER
/// ([`MagazineSize`](crate::weapon::MagazineSize)), while the rounds *currently*
/// loaded are this per-ganger state, clamped at construction by that capacity
/// (never exceeds it). [`spend_round`](Magazine::spend_round) drains it one round
/// at a time (saturating); [`clamp_burst`] reads it to bound a burst's shot count.
///
/// A `u16` count (matching [`MagazineSize`](crate::weapon::MagazineSize)). Private
/// inner + derived [`Deref`]; a distinct Component so a shot / HUD system can query
/// `&Magazine` alone. Build it with [`Magazine::loaded`] (full) or
/// [`Magazine::new`] (a partial load, clamped). Defaults to `0` (empty — a fresh
/// ganger carries no ammo until the situation setup loads a magazine; a structural
/// spawn default, not a balance value).
#[derive(Deref, Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Magazine(u16);

impl Magazine {
    /// Build a magazine loaded with `rounds`, **clamped** to the weapon's
    /// [`MagazineSize`](crate::weapon::MagazineSize) capacity.
    ///
    /// The general constructor: a request above capacity is clamped down to
    /// capacity (`min(rounds, *size)`) — the magazine can never hold more than its
    /// weapon's size — and a request within capacity is preserved exactly
    /// (AC1). Use [`loaded`](Magazine::loaded) for a full magazine.
    #[must_use]
    pub fn new(rounds: u16, size: MagazineSize) -> Self {
        Self(rounds.min(*size))
    }

    /// Build a **full** magazine — loaded to the weapon's
    /// [`MagazineSize`](crate::weapon::MagazineSize) capacity.
    ///
    /// The convenience constructor for a freshly-reloaded weapon at full capacity
    /// ([`new`](Magazine::new) with `rounds == *size`).
    #[must_use]
    pub fn loaded(size: MagazineSize) -> Self {
        Self(*size)
    }

    /// Spend **one** round — a **saturating** decrement that floors at `0`.
    ///
    /// The per-round primitive the E4.5 `fire()` burst loop calls once per fired
    /// round (resolution.md §"What's pure math vs sim": `fire()` "per-round spend").
    /// Uses [`u16::saturating_sub`]: spending a round from an already-empty
    /// magazine leaves it at `0` — **never** underflows / wraps to `~65535` (AC2).
    pub const fn spend_round(&mut self) {
        self.0 = self.0.saturating_sub(1);
    }
}

/// Clamp a fire mode's shot count to the rounds actually in the magazine — the
/// burst-clamp primitive the E4.5 `fire()` burst loop runs.
///
/// `fire()` can never loop more rounds than are loaded (resolution.md §"What's pure
/// math vs sim": the "ammo clamp"): the looped count is `min(mode shots, rounds
/// left)`. Returns a [`ModeShots`](crate::weapon::ModeShots) so the bounded count
/// keeps its per-mode meaning. With an empty magazine the result is `0` (no round
/// fires); within ammo the mode's full shot count passes through unchanged.
#[must_use]
pub fn clamp_burst(shots: ModeShots, mag: &Magazine) -> ModeShots {
    ModeShots::new((*shots).min(**mag))
}

/// Round a non-negative `f32` TU charge into the unsigned [`Tu`] inner type
/// (`u8`), clamping into the `u8` range so a wild product can never wrap or lose
/// its sign.
///
/// The mode TU charge is a tuning `f32` product (`TuMax` × `ModeTuPercent` × the
/// aim premium); the TU pool is a `u8`. Rounding is **not** pinned by the design (it
/// is unspecified tuning detail) — this uses round-half-away-from-zero
/// ([`f32::round`]). A value below `0` clamps to `0` and above `u8::MAX` to
/// `u8::MAX`. The clamp + localized `#[expect]` is the crate's guarded-cast idiom
/// (see [`crate::resolve_hit`]'s `round_to_i32` / [`crate::apply_hit`]'s
/// `hp_damage_to_u16`), so no `unwrap`/`expect` is needed.
fn charge_to_u8(charge: f32) -> u8 {
    let rounded = charge.round();
    #[expect(
        clippy::cast_sign_loss,
        clippy::cast_possible_truncation,
        reason = "clamped into [0.0, u8::MAX] first, so the cast can neither wrap nor lose a sign; fractional part is gone after round"
    )]
    let clamped = rounded.clamp(0.0, f32::from(u8::MAX)) as u8;
    clamped
}

/// The TU charge a single shot in `mode` costs the shooter — the **shared source**
/// both [`can_fire`]'s affordability check and the E4.5 `fire()` debit read.
///
/// The per-shot charge (resolution.md §1: per-`FireMode` TU%; §1a: the aim ×1.5
/// premium): `round(TuMax × ModeTuPercent × aim_premium)`, where `aim_premium` is
/// the tuning [`AimTuPremium`](crate::tuning::AimTuPremium) **only when aiming**
/// (`1.0` hip-fired). Defining it once here keeps the affordability test and the
/// actual debit from ever diverging. Returns a [`Tu`] (the guarded `f32→u8` cast
/// via [`charge_to_u8`] — saturating, no `unwrap`); aiming costs strictly more than
/// hip-fire by the `AimTuPremium` factor (resolution.md §1a, default ×1.5).
#[must_use]
pub fn mode_tu_cost(
    mode: &FireModeSpec,
    tu_max: &TuMax,
    aiming: &Aiming,
    tuning: &CombatTuning,
) -> Tu {
    let aim_premium = if **aiming {
        *tuning.cone_stability.aim_mode.tu_premium
    } else {
        1.0
    };
    let charge = f32::from(**tu_max) * *mode.tu_percent * aim_premium;
    Tu::new(charge_to_u8(charge))
}

/// Whether a target `(cell, level)` is **inside** the coarse grid extent — the
/// `can_fire` in-bounds predicate.
///
/// The x/y bound is sourced from [`GRID_WIDTH`](crate::occupancy::GRID_WIDTH) /
/// [`GRID_HEIGHT`](crate::occupancy::GRID_HEIGHT) (the 60-cell ground extent's real
/// home in `occupancy.rs`, a STRUCTURAL grid constant — not a metric constant), and
/// the z bound from [`MAX_LEVELS`](crate::metric::MAX_LEVELS) (the 8-storey home in
/// `metric.rs`): `cell.x ∈ 0..GRID_WIDTH`, `cell.y ∈ 0..GRID_HEIGHT`,
/// `level ∈ 0..MAX_LEVELS`. Negative x/y fail (a [`Cell`] wraps a signed `IVec2`)
/// because the `usize::try_from` of a negative coordinate is `Err` — mirroring
/// [`OccupancyGrid`](crate::occupancy::OccupancyGrid)'s own bounds-check choke
/// point, so the firing guard and the occupancy buffer agree on the grid edge.
#[must_use]
pub fn in_bounds(cell: Cell, level: Level) -> bool {
    // Cell coordinates are signed (IVec2); a negative axis is out of bounds and
    // `usize::try_from` rejects it, so no `usize as i32` cast (which would trip
    // `cast_possible_wrap`) is ever needed.
    let Ok(x) = usize::try_from(cell.x) else {
        return false;
    };
    let Ok(y) = usize::try_from(cell.y) else {
        return false;
    };
    x < GRID_WIDTH && y < GRID_HEIGHT && (*level as usize) < MAX_LEVELS as usize
}

/// The shooter read-state [`can_fire`] reasons over — the ganger components a
/// firing decision depends on, bundled into one named record.
///
/// Grouping these five borrowed components keeps [`can_fire`] under clippy's
/// argument-count gate (the [`crate::aim::Shooter`] /
/// [`crate::resolve_coarse::ShotInputs`] bundle precedent), and states the
/// shooter's contribution to the firing guard as one value rather than five loose
/// params. Every field is a borrowed named domain newtype (no bare primitive); the
/// bundle is a transparent borrow record, not itself a wrapped domain scalar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FireActor<'a> {
    /// The shooter's [`LifeState`] — `can_fire` requires
    /// [`Alive`](crate::ganger::LifeState::Alive) (a Downed/Dead ganger cannot
    /// fire).
    pub life:     &'a LifeState,
    /// The shooter's current [`Tu`] pool — must afford the mode's TU charge.
    pub tu:       &'a Tu,
    /// The shooter's [`TuMax`] — the round-start ceiling the per-shot charge is a
    /// percentage of.
    pub tu_max:   &'a TuMax,
    /// The shooter's [`Aiming`] flag — selects whether the aim TU premium applies
    /// to the charge.
    pub aiming:   &'a Aiming,
    /// The shooter's [`Magazine`] — must hold at least one round.
    pub magazine: &'a Magazine,
}

/// Whether a shooter **can fire** the selected mode at a target — the guard set the
/// HUD fire button and the E4.5 `fire()` act SHARE (resolution.md §9
/// `can_stabilize` precedent: "button and act share one guard set").
///
/// Returns `true` iff ALL hold (AC3): the shooter is
/// [`Alive`](crate::ganger::LifeState::Alive) (AC4 — Downed/Dead fails); affords
/// the mode's TU charge ([`mode_tu_cost`], via E4.0
/// [`can_spend_tu`](crate::tu::can_spend_tu) — AC5, and the aiming case costs
/// strictly more by the [`AimTuPremium`](crate::tuning::AimTuPremium)); has at
/// least one round loaded (AC6 — an empty [`Magazine`] fails); and the target
/// `(target_cell, target_level)` is [`in_bounds`] (AC7).
///
/// **It takes NO `has_los`/visibility input** (AC8): LOS/fog is PLAYER POLICY in
/// the presenter (resolution.md §"What's pure math vs sim": fog "never enters the
/// shared act"), so an alive, affordable, loaded, in-bounds shooter passes here
/// regardless of any LOS state — the presenter applies its own fog gate on top.
#[must_use]
pub fn can_fire(
    actor: &FireActor,
    mode: &FireModeSpec,
    target_cell: Cell,
    target_level: Level,
    tuning: &CombatTuning,
) -> bool {
    *actor.life == LifeState::Alive
        && can_spend_tu(
            actor.tu,
            mode_tu_cost(mode, actor.tu_max, actor.aiming, tuning),
        )
        && **actor.magazine >= 1
        && in_bounds(target_cell, target_level)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::weapon::{ModeConeMult, ModeName, ModeTuPercent};

    /// A fire-mode spec with an arbitrary (non-pinned) TU% — the per-mode magnitude
    /// is tuning, so tests never assert it; they assert RELATIONS over it.
    fn mode(tu_percent: f32, shots: u16) -> FireModeSpec {
        FireModeSpec::new(
            ModeName::new("single".to_owned()),
            ModeConeMult::new(1.0),
            ModeTuPercent::new(tu_percent),
            ModeShots::new(shots),
        )
    }

    /// A loaded, affordable, alive, aim-flag-controllable actor over borrowed state
    /// — the shared fixture the `can_fire` cases vary one field at a time.
    fn actor<'a>(
        life: &'a LifeState,
        tu: &'a Tu,
        tu_max: &'a TuMax,
        aiming: &'a Aiming,
        magazine: &'a Magazine,
    ) -> FireActor<'a> {
        FireActor {
            life,
            tu,
            tu_max,
            aiming,
            magazine,
        }
    }

    // AC1 — Magazine is clamped by the weapon's MagazineSize: a request above
    // capacity clamps DOWN to capacity; a request within capacity is preserved
    // exactly. (loaded() is the full-capacity convenience.)

    #[test]
    fn magazine_clamps_request_to_magazine_size() {
        let size = MagazineSize::new(12);

        // Asked for more than capacity → clamped to capacity.
        let over = Magazine::new(100, size);
        assert_eq!(
            *over, *size,
            "a request above capacity clamps to MagazineSize"
        );

        // Asked for within capacity → preserved exactly.
        let within = Magazine::new(5, size);
        assert_eq!(*within, 5, "a request within capacity is preserved exactly");

        // The exact-capacity request is preserved (the boundary).
        let exact = Magazine::new(12, size);
        assert_eq!(*exact, *size, "a request equal to capacity is preserved");

        // loaded() is the full magazine.
        let full = Magazine::loaded(size);
        assert_eq!(*full, *size, "loaded() fills to MagazineSize");
    }

    // AC2 — the per-round decrement is saturating: an empty (0-round) magazine
    // stays 0 (no underflow/wrap), a non-empty one drops by EXACTLY one round.

    #[test]
    fn spend_round_is_saturating_on_empty_and_decrements_exactly() {
        let size = MagazineSize::new(30);

        // Empty magazine: spend_round must floor at 0, never wrap to ~65535.
        let mut empty = Magazine::new(0, size);
        empty.spend_round();
        assert_eq!(*empty, 0, "spending a round from an empty magazine stays 0");

        // Non-empty magazine: spend_round drops by exactly one.
        let mut loaded = Magazine::new(3, size);
        loaded.spend_round();
        assert_eq!(
            *loaded, 2,
            "spending one round drops the count by exactly 1"
        );
        loaded.spend_round();
        assert_eq!(*loaded, 1, "and again");
        loaded.spend_round();
        assert_eq!(*loaded, 0, "down to empty");
        loaded.spend_round();
        assert_eq!(*loaded, 0, "and the empty boundary still floors at 0");
    }

    // clamp_burst — the burst-clamp primitive: bounded by the rounds left, the
    // mode's full count passes through within ammo.

    #[test]
    fn clamp_burst_bounds_shots_to_rounds_left() {
        let size = MagazineSize::new(30);

        // Fewer rounds than the burst wants → clamped to rounds left.
        let low = Magazine::new(2, size);
        assert_eq!(
            *clamp_burst(ModeShots::new(5), &low),
            2,
            "a burst is clamped to the rounds actually loaded"
        );

        // Enough rounds → the mode's full shot count passes through.
        let full = Magazine::new(10, size);
        assert_eq!(
            *clamp_burst(ModeShots::new(5), &full),
            5,
            "within ammo, the mode's full shot count passes through"
        );

        // Empty magazine → zero shots fire.
        let empty = Magazine::new(0, size);
        assert_eq!(
            *clamp_burst(ModeShots::new(5), &empty),
            0,
            "an empty magazine fires zero rounds"
        );
    }

    // AC3 — can_fire is TRUE when all guards pass: Alive + affords the mode TU +
    // >=1 round + in-bounds target.

    #[test]
    fn can_fire_true_when_all_guards_pass() {
        let tuning = CombatTuning::default();
        let size = MagazineSize::new(30);
        let m = mode(0.2, 1);

        let life = LifeState::Alive;
        let tu = Tu::new(255); // amply affords any charge
        let tu_max = TuMax::new(100);
        let aiming = Aiming::new(false);
        let magazine = Magazine::new(10, size);
        let a = actor(&life, &tu, &tu_max, &aiming, &magazine);

        assert!(
            can_fire(&a, &m, Cell::new(10, 10), Level::new(0), &tuning),
            "an alive, affordable, loaded shooter at an in-bounds target can fire"
        );
    }

    // AC4 — can_fire is FALSE when the shooter is not Alive (Downed/Dead), all
    // else affordable.

    #[test]
    fn can_fire_false_when_not_alive() {
        let tuning = CombatTuning::default();
        let size = MagazineSize::new(30);
        let m = mode(0.2, 1);

        let tu = Tu::new(255);
        let tu_max = TuMax::new(100);
        let aiming = Aiming::new(false);
        let magazine = Magazine::new(10, size);

        for life in [LifeState::Downed, LifeState::Dead] {
            let a = actor(&life, &tu, &tu_max, &aiming, &magazine);
            assert!(
                !can_fire(&a, &m, Cell::new(10, 10), Level::new(0), &tuning),
                "a {life:?} shooter cannot fire even when everything else is affordable"
            );
        }
    }

    // AC5 — can_fire is FALSE when Tu is short of the mode charge; and the AIMING
    // case requires strictly MORE TU than hip-fire (RELATION via AimTuPremium,
    // never the literal 1.5).

    #[test]
    fn can_fire_false_when_tu_short_of_mode_charge() {
        let tuning = CombatTuning::default();
        let size = MagazineSize::new(30);
        let m = mode(0.5, 1);

        let life = LifeState::Alive;
        let tu_max = TuMax::new(100);
        let aiming = Aiming::new(false);
        let magazine = Magazine::new(10, size);

        // The exact hip-fire charge for this mode.
        let charge = mode_tu_cost(&m, &tu_max, &aiming, &tuning);
        assert!(*charge > 0, "the mode charge must be positive for the test");

        // One TU short of the charge → cannot afford → false.
        let short = Tu::new(charge.saturating_sub(1));
        let a_short = actor(&life, &short, &tu_max, &aiming, &magazine);
        assert!(
            !can_fire(&a_short, &m, Cell::new(10, 10), Level::new(0), &tuning),
            "a pool one below the mode charge cannot fire"
        );

        // Exactly the charge → can afford → true (equality affords).
        let exact = Tu::new(*charge);
        let a_exact = actor(&life, &exact, &tu_max, &aiming, &magazine);
        assert!(
            can_fire(&a_exact, &m, Cell::new(10, 10), Level::new(0), &tuning),
            "a pool exactly the mode charge can fire"
        );
    }

    #[test]
    fn aiming_costs_strictly_more_tu_than_hip_fire() {
        let tuning = CombatTuning::default();
        // The aim premium is > 1 (the cost lever), so aiming costs strictly more.
        // We assert the RELATION, never the literal 1.5 magnitude.
        assert!(
            *tuning.cone_stability.aim_mode.tu_premium > 1.0,
            "precondition: the aim TU premium is a >1 cost multiplier"
        );

        let m = mode(0.5, 1);
        let tu_max = TuMax::new(100);

        let hip = mode_tu_cost(&m, &tu_max, &Aiming::new(false), &tuning);
        let aimed = mode_tu_cost(&m, &tu_max, &Aiming::new(true), &tuning);

        assert!(
            *aimed > *hip,
            "aiming must cost strictly more TU than hip-fire (charge {} vs {})",
            *aimed,
            *hip,
        );
    }

    // AC6 — can_fire is FALSE when the Magazine is empty (0 rounds), all else
    // affordable.

    #[test]
    fn can_fire_false_when_magazine_empty() {
        let tuning = CombatTuning::default();
        let size = MagazineSize::new(30);
        let m = mode(0.2, 1);

        let life = LifeState::Alive;
        let tu = Tu::new(255);
        let tu_max = TuMax::new(100);
        let aiming = Aiming::new(false);
        let empty = Magazine::new(0, size);
        let a = actor(&life, &tu, &tu_max, &aiming, &empty);

        assert!(
            !can_fire(&a, &m, Cell::new(10, 10), Level::new(0), &tuning),
            "an empty magazine cannot fire even when everything else is affordable"
        );
    }

    // AC7 — can_fire's in-bounds predicate uses GRID_WIDTH/GRID_HEIGHT (x/y) +
    // MAX_LEVELS (z): an in-grid target passes; x==GRID_WIDTH, y==GRID_HEIGHT,
    // level==MAX_LEVELS, and any negative axis each FAIL. Structural-constant pins
    // (the coordinate-system exemption — these ARE the grid's real extents).

    #[test]
    fn in_bounds_uses_grid_width_height_and_max_levels() {
        // The grid extents as i32 cell coordinates — a checked conversion
        // (GRID_WIDTH/GRID_HEIGHT are usize; a raw `as i32` trips cast_possible_wrap).
        // The fallback is unreachable for the 60-cell extents but keeps the test
        // free of unwrap/expect (denied in tests too).
        let w = i32::try_from(GRID_WIDTH).unwrap_or(i32::MAX);
        let h = i32::try_from(GRID_HEIGHT).unwrap_or(i32::MAX);

        // A target well inside the grid passes.
        assert!(
            in_bounds(Cell::new(0, 0), Level::new(0)),
            "the origin cell on storey 0 is in-bounds"
        );
        assert!(
            in_bounds(Cell::new(w - 1, h - 1), Level::new(MAX_LEVELS - 1)),
            "the top corner (GRID_WIDTH-1, GRID_HEIGHT-1, MAX_LEVELS-1) is in-bounds"
        );

        // x == GRID_WIDTH is one past the x extent → out.
        assert!(
            !in_bounds(Cell::new(w, 0), Level::new(0)),
            "x == GRID_WIDTH is out of bounds"
        );
        // y == GRID_HEIGHT is one past the y extent → out.
        assert!(
            !in_bounds(Cell::new(0, h), Level::new(0)),
            "y == GRID_HEIGHT is out of bounds"
        );
        // level == MAX_LEVELS is one past the z extent → out.
        assert!(
            !in_bounds(Cell::new(0, 0), Level::new(MAX_LEVELS)),
            "level == MAX_LEVELS is out of bounds"
        );
        // Any negative axis is out (a Cell wraps a signed IVec2).
        assert!(
            !in_bounds(Cell::new(-1, 0), Level::new(0)),
            "negative x is out of bounds"
        );
        assert!(
            !in_bounds(Cell::new(0, -1), Level::new(0)),
            "negative y is out of bounds"
        );
    }

    // AC8 — can_fire takes NO has_los/visibility input: the boundary is that LOS
    // is presenter player policy (fog "never enters the shared act"). There is no
    // LOS parameter to vary; this documents that an in-bounds, affordable, alive,
    // loaded shooter passes can_fire's signature-level inputs alone — regardless
    // of any (presenter-side) LOS state, which can_fire cannot even observe.

    #[test]
    fn can_fire_takes_no_los_input_and_passes_regardless() {
        let tuning = CombatTuning::default();
        let size = MagazineSize::new(30);
        let m = mode(0.2, 1);

        let life = LifeState::Alive;
        let tu = Tu::new(255);
        let tu_max = TuMax::new(100);
        let aiming = Aiming::new(false);
        let magazine = Magazine::new(10, size);
        let a = actor(&life, &tu, &tu_max, &aiming, &magazine);

        // can_fire's full input set is (actor, mode, cell, level, tuning) — no
        // has_los / visibility argument exists. The shooter passes on those inputs
        // alone; whether a target is "seen" is the presenter's fog gate, which the
        // model act never consults.
        assert!(
            can_fire(&a, &m, Cell::new(20, 20), Level::new(2), &tuning),
            "an alive, affordable, loaded, in-bounds shooter passes can_fire with no LOS input"
        );
    }
}
