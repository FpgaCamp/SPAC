use spac_codegen::{
    generate_hls_csim_package, generate_hls_traits, hls_csim_run_report,
    hls_traits_generation_report, HlsCsimPackageFiles, HlsCsimRunReportInput, HlsCsimToolReport,
};
use spac_core::{
    build_artifact_manifest, validate_architecture_config_file, validate_board_profile_file,
    validate_constraints_config_file, validate_project_config_file, write_artifact_manifest_file,
    BoardProfile, Diagnostic, ExperimentRun, MetadataModel, Severity, ValidationReport,
    DEFAULT_ARTIFACT_MANIFEST_SCHEMA_VERSION, EXPERIMENT_RUN_SCHEMA_VERSION,
    SUPPORTED_ARCHITECTURE_SCHEMA_VERSION, SUPPORTED_BOARD_PROFILE_SCHEMA_VERSION,
    SUPPORTED_CONSTRAINTS_SCHEMA_VERSION, SUPPORTED_PROJECT_SCHEMA_VERSION,
    SUPPORTED_TRACE_SCHEMA_VERSION,
};
use spac_dse::{
    generate_spac_ae_dse_space, run_dse as execute_dse, run_spac_ae_phase2_buffer_dse,
    validate_dse_space_file, Phase2BufferOptions,
};
use spac_dsl::parse_protocol_text;
use spac_hwreport::{accept_hw_report, parse_hw_report_file, HwReport, HwReportTool};
use spac_layout::analyze_layout;
use spac_sim::run_simulation;
use spac_trace::{
    import_spac_ae_trace_file, parse_spac_ae_topology_file, validate_trace_file, WorkloadClass,
};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const EXIT_OK: u8 = 0;
const EXIT_USAGE: u8 = 64;
const EXIT_VALIDATION: u8 = 2;
const EXIT_INCONCLUSIVE: u8 = 3;

fn main() -> ExitCode {
    match run(env::args().skip(1).collect()) {
        Ok(()) => ExitCode::from(EXIT_OK),
        Err(exit) => ExitCode::from(exit),
    }
}

fn run(args: Vec<String>) -> Result<(), u8> {
    match args.first().map(String::as_str) {
        Some("--version" | "-V" | "version") => {
            println!("spac {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Some("validate") => run_validate(&args[1..]),
        Some("check-config") => run_check_config(&args[1..]),
        Some("check-constraints") => run_check_constraints(&args[1..]),
        Some("check-board-profile") => run_check_board_profile(&args[1..]),
        Some("check-trace") => run_check_trace(&args[1..]),
        Some("validate-protocol") => run_validate_protocol(&args[1..]),
        Some("analyze-layout") => run_analyze_layout(&args[1..]),
        Some("generate-metadata") => run_generate_metadata(&args[1..]),
        Some("generate-hls-traits") => run_generate_hls_traits(&args[1..]),
        Some("package-hls-csim") => run_package_hls_csim(&args[1..]),
        Some("parse-hw-report") => run_parse_hw_report(&args[1..]),
        Some("accept-hw-report") => run_accept_hw_report(&args[1..]),
        Some("package-experiment") => run_package_experiment(&args[1..]),
        Some("import-spac-ae-trace") => run_import_spac_ae_trace(&args[1..]),
        Some("generate-spac-ae-dse-space") => run_generate_spac_ae_dse_space(&args[1..]),
        Some("simulate") => run_simulate(&args[1..]),
        Some("dse") => run_dse(&args[1..]),
        Some("--help" | "-h" | "help") | None => {
            print_usage();
            Ok(())
        }
        Some(command) => {
            eprintln!("unknown command '{command}'");
            print_usage();
            Err(EXIT_USAGE)
        }
    }
}

fn run_validate(args: &[String]) -> Result<(), u8> {
    let Some(config_path) = parse_config_path(args) else {
        eprintln!("usage error: validate requires --config <path>");
        print_usage();
        return Err(EXIT_USAGE);
    };

    match validate_project_config_file(&config_path) {
        Ok(_) => {
            let report = ValidationReport::Ok {
                schema_version: SUPPORTED_PROJECT_SCHEMA_VERSION.to_string(),
            };
            print_json(&report).map_err(|error| {
                eprintln!("failed to render validation report: {error}");
                EXIT_VALIDATION
            })?;
            Ok(())
        }
        Err(diagnostics) => {
            let report = ValidationReport::Error { diagnostics };
            print_json(&report).map_err(|error| {
                eprintln!("failed to render validation report: {error}");
                EXIT_VALIDATION
            })?;
            Err(EXIT_VALIDATION)
        }
    }
}

fn run_check_config(args: &[String]) -> Result<(), u8> {
    let Some(config_path) = parse_option_path(args, "--architecture") else {
        eprintln!("usage error: check-config requires --architecture <path>");
        print_usage();
        return Err(EXIT_USAGE);
    };

    match validate_architecture_config_file(&config_path) {
        Ok(_) => {
            let report = ValidationReport::Ok {
                schema_version: SUPPORTED_ARCHITECTURE_SCHEMA_VERSION.to_string(),
            };
            print_json(&report).map_err(|error| {
                eprintln!("failed to render validation report: {error}");
                EXIT_VALIDATION
            })?;
            Ok(())
        }
        Err(diagnostics) => {
            let report = ValidationReport::Error { diagnostics };
            print_json(&report).map_err(|error| {
                eprintln!("failed to render validation report: {error}");
                EXIT_VALIDATION
            })?;
            Err(EXIT_VALIDATION)
        }
    }
}

fn run_dse(args: &[String]) -> Result<(), u8> {
    let Some(space_path) = parse_option_path(args, "--space") else {
        eprintln!("usage error: dse requires --space <path>");
        print_usage();
        return Err(EXIT_USAGE);
    };
    let Some(trace_path) = parse_option_path(args, "--trace") else {
        eprintln!("usage error: dse requires --trace <path>");
        print_usage();
        return Err(EXIT_USAGE);
    };
    let Some(constraints_path) = parse_option_path(args, "--constraints") else {
        eprintln!("usage error: dse requires --constraints <path>");
        print_usage();
        return Err(EXIT_USAGE);
    };
    let Some(out_dir) = parse_option_path(args, "--out") else {
        eprintln!("usage error: dse requires --out <dir>");
        print_usage();
        return Err(EXIT_USAGE);
    };
    let phase2_enabled = has_flag(args, "--spac-ae-phase2-buffers");
    let phase2_options = Phase2BufferOptions {
        top_n: parse_usize_option(args, "--phase2-top-n")?.unwrap_or(1),
        min_depth_packets: parse_u32_option(args, "--min-voq-depth-packets")?.unwrap_or(64),
        max_drop_rate: parse_f64_option(args, "--phase2-max-drop-rate")?.unwrap_or(0.01),
    };

    let space = match validate_dse_space_file(&space_path) {
        Ok(space) => space,
        Err(diagnostics) => {
            print_diagnostics(diagnostics)?;
            return Err(EXIT_VALIDATION);
        }
    };
    let trace = match validate_trace_file(&trace_path) {
        Ok(trace) => trace,
        Err(diagnostics) => {
            print_diagnostics(diagnostics)?;
            return Err(EXIT_VALIDATION);
        }
    };
    let constraints = match validate_constraints_config_file(&constraints_path) {
        Ok(constraints) => constraints,
        Err(diagnostics) => {
            print_diagnostics(diagnostics)?;
            return Err(EXIT_VALIDATION);
        }
    };
    let report = match if phase2_enabled {
        run_spac_ae_phase2_buffer_dse(&space, &trace, &constraints, phase2_options)
    } else {
        execute_dse(&space, &trace, &constraints)
    } {
        Ok(report) => report,
        Err(diagnostics) => {
            print_diagnostics(diagnostics)?;
            return Err(EXIT_VALIDATION);
        }
    };
    let report_json = serde_json::to_string_pretty(&report).map_err(|error| {
        eprintln!("failed to render DSE result: {error}");
        EXIT_VALIDATION
    })?;

    fs::create_dir_all(&out_dir).map_err(|error| {
        eprintln!(
            "failed to create output directory '{}': {error}",
            out_dir.display()
        );
        EXIT_VALIDATION
    })?;

    let report_path = out_dir.join("dse_result.json");
    fs::write(&report_path, report_json.as_bytes()).map_err(|error| {
        eprintln!("failed to write dse_result.json: {error}");
        EXIT_VALIDATION
    })?;

    let input_files = vec![space_path, trace_path, constraints_path];
    let output_files = vec![report_path];
    let command_mode = if phase2_enabled { "dse-phase2" } else { "dse" };
    let run_id = format!(
        "{command_mode}-{}-{}-{}",
        space.name, trace.name, constraints.name
    );
    let manifest = build_artifact_manifest(
        run_id,
        "spac",
        env!("CARGO_PKG_VERSION"),
        &input_files,
        &output_files,
    )
    .map_err(|error| {
        eprintln!("failed to build {DEFAULT_ARTIFACT_MANIFEST_SCHEMA_VERSION}: {error}");
        EXIT_VALIDATION
    })?;

    write_artifact_manifest_file(&out_dir.join("manifest.json"), &manifest).map_err(|error| {
        eprintln!("failed to write manifest.json: {error}");
        EXIT_VALIDATION
    })?;

    println!("{report_json}");
    Ok(())
}

fn run_generate_spac_ae_dse_space(args: &[String]) -> Result<(), u8> {
    let name =
        parse_option_value(args, "--name").unwrap_or_else(|| "spac_ae_dse_space".to_string());
    let Some(ports) = parse_u16_option(args, "--ports")? else {
        eprintln!("usage error: generate-spac-ae-dse-space requires --ports <n>");
        print_usage();
        return Err(EXIT_USAGE);
    };
    let Some(out_dir) = parse_option_path(args, "--out") else {
        eprintln!("usage error: generate-spac-ae-dse-space requires --out <dir>");
        print_usage();
        return Err(EXIT_USAGE);
    };

    if ports < 2 {
        print_diagnostics(vec![Diagnostic::error(
            "SPAC_AE_DSE_PORTS",
            "--ports",
            "ports must be at least 2",
        )])?;
        return Err(EXIT_VALIDATION);
    }

    let space = generate_spac_ae_dse_space(name, ports);
    let space_json = serde_json::to_string_pretty(&space).map_err(|error| {
        eprintln!("failed to render SPAC-AE DSE space: {error}");
        EXIT_VALIDATION
    })?;

    fs::create_dir_all(&out_dir).map_err(|error| {
        eprintln!(
            "failed to create output directory '{}': {error}",
            out_dir.display()
        );
        EXIT_VALIDATION
    })?;

    let space_path = out_dir.join("dse_space.json");
    fs::write(&space_path, space_json.as_bytes()).map_err(|error| {
        eprintln!("failed to write dse_space.json: {error}");
        EXIT_VALIDATION
    })?;

    let output_files = vec![space_path];
    let run_id = format!("generate-spac-ae-dse-space-{}-{}p", space.name, ports);
    let manifest = build_artifact_manifest(
        run_id,
        "spac",
        env!("CARGO_PKG_VERSION"),
        &[],
        &output_files,
    )
    .map_err(|error| {
        eprintln!("failed to build {DEFAULT_ARTIFACT_MANIFEST_SCHEMA_VERSION}: {error}");
        EXIT_VALIDATION
    })?;

    write_artifact_manifest_file(&out_dir.join("manifest.json"), &manifest).map_err(|error| {
        eprintln!("failed to write manifest.json: {error}");
        EXIT_VALIDATION
    })?;

    println!("{space_json}");
    Ok(())
}

fn run_check_board_profile(args: &[String]) -> Result<(), u8> {
    let Some(board_profile_path) = parse_option_path(args, "--board-profile") else {
        eprintln!("usage error: check-board-profile requires --board-profile <path>");
        print_usage();
        return Err(EXIT_USAGE);
    };

    match validate_board_profile_file(&board_profile_path) {
        Ok(_) => {
            let report = ValidationReport::Ok {
                schema_version: SUPPORTED_BOARD_PROFILE_SCHEMA_VERSION.to_string(),
            };
            print_json(&report).map_err(|error| {
                eprintln!("failed to render validation report: {error}");
                EXIT_VALIDATION
            })?;
            Ok(())
        }
        Err(diagnostics) => {
            let report = ValidationReport::Error { diagnostics };
            print_json(&report).map_err(|error| {
                eprintln!("failed to render validation report: {error}");
                EXIT_VALIDATION
            })?;
            Err(EXIT_VALIDATION)
        }
    }
}

fn run_check_constraints(args: &[String]) -> Result<(), u8> {
    let Some(constraints_path) = parse_option_path(args, "--constraints") else {
        eprintln!("usage error: check-constraints requires --constraints <path>");
        print_usage();
        return Err(EXIT_USAGE);
    };

    match validate_constraints_config_file(&constraints_path) {
        Ok(_) => {
            let report = ValidationReport::Ok {
                schema_version: SUPPORTED_CONSTRAINTS_SCHEMA_VERSION.to_string(),
            };
            print_json(&report).map_err(|error| {
                eprintln!("failed to render validation report: {error}");
                EXIT_VALIDATION
            })?;
            Ok(())
        }
        Err(diagnostics) => {
            let report = ValidationReport::Error { diagnostics };
            print_json(&report).map_err(|error| {
                eprintln!("failed to render validation report: {error}");
                EXIT_VALIDATION
            })?;
            Err(EXIT_VALIDATION)
        }
    }
}

fn run_check_trace(args: &[String]) -> Result<(), u8> {
    let Some(trace_path) = parse_option_path(args, "--trace") else {
        eprintln!("usage error: check-trace requires --trace <path>");
        print_usage();
        return Err(EXIT_USAGE);
    };

    match validate_trace_file(&trace_path) {
        Ok(_) => {
            let report = ValidationReport::Ok {
                schema_version: SUPPORTED_TRACE_SCHEMA_VERSION.to_string(),
            };
            print_json(&report).map_err(|error| {
                eprintln!("failed to render validation report: {error}");
                EXIT_VALIDATION
            })?;
            Ok(())
        }
        Err(diagnostics) => {
            let report = ValidationReport::Error { diagnostics };
            print_json(&report).map_err(|error| {
                eprintln!("failed to render validation report: {error}");
                EXIT_VALIDATION
            })?;
            Err(EXIT_VALIDATION)
        }
    }
}

fn run_import_spac_ae_trace(args: &[String]) -> Result<(), u8> {
    let Some(trace_path) = parse_option_path(args, "--trace") else {
        eprintln!("usage error: import-spac-ae-trace requires --trace <csv>");
        print_usage();
        return Err(EXIT_USAGE);
    };
    let Some(name) = parse_option_value(args, "--name") else {
        eprintln!("usage error: import-spac-ae-trace requires --name <trace-name>");
        print_usage();
        return Err(EXIT_USAGE);
    };
    let Some(workload_text) = parse_option_value(args, "--workload-class") else {
        eprintln!("usage error: import-spac-ae-trace requires --workload-class <class>");
        print_usage();
        return Err(EXIT_USAGE);
    };
    let Some(ports) = parse_u16_option(args, "--ports")? else {
        eprintln!("usage error: import-spac-ae-trace requires --ports <n>");
        print_usage();
        return Err(EXIT_USAGE);
    };
    let Some(out_dir) = parse_option_path(args, "--out") else {
        eprintln!("usage error: import-spac-ae-trace requires --out <dir>");
        print_usage();
        return Err(EXIT_USAGE);
    };

    let workload_class = match workload_text.parse::<WorkloadClass>() {
        Ok(workload_class) => workload_class,
        Err(diagnostic) => {
            print_diagnostics(vec![diagnostic])?;
            return Err(EXIT_VALIDATION);
        }
    };
    let topology_path = parse_option_path(args, "--topology");
    let topology = match &topology_path {
        Some(path) => match parse_spac_ae_topology_file(path) {
            Ok(topology) => Some(topology),
            Err(diagnostics) => {
                print_diagnostics(diagnostics)?;
                return Err(EXIT_VALIDATION);
            }
        },
        None => None,
    };

    let trace = match import_spac_ae_trace_file(
        &trace_path,
        name,
        workload_class,
        ports,
        topology.as_ref(),
    ) {
        Ok(trace) => trace,
        Err(diagnostics) => {
            print_diagnostics(diagnostics)?;
            return Err(EXIT_VALIDATION);
        }
    };
    let trace_json = serde_json::to_string_pretty(&trace).map_err(|error| {
        eprintln!("failed to render imported trace: {error}");
        EXIT_VALIDATION
    })?;

    fs::create_dir_all(&out_dir).map_err(|error| {
        eprintln!(
            "failed to create output directory '{}': {error}",
            out_dir.display()
        );
        EXIT_VALIDATION
    })?;

    let output_path = out_dir.join("trace.json");
    fs::write(&output_path, trace_json.as_bytes()).map_err(|error| {
        eprintln!("failed to write trace.json: {error}");
        EXIT_VALIDATION
    })?;

    let mut input_files = vec![trace_path];
    if let Some(path) = topology_path {
        input_files.push(path);
    }
    let output_files = vec![output_path];
    let run_id = format!("import-spac-ae-trace-{}", trace.name);
    let manifest = build_artifact_manifest(
        run_id,
        "spac",
        env!("CARGO_PKG_VERSION"),
        &input_files,
        &output_files,
    )
    .map_err(|error| {
        eprintln!("failed to build {DEFAULT_ARTIFACT_MANIFEST_SCHEMA_VERSION}: {error}");
        EXIT_VALIDATION
    })?;

    write_artifact_manifest_file(&out_dir.join("manifest.json"), &manifest).map_err(|error| {
        eprintln!("failed to write manifest.json: {error}");
        EXIT_VALIDATION
    })?;

    println!("{trace_json}");
    Ok(())
}

fn run_validate_protocol(args: &[String]) -> Result<(), u8> {
    let Some(protocol_path) = parse_option_path(args, "--protocol") else {
        eprintln!("usage error: validate-protocol requires --protocol <path>");
        print_usage();
        return Err(EXIT_USAGE);
    };

    match read_protocol(&protocol_path).and_then(|text| parse_protocol_text(&text)) {
        Ok(protocol) => {
            let report = serde_json::json!({
                "status": "ok",
                "protocol_name": protocol.name,
                "fields": protocol.fields.len()
            });
            print_json_value(&report)?;
            Ok(())
        }
        Err(diagnostics) => {
            print_diagnostics(diagnostics)?;
            Err(EXIT_VALIDATION)
        }
    }
}

fn run_analyze_layout(args: &[String]) -> Result<(), u8> {
    let (protocol_path, bus_width_bits) = parse_protocol_and_bus_width_args(args)?;
    let metadata = load_metadata_from_protocol(&protocol_path, bus_width_bits)?;
    let metadata_json = render_metadata_json(&metadata)?;

    if let Some(out_dir) = parse_option_path(args, "--out") {
        write_metadata_artifacts(
            "analyze-layout",
            &protocol_path,
            &out_dir,
            bus_width_bits,
            &metadata,
            &metadata_json,
        )?;
    }

    println!("{metadata_json}");
    Ok(())
}

fn run_generate_metadata(args: &[String]) -> Result<(), u8> {
    let (protocol_path, bus_width_bits) = parse_protocol_and_bus_width_args(args)?;
    let Some(out_dir) = parse_option_path(args, "--out") else {
        eprintln!("usage error: generate-metadata requires --out <dir>");
        print_usage();
        return Err(EXIT_USAGE);
    };

    let metadata = load_metadata_from_protocol(&protocol_path, bus_width_bits)?;
    let metadata_json = render_metadata_json(&metadata)?;

    write_metadata_artifacts(
        "generate-metadata",
        &protocol_path,
        &out_dir,
        bus_width_bits,
        &metadata,
        &metadata_json,
    )?;

    println!("{metadata_json}");
    Ok(())
}

fn run_simulate(args: &[String]) -> Result<(), u8> {
    let Some(architecture_path) = parse_option_path(args, "--architecture") else {
        eprintln!("usage error: simulate requires --architecture <path>");
        print_usage();
        return Err(EXIT_USAGE);
    };
    let Some(trace_path) = parse_option_path(args, "--trace") else {
        eprintln!("usage error: simulate requires --trace <path>");
        print_usage();
        return Err(EXIT_USAGE);
    };
    let Some(out_dir) = parse_option_path(args, "--out") else {
        eprintln!("usage error: simulate requires --out <dir>");
        print_usage();
        return Err(EXIT_USAGE);
    };

    let architecture = match validate_architecture_config_file(&architecture_path) {
        Ok(architecture) => architecture,
        Err(diagnostics) => {
            print_diagnostics(diagnostics)?;
            return Err(EXIT_VALIDATION);
        }
    };
    let trace = match validate_trace_file(&trace_path) {
        Ok(trace) => trace,
        Err(diagnostics) => {
            print_diagnostics(diagnostics)?;
            return Err(EXIT_VALIDATION);
        }
    };
    let report = match run_simulation(&architecture, &trace) {
        Ok(report) => report,
        Err(diagnostics) => {
            print_diagnostics(diagnostics)?;
            return Err(EXIT_VALIDATION);
        }
    };
    let report_json = serde_json::to_string_pretty(&report).map_err(|error| {
        eprintln!("failed to render simulation report: {error}");
        EXIT_VALIDATION
    })?;

    fs::create_dir_all(&out_dir).map_err(|error| {
        eprintln!(
            "failed to create output directory '{}': {error}",
            out_dir.display()
        );
        EXIT_VALIDATION
    })?;

    let report_path = out_dir.join("simulation_report.json");
    fs::write(&report_path, report_json.as_bytes()).map_err(|error| {
        eprintln!("failed to write simulation_report.json: {error}");
        EXIT_VALIDATION
    })?;

    let input_files = vec![architecture_path, trace_path];
    let output_files = vec![report_path];
    let run_id = format!("simulate-{}-{}", architecture.name, trace.name);
    let manifest = build_artifact_manifest(
        run_id,
        "spac",
        env!("CARGO_PKG_VERSION"),
        &input_files,
        &output_files,
    )
    .map_err(|error| {
        eprintln!("failed to build {DEFAULT_ARTIFACT_MANIFEST_SCHEMA_VERSION}: {error}");
        EXIT_VALIDATION
    })?;

    write_artifact_manifest_file(&out_dir.join("manifest.json"), &manifest).map_err(|error| {
        eprintln!("failed to write manifest.json: {error}");
        EXIT_VALIDATION
    })?;

    println!("{report_json}");
    Ok(())
}

fn run_generate_hls_traits(args: &[String]) -> Result<(), u8> {
    let Some(metadata_path) = parse_option_path(args, "--metadata") else {
        eprintln!("usage error: generate-hls-traits requires --metadata <path>");
        print_usage();
        return Err(EXIT_USAGE);
    };
    let Some(out_dir) = parse_option_path(args, "--out") else {
        eprintln!("usage error: generate-hls-traits requires --out <dir>");
        print_usage();
        return Err(EXIT_USAGE);
    };

    let metadata = load_metadata_file(&metadata_path)?;
    let generated = match generate_hls_traits(&metadata) {
        Ok(generated) => generated,
        Err(diagnostics) => {
            print_diagnostics(diagnostics)?;
            return Err(EXIT_VALIDATION);
        }
    };

    fs::create_dir_all(&out_dir).map_err(|error| {
        eprintln!(
            "failed to create output directory '{}': {error}",
            out_dir.display()
        );
        EXIT_VALIDATION
    })?;

    let header_path = out_dir.join(&generated.file_name);
    fs::write(&header_path, generated.text.as_bytes()).map_err(|error| {
        eprintln!("failed to write {}: {error}", generated.file_name);
        EXIT_VALIDATION
    })?;

    let run_id = format!("generate-hls-traits-{}", metadata.protocol_name);
    let manifest = build_artifact_manifest(
        run_id,
        "spac",
        env!("CARGO_PKG_VERSION"),
        std::slice::from_ref(&metadata_path),
        std::slice::from_ref(&header_path),
    )
    .map_err(|error| {
        eprintln!("failed to build {DEFAULT_ARTIFACT_MANIFEST_SCHEMA_VERSION}: {error}");
        EXIT_VALIDATION
    })?;

    let manifest_path = out_dir.join("manifest.json");
    write_artifact_manifest_file(&manifest_path, &manifest).map_err(|error| {
        eprintln!("failed to write manifest.json: {error}");
        EXIT_VALIDATION
    })?;

    let report = hls_traits_generation_report(
        &metadata,
        header_path.display().to_string(),
        manifest_path.display().to_string(),
    );
    let report_value = serde_json::to_value(report).map_err(|error| {
        eprintln!("failed to render HLS traits generation report: {error}");
        EXIT_VALIDATION
    })?;
    print_json_value(&report_value)
}

fn run_package_hls_csim(args: &[String]) -> Result<(), u8> {
    let Some(metadata_path) = parse_option_path(args, "--metadata") else {
        eprintln!("usage error: package-hls-csim requires --metadata <path>");
        print_usage();
        return Err(EXIT_USAGE);
    };
    let Some(board_profile_path) = parse_option_path(args, "--board-profile") else {
        eprintln!("usage error: package-hls-csim requires --board-profile <path>");
        print_usage();
        return Err(EXIT_USAGE);
    };
    let Some(out_dir) = parse_option_path(args, "--out") else {
        eprintln!("usage error: package-hls-csim requires --out <dir>");
        print_usage();
        return Err(EXIT_USAGE);
    };
    let execute = has_flag(args, "--execute");
    let vitis_hls_bin = parse_option_path(args, "--vitis-hls-bin");
    let timeout_seconds = parse_u64_option(args, "--timeout-seconds")?.unwrap_or(300);
    if timeout_seconds == 0 {
        print_diagnostics(vec![Diagnostic::error(
            "SPAC_HLS_CSIM_TIMEOUT",
            "--timeout-seconds",
            "timeout must be greater than zero seconds",
        )])?;
        return Err(EXIT_VALIDATION);
    }

    let metadata = load_metadata_file(&metadata_path)?;
    let board_profile = match validate_board_profile_file(&board_profile_path) {
        Ok(board_profile) => board_profile,
        Err(diagnostics) => {
            print_diagnostics(diagnostics)?;
            return Err(EXIT_VALIDATION);
        }
    };
    let package = match generate_hls_csim_package(&metadata, &board_profile) {
        Ok(package) => package,
        Err(diagnostics) => {
            print_diagnostics(diagnostics)?;
            return Err(EXIT_VALIDATION);
        }
    };

    fs::create_dir_all(&out_dir).map_err(|error| {
        eprintln!(
            "failed to create output directory '{}': {error}",
            out_dir.display()
        );
        EXIT_VALIDATION
    })?;

    let mut output_files = Vec::new();
    for artifact in package.artifacts {
        let path = out_dir.join(&artifact.file_name);
        fs::write(&path, artifact.text.as_bytes()).map_err(|error| {
            eprintln!("failed to write {}: {error}", artifact.file_name);
            EXIT_VALIDATION
        })?;
        output_files.push(path);
    }

    let execution = prepare_or_run_hls_csim(
        execute,
        vitis_hls_bin.as_ref(),
        &out_dir,
        timeout_seconds,
        &board_profile,
    )?;
    output_files.extend(execution.log_paths.iter().cloned());

    let report_path = out_dir.join("hls_csim_run.json");
    let manifest_path = out_dir.join("manifest.json");
    let package_files = HlsCsimPackageFiles {
        packet_header_path: out_dir.join("packet.hpp").display().to_string(),
        smoke_source_path: out_dir.join("csim_smoke.cpp").display().to_string(),
        hls_config_path: out_dir.join("hls_config.cfg").display().to_string(),
        run_tcl_path: out_dir.join("run_csim.tcl").display().to_string(),
        log_paths: execution
            .log_paths
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
    };
    let command = execution.command.clone();
    let report = hls_csim_run_report(
        &metadata,
        &board_profile,
        HlsCsimRunReportInput {
            status: execution.status,
            trust_level: execution.trust_level,
            package_files,
            manifest_path: manifest_path.display().to_string(),
            command,
            tool: execution.tool,
            diagnostics: execution.diagnostics,
        },
    );
    let report_json = serde_json::to_string_pretty(&report).map_err(|error| {
        eprintln!("failed to render HLS csim run report: {error}");
        EXIT_VALIDATION
    })?;
    fs::write(&report_path, report_json.as_bytes()).map_err(|error| {
        eprintln!("failed to write hls_csim_run.json: {error}");
        EXIT_VALIDATION
    })?;
    output_files.push(report_path);

    let input_files = vec![metadata_path, board_profile_path];
    let manifest = build_artifact_manifest(
        format!(
            "package-hls-csim-{}-{}",
            metadata.protocol_name, board_profile.board_id
        ),
        "spac",
        env!("CARGO_PKG_VERSION"),
        &input_files,
        &output_files,
    )
    .map_err(|error| {
        eprintln!("failed to build {DEFAULT_ARTIFACT_MANIFEST_SCHEMA_VERSION}: {error}");
        EXIT_VALIDATION
    })?;

    write_artifact_manifest_file(&manifest_path, &manifest).map_err(|error| {
        eprintln!("failed to write manifest.json: {error}");
        EXIT_VALIDATION
    })?;

    println!("{report_json}");
    Ok(())
}

fn run_parse_hw_report(args: &[String]) -> Result<(), u8> {
    let Some(tool_text) = parse_option_value(args, "--tool") else {
        eprintln!("usage error: parse-hw-report requires --tool <vitis|vivado>");
        print_usage();
        return Err(EXIT_USAGE);
    };
    let tool = match tool_text.parse::<HwReportTool>() {
        Ok(tool) => tool,
        Err(diagnostic) => {
            print_diagnostics(vec![diagnostic])?;
            return Err(EXIT_VALIDATION);
        }
    };
    let Some(report_path) = parse_option_path(args, "--report") else {
        eprintln!("usage error: parse-hw-report requires --report <path>");
        print_usage();
        return Err(EXIT_USAGE);
    };
    let Some(board_profile_path) = parse_option_path(args, "--board-profile") else {
        eprintln!("usage error: parse-hw-report requires --board-profile <path>");
        print_usage();
        return Err(EXIT_USAGE);
    };
    let Some(out_dir) = parse_option_path(args, "--out") else {
        eprintln!("usage error: parse-hw-report requires --out <dir>");
        print_usage();
        return Err(EXIT_USAGE);
    };

    let board_profile = match validate_board_profile_file(&board_profile_path) {
        Ok(board_profile) => board_profile,
        Err(diagnostics) => {
            print_diagnostics(diagnostics)?;
            return Err(EXIT_VALIDATION);
        }
    };
    let report = match parse_hw_report_file(tool, &report_path, &board_profile) {
        Ok(report) => report,
        Err(diagnostics) => {
            print_diagnostics(diagnostics)?;
            return Err(EXIT_VALIDATION);
        }
    };
    let report_json = serde_json::to_string_pretty(&report).map_err(|error| {
        eprintln!("failed to render hardware report: {error}");
        EXIT_VALIDATION
    })?;

    fs::create_dir_all(&out_dir).map_err(|error| {
        eprintln!(
            "failed to create output directory '{}': {error}",
            out_dir.display()
        );
        EXIT_VALIDATION
    })?;

    let hw_report_path = out_dir.join("hw_report.json");
    fs::write(&hw_report_path, report_json.as_bytes()).map_err(|error| {
        eprintln!("failed to write hw_report.json: {error}");
        EXIT_VALIDATION
    })?;

    let input_files = vec![report_path, board_profile_path];
    let output_files = vec![hw_report_path];
    let manifest = build_artifact_manifest(
        format!(
            "parse-hw-report-{}-{}",
            report.tool, report.board_profile_id
        ),
        "spac",
        env!("CARGO_PKG_VERSION"),
        &input_files,
        &output_files,
    )
    .map_err(|error| {
        eprintln!("failed to build {DEFAULT_ARTIFACT_MANIFEST_SCHEMA_VERSION}: {error}");
        EXIT_VALIDATION
    })?;

    write_artifact_manifest_file(&out_dir.join("manifest.json"), &manifest).map_err(|error| {
        eprintln!("failed to write manifest.json: {error}");
        EXIT_VALIDATION
    })?;

    println!("{report_json}");
    Ok(())
}

fn run_accept_hw_report(args: &[String]) -> Result<(), u8> {
    let Some(report_path) = parse_option_path(args, "--report") else {
        eprintln!("usage error: accept-hw-report requires --report <hw_report.json>");
        print_usage();
        return Err(EXIT_USAGE);
    };
    let Some(constraints_path) = parse_option_path(args, "--constraints") else {
        eprintln!("usage error: accept-hw-report requires --constraints <constraints.json>");
        print_usage();
        return Err(EXIT_USAGE);
    };
    let Some(out_dir) = parse_option_path(args, "--out") else {
        eprintln!("usage error: accept-hw-report requires --out <dir>");
        print_usage();
        return Err(EXIT_USAGE);
    };

    let report = load_hw_report_file(&report_path)?;
    let constraints = match validate_constraints_config_file(&constraints_path) {
        Ok(constraints) => constraints,
        Err(diagnostics) => {
            print_diagnostics(diagnostics)?;
            return Err(EXIT_VALIDATION);
        }
    };

    let acceptance = accept_hw_report(
        &report,
        &constraints,
        report_path.display().to_string(),
        constraints_path.display().to_string(),
    );
    let acceptance_status = acceptance.status.clone();
    let acceptance_json = serde_json::to_string_pretty(&acceptance).map_err(|error| {
        eprintln!("failed to render hardware acceptance report: {error}");
        EXIT_VALIDATION
    })?;

    fs::create_dir_all(&out_dir).map_err(|error| {
        eprintln!(
            "failed to create output directory '{}': {error}",
            out_dir.display()
        );
        EXIT_VALIDATION
    })?;

    let acceptance_path = out_dir.join("hw_acceptance.json");
    fs::write(&acceptance_path, acceptance_json.as_bytes()).map_err(|error| {
        eprintln!("failed to write hw_acceptance.json: {error}");
        EXIT_VALIDATION
    })?;

    let input_files = vec![report_path, constraints_path];
    let output_files = vec![acceptance_path];
    let manifest = build_artifact_manifest(
        format!(
            "accept-hw-report-{}-{}",
            acceptance.board_profile_id, acceptance.constraints_name
        ),
        "spac",
        env!("CARGO_PKG_VERSION"),
        &input_files,
        &output_files,
    )
    .map_err(|error| {
        eprintln!("failed to build {DEFAULT_ARTIFACT_MANIFEST_SCHEMA_VERSION}: {error}");
        EXIT_VALIDATION
    })?;

    write_artifact_manifest_file(&out_dir.join("manifest.json"), &manifest).map_err(|error| {
        eprintln!("failed to write manifest.json: {error}");
        EXIT_VALIDATION
    })?;

    println!("{acceptance_json}");
    match acceptance_status.as_str() {
        "pass" => Ok(()),
        "inconclusive" => Err(EXIT_INCONCLUSIVE),
        _ => Err(EXIT_VALIDATION),
    }
}

fn run_package_experiment(args: &[String]) -> Result<(), u8> {
    let Some(run_dir) = parse_option_path(args, "--run-dir") else {
        eprintln!("usage error: package-experiment requires --run-dir <dir>");
        print_usage();
        return Err(EXIT_USAGE);
    };
    let Some(board_profile_path) = parse_option_path(args, "--board-profile") else {
        eprintln!("usage error: package-experiment requires --board-profile <path>");
        print_usage();
        return Err(EXIT_USAGE);
    };
    let Some(trust_level) = parse_option_value(args, "--trust-level") else {
        eprintln!("usage error: package-experiment requires --trust-level <level>");
        print_usage();
        return Err(EXIT_USAGE);
    };
    let Some(out_dir) = parse_option_path(args, "--out") else {
        eprintln!("usage error: package-experiment requires --out <dir>");
        print_usage();
        return Err(EXIT_USAGE);
    };

    let Some(stage) = stage_for_trust_level(&trust_level) else {
        print_diagnostics(vec![Diagnostic::error(
            "SPAC_EXPERIMENT_TRUST_LEVEL",
            "--trust-level",
            "supported trust levels are software_model, hls_csim, post_synthesis, and hardware_measured",
        )])?;
        return Err(EXIT_VALIDATION);
    };
    let board_profile = match validate_board_profile_file(&board_profile_path) {
        Ok(board_profile) => board_profile,
        Err(diagnostics) => {
            print_diagnostics(diagnostics)?;
            return Err(EXIT_VALIDATION);
        }
    };
    let run_files = match collect_regular_files(&run_dir) {
        Ok(files) if !files.is_empty() => files,
        Ok(_) => {
            print_diagnostics(vec![Diagnostic::error(
                "SPAC_EXPERIMENT_RUN_EMPTY",
                run_dir.display().to_string(),
                "run directory must contain at least one regular evidence file",
            )])?;
            return Err(EXIT_VALIDATION);
        }
        Err(diagnostic) => {
            print_diagnostics(vec![diagnostic])?;
            return Err(EXIT_VALIDATION);
        }
    };

    let manifest_probe = build_artifact_manifest(
        "package-experiment-probe",
        "spac",
        env!("CARGO_PKG_VERSION"),
        std::slice::from_ref(&board_profile_path),
        &run_files,
    )
    .map_err(|error| {
        eprintln!("failed to hash experiment evidence files: {error}");
        EXIT_VALIDATION
    })?;

    let run_id = format!(
        "package-experiment-{}-{}",
        trust_level, board_profile.board_id
    );
    let command = format!(
        "spac package-experiment --run-dir {} --board-profile {} --trust-level {} --out {}",
        run_dir.display(),
        board_profile_path.display(),
        trust_level,
        out_dir.display()
    );
    let experiment = ExperimentRun {
        schema_version: EXPERIMENT_RUN_SCHEMA_VERSION.to_string(),
        run_id: run_id.clone(),
        stage: stage.to_string(),
        trust_level: trust_level.clone(),
        status: "success".to_string(),
        artifact_manifest_schema: DEFAULT_ARTIFACT_MANIFEST_SCHEMA_VERSION.to_string(),
        command,
        board_profile_id: board_profile.board_id.clone(),
        input_files: manifest_probe.input_files,
        output_files: manifest_probe.output_files,
        known_limitations: experiment_limitations(&trust_level),
    };
    let experiment_json = serde_json::to_string_pretty(&experiment).map_err(|error| {
        eprintln!("failed to render experiment run report: {error}");
        EXIT_VALIDATION
    })?;

    fs::create_dir_all(&out_dir).map_err(|error| {
        eprintln!(
            "failed to create output directory '{}': {error}",
            out_dir.display()
        );
        EXIT_VALIDATION
    })?;

    let experiment_path = out_dir.join("experiment_run.json");
    fs::write(&experiment_path, experiment_json.as_bytes()).map_err(|error| {
        eprintln!("failed to write experiment_run.json: {error}");
        EXIT_VALIDATION
    })?;

    let mut manifest_inputs = vec![board_profile_path];
    manifest_inputs.extend(run_files);
    let manifest = build_artifact_manifest(
        run_id,
        "spac",
        env!("CARGO_PKG_VERSION"),
        &manifest_inputs,
        std::slice::from_ref(&experiment_path),
    )
    .map_err(|error| {
        eprintln!("failed to build {DEFAULT_ARTIFACT_MANIFEST_SCHEMA_VERSION}: {error}");
        EXIT_VALIDATION
    })?;

    write_artifact_manifest_file(&out_dir.join("manifest.json"), &manifest).map_err(|error| {
        eprintln!("failed to write manifest.json: {error}");
        EXIT_VALIDATION
    })?;

    println!("{experiment_json}");
    Ok(())
}

struct HlsCsimExecution {
    status: String,
    trust_level: String,
    command: String,
    tool: HlsCsimToolReport,
    diagnostics: Vec<Diagnostic>,
    log_paths: Vec<PathBuf>,
}

fn prepare_or_run_hls_csim(
    execute: bool,
    vitis_hls_bin: Option<&PathBuf>,
    out_dir: &PathBuf,
    timeout_seconds: u64,
    board_profile: &BoardProfile,
) -> Result<HlsCsimExecution, u8> {
    let requested_binary = vitis_hls_bin.map(|path| path.display().to_string());
    let command = requested_binary
        .as_ref()
        .map(|binary| format!("{binary} -f run_csim.tcl"))
        .unwrap_or_else(|| "package-only: no EDA tool executed".to_string());
    let base_tool = HlsCsimToolReport {
        declared_family: board_profile.toolchain.family.clone(),
        declared_version: board_profile.toolchain.version.clone(),
        requested_binary: requested_binary.clone(),
        exit_code: None,
    };

    if !execute {
        return Ok(HlsCsimExecution {
            status: "blocked".to_string(),
            trust_level: "software_model".to_string(),
            command,
            tool: base_tool,
            diagnostics: vec![Diagnostic::error(
                "SPAC_HLS_CSIM_NOT_EXECUTED",
                "--execute",
                "HLS csim package was generated, but no EDA tool execution was requested",
            )],
            log_paths: Vec::new(),
        });
    }

    let Some(binary) = vitis_hls_bin else {
        return Ok(HlsCsimExecution {
            status: "blocked".to_string(),
            trust_level: "software_model".to_string(),
            command,
            tool: base_tool,
            diagnostics: vec![Diagnostic::error(
                "SPAC_HLS_CSIM_TOOL_MISSING",
                "--vitis-hls-bin",
                "--execute requires an explicit Vitis HLS binary path",
            )],
            log_paths: Vec::new(),
        });
    };

    if !binary.exists() {
        return Ok(HlsCsimExecution {
            status: "blocked".to_string(),
            trust_level: "software_model".to_string(),
            command,
            tool: base_tool,
            diagnostics: vec![Diagnostic::error(
                "SPAC_HLS_CSIM_TOOL_NOT_FOUND",
                binary.display().to_string(),
                "requested Vitis HLS binary does not exist",
            )],
            log_paths: Vec::new(),
        });
    }

    run_hls_csim_process(binary, out_dir, timeout_seconds, command, base_tool)
}

fn run_hls_csim_process(
    binary: &PathBuf,
    out_dir: &PathBuf,
    timeout_seconds: u64,
    command: String,
    base_tool: HlsCsimToolReport,
) -> Result<HlsCsimExecution, u8> {
    let mut child = match Command::new(binary)
        .arg("-f")
        .arg("run_csim.tcl")
        .current_dir(out_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(error) => {
            return Ok(HlsCsimExecution {
                status: "blocked".to_string(),
                trust_level: "software_model".to_string(),
                command,
                tool: base_tool,
                diagnostics: vec![Diagnostic::error(
                    "SPAC_HLS_CSIM_SPAWN",
                    binary.display().to_string(),
                    format!("failed to start Vitis HLS: {error}"),
                )],
                log_paths: Vec::new(),
            });
        }
    };

    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => {
                if started.elapsed() >= Duration::from_secs(timeout_seconds) {
                    let _ = child.kill();
                    let output = child.wait_with_output().map_err(|error| {
                        eprintln!("failed to collect timed-out HLS csim output: {error}");
                        EXIT_VALIDATION
                    })?;
                    let log_paths = write_hls_csim_logs(out_dir, &output.stdout, &output.stderr)?;
                    return Ok(HlsCsimExecution {
                        status: "failure".to_string(),
                        trust_level: "software_model".to_string(),
                        command,
                        tool: HlsCsimToolReport {
                            exit_code: output.status.code(),
                            ..base_tool
                        },
                        diagnostics: vec![Diagnostic::error(
                            "SPAC_HLS_CSIM_TIMEOUT",
                            "--timeout-seconds",
                            format!("HLS csim exceeded {timeout_seconds} seconds"),
                        )],
                        log_paths,
                    });
                }
                thread::sleep(Duration::from_millis(100));
            }
            Err(error) => {
                return Ok(HlsCsimExecution {
                    status: "failure".to_string(),
                    trust_level: "software_model".to_string(),
                    command,
                    tool: base_tool,
                    diagnostics: vec![Diagnostic::error(
                        "SPAC_HLS_CSIM_WAIT",
                        binary.display().to_string(),
                        format!("failed while waiting for HLS csim: {error}"),
                    )],
                    log_paths: Vec::new(),
                });
            }
        }
    }

    let output = child.wait_with_output().map_err(|error| {
        eprintln!("failed to collect HLS csim output: {error}");
        EXIT_VALIDATION
    })?;
    let log_paths = write_hls_csim_logs(out_dir, &output.stdout, &output.stderr)?;
    let success = output.status.success();
    Ok(HlsCsimExecution {
        status: if success { "success" } else { "failure" }.to_string(),
        trust_level: if success {
            "hls_csim"
        } else {
            "software_model"
        }
        .to_string(),
        command,
        tool: HlsCsimToolReport {
            exit_code: output.status.code(),
            ..base_tool
        },
        diagnostics: if success {
            Vec::new()
        } else {
            vec![Diagnostic::error(
                "SPAC_HLS_CSIM_FAILED",
                binary.display().to_string(),
                "HLS csim process exited with a non-zero status",
            )]
        },
        log_paths,
    })
}

fn write_hls_csim_logs(out_dir: &Path, stdout: &[u8], stderr: &[u8]) -> Result<Vec<PathBuf>, u8> {
    let stdout_path = out_dir.join("vitis_hls.stdout.log");
    let stderr_path = out_dir.join("vitis_hls.stderr.log");
    fs::write(&stdout_path, stdout).map_err(|error| {
        eprintln!("failed to write vitis_hls.stdout.log: {error}");
        EXIT_VALIDATION
    })?;
    fs::write(&stderr_path, stderr).map_err(|error| {
        eprintln!("failed to write vitis_hls.stderr.log: {error}");
        EXIT_VALIDATION
    })?;
    Ok(vec![stdout_path, stderr_path])
}

fn load_metadata_file(metadata_path: &PathBuf) -> Result<MetadataModel, u8> {
    let metadata_text = fs::read_to_string(metadata_path).map_err(|error| {
        eprintln!(
            "failed to read metadata file '{}': {error}",
            metadata_path.display()
        );
        EXIT_VALIDATION
    })?;
    match serde_json::from_str(&metadata_text) {
        Ok(metadata) => Ok(metadata),
        Err(error) => {
            print_diagnostics(vec![Diagnostic::error(
                "SPAC_METADATA_PARSE",
                metadata_path.display().to_string(),
                format!("failed to parse metadata JSON: {error}"),
            )])?;
            Err(EXIT_VALIDATION)
        }
    }
}

fn load_hw_report_file(report_path: &PathBuf) -> Result<HwReport, u8> {
    let report_text = fs::read_to_string(report_path).map_err(|error| {
        eprintln!(
            "failed to read hardware report file '{}': {error}",
            report_path.display()
        );
        EXIT_VALIDATION
    })?;
    match serde_json::from_str(&report_text) {
        Ok(report) => Ok(report),
        Err(error) => {
            print_diagnostics(vec![Diagnostic::error(
                "SPAC_HW_REPORT_PARSE",
                report_path.display().to_string(),
                format!("failed to parse spac.hw-report.v0 JSON: {error}"),
            )])?;
            Err(EXIT_VALIDATION)
        }
    }
}

fn stage_for_trust_level(trust_level: &str) -> Option<&'static str> {
    match trust_level {
        "software_model" => Some("E0"),
        "hls_csim" => Some("E4"),
        "post_synthesis" => Some("E6"),
        "hardware_measured" => Some("E7"),
        _ => None,
    }
}

fn experiment_limitations(trust_level: &str) -> Vec<String> {
    let mut limitations = match trust_level {
        "software_model" => vec![
            "Software-model evidence only; no HLS synthesis was performed".to_string(),
            "No FPGA hardware measurement was performed".to_string(),
        ],
        "hls_csim" => vec![
            "HLS C simulation evidence only; no post-synthesis timing/resource report is implied"
                .to_string(),
            "No FPGA hardware measurement was performed".to_string(),
        ],
        "post_synthesis" => vec![
            "Post-synthesis evidence is limited to parsed vendor reports and declared constraints"
                .to_string(),
            "No FPGA hardware measurement was performed".to_string(),
        ],
        "hardware_measured" => vec![
            "Hardware-measured evidence is board-specific and does not imply cross-board parity"
                .to_string(),
        ],
        _ => Vec::new(),
    };
    limitations.push(
        "This experiment bundle does not reproduce SPAC paper metrics without matching traces, configs, EDA settings, and hardware evidence"
            .to_string(),
    );
    limitations
}

fn collect_regular_files(root: &Path) -> Result<Vec<PathBuf>, Diagnostic> {
    let metadata = fs::metadata(root).map_err(|error| {
        Diagnostic::error(
            "SPAC_EXPERIMENT_RUN_DIR_READ",
            root.display().to_string(),
            format!("failed to read run directory metadata: {error}"),
        )
    })?;
    if !metadata.is_dir() {
        return Err(Diagnostic::error(
            "SPAC_EXPERIMENT_RUN_DIR",
            root.display().to_string(),
            "run-dir must be an existing directory",
        ));
    }

    let mut files = Vec::new();
    collect_regular_files_inner(root, &mut files)?;
    files.sort_by_key(|path| path.display().to_string());
    Ok(files)
}

fn collect_regular_files_inner(root: &Path, files: &mut Vec<PathBuf>) -> Result<(), Diagnostic> {
    let entries = fs::read_dir(root).map_err(|error| {
        Diagnostic::error(
            "SPAC_EXPERIMENT_RUN_DIR_READ",
            root.display().to_string(),
            format!("failed to read run directory: {error}"),
        )
    })?;

    for entry in entries {
        let entry = entry.map_err(|error| {
            Diagnostic::error(
                "SPAC_EXPERIMENT_RUN_DIR_READ",
                root.display().to_string(),
                format!("failed to read run directory entry: {error}"),
            )
        })?;
        let path = entry.path();
        let metadata = entry.metadata().map_err(|error| {
            Diagnostic::error(
                "SPAC_EXPERIMENT_RUN_FILE_READ",
                path.display().to_string(),
                format!("failed to read evidence file metadata: {error}"),
            )
        })?;
        if metadata.is_dir() {
            collect_regular_files_inner(&path, files)?;
        } else if metadata.is_file() {
            files.push(path);
        }
    }

    Ok(())
}

fn parse_protocol_and_bus_width_args(args: &[String]) -> Result<(PathBuf, u32), u8> {
    let Some(protocol_path) = parse_option_path(args, "--protocol") else {
        eprintln!("usage error: protocol command requires --protocol <path>");
        print_usage();
        return Err(EXIT_USAGE);
    };
    let Some(bus_width_text) = parse_option_value(args, "--bus-width") else {
        eprintln!("usage error: protocol command requires --bus-width <bits>");
        print_usage();
        return Err(EXIT_USAGE);
    };

    let Ok(bus_width_bits) = bus_width_text.parse::<u32>() else {
        print_diagnostics(vec![Diagnostic {
            severity: Severity::Error,
            code: "SPAC_BUS_WIDTH_PARSE".to_string(),
            message: format!("bus width '{bus_width_text}' is not an unsigned integer"),
            path: "--bus-width".to_string(),
        }])?;
        return Err(EXIT_VALIDATION);
    };

    Ok((protocol_path, bus_width_bits))
}

fn load_metadata_from_protocol(
    protocol_path: &PathBuf,
    bus_width_bits: u32,
) -> Result<MetadataModel, u8> {
    match read_protocol(protocol_path).and_then(|text| parse_protocol_text(&text)) {
        Ok(protocol) => match analyze_layout(&protocol, bus_width_bits) {
            Ok(metadata) => Ok(metadata),
            Err(diagnostics) => {
                print_diagnostics(diagnostics)?;
                Err(EXIT_VALIDATION)
            }
        },
        Err(diagnostics) => {
            print_diagnostics(diagnostics)?;
            Err(EXIT_VALIDATION)
        }
    }
}

fn render_metadata_json(metadata: &MetadataModel) -> Result<String, u8> {
    serde_json::to_string_pretty(metadata).map_err(|error| {
        eprintln!("failed to render metadata model: {error}");
        EXIT_VALIDATION
    })
}

fn write_metadata_artifacts(
    command_name: &str,
    protocol_path: &PathBuf,
    out_dir: &PathBuf,
    bus_width_bits: u32,
    metadata: &MetadataModel,
    metadata_json: &str,
) -> Result<(), u8> {
    fs::create_dir_all(out_dir).map_err(|error| {
        eprintln!(
            "failed to create output directory '{}': {error}",
            out_dir.display()
        );
        EXIT_VALIDATION
    })?;

    let metadata_path = out_dir.join("metadata.json");
    fs::write(&metadata_path, metadata_json.as_bytes()).map_err(|error| {
        eprintln!("failed to write metadata.json: {error}");
        EXIT_VALIDATION
    })?;

    let run_id = format!(
        "{command_name}-{}-bus{bus_width_bits}",
        metadata.protocol_name
    );
    let manifest = build_artifact_manifest(
        run_id,
        "spac",
        env!("CARGO_PKG_VERSION"),
        std::slice::from_ref(protocol_path),
        std::slice::from_ref(&metadata_path),
    )
    .map_err(|error| {
        eprintln!("failed to build {DEFAULT_ARTIFACT_MANIFEST_SCHEMA_VERSION}: {error}");
        EXIT_VALIDATION
    })?;

    write_artifact_manifest_file(&out_dir.join("manifest.json"), &manifest).map_err(|error| {
        eprintln!("failed to write manifest.json: {error}");
        EXIT_VALIDATION
    })
}

fn parse_config_path(args: &[String]) -> Option<PathBuf> {
    parse_option_path(args, "--config")
}

fn parse_option_path(args: &[String], option: &str) -> Option<PathBuf> {
    parse_option_value(args, option).map(PathBuf::from)
}

fn parse_option_value(args: &[String], option: &str) -> Option<String> {
    args.windows(2)
        .find(|window| window[0] == option)
        .map(|window| window[1].clone())
}

fn parse_u16_option(args: &[String], option: &str) -> Result<Option<u16>, u8> {
    let Some(value) = parse_option_value(args, option) else {
        return Ok(None);
    };

    match value.parse::<u16>() {
        Ok(parsed) => Ok(Some(parsed)),
        Err(error) => {
            print_diagnostics(vec![Diagnostic::error(
                "SPAC_U16_PARSE",
                option,
                format!("failed to parse {option} as u16: {error}"),
            )])?;
            Err(EXIT_VALIDATION)
        }
    }
}

fn parse_u32_option(args: &[String], option: &str) -> Result<Option<u32>, u8> {
    let Some(value) = parse_option_value(args, option) else {
        return Ok(None);
    };

    match value.parse::<u32>() {
        Ok(parsed) => Ok(Some(parsed)),
        Err(error) => {
            print_diagnostics(vec![Diagnostic::error(
                "SPAC_U32_PARSE",
                option,
                format!("failed to parse {option} as u32: {error}"),
            )])?;
            Err(EXIT_VALIDATION)
        }
    }
}

fn parse_u64_option(args: &[String], option: &str) -> Result<Option<u64>, u8> {
    let Some(value) = parse_option_value(args, option) else {
        return Ok(None);
    };

    match value.parse::<u64>() {
        Ok(parsed) => Ok(Some(parsed)),
        Err(error) => {
            print_diagnostics(vec![Diagnostic::error(
                "SPAC_U64_PARSE",
                option,
                format!("failed to parse {option} as u64: {error}"),
            )])?;
            Err(EXIT_VALIDATION)
        }
    }
}

fn parse_usize_option(args: &[String], option: &str) -> Result<Option<usize>, u8> {
    let Some(value) = parse_option_value(args, option) else {
        return Ok(None);
    };

    match value.parse::<usize>() {
        Ok(parsed) => Ok(Some(parsed)),
        Err(error) => {
            print_diagnostics(vec![Diagnostic::error(
                "SPAC_USIZE_PARSE",
                option,
                format!("failed to parse {option} as usize: {error}"),
            )])?;
            Err(EXIT_VALIDATION)
        }
    }
}

fn parse_f64_option(args: &[String], option: &str) -> Result<Option<f64>, u8> {
    let Some(value) = parse_option_value(args, option) else {
        return Ok(None);
    };

    match value.parse::<f64>() {
        Ok(parsed) => Ok(Some(parsed)),
        Err(error) => {
            print_diagnostics(vec![Diagnostic::error(
                "SPAC_F64_PARSE",
                option,
                format!("failed to parse {option} as f64: {error}"),
            )])?;
            Err(EXIT_VALIDATION)
        }
    }
}

fn has_flag(args: &[String], flag: &str) -> bool {
    args.iter().any(|arg| arg == flag)
}

fn print_json(report: &ValidationReport) -> Result<(), serde_json::Error> {
    let text = serde_json::to_string_pretty(report)?;
    println!("{text}");
    Ok(())
}

fn print_json_value(value: &serde_json::Value) -> Result<(), u8> {
    let text = serde_json::to_string_pretty(value).map_err(|error| {
        eprintln!("failed to render JSON report: {error}");
        EXIT_VALIDATION
    })?;
    println!("{text}");
    Ok(())
}

fn print_diagnostics(diagnostics: Vec<Diagnostic>) -> Result<(), u8> {
    let report = ValidationReport::Error { diagnostics };
    print_json(&report).map_err(|error| {
        eprintln!("failed to render diagnostics: {error}");
        EXIT_VALIDATION
    })
}

fn read_protocol(protocol_path: &PathBuf) -> Result<String, Vec<Diagnostic>> {
    fs::read_to_string(protocol_path).map_err(|error| {
        vec![Diagnostic {
            severity: Severity::Error,
            code: "SPAC_PROTOCOL_READ".to_string(),
            message: format!("failed to read protocol DSL: {error}"),
            path: protocol_path.display().to_string(),
        }]
    })
}

fn print_usage() {
    eprintln!(
        "Usage:\n  spac --version\n  spac validate --config <spac.project.json>\n  spac check-config --architecture <architecture.json>\n  spac check-constraints --constraints <constraints.json>\n  spac check-board-profile --board-profile <board-profile.json>\n  spac check-trace --trace <trace.json>\n  spac import-spac-ae-trace --trace <trace.csv> [--topology <topology.csv>] --name <trace-name> --workload-class <class> --ports <n> --out <dir>\n  spac validate-protocol --protocol <protocol.spac>\n  spac analyze-layout --protocol <protocol.spac> --bus-width <bits> [--out <dir>]\n  spac generate-metadata --protocol <protocol.spac> --bus-width <bits> --out <dir>\n  spac generate-hls-traits --metadata <metadata.json> --out <dir>\n  spac package-hls-csim --metadata <metadata.json> --board-profile <board-profile.json> --out <dir> [--execute --vitis-hls-bin <path> --timeout-seconds <n>]\n  spac parse-hw-report --tool <vitis|vivado> --report <path> --board-profile <board-profile.json> --out <dir>\n  spac accept-hw-report --report <hw_report.json> --constraints <constraints.json> --out <dir>\n  spac package-experiment --run-dir <dir> --board-profile <board-profile.json> --trust-level <level> --out <dir>\n  spac generate-spac-ae-dse-space --ports <n> [--name <name>] --out <dir>\n  spac simulate --architecture <architecture.json> --trace <trace.json> --out <dir>\n  spac dse --space <dse-space.json> --trace <trace.json> --constraints <constraints.json> --out <dir> [--spac-ae-phase2-buffers] [--phase2-top-n <n>] [--min-voq-depth-packets <n>] [--phase2-max-drop-rate <rate>]"
    );
}
