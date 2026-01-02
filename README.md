# GMTI-Rust

GMTI-Rust is a Rust+Qt rewrite of the legacy AESADIRP/AIRRADAR GMTI pipeline. The goal is to keep the original Ground Moving Target Indicator methodology (range FFT → Doppler FFT → CFAR) while adding ownership safety, async-friendly ingestion, and a modern visualizer that supports both offline datasets and real-time telemetry.

## What is GMTI and how it works here
- **Pulse Repetition Interval (PRI) stream:** The system ingests a burst of complex-valued samples that represent radar returns over multiple transmit-receive cycles. These samples are consumed by `generator::profile` (synthetic), `simulator::workflow`, or the `GuiBridge` ingestion endpoints.
- **Range processing:** `gmticore::processing::range::RangeStage` mimics the legacy `GPD_RangeProcessing` by computing per-range-bin power profiles. Each tap is windowed and accumulated to build the initial range power spectrum, which is the first step toward detecting moving targets.
- **Doppler processing:** `DopplerStage` applies FFTs across the tap dimension using `rustfft` helpers (`math::fft`). This reveals velocity information; stationary clutter clusters around zero Doppler while moving targets produce sidebands or peaks at nonzero bins.
- **Clutter suppression & detection:** `ClutterStage` implements a CFAR-like threshold by scanning Doppler bins, estimating local noise floors, and flagging cells that exceed the dynamic threshold. Detection metadata (`DetectionRecord`) is collected for logs, GUI annotations, and telemetry.
- **Visualization:** The Qt `StatusGraph` renders the range power profile waveform and overlays detection counts/annotations so users can inspect when moving targets appear. The `VisualizationModel` and `DataProvider` poll the Warp server (`/payload`) to keep the GUI in sync with whatever data stream is active.

The README below guides a new user from setup to successful offline/real-time execution while explaining how each component reflects the GMTI conceptually.

## Repository layout

```
GMTI-Rust/
├─ core/                # `gmticore` crate, DSP math, telemetry, AGP models, processing stages
├─ simulator/            # CLI workflows, generator, YAML config runner, Warp HTTP bridge
├─ ui/qt/               # Qt6 visualizer (DataProvider, StatusGraph, InputConfigurator)
├─ docs/                # requirements, validation checklists, testing/coverage guidance
├─ tools/               # automation scripts and stored offline logs/data
├─ architecture.md      # architecture tree + roadmap
├─ workspace.toml       # cargo workspace definition
└─ README.md            # this guide
```

## Prerequisites
1. **Rust toolchain:** install [rustup](https://rustup.rs) and set `rustc 1.92.0` / `cargo 1.92.0`.
2. **Qt 6 (Widgets + Network):** required for the visualization. Set `CMAKE_PREFIX_PATH` (e.g., `%QTDIR%` on Windows or `/opt/Qt/6.x/clang_64` on Linux).
3. **CMake ≥3.16** plus a matching C++ compiler for the Qt build.
4. **Python 3** (optional) for helper automation scripts such as `tools/scripts/run_offline_scenarios.py`.

## Building
### Rust workspace
```sh
cd GMTI-Rust
cargo fmt
cargo clippy
cargo test
cargo build --bins
```
This builds the `gmticore` library and the `simulator` binary.

### Qt visualizer
```sh
cd GMTI-Rust/ui/qt
mkdir -p build && cd build
cmake .. -DCMAKE_PREFIX_PATH="<path-to-Qt6>"
cmake --build .
```
The resulting `gmti_visualizer` polls `http://127.0.0.1:9000/payload` for telemetry.

## Offline workflows
1. Run default offline scenario:
   ```sh
   cargo run --bin simulator -- --offline --workflow simulator/configs/default.yaml
   ```
   This generates synthetic PRI data, runs range→Doppler→clutter stages, logs metrics to `tools/data/offline_detection.log`, and exposes visualization payloads for Qt.
2. Customize scenarios:
   - Edit or add YAML files under `simulator/configs/` (e.g., `stare.yaml`, `scan.yaml`). Each file configures taps, range/doppler bins, frequency, noise, seed, and description.
   - Use `tools/scripts/run_offline_scenarios.py` to POST each YAML to `/ingest-config`. The simulator rebuilds the DSP pipeline for every case, ensuring the generator, stages, and GUI reflect each configuration.
   - Regenerate baselines with `tools/scripts/regen_baselines.sh` which loops through scenarios and appends detection metadata to `tools/data/offline_detection.log`.
3. Interpreting outputs:
   - **Power profile:** `tools/data/offline_detection.log` records profiles lengths and detection counts; the GUI `StatusGraph` overlays the current waveform.
   - **Detections:** Every CFAR detection produces metadata (range bin, doppler bin, anomaly notes) visible in logs and the Qt status badges.
   - **Payload:** `GET http://127.0.0.1:9000/payload` returns JSON with `power_profile` (array of power floats) and `detection_count`.
4. Synthetic data generator ensures you can validate the pipeline end-to-end without live radar inputs.

## Real-time/staging workflow
1. Start the HTTP bridge (Warp server):
   ```sh
   cargo run --bin simulator -- --serve
   ```
2. Launch `gmti_visualizer`.
3. Use the Qt Input Configurator to:
   - Start/stop the simulator server.
   - Select a scenario (loads YAML metadata) or adjust taps/range/doppler/frequency/noise interactively.
   - POST generator payloads to `/ingest-config` for synthetic runs, or post raw PRI frames to `/ingest` using the legacy AGP schema (`gmticore::agp_interface::PriPayload`).
   - Observe live detection results and power-profile traces in real time on the StatusGraph.

## Inputs & outputs overview
- **Inputs:** Real-time TCP/Cloud POSTs to `/ingest`, offline YAML workflows, synthetic generator payloads, and legacy configuration templates (`GoDatabase.txt`, `ProjectLogin`, `Make*` files) that feed `WorkflowRunner`.
- **Outputs:** Structured telemetry (power profiles, detection counts, CFAR notes), `GMTEVisualizationModel` payloads, baseline logs (`tools/data/offline_detection.log`), and Qt GUI graphs/badges that reflect realtime data interpretation.

## Testing and coverage
- `cargo test` runs unit tests for `gmticore` and `simulator`.
- `tools/scripts/run_offline_scenarios.py` exercises YAML cases against `/ingest-config`.
- Coverage (≥80%) is tracked via Linux/macOS tools (`cargo-tarpaulin`, `grcov`); follow `docs/coverage_report.md` for setup and reporting.

## Documentation & legacy mapping
- `docs/legacy_mapping.md` maps each legacy AESADIRP/AIRRADAR file or pipeline stage to the corresponding Rust module.
- `architecture.md` describes the workspace layout, data flow, and roadmap so new contributors can trace the GMTI methodology through the modern stack.
- `docs/validation.md` lists parity checklists to ensure new code maintains the legacy input/output behavior.

Reading this README and the referenced docs should equip any new user to build, run, and interpret GMTI-Rust in offline and real-time scenarios while understanding how the core GMTI algorithms are implemented.
