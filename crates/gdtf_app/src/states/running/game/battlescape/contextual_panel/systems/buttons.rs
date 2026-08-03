use bevy::{prelude::*, ui::Interaction};
use gdtf_battle_input::{PanelNavOrder, contextual::PendingContextualIntents};
use gdtf_ui::{DisabledButton, spawn_button, theme::GdtfTheme};

use crate::states::running::game::battlescape::{
    contextual_panel::{
        components::ContextualPanelRoot,
        seam::{ContextualActButton, ContextualOffer, ContextualPanelAct, VisibleSlotRank},
    },
    focus_nav::CONTEXTUAL_NAV_BASE,
};

type PressedButton<M> = (Changed<Interaction>, With<M>, Without<DisabledButton>);

const fn is_press(interaction: Interaction) -> bool {
    matches!(interaction, Interaction::Pressed)
}

pub(in crate::states::running::game::battlescape) fn spawn_contextual_button<
    A: ContextualPanelAct,
>(
    mut commands: Commands,
    theme: Option<Res<GdtfTheme>>,
    roots: Query<Entity, With<ContextualPanelRoot>>,
) {
    let Some(theme) = theme else {
        return;
    };
    let Ok(root) = roots.single() else {
        return;
    };
    let button = spawn_button(
        &mut commands,
        &theme,
        A::label(),
        (
            A::Marker::default(),
            ContextualActButton::new(A::SLOT),
            VisibleSlotRank::unranked(),
            PanelNavOrder::new(CONTEXTUAL_NAV_BASE + u16::from(*A::SLOT)),
            Visibility::Hidden,
        ),
    );
    commands
        .entity(button)
        .entry::<Node>()
        .and_modify(|mut node| node.display = Display::None);
    commands.entity(root).add_child(button);
}

pub(in crate::states::running::game::battlescape) fn order_contextual_buttons(
    mut commands: Commands,
    buttons: Query<(Entity, &ContextualActButton)>,
    roots: Query<Entity, With<ContextualPanelRoot>>,
) {
    let Ok(root) = roots.single() else {
        return;
    };
    let mut ordered: Vec<(Entity, ContextualActButton)> = buttons
        .iter()
        .map(|(entity, slot)| (entity, *slot))
        .collect();
    ordered.sort_by_key(|(entity, slot)| (**slot, *entity));
    let children: Vec<Entity> = ordered.into_iter().map(|(entity, _)| entity).collect();
    commands.entity(root).replace_children(&children);
}

pub(in crate::states::running::game::battlescape) fn sync_contextual_button_visibility<
    A: ContextualPanelAct,
>(
    offer: Res<ContextualOffer<A>>,
    mut buttons: Query<(&mut Visibility, &mut Node), With<A::Marker>>,
) {
    set_button_shown(&mut buttons, offer.is_offered());
}

pub(in crate::states::running::game::battlescape) fn press_contextual_button<
    A: ContextualPanelAct,
>(
    offer: Res<ContextualOffer<A>>,
    presses: Query<&Interaction, PressedButton<A::Marker>>,
    mut pending: ResMut<PendingContextualIntents<A>>,
) {
    if presses.iter().copied().any(is_press)
        && let Some(target) = offer.target()
    {
        pending.push(target);
    }
}

pub(in crate::states::running::game::battlescape) fn sync_panel_root_visibility(
    buttons: Query<&Visibility, With<ContextualActButton>>,
    mut roots: Query<&mut Visibility, (With<ContextualPanelRoot>, Without<ContextualActButton>)>,
) {
    let any_offered = buttons
        .iter()
        .any(|visibility| *visibility == Visibility::Visible);
    set_visibility(&mut roots, any_offered);
}

fn set_visibility<F: bevy::ecs::query::QueryFilter>(
    query: &mut Query<&mut Visibility, F>,
    show: bool,
) {
    let want = if show {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
    for mut visibility in query {
        if *visibility != want {
            *visibility = want;
        }
    }
}

fn set_button_shown<F: bevy::ecs::query::QueryFilter>(
    query: &mut Query<(&mut Visibility, &mut Node), F>,
    show: bool,
) {
    let (want_visibility, want_display) = if show {
        (Visibility::Visible, Display::Flex)
    } else {
        (Visibility::Hidden, Display::None)
    };
    for (mut visibility, mut node) in query {
        if *visibility != want_visibility {
            *visibility = want_visibility;
        }
        if node.display != want_display {
            node.display = want_display;
        }
    }
}
