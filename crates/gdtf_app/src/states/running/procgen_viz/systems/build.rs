//! The DEV-ONLY procgen-visualizer `OnEnter` builders (GTW-434): insert the model, then
//! spawn the screen — the dark board quad, the per-prefab light quads, and the STEP / AUTO
//! control bar.
//!
//! `OnEnter(DebugProcgenVisualizer)` runs [`insert_viz_model`] BEFORE [`spawn_viz_screen`]
//! (ordered `.chain()` by the scene plugin) so the screen spawn reads the freshly-built
//! model. The whole module is `#[cfg(debug_assertions)]`-gated by its parent.

use bevy::{prelude::*, ui::Val};
use gdtf_battle_sim::{level::PrefabRegistry2, rng::BattleSeed};
use gdtf_ui::{
    ButtonLabel, spawn_button, spawn_panel,
    theme::GdtfTheme,
    themed::{ThemeRole, Themed},
};

use crate::states::{
    LoadedSituation, RunningState,
    running::procgen_viz::{
        components::{AutoButton, BoardQuad, ProcgenVizRoot, StepButton, prefab_quad_marker},
        model::{BoardExtent, ProcgenViz, QuadRect, RevealIndex, VizQuad, default_viz_seed},
    },
};

/// The `GlobalZIndex` of the dark board quad — above the themed backdrop, below the light
/// per-prefab quads (bevy-traps #8: an explicit z so the quads never get painted over).
const BOARD_Z: i32 = 10;
/// The `GlobalZIndex` of the per-prefab light quads — strictly above the dark board quad so
/// they read on top of it (C2 — light quads in front of the dark board).
const QUAD_Z: i32 = 11;
/// The `GlobalZIndex` of the per-prefab quad LABELS — above their quad so the text reads.
const LABEL_Z: i32 = 12;
/// The `GlobalZIndex` of the control bar — above everything so STEP / AUTO are clickable.
const BAR_Z: i32 = 20;

/// The board quad's on-screen side length, as a viewport-height percentage — a square area
/// the cell grid maps into. Relative units (the responsive-UI rule), not px.
const BOARD_SIDE_VH: f32 = 80.0;
/// The dark board fill color — a near-black quad the light tinted quads read over (C2).
const BOARD_COLOR: Color = Color::srgb(0.08, 0.08, 0.10);

/// Inter-child gap of the control bar, in `Vh` (the menu's calibrated separation).
const BAR_GAP_VH: f32 = 1.388_89;

/// Insert the [`ProcgenViz`] model `OnEnter(DebugProcgenVisualizer)` — built by running the
/// sim space-packing pipeline against the loaded UUID-keyed [`PrefabRegistry2`] for the
/// loaded situation's theme ([`ThemeUuid`](gdtf_battle_sim::level::ThemeUuid)) + grid-size + a
/// seed (the [`BattleSeed`] override resource if a test injected one, else the deterministic
/// [`default_viz_seed`]).
///
/// Reads everything as `Option<Res<…>>` (the state-scoped-resource convention): an absent
/// registry / situation yields an EMPTY model (board extent only, no quads) rather than a
/// panic (fail-open). Param-only (`bevy-traps.md` #7). Ordered BEFORE [`spawn_viz_screen`].
pub(in crate::states::running::procgen_viz) fn insert_viz_model(
    mut commands: Commands,
    registry: Option<Res<PrefabRegistry2>>,
    situation: Option<Res<LoadedSituation>>,
    seed_override: Option<Res<BattleSeed>>,
) {
    // Grid-size + theme come from the loaded situation; absent it, the default sim extent +
    // the nil theme (so the visualizer is still reachable on a no-content harness — the v2
    // registry is then empty and `build` takes the empty-model fallback). GTW-492: the
    // visualizer drives the UUID-keyed v2 pipeline, so the theme is the situation's `ThemeUuid`
    // DIRECTLY (no `LevelTheme` shim) — the same key the migrated v2 prefabs author.
    let (grid_size, theme) = situation
        .as_deref()
        .map_or_else(Default::default, |s| (s.grid_size, s.theme));
    let seed = seed_override
        .as_deref()
        .copied()
        .unwrap_or_else(default_viz_seed);
    let model = ProcgenViz::build(registry.as_deref(), theme, grid_size, seed);
    commands.insert_resource(model);
}

/// Spawn the visualizer screen `OnEnter(DebugProcgenVisualizer)` — the themed backdrop root,
/// the dark board quad (C2), one light tinted quad per placement-sequence entry (initially
/// hidden — the draw layer reveals them, C2/C3), each with a name + size label, plus the
/// STEP / AUTO control bar (C1) and a status panel.
///
/// Reads the [`GdtfTheme`] + the just-inserted [`ProcgenViz`] model as `Option<Res<…>>`
/// (`bevy-traps.md` #1); no-ops the themed parts if the theme is absent (the menu precedent),
/// but always spawns the root + board so `OnExit` despawn + the test root-count hold. Every
/// node carries [`DespawnOnExit(RunningState::DebugProcgenVisualizer)`]. Ordered AFTER
/// [`insert_viz_model`].
pub(in crate::states::running::procgen_viz) fn spawn_viz_screen(
    mut commands: Commands,
    theme: Option<Res<GdtfTheme>>,
    model: Option<Res<ProcgenViz>>,
) {
    let Some(theme) = theme else {
        // No theme yet — spawn nothing (the menu precedent). The running app always has it.
        return;
    };
    let Some(model) = model else {
        // The OnEnter order guarantees the model is present; guard fail-open anyway.
        return;
    };

    // ROOT — a full-viewport themed backdrop column.
    let root = commands
        .spawn((
            ProcgenVizRoot,
            Themed::new(ThemeRole::Background),
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: Val::Vh(BAR_GAP_VH),
                ..default()
            },
            DespawnOnExit(RunningState::DebugProcgenVisualizer),
        ))
        .id();

    // TITLE — a themed heading.
    let title = commands
        .spawn((
            Themed::new(ThemeRole::Title),
            Text::new("PROCGEN VISUALIZER"),
            DespawnOnExit(RunningState::DebugProcgenVisualizer),
        ))
        .id();

    // BOARD QUAD — the single DARK whole-level quad (C2): a fixed-aspect square the cell grid
    // maps into. Per-prefab quads are absolutely-positioned children of it.
    let board = commands
        .spawn((
            BoardQuad,
            Node {
                width: Val::Vh(BOARD_SIDE_VH),
                height: Val::Vh(BOARD_SIDE_VH),
                position_type: PositionType::Relative,
                ..default()
            },
            BackgroundColor(BOARD_COLOR),
            GlobalZIndex(BOARD_Z),
            DespawnOnExit(RunningState::DebugProcgenVisualizer),
        ))
        .id();

    // One light tinted quad per placement-sequence entry, parented into the board quad and
    // positioned by its cell rectangle. Initially `Display::None` — the draw layer reveals
    // them as the reveal count crosses each index (C2/C3).
    let board_extent = model.board();
    for (index, quad) in model.quads().iter().enumerate() {
        spawn_prefab_quad(
            &mut commands,
            board,
            RevealIndex::new(index),
            quad,
            board_extent,
        );
    }

    // CONTROL BAR — the STEP / AUTO buttons (C1) + a status text, in a themed panel.
    let bar = spawn_control_bar(&mut commands, &theme);

    commands.entity(root).add_children(&[title, board, bar]);
}

/// Spawn one per-prefab light tinted quad (C2/C3) into the board quad, positioned + sized by
/// its cell rectangle against the board extent, with a name + size label child.
///
/// The quad starts `Display::None` (hidden); the draw layer flips it `Flex` once revealed.
/// Y is FLIPPED for display (board cell origin is bottom-left; UI top is `top: 0`), so a
/// higher cell-y maps to a smaller `top`.
fn spawn_prefab_quad(
    commands: &mut Commands,
    board: Entity,
    index: RevealIndex,
    quad: &VizQuad,
    board_extent: BoardExtent,
) {
    let placement = quad_placement(quad.rect(), board_extent);
    let tint = quad.tint();
    let label = format!(
        "{} {}x{}",
        quad.name().as_str(),
        *quad.size().width(),
        *quad.size().height()
    );

    let quad_entity = commands
        .spawn((
            prefab_quad_marker(index, quad),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Percent(placement.left),
                top: Val::Percent(placement.top),
                width: Val::Percent(placement.width),
                height: Val::Percent(placement.height),
                // Hidden until revealed: the draw layer flips this to `Flex` (C1).
                display: Display::None,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(tint.color()),
            GlobalZIndex(QUAD_Z),
            DespawnOnExit(RunningState::DebugProcgenVisualizer),
        ))
        .id();

    // Name + size label, a child of the quad so it rides the quad's reveal.
    let label_entity = commands
        .spawn((
            Text::new(label),
            TextColor(Color::srgb(0.05, 0.05, 0.08)),
            GlobalZIndex(LABEL_Z),
            DespawnOnExit(RunningState::DebugProcgenVisualizer),
        ))
        .id();

    commands.entity(quad_entity).add_child(label_entity);
    commands.entity(board).add_child(quad_entity);
}

/// Spawn the STEP / AUTO control bar (C1) in a themed panel, with a status text, and return
/// its root entity.
fn spawn_control_bar(commands: &mut Commands, theme: &GdtfTheme) -> Entity {
    let bar = spawn_panel(commands, theme);
    commands.entity(bar).insert((
        Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            column_gap: Val::Vh(BAR_GAP_VH),
            ..default()
        },
        GlobalZIndex(BAR_Z),
        DespawnOnExit(RunningState::DebugProcgenVisualizer),
    ));

    let step = spawn_button(
        commands,
        theme,
        ButtonLabel::new("Step"),
        (
            StepButton,
            DespawnOnExit(RunningState::DebugProcgenVisualizer),
        ),
    );
    let auto = spawn_button(
        commands,
        theme,
        ButtonLabel::new("Auto"),
        (
            AutoButton,
            DespawnOnExit(RunningState::DebugProcgenVisualizer),
        ),
    );
    commands.entity(bar).add_children(&[step, auto]);
    bar
}

/// A per-prefab quad's on-screen placement as percentages of the board quad — left / top /
/// width / height. Pure layout plumbing (percentages), not a domain value.
struct QuadPlacement {
    /// The quad's left edge as a percent of the board width.
    left:   f32,
    /// The quad's top edge as a percent of the board height (y flipped for display).
    top:    f32,
    /// The quad's width as a percent of the board width.
    width:  f32,
    /// The quad's height as a percent of the board height.
    height: f32,
}

/// Map a quad's cell rectangle to its on-screen placement (percent of the board quad),
/// flipping y so a higher cell-y maps to a smaller `top` (UI is top-down; cell origin is
/// bottom-left).
#[expect(
    clippy::cast_precision_loss,
    reason = "board cell coords / extents are tiny (<= MAX_GRID_SPAN = 60); the f32 \
              conversion is exact for this range, so the percentage math is exact"
)]
fn quad_placement(rect: QuadRect, board: BoardExtent) -> QuadPlacement {
    let bw = (*board.size().width()).max(1) as f32;
    let bh = (*board.size().height()).max(1) as f32;
    let x = *rect.min_x() as f32;
    let y = *rect.min_y() as f32;
    let w = *rect.extent().width() as f32;
    let h = *rect.extent().height() as f32;
    QuadPlacement {
        left:   x / bw * 100.0,
        // Flip y: the quad's TOP edge is the board's far edge minus the quad's max-y row.
        top:    (bh - (y + h)) / bh * 100.0,
        width:  w / bw * 100.0,
        height: h / bh * 100.0,
    }
}
