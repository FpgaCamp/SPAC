use spac_core::sha256_file_hex;
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
fn check_config_accepts_architecture_example() {
    let output = Command::new(env!("CARGO_BIN_EXE_spac"))
        .args(["check-config", "--architecture"])
        .arg(workspace_root().join("examples/contracts/architecture.hft_full_lookup_rr.json"))
        .output()
        .expect("run spac check-config");

    assert!(
        output.status.success(),
        "expected success, stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("\"status\": \"ok\""));
    assert!(stdout.contains("\"schema_version\": \"spac.architecture.v0\""));
}

#[test]
fn check_config_rejects_invalid_bus_width() {
    let path = write_temp_config(
        "invalid-architecture",
        r#"{
          "schema_version": "spac.architecture.v0",
          "name": "bad_arch",
          "ports": 8,
          "bus_width_bits": 20,
          "forwarding_table": {
            "type": "full_lookup",
            "address_width_bits": 8
          },
          "voq": {
            "type": "n_by_n",
            "depth_packets": 64
          },
          "scheduler": {
            "type": "round_robin",
            "pipeline_stages": 1
          },
          "custom_kernels": []
        }"#,
    );

    let output = Command::new(env!("CARGO_BIN_EXE_spac"))
        .args(["check-config", "--architecture"])
        .arg(path)
        .output()
        .expect("run spac check-config");

    assert!(!output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("\"status\": \"error\""));
    assert!(stdout.contains("SPAC_ARCHITECTURE_BUS_WIDTH"));
}

#[test]
fn check_constraints_accepts_constraints_example() {
    let output = Command::new(env!("CARGO_BIN_EXE_spac"))
        .args(["check-constraints", "--constraints"])
        .arg(workspace_root().join("examples/contracts/constraints.paper_aligned_u45n.json"))
        .output()
        .expect("run spac check-constraints");

    assert!(
        output.status.success(),
        "expected success, stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("\"status\": \"ok\""));
    assert!(stdout.contains("\"schema_version\": \"spac.constraints.v0\""));
}

#[test]
fn check_constraints_rejects_invalid_drop_rate() {
    let path = write_temp_config(
        "invalid-constraints",
        r#"{
          "schema_version": "spac.constraints.v0",
          "name": "bad_constraints",
          "board_target": "amd-alveo-u45n",
          "max_lut": 100000,
          "max_ff": 100000,
          "max_bram": 300,
          "max_dsp": 400,
          "target_fmax_mhz": 350,
          "max_p99_latency_ns": 1000,
          "max_packet_drop_rate": 1.5,
          "max_initiation_interval": 1
        }"#,
    );

    let output = Command::new(env!("CARGO_BIN_EXE_spac"))
        .args(["check-constraints", "--constraints"])
        .arg(path)
        .output()
        .expect("run spac check-constraints");

    assert!(!output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("\"status\": \"error\""));
    assert!(stdout.contains("SPAC_CONSTRAINTS_MAX_PACKET_DROP_RATE"));
}

#[test]
fn check_board_profile_accepts_board_profile_example() {
    let output = Command::new(env!("CARGO_BIN_EXE_spac"))
        .args(["check-board-profile", "--board-profile"])
        .arg(workspace_root().join("examples/contracts/board-profile.alveo-u45n.json"))
        .output()
        .expect("run spac check-board-profile");

    assert!(
        output.status.success(),
        "expected success, stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("\"status\": \"ok\""));
    assert!(stdout.contains("\"schema_version\": \"spac.board-profile.v0\""));
}

#[test]
fn check_board_profile_rejects_invalid_target_clock() {
    let path = write_temp_config(
        "invalid-board-profile",
        r#"{
          "schema_version": "spac.board-profile.v0",
          "board_id": "bad-board",
          "vendor": "AMD",
          "board_model": "Alveo U45N",
          "fpga_part": "xcu26-vsva1365-2LV-e",
          "toolchain": {
            "family": "Vitis HLS",
            "version": "2023.2"
          },
          "target_clock_mhz": 0,
          "host_interface": "PCIe",
          "loopback_topology": "host-fpga-host loopback",
          "report_locations": {
            "synthesis_summary": "reports/post_synthesis.json",
            "timing_summary": "reports/timing_summary.rpt"
          }
        }"#,
    );

    let output = Command::new(env!("CARGO_BIN_EXE_spac"))
        .args(["check-board-profile", "--board-profile"])
        .arg(path)
        .output()
        .expect("run spac check-board-profile");

    assert!(!output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("\"status\": \"error\""));
    assert!(stdout.contains("SPAC_BOARD_PROFILE_TARGET_CLOCK"));
}

#[test]
fn parse_hw_report_writes_report_and_manifest() {
    let out_dir = write_temp_directory("parse-hw-report");
    let output = Command::new(env!("CARGO_BIN_EXE_spac"))
        .args(["parse-hw-report", "--tool", "vitis", "--report"])
        .arg(workspace_root().join("examples/contracts/hw-report.vitis_hls_summary.rpt"))
        .args(["--board-profile"])
        .arg(workspace_root().join("examples/contracts/board-profile.alveo-u45n.json"))
        .args(["--out"])
        .arg(&out_dir)
        .output()
        .expect("run spac parse-hw-report");

    assert!(
        output.status.success(),
        "expected success, stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let report_path = out_dir.join("hw_report.json");
    let manifest_path = out_dir.join("manifest.json");
    assert!(report_path.exists(), "hw_report.json must exist");
    assert!(manifest_path.exists(), "manifest.json must exist");

    let stdout_value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("hardware report stdout JSON");
    let report_text = fs::read_to_string(&report_path).expect("read hardware report");
    let report_value: serde_json::Value =
        serde_json::from_str(&report_text).expect("hardware report JSON");
    assert_eq!(stdout_value, report_value);
    assert_eq!(report_value["schema_version"], "spac.hw-report.v0");
    assert_eq!(report_value["trust_level"], "post_synthesis");
    assert_eq!(report_value["tool"], "vitis");
    assert_eq!(report_value["board_profile_id"], "amd-alveo-u45n");
    assert_eq!(report_value["metrics"]["lut"], 48029);
    assert_eq!(report_value["metrics"]["fmax_mhz"], 350.0);
    assert!(report_value["limitations"][1]
        .as_str()
        .expect("limitation")
        .contains("does not reproduce SPAC paper metrics"));

    let manifest_text = fs::read_to_string(&manifest_path).expect("read manifest");
    let manifest_value: serde_json::Value =
        serde_json::from_str(&manifest_text).expect("manifest JSON");
    assert_eq!(
        manifest_value["schema_version"],
        "spac.artifact-manifest.v0"
    );
    assert_eq!(
        manifest_value["run_id"],
        "parse-hw-report-vitis-amd-alveo-u45n"
    );
    assert_eq!(
        manifest_value["output_files"][0]["sha256"],
        sha256_file_hex(&report_path).expect("hash hardware report")
    );
}

#[test]
fn parse_hw_report_rejects_unknown_tool() {
    let out_dir = write_temp_directory("parse-hw-report-unknown");
    let output = Command::new(env!("CARGO_BIN_EXE_spac"))
        .args(["parse-hw-report", "--tool", "quartus", "--report"])
        .arg(workspace_root().join("examples/contracts/hw-report.vitis_hls_summary.rpt"))
        .args(["--board-profile"])
        .arg(workspace_root().join("examples/contracts/board-profile.alveo-u45n.json"))
        .args(["--out"])
        .arg(&out_dir)
        .output()
        .expect("run spac parse-hw-report with unsupported tool");

    assert!(!output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("\"status\": \"error\""));
    assert!(stdout.contains("SPAC_HW_REPORT_TOOL"));
}

#[test]
fn accept_hw_report_writes_acceptance_and_manifest() {
    let parsed_dir = write_temp_directory("accept-hw-report-parse");
    let parse_output = Command::new(env!("CARGO_BIN_EXE_spac"))
        .args(["parse-hw-report", "--tool", "vitis", "--report"])
        .arg(workspace_root().join("examples/contracts/hw-report.vitis_hls_summary.rpt"))
        .args(["--board-profile"])
        .arg(workspace_root().join("examples/contracts/board-profile.alveo-u45n.json"))
        .args(["--out"])
        .arg(&parsed_dir)
        .output()
        .expect("run spac parse-hw-report");
    assert!(
        parse_output.status.success(),
        "parse must succeed, stderr={}",
        String::from_utf8_lossy(&parse_output.stderr)
    );

    let constraints_path = write_lenient_hw_constraints("acceptance_pass");
    let out_dir = write_temp_directory("accept-hw-report");
    let output = Command::new(env!("CARGO_BIN_EXE_spac"))
        .args(["accept-hw-report", "--report"])
        .arg(parsed_dir.join("hw_report.json"))
        .args(["--constraints"])
        .arg(&constraints_path)
        .args(["--out"])
        .arg(&out_dir)
        .output()
        .expect("run spac accept-hw-report");

    assert!(
        output.status.success(),
        "expected acceptance pass, stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let acceptance_path = out_dir.join("hw_acceptance.json");
    let manifest_path = out_dir.join("manifest.json");
    assert!(acceptance_path.exists(), "hw_acceptance.json must exist");
    assert!(manifest_path.exists(), "manifest.json must exist");

    let stdout_value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("acceptance stdout JSON");
    let acceptance_text = fs::read_to_string(&acceptance_path).expect("read acceptance report");
    let acceptance_value: serde_json::Value =
        serde_json::from_str(&acceptance_text).expect("acceptance report JSON");
    assert_eq!(stdout_value, acceptance_value);
    assert_eq!(acceptance_value["schema_version"], "spac.hw-acceptance.v0");
    assert_eq!(acceptance_value["trust_level"], "post_synthesis");
    assert_eq!(acceptance_value["status"], "pass");
    assert!(acceptance_value["checks"]
        .as_array()
        .expect("checks")
        .iter()
        .any(|check| check["metric"] == "packet_drop_rate" && check["status"] == "not_evaluated"));

    let manifest_text = fs::read_to_string(&manifest_path).expect("read manifest");
    let manifest_value: serde_json::Value =
        serde_json::from_str(&manifest_text).expect("manifest JSON");
    assert_eq!(
        manifest_value["schema_version"],
        "spac.artifact-manifest.v0"
    );
    assert_eq!(
        manifest_value["output_files"][0]["sha256"],
        sha256_file_hex(&acceptance_path).expect("hash acceptance report")
    );
}

#[test]
fn accept_hw_report_returns_failure_exit_for_constraint_violation() {
    let parsed_dir = write_temp_directory("accept-hw-report-fail-parse");
    let parse_output = Command::new(env!("CARGO_BIN_EXE_spac"))
        .args(["parse-hw-report", "--tool", "vitis", "--report"])
        .arg(workspace_root().join("examples/contracts/hw-report.vitis_hls_summary.rpt"))
        .args(["--board-profile"])
        .arg(workspace_root().join("examples/contracts/board-profile.alveo-u45n.json"))
        .args(["--out"])
        .arg(&parsed_dir)
        .output()
        .expect("run spac parse-hw-report");
    assert!(parse_output.status.success());

    let out_dir = write_temp_directory("accept-hw-report-fail");
    let output = Command::new(env!("CARGO_BIN_EXE_spac"))
        .args(["accept-hw-report", "--report"])
        .arg(parsed_dir.join("hw_report.json"))
        .args(["--constraints"])
        .arg(workspace_root().join("examples/contracts/constraints.paper_aligned_u45n.json"))
        .args(["--out"])
        .arg(&out_dir)
        .output()
        .expect("run spac accept-hw-report");

    assert_eq!(output.status.code(), Some(2));
    let acceptance_value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("acceptance failure stdout JSON");
    assert_eq!(acceptance_value["status"], "fail");
    assert!(acceptance_value["checks"]
        .as_array()
        .expect("checks")
        .iter()
        .any(|check| check["metric"] == "bram" && check["status"] == "fail"));
}

#[test]
fn accept_hw_report_returns_inconclusive_exit_for_missing_metric() {
    let report_path = write_temp_config(
        "hw-report-missing-bram",
        r#"{
          "schema_version": "spac.hw-report.v0",
          "trust_level": "post_synthesis",
          "tool": "vitis",
          "board_profile_id": "amd-alveo-u45n",
          "board_model": "Alveo U45N",
          "fpga_part": "xcu26-vsva1365-2LV-e",
          "toolchain_family": "Vitis HLS",
          "toolchain_version": "2023.2",
          "source_report_path": "synthetic.rpt",
          "metrics": {
            "lut": 48029,
            "ff": 61264,
            "bram": null,
            "dsp": 0,
            "fmax_mhz": 350.0,
            "initiation_interval": 1,
            "latency_cycles_min": null,
            "latency_cycles_max": null,
            "throughput_gbps": null
          },
          "warnings": ["fixture"],
          "limitations": ["fixture"]
        }"#,
    );
    let constraints_path = write_lenient_hw_constraints("acceptance_inconclusive");
    let out_dir = write_temp_directory("accept-hw-report-inconclusive");

    let output = Command::new(env!("CARGO_BIN_EXE_spac"))
        .args(["accept-hw-report", "--report"])
        .arg(report_path)
        .args(["--constraints"])
        .arg(constraints_path)
        .args(["--out"])
        .arg(&out_dir)
        .output()
        .expect("run spac accept-hw-report");

    assert_eq!(output.status.code(), Some(3));
    let acceptance_value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("acceptance inconclusive stdout JSON");
    assert_eq!(acceptance_value["status"], "inconclusive");
    assert!(acceptance_value["checks"]
        .as_array()
        .expect("checks")
        .iter()
        .any(|check| check["metric"] == "bram" && check["status"] == "inconclusive"));
}

#[test]
fn package_experiment_writes_run_bundle_and_manifest() {
    let run_dir = write_temp_directory("package-experiment-run");
    let evidence_report = run_dir.join("hw_report.json");
    let evidence_acceptance = run_dir.join("hw_acceptance.json");
    fs::write(
        &evidence_report,
        br#"{"schema_version":"spac.hw-report.v0"}"#,
    )
    .expect("write report evidence");
    fs::write(
        &evidence_acceptance,
        br#"{"schema_version":"spac.hw-acceptance.v0"}"#,
    )
    .expect("write acceptance evidence");

    let out_dir = write_temp_directory("package-experiment-out");
    let output = Command::new(env!("CARGO_BIN_EXE_spac"))
        .args(["package-experiment", "--run-dir"])
        .arg(&run_dir)
        .args(["--board-profile"])
        .arg(workspace_root().join("examples/contracts/board-profile.alveo-u45n.json"))
        .args(["--trust-level", "post_synthesis", "--out"])
        .arg(&out_dir)
        .output()
        .expect("run spac package-experiment");

    assert!(
        output.status.success(),
        "expected experiment package success, stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let experiment_path = out_dir.join("experiment_run.json");
    let manifest_path = out_dir.join("manifest.json");
    assert!(experiment_path.exists(), "experiment_run.json must exist");
    assert!(manifest_path.exists(), "manifest.json must exist");

    let stdout_value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("experiment run stdout JSON");
    let experiment_text = fs::read_to_string(&experiment_path).expect("read experiment run");
    let experiment_value: serde_json::Value =
        serde_json::from_str(&experiment_text).expect("experiment run JSON");
    assert_eq!(stdout_value, experiment_value);
    assert_eq!(experiment_value["schema_version"], "spac.experiment-run.v0");
    assert_eq!(experiment_value["stage"], "E6");
    assert_eq!(experiment_value["trust_level"], "post_synthesis");
    assert_eq!(experiment_value["board_profile_id"], "amd-alveo-u45n");
    assert!(experiment_value["known_limitations"]
        .as_array()
        .expect("limitations")
        .iter()
        .any(|limitation| limitation
            .as_str()
            .expect("limitation text")
            .contains("does not reproduce SPAC paper metrics")));
    assert_eq!(
        experiment_value["output_files"]
            .as_array()
            .expect("output files")
            .len(),
        2
    );

    let manifest_text = fs::read_to_string(&manifest_path).expect("read manifest");
    let manifest_value: serde_json::Value =
        serde_json::from_str(&manifest_text).expect("manifest JSON");
    assert_eq!(
        manifest_value["schema_version"],
        "spac.artifact-manifest.v0"
    );
    assert_eq!(
        manifest_value["output_files"][0]["sha256"],
        sha256_file_hex(&experiment_path).expect("hash experiment run")
    );
}

#[test]
fn package_experiment_rejects_unknown_trust_level() {
    let run_dir = write_temp_directory("package-experiment-bad-trust");
    fs::write(run_dir.join("evidence.json"), "{}").expect("write evidence");
    let out_dir = write_temp_directory("package-experiment-bad-trust-out");

    let output = Command::new(env!("CARGO_BIN_EXE_spac"))
        .args(["package-experiment", "--run-dir"])
        .arg(&run_dir)
        .args(["--board-profile"])
        .arg(workspace_root().join("examples/contracts/board-profile.alveo-u45n.json"))
        .args(["--trust-level", "paper_reproduced", "--out"])
        .arg(&out_dir)
        .output()
        .expect("run spac package-experiment");

    assert!(!output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("\"status\": \"error\""));
    assert!(stdout.contains("SPAC_EXPERIMENT_TRUST_LEVEL"));
}

#[test]
fn package_experiment_rejects_missing_run_dir() {
    let out_dir = write_temp_directory("package-experiment-missing-run-out");
    let missing = std::env::temp_dir().join(format!(
        "spac-missing-run-dir-{}-{}",
        std::process::id(),
        monotonic_nanos()
    ));

    let output = Command::new(env!("CARGO_BIN_EXE_spac"))
        .args(["package-experiment", "--run-dir"])
        .arg(missing)
        .args(["--board-profile"])
        .arg(workspace_root().join("examples/contracts/board-profile.alveo-u45n.json"))
        .args(["--trust-level", "post_synthesis", "--out"])
        .arg(&out_dir)
        .output()
        .expect("run spac package-experiment");

    assert!(!output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("SPAC_EXPERIMENT_RUN_DIR_READ"));
}

#[test]
fn check_trace_accepts_trace_example() {
    let output = Command::new(env!("CARGO_BIN_EXE_spac"))
        .args(["check-trace", "--trace"])
        .arg(workspace_root().join("examples/contracts/trace.hft_tiny.json"))
        .output()
        .expect("run spac check-trace");

    assert!(
        output.status.success(),
        "expected success, stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("\"status\": \"ok\""));
    assert!(stdout.contains("\"schema_version\": \"spac.trace.v0\""));
}

#[test]
fn check_trace_rejects_nonmonotonic_timestamp() {
    let path = write_temp_config(
        "invalid-trace",
        r#"{
          "schema_version": "spac.trace.v0",
          "name": "bad_trace",
          "workload_class": "hft",
          "time_unit": "ns",
          "packets": [
            {
              "timestamp_ns": 40,
              "ingress_port": 0,
              "src": 1,
              "dst": 2,
              "payload_bytes": 24,
              "flow_id": "flow_0"
            },
            {
              "timestamp_ns": 0,
              "ingress_port": 1,
              "src": 2,
              "dst": 3,
              "payload_bytes": 24,
              "flow_id": "flow_1"
            }
          ]
        }"#,
    );

    let output = Command::new(env!("CARGO_BIN_EXE_spac"))
        .args(["check-trace", "--trace"])
        .arg(path)
        .output()
        .expect("run spac check-trace");

    assert!(!output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("\"status\": \"error\""));
    assert!(stdout.contains("SPAC_TRACE_TIMESTAMP_ORDER"));
}

#[test]
fn import_spac_ae_trace_writes_normalized_trace_and_manifest() {
    let out_dir = write_temp_directory("import-spac-ae-trace");
    let output = Command::new(env!("CARGO_BIN_EXE_spac"))
        .args(["import-spac-ae-trace", "--trace"])
        .arg(workspace_root().join("examples/contracts/spac-ae.hft_trace_sample.csv"))
        .args(["--topology"])
        .arg(workspace_root().join("examples/contracts/spac-ae.dse_8nodes_sample.csv"))
        .args([
            "--name",
            "spac_ae_hft_sample",
            "--workload-class",
            "hft",
            "--ports",
            "8",
            "--out",
        ])
        .arg(&out_dir)
        .output()
        .expect("run spac import-spac-ae-trace");

    assert!(
        output.status.success(),
        "expected success, stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let trace_path = out_dir.join("trace.json");
    let manifest_path = out_dir.join("manifest.json");
    assert!(trace_path.exists(), "trace.json must exist");
    assert!(manifest_path.exists(), "manifest.json must exist");

    let stdout_value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("trace stdout JSON");
    let trace_text = fs::read_to_string(&trace_path).expect("read trace");
    let trace_value: serde_json::Value = serde_json::from_str(&trace_text).expect("trace JSON");
    assert_eq!(stdout_value, trace_value);
    assert_eq!(trace_value["schema_version"], "spac.trace.v0");
    assert_eq!(trace_value["name"], "spac_ae_hft_sample");
    assert_eq!(trace_value["packets"][0]["timestamp_ns"], 1549);
    assert_eq!(trace_value["packets"][0]["payload_bytes"], 59);
    assert_eq!(trace_value["packets"][1]["ingress_port"], 5);

    let manifest_text = fs::read_to_string(&manifest_path).expect("read manifest");
    let manifest_value: serde_json::Value =
        serde_json::from_str(&manifest_text).expect("manifest JSON");
    assert_eq!(
        manifest_value["output_files"][0]["sha256"],
        sha256_file_hex(&trace_path).expect("hash imported trace")
    );
}

#[test]
fn generate_spac_ae_dse_space_writes_candidates_and_manifest() {
    let out_dir = write_temp_directory("generate-spac-ae-dse-space");
    let output = Command::new(env!("CARGO_BIN_EXE_spac"))
        .args([
            "generate-spac-ae-dse-space",
            "--ports",
            "8",
            "--name",
            "spac_ae_8p",
            "--out",
        ])
        .arg(&out_dir)
        .output()
        .expect("run spac generate-spac-ae-dse-space");

    assert!(
        output.status.success(),
        "expected success, stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let space_path = out_dir.join("dse_space.json");
    let manifest_path = out_dir.join("manifest.json");
    assert!(space_path.exists(), "dse_space.json must exist");
    assert!(manifest_path.exists(), "manifest.json must exist");

    let space_value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("space stdout JSON");
    assert_eq!(space_value["schema_version"], "spac.dse-space.v0");
    assert_eq!(
        space_value["candidates"]
            .as_array()
            .expect("DSE candidates")
            .len(),
        48
    );
    assert!(space_value["candidates"]
        .as_array()
        .expect("DSE candidates")
        .iter()
        .any(|candidate| candidate["architecture"]["bus_width_bits"] == 640));

    let manifest_text = fs::read_to_string(&manifest_path).expect("read manifest");
    let manifest_value: serde_json::Value =
        serde_json::from_str(&manifest_text).expect("manifest JSON");
    assert_eq!(
        manifest_value["output_files"][0]["sha256"],
        sha256_file_hex(&space_path).expect("hash generated DSE space")
    );
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

#[test]
fn simulate_writes_report_and_manifest() {
    let out_dir = write_temp_directory("simulate");
    let output = Command::new(env!("CARGO_BIN_EXE_spac"))
        .args(["simulate", "--architecture"])
        .arg(workspace_root().join("examples/contracts/architecture.hft_full_lookup_rr.json"))
        .args(["--trace"])
        .arg(workspace_root().join("examples/contracts/trace.hft_tiny.json"))
        .args(["--out"])
        .arg(&out_dir)
        .output()
        .expect("run spac simulate");

    assert!(
        output.status.success(),
        "expected success, stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let report_path = out_dir.join("simulation_report.json");
    let manifest_path = out_dir.join("manifest.json");
    assert!(report_path.exists(), "simulation_report.json must exist");
    assert!(manifest_path.exists(), "manifest.json must exist");

    let report_text = fs::read_to_string(&report_path).expect("read simulation report");
    let report_value: serde_json::Value =
        serde_json::from_str(&report_text).expect("simulation report JSON");
    assert_eq!(report_value["schema_version"], "spac.simulation-run.v0");
    assert_eq!(report_value["trust_level"], "software_model");
    assert_eq!(report_value["metrics"]["packets_forwarded"], 3);
    assert_eq!(report_value["metrics"]["packets_dropped"], 0);
    assert_eq!(report_value["supported_model"]["scheduler"], "round_robin");

    let manifest_text = fs::read_to_string(&manifest_path).expect("read manifest");
    let manifest_value: serde_json::Value =
        serde_json::from_str(&manifest_text).expect("manifest JSON");
    assert_eq!(
        manifest_value["schema_version"], "spac.artifact-manifest.v0",
        "manifest schema version"
    );
    assert_eq!(
        manifest_value["run_id"],
        "simulate-hft_8p_full_lookup_rr-hft_tiny"
    );
    assert_eq!(
        manifest_value["output_files"][0]["sha256"],
        sha256_file_hex(&report_path).expect("hash simulation report")
    );
}

#[test]
fn dse_writes_result_and_manifest() {
    let out_dir = write_temp_directory("dse");
    let output = Command::new(env!("CARGO_BIN_EXE_spac"))
        .args(["dse", "--space"])
        .arg(workspace_root().join("examples/contracts/dse-space.tiny.json"))
        .args(["--trace"])
        .arg(workspace_root().join("examples/contracts/trace.dse_tiny_burst.json"))
        .args(["--constraints"])
        .arg(workspace_root().join("examples/contracts/constraints.dse_tiny_lenient.json"))
        .args(["--out"])
        .arg(&out_dir)
        .output()
        .expect("run spac dse");

    assert!(
        output.status.success(),
        "expected success, stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let result_path = out_dir.join("dse_result.json");
    let manifest_path = out_dir.join("manifest.json");
    assert!(result_path.exists(), "dse_result.json must exist");
    assert!(manifest_path.exists(), "manifest.json must exist");

    let stdout_value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("DSE stdout JSON");
    let result_text = fs::read_to_string(&result_path).expect("read DSE result");
    let result_value: serde_json::Value =
        serde_json::from_str(&result_text).expect("DSE result JSON");

    assert_eq!(stdout_value, result_value);
    assert_eq!(result_value["schema_version"], "spac.dse-result.v0");
    assert_eq!(result_value["trust_level"], "software_model");
    assert_eq!(
        result_value["frontier"],
        serde_json::json!(["minimal_nbyn", "balanced_nbyn", "deep_nbyn"])
    );
    assert_eq!(result_value["candidates"][3]["name"], "wasteful_shared");
    assert_eq!(result_value["candidates"][3]["status"], "dominated");
    assert!(
        result_value["warnings"][0]
            .as_str()
            .expect("warning string")
            .contains("no HLS synthesis"),
        "DSE result must keep evidence limitations explicit"
    );

    let manifest_text = fs::read_to_string(&manifest_path).expect("read manifest");
    let manifest_value: serde_json::Value =
        serde_json::from_str(&manifest_text).expect("manifest JSON");
    assert_eq!(
        manifest_value["schema_version"],
        "spac.artifact-manifest.v0"
    );
    assert_eq!(
        manifest_value["run_id"],
        "dse-tiny_dse_space-dse_tiny_burst-dse_tiny_lenient"
    );
    assert_eq!(
        manifest_value["output_files"][0]["sha256"],
        sha256_file_hex(&result_path).expect("hash DSE result")
    );
}

#[test]
fn dse_phase2_writes_buffer_optimization_metadata() {
    let out_dir = write_temp_directory("dse-phase2");
    let output = Command::new(env!("CARGO_BIN_EXE_spac"))
        .args(["dse", "--space"])
        .arg(workspace_root().join("examples/contracts/dse-space.tiny.json"))
        .args(["--trace"])
        .arg(workspace_root().join("examples/contracts/trace.dse_tiny_burst.json"))
        .args(["--constraints"])
        .arg(workspace_root().join("examples/contracts/constraints.dse_tiny_lenient.json"))
        .args(["--out"])
        .arg(&out_dir)
        .args([
            "--spac-ae-phase2-buffers",
            "--phase2-top-n",
            "1",
            "--min-voq-depth-packets",
            "1",
            "--phase2-max-drop-rate",
            "1.0",
        ])
        .output()
        .expect("run spac dse phase2");

    assert!(
        output.status.success(),
        "expected success, stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let result_path = out_dir.join("dse_result.json");
    let result_text = fs::read_to_string(&result_path).expect("read DSE result");
    let result_value: serde_json::Value =
        serde_json::from_str(&result_text).expect("DSE result JSON");
    let candidates = result_value["candidates"]
        .as_array()
        .expect("DSE candidates");
    let phase2 = candidates
        .iter()
        .find(|candidate| candidate["phase"] == 2)
        .expect("phase2 candidate");

    assert_eq!(phase2["optimized_from"], "deep_nbyn");
    assert_eq!(
        phase2["buffer_optimization"]["min_depth_packets"],
        serde_json::json!(1)
    );
    assert!(result_value["warnings"][1]
        .as_str()
        .expect("phase2 warning")
        .contains("phase-2 buffer optimization"));

    let manifest_text = fs::read_to_string(out_dir.join("manifest.json")).expect("read manifest");
    let manifest_value: serde_json::Value =
        serde_json::from_str(&manifest_text).expect("manifest JSON");
    assert_eq!(
        manifest_value["run_id"],
        "dse-phase2-tiny_dse_space-dse_tiny_burst-dse_tiny_lenient"
    );
}

#[test]
fn generate_metadata_writes_metadata_and_manifest() {
    let out_dir = write_temp_directory("generate-metadata");
    let output = Command::new(env!("CARGO_BIN_EXE_spac"))
        .args(["generate-metadata", "--protocol"])
        .arg(workspace_root().join("examples/protocols/basic.spac"))
        .args(["--bus-width", "8", "--out"])
        .arg(&out_dir)
        .output()
        .expect("run spac generate-metadata");

    assert!(
        output.status.success(),
        "expected success, stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let metadata_path = out_dir.join("metadata.json");
    let manifest_path = out_dir.join("manifest.json");
    assert!(metadata_path.exists(), "metadata.json must exist");
    assert!(manifest_path.exists(), "manifest.json must exist");

    let metadata_text = fs::read_to_string(&metadata_path).expect("read metadata");
    assert!(metadata_text.contains("\"schema_version\": \"spac.metadata.v0\""));

    let first_manifest = fs::read_to_string(&manifest_path).expect("read first manifest");
    let manifest_value: serde_json::Value =
        serde_json::from_str(&first_manifest).expect("manifest JSON");
    assert_eq!(
        manifest_value["schema_version"], "spac.artifact-manifest.v0",
        "manifest schema version"
    );
    assert_eq!(manifest_value["tool_name"], "spac");
    assert_eq!(manifest_value["run_id"], "generate-metadata-basic-bus8");

    let output_files = manifest_value["output_files"]
        .as_array()
        .expect("output_files array");
    assert_eq!(output_files.len(), 1);
    assert_eq!(
        output_files[0]["sha256"],
        sha256_file_hex(&metadata_path).expect("hash metadata")
    );

    let rerun = Command::new(env!("CARGO_BIN_EXE_spac"))
        .args(["generate-metadata", "--protocol"])
        .arg(workspace_root().join("examples/protocols/basic.spac"))
        .args(["--bus-width", "8", "--out"])
        .arg(&out_dir)
        .output()
        .expect("rerun spac generate-metadata");
    assert!(
        rerun.status.success(),
        "expected rerun success, stderr={}",
        String::from_utf8_lossy(&rerun.stderr)
    );

    let second_manifest = fs::read_to_string(&manifest_path).expect("read second manifest");
    assert_eq!(
        first_manifest, second_manifest,
        "manifest must be deterministic"
    );
}

#[test]
fn generate_hls_traits_writes_packet_header_and_manifest() {
    let out_dir = write_temp_directory("generate-hls-traits");
    let output = Command::new(env!("CARGO_BIN_EXE_spac"))
        .args(["generate-hls-traits", "--metadata"])
        .arg(workspace_root().join("examples/golden/metadata/hft.bus64.metadata.json"))
        .args(["--out"])
        .arg(&out_dir)
        .output()
        .expect("run spac generate-hls-traits");

    assert!(
        output.status.success(),
        "expected success, stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let header_path = out_dir.join("packet.hpp");
    let manifest_path = out_dir.join("manifest.json");
    assert!(header_path.exists(), "packet.hpp must exist");
    assert!(manifest_path.exists(), "manifest.json must exist");

    let stdout_value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("HLS report JSON");
    assert_eq!(stdout_value["schema_version"], "spac.hls-traits-run.v0");
    assert_eq!(stdout_value["protocol_name"], "hft");
    assert_eq!(stdout_value["trust_level"], "software_model");

    let header_text = fs::read_to_string(&header_path).expect("read generated header");
    assert!(header_text.contains("// Generated by SPAC. Do not edit by hand."));
    assert!(header_text.contains("using packet_word_t = ap_uint<BUS_WIDTH_BITS>;"));
    assert!(header_text.contains("inline constexpr const char* ROUTING_KEY_FIELD = \"venue\";"));
    assert!(header_text.contains("{\"price\", \"price_ticks\", 41, 32, 5, 0, true},"));
    assert!(header_text.contains("no HLS csim, synthesis, timing closure, or FPGA measurement"));

    let manifest_text = fs::read_to_string(&manifest_path).expect("read manifest");
    let manifest_value: serde_json::Value =
        serde_json::from_str(&manifest_text).expect("manifest JSON");
    assert_eq!(
        manifest_value["schema_version"],
        "spac.artifact-manifest.v0"
    );
    assert_eq!(manifest_value["run_id"], "generate-hls-traits-hft");
    assert_eq!(
        manifest_value["output_files"][0]["sha256"],
        sha256_file_hex(&header_path).expect("hash generated header")
    );
}

#[test]
fn package_hls_csim_writes_generated_smoke_package_and_manifest() {
    let out_dir = write_temp_directory("package-hls-csim");
    let output = Command::new(env!("CARGO_BIN_EXE_spac"))
        .args(["package-hls-csim", "--metadata"])
        .arg(workspace_root().join("examples/golden/metadata/hft.bus64.metadata.json"))
        .args(["--board-profile"])
        .arg(workspace_root().join("examples/contracts/board-profile.alveo-u45n.json"))
        .args(["--out"])
        .arg(&out_dir)
        .output()
        .expect("run spac package-hls-csim");

    assert!(
        output.status.success(),
        "expected success, stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let header_path = out_dir.join("packet.hpp");
    let smoke_path = out_dir.join("csim_smoke.cpp");
    let config_path = out_dir.join("hls_config.cfg");
    let tcl_path = out_dir.join("run_csim.tcl");
    let report_path = out_dir.join("hls_csim_run.json");
    let manifest_path = out_dir.join("manifest.json");
    for path in [
        &header_path,
        &smoke_path,
        &config_path,
        &tcl_path,
        &report_path,
        &manifest_path,
    ] {
        assert!(path.exists(), "{} must exist", path.display());
    }

    let stdout_value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("HLS csim report JSON");
    let report_value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&report_path).expect("read csim report"))
            .expect("csim report JSON");
    assert_eq!(stdout_value, report_value);
    assert_eq!(report_value["schema_version"], "spac.hls-csim-run.v0");
    assert_eq!(report_value["protocol_name"], "hft");
    assert_eq!(report_value["board_profile_id"], "amd-alveo-u45n");
    assert_eq!(report_value["trust_level"], "software_model");
    assert_eq!(report_value["status"], "blocked");
    assert_eq!(
        report_value["diagnostics"][0]["code"],
        "SPAC_HLS_CSIM_NOT_EXECUTED"
    );

    let smoke_text = fs::read_to_string(&smoke_path).expect("read smoke source");
    assert!(smoke_text.contains("#include \"packet.hpp\""));
    assert!(smoke_text.contains("static_assert(spac_generated::FIELD_COUNT"));
    assert!(smoke_text.contains("extern \"C\" void spac_csim_smoke"));

    let config_text = fs::read_to_string(&config_path).expect("read hls config");
    assert!(config_text.contains("part=xcu26-vsva1365-2LV-e"));
    assert!(config_text.contains("syn.top=spac_csim_smoke"));
    assert!(config_text.contains("clock=350MHz"));

    let tcl_text = fs::read_to_string(&tcl_path).expect("read csim tcl");
    assert!(tcl_text.contains("set_top spac_csim_smoke"));
    assert!(tcl_text.contains("csim_design"));

    let manifest_text = fs::read_to_string(&manifest_path).expect("read manifest");
    let manifest_value: serde_json::Value =
        serde_json::from_str(&manifest_text).expect("manifest JSON");
    assert_eq!(
        manifest_value["schema_version"],
        "spac.artifact-manifest.v0"
    );
    assert_eq!(
        manifest_value["run_id"],
        "package-hls-csim-hft-amd-alveo-u45n"
    );
    let output_files = manifest_value["output_files"]
        .as_array()
        .expect("manifest output files");
    assert_eq!(output_files.len(), 5);
    assert!(output_files
        .iter()
        .any(|artifact| artifact["sha256"]
            == sha256_file_hex(&report_path).expect("hash csim report")));
}

#[test]
fn package_hls_csim_execute_without_tool_reports_blocked_not_hls_csim() {
    let out_dir = write_temp_directory("package-hls-csim-blocked");
    let missing_tool = std::env::temp_dir().join(format!(
        "spac-missing-vitis-hls-{}-{}",
        std::process::id(),
        monotonic_nanos()
    ));
    let output = Command::new(env!("CARGO_BIN_EXE_spac"))
        .args(["package-hls-csim", "--metadata"])
        .arg(workspace_root().join("examples/golden/metadata/hft.bus64.metadata.json"))
        .args(["--board-profile"])
        .arg(workspace_root().join("examples/contracts/board-profile.alveo-u45n.json"))
        .args(["--out"])
        .arg(&out_dir)
        .args(["--execute", "--vitis-hls-bin"])
        .arg(&missing_tool)
        .output()
        .expect("run spac package-hls-csim --execute");

    assert!(
        output.status.success(),
        "expected blocked report success, stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );

    let report_value: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("HLS csim blocked report JSON");
    assert_eq!(report_value["schema_version"], "spac.hls-csim-run.v0");
    assert_eq!(report_value["status"], "blocked");
    assert_eq!(report_value["trust_level"], "software_model");
    assert_eq!(
        report_value["diagnostics"][0]["code"],
        "SPAC_HLS_CSIM_TOOL_NOT_FOUND"
    );
    assert_eq!(report_value["tool"]["exit_code"], serde_json::Value::Null);
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

fn write_temp_directory(name: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!(
        "spac-{name}-{}-{}",
        std::process::id(),
        monotonic_nanos()
    ));
    fs::create_dir_all(&path).expect("create temp directory");
    path
}

fn write_lenient_hw_constraints(name: &str) -> PathBuf {
    write_temp_config(
        name,
        r#"{
          "schema_version": "spac.constraints.v0",
          "name": "lenient_hw_constraints",
          "board_target": "amd-alveo-u45n",
          "max_lut": 100000,
          "max_ff": 100000,
          "max_bram": 4000,
          "max_dsp": 400,
          "target_fmax_mhz": 350,
          "max_p99_latency_ns": 1000,
          "max_packet_drop_rate": 0.000001,
          "max_initiation_interval": 1
        }"#,
    )
}

fn monotonic_nanos() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time")
        .as_nanos()
}
