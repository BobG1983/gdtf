//! Shared grow/show/hide pool for overlay sprites.

use bevy::prelude::{DetectChangesMut, Mut, Visibility};

/// Reuse pooled items for each draw, grow when short, hide the rest.
pub fn draw_pool<'v, Item, Draw>(
    mut pooled: impl Iterator<Item = Item>,
    draws: impl IntoIterator<Item = Draw>,
    mut show: impl FnMut(Draw, &mut Item),
    mut grow: impl FnMut(Draw),
    mut hide: impl for<'a> FnMut(&'a mut Item) -> &'a mut Mut<'v, Visibility>,
) {
    for draw in draws {
        if let Some(mut item) = pooled.next() {
            show(draw, &mut item);
            hide(&mut item).set_if_neq(Visibility::Visible);
        } else {
            grow(draw);
        }
    }
    for mut item in pooled {
        hide(&mut item).set_if_neq(Visibility::Hidden);
    }
}
