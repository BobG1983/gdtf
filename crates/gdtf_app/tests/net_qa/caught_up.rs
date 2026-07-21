//! T21 / T22 — the QA-net half of the gate: an inject while catching up is rejected with an
//! ACCURATE reason and the app-flow snapshot says so; `get_output` stays available.

use gdtf_app::test_support::request_available_for;
use gdtf_qa_protocol::view::RequestKindNet;

/// **T21 — the rejection and the advertisement cannot drift.** While the presenter is still
/// catching up, the act-bearing request kinds must be BOTH unavailable in the advertised set
/// AND refused by the router — because they are the same predicate.
///
/// Rejecting at ROUTE time rather than queueing matters: the pending-queue deadline is a
/// few frames, far shorter than a catch-up window, so a held inject would time out on
/// virtually every one and force a deadline retune for every other request kind too. It also
/// preserves the honest-receipt contract — a client is never told `Queued` for an act the
/// drain will silently discard.
#[test]
fn an_inject_while_catching_up_is_rejected_and_app_flow_reports_it_unavailable() {
    const IN_BATTLE: bool = true;
    const CAUGHT_UP: bool = false;

    assert!(
        !request_available_for(RequestKindNet::Inject, IN_BATTLE, CAUGHT_UP),
        "an Inject must be unavailable while the presenter is catching up — it carries a \
         battle intent, so it needs exactly what the player needs: a current screen",
    );
    assert!(
        !request_available_for(RequestKindNet::ScreenshotAfter, IN_BATTLE, CAUGHT_UP),
        "ScreenshotAfter embeds an intent, so it is gated identically to a bare Inject",
    );

    // And once caught up, both are available again — so the gate is a gate, not a wall.
    assert!(
        request_available_for(RequestKindNet::Inject, IN_BATTLE, true),
        "with the screen caught up an Inject is serviceable",
    );
    assert!(
        request_available_for(RequestKindNet::ScreenshotAfter, IN_BATTLE, true),
        "with the screen caught up a ScreenshotAfter is serviceable",
    );
}

/// **T22 — `get_output` is NEVER gated on catch-up.**
///
/// It is the QA client's observation channel. Throttling it while pacing happens would make
/// an agent unable to watch the very thing it is testing — which is the single use case the
/// output channel exists for. The same goes for the plain screenshot and the app-flow poll:
/// an agent must always be able to LOOK.
#[test]
fn get_output_stays_available_while_catching_up() {
    const IN_BATTLE: bool = true;
    const CATCHING_UP: bool = false;

    assert!(
        request_available_for(RequestKindNet::GetOutput, IN_BATTLE, CATCHING_UP),
        "get_output must stay available while the presenter is catching up — it is how an \
         agent watches the pacing happen",
    );
    assert!(
        request_available_for(RequestKindNet::GetBattleState, IN_BATTLE, CATCHING_UP),
        "the battle snapshot is a READ; it reports sim time and is never gated",
    );
    assert!(
        request_available_for(RequestKindNet::TakeScreenshot, IN_BATTLE, CATCHING_UP),
        "a plain screenshot is how an agent sees cursor time — gating it would remove the \
         only evidence tool that works during playback",
    );
    assert!(
        request_available_for(RequestKindNet::GetAppFlow, IN_BATTLE, CATCHING_UP),
        "the app-flow poll must always answer — it is how a client learns it should wait",
    );
}

/// The battle-dependent kinds still need a battle, catch-up or not — the older gate is
/// intact underneath the new one.
#[test]
fn the_battle_gate_still_applies_underneath_the_catch_up_gate() {
    const NO_BATTLE: bool = false;

    for kind in [
        RequestKindNet::Inject,
        RequestKindNet::GetBattleState,
        RequestKindNet::GetOutput,
        RequestKindNet::ScreenshotAfter,
    ] {
        assert!(
            !request_available_for(kind, NO_BATTLE, true),
            "{kind:?} needs a live battle regardless of the catch-up state",
        );
    }
    for kind in [
        RequestKindNet::Hello,
        RequestKindNet::GetAppFlow,
        RequestKindNet::TakeScreenshot,
        RequestKindNet::StartBattle,
    ] {
        assert!(
            request_available_for(kind, NO_BATTLE, false),
            "{kind:?} is serviceable in any state",
        );
    }
}
