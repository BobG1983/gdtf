//! The [`FallsPlugin`] — registers the [`FallOccurred`] output buffer and wires the
//! [`apply_falls`] system into the sim runtime with its EXPLICIT ordering (GTW-523 C7).

use bevy::prelude::{App, IntoScheduleConfigs, Plugin, Update};

use crate::{
    acts::dispatch_fire,
    falls::{FallOccurred, apply_falls},
    occupancy_sync::{SimSystems, sync_destroyed_slab},
};

/// Wires the GTW-523 fall mechanic into a Bevy [`App`] — the falls registration unit.
///
/// In `build()` the plugin:
///
/// - [`add_message`](App::add_message)s the [`FallOccurred`] output buffer (the presenter's
///   fall FX / log reader drains it), making [`apply_falls`]'s
///   [`MessageWriter<FallOccurred>`](bevy::prelude::MessageWriter) param valid
///   (`bevy-traps.md` #5); and
/// - adds [`apply_falls`] to [`Update`], `.in_set(`[`SimSystems::Simulate`]`)` (so it rides
///   the same `BattleInProgress` gate the rest of the runtime does — its `Res` grids /
///   tuning / injury content + the two `ResMut` RNG streams are battle-lifetime,
///   `bevy-traps.md` #1) with the C7 EXPLICIT ordering:
///   - `.after(`[`dispatch_fire`]`)` — the SHOT path emits
///     [`SlabDestroyed`](crate::occupancy_sync::SlabDestroyed) this frame, so `apply_falls`
///     must run AFTER it to see the SAME-FRAME destruction in the buffer (else a one-frame
///     lag); and
///   - `.after(`[`sync_destroyed_slab`]`)` — the slab is folded `Destroyed` onto the
///     `SurfaceGrid` BEFORE the fall resolves (the drop scan reads a settled surface). The
///     two share the SAME-FRAME [`SlabDestroyed`](crate::occupancy_sync::SlabDestroyed)
///     buffer via INDEPENDENT `MessageReader`
///     cursors (each reader has its own cursor — one never steals the other's messages;
///     `bevy-traps.md` #3 — explicit ordering, not cursor sharing).
///
/// The `FallOccurred` `add_message` is registered ONLY here (this plugin is the sole
/// producer). The production app adds this plugin alongside the other runtime plugins in
/// [`BattleSimPlugin`](crate::battle::BattleSimPlugin).
#[derive(Debug, Default, Clone, Copy)]
pub struct FallsPlugin;

impl Plugin for FallsPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<FallOccurred>().add_systems(
            Update,
            apply_falls
                // C7: run AFTER the shot path emits SlabDestroyed (same-frame), so the fall
                // resolves the frame the slab is smashed rather than one frame late.
                .after(dispatch_fire)
                // C7: run AFTER the slab is folded Destroyed onto the SurfaceGrid, so the drop
                // scan reads a settled surface. Independent MessageReader cursors (never a steal).
                .after(sync_destroyed_slab)
                .in_set(SimSystems::Simulate),
        );
    }
}
