# Legacy to Rust Mapping

This project keeps the essential theory and intent of the AESADIRP/AIRRADAR C++ code while recasting it in Rust. Key correspondences include:

- `selex/agp.c`, `control/sip/src/agp.c`, and `common/ext/src/EXT_SipCsciInterface.h` → `simulator::gui_bridge` plus `gmticore::agp_interface`, preserving the PRI/detection record layout, CFAR constants, and the AGP handshake protocol.
- `gmti/gpd/src/GPD_RangeProcessing.*`, `GPD_DopplerProcessing.*`, and `GPD_ExoClutterDetection.*` → `gmticore::processing::RangeStage` / `DopplerStage` / `ClutterStage`, with scoped SPL-like buffers (`BufferPool`), FFT/magnitude helpers (`gmticore::math::fft`), and telemetry of RMS/detection counts.
- `common/spl` (vectors, FFT, math utilities) → `gmticore::math` (FFT helper, matrix ops, stats); the new helpers expose similar capabilities with `ndarray`, `rustfft`, and explicit scalars.
- `common/tcs` utilities are referenced via the documentation and will feed into future telemetry/helper crates; for now, the generator/workflow modules ingest configuration metadata similar to TCS parameters.
- `gmti/gpd/test/*.cc` scripts → `tools/scripts/regen_baselines.sh`, `tools/data/offline_detection.log`, and `simulator`’s `WorkflowRunner`, ensuring regression data is still reproducible.
- Real-time ingestion paths (`GET /payload`, `POST /ingest`) mimic the constant AGP handshake and feed the Rust pipeline the same telemetry that previously flowed through the SELEX AGP bridge.
- Build/config details (`config/Makesite.unix.*`, `ProjectLogin`, `Readme.txt`, etc.) inform environment setup instructions in `docs/requirements.md` and `architecture.md`, keeping the original build flags, core paths, and DB aliases (e.g., `AIRRADARCOREHOME`, `go` helper) accessible to the modern stack.
