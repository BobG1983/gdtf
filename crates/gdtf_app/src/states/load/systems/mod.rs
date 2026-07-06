mod cleanup;
pub(in crate::states::load) use cleanup::cleanup;

mod kick_off;
pub(in crate::states::load) use kick_off::kick_off_loads;

mod resolve;
// GTW-582: surface the shared tracing-capture scaffold to the whole crate's lib
// test binary (the C5 procgen warn-capture test lives under `states::running`).
#[cfg(test)]
pub(crate) use resolve::hot_reload_test_support;
pub(in crate::states::load) use resolve::{
    poll_and_resolve, redrive_injuries_on_asset_event, redrive_prefabs_on_asset_event,
};

mod transition;
pub(in crate::states::load) use transition::transition_to_intro;

mod validate;
pub(in crate::states::load) use validate::add_content_validation;
