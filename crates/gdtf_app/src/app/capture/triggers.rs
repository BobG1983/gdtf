//! The dev fire / fall trigger systems of the DEV-ONLY capture affordance — scripted
//! triggers that drive the REAL sim paths (a `FireRequested` message / the GTW-523
//! fall path) at a configured `BattleRunning` frame. Split out of the sibling `plugin`
//! module (GTW-583); see its header for the full affordance rationale.

use bevy::prelude::*;
use gdtf_battle_input::{SelectedFireMode, SelectedShooter};
use gdtf_battle_sim::{
    CellLevel, Faction, FireMode, FireModeSpec, Level, ModeKind, PlayerFaction, Position,
    SlabDestroyed, acts::FireRequested,
};

use super::trigger_config::{FallConfig, FireConfig};

/// At the configured [`FireAtFrame`](super::trigger_config::FireAtFrame) (counted in
/// [`BattleRunning`](crate::states::BattleScapeState::BattleRunning) frames), makes the selected player
/// ganger fire at the nearest enemy via the REAL fire path.
///
/// This is the GTW-306 dev fire-trigger. It does NOT fake a shot: it writes a
/// [`FireRequested`](gdtf_battle_sim::acts::FireRequested) message — the exact message a
/// left-click over an enemy produces — so the sim's `dispatch_fire` resolves the volley
/// (TU spend, arc, hit roll, `ShotFired`), and the FX slices then render the projectile /
/// impact off `ShotFired`. The shooter is the auto-selected
/// [`SelectedShooter`](gdtf_battle_input::SelectedShooter) (a player-faction ganger), the
/// mode is the [`SelectedFireMode`](gdtf_battle_input::SelectedFireMode), and the target
/// is the nearest enemy ganger (a [`Faction`] `!=` [`PlayerFaction`]) by squared cell
/// distance.
///
/// Fires exactly once: it spends only while its [`Local<u32>`] counter equals the target
/// frame. A frame with no selection, no player faction, or no enemy in range is a no-op
/// (the shot simply does not fire — the affordance is best-effort dev tooling).
///
/// Param-only (`bevy-traps.md` #7): a [`MessageWriter<FireRequested>`], the
/// `Res<SelectedShooter>` / `Res<SelectedFireMode>` / `Option<Res<PlayerFaction>>` reads,
/// a read-only `Query<(Entity, &Faction, &Position)>`, and a [`Local<u32>`] — no
/// `&mut World`. `Option<Res<PlayerFaction>>` because that resource exists only inside
/// the battle window (`bevy-traps.md` #1).
///
/// `pub(crate)` so the headless test drives this REAL system directly (registered in
/// `Update` minus the unrelated `BattleScapeState` sub-state gate — the same "drive the
/// real system on its real schedule, minus unrelated state wiring" idiom the auto-battle
/// A1 test uses), asserting it emits one `FireRequested` at frame N.
pub(crate) fn trigger_fire_at_frame(
    mut fires: MessageWriter<FireRequested>,
    config: Res<FireConfig>,
    selected: Res<SelectedShooter>,
    fire_mode: Res<SelectedFireMode>,
    player: Option<Res<PlayerFaction>>,
    gangers: Query<(Entity, &Faction, &Position, Option<&FireMode>)>,
    mut frames_in_battle: Local<u32>,
) {
    *frames_in_battle += 1;
    if *frames_in_battle != *config.frame {
        // Not the trigger frame (or already fired): wait. `!=` keeps the fire to the one
        // target frame.
        return;
    }
    // The selected player ganger; bail — LOUDLY (GTW-590 C3: this is the one frame the
    // scripted shot can happen, so a silent no-op is a dead QA run) — if nothing is
    // selected.
    let Some(shooter) = **selected else {
        warn!(
            "dev-capture: fire trigger at frame {}: no shooter selected; the scripted \
             shot is skipped",
            *config.frame,
        );
        return;
    };
    let Some(player) = player else {
        warn!(
            "dev-capture: fire trigger at frame {}: no player faction in the battle; \
             the scripted shot is skipped",
            *config.frame,
        );
        return;
    };
    let player_faction = **player;
    // The shooter's own cell (to pick the NEAREST enemy) + its authored fire-mode selector
    // (consulted only when GDTF_FIRE_MODE overrides the mode).
    let Ok((_, _, shooter_pos, shooter_modes)) = gangers.get(shooter) else {
        warn!(
            "dev-capture: fire trigger at frame {}: the selected shooter is not a live \
             ganger; the scripted shot is skipped",
            *config.frame,
        );
        return;
    };
    // The canonical CellLevel accessors through Position's deref (GTW-565).
    let shooter_cell = shooter_pos.cell();
    // The mode the shot fires in: the resident SelectedFireMode by default, or — when
    // GDTF_FIRE_MODE is set — the matching authored mode off the shooter's FireMode selector
    // (so a `full` override yields a multi-round volley the FX stagger can spread out). An
    // override naming a mode the weapon does not offer (or an unarmed shooter) falls back to
    // the resident mode.
    let mode = config
        .mode
        .and_then(|override_kind| fire_mode_spec(shooter_modes, *override_kind))
        .unwrap_or(**fire_mode);
    // The nearest enemy ganger (a faction != the player's) by squared cell distance.
    let Some((_, enemy_pos)) = gangers
        .iter()
        .filter(|(entity, faction, ..)| *entity != shooter && **faction != player_faction)
        .map(|(_, _, pos, _)| {
            let delta = pos.cell();
            let dx = delta.x - shooter_cell.x;
            let dy = delta.y - shooter_cell.y;
            (dx * dx + dy * dy, pos)
        })
        .min_by_key(|(dist_sq, _)| *dist_sq)
    else {
        warn!(
            "dev-capture: fire trigger at frame {}: no enemy ganger to target; the \
             scripted shot is skipped",
            *config.frame,
        );
        return;
    };
    let (target_cell, target_level) = enemy_pos.split();
    fires.write(FireRequested::new(shooter, mode, target_cell, target_level));
    // GTW-590 C3: the fired trigger is as loud as a skipped one.
    info!(
        "dev-capture: fire trigger fired at frame {} (mode {:?})",
        *config.frame, mode.kind,
    );
}

/// The authored [`FireModeSpec`] for `kind` on a shooter's optional [`FireMode`] selector,
/// or [`None`] when the shooter is unarmed (no selector) or does not offer that mode.
///
/// Used by the dev fire-trigger's `GDTF_FIRE_MODE` override to fire in a specific authored
/// mode (e.g. `Full`) rather than the resident [`SelectedFireMode`]. Pure read-only lookup.
fn fire_mode_spec(modes: Option<&FireMode>, kind: ModeKind) -> Option<FireModeSpec> {
    modes?.iter().copied().find(|spec| spec.kind == kind)
}

/// The storey the dev fall-trigger elevates the chosen ganger to before smashing the slab
/// under it — storey 1 (the lowest UPPER storey).
///
/// Dropping from storey 1 always lands on the ground (`k == 0` supports unconditionally in
/// [`resolve_drop`](gdtf_battle_sim::resolve_drop)), so the forced fall is RELIABLE on any
/// battlefield — it needs no procgen-placed intact slab below. A named newtype so the trigger
/// never passes a bare storey index (no-bare-types).
const FALL_TRIGGER_STOREY: Level = Level::new(1);

/// At the configured [`FallAtFrame`](super::trigger_config::FallAtFrame) (counted in
/// [`BattleRunning`](crate::states::BattleScapeState::BattleRunning) frames), forces a determinate player
/// ganger to FALL via the REAL GTW-523 fall path.
///
/// This is the GTW-529 dev fall-trigger — the fall counterpart of
/// [`trigger_fire_at_frame`], added so the GTW-524 fall FX has a scripted in-engine QA
/// trigger (`GDTF_FIRE_AT_FRAME` only targets an enemy ganger, never a slab under a
/// friendly). It does NOT fake a fall: it drives the authoritative path end-to-end.
///
/// On the trigger frame, for a determinate player-faction ganger (the auto-selected
/// [`SelectedShooter`](gdtf_battle_input::SelectedShooter) when it is player-faction,
/// otherwise the lowest-[`Entity`] player-faction ganger — a stable, deterministic pick):
///
/// 1. **Elevate.** Its [`Position`] is rewritten to `(same cell, `[`FALL_TRIGGER_STOREY`]`)`
///    — the lowest upper storey — as ONE write. This stands the ganger on an upper storey so
///    there is a floor beneath it to smash, RELIABLY on any battlefield (the default skirmish
///    spawns everyone on the ground), keeping the fall deterministic (same frame ⇒ same fall).
/// 2. **Smash.** It writes one [`SlabDestroyed`](gdtf_battle_sim::SlabDestroyed) at that SAME
///    `(cell, level)` — the slab the ganger now stands on. Because this system is ordered
///    `.before(`[`apply_falls`](gdtf_battle_sim::apply_falls)`)`, the same-frame
///    `SlabDestroyed` is buffered AND the elevating `Position` write is visible when
///    `apply_falls` reads its faller query, so the GTW-523 drop resolves THIS frame (down to
///    the ground `k == 0`) and the GTW-524 impact flash fires at the landing — both captured
///    in the same frame by the GTW-297 [`capture_when_ready`](super::screenshot::capture_when_ready) path (no second capture
///    mechanism).
///
/// Fires exactly once: it acts only while its [`Local<u32>`] counter equals the target frame.
/// A frame with no player ganger is a no-op (best-effort dev tooling — the fall simply does
/// not fire).
///
/// Param-only (`bevy-traps.md` #7): a [`MessageWriter<SlabDestroyed>`], the
/// `Res<FallConfig>` / `Res<SelectedShooter>` / `Option<Res<PlayerFaction>>` reads, a
/// `Query<(Entity, &Faction, &mut Position)>` (the `&mut Position` is the C2-style elevating
/// write), and a [`Local<u32>`] — no `&mut World`. `Option<Res<PlayerFaction>>` because that
/// resource exists only inside the battle window (`bevy-traps.md` #1).
///
/// `pub(crate)` so the headless test drives this REAL system directly (registered in `Update`
/// minus the unrelated `BattleScapeState` sub-state gate — the same "drive the real system on
/// its real schedule, minus unrelated state wiring" idiom the fire-trigger test uses),
/// asserting the chosen ganger's `Position` drops via the real `apply_falls`.
pub(crate) fn trigger_fall_at_frame(
    mut destroyed: MessageWriter<SlabDestroyed>,
    config: Res<FallConfig>,
    selected: Res<SelectedShooter>,
    player: Option<Res<PlayerFaction>>,
    mut gangers: Query<(Entity, &Faction, &mut Position)>,
    mut frames_in_battle: Local<u32>,
) {
    *frames_in_battle += 1;
    if *frames_in_battle != *config.frame {
        // Not the trigger frame (or already fired): wait. `!=` keeps the fall to the one
        // target frame (the one-shot discipline the fire-trigger uses).
        return;
    }
    let Some(player) = player else {
        warn!(
            "dev-capture: fall trigger at frame {}: no player faction in the battle; \
             the scripted fall is skipped",
            *config.frame,
        );
        return;
    };
    let player_faction = **player;
    // The determinate faller: the auto-selected SelectedShooter when it is player-faction,
    // else the lowest-Entity player-faction ganger (a stable, deterministic tiebreak). Both
    // reads go through the same query, so a single scan yields the pick.
    let selected_player = (**selected).filter(|entity| {
        gangers
            .get(*entity)
            .is_ok_and(|(_, faction, _)| *faction == player_faction)
    });
    let Some(faller) = selected_player.or_else(|| {
        gangers
            .iter()
            .filter(|(_, faction, _)| **faction == player_faction)
            .map(|(entity, ..)| entity)
            .min()
    }) else {
        warn!(
            "dev-capture: fall trigger at frame {}: no player-faction ganger to drop; \
             the scripted fall is skipped",
            *config.frame,
        );
        return;
    };
    let Ok((_, _, mut position)) = gangers.get_mut(faller) else {
        // Structurally unreachable (the faller came out of this same query), but the
        // GTW-590 loudness contract forbids a silent one-shot no-op even here.
        warn!(
            "dev-capture: fall trigger at frame {}: the chosen faller vanished from the \
             query; the scripted fall is skipped",
            *config.frame,
        );
        return;
    };
    // 1. Elevate: stand the ganger on the lowest upper storey (same cell), so there is a floor
    //    beneath it to smash. ONE involuntary write; `apply_falls` reads the live query this
    //    frame (we run `.before` it), so it sees the elevated position. The ground cell is
    //    the canonical CellLevel::cell accessor through Position's deref (GTW-565).
    let elevated = CellLevel::new(position.cell(), FALL_TRIGGER_STOREY);
    *position = Position::new(elevated);
    // 2. Smash: destroy the slab the ganger now stands on. `apply_falls` (ordered after) reads
    //    this same-frame message + the elevated position and drops the ganger to the ground.
    destroyed.write(SlabDestroyed::new(elevated));
    // GTW-590 C3: the fired trigger is as loud as a skipped one.
    info!(
        "dev-capture: fall trigger fired at frame {}: ganger {faller} elevated to storey {} \
         and its slab smashed",
        *config.frame, *FALL_TRIGGER_STOREY,
    );
}
