# SPAC FPGA Validation Ladder

This repository uses an explicit evidence ladder so software-only progress is
not confused with synthesis evidence or real-hardware validation.

## Trust Levels

| Trust level | Meaning | Minimum evidence |
|---|---|---|
| `software_model` | parser, layout, metadata, and simulator outputs only | deterministic local artifacts and tests |
| `hls_csim` | generated HLS artifacts survive C simulation | generated artifacts, csim harness, csim logs |
| `post_synthesis` | timing/resource evidence is parsed and attributable | tool versions, raw reports, normalized report output |
| `hardware_measured` | results are measured on a real board | board profile, bitstream identity, runtime evidence |

Only the highest fully supported trust level may be claimed for a given run.

## Experiment Ladder

| Stage | Name | Trust level | Current status | Purpose |
|---|---|---|---|---|
| `E0` | Software contract check | `software_model` | runnable now | verify protocol, layout, metadata, and manifests |
| `E1` | Software simulator check | `software_model` | runnable now | verify trace-driven forwarding, VOQ, scheduler, and metrics reports |
| `E2` | Bounded DSE frontier check | `software_model` | runnable now | verify deterministic DSE reports and manifests |
| `E3` | HLS artifact contract check | `software_model` | runnable now | generate deterministic parser/header artifacts |
| `E4` | HLS csim smoke | `hls_csim` | package harness runnable; execution requires Vitis HLS | verify generated artifacts under HLS C simulation |
| `E5` | Synthesis report ingestion | `post_synthesis` | synthetic fixture parser runnable; real reports unavailable | normalize timing and resource reports |
| `E6` | Timing/resource acceptance | `post_synthesis` | runnable for parsed reports | check constraints against parsed report evidence |
| `E7` | Board bring-up and packet loopback | `hardware_measured` | planned | verify packet correctness on a real FPGA path |
| `E8` | Applied workload replay | `hardware_measured` | planned | capture latency, drop, occupancy, and throughput evidence |

## Paper-Aligned Board Target

The default paper-aligned target is:

- board: `AMD Alveo U45N`
- FPGA part: `xcu26-vsva1365-2LV-e`
- toolchain: `Vitis HLS 2023.2`
- target clock: `350 MHz`

The tracked public example contract is:

- `examples/contracts/board-profile.alveo-u45n.json`
- operator checklist: `docs-public/fpga-first-light.md`

## Fallback Board Policy

Boards other than the paper-aligned U45N target are allowed only through an
explicit `spac.board-profile.v0` artifact. Fallback-board results must:

- keep trust levels honest
- downgrade any paper-parity interpretation
- record exact board, part, toolchain, and report provenance

## Allowed Claims by Evidence Level

| Highest evidence level present | Allowed claim scope |
|---|---|
| `software_model` | parser/layout correctness and software-model outputs only |
| `hls_csim` | generated HLS contract viability in csim only |
| `post_synthesis` | timing/resource observations for a known toolchain/board target |
| `hardware_measured` | board-specific measured packet behavior and workload evidence |

This repository does not currently claim paper-result reproduction or hardware
parity.

## Post-Synthesis Acceptance

`spac accept-hw-report --report <hw_report.json> --constraints <constraints.json> --out <dir>`
compares `spac.hw-report.v0` resource, timing, and initiation-interval metrics
against `spac.constraints.v0`. Trace-dependent p99 latency and packet drop
constraints are recorded as `not_evaluated` unless a later measured workload
evidence format supplies those metrics.

## Experiment Packaging

`spac package-experiment --run-dir <dir> --board-profile <path> --trust-level <level> --out <dir>`
packages an evidence directory into `spac.experiment-run.v0` with file hashes,
board identity, trust level, and explicit limitations. Packaging evidence does
not upgrade the trust level; it only records what evidence is already present.
