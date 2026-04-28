use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

#[test]
fn public_engineering_docs_exist() {
    for relative_path in [
        "docs-public/engineering-overview.md",
        "docs-public/fpga-validation.md",
        "docs-public/fpga-first-light.md",
        "docs-public/spac-ae-comparison.md",
        "docs-public/maturity-todo.md",
        "configs/schemas/architecture.schema.json",
        "configs/schemas/constraints.schema.json",
        "configs/schemas/trace.schema.json",
        "configs/schemas/simulation-run.schema.json",
        "configs/schemas/dse-space.schema.json",
        "configs/schemas/dse-result.schema.json",
        "configs/schemas/hls-traits-run.schema.json",
        "configs/schemas/hls-csim-run.schema.json",
        "configs/schemas/hw-report.schema.json",
        "configs/schemas/hw-acceptance.schema.json",
        "configs/schemas/board-profile.schema.json",
        "configs/schemas/experiment-run.schema.json",
    ] {
        assert!(
            workspace_root().join(relative_path).exists(),
            "missing tracked public surface {relative_path}"
        );
    }
}

#[test]
fn readme_references_public_engineering_docs() {
    let readme = fs::read_to_string(workspace_root().join("README.md")).expect("read README");
    assert!(readme.contains("docs-public/engineering-overview.md"));
    assert!(readme.contains("docs-public/fpga-validation.md"));
    assert!(readme.contains("docs-public/fpga-first-light.md"));
    assert!(readme.contains("docs-public/maturity-todo.md"));
    assert!(readme.contains("Active development notice"));
    assert!(readme.contains("https://github.com/FpgaCamp/SPAC/issues"));
    assert!(readme.contains("docs-public/spac-ae-comparison.md"));
    assert!(readme.contains("spac.architecture.v0"));
    assert!(readme.contains("spac.board-profile.v0"));
    assert!(readme.contains("spac.hw-report.v0"));
    assert!(readme.contains("spac.hw-acceptance.v0"));
    assert!(readme.contains("spac check-config --architecture <path>"));
    assert!(readme.contains("spac check-constraints --constraints <path>"));
    assert!(readme.contains("spac check-board-profile --board-profile <path>"));
    assert!(readme.contains("spac check-trace --trace <path>"));
    assert!(readme.contains("spac import-spac-ae-trace --trace <trace.csv>"));
    assert!(
        readme.contains("spac generate-metadata --protocol <path> --bus-width <bits> --out <dir>")
    );
    assert!(readme.contains("spac generate-hls-traits --metadata <path> --out <dir>"));
    assert!(readme.contains("spac package-hls-csim --metadata <path>"));
    assert!(readme.contains("spac parse-hw-report --tool <tool>"));
    assert!(readme.contains("spac accept-hw-report --report <hw_report.json>"));
    assert!(readme.contains("spac package-experiment --run-dir <dir>"));
    assert!(readme.contains("spac generate-spac-ae-dse-space --ports <n>"));
    assert!(readme.contains("spac simulate --architecture <path> --trace <path> --out <dir>"));
    assert!(
        readme.contains("spac dse --space <path> --trace <path> --constraints <path> --out <dir>")
    );
}

#[test]
fn public_contract_examples_are_internally_consistent() {
    let architecture = read_json("examples/contracts/architecture.hft_full_lookup_rr.json");
    assert_eq!(architecture["schema_version"], "spac.architecture.v0");
    assert_eq!(architecture["forwarding_table"]["type"], "full_lookup");
    assert_eq!(architecture["voq"]["type"], "n_by_n");
    assert_eq!(architecture["scheduler"]["type"], "round_robin");

    let constraints = read_json("examples/contracts/constraints.paper_aligned_u45n.json");
    assert_eq!(constraints["schema_version"], "spac.constraints.v0");
    assert_eq!(constraints["board_target"], "amd-alveo-u45n");
    assert_eq!(constraints["target_fmax_mhz"], 350);

    let board_profile = read_json("examples/contracts/board-profile.alveo-u45n.json");
    assert_eq!(board_profile["schema_version"], "spac.board-profile.v0");
    assert_eq!(board_profile["board_model"], "Alveo U45N");
    assert_eq!(board_profile["toolchain"]["version"], "2023.2");

    let vitis_report = fs::read_to_string(
        workspace_root().join("examples/contracts/hw-report.vitis_hls_summary.rpt"),
    )
    .expect("read Vitis report fixture");
    assert!(vitis_report.contains("Synthetic Vitis HLS summary fixture"));
    assert!(vitis_report.contains("FMax_MHz: 350.0"));

    let vivado_report = fs::read_to_string(
        workspace_root().join("examples/contracts/hw-report.vivado_timing_summary.rpt"),
    )
    .expect("read Vivado report fixture");
    assert!(vivado_report.contains("Synthetic Vivado timing/resource fixture"));
    assert!(vivado_report.contains("Achieved Fmax MHz: 347.5"));

    let trace = read_json("examples/contracts/trace.hft_tiny.json");
    assert_eq!(trace["schema_version"], "spac.trace.v0");
    assert_eq!(trace["workload_class"], "hft");
    assert_eq!(trace["packets"].as_array().expect("trace packets").len(), 3);

    let datacenter_architecture =
        read_json("examples/contracts/architecture.datacenter_hash_shared_islip.json");
    assert_eq!(
        datacenter_architecture["forwarding_table"]["type"],
        "multi_bank_hash"
    );
    assert_eq!(datacenter_architecture["voq"]["type"], "shared");
    assert_eq!(datacenter_architecture["scheduler"]["type"], "islip");

    let one_buffer_architecture =
        read_json("examples/contracts/architecture.hft_full_lookup_one_buffer_islip.json");
    assert_eq!(
        one_buffer_architecture["voq"]["type"],
        "one_buffer_per_port"
    );
    assert_eq!(one_buffer_architecture["bus_width_bits"], 512);

    let datacenter_trace = read_json("examples/contracts/trace.datacenter_incast.json");
    assert_eq!(datacenter_trace["workload_class"], "datacenter");
    assert_eq!(
        datacenter_trace["packets"]
            .as_array()
            .expect("datacenter trace packets")
            .len(),
        4
    );

    let underwater_architecture =
        read_json("examples/contracts/architecture.underwater_hash_nbyn_edrrm.json");
    assert_eq!(underwater_architecture["voq"]["type"], "n_by_n");
    assert_eq!(underwater_architecture["scheduler"]["type"], "edrrm");

    let underwater_trace = read_json("examples/contracts/trace.underwater_burst.json");
    assert_eq!(underwater_trace["workload_class"], "underwater_sensor");

    let dse_space = read_json("examples/contracts/dse-space.tiny.json");
    assert_eq!(dse_space["schema_version"], "spac.dse-space.v0");
    assert_eq!(
        dse_space["candidates"]
            .as_array()
            .expect("DSE candidates")
            .len(),
        4
    );

    let dse_trace = read_json("examples/contracts/trace.dse_tiny_burst.json");
    assert_eq!(dse_trace["schema_version"], "spac.trace.v0");
    assert_eq!(
        dse_trace["packets"]
            .as_array()
            .expect("DSE trace packets")
            .len(),
        3
    );

    let dse_constraints = read_json("examples/contracts/constraints.dse_tiny_lenient.json");
    assert_eq!(dse_constraints["schema_version"], "spac.constraints.v0");
    assert_eq!(dse_constraints["board_target"], "software-only");

    let run = read_json("examples/contracts/experiment-run.e0_hft_layout.json");
    assert_eq!(run["schema_version"], "spac.experiment-run.v0");
    assert_eq!(run["stage"], "E0");
    assert_eq!(run["trust_level"], "software_model");
    assert_eq!(run["board_profile_id"], "not_applicable");
    assert_eq!(run["artifact_manifest_schema"], "spac.artifact-manifest.v0");

    for bucket in ["input_files", "output_files"] {
        for artifact in run[bucket].as_array().expect("artifact array") {
            let path = artifact["path"].as_str().expect("artifact path");
            let sha = artifact["sha256"].as_str().expect("artifact hash");
            assert!(
                workspace_root().join(path).exists(),
                "artifact path must exist: {path}"
            );
            assert_eq!(sha.len(), 64, "artifact hash must be 64 hex chars");
            assert!(
                sha.chars().all(|ch| ch.is_ascii_hexdigit()),
                "artifact hash must be hex: {sha}"
            );
        }
    }
}

#[test]
fn spac_ae_comparison_records_reuse_boundaries() {
    let doc = fs::read_to_string(workspace_root().join("docs-public/spac-ae-comparison.md"))
        .expect("read SPAC-AE comparison");

    assert!(doc.contains("cfa56eaf3ebf5ffba0a92753a1d6581c4255ec73"));
    assert!(doc.contains("Take as provenance-tagged fixtures"));
    assert!(doc.contains("Do not import as maintained production source"));
    assert!(doc.contains("Vitis IDE 2025.2"));
    assert!(doc.contains("does not provide enough public raw Vitis/Vivado report evidence"));
}

#[test]
fn tracked_public_docs_do_not_reintroduce_forbidden_surfaces() {
    let combined = [
        fs::read_to_string(workspace_root().join("README.md")).expect("read README"),
        fs::read_to_string(workspace_root().join("docs-public/engineering-overview.md"))
            .expect("read engineering overview"),
        fs::read_to_string(workspace_root().join("docs-public/fpga-validation.md"))
            .expect("read fpga validation"),
        fs::read_to_string(workspace_root().join("docs-public/fpga-first-light.md"))
            .expect("read fpga first light"),
        fs::read_to_string(workspace_root().join("docs-public/spac-ae-comparison.md"))
            .expect("read SPAC-AE comparison"),
        fs::read_to_string(workspace_root().join("docs-public/maturity-todo.md"))
            .expect("read maturity TODO"),
    ]
    .join("\n");

    let forbidden_surfaces = [
        ["spac", "-lite"].concat(),
        ["py", "project"].concat(),
        ["py", "thon -m"].concat(),
        ["P", "ython API"].concat(),
        ["single P", "ython package"].concat(),
        ["L", "LM"].concat(),
        ["Chat", "GPT"].concat(),
        ["Co", "dex"].concat(),
        ["auto", "nomous engineering ", "ag", "ent"].concat(),
    ];

    for forbidden in forbidden_surfaces {
        assert!(
            !combined.contains(&forbidden),
            "tracked public docs must not contain forbidden surface '{forbidden}'"
        );
    }
}

fn read_json(relative_path: &str) -> Value {
    let text = fs::read_to_string(workspace_root().join(relative_path))
        .unwrap_or_else(|error| panic!("failed to read {relative_path}: {error}"));
    serde_json::from_str(&text)
        .unwrap_or_else(|error| panic!("failed to parse {relative_path} as JSON: {error}"))
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root")
        .to_path_buf()
}
