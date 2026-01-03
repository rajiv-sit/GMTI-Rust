# PyQt6 Visualizer

This PyQt6-based companion visualizer mirrors the Rust UI’s layout while providing a richer, more interactive 3D exploration surface.

## Features

- Real-time polling of `http://127.0.0.1:9000/payload`, recreating the live power profile and detection stream.
- Config panel with the same parameters as the Rust visualizer plus a streaming button that posts to `/ingest-config` every second for ten minutes.
- 3D scatter view powered by `pyqtgraph.opengl`, letting you pan/zoom/rotate detections with a mouse.
- Detection table, metadata teaser, and status/log output to keep analysts up to speed with the ongoing scenario.

## Getting started

1. Install the dependencies (ideally inside a virtualenv):
   ```bash
   cd pyqt_visualizer
   pip install -r requirements.txt
   ```
   > **Tip:** these requirements target Python 3.11/3.12 because prebuilt `numpy` wheels are available; running under 3.14 currently triggers a source build that fails if `setuptools.build_meta` isn’t present. If you must use 3.14, install the `setuptools`/`wheel` packages (`python -m pip install -U pip setuptools wheel`) before `pip install -r requirements.txt`, or switch to one of the earlier interpreters to avoid builds.
   > **Note:** PyOpenGL is now part of the dependencies so the 3D scatter view can import `OpenGL.GL`; the requirements file includes it.
2. Keep the Rust simulator running (`cargo run --bin simulator -- --serve`) so the HTTP endpoints stay available.
3. Launch this visualizer:
   ```bash
   python main.py
   ```
4. Use the form to POST a scenario and optionally click “Start 10-min run” to stream generator configs every second.
5. Drag inside the 3D view to rotate and scroll/wheel to zoom; the widgets stay synchronized for a consistent experience with the Rust UI.

Because the visualizer polls `/payload`, make sure your simulator or `tools/scripts/stream_iq_dataset.py` feed is active before relying on detections.
