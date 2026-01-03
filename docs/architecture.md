# Advanced GMTI Architecture & Roadmap

## Workspace Layout
```
GMTI-Rust
│
├─ .vscode/
│   ├─ launch.json       # debugger + visualizer/simulator run shortcuts
│   ├─ settings.json     # formatting/linting rules for Rust binaries
│   └─ tasks.json        # cargo fmt/clippy/test helpers
│
├─ core/                # gmticore crate (formerly core) encapsulates DSP invariants
│   ├─ Cargo.toml
│   └─ src/
│       ├─ lib.rs
│       ├─ prelude.rs
│       ├─ agp_interface/
│       │   ├─ pri.rs           # PriPayload, ancillary metadata and converters
│       │   └─ detection.rs     # DetectionRecord/CFAR thresholds
│       ├─ math/
│       │   ├─ fft.rs           # rustfft helpers mirroring common/spl
│       │   ├─ stats.rs         # stats helpers (RMS, centroid, etc.)
│       │   └─ matrix.rs        # ndarray helpers for SPL-like ops
│       ├─ processing/
│       │   ├─ buffer_pool.rs   # scoped buffer reuse replacing raw arrays
│       │   ├─ range.rs         # RangeStage trait implementation
│       │   ├─ doppler.rs       # DopplerStage (FFT, magnitude, notes)
│       │   └─ clutter.rs       # ClutterStage (CFAR/detection counts)
│       └─ telemetry/
│           ├─ metrics.rs       # structured logs matching legacy traces
│           └─ log.rs           # csv/json writers for runner & GUI bridge
│
├─ simulator/           # workflow + generator + GUI bridge
│   ├─ Cargo.toml
│   ├─ configs/
│   │   └─ default.yaml     # taps/range/doppler defaults
│   └─ src/
│       ├─ main.rs          # CLI (offline mode, workflow loader, baselines)
│       ├─ generator/
│       │   ├─ profile.rs        # sine/simulated PRI builder for testing
│       │   └─ template.rs       # waveform templates (e.g., sine)
│       ├─ workflow/
│       │   ├─ config.rs         # YAML loader to StageConfig
│       │   └─ runner.rs         # Range → Doppler → Clutter orchestration
│       └─ gui_bridge/
│           ├─ bridge.rs         # Warp `GET /payload` + `POST /ingest`
│           └─ model.rs          # VisualizationModel (power profile, counts)
│
├─ ui/
│   └─ assets/              # future media or shared resources
├─ visualizer/
│   ├─ Cargo.toml
│   └─ src/
│       └─ main.rs          # iced-based GUI (config panel + status graph)
│
├─ docs/
│   ├─ requirements.md       # functional + safety + visualization requirements
│   ├─ validation.md         # checklist for parity, GUI sync, telemetry
│   ├─ testing.md            # testing/coverage strategy (80%+ target)
│   ├─ legacy_mapping.md     # AESADIRP/AIRRADAR → Rust traceability
│   └─ README.md             # high-level description
│
├─ tools/
│   ├─ scripts/
│   │   └─ regen_baselines.sh   # replays configs → `simulator` → `tools/data`
│   └─ data/
│       └─ offline_detection.log # regression output (detection counts, profiles)
│
├─ tests/
│   ├─ unit/                   # placeholder for future cargo test targets
│   └─ integration/
│
├─ Cargo.toml
├─ Cargo.lock
└─ workspace.toml
```

## Data Flow & Interfaces

### Inputs
- **Real-time TCP/Cloud ingestion:** `GuiBridge` exposes `POST /ingest` (JSON) so modern data sources can push PRI frames; the payload uses `gmticore::agp_interface::PriPayload`, preserving AESADIRP handshake semantics (`EXT_SipCsciInterface`).  
- **Offline mode:** `simulator` can be launched with `--offline --workflow <yaml>` using configs in `simulator/configs` (or an existing replay archive), ensuring the same config/calibration values (`taps`, `range_bins`, `doppler_bins`) as legacy `config/ProjectLogin` + `Makesite.*`.  
- **Synthetic data generator:** `generator::profile::build_pri_payload` produces sine-wave samples with metadata for visualization and regression. This mock data satisfies visual inspection requirements until real telemetry arrives.
- **Legacy files/configs:** AESADIRP's `mdaRecordRawFormat` outputs and AIRRADAR `config/Make*`/`GoDatabase.txt` inform stage parameters, channel counts, and runtime calibrations captured in YAML/workflow artifacts.

### Outputs
- **Structured telemetry:** Each stage emits `StageMetadata` (power profiles, notes, detection counts) consumed by `GuiBridge` and `WorkflowRunner` logs (`tools/data/offline_detection.log`).  
- **Visualization payload:** `GET /payload` now serves `VisualizationModel` with the power profile, detection count, `detection_records` (range/doppler/SNR tuples), and `detection_notes` so the Rust visualizer can render the polar detection map and textual logs.
- **Baseline logs:** `tools/scripts/regen_baselines.sh` drives offline configs and appends summaries to `tools/data/offline_detection.log`, mimicking legacy `.out` regression logs.  
- **Console/GUI traces:** `GuiBridge` prints status updates and errors; the Rust visualizer renders waveforms, detection counters, and exposes configuration controls for both offline datasets and live streams.

## Legacy Correspondence

- **AESADIRP_935_picosar** contains the AGP handshake (`control/sip/src/agp.c`, `common/ext/src/EXT_SipCsciInterface.h`, `agp/src/AGP_Main`), raw recorder utilities, and delivery/install instructions. Rust reuses that theory through `gmticore::agp_interface`, `generator` dummy payloads, and `GuiBridge` (Warp HTTP endpoints replicating AGP semantics).  
- **AIRRADAR_351_cpp** houses the GPD processing pipeline (`gmti/gpd`, `common/spl`, `stg`, etc.), math helpers, and calibration templates. Rust mirrors it via `gmticore::processing::{RangeStage,DopplerStage,ClutterStage}`, `gmticore::math`, and `BufferPool`, retaining CFAR thresholds, FFT math, and detection metadata while adding ownership guarantees and telemetry.  
- **Common configs/templates** (`GoDatabase.txt`, `template/config/Make*`, `ProjectLogin`) feed into `WorkflowConfig` and `WorkflowRunner`, so the modern YAML-driven flows remain compatible with the legacy environment names and tuning.

## Implementation Highlights

- **OOP-friendly trait system:** `gmticore::prelude::ProcessingStage` defines `initialize/execute/cleanup`; each stage implements the trait, which keeps the lifecycle extensible and enforces consistent error handling.  
- **Ownership-safe buffers:** `processing::BufferPool` recreates the legacy SPL buffer pools with Rust’s borrow checker, eliminating double-free/use-after-free risks.  
- **Dedicated math helpers:** `math::fft`, `math::stats`, and `math::matrix` wrap `rustfft`/`ndarray` but expose interfaces analogous to the old SPL utilities, making the algorithmic logic easier to trace during the upcoming code comparison.  
- **Workflow orchestration:** `simulator::workflow::Runner` runs range → doppler → clutter sequentially, harvests `StageMetadata`, and funnels detection counts into both logs and the GUI payload.  
- **Real-time bridge & Rust visualizer:** `GuiBridge` spins up a Warp HTTP server (9000) to feed the `visualizer` over `http://127.0.0.1:9000/payload`. Both offline and live modes reuse the same `VisualizationModel`, ensuring the UI always reflects the current data stream.  
- **Scenario-driven generator endpoint:** `POST /ingest-config` lets the visualizer’s Input Config panel describe offline test cases (taps/bins/frequency/noise/seed) and delegates PRI sample generation to `generator::profile` before rerunning Range→Doppler→Clutter, so generated data exercises the same DSP code as the legacy pipeline.  
- **Offline test configurator:** The new Rust visualizer panel lets operators point to the workspace, launch `cargo run --bin simulator -- --serve`, load YAML scenarios, adjust taps/range/doppler/noise, and POST generated CPI payloads to `/ingest`, while the dark-themed StatusGraph renders the power profile and detection count in real time.  
- **Generator for offline validation:** `generator::profile` produces deterministic sine-wave CPI data; future enhancements will add PRI metadata, noise, multi-tap bursts, and configuration files so we can visually validate inputs before real data arrives.  
- **Documentation-driven parity:** `docs/legacy_mapping.md` already aligns legacy files with Rust modules and will expand to capture per-file reasoning, honoring the requirement to compare code (not just outputs).

## Plan & Roadmap (incremental steps 1–5)

1. **Capture the existing architecture** (this document + `docs/legacy_mapping.md`) to prove how AESADIRP/AIRRADAR theory flows into the Rust workspace, documenting inputs, outputs, and config dependencies.  
2. **Secure the core math/processing pipeline** by verifying `Range/Doppler/Clutter` stages, buffer pools, and telemetry helpers against the legacy SPL/GMTP implementations, then extend tests and docs to cover any refinements.  
3. **Extend data generation & workflow automation**—expand `generator` to model PRI metadata, multi-tap bursts, and noise, tune `simulator/configs/*.yaml`, and run `tools/scripts/regen_baselines.sh` to seed `tools/data/offline_detection.log` for all scenarios.  
4. **Instrument coverage & testing** by integrating `docs/testing.md` (cargo fmt/clippy/test) plus coverage tooling (`cargo tarpaulin`/`grcov`) and aim for ≥80% coverage with automation in `docs/coverage_report.md`.  
5. **Validate & compare code**—keep `docs/legacy_mapping.md` and `docs/validation.md` updated with structured checklists, then iterate the Rust visualizer, workflow runner, and HTTP bridge so both real-time (POST/GET) and offline flows stay consistent with the legacy pipelines.

## Testing & Coverage Notes

- `docs/testing.md` already lists core tests (`core/src/processing/*`, `core/src/math/*`, `simulator/src/{generator,workflow,gui_bridge}`) and coverage benchmarks.  
- Run `cargo fmt`, `cargo clippy`, and `cargo test` before launching the Rust visualizer; once coverage tooling is in place (see `docs/coverage_report.md`), document the ≥80% results there.  
- Gradually add integration tests (e.g., `tests/integration`) that POST mock payloads to `GuiBridge` and assert the visualizer updates accordingly.

## Validation & Comparison Notes

- `docs/validation.md` offers a checklist covering Rust parity (logs), GUI sync, workflow configs, telemetry/logging, visualizer bridging, and baseline automation. Use it to confirm the system still accepts both recorded data files and live TCP/HTTP sources, producing the same detection/power-profile metadata as AESADIRP/AIRRADAR.  
- Continue documenting modernizations (structured telemetry, GUI bridge, HTTP endpoints) in `docs/legacy_mapping.md` so future contributors know which Rust modules correspond to legacy files.  
- The new Rust visualizer and `GuiBridge` ensure public-facing usability, while the generator plus workflow runner guarantee offline capability—even before real data arrives.

## Next Actions

- Expand the generator to cover multi-tap PRI scenarios with noise and metadata, store the resulting payloads for `POST /ingest`, and log matching visual states.  
- Integrate coverage tooling (`cargo tarpaulin`, `grcov`) and publish results to `docs/coverage_report.md`.  
- Keep `docs/legacy_mapping.md` in sync with each refactor so the Rust rewrite stays faithful to AESADIRP/AIRRADAR at the file-and-algorithm level.  
- Map additional legacy configs (e.g., `GoDatabase.txt`, `ProjectLogin`, `Makefile` templates) into the workflow configs so engineers can plug real configuration/calibration files directly into `WorkflowRunner`.  
- Once the Rust visualizer is stable, plan for more advanced instrumentation (telemetry dashboard, advanced visual exploration) while preserving the core detection pipeline guarantees.
