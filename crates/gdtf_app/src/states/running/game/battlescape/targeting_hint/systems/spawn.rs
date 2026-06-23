//! Spawns + despawns the battlescape targeting-fog HINT Text node (GTW-11).
//!
//! [`spawn_targeting_hint`] runs `OnEnter(BattleScapeState::BattleRunning)` and builds the ONE
//! themed [`Text`](bevy::prelude::Text) node carrying the [`TargetingHintText`] marker, anchored
//! as an ABSOLUTE bottom-centre overlay just above the bottom bar, starting EMPTY and
//! [`Visibility::Hidden`]. [`update_targeting_hint`](super::update::update_targeting_hint) fills
//! it with the canon "unseen — hold your fire" string + shows it whenever the targeted cell is
//! non-VISIBLE, and hides it otherwise — mutating the node in place (never respawning).
//!
//! [`despawn_targeting_hint`] runs `OnExit(BattleScapeState::BattleRunning)` and despawns the
//! node by its [`TargetingHintText`] marker, so the hint is battle-scoped (the sibling
//! status-panel lifecycle).

use bevy::{
    prelude::*,
    text::{FontSize, TextColor as UiTextColor, TextFont},
    ui::{GlobalZIndex, Node, PositionType, Val},
};
use gdtf_ui::{
    theme::GdtfTheme,
    themed::{ThemeRole, Themed},
};

use crate::states::running::game::battlescape::targeting_hint::components::TargetingHintText;

/// The hint's stacking order ([`GlobalZIndex`] — the higher, the nearer the viewer).
///
/// A framework-plumbing `const` fed straight to a [`GlobalZIndex`] (the framework carve-out, not
/// a domain value). Set strictly ABOVE the opaque bottom bar's `GlobalZIndex(10)` and the on-bar
/// clusters (`11`) and the combat log (`11`) so the hint is never occluded by an opaque sibling
/// (`bevy-traps.md` #8 — a Visible, sized, on-screen node can still draw nothing if a
/// higher-`GlobalZIndex` opaque sibling paints over it). Above the contextual panel (`20`) too.
const TARGETING_HINT_Z: i32 = 30;

/// The hint's distance UP from the bottom of the viewport, as a fraction of the viewport HEIGHT
/// ([`Val::Vh`](bevy::ui::Val::Vh)). A relative unit (`ui-responsive-not-px`) so the hint floats
/// above the bottom bar regardless of window size.
const HINT_BOTTOM_VH: f32 = 14.0;

/// Builds the hint Text node on `OnEnter(BattleScapeState::BattleRunning)`.
///
/// Reads the live [`GdtfTheme`] as `Option<Res<GdtfTheme>>` and no-ops if it is absent
/// (`bevy-traps.md` #1, the sibling status-panel precedent — in the running app the theme is
/// present by battle start). With the theme present it spawns ONE [`Text`] node tagged
/// [`TargetingHintText`], laid out as an ABSOLUTE bottom-centre overlay (responsive
/// [`Val::Vw`](bevy::ui::Val) / [`Val::Vh`](bevy::ui::Val), per `ui-responsive-not-px`), with a
/// [`GlobalZIndex`] above the opaque siblings, starting EMPTY + [`Visibility::Hidden`]
/// ([`update_targeting_hint`](super::update::update_targeting_hint) fills + shows it).
///
/// Param-only (`bevy-traps.md` #7): [`Commands`] for the spawn + the theme read.
pub(in crate::states::running::game::battlescape) fn spawn_targeting_hint(
    mut commands: Commands,
    theme: Option<Res<GdtfTheme>>,
) {
    let Some(theme) = theme else {
        // No theme yet — spawn nothing rather than an un-themed hint (the status-panel
        // precedent). The running app always has it by the time a battle starts.
        return;
    };

    let text_color = *theme.text.text_color;
    let text_font = TextFont {
        font: theme.text.font.clone().into(),
        font_size: FontSize::Px(*theme.text.font_size_pt),
        ..default()
    };

    commands.spawn((
        TargetingHintText,
        Themed::new(ThemeRole::Text),
        // EMPTY until the update fills it with the canon string (or clears it when hidden).
        Text::new(String::new()),
        UiTextColor(text_color),
        text_font,
        // ABSOLUTE bottom-centre overlay (responsive units, `ui-responsive-not-px`): the hint
        // floats above the bottom bar, horizontally centred, hovering over the map.
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Vh(HINT_BOTTOM_VH),
            left: Val::Vw(0.0),
            right: Val::Vw(0.0),
            justify_content: JustifyContent::Center,
            ..default()
        },
        // ABOVE the opaque siblings so it is never occluded (`bevy-traps.md` #8).
        GlobalZIndex(TARGETING_HINT_Z),
        // Hidden until the update shows it on a non-VISIBLE targeted cell.
        Visibility::Hidden,
    ));
}

/// Despawns the targeting hint on `OnExit(BattleScapeState::BattleRunning)`.
///
/// Recursively despawns the [`TargetingHintText`] node so the hint is gone the moment the battle
/// leaves `BattleRunning` — battle-scoped lifecycle (the sibling status-panel cleanup). Param-only
/// (`bevy-traps.md` #7): [`Commands`] + a `Query<Entity, With<TargetingHintText>>`.
pub(in crate::states::running::game::battlescape) fn despawn_targeting_hint(
    mut commands: Commands,
    hints: Query<Entity, With<TargetingHintText>>,
) {
    for hint in &hints {
        commands.entity(hint).despawn();
    }
}
