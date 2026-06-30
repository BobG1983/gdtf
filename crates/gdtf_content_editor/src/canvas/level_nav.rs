//! Canvas **level navigation** (GTW-500 C1) — the [`CurrentEditLevel`] resource, the up/down
//! step systems (keyboard hotkeys + chrome buttons), and the current-level chrome readout.
//!
//! The GTW-423 canvas always drew the ground plane (`L0`) because no storey selector existed
//! (the canvas [`mod`](super) doc-comment named this OUT of scope until a selector landed). This
//! module IS that selector: a state-scoped [`CurrentEditLevel`] (a [`Level`] newtype) the author
//! steps up/down with `[` / `]` (and `PageUp` / `PageDown`) or the two chrome buttons, CLAMPED to
//! the prefab's `[0, levels-1]` storey range. The canvas render ([`sync_canvas`](super::sync)),
//! the paint flow ([`paint_cell`](super::paint)), and the hover ghost
//! ([`follow_hover_ghost`](super::ghost)) all read this resource instead of a hardcoded
//! `GROUND_LEVEL`, so stepping the level changes the rendered/painted slice live.

use bevy::prelude::*;
use gdtf_battle_sim::{level::GridSize, metric::Level};
use gdtf_ui::theme::GdtfTheme;

use crate::{CanvasRegion, mode::PrefabModeContent, mode_host::mode_host_under_region};

/// The storey the canvas is currently editing (GTW-500 C1) — the x/y slice the canvas draws,
/// paints, and previews the hover ghost on.
///
/// A named newtype over the sim's [`Level`] storey index (no-bare-types: the edited storey is a
/// domain coordinate, not a bare `u8`/`Level`-less value). PRIVATE inner, derived [`Deref`] to
/// the wrapped [`Level`]; the only mutators are [`CurrentEditLevel::stepped`] /
/// [`CurrentEditLevel::clamped`], which keep the value inside the prefab's storey range so the
/// canvas can never read a slice past the drawable volume. A state-scoped [`Resource`] (inserted
/// `OnEnter(Editing)`, removed `OnExit(Editing)` — bevy-traps #1), seeded to the ground storey.
#[derive(Resource, Deref, Clone, Copy, PartialEq, Eq, Debug)]
pub struct CurrentEditLevel(Level);

impl CurrentEditLevel {
    /// The ground storey (`L0`) — the level the editor opens on (the GTW-423 canvas's old
    /// hardcoded plane, now the seed of the selector).
    #[must_use]
    pub const fn ground() -> Self {
        Self(Level::new(0))
    }

    /// This level stepped by `delta` storeys, CLAMPED to the prefab's `[0, levels-1]` range so the
    /// result is always inside the drawable volume (C1 — no out-of-bounds storey). A step that
    /// would leave the range saturates at the nearest end (the floor/ceiling stays put).
    #[must_use]
    pub(crate) fn stepped(self, delta: LevelStep, size: GridSize) -> Self {
        let current = i32::from(*self.0);
        let max = i32::from(*size.levels()).saturating_sub(1);
        let next = (current + delta.delta()).clamp(0, max);
        // `next` is clamped into `[0, max]` where `max < levels <= MAX_LEVELS` (a `u8`), so the
        // `u8` conversion is always in range — no panic, no truncation in practice.
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "next is clamped to [0, levels-1] with levels <= MAX_LEVELS (u8), so it fits \
                      a u8 without wrap or sign-flip"
        )]
        let storey = next as u8;
        Self(Level::new(storey))
    }

    /// This level CLAMPED to the prefab's `[0, levels-1]` range — used when the grid shrinks below
    /// the current storey (a size change must never leave the selector pointing past the new
    /// volume).
    #[must_use]
    pub fn clamped(self, size: GridSize) -> Self {
        self.stepped(LevelStep::none(), size)
    }

    /// The wrapped storey index — the [`Level`] the canvas render / paint / ghost read.
    #[must_use]
    pub const fn level(self) -> Level {
        self.0
    }
}

/// A signed level-navigation step in storeys (no-bare-types: a step is a domain delta, not a bare
/// `i32`). `+1` steps up one storey, `-1` down; `0` is the identity used for a re-clamp.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct LevelStep(i32);

impl LevelStep {
    /// Step UP one storey (toward the ceiling).
    pub(crate) const fn up() -> Self {
        Self(1)
    }

    /// Step DOWN one storey (toward the ground).
    pub(crate) const fn down() -> Self {
        Self(-1)
    }

    /// No step — the identity used to re-clamp the current level after a grid shrink.
    pub(crate) const fn none() -> Self {
        Self(0)
    }

    /// The signed storey delta.
    const fn delta(self) -> i32 {
        self.0
    }
}

/// Which way a level-nav CHROME BUTTON steps the [`CurrentEditLevel`] (C1) — a marker carrying
/// its [`LevelStep`] so [`level_nav_buttons`] maps a button press to a step.
///
/// A named newtype-bearing component (no-bare-types): the direction is the typed [`LevelStep`],
/// not a bare bool/int. One marker per button (the up button carries [`LevelStep::up`], the down
/// button [`LevelStep::down`]).
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub struct LevelNavButton {
    /// The step this button applies on a press.
    step: LevelStep,
}

impl LevelNavButton {
    /// Build a nav-button marker for a [`LevelStep`].
    const fn new(step: LevelStep) -> Self {
        Self { step }
    }

    /// The step this button applies.
    const fn step(self) -> LevelStep {
        self.step
    }
}

/// Marker on the canvas chrome's CURRENT-LEVEL readout [`Text`] (C1) — the prominent
/// `Level n / m` line [`refresh_level_readout`] rewrites whenever the level or grid changes. A
/// unit marker (no-bare-types).
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub struct LevelReadout;

/// `OnEnter(Editing)`: spawn the canvas level-nav chrome — a thin bar (DOWN button, the
/// current-level readout, UP button) floated over the top of the [`CanvasRegion`] (C1).
///
/// Parented into the region's [`PrefabModeContent`] host (deferred, the
/// `spawn_canvas_scroll` precedent) so the whole nav bar hides in TERRAIN / THEME mode with the
/// rest of the prefab canvas. It is absolutely positioned with a [`GlobalZIndex`] above the cells
/// (bevy-traps #8) so it floats over the canvas top edge WITHOUT disturbing the scroll list's flex
/// sizing (a flow sibling would fight the scroll area's `height: 100%`).
pub(crate) fn spawn_level_nav(mut commands: Commands, theme: Res<GdtfTheme>) {
    let text_color = *theme.text.text_color;
    let bg = *theme.panel.color;
    let border = *theme.panel.border_color;

    let down = nav_button(
        &mut commands,
        LevelStep::down(),
        "v Level",
        text_color,
        border,
    );
    let readout = commands
        .spawn((
            LevelReadout,
            Text::new("Level 1 / 8"),
            TextColor(text_color),
            Node {
                margin: UiRect::horizontal(Val::Vw(0.8)),
                ..default()
            },
        ))
        .id();
    let up = nav_button(
        &mut commands,
        LevelStep::up(),
        "^ Level",
        text_color,
        border,
    );

    let bar = commands
        .spawn((
            // Above the cells (bevy-traps #8) so the chrome floats over the canvas top edge; below
            // the shell bars' z (10) so it never overlaps the top bar.
            GlobalZIndex(CANVAS_CHROME_Z),
            BackgroundColor(bg),
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(0.0),
                left: Val::Px(0.0),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                padding: UiRect::axes(Val::Vw(0.4), Val::Vh(0.3)),
                ..default()
            },
        ))
        .add_children(&[down, readout, up])
        .id();

    commands.queue(move |world: &mut World| {
        let Some(host) = mode_host_under_region::<CanvasRegion, PrefabModeContent>(world) else {
            return;
        };
        if let Ok(mut host_entity) = world.get_entity_mut(host) {
            host_entity.add_child(bar);
        }
    });
}

/// The [`GlobalZIndex`] the canvas chrome (the level-nav bar + the zoom readout) floats on —
/// above the cells (default `0`) and the ghost (`1`) so it draws over the canvas, but below the
/// shell top/status bars (`10`) so it never overlaps them (bevy-traps #8).
pub(crate) const CANVAS_CHROME_Z: i32 = 5;

/// Spawn one level-nav button carrying its [`LevelNavButton`] step marker.
fn nav_button(
    commands: &mut Commands,
    step: LevelStep,
    label: &str,
    text_color: Color,
    border: Color,
) -> Entity {
    let text = commands
        .spawn((Text::new(label.to_owned()), TextColor(text_color)))
        .id();
    commands
        .spawn((
            LevelNavButton::new(step),
            Button,
            BackgroundColor(border),
            Node {
                padding: UiRect::axes(Val::Vw(0.4), Val::Vh(0.2)),
                ..default()
            },
        ))
        .add_child(text)
        .id()
}

/// `Update` (in `Editing`): keyboard hotkeys step the [`CurrentEditLevel`] up/down (C1).
///
/// Mirrors [`mode_hotkeys`](crate::mode::mode_hotkeys): `]` / `PageUp` step UP a storey, `[` /
/// `PageDown` step DOWN. (Both vocabularies are provided — the bracket keys are the editor-native
/// step and `PageUp`/`PageDown` are the battle keybind vocabulary, unused in the editor until
/// now.) Each step CLAMPS to the prefab's `[0, levels-1]` range via [`CurrentEditLevel::stepped`],
/// written with [`set_if_neq`](DetectChangesMut::set_if_neq) so an at-the-edge press neither
/// re-renders nor re-reads. Guarded on the optional state-scoped resources (bevy-traps #1).
pub(crate) fn level_nav_hotkeys(
    keys: Res<ButtonInput<KeyCode>>,
    level: Option<ResMut<CurrentEditLevel>>,
    session: Option<Res<crate::session::MapEditorSession>>,
) {
    let (Some(mut level), Some(session)) = (level, session) else {
        return;
    };
    let step = if keys.just_pressed(KeyCode::BracketRight) || keys.just_pressed(KeyCode::PageUp) {
        Some(LevelStep::up())
    } else if keys.just_pressed(KeyCode::BracketLeft) || keys.just_pressed(KeyCode::PageDown) {
        Some(LevelStep::down())
    } else {
        None
    };
    if let Some(step) = step {
        level.set_if_neq(level.stepped(step, session.grid_size()));
    }
}

/// The press-edge query filter [`level_nav_buttons`] reads — a [`LevelNavButton`] whose
/// [`Interaction`] changed this frame. A named alias to keep the system signature under clippy's
/// `type_complexity` gate (the [`paint_cell`](super::paint) `PressedCell` precedent).
type PressedNavButton = (Changed<Interaction>, With<LevelNavButton>);

/// `Update` (in `Editing`): a level-nav CHROME BUTTON press steps the [`CurrentEditLevel`] (C1).
///
/// Reads each pressed [`LevelNavButton`] (the press edge — `Changed<Interaction>` filtered to a
/// `Pressed` state, the [`paint_cell`](super::paint) precedent) and applies its [`LevelStep`],
/// CLAMPED to the prefab's `[0, levels-1]` range, with [`set_if_neq`] hygiene. Guarded on the
/// optional state-scoped resources (bevy-traps #1).
pub(crate) fn level_nav_buttons(
    pressed: Query<(&Interaction, &LevelNavButton), PressedNavButton>,
    level: Option<ResMut<CurrentEditLevel>>,
    session: Option<Res<crate::session::MapEditorSession>>,
) {
    let (Some(mut level), Some(session)) = (level, session) else {
        return;
    };
    for (interaction, button) in &pressed {
        if matches!(interaction, Interaction::Pressed) {
            level.set_if_neq(level.stepped(button.step(), session.grid_size()));
        }
    }
}

/// `Update` (in `Editing`): re-clamp the [`CurrentEditLevel`] when the grid shrinks below it (C1).
///
/// A size change (the right-panel size commit) can drop `levels` below the current storey; this
/// re-clamps so the selector never points past the new drawable volume. Runs only when the
/// session changed this frame, with [`set_if_neq`] so an unrelated session mutation (a palette
/// pick) is a no-op. Guarded on the optional state-scoped resources (bevy-traps #1).
pub(crate) fn clamp_level_to_grid(
    level: Option<ResMut<CurrentEditLevel>>,
    session: Option<Res<crate::session::MapEditorSession>>,
) {
    let (Some(mut level), Some(session)) = (level, session) else {
        return;
    };
    if !session.is_changed() {
        return;
    }
    level.set_if_neq(level.clamped(session.grid_size()));
}

/// `Update` (in `Editing`): rewrite the current-level chrome readout `Level n / m` (C1).
///
/// Mutates the [`LevelReadout`] [`Text`] in place (the ui-mutate-in-place rule) whenever the level
/// or grid changed, showing the 1-based current storey over the prefab's storey count. Guarded on
/// the optional state-scoped resources (bevy-traps #1).
pub(crate) fn refresh_level_readout(
    level: Option<Res<CurrentEditLevel>>,
    session: Option<Res<crate::session::MapEditorSession>>,
    mut readouts: Query<&mut Text, With<LevelReadout>>,
) {
    let (Some(level), Some(session)) = (level, session) else {
        return;
    };
    if !(level.is_changed() || session.is_changed()) {
        return;
    }
    let current = u16::from(*level.level()).saturating_add(1);
    let total = u16::from(*session.grid_size().levels());
    let line = format!("Level {current} / {total}");
    for mut text in &mut readouts {
        if **text != line {
            (**text).clone_from(&line);
        }
    }
}
