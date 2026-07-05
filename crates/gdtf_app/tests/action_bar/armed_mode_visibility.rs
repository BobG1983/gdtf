//! GTW-272 / GTW-273 mode-panel visibility gated on an armed selection, with its bespoke armed-situation harness.

use bevy::prelude::*;
use gdtf_app::test_support::{AppState, LoadedSituation, ModePanelRoot, ModeSingleButton};
use gdtf_battle_input::SelectedShooter;
use gdtf_battle_sim::{
    Aim, Aiming, ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorName, ArmorPiece, ArmorProtection,
    ArmorRegistry, ArmorSpec, ArmorType, Cell, CellLevel, Faction, GangerName, Level, Situation,
    test_support::test_weapon_spec,
    tuning::CombatTuning,
    weapon::{FatalBias, WeaponName, WeaponRegistry, WeaponSpec},
};
use gdtf_test_utils::GdtfTestAppBuilder;
use gdtf_ui::theme::default_theme;

use super::harness::*;

// =================================================================================
// GTW-272 + GTW-273 — Mode-panel visibility: hide the Mode panel when nothing armed is
// selected (AC3). Pin-discriminating headless tests. (GTW-298 RELOCATED the Mode + Stance
// sub-panels OUT of the action bar into the weapon cluster, so the old Mode-LEFT-of-Stance
// bar-child-order test is gone — that layout is now covered by the weapon-cluster structure
// test in `weapon_panel.rs`.)
//
// AC1 (fit-content / no clipping) is largely a VISUAL check → in-engine QA per
// verification.md #3; the headless slice asserts the explicit fit-content `Node`
// fields the fix sets (no brittle pixel pin), see `action_bar_root_fits_contents`.
// =================================================================================

/// The weapon key the real-flow armed ganger references — present in [`armed_registry`].
const PLAYER_WEAPON_KEY: &str = "test-weapon";

/// The armor key the real-flow armed ganger references — present in
/// [`armed_armor_registry`] (GTW-269).
const PLAYER_ARMOR_KEY: &str = "test-armor";

/// The gang the player controls in these tests — `Situation::player_faction` defaults to
/// gang `0`, so the armed ganger is faction `0` for `auto_select_first_player_ganger` to
/// pick it via the real flow (the `battle_running_driver.rs` precedent).
const PLAYER_FACTION: u8 = 0;

/// A [`WeaponRegistry`] holding the one [`PLAYER_WEAPON_KEY`] weapon the armed real-flow
/// ganger references (stands in for the `Load`-built registry, GTW-257, so the Generation
/// setup arms the ganger). Its `fire_mode` offers a single Single mode, so an armed
/// selection yields one Mode toggle → the Mode panel is `Visible` (the
/// `battle_running_driver.rs::weapon_registry` shape).
fn armed_registry() -> WeaponRegistry {
    WeaponRegistry::new([(
        WeaponName::new(PLAYER_WEAPON_KEY.to_owned()),
        WeaponSpec {
            // Not Fatal-skewed (the action bar never resolves a wound) — the only
            // divergence from the canonical fixture.
            fatal_bias: FatalBias::new(0.0),
            ..test_weapon_spec()
        },
    )])
}

/// An arbitrary armor SPEC (distinct per-part magnitudes, NOT shipped tuning) — the suit
/// the [`PLAYER_ARMOR_KEY`] resolves to in [`armed_armor_registry`] (GTW-269, the
/// `battle_running_driver.rs::arbitrary_armor` shape).
const fn arbitrary_armor() -> ArmorSpec {
    ArmorSpec::uniform(ArmorPiece::new(
        ArmorFloor::new(0),
        ArmorProtection::new(1),
        ArmorIntegrity::new(2),
        ArmorHardness::new(3),
        ArmorType::DEFAULT,
    ))
}

/// An [`ArmorRegistry`] holding the one [`PLAYER_ARMOR_KEY`] armor suit the armed
/// real-flow ganger references (stands in for the `Load`-built registry, GTW-269, so the
/// Generation setup armors the ganger).
fn armed_armor_registry() -> ArmorRegistry {
    ArmorRegistry::new([(
        ArmorName::new(PLAYER_ARMOR_KEY.to_owned()),
        arbitrary_armor(),
    )])
}

/// A one-ganger real situation: a single armed faction-[`PLAYER_FACTION`] ganger the
/// `SetupBattleRequested` (sent on `OnEnter(Generation)`) spawns via `setup_battle` and
/// `auto_select_first_player_ganger` then selects through the REAL flow — NO hand-inserted
/// `SelectedShooter`. Its weapon key is present in [`armed_registry`], so setup arms it
/// with a `FireMode`, which `rebuild_mode_buttons` reads to show the Mode panel.
fn armed_player_situation() -> (Situation, gdtf_battle_sim::ganger::GangRegistry) {
    use gdtf_battle_sim::test_support::{GangerSpawnBuilder, SituationBuilder};
    // Routed through the canonical shared builders (GTW-324) — value-for-value identical to
    // the prior 18-field `GangerSpawn` struct literal. Every field whose value differs from a
    // `GangerSpawnBuilder` default is set explicitly: the place, the authored name, the player
    // faction, hip-fire (`aiming` false — the builder default aims), the Shooting 3.0 stat (the
    // builder default is 2.0), and the explicit `PLAYER_ARMOR_KEY` / `PLAYER_WEAPON_KEY` keys
    // (so `armed_registry` / `armed_armor_registry` still resolve). The remaining fields —
    // facing East, Standing, full vitals (HP 40, Wounds 3, TU 60), Alive, Toughness 3.0, Luck
    // 1.0 — are the builder defaults already.
    //
    // GTW-414/415: build the situation AND its synthesized GangRegistry together — the v2
    // `setup_battle` resolves the placed ganger's `(gang, member)` ref against the registry
    // (which carries this member's PLAYER_WEAPON_KEY / PLAYER_ARMOR_KEY), so the walk must
    // seed it or setup fails closed and no player ganger spawns.
    SituationBuilder::new()
        .with_ganger(
            GangerSpawnBuilder::new()
                .at(CellLevel::new(Cell::new(2, 5), Level::new(0)))
                .name(GangerName::new("Alex Mercer".to_owned()))
                .faction(Faction::new(PLAYER_FACTION))
                .aiming(Aiming::new(false))
                .aim(Aim::new(3.0))
                .armor(ArmorName::new(PLAYER_ARMOR_KEY.to_owned()))
                .weapon(WeaponName::new(PLAYER_WEAPON_KEY.to_owned()))
                .build(),
        )
        .build_with_gangs()
}

/// Builds the headless walk app exactly like [`walk_app`], but with a real
/// [`LoadedSituation`] for the Generation setup to pour into the world (so a real player
/// ganger is spawned + auto-selected), the matching [`armed_registry`] so setup can arm
/// it, AND the synthesized [`GangRegistry`](gdtf_battle_sim::ganger::GangRegistry) the v2
/// setup resolves the placed ganger's `(gang, member)` ref against (GTW-414/415). Mirrors
/// the `battle_running_driver.rs` real-flow harness.
fn walk_app_with_situation(
    situation: Situation,
    gangs: gdtf_battle_sim::ganger::GangRegistry,
) -> App {
    let mut app = GdtfTestAppBuilder::new_with_scene_support()
        .starting_in(AppState::Running)
        .build();
    app.world_mut().insert_resource(default_theme());
    app.world_mut().insert_resource(CombatTuning::default());
    app.world_mut().insert_resource(armed_registry());
    // GTW-505: the MeleeWeaponRegistry (with the `fists` default) so the Generation setup
    // arms the player ganger's melee weapon — the placed ganger authors none, so it
    // resolves to `fists`; without it `setup_battle_on_request` fails closed and the walk
    // never reaches BattleRunning.
    app.world_mut()
        .insert_resource(gdtf_battle_sim::test_support::test_melee_weapon_registry());
    // The Load-built ArmorRegistry (GTW-269) so the Generation setup armors the player
    // ganger: it references PLAYER_ARMOR_KEY, which this registry holds.
    app.world_mut().insert_resource(armed_armor_registry());
    // GTW-414/415: the synthesized gang registry the placed ganger resolves against.
    app.world_mut().insert_resource(gangs);
    app.world_mut()
        .insert_resource(LoadedSituation::new(situation));
    app
}

/// Drives the real-situation walk to `BattleRunning` and returns the app, asserting the
/// descent succeeded — the live battle where the bar is spawned AND the real auto-select
/// has run on the spawned player ganger.
fn battle_running_app_with_situation(
    situation: Situation,
    gangs: gdtf_battle_sim::ganger::GangRegistry,
) -> App {
    let mut app = walk_app_with_situation(situation, gangs);
    assert!(
        drive_to_battle_running(&mut app),
        "the real-situation walk should reach BattleScapeState::BattleRunning within {BUDGET} \
         updates; last observed BattleScapeState was {:?}",
        battlescape_state(&app),
    );
    app
}

/// The `ModePanelRoot`'s current [`Visibility`], if exactly one exists.
fn mode_panel_visibility(app: &mut App) -> Option<Visibility> {
    single_with::<ModePanelRoot>(app)
        .and_then(|panel| app.world().get::<Visibility>(panel).copied())
}

// ---------------------------------------------------------------------------------
// AC3 — the Mode panel is Hidden when nothing armed is selected, Visible when an armed
// player ganger is selected (via the REAL selection flow — no hand-inserted selection).
// ---------------------------------------------------------------------------------

/// GTW-273 AC3 (hidden) — with NO armed selection reached via the REAL flow (the empty
/// situation spawns no gangers, so `auto_select_first_player_ganger` selects nothing), the
/// `ModePanelRoot` is `Visibility::Hidden` — no empty Mode box. Pin-discriminating:
/// reverting the visibility toggle (leaving the panel `Inherited`/visible) fails this.
#[test]
fn mode_panel_hidden_when_nothing_armed_selected() {
    let mut app = battle_running_app();
    // Settle the initial selection (stays None — no gangers) + the rebuild's no-op /
    // unarmed branch, which sets the panel Hidden.
    app.update();
    app.update();

    assert_eq!(
        mode_panel_visibility(&mut app),
        Some(Visibility::Hidden),
        "with nothing armed selected, the Mode panel must be Hidden (no empty Mode box)",
    );
}

/// GTW-273 AC3 (visible) — when an armed player ganger is selected via the REAL flow (the
/// real `LoadedSituation` setup spawns + arms it, `auto_select_first_player_ganger` picks
/// it — NO hand-inserted `SelectedShooter`), the `ModePanelRoot` is `Visibility::Visible`
/// (its weapon offers a mode → a toggle to show). Pin-discriminating: reverting the
/// visibility toggle (leaving it Hidden) fails this.
#[test]
fn mode_panel_visible_when_armed_player_ganger_selected() {
    let (situation, gangs) = armed_player_situation();
    let mut app = battle_running_app_with_situation(situation, gangs);
    // Settle the real auto-select (fills SelectedShooter with the player ganger) + the
    // rebuild (.after ApplyTheme) which, for an armed selection with modes, sets Visible.
    app.update();
    app.update();

    // The selection was filled by the REAL auto-select path (not hand-inserted).
    assert!(
        app.world()
            .get_resource::<SelectedShooter>()
            .is_some_and(|s| s.is_some()),
        "precondition: the real auto-select must have filled SelectedShooter with the player \
         ganger (no hand-inserted selection)",
    );
    // The armed weapon offers exactly one mode → exactly one toggle is built.
    assert_eq!(
        count_with::<ModeSingleButton>(&mut app),
        1,
        "the armed player ganger's single-mode weapon must build one Mode toggle",
    );
    assert_eq!(
        mode_panel_visibility(&mut app),
        Some(Visibility::Visible),
        "with an armed player ganger selected, the Mode panel must be Visible",
    );
}
