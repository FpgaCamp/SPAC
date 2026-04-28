# SPAC-AE Comparative Analysis and Adoption Plan

## Evidence Snapshot

Observed source:

- repository: `https://github.com/spac-proj/SPAC-AE`
- observed commit: `cfa56eaf3ebf5ffba0a92753a1d6581c4255ec73`
- observed date: 2026-04-27
- license: Apache License 2.0

This document classifies SPAC-AE as an artifact-evaluation repository. It is
valuable for clean-room Rust compatibility fixtures, resource formulas, DSE
oracles, and HLS reference surfaces. It is not treated as production source for
this repository because the local implementation policy allows only Rust or
TypeScript production code.

## Comparative Findings

| Area | SPAC paper expectation | SPAC-AE evidence | Current Rust state | Gap |
|---|---|---|---|---|
| Protocol compiler | NetBlocks-compatible DSL, semantic binding, generated `packet.hpp` | HLS examples include fixed protocol constants and metadata fields | `.spac` parser, semantic `routing_key`, deterministic metadata, generated HLS traits | no NetBlocks frontend and no full compiler parity |
| Switch datapath | parser, custom kernels, forwarding, VOQ, scheduler, deparser | HLS reference designs under `ae_designs/{core_only,ethernet,basic}` | generated HLS trait/csim packages only | no maintained generated switch template yet |
| Forward table | FullLookupTable and MultiBankHash | Python simulator and HLS source model both variants | Rust simulator and DSE support both variants | no post-synthesis calibration per variant |
| VOQ | N*N VOQ and Shared VOQ | simulator distinguishes `OneBufferPerPort` and `NBuffersPerPort` | Rust supports `n_by_n`, `one_buffer_per_port`, and `shared` software models | shared-pointer hardware cost still approximate |
| Scheduler | RR, iSLIP, EDRRM | simulator exposes RR and iSLIP; paper discusses EDRRM | Rust exposes RR, iSLIP, and EDRRM software-model policies | EDRRM needs stronger SPAC-AE/paper oracle coverage |
| Statistical simulator | event-driven latency, II, line-rate, queue occupancy | Python simulator has cycle/module timing, utilization, and peak VOQ stats | Rust reports latency, drop, throughput, II, line-rate, utilization, peak VOQ | Rust model is less cycle-accurate than SPAC-AE internals |
| DSE | staged pruning, resource model, phase-2 sizing | phase-1 scan, phase-2 peak-VOQ buffer sizing, CSV result tables | bounded Pareto DSE, SPAC-AE-style candidate generation, phase-2 depth vectors | needs larger goldens, infeasible spaces, and calibration fields |
| Hardware evidence | Vitis HLS 2023.2, U45N, timing/resource reports | README names Vitis IDE 2025.2; no raw reports committed | board profile, HLS csim package, synthetic report parser | no real Vitis/Vivado report evidence |
| Hardware acceptance | constraints checked against synthesis evidence | no normalized acceptance artifact visible | `spac.hw-report.v0` can be compared with `spac.constraints.v0` through `spac.hw-acceptance.v0` | needs real reports before paper-level claims |
| Experiment packaging | evidence should remain reproducible and attributable | artifact scripts are present but no normalized bundle contract is visible | `spac.experiment-run.v0` packages evidence directories with hashes, board identity, trust level, and limitations | real SPAC-AE reports are still needed for paper-level bundles |
| ns-3 | custom protocol adapter and SPAC switch device | no ns-3 implementation visible in SPAC-AE snapshot | deferred | blocked until lower-level evidence is stable |

## Reuse Decisions

Take as provenance-tagged fixtures:

- `data/hft_trace.csv`
- `evaluation/traces/exp0_industry_trace_1s_h1b.csv`
- `evaluation/topology/dse_8nodes.csv`
- `evaluation/topology/dse_10nodes.csv`
- `evaluation/topology/dse_32nodes.csv`
- `data/dse_port_scan_results_final.csv`
- `data/dse_results_all.csv`

Derive clean-room Rust behavior:

- resource-estimation formulas for hash, RX, scheduler, and buffer BRAM
- DSE phase-1 candidate enumeration over hash, VOQ, scheduler, and bus width
- DSE phase-2 buffer sizing from peak VOQ occupancy with explicit
  `software_model` limitations
- latency/II and utilization modeling as testable simulator refinements
- HLS config/package shape for generated artifacts only

Do not import as maintained production source:

- Python simulator or DSE scripts
- Bash experiment scripts
- HLS C++ source as hand-maintained source
- notebooks, `__pycache__`, temporary files, or tool-generated outputs

## Priority Adoption Backlog

1. Add SPAC-AE industry trace and 10/32-node topology fixtures to harden trace
   import and DSE candidate-space tests.
2. Add larger DSE goldens from SPAC-AE result tables, including infeasible-space
   and dominated-candidate cases.
3. Add resource calibration fields so future real Vitis/Vivado reports can be
   compared against the clean-room estimator without claiming paper parity.
4. Harden simulator cycle semantics against SPAC-AE module timing behavior.

## Limitations

- SPAC-AE does not provide enough public raw Vitis/Vivado report evidence to
  reproduce the paper's FPGA metrics.
- The paper mentions Vitis HLS 2023.2, while SPAC-AE README mentions Vitis IDE
  2025.2 for Table 1 artifacts. Any hardware claim must record exact tool
  family and version.
- SPAC-AE-derived Rust fixtures remain below paper-reproduction evidence unless
  matching traces, configs, EDA settings, and hardware reports are present.
