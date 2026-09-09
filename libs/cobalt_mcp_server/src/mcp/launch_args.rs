//! Parse MCP launch tool arguments into a [`crate::lifecycle::launch::LaunchSpec`].

use serde_json::Value;

use crate::lifecycle::{
    CargoPackage, CargoProfile, FeatureList, FeatureName, LaunchSpec, WorkingDir,
};

/// Merge tool args onto `defaults` to produce a launch recipe.
///
/// # Errors
///
/// Returns a human-readable message when an argument has the wrong shape, or when the call
/// carries an `env` a launch no longer takes.
pub fn parse_launch_spec(defaults: &LaunchSpec, args: &Value) -> Result<LaunchSpec, String> {
    refuse_env(args)?;
    let package = parse_package(args)?.unwrap_or_else(|| defaults.package().clone());
    let features = parse_features(args)?.unwrap_or_else(|| defaults.features().clone());
    let profile = parse_profile(args)?;
    let working_dir = parse_working_dir(args)?;
    Ok(LaunchSpec::new(package, features, working_dir).with_profile(profile))
}

fn parse_profile(args: &Value) -> Result<Option<CargoProfile>, String> {
    match args.get("profile") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(name)) if !name.trim().is_empty() => {
            Ok(Some(CargoProfile::new(name.trim().to_owned())))
        }
        Some(_) => Err("`profile` must be a non-empty cargo profile name".to_owned()),
    }
}

fn parse_package(args: &Value) -> Result<Option<CargoPackage>, String> {
    match args.get("package") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(name)) if !name.trim().is_empty() => {
            Ok(Some(CargoPackage::new(name.trim().to_owned())))
        }
        Some(_) => Err("`package` must be a non-empty cargo package name".to_owned()),
    }
}

fn parse_features(args: &Value) -> Result<Option<FeatureList>, String> {
    match args.get("features") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(joined)) => Ok(Some(FeatureList::new(split_features(joined)))),
        Some(Value::Array(items)) => {
            let mut names = Vec::with_capacity(items.len());
            for item in items {
                let Some(text) = item.as_str() else {
                    return Err("`features` entries must be strings".to_owned());
                };
                names.extend(split_features(text));
            }
            Ok(Some(FeatureList::new(names)))
        }
        Some(_) => {
            Err("`features` must be an array of names or a comma-separated string".to_owned())
        }
    }
}

fn split_features(text: &str) -> Vec<FeatureName> {
    text.split(',')
        .map(str::trim)
        .filter(|piece| !piece.is_empty())
        .map(|piece| FeatureName::new(piece.to_owned()))
        .collect()
}

fn parse_working_dir(args: &Value) -> Result<Option<WorkingDir>, String> {
    match args.get("working_dir") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(path)) if !path.trim().is_empty() => {
            let dir = WorkingDir::new(path.trim().into());
            if dir.is_dir() {
                Ok(Some(dir))
            } else {
                Err(format!(
                    "`working_dir` is not an existing directory: {path}"
                ))
            }
        }
        Some(_) => Err("`working_dir` must be a non-empty directory path".to_owned()),
    }
}

fn refuse_env(args: &Value) -> Result<(), String> {
    match args.get("env") {
        None | Some(Value::Null) => Ok(()),
        Some(_) => Err(
            "launch takes no `env`: a child is configured by its package, features, \
                        profile and working directory, the spawner sets no variable at all, and \
                        the launch port reaches the child on its command line"
                .to_owned(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::parse_launch_spec;
    use crate::{hosts::test_support::registered, lifecycle::LaunchSpec};

    fn parse_against(name: &str, args: &serde_json::Value) -> Result<LaunchSpec, String> {
        parse_launch_spec(&registered(name, 4100, false).default_spec(), args)
    }

    fn parse_game(args: &serde_json::Value) -> Result<LaunchSpec, String> {
        parse_against("alpha", args)
    }

    #[test]
    fn empty_arguments_are_the_registered_hosts_own_recipe() {
        let Ok(spec) = parse_game(&json!({})) else {
            unreachable!("an empty argument object parses");
        };
        assert_eq!(spec.package().as_str(), "alpha_package");
        assert_eq!(spec.features().render(), Some("alpha_feature".to_owned()));
        assert!(spec.working_dir().is_none());
    }

    #[test]
    fn full_recipe_parses_every_argument() {
        let temp = std::env::temp_dir();
        let Ok(here) = std::env::current_dir() else {
            unreachable!("the test process has a current directory");
        };
        assert_ne!(temp, here, "the fixture directory differs from the host's");
        let Some(dir) = temp.to_str() else {
            unreachable!("the system temp directory path is valid UTF-8");
        };
        let Ok(spec) = parse_game(&json!({
            "package": "another_package",
            "features": ["dynamic_linking", "dev_tools"],
            "profile": "release",
            "working_dir": dir,
        })) else {
            unreachable!("a full recipe parses");
        };
        assert_eq!(spec.package().as_str(), "another_package");
        assert_eq!(
            spec.features().render(),
            Some("dynamic_linking,dev_tools".to_owned())
        );
        assert_eq!(
            spec.profile().map(|profile| profile.to_string()),
            Some("release".to_owned())
        );
        assert_eq!(
            spec.working_dir().map(|path| path.to_path_buf()),
            Some(dir.into())
        );
    }

    #[test]
    fn a_profile_argument_reaches_the_recipe_and_is_absent_by_default() {
        let Ok(release) = parse_game(&json!({ "profile": "release" })) else {
            unreachable!("a profile name parses");
        };
        assert_eq!(
            release.profile().map(|profile| profile.to_string()),
            Some("release".to_owned())
        );
        let Ok(plain) = parse_game(&json!({})) else {
            unreachable!("an empty argument object parses");
        };
        assert_eq!(plain.profile(), None);
    }

    #[test]
    fn features_accept_a_comma_separated_string() {
        let Ok(spec) = parse_game(&json!({ "features": "dev_tools" })) else {
            unreachable!("a comma-separated feature string parses");
        };
        assert_eq!(spec.features().render(), Some("dev_tools".to_owned()));
    }

    #[test]
    fn a_missing_working_dir_is_rejected() {
        let result = parse_game(&json!({ "working_dir": "/no/such/tree/here" }));
        let Err(message) = result else {
            unreachable!("a non-existent working directory is rejected");
        };
        assert!(message.contains("working_dir"), "message: {message}");
    }

    #[test]
    fn empty_arguments_against_a_second_host_are_that_hosts_recipe() {
        let Ok(spec) = parse_against("beta", &json!({})) else {
            unreachable!("an empty argument object parses");
        };
        assert_eq!(spec.package().as_str(), "beta_package");
        assert_eq!(spec.features().render(), Some("beta_feature".to_owned()));
    }

    #[test]
    fn an_env_argument_is_refused_rather_than_ignored() {
        for shape in [
            json!({ "env": { "ALPHA_CHANNEL": "1" } }),
            json!({ "env": { "SAMPLE_SEED": "42" } }),
            json!({ "env": "A=1" }),
        ] {
            let Err(message) = parse_against("beta", &shape) else {
                unreachable!("a launch that names `env` is refused: {shape}");
            };
            assert!(
                message.contains("env"),
                "the refusal names the argument it will not take: {message}"
            );
        }
        let Ok(spec) = parse_against("beta", &json!({ "env": null })) else {
            unreachable!("an absent `env` is not an argument at all");
        };
        assert_eq!(spec.package().as_str(), "beta_package");
    }

    #[test]
    fn wrongly_shaped_arguments_are_rejected() {
        assert!(parse_game(&json!({ "package": 7 })).is_err());
        assert!(parse_game(&json!({ "features": [7] })).is_err());
        assert!(parse_game(&json!({ "profile": 7 })).is_err());
        assert!(parse_game(&json!({ "profile": "  " })).is_err());
    }
}
