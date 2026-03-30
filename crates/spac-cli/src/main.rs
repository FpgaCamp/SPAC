use spac_core::{validate_project_config_file, Severity};
use spac_core::{Diagnostic, ValidationReport, SUPPORTED_PROJECT_SCHEMA_VERSION};
use spac_dsl::parse_protocol_text;
use spac_layout::analyze_layout;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

const EXIT_OK: u8 = 0;
const EXIT_USAGE: u8 = 64;
const EXIT_VALIDATION: u8 = 2;

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
        Some("validate-protocol") => run_validate_protocol(&args[1..]),
        Some("analyze-layout") => run_analyze_layout(&args[1..]),
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
    let Some(protocol_path) = parse_option_path(args, "--protocol") else {
        eprintln!("usage error: analyze-layout requires --protocol <path>");
        print_usage();
        return Err(EXIT_USAGE);
    };
    let Some(bus_width_text) = parse_option_value(args, "--bus-width") else {
        eprintln!("usage error: analyze-layout requires --bus-width <bits>");
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

    let metadata = match read_protocol(&protocol_path).and_then(|text| parse_protocol_text(&text)) {
        Ok(protocol) => match analyze_layout(&protocol, bus_width_bits) {
            Ok(metadata) => metadata,
            Err(diagnostics) => {
                print_diagnostics(diagnostics)?;
                return Err(EXIT_VALIDATION);
            }
        },
        Err(diagnostics) => {
            print_diagnostics(diagnostics)?;
            return Err(EXIT_VALIDATION);
        }
    };

    let metadata_json = serde_json::to_string_pretty(&metadata).map_err(|error| {
        eprintln!("failed to render metadata model: {error}");
        EXIT_VALIDATION
    })?;

    if let Some(out_dir) = parse_option_path(args, "--out") {
        fs::create_dir_all(&out_dir).map_err(|error| {
            eprintln!(
                "failed to create output directory '{}': {error}",
                out_dir.display()
            );
            EXIT_VALIDATION
        })?;
        fs::write(out_dir.join("metadata.json"), metadata_json.as_bytes()).map_err(|error| {
            eprintln!("failed to write metadata.json: {error}");
            EXIT_VALIDATION
        })?;
    }

    println!("{metadata_json}");
    Ok(())
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
        "Usage:\n  spac --version\n  spac validate --config <spac.project.json>\n  spac validate-protocol --protocol <protocol.spac>\n  spac analyze-layout --protocol <protocol.spac> --bus-width <bits> [--out <dir>]"
    );
}
