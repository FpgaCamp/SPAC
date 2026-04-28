use serde::{Deserialize, Serialize};
use spac_core::{Diagnostic, SUPPORTED_TRACE_SCHEMA_VERSION};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::str::FromStr;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct TraceSpec {
    pub schema_version: String,
    pub name: String,
    pub workload_class: WorkloadClass,
    pub time_unit: String,
    pub packets: Vec<TracePacket>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkloadClass {
    Hft,
    RlAllReduce,
    Datacenter,
    Industrial,
    UnderwaterSensor,
}

impl FromStr for WorkloadClass {
    type Err = Diagnostic;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "hft" => Ok(Self::Hft),
            "rl_all_reduce" => Ok(Self::RlAllReduce),
            "datacenter" => Ok(Self::Datacenter),
            "industrial" => Ok(Self::Industrial),
            "underwater_sensor" => Ok(Self::UnderwaterSensor),
            _ => Err(Diagnostic::error(
                "SPAC_TRACE_WORKLOAD_CLASS",
                "--workload-class",
                format!("unsupported workload_class '{value}'"),
            )),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct TracePacket {
    pub timestamp_ns: u64,
    pub ingress_port: u16,
    pub src: u64,
    pub dst: u64,
    pub payload_bytes: u32,
    pub flow_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AeTopology {
    pub host_to_switch_port: BTreeMap<u64, u16>,
    pub switch_count: usize,
}

pub fn parse_trace_text(text: &str) -> Result<TraceSpec, Vec<Diagnostic>> {
    serde_json::from_str::<TraceSpec>(text).map_err(|error| {
        vec![Diagnostic::error(
            "SPAC_TRACE_PARSE",
            "$",
            format!("failed to parse trace JSON: {error}"),
        )]
    })
}

pub fn validate_trace_text(text: &str) -> Result<TraceSpec, Vec<Diagnostic>> {
    let trace = parse_trace_text(text)?;
    let diagnostics = validate_trace(&trace);

    if diagnostics.is_empty() {
        Ok(trace)
    } else {
        Err(diagnostics)
    }
}

pub fn validate_trace_file(path: &Path) -> Result<TraceSpec, Vec<Diagnostic>> {
    let text = fs::read_to_string(path).map_err(|error| {
        vec![Diagnostic::error(
            "SPAC_TRACE_READ",
            path.display().to_string(),
            format!("failed to read trace: {error}"),
        )]
    })?;

    validate_trace_text(&text)
}

pub fn import_spac_ae_trace_file(
    path: &Path,
    name: impl Into<String>,
    workload_class: WorkloadClass,
    ports: u16,
    topology: Option<&AeTopology>,
) -> Result<TraceSpec, Vec<Diagnostic>> {
    let text = fs::read_to_string(path).map_err(|error| {
        vec![Diagnostic::error(
            "SPAC_AE_TRACE_READ",
            path.display().to_string(),
            format!("failed to read SPAC-AE trace CSV: {error}"),
        )]
    })?;

    import_spac_ae_trace_text(&text, name, workload_class, ports, topology)
}

pub fn import_spac_ae_trace_text(
    text: &str,
    name: impl Into<String>,
    workload_class: WorkloadClass,
    ports: u16,
    topology: Option<&AeTopology>,
) -> Result<TraceSpec, Vec<Diagnostic>> {
    let mut diagnostics = Vec::new();
    if ports == 0 {
        diagnostics.push(Diagnostic::error(
            "SPAC_AE_TRACE_PORTS",
            "--ports",
            "ports must be greater than zero",
        ));
    }

    let mut lines = text.lines();
    let Some(header) = lines.next() else {
        diagnostics.push(Diagnostic::error(
            "SPAC_AE_TRACE_EMPTY",
            "$",
            "SPAC-AE trace CSV must not be empty",
        ));
        return Err(diagnostics);
    };

    if strip_cr(header) != "time,src_addr,dst_addr,header_size,body_size,trace_id" {
        diagnostics.push(Diagnostic::error(
            "SPAC_AE_TRACE_HEADER",
            "$.header",
            "SPAC-AE trace header must be time,src_addr,dst_addr,header_size,body_size,trace_id",
        ));
    }

    let mut packets = Vec::new();
    let mut previous_timestamp = None;
    for (line_index, line) in lines.enumerate() {
        let path = format!("$.rows[{}]", line_index + 2);
        if line.trim().is_empty() {
            continue;
        }
        let columns: Vec<&str> = strip_cr(line).split(',').collect();
        if columns.len() != 6 {
            diagnostics.push(Diagnostic::error(
                "SPAC_AE_TRACE_ROW_WIDTH",
                &path,
                format!("expected 6 CSV columns, found {}", columns.len()),
            ));
            continue;
        }

        let timestamp_ns = match parse_ae_timestamp(columns[0], &path) {
            Ok(timestamp) => timestamp,
            Err(diagnostic) => {
                diagnostics.push(diagnostic);
                continue;
            }
        };
        let src = match parse_u64(columns[1], &path, "src_addr") {
            Ok(value) => value,
            Err(diagnostic) => {
                diagnostics.push(diagnostic);
                continue;
            }
        };
        let dst = match parse_u64(columns[2], &path, "dst_addr") {
            Ok(value) => value,
            Err(diagnostic) => {
                diagnostics.push(diagnostic);
                continue;
            }
        };
        let header_size = match parse_u32(columns[3], &path, "header_size") {
            Ok(value) => value,
            Err(diagnostic) => {
                diagnostics.push(diagnostic);
                continue;
            }
        };
        let body_size = match parse_u32(columns[4], &path, "body_size") {
            Ok(value) => value,
            Err(diagnostic) => {
                diagnostics.push(diagnostic);
                continue;
            }
        };
        let trace_id = columns[5].trim();
        if trace_id.is_empty() {
            diagnostics.push(Diagnostic::error(
                "SPAC_AE_TRACE_ID_EMPTY",
                format!("{path}.trace_id"),
                "trace_id must not be empty",
            ));
            continue;
        }

        if let Some(previous) = previous_timestamp {
            if timestamp_ns < previous {
                diagnostics.push(Diagnostic::error(
                    "SPAC_AE_TRACE_TIMESTAMP_ORDER",
                    format!("{path}.time"),
                    "SPAC-AE trace timestamps must be monotonic nondecreasing",
                ));
            }
        }
        previous_timestamp = Some(timestamp_ns);

        let Some(payload_bytes) = header_size.checked_add(body_size) else {
            diagnostics.push(Diagnostic::error(
                "SPAC_AE_TRACE_PAYLOAD_OVERFLOW",
                &path,
                "header_size + body_size overflows u32",
            ));
            continue;
        };
        let ingress_port = ingress_port_for_source(src, ports, topology);
        packets.push(TracePacket {
            timestamp_ns,
            ingress_port,
            src,
            dst,
            payload_bytes,
            flow_id: format!("spac_ae_trace_{trace_id}"),
        });
    }

    if packets.is_empty() {
        diagnostics.push(Diagnostic::error(
            "SPAC_AE_TRACE_NO_PACKETS",
            "$.rows",
            "SPAC-AE trace import produced no packets",
        ));
    }

    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }

    let trace = TraceSpec {
        schema_version: SUPPORTED_TRACE_SCHEMA_VERSION.to_string(),
        name: name.into(),
        workload_class,
        time_unit: "ns".to_string(),
        packets,
    };
    let validation = validate_trace(&trace);
    if validation.is_empty() {
        Ok(trace)
    } else {
        Err(validation)
    }
}

pub fn parse_spac_ae_topology_file(path: &Path) -> Result<AeTopology, Vec<Diagnostic>> {
    let text = fs::read_to_string(path).map_err(|error| {
        vec![Diagnostic::error(
            "SPAC_AE_TOPOLOGY_READ",
            path.display().to_string(),
            format!("failed to read SPAC-AE topology CSV: {error}"),
        )]
    })?;

    parse_spac_ae_topology_text(&text)
}

pub fn parse_spac_ae_topology_text(text: &str) -> Result<AeTopology, Vec<Diagnostic>> {
    let mut diagnostics = Vec::new();
    let mut host_to_switch_port = BTreeMap::new();
    let mut switches = std::collections::BTreeSet::new();

    for (line_index, line) in text.lines().enumerate() {
        let line = strip_cr(line).trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let path = format!("$.rows[{}]", line_index + 1);
        let columns: Vec<&str> = line.split(',').collect();
        if columns.len() != 4 {
            diagnostics.push(Diagnostic::error(
                "SPAC_AE_TOPOLOGY_ROW_WIDTH",
                &path,
                format!("expected 4 CSV columns, found {}", columns.len()),
            ));
            continue;
        }

        let node_a = columns[0].trim();
        let node_b = columns[2].trim();
        let port_b = match parse_u16(columns[3], &path, "port_b") {
            Ok(value) => value,
            Err(diagnostic) => {
                diagnostics.push(diagnostic);
                continue;
            }
        };

        if let Ok(host_id) = node_a.parse::<u64>() {
            if node_b.starts_with('s') {
                host_to_switch_port.insert(host_id, port_b);
                switches.insert(node_b.to_string());
            }
        }
    }

    if host_to_switch_port.is_empty() {
        diagnostics.push(Diagnostic::error(
            "SPAC_AE_TOPOLOGY_NO_HOST_LINKS",
            "$.rows",
            "topology must contain at least one host-to-switch link such as 0,0,s0,0",
        ));
    }

    if diagnostics.is_empty() {
        Ok(AeTopology {
            host_to_switch_port,
            switch_count: switches.len(),
        })
    } else {
        Err(diagnostics)
    }
}

pub fn validate_trace(trace: &TraceSpec) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    if trace.schema_version != SUPPORTED_TRACE_SCHEMA_VERSION {
        diagnostics.push(Diagnostic::error(
            "SPAC_TRACE_SCHEMA_VERSION",
            "$.schema_version",
            format!(
                "unsupported trace schema version '{}'; expected '{}'",
                trace.schema_version, SUPPORTED_TRACE_SCHEMA_VERSION
            ),
        ));
    }

    if trace.name.trim().is_empty() {
        diagnostics.push(Diagnostic::error(
            "SPAC_TRACE_NAME_EMPTY",
            "$.name",
            "trace name must not be empty",
        ));
    }

    if trace.time_unit != "ns" {
        diagnostics.push(Diagnostic::error(
            "SPAC_TRACE_TIME_UNIT",
            "$.time_unit",
            "time_unit must be 'ns'",
        ));
    }

    if trace.packets.is_empty() {
        diagnostics.push(Diagnostic::error(
            "SPAC_TRACE_PACKETS_EMPTY",
            "$.packets",
            "trace must contain at least one packet",
        ));
    }

    let mut previous_timestamp = None;
    for (index, packet) in trace.packets.iter().enumerate() {
        if packet.flow_id.trim().is_empty() {
            diagnostics.push(Diagnostic::error(
                "SPAC_TRACE_FLOW_ID_EMPTY",
                format!("$.packets[{index}].flow_id"),
                "packet flow_id must not be empty",
            ));
        }

        if let Some(previous) = previous_timestamp {
            if packet.timestamp_ns < previous {
                diagnostics.push(Diagnostic::error(
                    "SPAC_TRACE_TIMESTAMP_ORDER",
                    format!("$.packets[{index}].timestamp_ns"),
                    "packet timestamps must be monotonic nondecreasing",
                ));
            }
        }

        previous_timestamp = Some(packet.timestamp_ns);
    }

    diagnostics
}

fn ingress_port_for_source(src: u64, ports: u16, topology: Option<&AeTopology>) -> u16 {
    if let Some(topology) = topology {
        if let Some(port) = topology.host_to_switch_port.get(&src) {
            return *port;
        }
    }
    (src % u64::from(ports)) as u16
}

fn parse_ae_timestamp(value: &str, path: &str) -> Result<u64, Diagnostic> {
    let timestamp = value.trim().parse::<f64>().map_err(|error| {
        Diagnostic::error(
            "SPAC_AE_TRACE_TIME_PARSE",
            format!("{path}.time"),
            format!("failed to parse time as nanoseconds: {error}"),
        )
    })?;
    if !timestamp.is_finite() || timestamp < 0.0 {
        return Err(Diagnostic::error(
            "SPAC_AE_TRACE_TIME_RANGE",
            format!("{path}.time"),
            "time must be finite and nonnegative",
        ));
    }

    Ok(timestamp.round() as u64)
}

fn parse_u64(value: &str, path: &str, field: &str) -> Result<u64, Diagnostic> {
    value.trim().parse::<u64>().map_err(|error| {
        Diagnostic::error(
            format!("SPAC_AE_{}_PARSE", field.to_ascii_uppercase()),
            format!("{path}.{field}"),
            format!("failed to parse {field} as unsigned integer: {error}"),
        )
    })
}

fn parse_u32(value: &str, path: &str, field: &str) -> Result<u32, Diagnostic> {
    value.trim().parse::<u32>().map_err(|error| {
        Diagnostic::error(
            format!("SPAC_AE_{}_PARSE", field.to_ascii_uppercase()),
            format!("{path}.{field}"),
            format!("failed to parse {field} as unsigned integer: {error}"),
        )
    })
}

fn parse_u16(value: &str, path: &str, field: &str) -> Result<u16, Diagnostic> {
    value.trim().parse::<u16>().map_err(|error| {
        Diagnostic::error(
            format!("SPAC_AE_{}_PARSE", field.to_ascii_uppercase()),
            format!("{path}.{field}"),
            format!("failed to parse {field} as unsigned integer: {error}"),
        )
    })
}

fn strip_cr(value: &str) -> &str {
    value.strip_suffix('\r').unwrap_or(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_TRACE: &str = r#"{
      "schema_version": "spac.trace.v0",
      "name": "hft_tiny",
      "workload_class": "hft",
      "time_unit": "ns",
      "packets": [
        {
          "timestamp_ns": 0,
          "ingress_port": 0,
          "src": 1,
          "dst": 2,
          "payload_bytes": 24,
          "flow_id": "flow_0"
        },
        {
          "timestamp_ns": 40,
          "ingress_port": 1,
          "src": 2,
          "dst": 3,
          "payload_bytes": 24,
          "flow_id": "flow_1"
        }
      ]
    }"#;

    #[test]
    fn valid_trace_passes() {
        let trace = validate_trace_text(VALID_TRACE).expect("valid trace");

        assert_eq!(trace.schema_version, SUPPORTED_TRACE_SCHEMA_VERSION);
        assert_eq!(trace.packets.len(), 2);
    }

    #[test]
    fn unsupported_schema_version_is_rejected() {
        let invalid = VALID_TRACE.replace("spac.trace.v0", "spac.trace.v99");
        let diagnostics = validate_trace_text(&invalid).expect_err("invalid trace");

        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "SPAC_TRACE_SCHEMA_VERSION"));
    }

    #[test]
    fn empty_packet_list_is_rejected() {
        let invalid = VALID_TRACE.replace(
            r#""packets": [
        {
          "timestamp_ns": 0,
          "ingress_port": 0,
          "src": 1,
          "dst": 2,
          "payload_bytes": 24,
          "flow_id": "flow_0"
        },
        {
          "timestamp_ns": 40,
          "ingress_port": 1,
          "src": 2,
          "dst": 3,
          "payload_bytes": 24,
          "flow_id": "flow_1"
        }
      ]"#,
            r#""packets": []"#,
        );
        let diagnostics = validate_trace_text(&invalid).expect_err("invalid trace");

        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "SPAC_TRACE_PACKETS_EMPTY"));
    }

    #[test]
    fn nonmonotonic_timestamp_is_rejected() {
        let invalid = VALID_TRACE.replace("\"timestamp_ns\": 40", "\"timestamp_ns\": 0");
        let invalid = invalid.replacen("\"timestamp_ns\": 0", "\"timestamp_ns\": 80", 1);
        let diagnostics = validate_trace_text(&invalid).expect_err("invalid trace");

        assert!(diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "SPAC_TRACE_TIMESTAMP_ORDER"));
    }

    #[test]
    fn spac_ae_trace_import_normalizes_csv_rows() {
        let topology = parse_spac_ae_topology_text("0,0,s0,0\n5,0,s0,3\n").expect("valid topology");
        let trace = import_spac_ae_trace_text(
            "time,src_addr,dst_addr,header_size,body_size,trace_id\n1549.46,0,7,2,57,1\n3147.34,5,0,2,42,2\n",
            "ae_hft_sample",
            WorkloadClass::Hft,
            8,
            Some(&topology),
        )
        .expect("valid SPAC-AE trace");

        assert_eq!(trace.schema_version, SUPPORTED_TRACE_SCHEMA_VERSION);
        assert_eq!(trace.packets[0].timestamp_ns, 1549);
        assert_eq!(trace.packets[0].ingress_port, 0);
        assert_eq!(trace.packets[0].payload_bytes, 59);
        assert_eq!(trace.packets[1].ingress_port, 3);
        assert_eq!(trace.packets[1].flow_id, "spac_ae_trace_2");
    }

    #[test]
    fn spac_ae_topology_parser_maps_hosts_to_switch_ports() {
        let topology = parse_spac_ae_topology_text("0,0,s0,0\n7,0,s0,7\n").expect("valid topology");

        assert_eq!(topology.switch_count, 1);
        assert_eq!(topology.host_to_switch_port.get(&0), Some(&0));
        assert_eq!(topology.host_to_switch_port.get(&7), Some(&7));
    }

    #[test]
    fn spac_ae_full_hft_fixture_imports_all_rows() {
        let trace_csv = include_str!("../../../examples/contracts/spac-ae.hft_trace.csv");
        let topology_csv = include_str!("../../../examples/contracts/spac-ae.dse_8nodes.csv");
        let topology = parse_spac_ae_topology_text(topology_csv).expect("valid topology fixture");
        let trace = import_spac_ae_trace_text(
            trace_csv,
            "ae_hft_full_fixture",
            WorkloadClass::Hft,
            8,
            Some(&topology),
        )
        .expect("valid full SPAC-AE trace fixture");

        assert_eq!(trace.packets.len(), 734);
        assert_eq!(trace.packets[0].timestamp_ns, 1549);
        assert_eq!(trace.packets[0].payload_bytes, 59);
        assert_eq!(trace.packets[1].ingress_port, 5);
        assert!(trace
            .packets
            .windows(2)
            .all(|window| window[0].timestamp_ns <= window[1].timestamp_ns));
    }
}
