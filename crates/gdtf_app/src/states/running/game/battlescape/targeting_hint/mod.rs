//! The targeting fog HINT (GTW-11): a small battlescape-scoped [`Text`](bevy::prelude::Text)
//! node that reads the canon string **"unseen — hold your fire"** when the targeted cell is
//! NOT squad-VISIBLE, and is hidden otherwise.
//!
//! The VIEW arm of the targeting fog gate (`docs/combat/visibility.md` §"UX edges"): the
//! reticle recolours, the fire commit refuses, and this hint surfaces the reason — all three
//! driven by the SAME shared
//! [`cell_squad_visible`](gdtf_battle_presenter::cell_squad_visible) read, so the hint + the
//! act can never disagree. The hint is pure VIEW: it reads the input crate's
//! [`InspectTarget`](gdtf_battle_input::InspectTarget) hovered cell + the sim's
//! [`SquadVisibility`](gdtf_battle_sim::SquadVisibility) / occupancy / player faction, and
//! mutates ONE existing Text node in place (never respawns) — it owns no combat rule and
//! writes nothing back.

mod components;
mod plugin;
mod systems;

pub(in crate::states::running::game::battlescape) use plugin::GameBattleScapeTargetingHintScenePlugin;

// Test-support-only re-export of the hint Text marker (GTW-11), gated so the binary build stays
// `unused`/`unreachable_pub`-clean (the combat-log marker re-export chain precedent). The AC test
// names it through `crate::test_support` to assert the hint's canon text + visibility.
#[cfg(feature = "test-support")]
crate::support_use! {
    components::TargetingHintText;
}
