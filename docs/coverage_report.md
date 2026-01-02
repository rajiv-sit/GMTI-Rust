# Coverage Report

## Goal
Maintain ≥80% instruction coverage for the Rust workspace; ideally, approach 100% by binding every branch in the core DSP and simulator stack.

## Latest run
- `cargo test` (Windows 10, January 2026) — all unit suites succeeded, but the coverage toolchain (`cargo tarpaulin`/`grcov`) could not run on this platform. Record results here once Linux/macOS tooling is available.

## How to generate coverage
1. On Linux/macOS, install [`cargo-tarpaulin`](https://github.com/xd009642/tarpaulin): `cargo install cargo-tarpaulin`.
2. Run `cargo tarpaulin --workspace --out Xml`. Inspect `tarpaulin-report.xml` for coverage percentage and copy the summary into this file.
3. Optionally run `cargo llvm-cov --workspace --all-targets --no-run` if you prefer LLVM’s HTML reports.

Record each coverage run beneath “Latest run” so we can track regressions and ensure the 80% minimum remains satisfied.
