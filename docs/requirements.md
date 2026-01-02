## Requirements Summary

1. **Safety & concurrency** – Rust ownership and explicit buffer pooling prevent legacy memory issues, while async-friendly crates (`tokio`) keep future pipelines ready for streaming.
2. **Object-oriented design** – Trait-based stages (range, Doppler, clutter) implement `ProcessingStage`, providing a composable lifecycle with `initialize`, `execute`, and `cleanup`.
3. **Visualization & UX** – A Qt/C++ frontend consumes Rust-generated telemetry served by `GuiBridge` over HTTP (`http://127.0.0.1:9000/payload`), recreating the power-profile/detection views in a modern UI.
4. **Offline workflow** — The `simulator` CLI and generator modules synthesise PRI/CPI vectors, load configuration files (see `simulator/configs/default.yaml`), and run them through the pipeline to produce deterministic logs for regression.  
5. **Interactive offline testing** — Qt’s `InputConfigurator` now lets operators point to their workspace, start/stop the `simulator --serve` bridge, load YAML scenarios, tweak tap/range/doppler/noise settings, and run realistic offline CPI cases while the StatusGraph renders the resulting power profile and detection count in real time with a modern dark theme.  
6. **Interoperability** — The Rust AGP interface mirrors `EXT_SipCsciInterface` semantics (`PriType`, `DetectionRecord`), so legacy consumers can adapt or translate as needed.
6. **Workflow persistence** – The `workflow::Runner` loads YAML definitions, feeds them through the pipeline, and writes pass/fail reports (`tools/data/offline_detection.log`) so archived workloads can be replayed.
7. **Real-time ingestion** – `GuiBridge` exposes `POST /ingest` (JSON) to accept TCP/cloud-style PRI frames while simultaneously serving `GET /payload`, enabling modern input streams without altering the core algorithms.
