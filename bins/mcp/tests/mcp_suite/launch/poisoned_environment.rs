//! A launch hands the child no environment, and a poisoned one cannot move its port.
//! It writes the four retired names to prove that, so the census excludes it by name.

use std::{
    fs,
    net::{Ipv4Addr, TcpListener},
    path::{Path, PathBuf},
    process::Stdio,
};

use cobalt_mcp_server::{
    CargoPackage, FeatureList, LaunchSpec, McpPort, WorkingDir, build_command,
};

const PROBE_PACKAGE: &str = "launch_poison_probe";

// The four retired names, each set to a value no launch would ever pick.
const POISON: [(&str, &str); 4] = [
    ("GDTF_MCP_PORT", "1"),
    ("EDITOR_MCP_PORT", "2"),
    ("GDTF_MCP", "3"),
    ("GDTF_EDITOR_MCP", "4"),
];

// Prints the port it was handed on the command line, then every poisoned name it can read.
const PROBE_MAIN: &str = r#"fn main() {
    let mut after_flag = std::env::args_os().skip_while(|arg| *arg != "--mcp-port");
    let port = match after_flag.nth(1).and_then(|value| value.into_string().ok()) {
        Some(value) => value,
        None => "absent".to_owned(),
    };
    println!("mcp-port={port}");
    for name in ["GDTF_MCP_PORT", "EDITOR_MCP_PORT", "GDTF_MCP", "GDTF_EDITOR_MCP"] {
        let shown = match std::env::var(name) {
            Ok(value) => value,
            Err(_) => "absent".to_owned(),
        };
        println!("{name}={shown}");
    }
}
"#;

// A port the operating system says is free right now.
fn free_port() -> u16 {
    let Ok(listener) = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)) else {
        unreachable!("the test can bind a loopback port");
    };
    let Ok(address) = listener.local_addr() else {
        unreachable!("a bound listener knows its own address");
    };
    address.port()
}

fn write_probe_package(path: &Path) {
    let Ok(()) = fs::create_dir_all(path.join("src")) else {
        unreachable!("the test can create the package's src directory");
    };
    let manifest = format!(
        "[workspace]\n[package]\nname = \"{PROBE_PACKAGE}\"\nversion = \"0.0.0\"\n\
         edition = \"2021\"\n"
    );
    let Ok(()) = fs::write(path.join("Cargo.toml"), manifest) else {
        unreachable!("the test can write the probe manifest");
    };
    let Ok(()) = fs::write(path.join("src/main.rs"), PROBE_MAIN) else {
        unreachable!("the test can write the probe main");
    };
}

// The probe lives under cargo's tmp dir for this crate, so runs after the first find it built.
fn probe_package_kept_warm() -> PathBuf {
    let base =
        std::env::var_os("CARGO_TARGET_TMPDIR").map_or_else(std::env::temp_dir, PathBuf::from);
    let path = base.join("launch-poison-probe");
    write_probe_package(&path);
    path
}

#[test]
fn the_launcher_sets_no_variable_and_a_poisoned_one_cannot_move_the_childs_port() {
    let spec = LaunchSpec::new(
        CargoPackage::new(PROBE_PACKAGE.to_owned()),
        FeatureList::default(),
        Some(WorkingDir::new(probe_package_kept_warm())),
    );
    let port = free_port();
    let mut command = build_command(McpPort::new(port), &spec);

    let handed: Vec<String> = command
        .get_envs()
        .map(|(key, _)| key.to_string_lossy().into_owned())
        .collect();
    assert!(
        handed.is_empty(),
        "the launcher sets no variable on the child at all, so a later poison cannot overwrite \
         one it set: {handed:?}"
    );

    for (name, value) in POISON {
        command.env(name, value);
    }
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    // The first run compiles the probe; later runs find it built.
    let Ok(output) = command.output() else {
        unreachable!("cargo runs the probe package");
    };
    let printed = String::from_utf8_lossy(&output.stdout).into_owned();

    assert!(
        printed.contains(&format!("mcp-port={port}")),
        "the child took the launch port off its own `--mcp-port` argument: {printed}"
    );
    for (name, value) in POISON {
        assert!(
            printed.contains(&format!("{name}={value}")),
            "{name} reached the child holding the poison, and changed nothing: {printed}"
        );
    }
}
