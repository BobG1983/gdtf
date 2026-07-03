//! One panel-layer module per CONTEXTUAL act (GTW-571): each owns its button marker,
//! its [`ContextualPanelAct`](super::seam::ContextualPanelAct) descriptor impl, and its
//! bespoke OFFER scan — the act's whole surface in this crate layer. The plugin
//! registers each with ONE
//! [`add_contextual_act_button`](super::registrar::ContextualPanelActAppExt::add_contextual_act_button)
//! line (see `docs/authoring/contextual-act-recipe.md`). The modules are visible to the
//! battlescape subtree directly (no re-export hop) so the plugin names
//! `acts::<act>::offer_*` and the panel's `test_support` ledger names each marker from
//! its defining module (the GTW-569 one-hop pattern).

pub(in crate::states::running::game::battlescape) mod enter_emplacement;
pub(in crate::states::running::game::battlescape) mod execute;
pub(in crate::states::running::game::battlescape) mod exit_emplacement;
pub(in crate::states::running::game::battlescape) mod melee;
pub(in crate::states::running::game::battlescape) mod open_door;
pub(in crate::states::running::game::battlescape) mod shove;
pub(in crate::states::running::game::battlescape) mod stabilize;
pub(in crate::states::running::game::battlescape) mod throw_grenade;
