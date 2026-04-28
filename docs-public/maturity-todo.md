# Maturity TODO to 10/10

This backlog defines what must be true before the project can be rated 10/10 in
engineering maturity, article-faithful SPAC architecture coverage, and hardware
evidence. The list is intentionally conservative: no item can be marked complete
from prose alone.

## Engineering Maturity 10/10

- [ ] Keep the repository release-clean: no unreviewed dirty worktree, no
  untracked public contracts, and every public artifact committed or explicitly
  ignored.
- [ ] Require CI to run formatting, clippy, full workspace tests, public
  contract checks, CLI smoke examples, schema validation, and language-policy
  scans.
- [ ] Add versioned release gates: changelog, tag, reproducibility score,
  dependency review, license review, and rollback notes for every release.
- [ ] Add coverage reporting for critical crates and enforce coverage gates for
  parser, layout, simulator, DSE, code generation, and report parsing.
- [ ] Add mutation or fault-injection checks for semantic binding, layout
  offsets, scheduler fairness, Pareto ranking, and hardware acceptance logic.
- [ ] Split large CLI surfaces into maintainable command modules without
  changing public behavior.
- [ ] Freeze schema compatibility policy with migration tests for every
  `spac.*.v0` public contract.
- [ ] Add deterministic benchmark lanes for trace import, simulation throughput,
  DSE candidate scaling, and report parsing.
- [ ] Add signed or checksum-pinned release artifacts for generated examples and
  packaged experiment bundles.
- [ ] Add documented maintainer procedures for issue triage, security reports,
  release approval, and evidence review.

## Article-Faithful SPAC Architecture 10/10

- [ ] Validate the local `.spac` protocol grammar against any public SPAC or
  NetBlocks examples and document every deliberate divergence.
- [ ] Model the full protocol-to-semantic-IR-to-architecture-config pipeline
  with stable intermediate artifacts and round-trip validation.
- [ ] Extend forwarding models to match the paper-level variants and expose
  conflict, latency, and resource proxy metrics for each variant.
- [ ] Strengthen VOQ models with calibrated `N*N`, one-buffer-per-port, shared
  buffer, per-queue depth, and per-port depth behavior.
- [ ] Strengthen scheduler models for Round-Robin, iSLIP, and EDRRM with
  starvation, fairness, and contention oracle tests.
- [ ] Implement custom-kernel contracts with explicit latency/resource hints and
  simulation hooks while keeping untrusted input non-executable.
- [ ] Implement trace-aware DSE phases that mirror the paper: static pruning,
  coarse simulation, phase-2 buffer sizing, resource pruning, and verification
  simulation.
- [ ] Add larger DSE spaces with infeasible-space goldens, dominated-candidate
  goldens, and calibrated resource fields.
- [ ] Add report-level comparisons against SPAC-AE artifact tables without
  claiming paper reproduction unless traces, configs, EDA settings, and hardware
  evidence match.
- [ ] Defer ns-3 integration until simulator, DSE, generated HLS artifacts, and
  FPGA report ingestion are stable and independently tested.

## Hardware Evidence 10/10

- [ ] Run generated HLS packages through Vitis HLS 2023.2 or a declared
  compatible toolchain and store raw csim logs with manifests.
- [ ] Add HLS synthesis runs with raw reports, normalized `spac.hw-report.v0`
  outputs, tool versions, board profile, constraints, and command provenance.
- [ ] Add post-synthesis acceptance gates for LUT, FF, BRAM, DSP, Fmax, II,
  latency, and throughput constraints.
- [ ] Preserve raw Vitis/Vivado reports as provenance-tagged fixtures when
  licensing allows redistribution; otherwise store hashes and acquisition
  instructions.
- [ ] Produce a board-specific build package for the AMD Alveo U45N paper-aligned
  target or clearly downgrade any non-U45N comparison.
- [ ] Capture first-light evidence: board identity, bitstream identity, PCIe/DMA
  setup, clock settings, loopback logs, and packet correctness results.
- [ ] Add hardware-measured packet loopback tests with zero-mismatch acceptance
  criteria and reproducible input/output packet manifests.
- [ ] Run applied workload replay on real hardware and capture latency, drop
  rate, throughput, queue occupancy, and scheduler behavior.
- [ ] Calibrate software-model and DSE estimates against post-synthesis and
  hardware-measured evidence; record residual error bounds.
- [ ] Claim paper-level reproduction only after matching traces, preprocessing,
  architecture configs, EDA settings, board target, and measurement methodology
  are available and checked into the evidence chain.
