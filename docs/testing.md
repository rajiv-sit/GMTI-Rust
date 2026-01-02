# Testing & Coverage Strategy

## Unit Tests

Each core module exposes dedicated tests (see `core/src/processing/{range,doppler,clutter}.rs` and `core/src/math/{fft,stats}.rs`), plus the simulator layers include generator, workflow config, runner, and GUI bridge tests (`simulator/src/generator/profile.rs`, `simulator/src/workflow/{config,runner}.rs`, `simulator/src/gui_bridge/bridge.rs`). Running `cargo test` currently exercises every crate and will fail fast if any core assumption changes.

## Coverage Goal

- Target: **≥80% coverage**; strive for ~100% by writing tests that trigger every branch in critical modules.  
- Measurement: on Linux or macOS, use [`cargo tarpaulin`](https://github.com/xd009642/tarpaulin) or `cargo llvm-cov` to generate coverage reports; on Windows you can use `grcov`+`cargo test --tests` and report the percentage in `docs/coverage_report.md`.  
- Continuous improvement: add new tests whenever a refactor touches the indexed logic (e.g., additional detection heuristics), and record the resulting coverage snapshot in the repo so we can trace regressions.

## Automation

The base commands:
```
cargo fmt
cargo clippy
cargo test
```

For coverage reporting (on Unix-like machines):
```
cargo tarpaulin --out Xml  # use report in CI
```
or
```
cargo llvm-cov --workspace --all-targets --features "..." --no-run
```

Document any discrepancies between the desired coverage and the achieved one in `docs/coverage_report.md`, and keep iterating until the ≥80% threshold is satisfied.
