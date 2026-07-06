//! Fog x storey actor visibility + the lower-storey pick exclusion (GTW-520).

use bevy::{
    app::App,
    prelude::{Entity, Visibility},
};
use gdtf_battle_presenter::{ActiveLevel, GangerSprites, Layer, cell_to_world_layered};
use gdtf_battle_sim::{
    prelude::{Cell, CellLevel, Direction, Faction, Level, Position},
    test_support::SituationBuilder,
};

use super::{harness::*, probes::*};

/// The world translation of the presenter sprite mirroring sim ganger `sim` (via the map).
fn translation_of_sim(app: &mut App, sim: Option<Entity>) -> Option<bevy::math::Vec3> {
    let sim = sim?;
    let sprite = app
        .world()
        .get_resource::<GangerSprites>()
        .and_then(|m| m.sprite_for(sim))?;
    sprite_translation(app, sprite)
}

/// The sim `Position` (as a [`CellLevel`]) of sim ganger `sim`, if it has one.
fn position_of_sim(app: &mut App, sim: Option<Entity>) -> Option<CellLevel> {
    let sim = sim?;
    let mut q = app.world_mut().query::<&Position>();
    q.get(app.world(), sim).ok().map(|p| **p)
}

/// GTW-520 C1 — a live PLAYER ganger on a storey BELOW the active view level (a LOWER drawn
/// storey) ends `Visibility::Inherited` at its OWN storey's Z, AFTER the REAL
/// ganger-visibility resolver (`resolve_ganger_visibility`, GTW-627 — the one writer of
/// ganger-sprite visibility) runs.
///
/// The setup-inserted `SquadVisibility` + the authored `player_faction` make the
/// resolver's classifier COMPOSE the fog fact (the fog resources are resident — no
/// pseudo-gate; GTW-627 C2 deleted the old `CombatTuning` witness). A player ganger is
/// always shown by the fog predicate ([`FactionRelation::OwnSquad`]), so the
/// only thing under test is the STOREY axis: pre-GTW-520 the storey-0 ganger with active=1 was
/// HARD-CUT Hidden (`pos.z == active` fails); GTW-520 widens it to drawn-band membership
/// (`0 <= 1`) so it is shown. The Z assert pins C1's "positioned via
/// `cell_to_world_layered(cell, its-own-level, Actor)`" — the lower-storey sprite draws at
/// storey-0 Z, not the active storey's, so it occludes / peeks correctly.
///
/// Pin-discriminating: reverting the classifier's band fact to the `pos.z == active` hard cut
/// leaves the storey-0 ganger Hidden at active=1 and this FAILS.
#[test]
fn player_ganger_on_lower_storey_stays_visible_at_own_z_after_fog() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let l0 = Level::new(0);
    let l0_cell = Cell::new(5, 6);
    let l0_at = CellLevel::new(l0_cell, l0);
    // One PLAYER ganger on the ground floor. `player_faction(0)` so the fog predicate treats it
    // as OwnSquad (always shown by fog); the storey axis is the only variable.
    let situation = SituationBuilder::new()
        .with_ganger(ganger_at(l0_at, 0, Direction::East))
        .player_faction(Faction::new(0))
        .slab_at(l0_at)
        .build();
    assert!(drive_setup(&mut app, situation), "setup must complete");

    let l0_sim = sim_entity_at(&mut app, l0_at);
    assert!(
        l0_sim.is_some(),
        "the ground-floor ganger must have spawned"
    );
    assert!(
        settle_actor(&mut app, l0_sim),
        "the ganger sprite must have materialized",
    );

    // Fog: the player's own cell is VISIBLE (a player is shown by fog regardless, but authoring
    // it keeps the fog sets honest). Raise the active view level to storey 1 (the ground floor
    // is now a LOWER drawn storey) and run the fog writer.
    set_fog(&mut app, &[l0_at], &[]);
    *app.world_mut().resource_mut::<ActiveLevel>() = ActiveLevel::new(Level::new(1));
    app.update();

    // C1: the ground-floor player ganger is SHOWN even though active == 1 (it is within the
    // drawn band 0..=1) — the app's REAL final visibility, with the fog writer LIVE.
    assert_eq!(
        visibility_of_sim(&mut app, l0_sim),
        Some(Visibility::Inherited),
        "a player ganger on a LOWER drawn storey (0) is shown at active level 1 — the drawn-band \
         widening, asserted through the LIVE resolver (resolve_ganger_visibility)",
    );
    // C1: it draws at its OWN storey's Z (storey 0), not the active storey's — so it occludes /
    // peeks correctly against the lower-storey terrain.
    let expected_z = cell_to_world_layered(l0_cell, l0, Layer::Actor).z;
    let drawn_z = translation_of_sim(&mut app, l0_sim).map(|t| t.z);
    assert!(
        drawn_z.is_some_and(|z| (z - expected_z).abs() < 1.0e-3),
        "the lower-storey ganger draws at its OWN storey-0 Z ({expected_z}), got {drawn_z:?} — \
         positioned via cell_to_world_layered(cell, its-own-level, Actor)",
    );
}

/// GTW-520 C2 — a live PLAYER ganger on a storey ABOVE the active view level (strictly above
/// the drawn band) ends `Visibility::Hidden`, AFTER the REAL fog writer runs.
///
/// The resolver COMPOSES the resident fog fact (setup-inserted `SquadVisibility` + the
/// authored `player_faction`). A player ganger passes the FOG predicate unconditionally, so a
/// `Hidden` verdict here can ONLY come from the STOREY axis: the storey-2 ganger is strictly
/// above active==1, so it is culled by drawn-band membership (`2 > 1`).
///
/// Pin-discriminating: a change that dropped the "cull above active" bound (e.g. drawing the
/// whole occupied stack) would show the storey-2 ganger and FAIL.
#[test]
fn ganger_above_active_is_hidden_after_fog() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let l0 = Level::new(0);
    let l2 = Level::new(2);
    let l0_at = CellLevel::new(Cell::new(5, 6), l0);
    let l2_at = CellLevel::new(Cell::new(7, 8), l2);
    // A ground-floor player ganger (so setup spawns a valid battle) plus a player ganger on
    // storey 2. Author both cells as Present slabs so storey 2 exists.
    let situation = SituationBuilder::new()
        .with_ganger(ganger_at(l0_at, 0, Direction::East))
        .with_ganger(ganger_at(l2_at, 0, Direction::West))
        .player_faction(Faction::new(0))
        .slab_at(l0_at)
        .slab_at(l2_at)
        .build();
    assert!(drive_setup(&mut app, situation), "setup must complete");

    let l2_sim = sim_entity_at(&mut app, l2_at);
    assert!(l2_sim.is_some(), "the storey-2 ganger must have spawned");
    assert!(
        settle_actor(&mut app, l2_sim),
        "the storey-2 ganger sprite must have materialized",
    );

    // Fog VISIBLE at both cells (a player is shown by fog anyway); active view level = 1, so
    // storey 2 is strictly ABOVE the drawn band 0..=1.
    set_fog(&mut app, &[l0_at, l2_at], &[]);
    *app.world_mut().resource_mut::<ActiveLevel>() = ActiveLevel::new(Level::new(1));
    app.update();

    // C2: the storey-2 ganger is CULLED (Hidden) even as a fog-shown player — it is above active.
    assert_eq!(
        visibility_of_sim(&mut app, l2_sim),
        Some(Visibility::Hidden),
        "a ganger on storey 2 (strictly above active level 1) is hidden — culled above the drawn \
         band, asserted through the LIVE fog writer",
    );
}

/// GTW-520 C3 — the FOG hard-cut is PRESERVED: an UNSEEN ENEMY on a LOWER drawn storey stays
/// `Visibility::Hidden` even though its storey IS drawn.
///
/// This is the decisive fidelity guard: GTW-520 widens ONLY the storey axis, never the fog
/// predicate. The resolver composes the resident fog fact for real. The enemy (faction 1,
/// with `player_faction` 0) is on the ground floor — WITHIN the drawn band at active==1 — but its
/// cell is NOT squad-VISIBLE, so `is_ganger_visible` (the untouched fog predicate) hides it. A
/// co-located PLAYER ganger on the same lower storey IS shown (proving the storey axis widened,
/// so the enemy's Hidden is the FOG cut, not the storey cut).
///
/// Pin-discriminating: if GTW-520 had widened the fog predicate (or dropped the `&& shown_by_fog`
/// conjunct), the unseen lower-storey enemy would wrongly show and this FAILS.
#[test]
fn unseen_enemy_on_lower_storey_stays_hidden_fog_preserved() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let l0 = Level::new(0);
    let player_at = CellLevel::new(Cell::new(5, 6), l0);
    let enemy_at = CellLevel::new(Cell::new(20, 20), l0);
    let situation = SituationBuilder::new()
        .with_ganger(ganger_at(player_at, 0, Direction::East))
        .with_ganger(ganger_at(enemy_at, 1, Direction::West))
        .player_faction(Faction::new(0))
        .slab_at(player_at)
        .slab_at(enemy_at)
        .build();
    assert!(drive_setup(&mut app, situation), "setup must complete");

    let player_sim = sim_entity_at(&mut app, player_at);
    let enemy_sim = sim_entity_at(&mut app, enemy_at);
    assert!(
        player_sim.is_some() && enemy_sim.is_some(),
        "both gangers must have spawned",
    );
    assert!(
        settle_actor(&mut app, player_sim) && settle_actor(&mut app, enemy_sim),
        "both ganger sprites must have materialized",
    );

    // Fog: ONLY the player's cell is VISIBLE; the enemy's ground-floor cell is UNSEEN. Active
    // view level = 1, so the ground floor is a LOWER drawn storey (both gangers are in-band).
    set_fog(&mut app, &[player_at], &[]);
    *app.world_mut().resource_mut::<ActiveLevel>() = ActiveLevel::new(Level::new(1));
    app.update();

    // C3: the UNSEEN enemy on the lower drawn storey stays Hidden — the fog hard-cut is
    // preserved (only the storey axis widened).
    assert_eq!(
        visibility_of_sim(&mut app, enemy_sim),
        Some(Visibility::Hidden),
        "an UNSEEN enemy on a LOWER drawn storey (in-band) stays Hidden — the fog hard-cut is \
         PRESERVED; GTW-520 widened only the storey axis, never the visibility predicate",
    );
    // The co-located PLAYER ganger on the SAME lower storey IS shown — so the enemy's Hidden is
    // the FOG cut, not a storey cut (proving the storey axis really did widen).
    assert_eq!(
        visibility_of_sim(&mut app, player_sim),
        Some(Visibility::Inherited),
        "a player ganger on the same LOWER drawn storey is shown — the storey axis widened, so \
         the enemy's Hidden above is the fog cut, not the band cut",
    );
}

/// GTW-520 C5 (verified no-op) — cursor / selection stays ACTIVE-LEVEL: hovering a ganger DRAWN
/// on a LOWER storey does NOT resolve to (and so cannot select) that ganger, because the pick
/// binds the hovered `CellLevel`'s storey to the ACTIVE level, not the drawn ganger's storey.
///
/// The pick path lives in `gdtf_battle_input` (a crate DOWNSTREAM of the presenter, so it is
/// unreachable + deliberately UNCHANGED here — its own `pointer/picking` tests pin that
/// `resolve_hovered_cell` binds the storey to the passed-in active [`Level`]). This
/// presenter-crate regression asserts the invariant that MAKES that a no-op: a ganger drawn on a
/// lower storey has a `CellLevel` whose storey (0) is NOT the active storey (1), so the
/// active-level pick — which always resolves to `CellLevel(cell, active)` — can never equal the
/// drawn lower-storey ganger's cell and therefore never selects it. The ganger is VISIBLE
/// (drawn-band widening, C1) yet NOT pickable (cross-storey TARGETING is the GTW-522 follow-on).
///
/// Pin-discriminating: if the presenter had (wrongly) re-homed a lower-storey ganger's sim
/// `Position` onto the active storey to make it pickable, the storeys would match and this FAILS.
#[test]
fn hover_over_lower_storey_ganger_does_not_select_it_active_level_pick() {
    let mut app = headless_renderer_app();
    settle_resources(&mut app);

    let l0 = Level::new(0);
    let active = Level::new(1);
    let ganger_cell = Cell::new(9, 9);
    let l0_at = CellLevel::new(ganger_cell, l0);
    let situation = SituationBuilder::new()
        .with_ganger(ganger_at(l0_at, 0, Direction::East))
        .player_faction(Faction::new(0))
        .slab_at(l0_at)
        .build();
    assert!(drive_setup(&mut app, situation), "setup must complete");

    let l0_sim = sim_entity_at(&mut app, l0_at);
    assert!(
        l0_sim.is_some(),
        "the ground-floor ganger must have spawned"
    );
    assert!(
        settle_actor(&mut app, l0_sim),
        "the ganger sprite must have materialized",
    );

    // Raise the view to storey 1 and run the fog writer — the ground-floor ganger is now DRAWN
    // as a lower-storey unit (C1) so it IS on screen at its cell.
    set_fog(&mut app, &[l0_at], &[]);
    *app.world_mut().resource_mut::<ActiveLevel>() = ActiveLevel::new(active);
    app.update();
    assert_eq!(
        visibility_of_sim(&mut app, l0_sim),
        Some(Visibility::Inherited),
        "precondition: the lower-storey ganger is drawn (visible) at active level 1",
    );

    // The pick resolves the hovered cell at the ACTIVE storey (the input crate's contract): even
    // hovering the drawn ganger's screen cell, the hovered CellLevel is (cell, active=1), NOT the
    // ganger's (cell, 0). So the hovered cell does not equal the ganger's cell and cannot select it.
    let hovered_at_active = CellLevel::new(ganger_cell, active);
    let ganger_pos = position_of_sim(&mut app, l0_sim);
    assert_eq!(
        ganger_pos,
        Some(l0_at),
        "the drawn lower-storey ganger's sim Position stays on storey 0 (the presenter never \
         re-homes it onto the active storey)",
    );
    assert_ne!(
        Some(hovered_at_active),
        ganger_pos,
        "the active-level pick resolves the hovered cell to storey 1 (the active level), which \
         does NOT match the drawn ganger's storey-0 cell — so hover/selection cannot select the \
         drawn lower-storey ganger (cross-storey TARGETING is the GTW-522 follow-on, not built)",
    );
}
