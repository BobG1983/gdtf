//! GTW-627 (C7, regression): a STEADY frame with UNCHANGED fog state must leave every
//! terrain tile's / ganger sprite's `Visibility` change ticks AND the
//! `Assets<TerrainFogMaterial>` store's change ticks untouched.
//!
//! The pre-GTW-627 fog writer assigned `*visibility` unconditionally for every drawn
//! terrain tile and every mapped actor sprite, and took `Assets::get_mut` per shown tile
//! per frame — re-marking components/assets changed every tick and re-uploading the
//! material uniform each frame (the exact re-dirty class GTW-568 fixed in the pooled
//! overlays). GTW-627 routes both arms through the one tick-quiet write seam
//! (`set_if_neq` visibility flips; compare-before-`get_mut` material knobs), so a steady
//! frame writes nothing.
//!
//! Probed end-to-end through the REAL registered systems (the `TopDownRendererPlugin`
//! wiring): a probe system `.after(PresenterSystems::Draw)` records, each frame, how many
//! fog-owned `Visibility` components were change-flagged, whether the material store's
//! resource tick advanced, and how many `AssetEvent::Modified` messages (the uniform
//! re-upload trigger) were emitted. After the fog-transition frames settle, a steady
//! frame must record all-quiet.

use bevy::{
    app::Update,
    asset::{AssetEvent, Assets},
    ecs::message::MessageReader,
    prelude::{
        Deref, DerefMut, DetectChanges, IntoScheduleConfigs, Query, Ref, Res, ResMut, Resource,
        Visibility, With,
    },
};
use gdtf_battle_presenter::{GangerSprite, PresenterSystems, TerrainFogMaterial, TerrainSprite};
use gdtf_battle_sim::{
    prelude::{Cell, CellLevel, Direction, Faction, Level},
    test_support::SituationBuilder,
};

use super::harness::*;

/// How many probed entities had their `Visibility` change-flagged on the probed frame —
/// the re-dirty witness (a named domain count, private inner, written through the derived
/// [`DerefMut`]).
#[derive(Default, Deref, DerefMut, Clone, Copy)]
struct RedirtyCount(usize);

/// Whether the `Assets<TerrainFogMaterial>` RESOURCE change tick advanced on the probed
/// frame — the store-level poke witness (a mutable deref through `ResMut` marks it even
/// when no asset field moved).
#[derive(Default, Deref, DerefMut, Clone, Copy)]
struct StorePoked(bool);

/// How many [`AssetEvent::Modified`] messages for [`TerrainFogMaterial`] the probe drained
/// on the probed frame — each one re-uploads that tile's uniform next frame.
#[derive(Default, Deref, DerefMut, Clone, Copy)]
struct ModifiedCount(usize);

/// The per-frame quiet witness the probe system records after the draw band ran.
#[derive(Resource, Default)]
struct QuietProbe {
    /// Terrain tiles whose `Visibility` was change-flagged this frame.
    terrain_redirtied: RedirtyCount,
    /// Ganger sprites whose `Visibility` was change-flagged this frame.
    actors_redirtied:  RedirtyCount,
    /// Whether the material store's resource tick advanced this frame.
    store_poked:       StorePoked,
    /// `AssetEvent::Modified` messages drained this frame.
    modified_events:   ModifiedCount,
}

/// Per-frame probe (`.after(PresenterSystems::Draw)`): snapshot the frame's fog-owned
/// change-tick activity — `Ref::is_changed` is relative to this probe's own last run, so
/// it sees exactly the frame's writes.
fn record_quiet_probe(
    terrain: Query<Ref<Visibility>, With<TerrainSprite>>,
    actors: Query<Ref<Visibility>, With<GangerSprite>>,
    materials: Res<Assets<TerrainFogMaterial>>,
    mut modified: MessageReader<AssetEvent<TerrainFogMaterial>>,
    mut probe: ResMut<QuietProbe>,
) {
    probe.terrain_redirtied = RedirtyCount(terrain.iter().filter(Ref::is_changed).count());
    probe.actors_redirtied = RedirtyCount(actors.iter().filter(Ref::is_changed).count());
    probe.store_poked = StorePoked(materials.is_changed());
    probe.modified_events = ModifiedCount(
        modified
            .read()
            .filter(|event| matches!(event, AssetEvent::Modified { .. }))
            .count(),
    );
}

/// GTW-627 A2/C7 — unchanged fog state leaves component AND asset change ticks untouched:
/// after the fog-authoring transition frames settle, a STEADY frame re-dirties ZERO
/// terrain-tile / ganger-sprite `Visibility` components, never pokes the
/// `Assets<TerrainFogMaterial>` store's resource tick, and emits ZERO
/// `AssetEvent::Modified` (no uniform re-upload).
///
/// The fixture drives the REAL setup path (a player + an enemy ganger) and authors a
/// mixed fog — a VISIBLE cell, an EXPLORED cell, everything else UNSEEN — so all three
/// terrain arms and both actor verdicts are exercised on the steady frame.
///
/// Pin-discriminating: the pre-GTW-627 writer assigned `*visibility` unconditionally and
/// `get_mut` every shown tile per frame, so this observes non-zero counts on every frame
/// against that code (RED before the GTW-627 write seam, GREEN after).
#[test]
fn steady_frame_leaves_fog_visibility_and_material_ticks_untouched() {
    let mut app = headless_renderer_app();
    app.init_resource::<QuietProbe>();
    // The probe reads the fog-owned components AFTER the draw band wrote them this frame,
    // so its `Ref::is_changed` (relative to its own last run) sees exactly this frame's
    // fog writes.
    app.add_systems(Update, record_quiet_probe.after(PresenterSystems::Draw));
    settle_resources(&mut app);

    let l0 = Level::new(0);
    let player_cell = CellLevel::new(Cell::new(5, 5), l0);
    let explored_cell = CellLevel::new(Cell::new(6, 6), l0);
    let enemy_cell = CellLevel::new(Cell::new(20, 20), l0);
    let situation = SituationBuilder::new()
        .with_ganger(ganger_at(player_cell, 0, Direction::East))
        .with_ganger(ganger_at(enemy_cell, 1, Direction::West))
        .player_faction(Faction::new(0))
        .build();
    assert!(drive_setup(&mut app, situation), "setup must complete");
    assert!(
        settle_terrain_at(&mut app, player_cell),
        "the terrain field must have drawn",
    );
    let player_sim = sim_entity_at(&mut app, player_cell);
    let enemy_sim = sim_entity_at(&mut app, enemy_cell);
    assert!(
        settle_actor(&mut app, player_sim) && settle_actor(&mut app, enemy_sim),
        "both ganger sprites must have materialized",
    );

    // Author the mixed fog, then let the TRANSITION frames settle: the first update does
    // the genuine flips/knob writes, the second lets the queued asset events drain so the
    // steady frame's probe reads a clean baseline.
    set_fog(&mut app, &[player_cell], &[explored_cell]);
    app.update();
    app.update();

    // STEADY frame: the fog state is unchanged — nothing fog-owned may be re-dirtied.
    app.update();
    let probe = app.world().resource::<QuietProbe>();
    assert_eq!(
        *probe.terrain_redirtied, 0,
        "a steady frame with unchanged fog must leave every terrain tile's Visibility \
         change ticks untouched (the pre-GTW-627 unconditional `*visibility` write \
         re-dirtied every tile every frame)",
    );
    assert_eq!(
        *probe.actors_redirtied, 0,
        "a steady frame with unchanged fog must leave every ganger sprite's Visibility \
         change ticks untouched (the resolver writes via set_if_neq)",
    );
    assert!(
        !*probe.store_poked,
        "a steady frame with unchanged fog must not poke the Assets<TerrainFogMaterial> \
         resource tick (knobs are compared via Assets::get before any get_mut)",
    );
    assert_eq!(
        *probe.modified_events, 0,
        "a steady frame with unchanged fog must emit zero AssetEvent::Modified — no \
         per-frame uniform re-upload (the pre-GTW-627 writer took get_mut per shown tile \
         per frame)",
    );
}
