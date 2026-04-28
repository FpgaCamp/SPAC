use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub const SUPPORTED_PROJECT_SCHEMA_VERSION: &str = "spac.project.v0";
pub const SUPPORTED_ARCHITECTURE_SCHEMA_VERSION: &str = "spac.architecture.v0";
pub const SUPPORTED_TRACE_SCHEMA_VERSION: &str = "spac.trace.v0";
pub const SUPPORTED_CONSTRAINTS_SCHEMA_VERSION: &str = "spac.constraints.v0";
pub const SUPPORTED_BOARD_PROFILE_SCHEMA_VERSION: &str = "spac.board-profile.v0";
pub const DEFAULT_ARTIFACT_MANIFEST_SCHEMA_VERSION: &str = "spac.artifact-manifest.v0";
pub const METADATA_SCHEMA_VERSION: &str = "spac.metadata.v0";
pub const SIMULATION_RUN_SCHEMA_VERSION: &str = "spac.simulation-run.v0";
pub const DSE_SPACE_SCHEMA_VERSION: &str = "spac.dse-space.v0";
pub const DSE_RESULT_SCHEMA_VERSION: &str = "spac.dse-result.v0";
pub const HLS_TRAITS_RUN_SCHEMA_VERSION: &str = "spac.hls-traits-run.v0";
pub const HLS_CSIM_RUN_SCHEMA_VERSION: &str = "spac.hls-csim-run.v0";
pub const HW_REPORT_SCHEMA_VERSION: &str = "spac.hw-report.v0";
pub const HW_ACCEPTANCE_SCHEMA_VERSION: &str = "spac.hw-acceptance.v0";
pub const EXPERIMENT_RUN_SCHEMA_VERSION: &str = "spac.experiment-run.v0";

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct ProjectConfig {
    pub schema_version: String,
    pub project: ProjectSection,
    pub language_policy: LanguagePolicy,
    pub reproducibility: ReproducibilitySection,
    pub outputs: OutputSection,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct ProjectSection {
    pub name: String,
    pub domain: String,
    pub source_article: String,
    pub selected_mvp: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct LanguagePolicy {
    pub implementation_languages: Vec<String>,
    pub generated_artifacts: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct ReproducibilitySection {
    pub deterministic_seed: u64,
    pub artifact_manifest_schema: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct OutputSection {
    pub directory: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct ArchitectureConfig {
    pub schema_version: String,
    pub name: String,
    pub ports: u16,
    pub bus_width_bits: u32,
    pub forwarding_table: ForwardingTableConfig,
    pub voq: VoqConfig,
    pub scheduler: SchedulerConfig,
    #[serde(default)]
    pub custom_kernels: Vec<CustomKernelConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct ConstraintsConfig {
    pub schema_version: String,
    pub name: String,
    pub board_target: String,
    pub max_lut: u64,
    pub max_ff: u64,
    pub max_bram: u64,
    pub max_dsp: u64,
    pub target_fmax_mhz: f64,
    pub max_p99_latency_ns: f64,
    pub max_packet_drop_rate: f64,
    pub max_initiation_interval: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct BoardProfile {
    pub schema_version: String,
    pub board_id: String,
    pub vendor: String,
    pub board_model: String,
    pub fpga_part: String,
    pub toolchain: ToolchainProfile,
    pub target_clock_mhz: f64,
    pub host_interface: String,
    pub loopback_topology: String,
    pub report_locations: ReportLocations,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct ToolchainProfile {
    pub family: String,
    pub version: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct ReportLocations {
    pub synthesis_summary: String,
    pub timing_summary: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ForwardingTableConfig {
    FullLookup { address_width_bits: u16 },
    MultiBankHash { banks: u16, entries_per_bank: u32 },
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum VoqConfig {
    NByN {
        depth_packets: u32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        per_queue_depth_packets: Option<Vec<u32>>,
    },
    OneBufferPerPort {
        depth_packets: u32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        per_port_depth_packets: Option<Vec<u32>>,
    },
    Shared {
        total_depth_packets: u32,
    },
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SchedulerConfig {
    RoundRobin { pipeline_stages: u16 },
    Islip { iterations: u16 },
    Edrrm { epochs: u16 },
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct CustomKernelConfig {
    pub name: String,
    pub latency_cycles: u32,
    pub resource_class: ResourceClass,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResourceClass {
    Light,
    Medium,
    Heavy,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct ArtifactManifest {
    pub schema_version: String,
    pub run_id: String,
    pub tool_name: String,
    pub tool_version: String,
    pub input_files: Vec<ArtifactFile>,
    pub output_files: Vec<ArtifactFile>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct ArtifactFile {
    pub path: String,
    pub sha256: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct ExperimentRun {
    pub schema_version: String,
    pub run_id: String,
    pub stage: String,
    pub trust_level: String,
    pub status: String,
    pub artifact_manifest_schema: String,
    pub command: String,
    pub board_profile_id: String,
    pub input_files: Vec<ArtifactFile>,
    pub output_files: Vec<ArtifactFile>,
    pub known_limitations: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct ProtocolSpec {
    pub name: String,
    pub fields: Vec<FieldSpec>,
    pub payload: Option<PayloadSpec>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct FieldSpec {
    pub name: String,
    pub bit_width: u16,
    pub semantic: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct PayloadSpec {
    pub kind: PayloadKind,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PayloadKind {
    Bytes,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct MetadataModel {
    pub schema_version: String,
    pub protocol_name: String,
    pub bus_width_bits: u32,
    pub total_header_bits: u64,
    pub total_header_bytes: u64,
    pub fields: Vec<FieldLayout>,
    pub semantic_bindings: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct FieldLayout {
    pub name: String,
    pub semantic: Option<String>,
    pub bit_offset: u64,
    pub bit_width: u16,
    pub byte_offset: u64,
    pub flit_index: u64,
    pub crosses_flit_boundary: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct Diagnostic {
    pub severity: Severity,
    pub code: String,
    pub message: String,
    pub path: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum ValidationReport {
    Ok { schema_version: String },
    Error { diagnostics: Vec<Diagnostic> },
}

impl Diagnostic {
    pub fn error(
        code: impl Into<String>,
        path: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity: Severity::Error,
            code: code.into(),
            message: message.into(),
            path: path.into(),
        }
    }
}

pub fn parse_project_config_text(text: &str) -> Result<ProjectConfig, Vec<Diagnostic>> {
    serde_json::from_str::<ProjectConfig>(text).map_err(|error| {
        vec![Diagnostic::error(
            "SPAC_CONFIG_PARSE",
            "$",
            format!("failed to parse project config JSON: {error}"),
        )]
    })
}

pub fn validate_project_config_text(text: &str) -> Result<ProjectConfig, Vec<Diagnostic>> {
    let config = parse_project_config_text(text)?;
    let diagnostics = validate_project_config(&config);

    if diagnostics.is_empty() {
        Ok(config)
    } else {
        Err(diagnostics)
    }
}

pub fn validate_project_config_file(path: &Path) -> Result<ProjectConfig, Vec<Diagnostic>> {
    let text = fs::read_to_string(path).map_err(|error| {
        vec![Diagnostic::error(
            "SPAC_CONFIG_READ",
            path.display().to_string(),
            format!("failed to read project config: {error}"),
        )]
    })?;

    validate_project_config_text(&text)
}

pub fn validate_project_config(config: &ProjectConfig) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    if config.schema_version != SUPPORTED_PROJECT_SCHEMA_VERSION {
        diagnostics.push(Diagnostic::error(
            "SPAC_SCHEMA_VERSION",
            "$.schema_version",
            format!(
                "unsupported schema version '{}'; expected '{}'",
                config.schema_version, SUPPORTED_PROJECT_SCHEMA_VERSION
            ),
        ));
    }

    if config.project.name.trim().is_empty() {
        diagnostics.push(Diagnostic::error(
            "SPAC_PROJECT_NAME_EMPTY",
            "$.project.name",
            "project name must not be empty",
        ));
    }

    if config.project.domain.trim() != "fpga-network-switch" {
        diagnostics.push(Diagnostic::error(
            "SPAC_PROJECT_DOMAIN",
            "$.project.domain",
            "project domain must be 'fpga-network-switch' for this SPAC implementation",
        ));
    }

    if !matches!(
        config.project.selected_mvp.as_str(),
        "MVP-A" | "MVP-B" | "MVP-C"
    ) {
        diagnostics.push(Diagnostic::error(
            "SPAC_SELECTED_MVP",
            "$.project.selected_mvp",
            "selected_mvp must be one of MVP-A, MVP-B, or MVP-C",
        ));
    }

    validate_language_policy(config, &mut diagnostics);

    if config.reproducibility.artifact_manifest_schema != DEFAULT_ARTIFACT_MANIFEST_SCHEMA_VERSION {
        diagnostics.push(Diagnostic::error(
            "SPAC_ARTIFACT_MANIFEST_SCHEMA",
            "$.reproducibility.artifact_manifest_schema",
            format!(
                "artifact manifest schema must be '{}'",
                DEFAULT_ARTIFACT_MANIFEST_SCHEMA_VERSION
            ),
        ));
    }

    if config.outputs.directory.trim().is_empty() {
        diagnostics.push(Diagnostic::error(
            "SPAC_OUTPUT_DIRECTORY_EMPTY",
            "$.outputs.directory",
            "output directory must not be empty",
        ));
    }

    diagnostics
}

pub fn parse_architecture_config_text(text: &str) -> Result<ArchitectureConfig, Vec<Diagnostic>> {
    serde_json::from_str::<ArchitectureConfig>(text).map_err(|error| {
        vec![Diagnostic::error(
            "SPAC_ARCHITECTURE_PARSE",
            "$",
            format!("failed to parse architecture config JSON: {error}"),
        )]
    })
}

pub fn validate_architecture_config_text(
    text: &str,
) -> Result<ArchitectureConfig, Vec<Diagnostic>> {
    let config = parse_architecture_config_text(text)?;
    let diagnostics = validate_architecture_config(&config);

    if diagnostics.is_empty() {
        Ok(config)
    } else {
        Err(diagnostics)
    }
}

pub fn validate_architecture_config_file(
    path: &Path,
) -> Result<ArchitectureConfig, Vec<Diagnostic>> {
    let text = fs::read_to_string(path).map_err(|error| {
        vec![Diagnostic::error(
            "SPAC_ARCHITECTURE_READ",
            path.display().to_string(),
            format!("failed to read architecture config: {error}"),
        )]
    })?;

    validate_architecture_config_text(&text)
}

pub fn parse_constraints_config_text(text: &str) -> Result<ConstraintsConfig, Vec<Diagnostic>> {
    serde_json::from_str::<ConstraintsConfig>(text).map_err(|error| {
        vec![Diagnostic::error(
            "SPAC_CONSTRAINTS_PARSE",
            "$",
            format!("failed to parse constraints config JSON: {error}"),
        )]
    })
}

pub fn validate_constraints_config_text(text: &str) -> Result<ConstraintsConfig, Vec<Diagnostic>> {
    let config = parse_constraints_config_text(text)?;
    let diagnostics = validate_constraints_config(&config);

    if diagnostics.is_empty() {
        Ok(config)
    } else {
        Err(diagnostics)
    }
}

pub fn validate_constraints_config_file(path: &Path) -> Result<ConstraintsConfig, Vec<Diagnostic>> {
    let text = fs::read_to_string(path).map_err(|error| {
        vec![Diagnostic::error(
            "SPAC_CONSTRAINTS_READ",
            path.display().to_string(),
            format!("failed to read constraints config: {error}"),
        )]
    })?;

    validate_constraints_config_text(&text)
}

pub fn validate_constraints_config(config: &ConstraintsConfig) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    if config.schema_version != SUPPORTED_CONSTRAINTS_SCHEMA_VERSION {
        diagnostics.push(Diagnostic::error(
            "SPAC_CONSTRAINTS_SCHEMA_VERSION",
            "$.schema_version",
            format!(
                "unsupported constraints schema version '{}'; expected '{}'",
                config.schema_version, SUPPORTED_CONSTRAINTS_SCHEMA_VERSION
            ),
        ));
    }

    if config.name.trim().is_empty() {
        diagnostics.push(Diagnostic::error(
            "SPAC_CONSTRAINTS_NAME_EMPTY",
            "$.name",
            "constraints name must not be empty",
        ));
    }

    if config.board_target.trim().is_empty() {
        diagnostics.push(Diagnostic::error(
            "SPAC_CONSTRAINTS_BOARD_TARGET_EMPTY",
            "$.board_target",
            "board_target must not be empty",
        ));
    }

    if !config.target_fmax_mhz.is_finite() || config.target_fmax_mhz <= 0.0 {
        diagnostics.push(Diagnostic::error(
            "SPAC_CONSTRAINTS_TARGET_FMAX",
            "$.target_fmax_mhz",
            "target_fmax_mhz must be finite and greater than zero",
        ));
    }

    if !config.max_p99_latency_ns.is_finite() || config.max_p99_latency_ns <= 0.0 {
        diagnostics.push(Diagnostic::error(
            "SPAC_CONSTRAINTS_MAX_P99_LATENCY",
            "$.max_p99_latency_ns",
            "max_p99_latency_ns must be finite and greater than zero",
        ));
    }

    if !config.max_packet_drop_rate.is_finite()
        || !(0.0..=1.0).contains(&config.max_packet_drop_rate)
    {
        diagnostics.push(Diagnostic::error(
            "SPAC_CONSTRAINTS_MAX_PACKET_DROP_RATE",
            "$.max_packet_drop_rate",
            "max_packet_drop_rate must be finite and within 0..=1",
        ));
    }

    if config.max_initiation_interval == 0 {
        diagnostics.push(Diagnostic::error(
            "SPAC_CONSTRAINTS_MAX_INITIATION_INTERVAL",
            "$.max_initiation_interval",
            "max_initiation_interval must be greater than zero",
        ));
    }

    diagnostics
}

pub fn parse_board_profile_text(text: &str) -> Result<BoardProfile, Vec<Diagnostic>> {
    serde_json::from_str::<BoardProfile>(text).map_err(|error| {
        vec![Diagnostic::error(
            "SPAC_BOARD_PROFILE_PARSE",
            "$",
            format!("failed to parse board profile JSON: {error}"),
        )]
    })
}

pub fn validate_board_profile_text(text: &str) -> Result<BoardProfile, Vec<Diagnostic>> {
    let profile = parse_board_profile_text(text)?;
    let diagnostics = validate_board_profile(&profile);

    if diagnostics.is_empty() {
        Ok(profile)
    } else {
        Err(diagnostics)
    }
}

pub fn validate_board_profile_file(path: &Path) -> Result<BoardProfile, Vec<Diagnostic>> {
    let text = fs::read_to_string(path).map_err(|error| {
        vec![Diagnostic::error(
            "SPAC_BOARD_PROFILE_READ",
            path.display().to_string(),
            format!("failed to read board profile: {error}"),
        )]
    })?;

    validate_board_profile_text(&text)
}

pub fn validate_board_profile(profile: &BoardProfile) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    if profile.schema_version != SUPPORTED_BOARD_PROFILE_SCHEMA_VERSION {
        diagnostics.push(Diagnostic::error(
            "SPAC_BOARD_PROFILE_SCHEMA_VERSION",
            "$.schema_version",
            format!(
                "unsupported board profile schema version '{}'; expected '{}'",
                profile.schema_version, SUPPORTED_BOARD_PROFILE_SCHEMA_VERSION
            ),
        ));
    }

    push_non_empty_diagnostic(
        &mut diagnostics,
        &profile.board_id,
        "$.board_id",
        "SPAC_BOARD_PROFILE_ID_EMPTY",
        "board_id must not be empty",
    );
    push_non_empty_diagnostic(
        &mut diagnostics,
        &profile.vendor,
        "$.vendor",
        "SPAC_BOARD_PROFILE_VENDOR_EMPTY",
        "vendor must not be empty",
    );
    push_non_empty_diagnostic(
        &mut diagnostics,
        &profile.board_model,
        "$.board_model",
        "SPAC_BOARD_PROFILE_MODEL_EMPTY",
        "board_model must not be empty",
    );
    push_non_empty_diagnostic(
        &mut diagnostics,
        &profile.fpga_part,
        "$.fpga_part",
        "SPAC_BOARD_PROFILE_FPGA_PART_EMPTY",
        "fpga_part must not be empty",
    );
    push_non_empty_diagnostic(
        &mut diagnostics,
        &profile.toolchain.family,
        "$.toolchain.family",
        "SPAC_BOARD_PROFILE_TOOLCHAIN_FAMILY_EMPTY",
        "toolchain family must not be empty",
    );
    push_non_empty_diagnostic(
        &mut diagnostics,
        &profile.toolchain.version,
        "$.toolchain.version",
        "SPAC_BOARD_PROFILE_TOOLCHAIN_VERSION_EMPTY",
        "toolchain version must not be empty",
    );
    push_non_empty_diagnostic(
        &mut diagnostics,
        &profile.host_interface,
        "$.host_interface",
        "SPAC_BOARD_PROFILE_HOST_INTERFACE_EMPTY",
        "host_interface must not be empty",
    );
    push_non_empty_diagnostic(
        &mut diagnostics,
        &profile.loopback_topology,
        "$.loopback_topology",
        "SPAC_BOARD_PROFILE_LOOPBACK_TOPOLOGY_EMPTY",
        "loopback_topology must not be empty",
    );
    push_non_empty_diagnostic(
        &mut diagnostics,
        &profile.report_locations.synthesis_summary,
        "$.report_locations.synthesis_summary",
        "SPAC_BOARD_PROFILE_SYNTHESIS_SUMMARY_EMPTY",
        "synthesis_summary report location must not be empty",
    );
    push_non_empty_diagnostic(
        &mut diagnostics,
        &profile.report_locations.timing_summary,
        "$.report_locations.timing_summary",
        "SPAC_BOARD_PROFILE_TIMING_SUMMARY_EMPTY",
        "timing_summary report location must not be empty",
    );

    if !profile.target_clock_mhz.is_finite() || profile.target_clock_mhz <= 0.0 {
        diagnostics.push(Diagnostic::error(
            "SPAC_BOARD_PROFILE_TARGET_CLOCK",
            "$.target_clock_mhz",
            "target_clock_mhz must be finite and greater than zero",
        ));
    }

    diagnostics
}

pub fn validate_architecture_config(config: &ArchitectureConfig) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    if config.schema_version != SUPPORTED_ARCHITECTURE_SCHEMA_VERSION {
        diagnostics.push(Diagnostic::error(
            "SPAC_ARCHITECTURE_SCHEMA_VERSION",
            "$.schema_version",
            format!(
                "unsupported architecture schema version '{}'; expected '{}'",
                config.schema_version, SUPPORTED_ARCHITECTURE_SCHEMA_VERSION
            ),
        ));
    }

    if config.name.trim().is_empty() {
        diagnostics.push(Diagnostic::error(
            "SPAC_ARCHITECTURE_NAME_EMPTY",
            "$.name",
            "architecture name must not be empty",
        ));
    }

    if config.ports < 2 {
        diagnostics.push(Diagnostic::error(
            "SPAC_ARCHITECTURE_PORTS",
            "$.ports",
            "architecture must declare at least two ports",
        ));
    }

    if config.bus_width_bits < 8 || config.bus_width_bits % 8 != 0 {
        diagnostics.push(Diagnostic::error(
            "SPAC_ARCHITECTURE_BUS_WIDTH",
            "$.bus_width_bits",
            "bus_width_bits must be a multiple of 8 and at least 8",
        ));
    }

    match &config.forwarding_table {
        ForwardingTableConfig::FullLookup { address_width_bits } => {
            if *address_width_bits == 0 {
                diagnostics.push(Diagnostic::error(
                    "SPAC_FORWARDING_ADDRESS_WIDTH",
                    "$.forwarding_table.address_width_bits",
                    "full_lookup address_width_bits must be greater than zero",
                ));
            }
        }
        ForwardingTableConfig::MultiBankHash {
            banks,
            entries_per_bank,
        } => {
            if *banks == 0 {
                diagnostics.push(Diagnostic::error(
                    "SPAC_FORWARDING_BANKS",
                    "$.forwarding_table.banks",
                    "multi_bank_hash banks must be greater than zero",
                ));
            }
            if *entries_per_bank == 0 {
                diagnostics.push(Diagnostic::error(
                    "SPAC_FORWARDING_ENTRIES",
                    "$.forwarding_table.entries_per_bank",
                    "multi_bank_hash entries_per_bank must be greater than zero",
                ));
            }
        }
    }

    match &config.voq {
        VoqConfig::NByN {
            depth_packets,
            per_queue_depth_packets,
        } => {
            if *depth_packets == 0 {
                diagnostics.push(Diagnostic::error(
                    "SPAC_VOQ_DEPTH",
                    "$.voq.depth_packets",
                    "n_by_n depth_packets must be greater than zero",
                ));
            }
            if let Some(depths) = per_queue_depth_packets {
                let expected = usize::from(config.ports) * usize::from(config.ports);
                if depths.len() != expected {
                    diagnostics.push(Diagnostic::error(
                        "SPAC_VOQ_PER_QUEUE_DEPTH_COUNT",
                        "$.voq.per_queue_depth_packets",
                        format!(
                            "n_by_n per_queue_depth_packets must contain exactly {expected} entries"
                        ),
                    ));
                }
            }
        }
        VoqConfig::OneBufferPerPort {
            depth_packets,
            per_port_depth_packets,
        } => {
            if *depth_packets == 0 {
                diagnostics.push(Diagnostic::error(
                    "SPAC_VOQ_DEPTH",
                    "$.voq.depth_packets",
                    "one_buffer_per_port depth_packets must be greater than zero",
                ));
            }
            if let Some(depths) = per_port_depth_packets {
                let expected = usize::from(config.ports);
                if depths.len() != expected {
                    diagnostics.push(Diagnostic::error(
                        "SPAC_VOQ_PER_PORT_DEPTH_COUNT",
                        "$.voq.per_port_depth_packets",
                        format!(
                            "one_buffer_per_port per_port_depth_packets must contain exactly {expected} entries"
                        ),
                    ));
                }
            }
        }
        VoqConfig::Shared {
            total_depth_packets,
        } => {
            if *total_depth_packets == 0 {
                diagnostics.push(Diagnostic::error(
                    "SPAC_VOQ_TOTAL_DEPTH",
                    "$.voq.total_depth_packets",
                    "shared total_depth_packets must be greater than zero",
                ));
            }
        }
    }

    match &config.scheduler {
        SchedulerConfig::RoundRobin { pipeline_stages } => {
            if *pipeline_stages == 0 {
                diagnostics.push(Diagnostic::error(
                    "SPAC_SCHEDULER_PIPELINE_STAGES",
                    "$.scheduler.pipeline_stages",
                    "round_robin pipeline_stages must be greater than zero",
                ));
            }
        }
        SchedulerConfig::Islip { iterations } => {
            if *iterations == 0 {
                diagnostics.push(Diagnostic::error(
                    "SPAC_SCHEDULER_ITERATIONS",
                    "$.scheduler.iterations",
                    "islip iterations must be greater than zero",
                ));
            }
        }
        SchedulerConfig::Edrrm { epochs } => {
            if *epochs == 0 {
                diagnostics.push(Diagnostic::error(
                    "SPAC_SCHEDULER_EPOCHS",
                    "$.scheduler.epochs",
                    "edrrm epochs must be greater than zero",
                ));
            }
        }
    }

    for (index, kernel) in config.custom_kernels.iter().enumerate() {
        if kernel.name.trim().is_empty() {
            diagnostics.push(Diagnostic::error(
                "SPAC_CUSTOM_KERNEL_NAME_EMPTY",
                format!("$.custom_kernels[{index}].name"),
                "custom kernel name must not be empty",
            ));
        }
    }

    diagnostics
}

pub fn validate_protocol_semantics(protocol: &ProtocolSpec) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    if protocol.name.trim().is_empty() {
        diagnostics.push(Diagnostic::error(
            "SPAC_PROTOCOL_NAME_EMPTY",
            "$.protocol.name",
            "protocol name must not be empty",
        ));
    }

    if protocol.fields.is_empty() {
        diagnostics.push(Diagnostic::error(
            "SPAC_PROTOCOL_FIELDS_EMPTY",
            "$.protocol.fields",
            "protocol must declare at least one field",
        ));
    }

    let mut field_names = BTreeSet::new();
    let mut routing_key_count = 0;

    for (index, field) in protocol.fields.iter().enumerate() {
        if field.name.trim().is_empty() {
            diagnostics.push(Diagnostic::error(
                "SPAC_FIELD_NAME_EMPTY",
                format!("$.protocol.fields[{index}].name"),
                "field name must not be empty",
            ));
        }

        if field.bit_width == 0 {
            diagnostics.push(Diagnostic::error(
                "SPAC_FIELD_WIDTH_ZERO",
                format!("$.protocol.fields[{index}].bit_width"),
                "field bit width must be greater than zero",
            ));
        }

        if !field_names.insert(field.name.as_str()) {
            diagnostics.push(Diagnostic::error(
                "SPAC_FIELD_DUPLICATE",
                format!("$.protocol.fields[{index}].name"),
                format!("duplicate field '{}'", field.name),
            ));
        }

        if field.semantic.as_deref() == Some("routing_key") {
            routing_key_count += 1;
        }
    }

    match routing_key_count {
        1 => {}
        0 => diagnostics.push(Diagnostic::error(
            "SPAC_ROUTING_KEY_MISSING",
            "$.protocol.fields",
            "exactly one field must use semantic routing_key",
        )),
        _ => diagnostics.push(Diagnostic::error(
            "SPAC_ROUTING_KEY_DUPLICATE",
            "$.protocol.fields",
            "only one field may use semantic routing_key",
        )),
    }

    diagnostics
}

pub fn sha256_bytes_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        hex.push(hex_digit(byte >> 4));
        hex.push(hex_digit(byte & 0x0f));
    }
    hex
}

pub fn sha256_file_hex(path: &Path) -> io::Result<String> {
    let bytes = fs::read(path)?;
    Ok(sha256_bytes_hex(&bytes))
}

pub fn build_artifact_manifest(
    run_id: impl Into<String>,
    tool_name: impl Into<String>,
    tool_version: impl Into<String>,
    input_files: &[PathBuf],
    output_files: &[PathBuf],
) -> io::Result<ArtifactManifest> {
    Ok(ArtifactManifest {
        schema_version: DEFAULT_ARTIFACT_MANIFEST_SCHEMA_VERSION.to_string(),
        run_id: run_id.into(),
        tool_name: tool_name.into(),
        tool_version: tool_version.into(),
        input_files: input_files
            .iter()
            .map(|path| hash_artifact_file(path))
            .collect::<io::Result<Vec<_>>>()?,
        output_files: output_files
            .iter()
            .map(|path| hash_artifact_file(path))
            .collect::<io::Result<Vec<_>>>()?,
    })
}

pub fn write_artifact_manifest_file(path: &Path, manifest: &ArtifactManifest) -> io::Result<()> {
    let text = serde_json::to_string_pretty(manifest)
        .map_err(|error| io::Error::other(format!("failed to serialize manifest: {error}")))?;
    fs::write(path, text)
}

fn validate_language_policy(config: &ProjectConfig, diagnostics: &mut Vec<Diagnostic>) {
    let allowed: BTreeSet<&str> = ["Rust", "TypeScript"].into_iter().collect();

    if config.language_policy.implementation_languages.is_empty() {
        diagnostics.push(Diagnostic::error(
            "SPAC_LANGUAGE_POLICY_EMPTY",
            "$.language_policy.implementation_languages",
            "at least one implementation language must be declared",
        ));
    }

    for (index, language) in config
        .language_policy
        .implementation_languages
        .iter()
        .enumerate()
    {
        if !allowed.contains(language.as_str()) {
            diagnostics.push(Diagnostic::error(
                "SPAC_LANGUAGE_UNSUPPORTED",
                format!("$.language_policy.implementation_languages[{index}]"),
                format!(
                    "implementation language '{language}' is not allowed; use Rust or TypeScript"
                ),
            ));
        }
    }
}

fn push_non_empty_diagnostic(
    diagnostics: &mut Vec<Diagnostic>,
    value: &str,
    path: &str,
    code: &str,
    message: &str,
) {
    if value.trim().is_empty() {
        diagnostics.push(Diagnostic::error(code, path, message));
    }
}

pub fn project_root_from_manifest_dir(manifest_dir: &Path) -> PathBuf {
    manifest_dir
        .ancestors()
        .nth(2)
        .unwrap_or(manifest_dir)
        .to_path_buf()
}

fn hash_artifact_file(path: &Path) -> io::Result<ArtifactFile> {
    Ok(ArtifactFile {
        path: path.display().to_string(),
        sha256: sha256_file_hex(path)?,
    })
}

fn hex_digit(value: u8) -> char {
    match value {
        0..=9 => char::from(b'0' + value),
        10..=15 => char::from(b'a' + (value - 10)),
        _ => unreachable!("nibble out of range"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsStr;

    const VALID_CONFIG: &str = r#"{
      "schema_version": "spac.project.v0",
      "project": {
        "name": "spac",
        "domain": "fpga-network-switch",
        "source_article": "https://arxiv.org/html/2604.21881v1",
        "selected_mvp": "MVP-A"
      },
      "language_policy": {
        "implementation_languages": ["Rust", "TypeScript"],
        "generated_artifacts": ["HLS C++ header"]
      },
      "reproducibility": {
        "deterministic_seed": 260421881,
        "artifact_manifest_schema": "spac.artifact-manifest.v0"
      },
      "outputs": {
        "directory": "out"
      }
    }"#;

    const VALID_ARCHITECTURE: &str = r#"{
      "schema_version": "spac.architecture.v0",
      "name": "hft_8p_full_lookup_rr",
      "ports": 8,
      "bus_width_bits": 256,
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
    }"#;

    const VALID_CONSTRAINTS: &str = r#"{
      "schema_version": "spac.constraints.v0",
      "name": "paper_aligned_u45n_350mhz",
      "board_target": "amd-alveo-u45n",
      "max_lut": 100000,
      "max_ff": 100000,
      "max_bram": 300,
      "max_dsp": 400,
      "target_fmax_mhz": 350,
      "max_p99_latency_ns": 1000,
      "max_packet_drop_rate": 0.000001,
      "max_initiation_interval": 1
    }"#;

    const VALID_BOARD_PROFILE: &str = r#"{
      "schema_version": "spac.board-profile.v0",
      "board_id": "amd-alveo-u45n",
      "vendor": "AMD",
      "board_model": "Alveo U45N",
      "fpga_part": "xcu26-vsva1365-2LV-e",
      "toolchain": {
        "family": "Vitis HLS",
        "version": "2023.2"
      },
      "target_clock_mhz": 350,
      "host_interface": "PCIe",
      "loopback_topology": "host-fpga-host loopback or external packet generator",
      "report_locations": {
        "synthesis_summary": "reports/post_synthesis.json",
        "timing_summary": "reports/timing_summary.rpt"
      }
    }"#;

    #[test]
    fn valid_config_passes() {
        let parsed = validate_project_config_text(VALID_CONFIG).expect("valid config");

        assert_eq!(parsed.schema_version, SUPPORTED_PROJECT_SCHEMA_VERSION);
        assert_eq!(parsed.project.selected_mvp, "MVP-A");
    }

    #[test]
    fn unsupported_language_is_rejected() {
        let invalid = VALID_CONFIG.replace("\"TypeScript\"", "\"C++\"");
        let diagnostics = validate_project_config_text(&invalid).expect_err("invalid config");

        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "SPAC_LANGUAGE_UNSUPPORTED"));
    }

    #[test]
    fn unsupported_schema_version_is_rejected() {
        let invalid = VALID_CONFIG.replace("spac.project.v0", "spac.project.v99");
        let diagnostics = validate_project_config_text(&invalid).expect_err("invalid config");

        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "SPAC_SCHEMA_VERSION"));
    }

    #[test]
    fn protocol_requires_exactly_one_routing_key() {
        let protocol = ProtocolSpec {
            name: "bad".to_string(),
            fields: vec![FieldSpec {
                name: "dst".to_string(),
                bit_width: 8,
                semantic: None,
            }],
            payload: None,
        };

        let diagnostics = validate_protocol_semantics(&protocol);

        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "SPAC_ROUTING_KEY_MISSING"));
    }

    #[test]
    fn valid_architecture_config_passes() {
        let parsed = validate_architecture_config_text(VALID_ARCHITECTURE)
            .expect("valid architecture config");

        assert_eq!(parsed.schema_version, SUPPORTED_ARCHITECTURE_SCHEMA_VERSION);
        assert_eq!(parsed.ports, 8);
    }

    #[test]
    fn invalid_architecture_bus_width_is_rejected() {
        let invalid =
            VALID_ARCHITECTURE.replace("\"bus_width_bits\": 256", "\"bus_width_bits\": 20");
        let diagnostics =
            validate_architecture_config_text(&invalid).expect_err("invalid architecture config");

        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "SPAC_ARCHITECTURE_BUS_WIDTH"));
    }

    #[test]
    fn architecture_accepts_phase2_per_queue_depths() {
        let with_depths = VALID_ARCHITECTURE
            .replace(
                "\"depth_packets\": 64",
                "\"depth_packets\": 64,\n        \"per_queue_depth_packets\": [1, 2, 3, 4]",
            )
            .replace("\"ports\": 8", "\"ports\": 2");

        let parsed =
            validate_architecture_config_text(&with_depths).expect("valid per-queue depths");

        assert!(matches!(
            parsed.voq,
            VoqConfig::NByN {
                per_queue_depth_packets: Some(_),
                ..
            }
        ));
    }

    #[test]
    fn architecture_rejects_wrong_per_queue_depth_count() {
        let invalid = VALID_ARCHITECTURE
            .replace(
                "\"depth_packets\": 64",
                "\"depth_packets\": 64,\n        \"per_queue_depth_packets\": [1, 2, 3]",
            )
            .replace("\"ports\": 8", "\"ports\": 2");

        let diagnostics =
            validate_architecture_config_text(&invalid).expect_err("invalid per-queue depths");

        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "SPAC_VOQ_PER_QUEUE_DEPTH_COUNT"));
    }

    #[test]
    fn valid_constraints_config_passes() {
        let parsed =
            validate_constraints_config_text(VALID_CONSTRAINTS).expect("valid constraints config");

        assert_eq!(parsed.schema_version, SUPPORTED_CONSTRAINTS_SCHEMA_VERSION);
        assert_eq!(parsed.board_target, "amd-alveo-u45n");
    }

    #[test]
    fn invalid_constraints_drop_rate_is_rejected() {
        let invalid = VALID_CONSTRAINTS.replace(
            "\"max_packet_drop_rate\": 0.000001",
            "\"max_packet_drop_rate\": 1.5",
        );
        let diagnostics =
            validate_constraints_config_text(&invalid).expect_err("invalid constraints config");

        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "SPAC_CONSTRAINTS_MAX_PACKET_DROP_RATE"));
    }

    #[test]
    fn invalid_constraints_target_fmax_is_rejected() {
        let invalid =
            VALID_CONSTRAINTS.replace("\"target_fmax_mhz\": 350", "\"target_fmax_mhz\": 0");
        let diagnostics =
            validate_constraints_config_text(&invalid).expect_err("invalid constraints config");

        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "SPAC_CONSTRAINTS_TARGET_FMAX"));
    }

    #[test]
    fn valid_board_profile_passes() {
        let parsed = validate_board_profile_text(VALID_BOARD_PROFILE).expect("valid board profile");

        assert_eq!(
            parsed.schema_version,
            SUPPORTED_BOARD_PROFILE_SCHEMA_VERSION
        );
        assert_eq!(parsed.board_id, "amd-alveo-u45n");
    }

    #[test]
    fn invalid_board_profile_target_clock_is_rejected() {
        let invalid =
            VALID_BOARD_PROFILE.replace("\"target_clock_mhz\": 350", "\"target_clock_mhz\": 0");
        let diagnostics = validate_board_profile_text(&invalid).expect_err("invalid board profile");

        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "SPAC_BOARD_PROFILE_TARGET_CLOCK"));
    }

    #[test]
    fn invalid_board_profile_empty_toolchain_version_is_rejected() {
        let invalid = VALID_BOARD_PROFILE.replace("\"version\": \"2023.2\"", "\"version\": \"\"");
        let diagnostics = validate_board_profile_text(&invalid).expect_err("invalid board profile");

        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "SPAC_BOARD_PROFILE_TOOLCHAIN_VERSION_EMPTY"));
    }

    #[test]
    fn artifact_manifest_hashes_files() {
        let temp_root =
            std::env::temp_dir().join(format!("spac-core-manifest-{}", std::process::id()));
        let _ = fs::create_dir_all(&temp_root);

        let input_path = temp_root.join("input.txt");
        let output_path = temp_root.join("output.txt");
        fs::write(&input_path, b"input").expect("write input");
        fs::write(&output_path, b"output").expect("write output");

        let manifest = build_artifact_manifest(
            "test-run",
            "spac",
            "0.1.0",
            std::slice::from_ref(&input_path),
            std::slice::from_ref(&output_path),
        )
        .expect("build manifest");

        assert_eq!(
            manifest.schema_version,
            DEFAULT_ARTIFACT_MANIFEST_SCHEMA_VERSION
        );
        assert_eq!(
            manifest.input_files[0].sha256,
            sha256_file_hex(&input_path).unwrap()
        );
        assert_eq!(
            manifest.output_files[0].sha256,
            sha256_file_hex(&output_path).unwrap()
        );
    }

    #[test]
    fn implementation_language_policy_allows_only_rust_typescript_sources() {
        let workspace_root = project_root_from_manifest_dir(Path::new(env!("CARGO_MANIFEST_DIR")));
        let mut forbidden = Vec::new();

        collect_forbidden_source_files(&workspace_root, &mut forbidden);

        assert!(
            forbidden.is_empty(),
            "forbidden implementation-language files found: {forbidden:?}"
        );
    }

    fn collect_forbidden_source_files(dir: &Path, forbidden: &mut Vec<PathBuf>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };

        for entry in entries.flatten() {
            let path = entry.path();
            let file_name = path.file_name().and_then(OsStr::to_str).unwrap_or_default();

            if file_name == ".git" || file_name == "target" || file_name == "out" {
                continue;
            }

            if path.is_dir() {
                if file_name.starts_with('.') {
                    continue;
                }

                collect_forbidden_source_files(&path, forbidden);
                continue;
            }

            if is_forbidden_implementation_source(&path) {
                forbidden.push(path);
            }
        }
    }

    fn is_forbidden_implementation_source(path: &Path) -> bool {
        matches!(
            path.extension().and_then(OsStr::to_str),
            Some(
                "c" | "cc"
                    | "cpp"
                    | "cxx"
                    | "h"
                    | "hh"
                    | "hpp"
                    | "py"
                    | "go"
                    | "java"
                    | "kt"
                    | "scala"
                    | "rb"
                    | "php"
                    | "sh"
                    | "tcl"
                    | "sv"
                    | "v"
                    | "vhd"
                    | "vhdl"
            )
        )
    }
}
