//! The action-bar scene-plugin (GTW-228 / GTW-48 S9 / 222c).
//!
//! Registers the battle-scoped themed action-bar in the battlescape neighborhood,
//! beside the presenter + input plugins:
//!
//! - **Lifecycle** — `spawn_action_bar` `OnEnter(BattleScapeState::BattleRunning)`,
//!   `despawn_action_bar` `OnExit(BattleScapeState::BattleRunning)`, so the bar exists
//!   only during the live tactical layer (NOT the whole `GameState::BattleScape`, which
//!   spans the `Generation` → `AnimateIn` → `BattleRunning` → `AnimateOut` → `AfterMath`
//!   walk). This is the contract's chosen lifecycle clause.
//! - **Interactivity** — `action_bar_button_intents` runs in `Update` gated
//!   `run_if(resource_exists::<BattleInProgress>)`, the SAME live-battle witness the S7
//!   input + presenter draws gate on (`bevy-traps.md` #1), so a press is inert when no
//!   battle is live — no press is a silent no-op during generation/aftermath because the
//!   bar is only spawned in `BattleRunning` AND the action system only runs while the
//!   battle witness is present.
//! - **Aim control** (GTW-253 / GTW-277) — a `gdtf_ui` `Switch`. `aim_switch_flip_intent`
//!   reads the widget's `ToggleFlipped` message and pushes `ActIntent::AimToggle` on the
//!   SAME 222a seam the aim key does; `sync_aim_switch_state` mirrors the selected ganger's
//!   `Aiming` onto the switch's `SwitchState` (+ re-derives the track look) so the switch
//!   visibly shows ON/OFF, under the `run_if(resource_exists::<BattleInProgress>)` gate.
//! - **Stance control** (GTW-267 / GTW-277) — a `gdtf_ui` vertical `SegmentedControl`
//!   (Stand / Kneel / Prone). `stance_segment_intent` reads the widget's `SegmentSelected`
//!   message and pushes a DIRECT `ActIntent::SetStance(kind)`; `sync_stance_active_segment`
//!   writes the control's `ActiveSegment` from the selected ganger's `Stance` (mutually
//!   exclusive), under the same gate.
//! - **Mode control** (GTW-265 / GTW-277 / GTW-284) — a `gdtf_ui` horizontal
//!   `SegmentedControl` (Single / Burst / Full). Its three segments are spawned once;
//!   `rebuild_mode_segments` MUTATES per-segment `Display` (via
//!   `gdtf_ui::set_segment_visible`) to the selected weapon's offered modes on a selection
//!   change (`.after(UiSystems::ApplyTheme)`), NEVER despawning/respawning them (GTW-284:
//!   stable ids, no `Themed` churn); `mode_segment_write` sets `SelectedFireMode` directly
//!   to the chosen mode's read-back spec; `sync_mode_active_segment` writes the control's
//!   `ActiveSegment`. This REPLACED the GTW-254 popup picker (no modal, no scrim).
//!
//! The bar WRITES the shared 222a [`PendingActIntent`](gdtf_battle_input::PendingActIntent)
//! seam that `gdtf_battle_input`'s keyboard surface also writes (parallel surfaces, one
//! drain). The action system runs UPSTREAM of
//! `gdtf_battle_input::dispatch_act_intents` (registered `.after` its keyboard writers,
//! draining the same update's pushes), so a button press queued this update is drained
//! this update — exactly like a key press. The bar reaches the sim acts ONLY via the
//! `gdtf_app -> gdtf_battle_input` DATA seam, never a reverse edge or a cross-crate fn
//! (ADR-0001).

use bevy::prelude::*;
use gdtf_battle_input::dispatch_act_intents;
use gdtf_battle_presenter::ActiveLevel;
use gdtf_battle_sim::prelude::BattleInProgress;
use gdtf_ui::{drive_switches, repaint_segments, select_segment_on_press, themed::UiSystems};

use crate::states::{
    BattleScapeState,
    running::game::battlescape::action_bar::systems::{
        action_bar_button_intents, aim_switch_flip_intent, despawn_action_bar, flee_button_pressed,
        mode_segment_write, nowrap_control_labels, rebuild_mode_segments, spawn_action_bar,
        stance_segment_intent, sync_aim_switch_state, sync_level_button_bounds,
        sync_mode_active_segment, sync_mode_tu_cost_lines, sync_stance_active_segment,
        tag_mode_segments, tag_stance_segments,
    },
};

/// The action-bar scene-plugin — spawns/despawns the bar on the `BattleRunning`
/// boundary and runs its button-action system gated on the live-battle witness.
pub(in crate::states::running::game::battlescape) struct GameBattleScapeActionBarScenePlugin;

impl Plugin for GameBattleScapeActionBarScenePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(BattleScapeState::BattleRunning), spawn_action_bar)
            .add_systems(OnExit(BattleScapeState::BattleRunning), despawn_action_bar)
            .add_systems(
                Update,
                action_bar_button_intents
                    // Ordered `.before` the ONE intent drain (`bevy-traps.md` #3): a button
                    // press queued THIS update is drained THIS update — the same-frame
                    // guarantee the keyboard writers get (they are registered `.before`
                    // the drain in `gdtf_battle_input`). Without this the press could be
                    // queued AFTER the drain ran and only take effect a frame late.
                    .before(dispatch_act_intents)
                    .run_if(resource_exists::<BattleInProgress>),
            )
            // GTW-293: `sync_level_button_bounds` greys (inserts `DisabledButton` on) the
            // **Level −** button at the floor storey (0) and the **Level +** button at the
            // ceiling (`MAX_LEVELS - 1`), removing it off the bound — reacting to the live
            // `ActiveLevel`. The same marker disables the look AND makes the press router's
            // `Without<DisabledButton>` filter ignore the bounded button, so it is ordered
            // `.before(action_bar_button_intents)` (`bevy-traps.md` #3): the bound is applied
            // BEFORE the press is read this update, so a press at the bound never fires. Gated on
            // BOTH the live-battle witness AND `resource_exists::<ActiveLevel>` (`bevy-traps.md`
            // #1) — the presenter inserts `ActiveLevel` for the whole battle span, but the gate
            // keeps the system inert if it is somehow absent.
            .add_systems(
                Update,
                sync_level_button_bounds
                    .before(action_bar_button_intents)
                    .run_if(resource_exists::<BattleInProgress>)
                    .run_if(resource_exists::<ActiveLevel>),
            )
            // The flee button (GTW-240): an ENABLED app/lifecycle button whose press ends the
            // persisting battle. Gated on the SAME live-battle witness so a press is inert
            // outside a live battle (AC3). It needs NO `.before(dispatch_act_intents)` ordering
            // — it writes the `BattleRunningComplete` lifecycle marker directly (via the typed
            // `insert_battle_running_complete` door), never the intent seam — so it is ordered
            // independently.
            .add_systems(
                Update,
                flee_button_pressed.run_if(resource_exists::<BattleInProgress>),
            )
            // GTW-277 AIM (a `gdtf_ui` `Switch`): `aim_switch_flip_intent` reads the
            // widget's `ToggleFlipped` message and pushes `ActIntent::AimToggle` on the SAME
            // 222a seam the aim key does. It is ordered `.before(dispatch_act_intents)` so a
            // flip queued this update is drained this update (the byte-equal same-frame
            // guarantee). `sync_aim_switch_state` mirrors the selected ganger's `Aiming` onto
            // the switch's `SwitchState` + re-derives the track look (a sim-driven set, never
            // re-emitting a flip). Same live-battle gate.
            .add_systems(
                Update,
                aim_switch_flip_intent
                    // READS the `ToggleFlipped` message `drive_switches` writes this update, so
                    // it must run AFTER the widget driver to see the same-update flip — and
                    // BEFORE the intent drain so the pushed intent is drained this update
                    // (`bevy-traps.md` #3).
                    .after(drive_switches)
                    .before(dispatch_act_intents)
                    .run_if(resource_exists::<BattleInProgress>),
            )
            .add_systems(
                Update,
                sync_aim_switch_state.run_if(resource_exists::<BattleInProgress>),
            )
            // GTW-277 STANCE (a `gdtf_ui` vertical `SegmentedControl`): `stance_segment_intent`
            // reads the widget's `SegmentSelected` message and pushes a DIRECT
            // `ActIntent::SetStance(kind)` on the 222a seam (ordered `.before` the drain like
            // the aim flip). `sync_stance_active_segment` writes the control's `ActiveSegment`
            // from the selected ganger's `Stance`, ordered `.before(repaint_segments)` so the
            // active-segment highlight repaints the SAME frame the selection changes
            // (`bevy-traps.md` #3 — the `gdtf_ui` repaint reacts to `Changed<ActiveSegment>`).
            // `tag_stance_segments` runs once (on `Added<StanceControl>`) to attach the
            // per-stance markers to the freshly-spawned segments. Same live-battle gate.
            .add_systems(
                Update,
                stance_segment_intent
                    // READS the `SegmentSelected` message `select_segment_on_press` writes this
                    // update, so it runs AFTER the widget driver to see the same-update select —
                    // and BEFORE the intent drain so the pushed intent is drained this update
                    // (`bevy-traps.md` #3).
                    .after(select_segment_on_press)
                    .before(dispatch_act_intents)
                    .run_if(resource_exists::<BattleInProgress>),
            )
            .add_systems(
                Update,
                (
                    tag_stance_segments,
                    sync_stance_active_segment.before(repaint_segments),
                )
                    .run_if(resource_exists::<BattleInProgress>),
            )
            // GTW-277 MODE (a `gdtf_ui` horizontal `SegmentedControl`): `mode_segment_write`
            // reads the widget's `SegmentSelected` message and sets `SelectedFireMode` DIRECTLY
            // to the chosen mode's read-back spec (Mode does NOT use the intent seam — GTW-265).
            // `sync_mode_active_segment` writes the control's `ActiveSegment` from
            // `SelectedFireMode`, ordered `.after(mode_segment_write)` (read-after-write — the
            // active mark moves the SAME update a select lands) AND `.before(repaint_segments)`
            // (the highlight repaints the same frame; `bevy-traps.md` #3). `tag_mode_segments`
            // runs once (on `Added<ModeControl>`) to attach the per-mode markers. Same gate.
            .add_systems(
                Update,
                (
                    // `mode_segment_write` READS the `SegmentSelected` message
                    // `select_segment_on_press` writes this update → run AFTER the widget driver
                    // (`bevy-traps.md` #3). `sync_mode_active_segment` READS `SelectedFireMode`
                    // that the write produces → run AFTER it AND `.before(repaint_segments)` so
                    // the highlight repaints the same frame.
                    mode_segment_write.after(select_segment_on_press),
                    tag_mode_segments,
                    sync_mode_active_segment
                        .after(mode_segment_write)
                        .before(repaint_segments),
                )
                    .run_if(resource_exists::<BattleInProgress>),
            )
            // GTW-277 / GTW-284: `rebuild_mode_segments` MUTATES the three FIXED Mode segments'
            // per-segment `Display` (via `gdtf_ui::set_segment_visible`) on a selection change
            // (it never despawns/respawns them), ordered `.after(UiSystems::ApplyTheme)`
            // (`bevy-traps.md` #3) so its writes settle deterministically relative to the theme
            // pass. Because no segment entity is added/removed, the segment ids stay STABLE and
            // there is no spurious `Added<Themed>` to trigger a global repaint — the GTW-284
            // root-cause is gone.
            .add_systems(
                Update,
                rebuild_mode_segments
                    .after(UiSystems::ApplyTheme)
                    .run_if(resource_exists::<BattleInProgress>),
            )
            // GTW-303 slice 2: `sync_mode_tu_cost_lines` MUTATES each offered Mode segment's
            // sub-line to its aim-adjusted per-shot TU cost ("{n} TU"), reusing the sim's
            // `mode_tu_cost` so the display equals the charge (slice 1's `set_segment_sub_line`
            // writes/clears the line in place — no respawn). It re-derives on a selection
            // change, a `Changed<Aiming>` Aim flip, or the freshly-spawned control, ordered
            // `.after(rebuild_mode_segments)` so the offered set is settled (hence
            // `.after(UiSystems::ApplyTheme)` transitively; `bevy-traps.md` #3) before the cost
            // lines are written. Same live-battle gate.
            .add_systems(
                Update,
                sync_mode_tu_cost_lines
                    .after(rebuild_mode_segments)
                    .run_if(resource_exists::<BattleInProgress>),
            )
            // GTW-298: keep the relocated firemode / aim / stance toggle LABELS on one line so a
            // too-wide caption (e.g. `full-auto`) clips inside its toggle instead of soft-
            // wrapping + overflowing the short firemode cell into the row above (contract item
            // 8). Ordered `.after(UiSystems::ApplyTheme)` so it runs after the theme pass settles
            // the freshly-spawned labels; same live-battle gate as the other control systems.
            .add_systems(
                Update,
                nowrap_control_labels
                    .after(UiSystems::ApplyTheme)
                    .run_if(resource_exists::<BattleInProgress>),
            );
    }
}
