# SPAC Engineering Overview

SPAC is being built as a clean-room Rust-first developer tool for FPGA-oriented
network-switch exploration. The public repository currently implements the
MVP-B software-model foundation, an initial bounded MVP-C DSE path, and the
contracts that later FPGA-validation stages will depend on.

## Current Repo vs Target Product

| Capability | Target state | Current public repo | Maturity |
|---|---|---|---|
| Product direction | developer tool via research-ready skeleton | aligned | strong |
| Language policy | Rust/TypeScript only | aligned | strong |
| Protocol frontend | stable human-authored DSL | `.spac` parser implemented | strong |
| Semantic binding | explicit routing semantic contract | `routing_key` validation implemented | strong |
| Layout analysis | deterministic offsets and boundary checks | implemented | strong |
| Metadata generation | reusable packet metadata contract | implemented with manifest-backed artifact emission | strong |
| Architecture config | versioned switch-policy schema | runtime validation plus public schema | strong |
| Constraints config | versioned DSE/hardware gate schema | runtime validation plus public schema | strong |
| Board profile | versioned board/toolchain evidence schema | runtime validation plus public schema | strong |
| Trace normalization | versioned machine-facing workload schema | runtime validation implemented | strong |
| Simulator | software-only switch model | deterministic forwarding/VOQ/scheduler model with golden scenarios implemented | partial |
| DSE | trace-aware candidate ranking | bounded candidate-space ranking implemented at software-model fidelity | partial |
| HLS bridge | generated `packet.hpp`-style traits | deterministic generated artifact command implemented | partial |
| FPGA evidence | report parsing and hardware-ready packaging | report parsing, acceptance, and experiment bundle packaging implemented | partial |
| Real-FPGA validation | staged experiment ladder | public validation plan only | partial |

## Selected Product Direction

The selected build order is fixed:

1. `MVP-A`: protocol DSL, semantic binding, layout analysis, metadata
2. `MVP-B`: software-only switch simulator
3. `MVP-C`: trace-aware DSE over explicit architecture and constraints contracts

Non-goals for the current repo:

- no claim of paper-result reproduction
- no claim of hardware parity without synthesis or board evidence
- no additional implementation-language path
- no public API or dashboard before CLI and artifact contracts are stable

## Public Contract Surfaces

The tracked schemas under `configs/schemas/` define the normalized public
machine-facing surfaces for the next stages:

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

Examples for these contracts live under `examples/contracts/`.

Important rule:

- `.spac` remains the implemented human-facing protocol frontend
- JSON contracts are additive downstream/public contracts
- this repository does not pivot to another implementation language or a
  renamed product surface

The maturity backlog for reaching 10/10 engineering, architecture, and hardware
evidence scores is tracked in [maturity-todo.md](maturity-todo.md).

## CLI Status

Implemented now:

- `spac validate --config <path>`
- `spac check-config --architecture <path>`
- `spac check-constraints --constraints <path>`
- `spac check-board-profile --board-profile <path>`
- `spac check-trace --trace <path>`
- `spac validate-protocol --protocol <path>`
- `spac analyze-layout --protocol <path> --bus-width <bits> [--out <dir>]`
- `spac generate-metadata --protocol <path> --bus-width <bits> --out <dir>`
- `spac generate-hls-traits --metadata <path> --out <dir>`
- `spac package-hls-csim --metadata <path> --board-profile <path> --out <dir>`
- `spac parse-hw-report --tool <tool> --report <path> --board-profile <path> --out <dir>`
- `spac accept-hw-report --report <hw_report.json> --constraints <constraints.json> --out <dir>`
- `spac simulate --architecture <path> --trace <path> --out <dir>`
- `spac dse --space <path> --trace <path> --constraints <path> --out <dir>`
- `spac package-experiment --run-dir <dir> --board-profile <path> --trust-level <level> --out <dir>`
