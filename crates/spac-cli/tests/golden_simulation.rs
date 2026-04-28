use serde_json::Value;
use spac_core::sha256_file_hex;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[test]
fn simulation_reports_match_golden_fixtures() {
    for case in golden_cases() {
        let actual = simulate(&case);
        let expected = read_json(case.golden_path);

        assert_eq!(actual, expected, "simulation mismatch for {}", case.name);
        assert_eq!(actual["schema_version"], "spac.simulation-run.v0");
        assert_eq!(actual["trust_level"], "software_model");
        assert!(
            actual["warnings"][0]
                .as_str()
                .expect("warning must be a string")
                .contains("no HLS synthesis"),
            "{} must keep simulator evidence limitations explicit",
            case.name
        );
    }
}

fn simulate(case: &GoldenCase) -> Value {
    let out_dir = write_temp_directory(case.name);
    let output = Command::new(env!("CARGO_BIN_EXE_spac"))
        .args(["simulate", "--architecture"])
        .arg(workspace_root().join(case.architecture_path))
        .args(["--trace"])
        .arg(workspace_root().join(case.trace_path))
        .args(["--out"])
        .arg(&out_dir)
        .output()
        .unwrap_or_else(|error| panic!("run spac simulate for {}: {error}", case.name));

    assert!(
        output.status.success(),
        "expected simulate success for {}; stderr={}",
        case.name,
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout_value: Value =
        serde_json::from_slice(&output.stdout).expect("simulation stdout must be JSON");
    let report_path = out_dir.join("simulation_report.json");
    let report_value = read_json_path(&report_path);
    assert_eq!(
        stdout_value, report_value,
        "{} stdout must match simulation_report.json",
        case.name
    );

    let manifest = read_json_path(&out_dir.join("manifest.json"));
    assert_eq!(manifest["schema_version"], "spac.artifact-manifest.v0");
    assert_eq!(
        manifest["output_files"][0]["sha256"],
        sha256_file_hex(&report_path).expect("hash simulation report"),
        "{} manifest must hash simulation_report.json",
        case.name
    );

    report_value
}

fn golden_cases() -> Vec<GoldenCase> {
    vec![
        GoldenCase {
            name: "hft_tiny_full_lookup_rr",
            architecture_path: "examples/contracts/architecture.hft_full_lookup_rr.json",
            trace_path: "examples/contracts/trace.hft_tiny.json",
            golden_path: "examples/golden/simulation/hft_tiny.full_lookup_rr.report.json",
        },
        GoldenCase {
            name: "datacenter_incast_hash_shared_islip",
            architecture_path: "examples/contracts/architecture.datacenter_hash_shared_islip.json",
            trace_path: "examples/contracts/trace.datacenter_incast.json",
            golden_path:
                "examples/golden/simulation/datacenter_incast.hash_shared_islip.report.json",
        },
        GoldenCase {
            name: "underwater_burst_hash_nbyn_edrrm",
            architecture_path: "examples/contracts/architecture.underwater_hash_nbyn_edrrm.json",
            trace_path: "examples/contracts/trace.underwater_burst.json",
            golden_path: "examples/golden/simulation/underwater_burst.hash_nbyn_edrrm.report.json",
        },
    ]
}

struct GoldenCase {
    name: &'static str,
    architecture_path: &'static str,
    trace_path: &'static str,
    golden_path: &'static str,
}

fn read_json(relative_path: &str) -> Value {
    read_json_path(&workspace_root().join(relative_path))
}

fn read_json_path(path: &Path) -> Value {
    let text = fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
    serde_json::from_str(&text)
        .unwrap_or_else(|error| panic!("failed to parse {} as JSON: {error}", path.display()))
}

fn write_temp_directory(name: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!(
        "spac-golden-sim-{name}-{}-{}",
        std::process::id(),
        monotonic_nanos()
    ));
    fs::create_dir_all(&path).expect("create temp directory");
    path
}

fn monotonic_nanos() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time")
        .as_nanos()
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root")
        .to_path_buf()
}
