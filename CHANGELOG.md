# Changelog

## Unreleased

- Created the initial Rust workspace foundation.
- Added `spac validate --config <path>`.
- Added project and artifact manifest schemas.
- Added governance, architecture, quality, security, and reproducibility docs.
- Added tracked public engineering docs for maturity and FPGA validation.
- Added public versioned schemas and example fixtures for architecture,
  constraints, traces, board profiles, and experiment runs.
- Added `spac check-config --architecture <path>`.
- Added `spac check-constraints --constraints <path>`.
- Added `spac check-board-profile --board-profile <path>`.
- Added `spac check-trace --trace <path>`.
- Added `spac generate-metadata --protocol <path> --bus-width <bits> --out <dir>`.
- Added `spac simulate --architecture <path> --trace <path> --out <dir>`.
- Added manifest emission for metadata artifact generation.
- Added manifest emission for simulation reports.
- Added `spac-trace` and `spac-sim` crates for MVP-B trace validation and
  deterministic software-model simulation.
- Added golden simulation fixtures for HFT, datacenter incast, and underwater
  burst scenarios.
- Added a tiny-space brute-force simulator oracle for future DSE frontier
  validation.
- Added `spac.dse-space.v0`, `spac.dse-result.v0`, `spac-dse`, and
  `spac dse --space <path> --trace <path> --constraints <path> --out <dir>`
  for bounded software-model Pareto ranking.
- Added `spac-codegen` and
  `spac generate-hls-traits --metadata <path> --out <dir>` for deterministic
  generated `packet.hpp`-style HLS traits with manifests.
- Added a tracked FPGA first-light checklist.
- Added `spac import-spac-ae-trace` for SPAC-AE CSV trace/topology import into
  `spac.trace.v0`.
- Added `one_buffer_per_port` VOQ support and SPAC-AE-derived latency/II,
  line-rate, throughput, utilization, and peak-VOQ simulator metrics.
- Added `spac generate-spac-ae-dse-space` and clean-room Rust SPAC-AE resource
  estimates for generated DSE candidate spaces.
- Added SPAC-AE artifact oracle fixtures for full HFT trace, 8-node topology,
  port-scan resource table, and DSE result table.
- Added optional `spac dse --spac-ae-phase2-buffers` buffer optimization with
  per-queue/per-port VOQ depth vectors and explicit phase-2 metadata.
- Documented SPAC-AE provenance and compatibility limits without claiming paper
  metric reproduction.
- Added `spac package-hls-csim` and `spac.hls-csim-run.v0` for deterministic
  generated HLS csim smoke packages, with `hls_csim` trust level reserved for
  successful explicit Vitis HLS execution only.
- Added `spac parse-hw-report` and `spac.hw-report.v0` for fixture-backed
  Vitis/Vivado report ingestion at `post_synthesis` trust level.
- Added `spac accept-hw-report` and `spac.hw-acceptance.v0` for
  post-synthesis constraint acceptance gates over parsed hardware reports.
- Added `spac package-experiment` for runtime-backed
  `spac.experiment-run.v0` evidence bundles with manifests.
- Added SPAC-AE comparative analysis and an active-development README notice
  that directs issues to the project repository.
