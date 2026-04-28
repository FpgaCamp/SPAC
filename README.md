# SPAC

SPAC is a clean-room engineering project inspired by the paper
[**"SPAC: Automating FPGA-based Network Switches with Protocol Adaptive
Customization"**](https://arxiv.org/html/2604.21881v1). The repository is being
built as a reproducible developer toolchain for FPGA-oriented network switch
exploration.

Current implementation status: MVP-B software-model foundation. The project can
validate architecture and trace contracts, emit deterministic metadata, and run
a deterministic switch simulator plus bounded software-model DSE ranking, but
it does not reproduce the paper's reported FPGA metrics.

> **Active development notice:** SPAC is under active development. Interfaces,
> schemas, fixtures, and evidence flows may change before a tagged release. If
> you hit a bug, broken example, missing artifact, or unclear limitation, open
> an Issue in this repository: <https://github.com/FpgaCamp/SPAC/issues>.

## Quickstart

```bash
cargo test --workspace
cargo run -p spac-cli -- validate --config examples/minimal/spac.project.json
cargo run -p spac-cli -- check-config --architecture examples/contracts/architecture.hft_full_lookup_rr.json
cargo run -p spac-cli -- check-constraints --constraints examples/contracts/constraints.paper_aligned_u45n.json
cargo run -p spac-cli -- check-board-profile --board-profile examples/contracts/board-profile.alveo-u45n.json
cargo run -p spac-cli -- check-trace --trace examples/contracts/trace.hft_tiny.json
cargo run -p spac-cli -- import-spac-ae-trace --trace examples/contracts/spac-ae.hft_trace_sample.csv --topology examples/contracts/spac-ae.dse_8nodes_sample.csv --name spac_ae_hft_sample --workload-class hft --ports 8 --out out/spac-ae-hft
cargo run -p spac-cli -- validate-protocol --protocol examples/protocols/basic.spac
cargo run -p spac-cli -- analyze-layout --protocol examples/protocols/basic.spac --bus-width 8
cargo run -p spac-cli -- analyze-layout --protocol examples/protocols/hft.spac --bus-width 64
cargo run -p spac-cli -- generate-metadata --protocol examples/protocols/basic.spac --bus-width 8 --out out/basic
cargo run -p spac-cli -- generate-hls-traits --metadata out/basic/metadata.json --out out/hls-basic
cargo run -p spac-cli -- package-hls-csim --metadata out/basic/metadata.json --board-profile examples/contracts/board-profile.alveo-u45n.json --out out/hls-csim-basic
cargo run -p spac-cli -- parse-hw-report --tool vitis --report examples/contracts/hw-report.vitis_hls_summary.rpt --board-profile examples/contracts/board-profile.alveo-u45n.json --out out/hw-report-vitis
cargo run -p spac-cli -- package-experiment --run-dir out/hw-report-vitis --board-profile examples/contracts/board-profile.alveo-u45n.json --trust-level post_synthesis --out out/experiment-hw-report-vitis
cargo run -p spac-cli -- generate-spac-ae-dse-space --ports 8 --name spac_ae_8p --out out/spac-ae-dse-space
cargo run -p spac-cli -- simulate --architecture examples/contracts/architecture.hft_full_lookup_rr.json --trace examples/contracts/trace.hft_tiny.json --out out/sim-hft
cargo run -p spac-cli -- dse --space examples/contracts/dse-space.tiny.json --trace examples/contracts/trace.dse_tiny_burst.json --constraints examples/contracts/constraints.dse_tiny_lenient.json --out out/dse-tiny
cargo run -p spac-cli -- dse --space examples/contracts/dse-space.tiny.json --trace examples/contracts/trace.dse_tiny_burst.json --constraints examples/contracts/constraints.dse_tiny_lenient.json --out out/dse-phase2 --spac-ae-phase2-buffers --phase2-top-n 1 --min-voq-depth-packets 1 --phase2-max-drop-rate 1.0
```

Expected validation output:

```json
{
  "status": "ok",
  "schema_version": "spac.project.v0"
}
```

## Current Maturity

Implemented now:

- Rust workspace and CLI foundation
- `.spac` protocol parsing with semantic `routing_key` validation
- deterministic layout analysis and `spac.metadata.v0` generation
- runtime validation for `spac.architecture.v0`
- runtime validation for `spac.constraints.v0`
- runtime validation for `spac.board-profile.v0`
- runtime validation for `spac.trace.v0`
- SPAC-AE CSV trace and single-switch topology import into `spac.trace.v0`
- deterministic `spac.simulation-run.v0` software-model reports
- golden simulation oracles for HFT, datacenter incast, and underwater burst
  software-model scenarios
- AE-derived latency/II, bus-width, line-rate, and utilization metrics in the
  software simulator
- bounded `spac.dse-space.v0` candidate evaluation and `spac.dse-result.v0`
  Pareto-frontier reports at `software_model` trust level
- SPAC-AE-style DSE candidate-space generation with clean-room Rust resource
  estimates
- SPAC-AE-style phase-2 buffer optimization that converts simulator peak VOQ
  occupancy into deterministic per-queue/per-port packet-depth vectors
- generated `packet.hpp`-style HLS trait artifacts with
  `spac.hls-traits-run.v0` reports and manifests
- deterministic HLS csim smoke packages with `packet.hpp`, `csim_smoke.cpp`,
  `hls_config.cfg`, `run_csim.tcl`, `spac.hls-csim-run.v0`, and manifests
- Vitis/Vivado FPGA report ingestion into `spac.hw-report.v0` with
  `post_synthesis` trust level, synthetic fixtures, and manifests
- post-synthesis constraint acceptance into `spac.hw-acceptance.v0` over parsed
  hardware reports
- runtime-backed experiment bundles into `spac.experiment-run.v0`
- artifact-emitting metadata generation with `manifest.json`
- public project and artifact manifest schemas
- synthetic workload protocol fixtures and golden metadata tests

Deferred hardware-facing work:

- staged real-FPGA validation with explicit `hardware_measured` evidence

Public engineering references:

- [Engineering overview](docs-public/engineering-overview.md)
- [FPGA validation ladder](docs-public/fpga-validation.md)
- [FPGA first-light checklist](docs-public/fpga-first-light.md)
- [SPAC-AE comparison and adoption plan](docs-public/spac-ae-comparison.md)
- [Maturity TODO to 10/10](docs-public/maturity-todo.md)

## Scope

The first implementation slice establishes:

- Rust workspace and CLI entrypoint.
- Versioned project configuration model.
- Versioned architecture configuration validation model.
- Stable JSON diagnostics for validation errors.
- Minimal SPAC protocol DSL parser.
- Semantic binding validation for `routing_key`.
- Protocol layout metadata model with bit offsets and flit-boundary detection.
- Deterministic metadata artifact emission with a manifest.
- Normalized trace validation.
- Software-model simulation for forwarding, VOQ, and scheduler policies.
- Software-model DSE over explicit candidate spaces and constraints.
- Workload protocol examples and golden `spac.metadata.v0` fixtures.
- Governance documents for requirements, architecture, security, and
  reproducibility.
- CI gates for formatting, linting, tests, and the Rust/TypeScript-only
  implementation policy.

## Workload Examples

Synthetic protocol examples are provided for representative SPAC workload
classes:

- `examples/protocols/hft.spac`
- `examples/protocols/rl_all_reduce.spac`
- `examples/protocols/datacenter.spac`
- `examples/protocols/industrial.spac`
- `examples/protocols/underwater_sensor.spac`

Their `bus_width=64` metadata outputs are checked in under
`examples/golden/metadata/` and verified by the Rust test suite.

## Language Policy

Implementation code must be written only in Rust or TypeScript.
Generated HLS artifacts, such as C++ headers, are allowed only as generated
outputs with an artifact manifest. They must not be hand-written source files in
this repository.

No additional implementation language is part of the implementation strategy
for this repository.

## Planned Public Contracts

Tracked normalized machine-facing contracts are provided or reserved under
`configs/schemas/`:

- `spac.project.v0`
- `spac.artifact-manifest.v0`
- `spac.architecture.v0`
- `spac.constraints.v0`
- `spac.trace.v0`
- `spac.simulation-run.v0`
- `spac.dse-space.v0`
- `spac.dse-result.v0`
- `spac.hls-traits-run.v0`
- `spac.hls-csim-run.v0`
- `spac.hw-report.v0`
- `spac.hw-acceptance.v0`
- `spac.board-profile.v0`
- `spac.experiment-run.v0`

The current implemented human-facing protocol frontend remains `.spac`. JSON
contracts are additive public surfaces for downstream simulation, DSE, and
hardware-validation stages.

Implemented artifact-oriented CLI surfaces:

- `spac check-config --architecture <path>`
- `spac check-constraints --constraints <path>`
- `spac check-board-profile --board-profile <path>`
- `spac check-trace --trace <path>`
- `spac import-spac-ae-trace --trace <trace.csv> [--topology <topology.csv>] --name <trace-name> --workload-class <class> --ports <n> --out <dir>`
- `spac generate-metadata --protocol <path> --bus-width <bits> --out <dir>`
- `spac generate-hls-traits --metadata <path> --out <dir>`
- `spac package-hls-csim --metadata <path> --board-profile <path> --out <dir> [--execute --vitis-hls-bin <path> --timeout-seconds <n>]`
- `spac parse-hw-report --tool <tool> --report <path> --board-profile <path> --out <dir>`
- `spac accept-hw-report --report <hw_report.json> --constraints <constraints.json> --out <dir>`
- `spac package-experiment --run-dir <dir> --board-profile <path> --trust-level <level> --out <dir>`
- `spac generate-spac-ae-dse-space --ports <n> [--name <name>] --out <dir>`
- `spac simulate --architecture <path> --trace <path> --out <dir>`
- `spac dse --space <path> --trace <path> --constraints <path> --out <dir>`
- `spac dse --space <path> --trace <path> --constraints <path> --out <dir> --spac-ae-phase2-buffers [--phase2-top-n <n>] [--min-voq-depth-packets <n>] [--phase2-max-drop-rate <rate>]`

## SPAC-AE Compatibility

The repository includes a clean-room Rust compatibility layer for selected
artifacts from <https://github.com/spac-proj/SPAC-AE>. It imports the SPAC-AE
trace CSV shape (`time,src_addr,dst_addr,header_size,body_size,trace_id`) and
single-switch topology CSV shape (`node_a,port_a,node_b,port_b`) into local
versioned JSON contracts. It also generates SPAC-AE-style DSE candidate spaces
using the public artifact-evaluation resource formulas. The checked-in
SPAC-AE oracle fixtures include the HFT trace, 8-node topology, DSE scan
summary, and full DSE result table from the artifact repository. Phase-2 DSE
support uses simulator peak VOQ occupancy as a packet-depth proxy for
per-queue/per-port buffer sizing.

These outputs remain `software_model` evidence. They do not reproduce paper
FPGA metrics, Vitis runs, ns-3 results, post-synthesis timing, or hardware
measurements.

## License and Attribution

This source code is licensed under the [Apache License 2.0](LICENSE). Commercial
use, modification, and redistribution are allowed under the license terms.

When redistributing this work or derivative works, preserve the license and the
attribution notices in [NOTICE](NOTICE). Attribution should identify this
repository and the original SPAC paper authors:

- Repository: <https://github.com/FpgaCamp/SPAC>
- Paper: [SPAC: Automating FPGA-based Network Switches with Protocol Adaptive
  Customization](https://arxiv.org/html/2604.21881v1)
- Paper authors: Guoyu Li, Yang Cao, Lucas H L Ng, Alexander Charlton, Qianzhou
  Wang, Will Punter, Philippos Papaphilippou, Ce Guo, Hongxiang Fan, Wayne Luk,
  Saman Amarasinghe, and Ajay Brahmakshatriya.

This project is a clean-room implementation and does not claim endorsement by,
ownership of, or reproduction parity with the paper authors' implementation.
