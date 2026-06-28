//! The DEV-ONLY procgen-visualizer draw sync (GTW-434 C2) — reveal each per-prefab quad as
//! the [`ProcgenViz`] model's reveal count crosses its index.
//!
//! A per-prefab quad spawns hidden (`Display::None`); this system flips a quad to
//! `Display::Flex` once the model has REVEALED its index (`revealed > index`), and back to
//! hidden if it is no longer revealed (so AUTO → re-enter resets cleanly). It MUTATES the
//! existing quad nodes in place (never despawn / respawn — the responsive-UI mutate rule),
//! and guards on the model resource existing (`bevy-traps.md` #1). The whole module is
//! `#[cfg(debug_assertions)]`-gated by its parent.

use bevy::prelude::*;

use crate::states::running::procgen_viz::{components::PrefabQuad, model::ProcgenViz};

/// Reveal / hide each per-prefab quad to match the model's current reveal count (C2/C3).
///
/// For every [`PrefabQuad`], sets its [`Node`](bevy::ui::Node) display to `Flex` when its
/// index is within the revealed count (`revealed > index`) and `None` otherwise — so STEP
/// shows one more quad and AUTO shows them all (C2). It ALSO keeps the quad's
/// [`BackgroundColor`] authoritative from its [`QuadTint`](super::super::model::QuadTint) role
/// (player = green / enemy = red / fill = neutral, C3), so the rendered fill always matches the
/// tint even after a live re-theme. Guarded `run_if(resource_exists::<ProcgenViz>)` by the
/// scene plugin. Param-only (`bevy-traps.md` #7).
pub(in crate::states::running::procgen_viz) fn sync_revealed_quads(
    model: Res<ProcgenViz>,
    mut quads: Query<(&PrefabQuad, &mut Node, &mut BackgroundColor)>,
) {
    let revealed = model.revealed();
    for (quad, mut node, mut color) in &mut quads {
        let should_show = quad.index().is_revealed_within(revealed);
        let want = if should_show {
            Display::Flex
        } else {
            Display::None
        };
        if node.display != want {
            node.display = want;
        }
        // Keep the tint authoritative (C3): re-derive the fill from the quad's role.
        let tinted = quad.tint().color();
        if color.0 != tinted {
            color.0 = tinted;
        }
    }
}
