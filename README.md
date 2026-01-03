# GMTI-Rust Overview

GMTI-Rust is a Rust-based GMTI pipeline that keeps the legacy DSP methodology (Range → Doppler → CFAR) while adding safer ownership rules, async-friendly ingestion, baseline automation, and a cross-platform Rust visualizer built with `iced`.

## Repository layout

```
GMTI-Rust/
├─ core/        # gmticore crate: AGP models, math utilities, processing stages
├─ simulator/    # CLI/workflows, generator, YAML configs, Warp bridge
├─ visualizer/   # Rust/iced UI (config panel + status graph)
├─ docs/         # requirements, validation, testing, legacy mapping
├─ tools/        # automation scripts + offline logs/data
├─ architecture.md
├─ workspace.toml
└─ README.md     # this guide
```

## Key components
- **Input ingestion:** offline YAML workflows, synthetic generator payloads, HTTP `/ingest-config`, and `/ingest` align with the legacy AGP values.  
- **Core pipeline:** `gmticore::processing::{RangeStage, DopplerStage, ClutterStage}` implements the classical FFT + CFAR flow with ownership-safe buffer pools and telemetry helpers.  
- **Rust visualizer:** the `visualizer` crate polls `http://127.0.0.1:9000/payload`, displays the power profile, detection counts, and provides a config panel that POSTs back to `/ingest-config`.
- **Automation:** `tools/scripts/run_offline_scenarios.py` iterates YAML decks, `tools/scripts/regen_baselines.sh` rewrites regression logs, and `docs/coverage_report.md` tracks ≥80% coverage.

## Getting started
1. Install the Rust toolchain (`rustup`, `rustc 1.92.0`, `cargo 1.92.0`).  
2. `cd GMTI-Rust && cargo fmt && cargo clippy && cargo test`.  
3. Launch the simulator for offline or real-time execution (`cargo run --bin simulator`).  
4. Run the visualizer (`cargo run --bin visualizer`) and use its Input Config panel to exercise scenarios; the StatusGraph updates every second.

## Running modes
- **Offline:** `cargo run --bin simulator -- --offline --workflow simulator/configs/default.yaml` produces synthetic PRI, streams telemetry, and logs detections to `tools/data/offline_detection.log`.  
- **Real-time/staging:** `cargo run --bin simulator -- --serve` + `cargo run --bin visualizer` lets you tweak taps/bins/noise, POST generator configs, and observe live detection results. External systems can POST raw PRI frames to `/ingest` as well.

## Visualizer telemetry
- The visualizer now lists the meaning of every config parameter (taps, range/doppler bins, frequency, noise floor, seed, description) so you know how each value affects runtime behavior.  
- The telemetry pane draws a polar detection map (radius = range, angle = normalized Doppler) plus a textual table of `detection_records` that include range, Doppler, and SNR.  
- The detection canvas now toggles between polar and Cartesian views, offers zoom/rotation controls plus grid/label toggles, and surfaces scenario metadata tags so analysts can explore the 10 km × 10 km environment in 2D.  
- `GET /payload` returns `power_profile`, `detection_records`, `detection_notes`, and the detection count, allowing additional clients to render Cartesian scatter plots or export the same data for offline analysis.

### Figure 1 – Visualizer overview
<img width="1912" height="1033" alt="image" src="https://github.com/user-attachments/assets/c95149a8-cc08-4f90-8cad-3e00f1b85002" />
*Figure 1 shows the real-time GUI with the parameter form, waveform, polar detection map, and operational notes, representing the complete telemetry loop.*

### Running simulator + visualizer
1. **Terminal 1 (simulator server):**  
   ```powershell
   New-Item -Path 'E:\cargo-target' -ItemType Directory -Force
   cd /path/to/GMTI-Rust
   $env:CARGO_TARGET_DIR='E:\cargo-target'
   cargo run --bin simulator -- --serve
   ```
   Leave this running (Ctrl+C to stop) so `/payload` stays live for five minutes or as long as you need the GUI to poll.
   If you need to terminate the simulator from another shell (e.g., port 9000 is still bound), use PowerShell’s process commands instead of relying on Ctrl+C:
   ```powershell
   Get-Process simulator             # shows the PID
   Stop-Process -Id <PID>            # or use -Name simulator
   ```
   Alternatively, if you ran it via `cargo run` in that shell, pressing `Ctrl+C` in that same window also kills the process and frees the port.
2. **Terminal 2 (visualizer UI):**  
   ```powershell
   cd /path/to/GMTI-Rust
   $env:CARGO_TARGET_DIR='E:\cargo-target'
   cargo run --bin visualizer
   ```
   Keep this window open; the GUI repeatedly polls `http://127.0.0.1:9000/payload` and renders the waveform/detections in real time.

## PyQt6 visualizer

An experimental PyQt6 client lives in `pyqt_visualizer/` that mirrors the Rust GUI layout, adds a polar + 3D scatter view, and lets you pan/zoom/rotate detections with `pyqtgraph.opengl`.

1. Install the Python dependencies:
   ```powershell
   cd /path/to/GMTI-Rust/pyqt_visualizer
   pip install -r requirements.txt
   ```
2. Keep the simulator (`cargo run --bin simulator -- --serve`) running so `/payload` and `/ingest-config` stay live.
3. Launch the PyQt visualizer:
   ```powershell
   python main.py
   ```
4. The GUI polls the same endpoints, shows the power profile, detection table, scenario metadata, and a full 3D scatter plot. Use the form the same as the Rust version and click “Start 10-min run” to trigger repeated HTTP POSTs during streaming sessions.

The PyQt client is meant for side-by-side comparison with the Rust visualization layer; feel free to run both to evaluate their usability and performance.

## Documentation
- `architecture.md`: workspace layout, data flow, roadmap.  
- `docs/requirements.md`, `docs/validation.md`, `docs/testing.md`: requirements, parity checks, coverage strategy.  
- `docs/coverage_report.md`: tells you how to capture ≥80% coverage with tarpaulin/grcov on Linux/macOS.

