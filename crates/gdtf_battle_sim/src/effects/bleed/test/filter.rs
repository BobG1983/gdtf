//! AC3/AC4 — who bleeds and who is skipped: only the Downed bleed (Alive/Dead
//! untouched), and a `Stabilized(true)` Downed ganger is skipped while a
//! `Stabilized(false)` one still bleeds.

use super::support::{
    LifeState, Stabilized, Wounds, bleed_app, bleed_rate, bleeding_count_for, life_of, wounds_of,
};

/// AC3 — only Downed gangers bleed: an Alive ganger and a Dead ganger are
/// untouched across a tick (Wounds unchanged, no `Bleeding`). Spawns one of each
/// alongside a Downed control, ticks once, and asserts the non-Downed pools are
/// unchanged while the Downed one drained.
#[test]
fn alive_and_dead_gangers_are_untouched() {
    let rate = bleed_rate();
    let start = rate.saturating_add(10);

    let mut app = bleed_app();
    let alive = app
        .world_mut()
        .spawn((Wounds::new(start), LifeState::Alive))
        .id();
    let dead = app
        .world_mut()
        .spawn((Wounds::new(start), LifeState::Dead))
        .id();
    let downed = app
        .world_mut()
        .spawn((Wounds::new(start), LifeState::Downed))
        .id();

    app.update();

    // The Alive ganger is up and fighting — no drain, no Bleeding.
    assert_eq!(
        wounds_of(&app, alive),
        start,
        "an Alive ganger must not bleed (Wounds unchanged)",
    );
    assert_eq!(
        bleeding_count_for(&app, alive),
        0,
        "no Bleeding for an Alive ganger"
    );
    assert_eq!(
        life_of(&app, alive),
        LifeState::Alive,
        "an Alive ganger stays Alive"
    );

    // The Dead ganger is a corpse — the once-only property skips it.
    assert_eq!(
        wounds_of(&app, dead),
        start,
        "a Dead ganger must not bleed (the once-only skip)",
    );
    assert_eq!(
        bleeding_count_for(&app, dead),
        0,
        "no Bleeding for a Dead ganger"
    );
    assert_eq!(
        life_of(&app, dead),
        LifeState::Dead,
        "a Dead ganger stays Dead"
    );

    // The Downed control DID bleed — proving the tick ran and only Downed drain.
    assert_eq!(
        wounds_of(&app, downed),
        start - rate,
        "the Downed control must bleed by exactly bleed_rate (the tick ran)",
    );
    assert_eq!(
        bleeding_count_for(&app, downed),
        1,
        "the Downed control must emit exactly one Bleeding",
    );
}

/// AC4 — a `Stabilized(true)` Downed ganger is SKIPPED: a tick drains nothing AND
/// any previously-lost Wounds stay lost. Spawns a Downed ganger at a partial pool
/// (modelling earlier bleeding), stabilizes it, ticks, and asserts no further drop
/// and no `Bleeding`.
#[test]
fn stabilized_downed_ganger_is_skipped() {
    let rate = bleed_rate();
    // A partial pool — Wounds already lost before stabilizing (e.g. some bled),
    // chosen above the rate so a drain (if it wrongly ran) would be measurable.
    let partial = rate.saturating_add(2);

    let mut app = bleed_app();
    let ganger = app
        .world_mut()
        .spawn((
            Wounds::new(partial),
            LifeState::Downed,
            Stabilized::new(true),
        ))
        .id();

    app.update();

    assert_eq!(
        wounds_of(&app, ganger),
        partial,
        "a stabilized ganger must lose NO further Wounds (the already-lost stay lost)",
    );
    assert_eq!(
        bleeding_count_for(&app, ganger),
        0,
        "a stabilized ganger must emit no Bleeding",
    );
    assert_eq!(
        life_of(&app, ganger),
        LifeState::Downed,
        "a stabilized ganger remains Downed",
    );
}

/// AC4 (flag value matters) — `Stabilized(false)` does NOT skip: a Downed ganger
/// carrying the flag set `false` still bleeds (the skip is the flag being present
/// AND true, not merely present). Distinguishes "has the component" from "is
/// stabilized".
#[test]
fn stabilized_false_still_bleeds() {
    let rate = bleed_rate();
    let start = rate.saturating_add(10);

    let mut app = bleed_app();
    let ganger = app
        .world_mut()
        .spawn((
            Wounds::new(start),
            LifeState::Downed,
            Stabilized::new(false),
        ))
        .id();

    app.update();

    assert_eq!(
        wounds_of(&app, ganger),
        start - rate,
        "Stabilized(false) must NOT skip — the ganger still bleeds by bleed_rate",
    );
    assert_eq!(
        bleeding_count_for(&app, ganger),
        1,
        "a Stabilized(false) Downed ganger still emits one Bleeding",
    );
}
