# SPAC

SPAC is a clean-room engineering project inspired by the paper
[**"SPAC: Automating FPGA-based Network Switches with Protocol Adaptive
Customization"**](https://arxiv.org/html/2604.21881v1). The repository is being
built as a reproducible developer toolchain for FPGA-oriented network switch
exploration.

Current implementation status: reproducible foundation only. The project does
not yet reproduce the paper's reported FPGA metrics.

## Quickstart

```bash
cargo test --workspace
cargo run -p spac-cli -- validate --config examples/minimal/spac.project.json
cargo run -p spac-cli -- validate-protocol --protocol examples/protocols/basic.spac
cargo run -p spac-cli -- analyze-layout --protocol examples/protocols/basic.spac --bus-width 8
cargo run -p spac-cli -- analyze-layout --protocol examples/protocols/hft.spac --bus-width 64
```

Expected validation output:

```json
{
  "status": "ok",
  "schema_version": "spac.project.v0"
}
```

## Scope

The first implementation slice establishes:

- Rust workspace and CLI entrypoint.
- Versioned project configuration model.
- Stable JSON diagnostics for validation errors.
- Minimal SPAC protocol DSL parser.
- Semantic binding validation for `routing_key`.
- Protocol layout metadata model with bit offsets and flit-boundary detection.
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
