use serde::{Deserialize, Serialize};
use spac_core::{
    validate_architecture_config, validate_constraints_config, ArchitectureConfig,
    ConstraintsConfig, Diagnostic, ForwardingTableConfig, SchedulerConfig, VoqConfig,
    DSE_RESULT_SCHEMA_VERSION, DSE_SPACE_SCHEMA_VERSION,
};
use spac_sim::{run_simulation, SimulationMetrics};
use spac_trace::{validate_trace, TraceSpec};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct DseSpace {
    pub schema_version: String,
    pub name: String,
    pub candidates: Vec<DseCandidate>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct DseCandidate {
    pub name: String,
    pub architecture: ArchitectureConfig,
    pub resource_estimate: ResourceEstimate,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct ResourceEstimate {
    pub lut: u64,
    pub ff: u64,
    pub bram: u64,
    pub dsp: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct DseResult {
    pub schema_version: String,
    pub space_name: String,
    pub trace_name: String,
    pub constraints_name: String,
    pub trust_level: String,
    pub frontier: Vec<String>,
    pub candidates: Vec<DseCandidateResult>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct DseCandidateResult {
    pub name: String,
    pub architecture_name: String,
    pub phase: u8,
    pub status: DseCandidateStatus,
    pub dominated_by: Option<String>,
    pub constraint_failures: Vec<String>,
    pub resource_estimate: ResourceEstimate,
    pub metrics: SimulationMetrics,
    pub optimized_from: Option<String>,
    pub buffer_optimization: Option<BufferOptimization>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct BufferOptimization {
    pub min_depth_packets: u32,
    pub original_buffer_memory_packets: u64,
    pub optimized_buffer_memory_packets: u64,
    pub packet_depth_saving_ratio: f64,
    pub peak_voq_occupancy_packets: Vec<u64>,
    pub optimized_depth_packets: Vec<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Phase2BufferOptions {
    pub top_n: usize,
    pub min_depth_packets: u32,
    pub max_drop_rate: f64,
}

impl Default for Phase2BufferOptions {
    fn default() -> Self {
        Self {
            top_n: 1,
            min_depth_packets: 64,
            max_drop_rate: 0.01,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DseCandidateStatus {
    Frontier,
    Dominated,
    ConstraintRejected,
}

pub fn parse_dse_space_text(text: &str) -> Result<DseSpace, Vec<Diagnostic>> {
    serde_json::from_str::<DseSpace>(text).map_err(|error| {
        vec![Diagnostic::error(
            "SPAC_DSE_SPACE_PARSE",
            "$",
            format!("failed to parse DSE space JSON: {error}"),
        )]
    })
}

pub fn validate_dse_space_text(text: &str) -> Result<DseSpace, Vec<Diagnostic>> {
    let space = parse_dse_space_text(text)?;
    let diagnostics = validate_dse_space(&space);

    if diagnostics.is_empty() {
        Ok(space)
    } else {
        Err(diagnostics)
    }
}

pub fn validate_dse_space_file(path: &Path) -> Result<DseSpace, Vec<Diagnostic>> {
    let text = fs::read_to_string(path).map_err(|error| {
        vec![Diagnostic::error(
            "SPAC_DSE_SPACE_READ",
            path.display().to_string(),
            format!("failed to read DSE space: {error}"),
        )]
    })?;

    validate_dse_space_text(&text)
}

pub fn validate_dse_space(space: &DseSpace) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    if space.schema_version != DSE_SPACE_SCHEMA_VERSION {
        diagnostics.push(Diagnostic::error(
            "SPAC_DSE_SPACE_SCHEMA_VERSION",
            "$.schema_version",
            format!(
                "unsupported DSE space schema version '{}'; expected '{}'",
                space.schema_version, DSE_SPACE_SCHEMA_VERSION
            ),
        ));
    }

    if space.name.trim().is_empty() {
        diagnostics.push(Diagnostic::error(
            "SPAC_DSE_SPACE_NAME_EMPTY",
            "$.name",
            "DSE space name must not be empty",
        ));
    }

    if space.candidates.is_empty() {
        diagnostics.push(Diagnostic::error(
            "SPAC_DSE_SPACE_CANDIDATES_EMPTY",
            "$.candidates",
            "DSE space must contain at least one candidate",
        ));
    }

    let mut names = std::collections::BTreeSet::new();
    for (index, candidate) in space.candidates.iter().enumerate() {
        if candidate.name.trim().is_empty() {
            diagnostics.push(Diagnostic::error(
                "SPAC_DSE_CANDIDATE_NAME_EMPTY",
                format!("$.candidates[{index}].name"),
                "candidate name must not be empty",
            ));
        }

        if !names.insert(candidate.name.as_str()) {
            diagnostics.push(Diagnostic::error(
                "SPAC_DSE_CANDIDATE_DUPLICATE",
                format!("$.candidates[{index}].name"),
                format!("duplicate candidate '{}'", candidate.name),
            ));
        }

        diagnostics.extend(validate_architecture_config(&candidate.architecture));
    }

    diagnostics
}

pub fn run_dse(
    space: &DseSpace,
    trace: &TraceSpec,
    constraints: &ConstraintsConfig,
) -> Result<DseResult, Vec<Diagnostic>> {
    let mut diagnostics = validate_dse_space(space);
    diagnostics.extend(validate_trace(trace));
    diagnostics.extend(validate_constraints_config(constraints));

    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }

    let mut candidates = evaluate_phase1_candidates(space, trace, constraints)?;
    rank_candidates(&mut candidates);

    Ok(DseResult {
        schema_version: DSE_RESULT_SCHEMA_VERSION.to_string(),
        space_name: space.name.clone(),
        trace_name: trace.name.clone(),
        constraints_name: constraints.name.clone(),
        trust_level: "software_model".to_string(),
        frontier: frontier_names(&candidates),
        candidates,
        warnings: vec![
            "software_model trust level: DSE uses the deterministic simulator only; no HLS synthesis, post-synthesis report, ns-3 run, or FPGA measurement was performed".to_string(),
        ],
    })
}

pub fn run_spac_ae_phase2_buffer_dse(
    space: &DseSpace,
    trace: &TraceSpec,
    constraints: &ConstraintsConfig,
    options: Phase2BufferOptions,
) -> Result<DseResult, Vec<Diagnostic>> {
    let mut diagnostics = validate_dse_space(space);
    diagnostics.extend(validate_trace(trace));
    diagnostics.extend(validate_constraints_config(constraints));
    diagnostics.extend(validate_phase2_options(options));

    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }

    let mut candidates = evaluate_phase1_candidates(space, trace, constraints)?;
    rank_candidates(&mut candidates);

    let selected_names = select_phase2_candidate_names(&candidates, options.top_n);
    for source_name in selected_names {
        let Some(source_candidate) = space
            .candidates
            .iter()
            .find(|candidate| candidate.name == source_name)
        else {
            continue;
        };
        let Some(source_result) = candidates
            .iter()
            .find(|candidate| candidate.name == source_name)
            .cloned()
        else {
            continue;
        };
        let Some((optimized_architecture, optimization)) = optimize_buffer_depths(
            &source_candidate.architecture,
            &source_result.metrics.peak_voq_occupancy_packets,
            options.min_depth_packets,
        ) else {
            continue;
        };

        let report = run_simulation(&optimized_architecture, trace)?;
        let resource_estimate = estimate_spac_ae_heuristic_v0(&optimized_architecture);
        let constraint_failures =
            constraint_failures(&resource_estimate, &report.metrics, constraints);
        let mut effective_failures = constraint_failures;
        if report.metrics.drop_rate > options.max_drop_rate
            && !effective_failures
                .iter()
                .any(|failure| failure == "phase2_max_drop_rate")
        {
            effective_failures.push("phase2_max_drop_rate".to_string());
        }
        let status = if effective_failures.is_empty() {
            DseCandidateStatus::Frontier
        } else {
            DseCandidateStatus::ConstraintRejected
        };
        let candidate_name = format!("{}_phase2_buffer_opt", source_candidate.name);

        candidates.push(DseCandidateResult {
            name: candidate_name,
            architecture_name: optimized_architecture.name,
            phase: 2,
            status,
            dominated_by: None,
            constraint_failures: effective_failures,
            resource_estimate,
            metrics: report.metrics,
            optimized_from: Some(source_candidate.name.clone()),
            buffer_optimization: Some(optimization),
        });
    }

    rank_candidates(&mut candidates);

    Ok(DseResult {
        schema_version: DSE_RESULT_SCHEMA_VERSION.to_string(),
        space_name: space.name.clone(),
        trace_name: trace.name.clone(),
        constraints_name: constraints.name.clone(),
        trust_level: "software_model".to_string(),
        frontier: frontier_names(&candidates),
        candidates,
        warnings: vec![
            "software_model trust level: DSE uses the deterministic simulator only; no HLS synthesis, post-synthesis report, ns-3 run, or FPGA measurement was performed".to_string(),
            "SPAC-AE phase-2 buffer optimization uses simulator peak VOQ occupancy as a packet-depth proxy; it is not hardware back-annotated BRAM sizing".to_string(),
        ],
    })
}

fn evaluate_phase1_candidates(
    space: &DseSpace,
    trace: &TraceSpec,
    constraints: &ConstraintsConfig,
) -> Result<Vec<DseCandidateResult>, Vec<Diagnostic>> {
    let mut candidates = Vec::with_capacity(space.candidates.len());

    for candidate in &space.candidates {
        let report = run_simulation(&candidate.architecture, trace)?;
        let constraint_failures =
            constraint_failures(&candidate.resource_estimate, &report.metrics, constraints);
        let status = if constraint_failures.is_empty() {
            DseCandidateStatus::Frontier
        } else {
            DseCandidateStatus::ConstraintRejected
        };

        candidates.push(DseCandidateResult {
            name: candidate.name.clone(),
            architecture_name: candidate.architecture.name.clone(),
            phase: 1,
            status,
            dominated_by: None,
            constraint_failures,
            resource_estimate: candidate.resource_estimate.clone(),
            metrics: report.metrics,
            optimized_from: None,
            buffer_optimization: None,
        });
    }

    Ok(candidates)
}

fn rank_candidates(candidates: &mut [DseCandidateResult]) {
    for candidate in candidates.iter_mut() {
        candidate.dominated_by = None;
        if candidate.constraint_failures.is_empty() {
            candidate.status = DseCandidateStatus::Frontier;
        } else {
            candidate.status = DseCandidateStatus::ConstraintRejected;
        }
    }

    mark_dominated_candidates(candidates);
}

fn frontier_names(candidates: &[DseCandidateResult]) -> Vec<String> {
    candidates
        .iter()
        .filter(|candidate| candidate.status == DseCandidateStatus::Frontier)
        .map(|candidate| candidate.name.clone())
        .collect()
}

fn mark_dominated_candidates(candidates: &mut [DseCandidateResult]) {
    let snapshot = candidates.to_vec();

    for candidate in candidates {
        if candidate.status != DseCandidateStatus::Frontier {
            continue;
        }

        if let Some(dominator) = snapshot.iter().find(|other| {
            other.name != candidate.name
                && other.status == DseCandidateStatus::Frontier
                && dominates(other, candidate)
        }) {
            candidate.status = DseCandidateStatus::Dominated;
            candidate.dominated_by = Some(dominator.name.clone());
        }
    }
}

fn dominates(left: &DseCandidateResult, right: &DseCandidateResult) -> bool {
    let no_worse = left.metrics.drop_rate <= right.metrics.drop_rate
        && left.metrics.latency_ns.p99 <= right.metrics.latency_ns.p99
        && left.resource_estimate.lut <= right.resource_estimate.lut
        && left.resource_estimate.ff <= right.resource_estimate.ff
        && left.resource_estimate.bram <= right.resource_estimate.bram
        && left.resource_estimate.dsp <= right.resource_estimate.dsp;
    let strictly_better = left.metrics.drop_rate < right.metrics.drop_rate
        || left.metrics.latency_ns.p99 < right.metrics.latency_ns.p99
        || left.resource_estimate.lut < right.resource_estimate.lut
        || left.resource_estimate.ff < right.resource_estimate.ff
        || left.resource_estimate.bram < right.resource_estimate.bram
        || left.resource_estimate.dsp < right.resource_estimate.dsp;

    no_worse && strictly_better
}

fn validate_phase2_options(options: Phase2BufferOptions) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    if options.top_n == 0 {
        diagnostics.push(Diagnostic::error(
            "SPAC_DSE_PHASE2_TOP_N",
            "--phase2-top-n",
            "phase-2 top_n must be greater than zero",
        ));
    }
    if options.min_depth_packets == 0 {
        diagnostics.push(Diagnostic::error(
            "SPAC_DSE_PHASE2_MIN_DEPTH",
            "--min-voq-depth-packets",
            "phase-2 minimum VOQ depth must be greater than zero",
        ));
    }
    if !(0.0..=1.0).contains(&options.max_drop_rate) {
        diagnostics.push(Diagnostic::error(
            "SPAC_DSE_PHASE2_MAX_DROP_RATE",
            "--phase2-max-drop-rate",
            "phase-2 max drop rate must be between 0.0 and 1.0",
        ));
    }

    diagnostics
}

fn select_phase2_candidate_names(candidates: &[DseCandidateResult], top_n: usize) -> Vec<String> {
    let mut selected = candidates
        .iter()
        .filter(|candidate| {
            candidate.status == DseCandidateStatus::Frontier
                && candidate.metrics.packets_dropped == 0
        })
        .collect::<Vec<_>>();

    selected.sort_by(|left, right| {
        left.metrics
            .latency_ns
            .p99
            .cmp(&right.metrics.latency_ns.p99)
            .then_with(|| {
                right
                    .metrics
                    .throughput_gbps
                    .total_cmp(&left.metrics.throughput_gbps)
            })
            .then_with(|| left.name.cmp(&right.name))
    });

    selected
        .into_iter()
        .take(top_n)
        .map(|candidate| candidate.name.clone())
        .collect()
}

fn optimize_buffer_depths(
    architecture: &ArchitectureConfig,
    peaks: &[u64],
    min_depth_packets: u32,
) -> Option<(ArchitectureConfig, BufferOptimization)> {
    let ports = usize::from(architecture.ports);
    let (optimized_voq, optimized_depth_packets) = match &architecture.voq {
        VoqConfig::NByN { .. } => {
            let mut depths = Vec::with_capacity(ports * ports);
            for src in 0..ports {
                for dst in 0..ports {
                    if src == dst {
                        depths.push(0);
                    } else {
                        let peak = peaks.get(src * ports + dst).copied().unwrap_or(0);
                        depths.push(next_power_of_two_at_least(peak, min_depth_packets));
                    }
                }
            }
            (
                VoqConfig::NByN {
                    depth_packets: min_depth_packets,
                    per_queue_depth_packets: Some(depths.clone()),
                },
                depths,
            )
        }
        VoqConfig::OneBufferPerPort { .. } => {
            let depths = (0..ports)
                .map(|dst| {
                    let peak = peaks.get(dst).copied().unwrap_or(0);
                    next_power_of_two_at_least(peak, min_depth_packets)
                })
                .collect::<Vec<_>>();
            (
                VoqConfig::OneBufferPerPort {
                    depth_packets: min_depth_packets,
                    per_port_depth_packets: Some(depths.clone()),
                },
                depths,
            )
        }
        VoqConfig::Shared { .. } => return None,
    };

    let original_buffer_memory_packets =
        buffer_memory_packets(&architecture.voq, architecture.ports);
    let optimized_buffer_memory_packets = optimized_depth_packets
        .iter()
        .map(|depth| u64::from(*depth))
        .sum::<u64>();
    let packet_depth_saving_ratio = if original_buffer_memory_packets == 0 {
        0.0
    } else {
        (original_buffer_memory_packets as f64 - optimized_buffer_memory_packets as f64)
            / original_buffer_memory_packets as f64
    };
    let mut optimized_architecture = architecture.clone();
    optimized_architecture.name = format!("{}_phase2_buffer_opt", architecture.name);
    optimized_architecture.voq = optimized_voq;

    Some((
        optimized_architecture,
        BufferOptimization {
            min_depth_packets,
            original_buffer_memory_packets,
            optimized_buffer_memory_packets,
            packet_depth_saving_ratio,
            peak_voq_occupancy_packets: peaks.to_vec(),
            optimized_depth_packets,
        },
    ))
}

fn next_power_of_two_at_least(value: u64, minimum: u32) -> u32 {
    let target = value.max(u64::from(minimum));
    let mut power = 1_u64;
    while power < target {
        power = power.saturating_mul(2);
        if power >= u64::from(u32::MAX) {
            return u32::MAX;
        }
    }
    power as u32
}

fn buffer_memory_packets(voq: &VoqConfig, ports: u16) -> u64 {
    match voq {
        VoqConfig::NByN {
            depth_packets,
            per_queue_depth_packets,
        } => per_queue_depth_packets
            .as_ref()
            .map(|depths| depths.iter().map(|depth| u64::from(*depth)).sum::<u64>())
            .unwrap_or_else(|| {
                u64::from(*depth_packets) * u64::from(ports) * u64::from(ports.saturating_sub(1))
            }),
        VoqConfig::OneBufferPerPort {
            depth_packets,
            per_port_depth_packets,
        } => per_port_depth_packets
            .as_ref()
            .map(|depths| depths.iter().map(|depth| u64::from(*depth)).sum::<u64>())
            .unwrap_or_else(|| u64::from(*depth_packets) * u64::from(ports)),
        VoqConfig::Shared {
            total_depth_packets,
        } => u64::from(*total_depth_packets),
    }
}

fn constraint_failures(
    resource: &ResourceEstimate,
    metrics: &SimulationMetrics,
    constraints: &ConstraintsConfig,
) -> Vec<String> {
    let mut failures = Vec::new();

    if resource.lut > constraints.max_lut {
        failures.push("max_lut".to_string());
    }
    if resource.ff > constraints.max_ff {
        failures.push("max_ff".to_string());
    }
    if resource.bram > constraints.max_bram {
        failures.push("max_bram".to_string());
    }
    if resource.dsp > constraints.max_dsp {
        failures.push("max_dsp".to_string());
    }
    if (metrics.latency_ns.p99 as f64) > constraints.max_p99_latency_ns {
        failures.push("max_p99_latency_ns".to_string());
    }
    if metrics.drop_rate > constraints.max_packet_drop_rate {
        failures.push("max_packet_drop_rate".to_string());
    }
    if metrics.estimated_initiation_interval > constraints.max_initiation_interval {
        failures.push("max_initiation_interval".to_string());
    }

    failures
}

pub fn generate_spac_ae_dse_space(name: impl Into<String>, ports: u16) -> DseSpace {
    let data_widths = [32_u32, 64, 128, 256, 512, 640];
    let mut candidates = Vec::new();
    for forwarding_table in [
        ForwardingTableConfig::FullLookup {
            address_width_bits: 8,
        },
        ForwardingTableConfig::MultiBankHash {
            banks: ports,
            entries_per_bank: 128,
        },
    ] {
        for voq_kind in [AeVoqKind::OneBufferPerPort, AeVoqKind::NBuffersPerPort] {
            for scheduler in [
                SchedulerConfig::RoundRobin { pipeline_stages: 1 },
                SchedulerConfig::Islip { iterations: 1 },
            ] {
                for bus_width_bits in data_widths {
                    let depth_packets = large_voq_depth_packets(bus_width_bits);
                    let voq = match voq_kind {
                        AeVoqKind::OneBufferPerPort => VoqConfig::OneBufferPerPort {
                            depth_packets,
                            per_port_depth_packets: None,
                        },
                        AeVoqKind::NBuffersPerPort => VoqConfig::NByN {
                            depth_packets,
                            per_queue_depth_packets: None,
                        },
                    };
                    let architecture = ArchitectureConfig {
                        schema_version: "spac.architecture.v0".to_string(),
                        name: ae_candidate_name(
                            &forwarding_table,
                            &voq,
                            &scheduler,
                            bus_width_bits,
                        ),
                        ports,
                        bus_width_bits,
                        forwarding_table: forwarding_table.clone(),
                        voq,
                        scheduler: scheduler.clone(),
                        custom_kernels: Vec::new(),
                    };
                    candidates.push(DseCandidate {
                        name: architecture.name.clone(),
                        resource_estimate: estimate_spac_ae_heuristic_v0(&architecture),
                        architecture,
                    });
                }
            }
        }
    }

    DseSpace {
        schema_version: DSE_SPACE_SCHEMA_VERSION.to_string(),
        name: name.into(),
        candidates,
    }
}

pub fn estimate_spac_ae_heuristic_v0(architecture: &ArchitectureConfig) -> ResourceEstimate {
    let axis_width = architecture.bus_width_bits;
    let (hash_lut, hash_ff, hash_bram) = estimate_hash_resources(
        &architecture.forwarding_table,
        architecture.ports,
        axis_width,
    );
    let (rx_lut, rx_ff, rx_bram) = estimate_rx_resources(architecture.ports, axis_width);
    let (sched_lut, sched_ff, _sched_bram) = estimate_scheduler_resources(
        &architecture.scheduler,
        &architecture.voq,
        architecture.ports,
        axis_width,
    );
    let buffer_bram = estimate_buffer_bram(&architecture.voq, architecture.ports, axis_width);

    ResourceEstimate {
        lut: nonnegative_trunc(hash_lut) + nonnegative_trunc(rx_lut) + nonnegative_trunc(sched_lut),
        ff: nonnegative_trunc(hash_ff) + nonnegative_trunc(rx_ff) + nonnegative_trunc(sched_ff),
        bram: u64::from(hash_bram) + u64::from(rx_bram) + u64::from(buffer_bram),
        dsp: 0,
    }
}

#[derive(Clone, Copy)]
enum AeVoqKind {
    OneBufferPerPort,
    NBuffersPerPort,
}

fn large_voq_depth_packets(bus_width_bits: u32) -> u32 {
    let bytes_per_packet = (bus_width_bits / 8).max(1);
    (1_048_576_u32 / bytes_per_packet).max(1)
}

fn ae_candidate_name(
    forwarding_table: &ForwardingTableConfig,
    voq: &VoqConfig,
    scheduler: &SchedulerConfig,
    bus_width_bits: u32,
) -> String {
    format!(
        "ae_{}_{}_{}_w{}",
        forwarding_name(forwarding_table),
        voq_name(voq),
        scheduler_name(scheduler),
        bus_width_bits
    )
}

fn forwarding_name(forwarding_table: &ForwardingTableConfig) -> &'static str {
    match forwarding_table {
        ForwardingTableConfig::FullLookup { .. } => "full_lookup",
        ForwardingTableConfig::MultiBankHash { .. } => "multi_bank_hash",
    }
}

fn voq_name(voq: &VoqConfig) -> &'static str {
    match voq {
        VoqConfig::NByN { .. } => "n_by_n",
        VoqConfig::OneBufferPerPort { .. } => "one_buffer_per_port",
        VoqConfig::Shared { .. } => "shared",
    }
}

fn scheduler_name(scheduler: &SchedulerConfig) -> &'static str {
    match scheduler {
        SchedulerConfig::RoundRobin { .. } => "round_robin",
        SchedulerConfig::Islip { .. } => "islip",
        SchedulerConfig::Edrrm { .. } => "edrrm",
    }
}

fn estimate_hash_resources(
    forwarding_table: &ForwardingTableConfig,
    ports: u16,
    axis_width: u32,
) -> (f64, f64, u32) {
    let n = f64::from(ports);
    let (lut_base, ff_base, bram) = match forwarding_table {
        ForwardingTableConfig::FullLookup { .. } => (
            16.0 * n.powi(2) + 658.0 * n + 129.0,
            91.0 * n.powi(2) - 200.0 * n + 300.0,
            0,
        ),
        ForwardingTableConfig::MultiBankHash {
            entries_per_bank, ..
        } => {
            let hash_bits = bit_width_for_entries(*entries_per_bank);
            (
                716.5 * n.powi(2) - 900.0 * n + 1296.0,
                198.667 * n.powi(2) - 412.5 * n + 689.333,
                if hash_bits <= 8 { 0 } else { u32::from(ports) },
            )
        }
    };

    (
        apply_bus_width_scaling(lut_base, axis_width),
        apply_bus_width_scaling(ff_base, axis_width),
        bram,
    )
}

fn estimate_rx_resources(ports: u16, axis_width: u32) -> (f64, f64, u32) {
    let n = f64::from(ports);
    (
        apply_bus_width_scaling(39.0 + 51.0 * n, axis_width),
        apply_bus_width_scaling(7.0 + 557.0 * n, axis_width),
        8 * u32::from(ports),
    )
}

fn estimate_scheduler_resources(
    scheduler: &SchedulerConfig,
    voq: &VoqConfig,
    ports: u16,
    axis_width: u32,
) -> (f64, f64, u32) {
    let n = f64::from(ports);
    let one_buffer_islip = matches!(scheduler, SchedulerConfig::Islip { .. })
        && matches!(voq, VoqConfig::OneBufferPerPort { .. });
    let (lut_base, ff_base, bram_base) = if one_buffer_islip {
        (
            943.75 * n.powi(2) - 655.5 * n + 137.0,
            564.833 * n.powi(2) + 3055.5 * n - 5021.333,
            21.286 * n - 20.0,
        )
    } else {
        (
            744.583 * n.powi(2) - 1000.0 * n + 1512.667,
            1023.167 * n.powi(2) - 2398.0 * n + 5978.333,
            92.571 * n - 180.0,
        )
    };

    (
        apply_bus_width_scaling(lut_base, axis_width),
        apply_bus_width_scaling(ff_base, axis_width),
        nonnegative_trunc(bram_base) as u32,
    )
}

fn estimate_buffer_bram(voq: &VoqConfig, ports: u16, axis_width: u32) -> u32 {
    let total_bytes = buffer_memory_bytes(voq, ports, axis_width);
    if total_bytes == 0 {
        return 0;
    }

    let brams_parallel = axis_width.div_ceil(36);
    let bytes_per_entry = (axis_width / 8).max(1);
    let capacity_per_group = 512_u64 * u64::from(bytes_per_entry);
    let groups_needed = total_bytes.div_ceil(capacity_per_group);
    brams_parallel.saturating_mul(groups_needed as u32)
}

fn buffer_memory_bytes(voq: &VoqConfig, ports: u16, axis_width: u32) -> u64 {
    let bytes_per_packet = u64::from((axis_width / 8).max(1));
    match voq {
        VoqConfig::NByN {
            depth_packets,
            per_queue_depth_packets,
        } => {
            per_queue_depth_packets
                .as_ref()
                .map(|depths| depths.iter().map(|depth| u64::from(*depth)).sum::<u64>())
                .unwrap_or_else(|| {
                    u64::from(*depth_packets)
                        * u64::from(ports)
                        * u64::from(ports.saturating_sub(1))
                })
                * bytes_per_packet
        }
        VoqConfig::OneBufferPerPort {
            depth_packets,
            per_port_depth_packets,
        } => {
            per_port_depth_packets
                .as_ref()
                .map(|depths| depths.iter().map(|depth| u64::from(*depth)).sum::<u64>())
                .unwrap_or_else(|| u64::from(*depth_packets) * u64::from(ports))
                * bytes_per_packet
        }
        VoqConfig::Shared {
            total_depth_packets,
        } => u64::from(*total_depth_packets) * bytes_per_packet,
    }
}

fn apply_bus_width_scaling(value: f64, axis_width: u32) -> f64 {
    let ratio = f64::from(axis_width) / 512.0;
    if ratio > 1.0 {
        value * ratio * 0.6
    } else if ratio < 1.0 {
        value * ratio * 1.2
    } else {
        value
    }
}

fn bit_width_for_entries(entries: u32) -> u32 {
    if entries <= 1 {
        1
    } else {
        u32::BITS - (entries - 1).leading_zeros()
    }
}

fn nonnegative_trunc(value: f64) -> u64 {
    if value <= 0.0 {
        0
    } else {
        value as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use spac_core::{ForwardingTableConfig, SchedulerConfig, VoqConfig};
    use spac_trace::{TracePacket, WorkloadClass};

    #[test]
    fn tiny_space_dse_frontier_excludes_dominated_candidate() {
        let result =
            run_dse(&tiny_space(), &burst_trace(), &lenient_constraints()).expect("DSE succeeds");

        assert_eq!(result.schema_version, DSE_RESULT_SCHEMA_VERSION);
        assert_eq!(
            result.frontier,
            vec!["minimal_nbyn", "balanced_nbyn", "deep_nbyn"]
        );
        assert_eq!(
            result
                .candidates
                .iter()
                .find(|candidate| candidate.name == "wasteful_shared")
                .expect("wasteful candidate")
                .status,
            DseCandidateStatus::Dominated
        );
    }

    #[test]
    fn constraints_reject_candidate_before_frontier_ranking() {
        let mut constraints = lenient_constraints();
        constraints.max_packet_drop_rate = 0.0;

        let result = run_dse(&tiny_space(), &burst_trace(), &constraints).expect("DSE succeeds");

        assert_eq!(result.frontier, vec!["deep_nbyn"]);
        assert!(result
            .candidates
            .iter()
            .find(|candidate| candidate.name == "minimal_nbyn")
            .expect("minimal candidate")
            .constraint_failures
            .contains(&"max_packet_drop_rate".to_string()));
    }

    #[test]
    fn ae_dse_space_generator_matches_expected_shape() {
        let space = generate_spac_ae_dse_space("ae_8p", 8);

        assert_eq!(space.schema_version, DSE_SPACE_SCHEMA_VERSION);
        assert_eq!(space.candidates.len(), 48);
        assert!(space
            .candidates
            .iter()
            .any(|candidate| candidate.architecture.bus_width_bits == 640));
        assert!(space.candidates.iter().any(|candidate| matches!(
            candidate.architecture.voq,
            VoqConfig::OneBufferPerPort { .. }
        )));
    }

    #[test]
    fn ae_resource_estimator_matches_known_full_lookup_8p_512_width_terms() {
        let architecture = ArchitectureConfig {
            schema_version: "spac.architecture.v0".to_string(),
            name: "ae_reference".to_string(),
            ports: 8,
            bus_width_bits: 512,
            forwarding_table: ForwardingTableConfig::FullLookup {
                address_width_bits: 8,
            },
            voq: VoqConfig::OneBufferPerPort {
                depth_packets: 8,
                per_port_depth_packets: None,
            },
            scheduler: SchedulerConfig::RoundRobin { pipeline_stages: 1 },
            custom_kernels: Vec::new(),
        };

        let estimate = estimate_spac_ae_heuristic_v0(&architecture);

        assert_eq!(estimate.lut, 48029);
        assert_eq!(estimate.ff, 61264);
        assert_eq!(estimate.bram, 79);
        assert_eq!(estimate.dsp, 0);
    }

    #[test]
    fn ae_resource_estimator_matches_fixture_table_row() {
        let fixture =
            include_str!("../../../examples/contracts/spac-ae.dse_port_scan_results_final.csv");
        let row = fixture
            .lines()
            .find(|line| line.starts_with("8,OneBufferPerPort,RoundRobin,FullLookupTable,"))
            .expect("8-port full lookup fixture row");
        let columns = row.split(',').collect::<Vec<_>>();
        let architecture = ArchitectureConfig {
            schema_version: "spac.architecture.v0".to_string(),
            name: "ae_fixture_row".to_string(),
            ports: 8,
            bus_width_bits: 512,
            forwarding_table: ForwardingTableConfig::FullLookup {
                address_width_bits: 8,
            },
            voq: VoqConfig::OneBufferPerPort {
                depth_packets: 8,
                per_port_depth_packets: None,
            },
            scheduler: SchedulerConfig::RoundRobin { pipeline_stages: 1 },
            custom_kernels: Vec::new(),
        };

        let estimate = estimate_spac_ae_heuristic_v0(&architecture);

        assert_eq!(estimate.lut.to_string(), columns[7]);
        assert_eq!(estimate.ff.to_string(), columns[8]);
    }

    #[test]
    fn phase2_buffer_dse_adds_optimized_candidate() {
        let result = run_spac_ae_phase2_buffer_dse(
            &tiny_space(),
            &burst_trace(),
            &lenient_constraints(),
            Phase2BufferOptions {
                top_n: 1,
                min_depth_packets: 1,
                max_drop_rate: 1.0,
            },
        )
        .expect("phase2 DSE succeeds");

        let phase2 = result
            .candidates
            .iter()
            .find(|candidate| candidate.phase == 2)
            .expect("phase2 candidate");
        let optimization = phase2
            .buffer_optimization
            .as_ref()
            .expect("phase2 optimization metadata");

        assert_eq!(phase2.optimized_from.as_deref(), Some("deep_nbyn"));
        assert_eq!(optimization.min_depth_packets, 1);
        assert!(optimization.optimized_buffer_memory_packets > 0);
        assert!(phase2.architecture_name.ends_with("_phase2_buffer_opt"));
    }

    fn tiny_space() -> DseSpace {
        DseSpace {
            schema_version: DSE_SPACE_SCHEMA_VERSION.to_string(),
            name: "tiny_space".to_string(),
            candidates: vec![
                candidate(
                    "minimal_nbyn",
                    VoqConfig::NByN {
                        depth_packets: 1,
                        per_queue_depth_packets: None,
                    },
                    100,
                    1,
                ),
                candidate(
                    "balanced_nbyn",
                    VoqConfig::NByN {
                        depth_packets: 2,
                        per_queue_depth_packets: None,
                    },
                    120,
                    2,
                ),
                candidate(
                    "deep_nbyn",
                    VoqConfig::NByN {
                        depth_packets: 3,
                        per_queue_depth_packets: None,
                    },
                    140,
                    3,
                ),
                candidate(
                    "wasteful_shared",
                    VoqConfig::Shared {
                        total_depth_packets: 4,
                    },
                    200,
                    4,
                ),
            ],
        }
    }

    fn candidate(name: &str, voq: VoqConfig, lut: u64, bram: u64) -> DseCandidate {
        DseCandidate {
            name: name.to_string(),
            architecture: ArchitectureConfig {
                schema_version: "spac.architecture.v0".to_string(),
                name: name.to_string(),
                ports: 2,
                bus_width_bits: 256,
                forwarding_table: ForwardingTableConfig::FullLookup {
                    address_width_bits: 8,
                },
                voq,
                scheduler: SchedulerConfig::RoundRobin { pipeline_stages: 1 },
                custom_kernels: Vec::new(),
            },
            resource_estimate: ResourceEstimate {
                lut,
                ff: lut * 2,
                bram,
                dsp: 0,
            },
        }
    }

    fn burst_trace() -> TraceSpec {
        TraceSpec {
            schema_version: "spac.trace.v0".to_string(),
            name: "tiny_dse_burst".to_string(),
            workload_class: WorkloadClass::Datacenter,
            time_unit: "ns".to_string(),
            packets: vec![packet("burst_0"), packet("burst_1"), packet("burst_2")],
        }
    }

    fn packet(flow_id: &str) -> TracePacket {
        TracePacket {
            timestamp_ns: 0,
            ingress_port: 0,
            src: 0,
            dst: 1,
            payload_bytes: 512,
            flow_id: flow_id.to_string(),
        }
    }

    fn lenient_constraints() -> ConstraintsConfig {
        ConstraintsConfig {
            schema_version: "spac.constraints.v0".to_string(),
            name: "lenient".to_string(),
            board_target: "software-only".to_string(),
            max_lut: 10_000,
            max_ff: 20_000,
            max_bram: 100,
            max_dsp: 10,
            target_fmax_mhz: 350.0,
            max_p99_latency_ns: 1_000.0,
            max_packet_drop_rate: 1.0,
            max_initiation_interval: 4,
        }
    }
}
