use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const BUS_WIDTH_BITS: &str = "64";

#[test]
fn workload_protocol_metadata_matches_golden_fixtures() {
    for case in golden_cases() {
        let actual = analyze_layout(&case.protocol_path);
        let expected = read_json(&case.golden_path);

        assert_eq!(
            actual, expected,
            "metadata mismatch for {}",
            case.protocol_name
        );
        assert_eq!(actual["schema_version"], "spac.metadata.v0");
        assert_eq!(actual["bus_width_bits"], 64);
        assert!(
            actual["semantic_bindings"]["routing_key"].is_string(),
            "{} metadata must include routing_key semantic binding",
            case.protocol_name
        );
    }
}

fn analyze_layout(protocol_path: &str) -> Value {
    let output = Command::new(env!("CARGO_BIN_EXE_spac"))
        .args(["analyze-layout", "--protocol"])
        .arg(workspace_root().join(protocol_path))
        .args(["--bus-width", BUS_WIDTH_BITS])
        .output()
        .expect("run spac analyze-layout");

    assert!(
        output.status.success(),
        "expected analyze-layout success for {protocol_path}; stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    serde_json::from_slice(&output.stdout).expect("metadata stdout must be JSON")
}

fn read_json(relative_path: &str) -> Value {
    let text = fs::read_to_string(workspace_root().join(relative_path))
        .unwrap_or_else(|error| panic!("failed to read {relative_path}: {error}"));
    serde_json::from_str(&text).unwrap_or_else(|error| {
        panic!("failed to parse {relative_path} as JSON: {error}");
    })
}

fn golden_cases() -> Vec<GoldenCase> {
    vec![
        GoldenCase::new("hft"),
        GoldenCase::new("rl_all_reduce"),
        GoldenCase::new("datacenter"),
        GoldenCase::new("industrial"),
        GoldenCase::new("underwater_sensor"),
    ]
}

struct GoldenCase {
    protocol_name: &'static str,
    protocol_path: String,
    golden_path: String,
}

impl GoldenCase {
    fn new(protocol_name: &'static str) -> Self {
        Self {
            protocol_name,
            protocol_path: format!("examples/protocols/{protocol_name}.spac"),
            golden_path: format!("examples/golden/metadata/{protocol_name}.bus64.metadata.json"),
        }
    }
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root")
        .to_path_buf()
}
