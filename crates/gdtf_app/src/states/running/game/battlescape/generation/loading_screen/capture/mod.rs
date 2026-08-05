//! Dev capture of the generation loading screen, driven by `GDTF_LOADING_SHOT`.

mod register;
mod systems;

#[cfg(test)]
mod test;

pub(in crate::states::running::game::battlescape::generation) use register::register_loading_capture;
