use spac_core::{ArchitectureConfig, ForwardingTableConfig, SchedulerConfig, VoqConfig};
use spac_sim::run_simulation;
use spac_trace::{TracePacket, TraceSpec, WorkloadClass};
use std::collections::BTreeSet;

#[test]
fn tiny_space_bruteforce_oracle_excludes_dominated_candidates() {
    let trace = burst_trace();
    let evaluated: Vec<EvaluatedCandidate> = candidate_space()
        .into_iter()
        .map(|candidate| evaluate_candidate(candidate, &trace))
        .collect();

    let frontier: BTreeSet<&str> = evaluated
        .iter()
        .filter(|candidate| {
            !evaluated
                .iter()
                .any(|other| other.name != candidate.name && dominates(other, candidate))
        })
        .map(|candidate| candidate.name)
        .collect();

    assert_eq!(
        frontier,
        BTreeSet::from(["balanced_nbyn", "deep_nbyn", "minimal_nbyn"]),
        "tiny-space oracle should retain only non-dominated simulator candidates"
    );
}

fn evaluate_candidate(candidate: Candidate, trace: &TraceSpec) -> EvaluatedCandidate {
    let report = run_simulation(&candidate.architecture, trace).expect("simulation succeeds");

    EvaluatedCandidate {
        name: candidate.name,
        packets_dropped: report.metrics.packets_dropped,
        max_latency_ns: report.metrics.latency_ns.max,
        resource_proxy: candidate.resource_proxy,
    }
}

fn dominates(left: &EvaluatedCandidate, right: &EvaluatedCandidate) -> bool {
    let no_worse = left.packets_dropped <= right.packets_dropped
        && left.max_latency_ns <= right.max_latency_ns
        && left.resource_proxy <= right.resource_proxy;
    let strictly_better = left.packets_dropped < right.packets_dropped
        || left.max_latency_ns < right.max_latency_ns
        || left.resource_proxy < right.resource_proxy;

    no_worse && strictly_better
}

fn candidate_space() -> Vec<Candidate> {
    vec![
        Candidate::new(
            "minimal_nbyn",
            VoqConfig::NByN {
                depth_packets: 1,
                per_queue_depth_packets: None,
            },
            1,
        ),
        Candidate::new(
            "balanced_nbyn",
            VoqConfig::NByN {
                depth_packets: 2,
                per_queue_depth_packets: None,
            },
            2,
        ),
        Candidate::new(
            "deep_nbyn",
            VoqConfig::NByN {
                depth_packets: 3,
                per_queue_depth_packets: None,
            },
            3,
        ),
        Candidate::new(
            "wasteful_shared",
            VoqConfig::Shared {
                total_depth_packets: 4,
            },
            4,
        ),
    ]
}

fn burst_trace() -> TraceSpec {
    TraceSpec {
        schema_version: "spac.trace.v0".to_string(),
        name: "tiny_space_burst".to_string(),
        workload_class: WorkloadClass::Datacenter,
        time_unit: "ns".to_string(),
        packets: vec![
            packet("burst_0", 0),
            packet("burst_1", 0),
            packet("burst_2", 0),
        ],
    }
}

fn packet(flow_id: &str, timestamp_ns: u64) -> TracePacket {
    TracePacket {
        timestamp_ns,
        ingress_port: 0,
        src: 0,
        dst: 1,
        payload_bytes: 512,
        flow_id: flow_id.to_string(),
    }
}

struct Candidate {
    name: &'static str,
    architecture: ArchitectureConfig,
    resource_proxy: u64,
}

impl Candidate {
    fn new(name: &'static str, voq: VoqConfig, resource_proxy: u64) -> Self {
        Self {
            name,
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
            resource_proxy,
        }
    }
}

struct EvaluatedCandidate {
    name: &'static str,
    packets_dropped: u64,
    max_latency_ns: u64,
    resource_proxy: u64,
}
