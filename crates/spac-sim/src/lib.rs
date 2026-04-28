use serde::{Deserialize, Serialize};
use spac_core::{
    validate_architecture_config, ArchitectureConfig, Diagnostic, ForwardingTableConfig,
    SchedulerConfig, VoqConfig, SIMULATION_RUN_SCHEMA_VERSION,
};
use spac_trace::{validate_trace, TracePacket, TraceSpec};
use std::collections::{BTreeMap, VecDeque};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct SimulationReport {
    pub schema_version: String,
    pub architecture_name: String,
    pub trace_name: String,
    pub trust_level: String,
    pub supported_model: SupportedModel,
    pub metrics: SimulationMetrics,
    pub packet_outcomes: Vec<PacketOutcome>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct SupportedModel {
    pub forwarding_table: String,
    pub voq: String,
    pub scheduler: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct SimulationMetrics {
    pub packets_received: u64,
    pub packets_forwarded: u64,
    pub packets_dropped: u64,
    pub drop_rate: f64,
    pub simulation_duration_ns: u64,
    pub throughput_packets_per_ns: f64,
    pub throughput_gbps: f64,
    pub latency_ns: LatencySummary,
    pub queue_occupancy_max: u64,
    pub peak_voq_occupancy_packets: Vec<u64>,
    pub bank_conflicts: u64,
    pub lookup_stall_cycles: u64,
    pub estimated_initiation_interval: u32,
    pub line_rate_achieved_ratio: f64,
    pub rx_utilization: f64,
    pub hash_utilization: f64,
    pub scheduler_utilization: f64,
    pub per_port_forwarded: Vec<u64>,
    pub per_port_dropped: Vec<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct LatencySummary {
    pub p50: u64,
    pub p95: u64,
    pub p99: u64,
    pub max: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct PacketOutcome {
    pub flow_id: String,
    pub ingress_port: u16,
    pub egress_port: Option<u16>,
    pub status: PacketStatus,
    pub timestamp_ns: u64,
    pub completion_ns: Option<u64>,
    pub latency_ns: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PacketStatus {
    Forwarded,
    Dropped,
}

#[derive(Debug, Clone)]
struct QueuedPacket {
    trace_index: usize,
    timestamp_ns: u64,
    ingress_port: usize,
    egress_port: usize,
    flow_id: String,
    payload_bytes: u32,
}

pub fn run_simulation(
    architecture: &ArchitectureConfig,
    trace: &TraceSpec,
) -> Result<SimulationReport, Vec<Diagnostic>> {
    let mut diagnostics = validate_architecture_config(architecture);
    diagnostics.extend(validate_trace(trace));
    diagnostics.extend(validate_trace_ports(architecture, trace));

    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }

    let ports = usize::from(architecture.ports);
    let mut queues = vec![vec![VecDeque::<QueuedPacket>::new(); ports]; ports];
    let mut rr_pointers = vec![0usize; ports];
    let mut packet_outcomes = Vec::with_capacity(trace.packets.len());
    let mut latencies = Vec::new();
    let mut forwarded_payload_bytes = 0u64;
    let mut per_port_forwarded = vec![0u64; ports];
    let mut per_port_dropped = vec![0u64; ports];
    let mut peak_voq_occupancy_packets = vec![0u64; voq_peak_slots(architecture, ports)];
    let mut next_packet_index = 0usize;
    let mut queued_packets = 0usize;
    let mut queue_occupancy_max = 0u64;
    let pipeline_latency_cycles = estimated_pipeline_latency_cycles(architecture);
    let mut current_time = trace
        .packets
        .first()
        .map(|packet| packet.timestamp_ns)
        .unwrap_or_default();

    while next_packet_index < trace.packets.len() || queued_packets > 0 {
        if queued_packets == 0 && next_packet_index < trace.packets.len() {
            let next_timestamp = trace.packets[next_packet_index].timestamp_ns;
            if next_timestamp > current_time {
                current_time = next_timestamp;
            }
        }

        while next_packet_index < trace.packets.len()
            && trace.packets[next_packet_index].timestamp_ns <= current_time
        {
            let packet = &trace.packets[next_packet_index];
            let egress_port = forwarding_egress(architecture, packet);

            if has_queue_capacity(architecture, &queues, packet.ingress_port, egress_port) {
                queues[usize::from(packet.ingress_port)][usize::from(egress_port)].push_back(
                    QueuedPacket {
                        trace_index: next_packet_index,
                        timestamp_ns: packet.timestamp_ns,
                        ingress_port: usize::from(packet.ingress_port),
                        egress_port: usize::from(egress_port),
                        flow_id: packet.flow_id.clone(),
                        payload_bytes: packet.payload_bytes,
                    },
                );
                queued_packets += 1;
                queue_occupancy_max = queue_occupancy_max.max(queued_packets as u64);
                update_peak_voq(
                    architecture,
                    &queues,
                    usize::from(packet.ingress_port),
                    usize::from(egress_port),
                    &mut peak_voq_occupancy_packets,
                );
            } else {
                per_port_dropped[usize::from(egress_port)] += 1;
                packet_outcomes.push(PacketOutcome {
                    flow_id: packet.flow_id.clone(),
                    ingress_port: packet.ingress_port,
                    egress_port: Some(egress_port),
                    status: PacketStatus::Dropped,
                    timestamp_ns: packet.timestamp_ns,
                    completion_ns: None,
                    latency_ns: None,
                });
            }

            next_packet_index += 1;
        }

        for egress_port in 0..ports {
            if let Some(ingress_port) =
                select_ingress(architecture, &queues, egress_port, &rr_pointers)
            {
                if let Some(packet) = queues[ingress_port][egress_port].pop_front() {
                    queued_packets -= 1;
                    update_scheduler_state(
                        architecture,
                        egress_port,
                        ingress_port,
                        &mut rr_pointers,
                    );
                    per_port_forwarded[egress_port] += 1;
                    forwarded_payload_bytes =
                        forwarded_payload_bytes.saturating_add(u64::from(packet.payload_bytes));
                    let completion_ns = current_time.saturating_add(pipeline_latency_cycles);
                    let latency_ns = completion_ns.saturating_sub(packet.timestamp_ns);
                    latencies.push(latency_ns);
                    packet_outcomes.push(PacketOutcome {
                        flow_id: packet.flow_id,
                        ingress_port: packet.ingress_port as u16,
                        egress_port: Some(packet.egress_port as u16),
                        status: PacketStatus::Forwarded,
                        timestamp_ns: packet.timestamp_ns,
                        completion_ns: Some(completion_ns),
                        latency_ns: Some(latency_ns),
                    });
                }
            }
        }

        if next_packet_index < trace.packets.len() || queued_packets > 0 {
            current_time = current_time.saturating_add(1);
        }
    }

    packet_outcomes.sort_by_key(|outcome| {
        (
            outcome.timestamp_ns,
            outcome.completion_ns.unwrap_or(u64::MAX),
            outcome.flow_id.clone(),
        )
    });

    let packets_received = trace.packets.len() as u64;
    let packets_forwarded = latencies.len() as u64;
    let packets_dropped = packets_received.saturating_sub(packets_forwarded);
    let drop_rate = if packets_received == 0 {
        0.0
    } else {
        packets_dropped as f64 / packets_received as f64
    };
    let simulation_duration_ns = trace
        .packets
        .first()
        .map(|packet| {
            current_time
                .saturating_sub(packet.timestamp_ns)
                .saturating_add(1)
        })
        .unwrap_or_default();
    let throughput_packets_per_ns = if simulation_duration_ns == 0 {
        0.0
    } else {
        packets_forwarded as f64 / simulation_duration_ns as f64
    };
    let throughput_gbps = if simulation_duration_ns == 0 {
        0.0
    } else {
        forwarded_payload_bytes as f64 * 8.0 / simulation_duration_ns as f64
    };
    let (bank_conflicts, lookup_stall_cycles) = forwarding_conflict_stats(architecture, trace);
    let estimated_initiation_interval = estimated_initiation_interval(architecture);
    let line_rate_achieved_ratio = line_rate_achieved_ratio(architecture, trace);
    let rx_utilization = utilization(packets_received, estimated_rx_ii(), simulation_duration_ns);
    let hash_utilization = utilization(
        packets_received,
        estimated_hash_ii(&architecture.forwarding_table),
        simulation_duration_ns,
    );
    let scheduler_utilization = utilization(
        packets_forwarded,
        estimated_scheduler_ii(&architecture.scheduler),
        simulation_duration_ns,
    );

    Ok(SimulationReport {
        schema_version: SIMULATION_RUN_SCHEMA_VERSION.to_string(),
        architecture_name: architecture.name.clone(),
        trace_name: trace.name.clone(),
        trust_level: "software_model".to_string(),
        supported_model: SupportedModel {
            forwarding_table: forwarding_model_name(architecture).to_string(),
            voq: voq_model_name(architecture).to_string(),
            scheduler: scheduler_model_name(architecture).to_string(),
        },
        metrics: SimulationMetrics {
            packets_received,
            packets_forwarded,
            packets_dropped,
            drop_rate,
            simulation_duration_ns,
            throughput_packets_per_ns,
            throughput_gbps,
            latency_ns: summarize_latencies(&mut latencies),
            queue_occupancy_max,
            peak_voq_occupancy_packets,
            bank_conflicts,
            lookup_stall_cycles,
            estimated_initiation_interval,
            line_rate_achieved_ratio,
            rx_utilization,
            hash_utilization,
            scheduler_utilization,
            per_port_forwarded,
            per_port_dropped,
        },
        packet_outcomes,
        warnings: simulation_warnings(architecture),
    })
}

fn validate_trace_ports(architecture: &ArchitectureConfig, trace: &TraceSpec) -> Vec<Diagnostic> {
    trace
        .packets
        .iter()
        .enumerate()
        .filter_map(|(index, packet)| {
            if packet.ingress_port >= architecture.ports {
                Some(Diagnostic::error(
                    "SPAC_TRACE_INGRESS_PORT",
                    format!("$.packets[{index}].ingress_port"),
                    format!(
                        "ingress_port {} is outside architecture port range 0..{}",
                        packet.ingress_port,
                        architecture.ports.saturating_sub(1)
                    ),
                ))
            } else {
                None
            }
        })
        .collect()
}

fn forwarding_egress(architecture: &ArchitectureConfig, packet: &TracePacket) -> u16 {
    (packet.dst % u64::from(architecture.ports)) as u16
}

fn has_queue_capacity(
    architecture: &ArchitectureConfig,
    queues: &[Vec<VecDeque<QueuedPacket>>],
    ingress_port: u16,
    egress_port: u16,
) -> bool {
    match &architecture.voq {
        VoqConfig::NByN {
            depth_packets,
            per_queue_depth_packets,
        } => {
            let queue = &queues[usize::from(ingress_port)][usize::from(egress_port)];
            queue.len()
                < n_by_n_depth(
                    *depth_packets,
                    per_queue_depth_packets.as_deref(),
                    queues.len(),
                    usize::from(ingress_port),
                    usize::from(egress_port),
                ) as usize
        }
        VoqConfig::OneBufferPerPort {
            depth_packets,
            per_port_depth_packets,
        } => {
            egress_queue_occupancy(queues, usize::from(egress_port))
                < one_buffer_depth(
                    *depth_packets,
                    per_port_depth_packets.as_deref(),
                    usize::from(egress_port),
                ) as usize
        }
        VoqConfig::Shared {
            total_depth_packets,
        } => total_queue_occupancy(queues) < *total_depth_packets as usize,
    }
}

fn select_ingress(
    architecture: &ArchitectureConfig,
    queues: &[Vec<VecDeque<QueuedPacket>>],
    egress_port: usize,
    rr_pointers: &[usize],
) -> Option<usize> {
    if matches!(&architecture.voq, VoqConfig::OneBufferPerPort { .. }) {
        return select_oldest(queues, egress_port);
    }

    match &architecture.scheduler {
        SchedulerConfig::RoundRobin { .. } | SchedulerConfig::Islip { .. } => {
            select_round_robin(queues, egress_port, rr_pointers[egress_port])
        }
        SchedulerConfig::Edrrm { .. } => select_oldest(queues, egress_port),
    }
}

fn select_round_robin(
    queues: &[Vec<VecDeque<QueuedPacket>>],
    egress_port: usize,
    pointer: usize,
) -> Option<usize> {
    for offset in 0..queues.len() {
        let ingress_port = (pointer + offset) % queues.len();
        if !queues[ingress_port][egress_port].is_empty() {
            return Some(ingress_port);
        }
    }

    None
}

fn select_oldest(queues: &[Vec<VecDeque<QueuedPacket>>], egress_port: usize) -> Option<usize> {
    queues
        .iter()
        .enumerate()
        .filter_map(|(ingress_port, per_egress)| {
            per_egress[egress_port]
                .front()
                .map(|packet| (packet.timestamp_ns, packet.trace_index, ingress_port))
        })
        .min()
        .map(|(_, _, ingress_port)| ingress_port)
}

fn update_scheduler_state(
    architecture: &ArchitectureConfig,
    egress_port: usize,
    ingress_port: usize,
    rr_pointers: &mut [usize],
) {
    if matches!(
        &architecture.scheduler,
        SchedulerConfig::RoundRobin { .. } | SchedulerConfig::Islip { .. }
    ) {
        rr_pointers[egress_port] = (ingress_port + 1) % rr_pointers.len();
    }
}

fn forwarding_conflict_stats(architecture: &ArchitectureConfig, trace: &TraceSpec) -> (u64, u64) {
    let ForwardingTableConfig::MultiBankHash { banks, .. } = &architecture.forwarding_table else {
        return (0, 0);
    };

    let mut hits_by_time_and_bank = BTreeMap::<(u64, u16), u64>::new();
    for packet in &trace.packets {
        let bank = (packet.dst % u64::from(*banks)) as u16;
        *hits_by_time_and_bank
            .entry((packet.timestamp_ns, bank))
            .or_default() += 1;
    }

    let conflicts = hits_by_time_and_bank
        .values()
        .map(|hits| hits.saturating_sub(1))
        .sum();

    (conflicts, conflicts)
}

fn summarize_latencies(latencies: &mut [u64]) -> LatencySummary {
    if latencies.is_empty() {
        return LatencySummary {
            p50: 0,
            p95: 0,
            p99: 0,
            max: 0,
        };
    }

    latencies.sort_unstable();

    LatencySummary {
        p50: percentile(latencies, 50),
        p95: percentile(latencies, 95),
        p99: percentile(latencies, 99),
        max: *latencies.last().unwrap_or(&0),
    }
}

fn percentile(sorted_values: &[u64], percentile: u64) -> u64 {
    let len = sorted_values.len() as u64;
    let rank = (percentile * len).div_ceil(100);
    let index = rank.saturating_sub(1).min(len.saturating_sub(1)) as usize;
    sorted_values[index]
}

fn total_queue_occupancy(queues: &[Vec<VecDeque<QueuedPacket>>]) -> usize {
    queues
        .iter()
        .flat_map(|per_ingress| per_ingress.iter())
        .map(VecDeque::len)
        .sum()
}

fn egress_queue_occupancy(queues: &[Vec<VecDeque<QueuedPacket>>], egress_port: usize) -> usize {
    queues
        .iter()
        .map(|per_egress| per_egress[egress_port].len())
        .sum()
}

fn n_by_n_depth(
    default_depth: u32,
    per_queue_depths: Option<&[u32]>,
    ports: usize,
    ingress_port: usize,
    egress_port: usize,
) -> u32 {
    per_queue_depths
        .and_then(|depths| depths.get(ingress_port * ports + egress_port).copied())
        .unwrap_or(default_depth)
}

fn one_buffer_depth(
    default_depth: u32,
    per_port_depths: Option<&[u32]>,
    egress_port: usize,
) -> u32 {
    per_port_depths
        .and_then(|depths| depths.get(egress_port).copied())
        .unwrap_or(default_depth)
}

fn voq_peak_slots(architecture: &ArchitectureConfig, ports: usize) -> usize {
    match &architecture.voq {
        VoqConfig::NByN { .. } => ports * ports,
        VoqConfig::OneBufferPerPort { .. } => ports,
        VoqConfig::Shared { .. } => 1,
    }
}

fn update_peak_voq(
    architecture: &ArchitectureConfig,
    queues: &[Vec<VecDeque<QueuedPacket>>],
    ingress_port: usize,
    egress_port: usize,
    peak: &mut [u64],
) {
    let (index, occupancy) = match &architecture.voq {
        VoqConfig::NByN { .. } => (
            ingress_port * queues.len() + egress_port,
            queues[ingress_port][egress_port].len(),
        ),
        VoqConfig::OneBufferPerPort { .. } => {
            (egress_port, egress_queue_occupancy(queues, egress_port))
        }
        VoqConfig::Shared { .. } => (0, total_queue_occupancy(queues)),
    };

    if let Some(slot) = peak.get_mut(index) {
        *slot = (*slot).max(occupancy as u64);
    }
}

fn estimated_pipeline_latency_cycles(architecture: &ArchitectureConfig) -> u64 {
    u64::from(estimated_rx_latency())
        + u64::from(estimated_hash_latency(
            &architecture.forwarding_table,
            architecture.ports,
        ))
        + u64::from(estimated_scheduler_latency(
            &architecture.scheduler,
            &architecture.voq,
            architecture.ports,
        ))
}

fn estimated_initiation_interval(architecture: &ArchitectureConfig) -> u32 {
    estimated_rx_ii()
        .max(estimated_hash_ii(&architecture.forwarding_table))
        .max(estimated_scheduler_ii(&architecture.scheduler))
}

fn estimated_rx_latency() -> u32 {
    2
}

fn estimated_rx_ii() -> u32 {
    1
}

fn estimated_hash_latency(forwarding_table: &ForwardingTableConfig, ports: u16) -> u32 {
    match forwarding_table {
        ForwardingTableConfig::FullLookup { .. } => u32::from(ports).div_ceil(2) + 1,
        ForwardingTableConfig::MultiBankHash { .. } => 4.max(u32::from(ports) + 2),
    }
}

fn estimated_hash_ii(forwarding_table: &ForwardingTableConfig) -> u32 {
    match forwarding_table {
        ForwardingTableConfig::FullLookup { .. } => 1,
        ForwardingTableConfig::MultiBankHash { .. } => 3,
    }
}

fn estimated_scheduler_latency(scheduler: &SchedulerConfig, _voq: &VoqConfig, ports: u16) -> u32 {
    let base_latency = match scheduler {
        SchedulerConfig::Islip { .. } => 6.5,
        SchedulerConfig::RoundRobin { .. } | SchedulerConfig::Edrrm { .. } => 3.5,
    };
    (0.679 * f64::from(ports) + base_latency).floor() as u32
}

fn estimated_scheduler_ii(scheduler: &SchedulerConfig) -> u32 {
    match scheduler {
        SchedulerConfig::Islip { .. } => 4,
        SchedulerConfig::RoundRobin { .. } | SchedulerConfig::Edrrm { .. } => 1,
    }
}

fn line_rate_achieved_ratio(architecture: &ArchitectureConfig, trace: &TraceSpec) -> f64 {
    if trace.packets.is_empty() {
        return 0.0;
    }

    let decision_ii = estimated_initiation_interval(architecture);
    let achieved = trace
        .packets
        .iter()
        .filter(|packet| {
            let transfer_cycles =
                transfer_cycles(packet.payload_bytes, architecture.bus_width_bits);
            decision_ii <= transfer_cycles
        })
        .count();

    achieved as f64 / trace.packets.len() as f64
}

fn transfer_cycles(packet_size_bytes: u32, bus_width_bits: u32) -> u32 {
    let packet_size_bits = packet_size_bytes.saturating_mul(8);
    packet_size_bits.div_ceil(bus_width_bits).max(1)
}

fn utilization(events: u64, initiation_interval: u32, duration_ns: u64) -> f64 {
    if duration_ns == 0 {
        0.0
    } else {
        (events as f64 * f64::from(initiation_interval) / duration_ns as f64).min(1.0)
    }
}

fn forwarding_model_name(architecture: &ArchitectureConfig) -> &'static str {
    match &architecture.forwarding_table {
        ForwardingTableConfig::FullLookup { .. } => "full_lookup",
        ForwardingTableConfig::MultiBankHash { .. } => "multi_bank_hash",
    }
}

fn voq_model_name(architecture: &ArchitectureConfig) -> &'static str {
    match &architecture.voq {
        VoqConfig::NByN { .. } => "n_by_n",
        VoqConfig::OneBufferPerPort { .. } => "one_buffer_per_port",
        VoqConfig::Shared { .. } => "shared",
    }
}

fn scheduler_model_name(architecture: &ArchitectureConfig) -> &'static str {
    match &architecture.scheduler {
        SchedulerConfig::RoundRobin { .. } => "round_robin",
        SchedulerConfig::Islip { .. } => "islip",
        SchedulerConfig::Edrrm { .. } => "edrrm",
    }
}

fn simulation_warnings(architecture: &ArchitectureConfig) -> Vec<String> {
    let mut warnings = vec![
        "software_model trust level: no HLS synthesis, RTL simulation, timing closure, or FPGA measurement was performed".to_string(),
    ];

    if !architecture.custom_kernels.is_empty() {
        warnings.push(
            "custom kernel latency/resource hints are accepted by the architecture contract but not modeled by the MVP-B simulator".to_string(),
        );
    }

    warnings
}

#[cfg(test)]
mod tests {
    use super::*;
    use spac_core::{CustomKernelConfig, ResourceClass};
    use spac_trace::WorkloadClass;

    #[test]
    fn no_contention_full_lookup_nbyn_round_robin_forwards_all_packets() {
        let report = run_simulation(&base_architecture(), &trace_with_packets(no_contention()))
            .expect("simulation succeeds");

        assert_eq!(report.schema_version, SIMULATION_RUN_SCHEMA_VERSION);
        assert_eq!(report.metrics.packets_received, 3);
        assert_eq!(report.metrics.packets_forwarded, 3);
        assert_eq!(report.metrics.packets_dropped, 0);
        assert_eq!(report.metrics.latency_ns.max, 11);
        assert_eq!(report.metrics.estimated_initiation_interval, 1);
        assert_eq!(report.metrics.line_rate_achieved_ratio, 1.0);
    }

    #[test]
    fn n_by_n_queue_full_drops_excess_packets() {
        let mut architecture = base_architecture();
        architecture.voq = VoqConfig::NByN {
            depth_packets: 1,
            per_queue_depth_packets: None,
        };
        let trace = trace_with_packets(vec![
            packet(0, 0, 2, "flow_0"),
            packet(0, 0, 2, "flow_1"),
            packet(0, 0, 2, "flow_2"),
        ]);

        let report = run_simulation(&architecture, &trace).expect("simulation succeeds");

        assert_eq!(report.metrics.packets_forwarded, 1);
        assert_eq!(report.metrics.packets_dropped, 2);
        assert_eq!(report.packet_outcomes[1].status, PacketStatus::Dropped);
    }

    #[test]
    fn round_robin_contention_rotates_ingress_grants() {
        let trace = trace_with_packets(vec![
            packet_from_ingress(0, 0, 2, "flow_0"),
            packet_from_ingress(0, 1, 2, "flow_1"),
            packet_from_ingress(0, 0, 2, "flow_2"),
        ]);

        let report = run_simulation(&base_architecture(), &trace).expect("simulation succeeds");
        let forwarded: Vec<&PacketOutcome> = report
            .packet_outcomes
            .iter()
            .filter(|outcome| outcome.status == PacketStatus::Forwarded)
            .collect();

        assert_eq!(forwarded[0].flow_id, "flow_0");
        assert_eq!(forwarded[1].flow_id, "flow_1");
        assert_eq!(forwarded[2].flow_id, "flow_2");
        assert_eq!(report.metrics.latency_ns.max, 13);
    }

    #[test]
    fn one_buffer_per_port_capacity_is_per_egress_port() {
        let mut architecture = base_architecture();
        architecture.voq = VoqConfig::OneBufferPerPort {
            depth_packets: 2,
            per_port_depth_packets: None,
        };
        let trace = trace_with_packets(vec![
            packet_from_ingress(0, 0, 2, "flow_0"),
            packet_from_ingress(0, 1, 2, "flow_1"),
            packet_from_ingress(0, 2, 2, "flow_2"),
        ]);

        let report = run_simulation(&architecture, &trace).expect("simulation succeeds");

        assert_eq!(report.supported_model.voq, "one_buffer_per_port");
        assert_eq!(report.metrics.packets_forwarded, 2);
        assert_eq!(report.metrics.packets_dropped, 1);
        assert_eq!(report.metrics.peak_voq_occupancy_packets[2], 2);
    }

    #[test]
    fn shared_voq_capacity_is_global() {
        let mut architecture = base_architecture();
        architecture.voq = VoqConfig::Shared {
            total_depth_packets: 2,
        };
        let trace = trace_with_packets(vec![
            packet_from_ingress(0, 0, 2, "flow_0"),
            packet_from_ingress(0, 1, 2, "flow_1"),
            packet_from_ingress(0, 2, 2, "flow_2"),
        ]);

        let report = run_simulation(&architecture, &trace).expect("simulation succeeds");

        assert_eq!(report.metrics.packets_forwarded, 2);
        assert_eq!(report.metrics.packets_dropped, 1);
        assert_eq!(report.metrics.queue_occupancy_max, 2);
    }

    #[test]
    fn multi_bank_hash_counts_same_cycle_bank_conflicts() {
        let mut architecture = base_architecture();
        architecture.forwarding_table = ForwardingTableConfig::MultiBankHash {
            banks: 2,
            entries_per_bank: 64,
        };
        let trace = trace_with_packets(vec![
            packet(0, 0, 2, "flow_0"),
            packet(0, 1, 4, "flow_1"),
            packet(0, 2, 3, "flow_2"),
        ]);

        let report = run_simulation(&architecture, &trace).expect("simulation succeeds");

        assert_eq!(report.metrics.bank_conflicts, 1);
        assert_eq!(report.metrics.lookup_stall_cycles, 1);
    }

    #[test]
    fn custom_kernel_warning_is_explicit() {
        let mut architecture = base_architecture();
        architecture.custom_kernels.push(CustomKernelConfig {
            name: "risk_filter".to_string(),
            latency_cycles: 3,
            resource_class: ResourceClass::Light,
        });

        let report = run_simulation(&architecture, &trace_with_packets(no_contention()))
            .expect("simulation succeeds");

        assert!(report
            .warnings
            .iter()
            .any(|warning| warning.contains("custom kernel")));
    }

    fn base_architecture() -> ArchitectureConfig {
        ArchitectureConfig {
            schema_version: "spac.architecture.v0".to_string(),
            name: "test_architecture".to_string(),
            ports: 4,
            bus_width_bits: 256,
            forwarding_table: ForwardingTableConfig::FullLookup {
                address_width_bits: 8,
            },
            voq: VoqConfig::NByN {
                depth_packets: 8,
                per_queue_depth_packets: None,
            },
            scheduler: SchedulerConfig::RoundRobin { pipeline_stages: 1 },
            custom_kernels: Vec::new(),
        }
    }

    fn trace_with_packets(packets: Vec<TracePacket>) -> TraceSpec {
        TraceSpec {
            schema_version: "spac.trace.v0".to_string(),
            name: "test_trace".to_string(),
            workload_class: WorkloadClass::Hft,
            time_unit: "ns".to_string(),
            packets,
        }
    }

    fn no_contention() -> Vec<TracePacket> {
        vec![
            packet(0, 0, 1, "flow_0"),
            packet(40, 1, 2, "flow_1"),
            packet(80, 2, 3, "flow_2"),
        ]
    }

    fn packet(timestamp_ns: u64, ingress_port: u16, dst: u64, flow_id: &str) -> TracePacket {
        packet_from_ingress(timestamp_ns, ingress_port, dst, flow_id)
    }

    fn packet_from_ingress(
        timestamp_ns: u64,
        ingress_port: u16,
        dst: u64,
        flow_id: &str,
    ) -> TracePacket {
        TracePacket {
            timestamp_ns,
            ingress_port,
            src: u64::from(ingress_port),
            dst,
            payload_bytes: 24,
            flow_id: flow_id.to_string(),
        }
    }
}
