//! Reading a launch call's arguments into a typed [`LaunchSpec`] (GTW-875, GTW-808).
//!
//! Four optional arguments name what to launch — `package`, `features`, `working_dir`,
//! `env` — and each omitted one falls back to the HOST'S OWN default recipe, so a bare
//! `launch` launches the game and a bare `launch(host="editor")` launches the editor, each
//! over its own QA channel. Every rejection is a message the caller sees as
//! invalid-params, never a silent default: a typo'd working directory that quietly
//! launched the wrong checkout is the exact failure this parsing exists to remove.

use serde_json::Value;

use crate::lifecycle::{
    CargoPackage, EnvOverrides, EnvVar, EnvVarName, EnvVarValue, FeatureList, FeatureName,
    LaunchSpec, WorkingDir,
};

/// Read the launch recipe out of a launch call's arguments, filling every omitted one
/// from `defaults` — the recipe of the host the tool names.
///
/// The QA-channel variable names are taken from `defaults` and are NOT a caller argument:
/// they are what the child binary reads, so letting a call name them would only produce a
/// child with no listener on the port the host is about to probe.
///
/// # Errors
///
/// A message naming the offending argument when one is present but not the shape the
/// schema advertises.
pub fn parse_launch_spec(defaults: &LaunchSpec, args: &Value) -> Result<LaunchSpec, String> {
    let package = parse_package(args)?.unwrap_or_else(|| defaults.package().clone());
    let features = parse_features(args)?.unwrap_or_else(|| defaults.features().clone());
    let working_dir = parse_working_dir(args)?;
    let env = parse_env(args)?;
    Ok(LaunchSpec::new(
        package,
        features,
        working_dir,
        env,
        defaults.channel().clone(),
    ))
}

/// Read the optional `package` argument.
fn parse_package(args: &Value) -> Result<Option<CargoPackage>, String> {
    match args.get("package") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(name)) if !name.trim().is_empty() => {
            Ok(Some(CargoPackage::new(name.trim().to_owned())))
        }
        Some(_) => Err("`package` must be a non-empty cargo package name".to_owned()),
    }
}

/// Read the optional `features` argument — an array of names, or one comma-separated
/// string (the shape a cargo command line uses). An explicit empty list means "no
/// features", which is NOT the same as omitting the argument.
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

/// Split a comma-separated feature string into names, dropping empty pieces.
fn split_features(text: &str) -> Vec<FeatureName> {
    text.split(',')
        .map(str::trim)
        .filter(|piece| !piece.is_empty())
        .map(|piece| FeatureName::new(piece.to_owned()))
        .collect()
}

/// Read the optional `working_dir` argument, rejecting a path that is not a directory —
/// launching from a mistyped path would otherwise fall back to the host's own checkout and
/// report a pass against code nobody reviewed.
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

/// Read the optional `env` argument — an object of variable name to string value.
fn parse_env(args: &Value) -> Result<EnvOverrides, String> {
    match args.get("env") {
        None | Some(Value::Null) => Ok(EnvOverrides::default()),
        Some(Value::Object(map)) => {
            let mut vars = Vec::with_capacity(map.len());
            for (name, value) in map {
                let Some(text) = value.as_str() else {
                    return Err(format!("`env` value for `{name}` must be a string"));
                };
                vars.push(EnvVar::new(
                    EnvVarName::new(name.clone()),
                    EnvVarValue::new(text.to_owned()),
                ));
            }
            Ok(EnvOverrides::new(vars))
        }
        Some(_) => Err("`env` must be an object of variable name to string value".to_owned()),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::parse_launch_spec;
    use crate::lifecycle::LaunchSpec;

    /// Parse against the GAME's defaults — what a `launch` naming no host does.
    fn parse_game(args: &serde_json::Value) -> Result<LaunchSpec, String> {
        parse_launch_spec(&LaunchSpec::game_default(), args)
    }

    /// No arguments at all yields the default game recipe — the launch this tool made
    /// before it took any recipe arguments.
    #[test]
    fn empty_arguments_are_the_default_game_recipe() {
        let Ok(spec) = parse_game(&json!({})) else {
            unreachable!("an empty argument object parses");
        };
        assert_eq!(spec.package().as_str(), "grimdark_turfwar");
        assert_eq!(
            spec.features().render(),
            Some("dynamic_linking,net_qa".to_owned())
        );
        assert!(spec.working_dir().is_none());
        assert!(spec.env().is_empty());
    }

    /// A feature array, a package, an env object, and a real directory all come through.
    ///
    /// The directory fixture is deliberately NOT the test process's own: a recipe whose
    /// directory was dropped resolves to the host's current directory downstream, so a
    /// fixture equal to that could not tell "carried through" from "dropped".
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
            "package": "grimdark_turfwar",
            "features": ["dynamic_linking", "net_qa", "dev_tools"],
            "working_dir": dir,
            "env": { "GDTF_BATTLE_SEED": "42" },
        })) else {
            unreachable!("a full recipe parses");
        };
        assert_eq!(
            spec.features().render(),
            Some("dynamic_linking,net_qa,dev_tools".to_owned())
        );
        assert_eq!(
            spec.working_dir().map(|path| path.to_path_buf()),
            Some(dir.into())
        );
        let Some(first) = spec.env().first() else {
            unreachable!("the recipe carries its one override");
        };
        assert_eq!(first.name().as_str(), "GDTF_BATTLE_SEED");
    }

    /// `features` also accepts the comma-separated string a cargo command line uses.
    #[test]
    fn features_accept_a_comma_separated_string() {
        let Ok(spec) = parse_game(&json!({ "features": "net_qa, dev_tools" })) else {
            unreachable!("a comma-separated feature string parses");
        };
        assert_eq!(
            spec.features().render(),
            Some("net_qa,dev_tools".to_owned())
        );
    }

    /// A working directory that does not exist is rejected rather than dropped — a
    /// mistyped path must never silently launch the host's own checkout.
    #[test]
    fn a_missing_working_dir_is_rejected() {
        let result = parse_game(&json!({ "working_dir": "/no/such/tree/here" }));
        let Err(message) = result else {
            unreachable!("a non-existent working directory is rejected");
        };
        assert!(message.contains("working_dir"), "message: {message}");
    }

    /// A bare `launch(host="editor")` gets the EDITOR's recipe, not the game's — the defaults are
    /// the host's, not a single hardcoded set (GTW-808).
    #[test]
    fn empty_arguments_against_the_editor_defaults_are_the_editor_recipe() {
        let Ok(spec) = parse_launch_spec(&LaunchSpec::editor_default(), &json!({})) else {
            unreachable!("an empty argument object parses");
        };
        assert_eq!(spec.package().as_str(), "gdtf_content_editor_bin");
        assert_eq!(spec.channel().enable().as_str(), "GDTF_EDITOR_NET_QA");
    }

    /// A launch call cannot re-point the QA channel: the variable names come from the
    /// host's own recipe, so naming them in `env` leaves the channel intact.
    #[test]
    fn the_qa_channel_names_are_not_a_caller_argument() {
        let Ok(spec) = parse_launch_spec(
            &LaunchSpec::editor_default(),
            &json!({ "env": { "GDTF_NET_QA": "1" } }),
        ) else {
            unreachable!("an env override parses");
        };
        assert_eq!(spec.channel().enable().as_str(), "GDTF_EDITOR_NET_QA");
        assert_eq!(spec.channel().port().as_str(), "GDTF_EDITOR_NET_QA_PORT");
    }

    /// Wrongly shaped arguments are rejected with a message naming the argument.
    #[test]
    fn wrongly_shaped_arguments_are_rejected() {
        assert!(parse_game(&json!({ "package": 7 })).is_err());
        assert!(parse_game(&json!({ "features": [7] })).is_err());
        assert!(parse_game(&json!({ "env": { "A": 1 } })).is_err());
        assert!(parse_game(&json!({ "env": "A=1" })).is_err());
    }
}
