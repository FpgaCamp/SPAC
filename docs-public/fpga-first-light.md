# SPAC FPGA First-Light Checklist

This checklist is the public operator-facing derivative of the local
engineering playbook. It defines the minimum evidence needed to progress from
software-only validation to real-FPGA experiments without overstating
confidence.

Current implementation reality:

- `E0` through `E3` are runnable with the tracked Rust workspace today
- `E4` package generation and `E5` synthetic report parsing are runnable, but
  higher-trust evidence still requires Vitis/Vivado execution outputs
- `E7` through `E8` remain planned stages gated by acceptance gates, generated
  hardware artifacts, or real board evidence

## Default Paper-Aligned Target

Use this setup when the goal is the closest available alignment with the paper:

- board: `AMD Alveo U45N`
- FPGA part: `xcu26-vsva1365-2LV-e`
- toolchain: `Vitis HLS 2023.2`
- target clock: `350 MHz`

Reference contract:

- `examples/contracts/board-profile.alveo-u45n.json`

Fallback boards are allowed only through an explicit `spac.board-profile.v0`
artifact and must downgrade any paper-parity interpretation.

## Evidence Ladder Checklist

| Stage | Prerequisites | Required artifacts | Pass criteria | Kill criteria | Rollback criteria |
|---|---|---|---|---|---|
| `E0` | clean checkout, passing Rust toolchain | CLI test summary, metadata output, manifest when artifacts are written | tests pass, outputs are deterministic, diagnostics are stable | parser/layout/config regressions or nondeterministic metadata | revert the last parser/layout/config change |
| `E1` | E0 passes, trace input validates | simulation report, manifest | deterministic `spac.simulation-run.v0`, complete manifest, explicit `software_model` trust level | nondeterministic report or missing manifest | keep simulator evidence out of release claims |
| `E2` | E1 passes, DSE space and constraints validate | DSE result, manifest | deterministic `spac.dse-result.v0`, explicit `software_model` trust level, non-dominated frontier | nondeterministic DSE report or missing limitations | keep DSE evidence out of release claims |
| `E3` | metadata exists | generated parser/header artifacts, manifest | byte-stable generated artifacts, complete manifest | generated artifacts drift or require hand-edited source | disable HLS generation surface and stay at metadata/simulator evidence |
| `E4` | E3 passes, csim harness exists | csim logs, fixture inputs, manifest | csim exits cleanly and decodes fixtures correctly | csim build failure or decode mismatch | demote evidence back to `software_model` |
| `E5` | E4 path exists or reports are externally available | raw reports, parsed report JSON, tool/version record | LUT, FF, BRAM, DSP, Fmax, II, latency, throughput parse cleanly | unknown tool/version or ambiguous report attribution | quarantine report parsing to fixture-only mode |
| `E6` | E5 passes, constraints are explicit | parsed reports, constraints artifact, pass/fail summary | every gate is judged explicitly against declared constraints | missing constraints or inconsistent report evidence | keep evidence at report-only level and block acceptance claims |
| `E7` | bitstream exists, board path exists, loopback path is wired | board profile, bitstream identity, loopback logs, manifest | zero packet mismatches and explicit board metadata | bring-up instability, missing loopback, unexplained mismatches | keep status blocked and do not promote to hardware-ready |
| `E8` | E7 passes, workload traces exist | trace bundle, board profile, telemetry, run summary | latency, drop, occupancy, and throughput evidence recorded with trust level | unstable replay path or missing telemetry | fall back to E7-only hardware correctness evidence |

## Commands and Operator Actions

Implemented now:

```bash
cargo test --workspace
cargo run -p spac-cli -- validate --config examples/minimal/spac.project.json
cargo run -p spac-cli -- check-config --architecture examples/contracts/architecture.hft_full_lookup_rr.json
cargo run -p spac-cli -- check-constraints --constraints examples/contracts/constraints.paper_aligned_u45n.json
cargo run -p spac-cli -- check-board-profile --board-profile examples/contracts/board-profile.alveo-u45n.json
cargo run -p spac-cli -- check-trace --trace examples/contracts/trace.hft_tiny.json
cargo run -p spac-cli -- validate-protocol --protocol examples/protocols/basic.spac
cargo run -p spac-cli -- analyze-layout --protocol examples/protocols/hft.spac --bus-width 64
cargo run -p spac-cli -- generate-metadata --protocol examples/protocols/basic.spac --bus-width 8 --out out/basic
cargo run -p spac-cli -- generate-hls-traits --metadata out/basic/metadata.json --out out/hls-basic
cargo run -p spac-cli -- package-hls-csim --metadata out/basic/metadata.json --board-profile examples/contracts/board-profile.alveo-u45n.json --out out/hls-csim-basic
cargo run -p spac-cli -- parse-hw-report --tool vitis --report examples/contracts/hw-report.vitis_hls_summary.rpt --board-profile examples/contracts/board-profile.alveo-u45n.json --out out/hw-report-vitis
cargo run -p spac-cli -- package-experiment --run-dir out/hw-report-vitis --board-profile examples/contracts/board-profile.alveo-u45n.json --trust-level post_synthesis --out out/experiment-hw-report-vitis
cargo run -p spac-cli -- simulate --architecture examples/contracts/architecture.hft_full_lookup_rr.json --trace examples/contracts/trace.hft_tiny.json --out out/sim-hft
cargo run -p spac-cli -- dse --space examples/contracts/dse-space.tiny.json --trace examples/contracts/trace.dse_tiny_burst.json --constraints examples/contracts/constraints.dse_tiny_lenient.json --out out/dse-tiny
```

## Allowed Claims by Stage

- `E0`: parser/layout/metadata correctness and software-model evidence only
- `E1`: trace-driven simulator behavior only, not hardware performance
- `E2`: DSE ranking behavior only, not hardware performance
- `E3` and `E4`: HLS contract viability only, not hardware performance
- `E5` and `E6`: toolchain- and board-target-specific timing/resource observations
- `E7` and `E8`: board-specific measured packet behavior and workload evidence

The tracked repository must not claim paper-result reproduction or hardware
parity until `hardware_measured` evidence exists.
