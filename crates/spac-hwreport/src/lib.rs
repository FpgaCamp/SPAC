use serde::{Deserialize, Serialize};
use spac_core::{
    BoardProfile, ConstraintsConfig, Diagnostic, HW_ACCEPTANCE_SCHEMA_VERSION,
    HW_REPORT_SCHEMA_VERSION,
};
use std::fs;
use std::path::Path;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HwReportTool {
    Vitis,
    Vivado,
}

impl HwReportTool {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Vitis => "vitis",
            Self::Vivado => "vivado",
        }
    }
}

impl FromStr for HwReportTool {
    type Err = Diagnostic;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "vitis" | "vitis-hls" | "vitis_hls" => Ok(Self::Vitis),
            "vivado" => Ok(Self::Vivado),
            _ => Err(Diagnostic::error(
                "SPAC_HW_REPORT_TOOL",
                "--tool",
                "supported report tools are vitis and vivado",
            )),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct HwReport {
    pub schema_version: String,
    pub trust_level: String,
    pub tool: String,
    pub board_profile_id: String,
    pub board_model: String,
    pub fpga_part: String,
    pub toolchain_family: String,
    pub toolchain_version: String,
    pub source_report_path: String,
    pub metrics: HwReportMetrics,
    pub warnings: Vec<String>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
pub struct HwReportMetrics {
    pub lut: Option<u64>,
    pub ff: Option<u64>,
    pub bram: Option<u64>,
    pub dsp: Option<u64>,
    pub fmax_mhz: Option<f64>,
    pub initiation_interval: Option<u32>,
    pub latency_cycles_min: Option<u64>,
    pub latency_cycles_max: Option<u64>,
    pub throughput_gbps: Option<f64>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct HwAcceptanceReport {
    pub schema_version: String,
    pub trust_level: String,
    pub status: String,
    pub hw_report_path: String,
    pub constraints_path: String,
    pub board_profile_id: String,
    pub constraints_name: String,
    pub checks: Vec<HwAcceptanceCheck>,
    pub warnings: Vec<String>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct HwAcceptanceCheck {
    pub metric: String,
    pub status: String,
    pub comparator: String,
    pub measured: Option<f64>,
    pub limit: Option<f64>,
    pub message: String,
}

pub fn accept_hw_report(
    report: &HwReport,
    constraints: &ConstraintsConfig,
    hw_report_path: String,
    constraints_path: String,
) -> HwAcceptanceReport {
    let mut checks = vec![
        max_check("lut", report.metrics.lut, constraints.max_lut),
        max_check("ff", report.metrics.ff, constraints.max_ff),
        max_check("bram", report.metrics.bram, constraints.max_bram),
        max_check("dsp", report.metrics.dsp, constraints.max_dsp),
        min_check(
            "fmax_mhz",
            report.metrics.fmax_mhz,
            constraints.target_fmax_mhz,
        ),
        max_check(
            "initiation_interval",
            report.metrics.initiation_interval.map(u64::from),
            u64::from(constraints.max_initiation_interval),
        ),
    ];
    checks.push(not_evaluated_check(
        "p99_latency_ns",
        constraints.max_p99_latency_ns,
        "p99 latency requires trace-driven simulation or measured packet latency evidence",
    ));
    checks.push(not_evaluated_check(
        "packet_drop_rate",
        constraints.max_packet_drop_rate,
        "packet drop rate requires trace-driven simulation or measured packet drop evidence",
    ));

    let status = if checks.iter().any(|check| check.status == "fail") {
        "fail"
    } else if checks.iter().any(|check| check.status == "inconclusive") {
        "inconclusive"
    } else {
        "pass"
    }
    .to_string();

    HwAcceptanceReport {
        schema_version: HW_ACCEPTANCE_SCHEMA_VERSION.to_string(),
        trust_level: "post_synthesis".to_string(),
        status,
        hw_report_path,
        constraints_path,
        board_profile_id: report.board_profile_id.clone(),
        constraints_name: constraints.name.clone(),
        checks,
        warnings: vec![
            "Hardware acceptance is based on parsed vendor report metrics only".to_string(),
        ],
        limitations: vec![
            "No FPGA hardware measurement was performed by this command".to_string(),
            "Trace-dependent p99 latency and packet drop constraints are recorded as not_evaluated".to_string(),
            "This acceptance report does not reproduce SPAC paper metrics without matching traces, configs, EDA settings, and hardware evidence".to_string(),
        ],
    }
}

fn max_check(metric: &str, measured: Option<u64>, limit: u64) -> HwAcceptanceCheck {
    match measured {
        Some(value) if value <= limit => HwAcceptanceCheck {
            metric: metric.to_string(),
            status: "pass".to_string(),
            comparator: "<=".to_string(),
            measured: Some(value as f64),
            limit: Some(limit as f64),
            message: format!("{metric} {value} is within limit {limit}"),
        },
        Some(value) => HwAcceptanceCheck {
            metric: metric.to_string(),
            status: "fail".to_string(),
            comparator: "<=".to_string(),
            measured: Some(value as f64),
            limit: Some(limit as f64),
            message: format!("{metric} {value} exceeds limit {limit}"),
        },
        None => HwAcceptanceCheck {
            metric: metric.to_string(),
            status: "inconclusive".to_string(),
            comparator: "<=".to_string(),
            measured: None,
            limit: Some(limit as f64),
            message: format!("{metric} metric is missing from the hardware report"),
        },
    }
}

fn min_check(metric: &str, measured: Option<f64>, limit: f64) -> HwAcceptanceCheck {
    match measured {
        Some(value) if value >= limit => HwAcceptanceCheck {
            metric: metric.to_string(),
            status: "pass".to_string(),
            comparator: ">=".to_string(),
            measured: Some(value),
            limit: Some(limit),
            message: format!("{metric} {value} meets minimum {limit}"),
        },
        Some(value) => HwAcceptanceCheck {
            metric: metric.to_string(),
            status: "fail".to_string(),
            comparator: ">=".to_string(),
            measured: Some(value),
            limit: Some(limit),
            message: format!("{metric} {value} is below minimum {limit}"),
        },
        None => HwAcceptanceCheck {
            metric: metric.to_string(),
            status: "inconclusive".to_string(),
            comparator: ">=".to_string(),
            measured: None,
            limit: Some(limit),
            message: format!("{metric} metric is missing from the hardware report"),
        },
    }
}

fn not_evaluated_check(metric: &str, limit: f64, message: &str) -> HwAcceptanceCheck {
    HwAcceptanceCheck {
        metric: metric.to_string(),
        status: "not_evaluated".to_string(),
        comparator: "<=".to_string(),
        measured: None,
        limit: Some(limit),
        message: message.to_string(),
    }
}

pub fn parse_hw_report_file(
    tool: HwReportTool,
    report_path: &Path,
    board_profile: &BoardProfile,
) -> Result<HwReport, Vec<Diagnostic>> {
    let text = fs::read_to_string(report_path).map_err(|error| {
        vec![Diagnostic::error(
            "SPAC_HW_REPORT_READ",
            report_path.display().to_string(),
            format!("failed to read hardware report: {error}"),
        )]
    })?;

    parse_hw_report_text(
        tool,
        &text,
        report_path.display().to_string(),
        board_profile,
    )
}

pub fn parse_hw_report_text(
    tool: HwReportTool,
    text: &str,
    source_report_path: String,
    board_profile: &BoardProfile,
) -> Result<HwReport, Vec<Diagnostic>> {
    let mut metrics = HwReportMetrics::default();

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let Some((raw_key, raw_value)) =
            trimmed.split_once(':').or_else(|| trimmed.split_once('='))
        else {
            continue;
        };
        apply_metric(&mut metrics, &normalize_key(raw_key), raw_value);
    }

    if !metrics.has_any_primary_metric() {
        return Err(vec![Diagnostic::error(
            "SPAC_HW_REPORT_NO_METRICS",
            source_report_path,
            "report did not contain any recognized FPGA resource, timing, II, latency, or throughput metric",
        )]);
    }

    Ok(HwReport {
        schema_version: HW_REPORT_SCHEMA_VERSION.to_string(),
        trust_level: "post_synthesis".to_string(),
        tool: tool.as_str().to_string(),
        board_profile_id: board_profile.board_id.clone(),
        board_model: board_profile.board_model.clone(),
        fpga_part: board_profile.fpga_part.clone(),
        toolchain_family: board_profile.toolchain.family.clone(),
        toolchain_version: board_profile.toolchain.version.clone(),
        source_report_path,
        metrics,
        warnings: vec![
            "Parsed vendor report evidence only; no FPGA hardware measurement was performed"
                .to_string(),
        ],
        limitations: vec![
            "Parser coverage is intentionally conservative and fixture-backed; vendor report variants may require additional aliases".to_string(),
            "This report does not reproduce SPAC paper metrics without matching traces, configs, EDA settings, and hardware evidence".to_string(),
        ],
    })
}

impl HwReportMetrics {
    fn has_any_primary_metric(&self) -> bool {
        self.lut.is_some()
            || self.ff.is_some()
            || self.bram.is_some()
            || self.dsp.is_some()
            || self.fmax_mhz.is_some()
            || self.initiation_interval.is_some()
            || self.latency_cycles_min.is_some()
            || self.latency_cycles_max.is_some()
            || self.throughput_gbps.is_some()
    }
}

fn apply_metric(metrics: &mut HwReportMetrics, key: &str, value: &str) {
    match key {
        "lut" | "lut_usage" | "clb_lut" | "total_lut" => {
            metrics.lut = parse_first_u64(value);
        }
        "ff" | "ff_usage" | "flip_flop" | "register" | "total_ff" => {
            metrics.ff = parse_first_u64(value);
        }
        "bram" | "bram_usage" | "bram_18k" | "block_ram_tile" | "total_bram" => {
            metrics.bram = parse_first_u64(value);
        }
        "dsp" | "dsp_usage" | "dsp48" | "dsp48e" => {
            metrics.dsp = parse_first_u64(value);
        }
        "fmax_mhz" | "estimated_fmax_mhz" | "achieved_fmax_mhz" | "frequency_mhz" => {
            metrics.fmax_mhz = parse_first_f64(value);
        }
        "ii" | "initiation_interval" | "interval_min" | "target_ii" => {
            metrics.initiation_interval = parse_first_u64(value).and_then(|value| {
                if value <= u32::MAX as u64 {
                    Some(value as u32)
                } else {
                    None
                }
            });
        }
        "latency_min_cycles" | "latency_cycles_min" | "latency_min" => {
            metrics.latency_cycles_min = parse_first_u64(value);
        }
        "latency_max_cycles" | "latency_cycles_max" | "latency_max" => {
            metrics.latency_cycles_max = parse_first_u64(value);
        }
        "throughput_gbps" | "max_throughput_gbps" => {
            metrics.throughput_gbps = parse_first_f64(value);
        }
        _ => {}
    }
}

fn normalize_key(raw: &str) -> String {
    let mut normalized = String::new();
    let mut last_was_separator = false;

    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() {
            normalized.push(ch.to_ascii_lowercase());
            last_was_separator = false;
        } else if !last_was_separator {
            normalized.push('_');
            last_was_separator = true;
        }
    }

    normalized.trim_matches('_').to_string()
}

fn parse_first_u64(value: &str) -> Option<u64> {
    for token in numeric_tokens(value) {
        if let Ok(parsed) = token.parse::<u64>() {
            return Some(parsed);
        }
    }
    None
}

fn parse_first_f64(value: &str) -> Option<f64> {
    for token in numeric_tokens(value) {
        if let Ok(parsed) = token.parse::<f64>() {
            if parsed.is_finite() {
                return Some(parsed);
            }
        }
    }
    None
}

fn numeric_tokens(value: &str) -> Vec<String> {
    let without_grouping = value.replace(',', "");
    without_grouping
        .split(|ch: char| !(ch.is_ascii_digit() || matches!(ch, '.' | '-' | '+')))
        .filter(|token| {
            let trimmed = token.trim();
            !(trimmed.is_empty()
                || trimmed == "+"
                || trimmed == "-"
                || trimmed == "."
                || trimmed == "+."
                || trimmed == "-.")
        })
        .map(ToOwned::to_owned)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use spac_core::{ReportLocations, ToolchainProfile};

    #[test]
    fn parses_vitis_fixture_metrics() {
        let report = parse_hw_report_text(
            HwReportTool::Vitis,
            r#"
            LUT: 48,029
            FF: 61264
            BRAM_18K: 3904
            DSP: 0
            FMax_MHz: 350.0 MHz
            Initiation Interval: 1
            Latency Min Cycles: 7
            Latency Max Cycles: 21
            Throughput Gbps: 179.2
            "#,
            "synthetic.rpt".to_string(),
            &board_profile(),
        )
        .expect("parse Vitis report");

        assert_eq!(report.schema_version, "spac.hw-report.v0");
        assert_eq!(report.trust_level, "post_synthesis");
        assert_eq!(report.tool, "vitis");
        assert_eq!(report.metrics.lut, Some(48029));
        assert_eq!(report.metrics.ff, Some(61264));
        assert_eq!(report.metrics.bram, Some(3904));
        assert_eq!(report.metrics.dsp, Some(0));
        assert_eq!(report.metrics.fmax_mhz, Some(350.0));
        assert_eq!(report.metrics.initiation_interval, Some(1));
        assert_eq!(report.metrics.latency_cycles_min, Some(7));
        assert_eq!(report.metrics.latency_cycles_max, Some(21));
        assert_eq!(report.metrics.throughput_gbps, Some(179.2));
    }

    #[test]
    fn parses_vivado_fixture_metrics() {
        let report = parse_hw_report_text(
            HwReportTool::Vivado,
            r#"
            CLB LUT: 50000
            Register: 63000
            Block RAM Tile: 300
            DSP48E: 0
            Achieved Fmax MHz: 347.5
            Target II: 1
            Latency Cycles Min: 9
            Latency Cycles Max: 24
            Max Throughput Gbps: 177.9
            "#,
            "synthetic.rpt".to_string(),
            &board_profile(),
        )
        .expect("parse Vivado report");

        assert_eq!(report.tool, "vivado");
        assert_eq!(report.metrics.lut, Some(50000));
        assert_eq!(report.metrics.ff, Some(63000));
        assert_eq!(report.metrics.bram, Some(300));
        assert_eq!(report.metrics.fmax_mhz, Some(347.5));
    }

    #[test]
    fn rejects_report_without_metrics() {
        let diagnostics = parse_hw_report_text(
            HwReportTool::Vitis,
            "This is not a recognized report.",
            "empty.rpt".to_string(),
            &board_profile(),
        )
        .expect_err("reject empty report");

        assert_eq!(diagnostics[0].code, "SPAC_HW_REPORT_NO_METRICS");
    }

    #[test]
    fn tool_from_str_rejects_unknown_tool() {
        let diagnostic = "quartus"
            .parse::<HwReportTool>()
            .expect_err("reject unknown tool");

        assert_eq!(diagnostic.code, "SPAC_HW_REPORT_TOOL");
    }

    #[test]
    fn acceptance_passes_when_report_meets_hardware_constraints() {
        let report = HwReport {
            schema_version: "spac.hw-report.v0".to_string(),
            trust_level: "post_synthesis".to_string(),
            tool: "vitis".to_string(),
            board_profile_id: "amd-alveo-u45n".to_string(),
            board_model: "Alveo U45N".to_string(),
            fpga_part: "xcu26-vsva1365-2LV-e".to_string(),
            toolchain_family: "Vitis HLS".to_string(),
            toolchain_version: "2023.2".to_string(),
            source_report_path: "fixture.rpt".to_string(),
            metrics: HwReportMetrics {
                lut: Some(48029),
                ff: Some(61264),
                bram: Some(300),
                dsp: Some(0),
                fmax_mhz: Some(350.0),
                initiation_interval: Some(1),
                latency_cycles_min: None,
                latency_cycles_max: None,
                throughput_gbps: None,
            },
            warnings: Vec::new(),
            limitations: Vec::new(),
        };

        let acceptance = accept_hw_report(
            &report,
            &constraints(),
            "hw_report.json".to_string(),
            "constraints.json".to_string(),
        );

        assert_eq!(acceptance.schema_version, "spac.hw-acceptance.v0");
        assert_eq!(acceptance.status, "pass");
        assert!(acceptance
            .checks
            .iter()
            .any(|check| check.metric == "p99_latency_ns" && check.status == "not_evaluated"));
    }

    #[test]
    fn acceptance_fails_when_report_violates_hardware_constraints() {
        let mut report = minimal_report();
        report.metrics.lut = Some(100001);
        report.metrics.fmax_mhz = Some(349.0);

        let acceptance = accept_hw_report(
            &report,
            &constraints(),
            "hw_report.json".to_string(),
            "constraints.json".to_string(),
        );

        assert_eq!(acceptance.status, "fail");
        assert!(acceptance
            .checks
            .iter()
            .any(|check| check.metric == "lut" && check.status == "fail"));
        assert!(acceptance
            .checks
            .iter()
            .any(|check| check.metric == "fmax_mhz" && check.status == "fail"));
    }

    #[test]
    fn acceptance_is_inconclusive_when_hardware_metric_is_missing() {
        let mut report = minimal_report();
        report.metrics.bram = None;

        let acceptance = accept_hw_report(
            &report,
            &constraints(),
            "hw_report.json".to_string(),
            "constraints.json".to_string(),
        );

        assert_eq!(acceptance.status, "inconclusive");
        assert!(acceptance
            .checks
            .iter()
            .any(|check| check.metric == "bram" && check.status == "inconclusive"));
    }

    fn board_profile() -> BoardProfile {
        BoardProfile {
            schema_version: "spac.board-profile.v0".to_string(),
            board_id: "amd-alveo-u45n".to_string(),
            vendor: "AMD".to_string(),
            board_model: "Alveo U45N".to_string(),
            fpga_part: "xcu26-vsva1365-2LV-e".to_string(),
            toolchain: ToolchainProfile {
                family: "Vitis HLS".to_string(),
                version: "2023.2".to_string(),
            },
            target_clock_mhz: 350.0,
            host_interface: "PCIe".to_string(),
            loopback_topology: "host-fpga-host loopback".to_string(),
            report_locations: ReportLocations {
                synthesis_summary: "reports/post_synthesis.json".to_string(),
                timing_summary: "reports/timing_summary.rpt".to_string(),
            },
        }
    }

    fn minimal_report() -> HwReport {
        HwReport {
            schema_version: "spac.hw-report.v0".to_string(),
            trust_level: "post_synthesis".to_string(),
            tool: "vitis".to_string(),
            board_profile_id: "amd-alveo-u45n".to_string(),
            board_model: "Alveo U45N".to_string(),
            fpga_part: "xcu26-vsva1365-2LV-e".to_string(),
            toolchain_family: "Vitis HLS".to_string(),
            toolchain_version: "2023.2".to_string(),
            source_report_path: "fixture.rpt".to_string(),
            metrics: HwReportMetrics {
                lut: Some(48029),
                ff: Some(61264),
                bram: Some(300),
                dsp: Some(0),
                fmax_mhz: Some(350.0),
                initiation_interval: Some(1),
                latency_cycles_min: None,
                latency_cycles_max: None,
                throughput_gbps: None,
            },
            warnings: Vec::new(),
            limitations: Vec::new(),
        }
    }

    fn constraints() -> ConstraintsConfig {
        ConstraintsConfig {
            schema_version: "spac.constraints.v0".to_string(),
            name: "paper_aligned_u45n".to_string(),
            board_target: "amd-alveo-u45n".to_string(),
            max_lut: 100000,
            max_ff: 100000,
            max_bram: 300,
            max_dsp: 0,
            target_fmax_mhz: 350.0,
            max_p99_latency_ns: 1000.0,
            max_packet_drop_rate: 0.000001,
            max_initiation_interval: 1,
        }
    }
}
