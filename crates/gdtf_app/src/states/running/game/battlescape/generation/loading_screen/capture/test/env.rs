use super::super::register::LOADING_SHOT_ENV;

#[test]
fn env_var_name_is_the_scene_contract() {
    assert_eq!(LOADING_SHOT_ENV, "GDTF_LOADING_SHOT");
}
