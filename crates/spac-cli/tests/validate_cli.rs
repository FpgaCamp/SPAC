use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[test]
fn validate_accepts_minimal_example() {
    let output = Command::new(env!("CARGO_BIN_EXE_spac"))
        .args(["validate", "--config"])
        .arg(workspace_root().join("examples/minimal/spac.project.json"))
        .output()
        .expect("run spac validate");

    assert!(
        output.status.success(),
        "expected success, stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("\"status\": \"ok\""));
    assert!(stdout.contains("\"schema_version\": \"spac.project.v0\""));
}

#[test]
fn validate_rejects_unsupported_language() {
    let path = write_temp_config(
        "unsupported-language",
        r#"{
          "schema_version": "spac.project.v0",
          "project": {
            "name": "spac",
            "domain": "fpga-network-switch",
            "source_article": "https://arxiv.org/html/2604.21881v1",
            "selected_mvp": "MVP-A"
          },
          "language_policy": {
            "implementation_languages": ["Rust", "C++"],
            "generated_artifacts": ["HLS C++ header"]
          },
          "reproducibility": {
            "deterministic_seed": 260421881,
            "artifact_manifest_schema": "spac.artifact-manifest.v0"
          },
          "outputs": {
            "directory": "out"
          }
        }"#,
    );

    let output = Command::new(env!("CARGO_BIN_EXE_spac"))
        .args(["validate", "--config"])
        .arg(path)
        .output()
        .expect("run spac validate");

    assert!(!output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("\"status\": \"error\""));
    assert!(stdout.contains("SPAC_LANGUAGE_UNSUPPORTED"));
}

#[test]
fn validate_protocol_accepts_basic_example() {
    let output = Command::new(env!("CARGO_BIN_EXE_spac"))
        .args(["validate-protocol", "--protocol"])
        .arg(workspace_root().join("examples/protocols/basic.spac"))
        .output()
        .expect("run spac validate-protocol");

    assert!(
        output.status.success(),
        "expected success, stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("\"status\": \"ok\""));
    assert!(stdout.contains("\"protocol_name\": \"basic\""));
}

#[test]
fn analyze_layout_emits_metadata() {
    let output = Command::new(env!("CARGO_BIN_EXE_spac"))
        .args(["analyze-layout", "--protocol"])
        .arg(workspace_root().join("examples/protocols/basic.spac"))
        .args(["--bus-width", "8"])
        .output()
        .expect("run spac analyze-layout");

    assert!(
        output.status.success(),
        "expected success, stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("\"schema_version\": \"spac.metadata.v0\""));
    assert!(stdout.contains("\"total_header_bits\": 19"));
    assert!(stdout.contains("\"routing_key\": \"dst\""));
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root")
        .to_path_buf()
}

fn write_temp_config(name: &str, text: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!(
        "spac-{name}-{}-{}.json",
        std::process::id(),
        monotonic_nanos()
    ));
    fs::write(&path, text).expect("write temp config");
    path
}

fn monotonic_nanos() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time")
        .as_nanos()
}
