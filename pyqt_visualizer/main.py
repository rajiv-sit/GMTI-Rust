"""PyQt6 visualization companion for GMTI-Rust."""

import math
import time
from dataclasses import dataclass
from typing import Any, Dict, List, Optional, Tuple

import numpy as np
import pyqtgraph as pg
import pyqtgraph.opengl as gl
import requests
from PyQt6.QtCore import QObject, QRunnable, QThreadPool, QTimer, Qt, pyqtSignal
from PyQt6.QtGui import QColor, QFont, QPalette
from PyQt6.QtWidgets import (
    QApplication,
    QGridLayout,
    QGroupBox,
    QHBoxLayout,
    QLabel,
    QLineEdit,
    QPushButton,
    QSlider,
    QTableWidget,
    QTableWidgetItem,
    QTextEdit,
    QVBoxLayout,
    QWidget,
)

try:
    from pyqt_visualizer.geometry import project_detection_coordinates
except ModuleNotFoundError:  # pragma: no cover
    from geometry import project_detection_coordinates

PAYLOAD_URL = "http://127.0.0.1:9000/payload"
CONFIG_URL = "http://127.0.0.1:9000/ingest-config"
POLL_INTERVAL_MS = 1000
STREAM_DURATION_SECS = 600


class WorkerSignals(QObject):
    """Signals emitted by background workers."""

    result = pyqtSignal(object)
    error = pyqtSignal(str)


class FetchPayloadTask(QRunnable):
    """Fetches telemetry from the simulator endpoint."""

    def __init__(self) -> None:
        super().__init__()
        self.signals = WorkerSignals()

    def run(self) -> None:
        try:
            response = requests.get(PAYLOAD_URL, timeout=2)
            response.raise_for_status()
            self.signals.result.emit(response.json())
        except Exception as err:  # pragma: no cover
            self.signals.error.emit(str(err))


class PostConfigTask(QRunnable):
    """Posts generator config to the simulator."""

    def __init__(self, payload: Dict[str, Any]) -> None:
        super().__init__()
        self.payload = payload
        self.signals = WorkerSignals()

    def run(self) -> None:
        try:
            response = requests.post(CONFIG_URL, json=self.payload, timeout=5)
            response.raise_for_status()
            self.signals.result.emit(response.json())
        except Exception as err:  # pragma: no cover
            self.signals.error.emit(str(err))


@dataclass
class FieldSpec:
    label: str
    default: str
    caster: Optional[Any] = str


class PyQtVisualizer(QWidget):
    """Main PyQt6-based visualization window."""

    def __init__(self) -> None:
        super().__init__()
        pg.setConfigOption("background", "k")
        pg.setConfigOption("foreground", "w")
        self.setWindowTitle("GMTI PyQt6 Visualizer")
        self.thread_pool = QThreadPool()
        self.stream_timer = QTimer()
        self.stream_timer.timeout.connect(self._stream_tick)
        self.payload_timer = QTimer()
        self.payload_timer.timeout.connect(self._poll_payload)
        self.payload_timer.start(POLL_INTERVAL_MS)

        self.form_fields: Dict[str, QLineEdit] = {}
        self.field_specs: Dict[str, FieldSpec] = {}
        self.stream_running = False
        self.stream_remaining = 0
        self.stream_elapsed = 0
        self.stream_start_ts = time.time()
        self.history: List[str] = []
        self.camera_zoom_factor = 1.0
        self.target_distance = 12000.0
        self.manual_zoom_active = False
        self.view_mode = "polar"
        self.last_detection_records: List[Dict[str, Any]] = []
        self.last_metadata: Dict[str, Any] = {}

        self._apply_dark_theme()
        self._setup_ui()
        self.resize(1500, 900)

    def _setup_ui(self) -> None:
        """Create the window layout."""
        layout = QHBoxLayout(self)

        config_group = QGroupBox("Input Config")
        config_layout = QGridLayout()
        field_specs = [
            ("taps", FieldSpec("Taps", "4", int)),
            ("range_bins", FieldSpec("Range bins", "2048", int)),
            ("doppler_bins", FieldSpec("Doppler bins", "256", int)),
            ("frequency", FieldSpec("Frequency (Hz)", "1050000000", float)),
            ("noise", FieldSpec("Noise floor", "0.07", float)),
            ("seed", FieldSpec("Seed", "312", int)),
            ("description", FieldSpec("Description", "Airborne PyQt6 scenario")),
            ("scenario", FieldSpec("Scenario name", "Airborne sweep")),
            ("platform_type", FieldSpec("Platform type", "Airborne ISR")),
            ("platform_velocity_kmh", FieldSpec("Platform velocity (km/h)", "750", float)),
            ("altitude_m", FieldSpec("Altitude (m)", "8200", float)),
            ("area_width_km", FieldSpec("Surveillance width (km)", "10", float)),
            ("area_height_km", FieldSpec("Surveillance height (km)", "10", float)),
            ("clutter_level", FieldSpec("Clutter level (0-1)", "0.45", float)),
            ("snr_target_db", FieldSpec("Target SNR (dB)", "18", float)),
            ("interference_db", FieldSpec("Interference (dB)", "-10", float)),
            ("target_motion", FieldSpec("Target motion summary", "Cruise, gentle zig-zag")),
        ]
        self.field_specs = {name: spec for name, spec in field_specs}

        for row, (name, spec) in enumerate(field_specs):
            label = QLabel(spec.label)
            edit = QLineEdit(spec.default)
            edit.setPlaceholderText(spec.label)
            config_layout.addWidget(label, row, 0)
            config_layout.addWidget(edit, row, 1)
            self.form_fields[name] = edit

        self.post_button = QPushButton("POST scenario")
        self.post_button.clicked.connect(lambda: self._send_config())
        self.stream_button = QPushButton("Start 10-min run")
        self.stream_button.clicked.connect(self._toggle_stream)
        self.status_label = QLabel("Waiting for telemetry...")
        self.stream_status_label = QLabel("Streaming idle.")

        config_layout.addWidget(self.post_button, len(field_specs), 0, 1, 2)
        config_layout.addWidget(self.stream_button, len(field_specs) + 1, 0, 1, 2)
        config_layout.addWidget(self.stream_status_label, len(field_specs) + 2, 0, 1, 2)
        config_layout.addWidget(self.status_label, len(field_specs) + 3, 0, 1, 2)
        config_group.setLayout(config_layout)
        config_group.setMaximumWidth(380)

        telemetry_layout = QVBoxLayout()
        telemetry_layout.setSpacing(12)

        self.detection_info_label = QLabel("Detections: n/a")
        telemetry_layout.addWidget(self.detection_info_label)

        self.power_plot = pg.PlotWidget(title="Power profile")
        self.power_plot.showGrid(x=True, y=True, alpha=0.3)
        self.power_curve = self.power_plot.plot(pen=pg.mkPen(color=(30, 200, 255), width=2))
        telemetry_layout.addWidget(self.power_plot, stretch=1)

        self.detection_table = QTableWidget(0, 6)
        self.detection_table.setHorizontalHeaderLabels(
            ["#", "Range (m)", "Bearing (deg)", "Elevation (deg)", "Doppler (m/s)", "SNR (dB)"]
        )
        telemetry_layout.addWidget(self.detection_table, stretch=1)

        self.metadata_label = QLabel("<b>Scenario metadata</b><br>No metadata yet.")
        self.metadata_label.setWordWrap(True)
        telemetry_layout.addWidget(self.metadata_label)

        self.detection_summary_label = QLabel("No detections yet.")
        self.detection_summary_label.setWordWrap(True)
        telemetry_layout.addWidget(self.detection_summary_label)

        self.detection_count_label = QLabel("Detections per iteration: n/a")
        telemetry_layout.addWidget(self.detection_count_label)

        self.gl_view = gl.GLViewWidget()
        self.gl_view.opts["distance"] = 5000
        self.gl_view.setMinimumSize(900, 600)
        self.gl_view.setBackgroundColor((0, 0, 0, 255))
        self.grid = gl.GLGridItem()
        self.grid.setSize(6000, 6000, 1)
        self.grid.setSpacing(100.0, 100.0, 1)
        self.gl_view.addItem(self.grid)
        self.axis_item = gl.GLAxisItem()
        self.axis_item.setSize(6000, 6000, 6000)
        self.gl_view.addItem(self.axis_item)
        self.scatter = gl.GLScatterPlotItem(size=6, pxMode=True)
        self.scatter.setGLOptions("additive")
        self.gl_view.addItem(self.scatter)
        self.sphere_mesh = gl.MeshData.sphere(rows=12, cols=12)
        self.detection_spheres: List[gl.GLMeshItem] = []
        self.detection_labels: List[gl.GLTextItem] = []
        self.gl_view.setCameraPosition(distance=12000, elevation=30, azimuth=45)
        telemetry_layout.addWidget(self.gl_view, stretch=3)

        view_buttons = QHBoxLayout()
        self.polar_button = QPushButton("Polar view")
        self.polar_button.setCheckable(True)
        self.polar_button.setChecked(True)
        self.cart_button = QPushButton("Cartesian view")
        self.cart_button.setCheckable(True)
        self.polar_button.clicked.connect(lambda: self._change_view_mode("polar"))
        self.cart_button.clicked.connect(lambda: self._change_view_mode("cartesian"))
        view_buttons.addWidget(self.polar_button)
        view_buttons.addWidget(self.cart_button)
        telemetry_layout.addLayout(view_buttons)

        self.zoom_label = QLabel("Zoom (3D): auto")
        self.zoom_slider = QSlider(Qt.Orientation.Horizontal)
        self.zoom_slider.setRange(60, 200)
        self.zoom_slider.setValue(100)
        self.zoom_slider.setSingleStep(5)
        self.zoom_slider.valueChanged.connect(self._camera_zoom_changed)
        telemetry_layout.addWidget(self.zoom_label)
        telemetry_layout.addWidget(self.zoom_slider)

        self.axis_label = QLabel()
        self.axis_label.setWordWrap(True)
        telemetry_layout.addWidget(self.axis_label)

        self.log_widget = QTextEdit()
        self.log_widget.setReadOnly(True)
        self.log_widget.setFixedHeight(120)
        telemetry_layout.addWidget(self.log_widget)

        self.info_label = QLabel(
            "For real telemetry run `cargo run --bin simulator -- --serve`, "
            "then `curl http://127.0.0.1:9000/payload` to inspect each detection's "
            "range, bearing_deg, elevation_deg, doppler, and snr."
        )
        self.info_label.setWordWrap(True)
        telemetry_layout.addWidget(self.info_label)

        layout.addWidget(config_group)
        layout.addLayout(telemetry_layout, stretch=1)

    def _poll_payload(self) -> None:
        """Kick off a payload fetch."""
        task = FetchPayloadTask()
        task.signals.result.connect(self._handle_payload)
        task.signals.error.connect(self._handle_payload_error)
        self.thread_pool.start(task)

    def _send_config(self, timestamp: Optional[float] = None) -> None:
        """Post the current config to the simulator."""
        payload = self._build_payload(timestamp)
        task = PostConfigTask(payload)
        task.signals.result.connect(self._handle_post_success)
        task.signals.error.connect(self._handle_post_error)
        self.thread_pool.start(task)

    def _toggle_stream(self) -> None:
        if self.stream_running:
            self._stop_stream()
        else:
            self._start_stream()

    def _start_stream(self) -> None:
        self.stream_running = True
        self.stream_remaining = STREAM_DURATION_SECS
        self.stream_elapsed = 0
        self.stream_start_ts = time.time()
        self.stream_status_label.setText(f"Streaming run: {self.stream_remaining}s remaining")
        self.stream_button.setText("Stop 10-min run")
        self._send_config(timestamp=self.stream_start_ts)
        self.stream_timer.start(POLL_INTERVAL_MS)

    def _change_view_mode(self, mode: str) -> None:
        self.view_mode = mode
        self.polar_button.setChecked(mode == "polar")
        self.cart_button.setChecked(mode == "cartesian")
        self._update_3d_view(self.last_detection_records, self.last_metadata)

    def _stop_stream(self) -> None:
        self.stream_running = False
        self.stream_timer.stop()
        self.stream_status_label.setText("Streaming idle.")
        self.stream_button.setText("Start 10-min run")

    def _stream_tick(self) -> None:
        if self.stream_remaining <= 0:
            self._stop_stream()
            self.status_label.setText("Streaming run complete.")
            return
        timestamp = self.stream_start_ts + self.stream_elapsed
        self.stream_elapsed += 1
        self.stream_remaining -= 1
        self.stream_status_label.setText(f"Streaming run: {self.stream_remaining}s remaining")
        self._send_config(timestamp=timestamp)

    def _camera_zoom_changed(self, value: int) -> None:
        self.camera_zoom_factor = value / 100.0
        self.manual_zoom_active = value != 100
        self.zoom_label.setText(f"Zoom (3D): {self.camera_zoom_factor:.2f}x")
        self._apply_zoom()

    def _apply_zoom(self) -> None:
        if self.manual_zoom_active:
            distance = max(self.target_distance * self.camera_zoom_factor, 3000.0)
        else:
            distance = max(self.target_distance, 3000.0)
        self.gl_view.opts["distance"] = distance
        zoom_mode = f"{self.camera_zoom_factor:.2f}x" if self.manual_zoom_active else "auto"
        self.zoom_label.setText(f"Zoom (3D): {zoom_mode} ({distance / 1000:.1f} km)")
        self.gl_view.update()

    def _handle_payload(self, payload: Dict[str, Any]) -> None:
        self.status_label.setText("Telemetry received.")
        detection_count = payload.get("detection_count")
        power_bins = payload.get("power_profile") or []
        detection_records = payload.get("detection_records") or []
        metadata = payload.get("scenario_metadata") or {}
        detection_count = detection_count or len(detection_records)
        self.detection_info_label.setText(
            f"Detections: {detection_count} / {len(power_bins)} bins"
        )
        self.detection_count_label.setText(f"Detections per iteration: {detection_count}")
        self._update_power_profile(power_bins)
        self._update_detection_table(detection_records, metadata)
        self._update_metadata(metadata)
        self.last_detection_records = detection_records
        self.last_metadata = metadata
        self._update_3d_view(detection_records, metadata)
        self._update_detection_summary(detection_records)
        self._append_log(f"Telemetry: {detection_count} detections, {len(power_bins)} bins.")

    def _handle_payload_error(self, message: str) -> None:
        self.status_label.setText("Telemetry error.")
        self._append_log(f"Telemetry error: {message}")

    def _handle_post_success(self, result: Dict[str, Any]) -> None:
        status = result.get("status", "ok")
        detections = result.get("detections")
        self._append_log(f"Config posted: {status} ({detections or 'n/a'} detections).")

    def _handle_post_error(self, message: str) -> None:
        self.status_label.setText("Config error.")
        self._append_log(f"Config error: {message}")

    def _build_payload(self, timestamp: Optional[float]) -> Dict[str, Any]:
        def safe_cast(value: str, caster: Any) -> Optional[Any]:
            try:
                return caster(value)
            except (ValueError, TypeError):
                return None

        payload: Dict[str, Any] = {}
        for name, spec in self.field_specs.items():
            raw = self.form_fields[name].text().strip()
            if not raw:
                payload[name] = None
                continue
            if spec.caster is int:
                payload[name] = safe_cast(raw, int)
            elif spec.caster is float:
                payload[name] = safe_cast(raw, float)
            else:
                payload[name] = raw
        payload["timestamp_start"] = timestamp
        return payload

    def _apply_dark_theme(self) -> None:
        palette = QPalette()
        palette.setColor(QPalette.ColorRole.Window, QColor("#0f1220"))
        palette.setColor(QPalette.ColorRole.WindowText, QColor("#f3f6ff"))
        palette.setColor(QPalette.ColorRole.Base, QColor("#1e2235"))
        palette.setColor(QPalette.ColorRole.AlternateBase, QColor("#232741"))
        palette.setColor(QPalette.ColorRole.ToolTipBase, QColor("#fdfdfd"))
        palette.setColor(QPalette.ColorRole.ToolTipText, QColor("#ffffff"))
        palette.setColor(QPalette.ColorRole.Text, QColor("#fdfdff"))
        palette.setColor(QPalette.ColorRole.Button, QColor("#1f1e33"))
        palette.setColor(QPalette.ColorRole.ButtonText, QColor("#f8fbff"))
        palette.setColor(QPalette.ColorRole.Highlight, QColor("#3d6cfb"))
        palette.setColor(QPalette.ColorRole.HighlightedText, QColor("#ffffff"))
        self.setPalette(palette)
        self.setStyleSheet(
            """
            QWidget { background-color: #0f1220; color: #f3f6ff; }
            QGroupBox { border: 1px solid #2f3347; margin-top: 16px; }
            QGroupBox::title { color: #cfd3ff; padding: 0 6px; }
            QPushButton { background-color: #1e1f2b; color: #fefeff; border: 1px solid #2b3050; padding: 6px; }
            QPushButton:hover { background-color: #272d52; }
            QLineEdit, QTableWidget, QTextEdit { background-color: #111428; color: #f8fbff; border: 1px solid #2d3251; }
            QLabel { color: #f6f7ff; }
            QTextEdit { border-radius: 4px; }
            """
        )

    def _update_power_profile(self, samples: List[float]) -> None:
        if samples:
            x = np.arange(len(samples), dtype=float)
            y = np.array(samples, dtype=float)
            self.power_curve.setData(x, y)
        else:
            self.power_curve.clear()

    def _update_detection_table(
        self, records: List[Dict[str, Any]], metadata: Dict[str, Any]
    ) -> None:
        if not records:
            self.detection_table.setRowCount(0)
            return
        max_rows = min(len(records), 12)
        self.detection_table.setRowCount(max_rows)
        for row in range(max_rows):
            record = records[row]
            self.detection_table.setItem(row, 0, QTableWidgetItem(str(row + 1)))
            self.detection_table.setItem(
                row, 1, QTableWidgetItem(f"{record.get('range', 0):.1f}")
            )
            elevation = self._resolve_detection_elevation(record, metadata)
            self.detection_table.setItem(
                row, 2, QTableWidgetItem(f"{record.get('bearing_deg', 0):.1f}")
            )
            self.detection_table.setItem(
                row, 3, QTableWidgetItem(f"{elevation:.1f}")
            )
            self.detection_table.setItem(
                row, 4, QTableWidgetItem(f"{record.get('doppler', 0):.2f}")
            )
            self.detection_table.setItem(
                row, 5, QTableWidgetItem(f"{record.get('snr', 0):.2f}")
            )

    def _update_detection_summary(self, records: List[Dict[str, Any]]) -> None:
        if not records:
            self.detection_summary_label.setText("No detections to visualize.")
            return
        top = sorted(records, key=lambda rec: rec.get("snr", 0), reverse=True)[:3]
        summary = "Top detections:\n"
        summary += "\n".join(
            f"#{idx + 1}: range {rec.get('range', 0):.1f} m, bearing {rec.get('bearing_deg', 0):.1f}°, "
            f"elevation {self._resolve_detection_elevation(rec, self.last_metadata):.1f}°, doppler {rec.get('doppler', 0):.2f} m/s, "
            f"SNR {rec.get('snr', 0):.2f} dB"
            for idx, rec in enumerate(top)
        )
        self.detection_summary_label.setText(summary)

    def _update_metadata(self, metadata: Dict[str, Any]) -> None:
        if not metadata:
            self.metadata_label.setText("<b>Scenario metadata</b><br>No metadata yet.")
            return
        html = (
            "<b>Scenario metadata</b><br>"
            f"Name: {metadata.get('name', 'n/a')}<br>"
            f"Platform: {metadata.get('platform_type', 'n/a')}<br>"
            f"Velocity: {metadata.get('platform_velocity_kmh', 'n/a')} km/h<br>"
            f"Area: {metadata.get('area_width_km', 'n/a')} x {metadata.get('area_height_km', 'n/a')} km<br>"
            f"Clutter: {metadata.get('clutter_level', 'n/a')}<br>"
            f"SNR target: {metadata.get('snr_target_db', 'n/a')} dB<br>"
            f"Interference: {metadata.get('interference_db', 'n/a')} dB<br>"
            f"Target motion: {metadata.get('target_motion', 'n/a')}"
        )
        self.metadata_label.setText(html)

    def _extract_area_value(
        self, metadata: Dict[str, Any], primary: str, fallback: str, default: float
    ) -> float:
        raw = metadata.get(primary, metadata.get(fallback))
        if raw is None:
            return default
        try:
            return float(raw)
        except (TypeError, ValueError):
            return default

    def _resolve_detection_elevation(
        self, record: Dict[str, Any], metadata: Dict[str, Any]
    ) -> float:
        base = float(record.get("elevation_deg", 0.0))
        if abs(base) > 1e-3:
            return base
        rng = float(record.get("range", 0.0))
        altitude = self._extract_area_value(
            metadata, "altitude_m", "platform_altitude_m", 4000.0
        )
        if rng <= 0.0 or altitude <= 0.0:
            return base
        ratio = min(altitude / rng, 1.0)
        return -math.degrees(math.asin(ratio))

    def _axis_ticks_label(self, min_km: float, max_km: float) -> str:
        span = max(max_km - min_km, 0.1)
        step = max(span / 10.0, 0.1)
        ticks = [min_km + i * step for i in range(11)]
        return " | ".join(f"{tick:+.1f} km" for tick in ticks)

    def _update_3d_view(self, records: List[Dict[str, Any]], metadata: Dict[str, Any]) -> None:
        half_span_x_km = max(
            self._extract_area_value(metadata, "area_width_km", "area_width", 10.0), 10.0
        )
        half_span_y_km = max(
            self._extract_area_value(metadata, "area_height_km", "area_height", 10.0), 10.0
        )
        half_span_x_m = half_span_x_km * 1000.0
        half_span_y_m = half_span_y_km * 1000.0
        grid_width = half_span_x_m * 2.0
        grid_height = half_span_y_m * 2.0

        spacing = 100.0  # 0.1 km grid spacing
        spacing_x = spacing
        spacing_y = spacing
        self.grid.setSize(grid_width, grid_height, 1)
        self.grid.setSpacing(spacing_x, spacing_y, 1)
        axis_z = max(grid_width, grid_height) * 0.5
        self.axis_item.setSize(grid_width, grid_height, axis_z)
        new_target_distance = max(grid_width, grid_height, axis_z, 6000.0)
        distance_changed = abs(new_target_distance - self.target_distance) > 1e-3
        self.target_distance = new_target_distance
        if distance_changed or not self.manual_zoom_active:
            self._apply_zoom()

        x_ticks = self._axis_ticks_label(-half_span_x_km, half_span_x_km)
        y_ticks = self._axis_ticks_label(-half_span_y_km, half_span_y_km)
        self.axis_label.setText(
            f"Area covers +/-{half_span_x_km:.1f} km (X) x +/-{half_span_y_km:.1f} km (Y)\n"
            f"X axis ticks: {x_ticks}\n"
            f"Y axis ticks: {y_ticks}"
        )

        if not records:
            self.scatter.setData(
                pos=np.array([[0, 0, 0]], dtype=float),
                size=np.array([3.0]),
                color=np.array([[1.0, 0.2, 0.2, 0.9]]),
            )
            self.gl_view.update()
            self._clear_detection_spheres()
            return

        max_doppler = max((abs(float(record.get("doppler", 0))) for record in records), default=0.5)
        max_doppler = max(max_doppler, 0.5)
        visible_records = records[:256]
        max_range_value = max((float(record.get("range", 0)) for record in visible_records), default=1.0)
        max_range_value = max(max_range_value, 1.0)
        positions = []
        colors = []
        sizes = []
        area_radius = half_span_x_m
        platform_altitude = self._extract_area_value(
            metadata, "altitude_m", "platform_altitude_m", 4000.0
        )
        detection_infos: List[Dict[str, float]] = []
        for idx, record in enumerate(visible_records, start=1):
            elevation = self._resolve_detection_elevation(record, metadata)
            projection_record = dict(record)
            projection_record["elevation_deg"] = elevation
            snr = float(record.get("snr", 0))
            x, y = project_detection_coordinates(
                projection_record,
                metadata,
                self.view_mode,
                half_span_x_m,
                half_span_y_m,
                area_radius,
                max_range_value,
                max_doppler,
            )
            rng = float(record.get("range", 0))
            elevation_rad = math.radians(elevation)
            detection_altitude = platform_altitude + rng * math.sin(elevation_rad)
            z = max(0.0, detection_altitude)
            positions.append((x, y, z))
            colors.append((1.0, 0.2, 0.2, 0.95))
            sizes.append(np.clip(3 + snr * 0.16, 4, 18))
            detection_infos.append(
                {
                    "id": idx,
                    "range": rng,
                    "bearing": float(record.get("bearing_deg", 0)),
                    "elevation": elevation,
                }
            )
        self.scatter.setData(
            pos=np.array(positions, dtype=float),
            color=np.array(colors, dtype=float),
            size=np.array(sizes, dtype=float),
            pxMode=True,
        )
        self._update_detection_spheres(positions, detection_infos)
        self.gl_view.update()

    def _clear_detection_spheres(self) -> None:
        while self.detection_spheres:
            item = self.detection_spheres.pop()
            self.gl_view.removeItem(item)
        self._clear_detection_labels()

    def _clear_detection_labels(self) -> None:
        while self.detection_labels:
            label = self.detection_labels.pop()
            self.gl_view.removeItem(label)

    def _update_detection_spheres(
        self,
        positions: List[Tuple[float, float, float]],
        infos: List[Dict[str, float]],
    ) -> None:
        self._clear_detection_spheres()
        sphere_radius = 5.0
        text_offset = sphere_radius + 2.0
        for (x, y, z), info in zip(positions, infos):
            sphere = gl.GLMeshItem(
                meshdata=self.sphere_mesh,
                smooth=True,
                color=(1.0, 0.3, 0.3, 0.8),
                shader="shaded",
                glOptions="additive",
            )
            sphere.scale(sphere_radius, sphere_radius, sphere_radius)
            sphere.translate(x, y, z)
            self.gl_view.addItem(sphere)
            self.detection_spheres.append(sphere)
            label = gl.GLTextItem()
            label.setData(
                pos=np.array([x, y, z + text_offset]),
                text=(
                    f"#{info['id']}: {info['range']:.1f} m, "
                    f"B{info['bearing']:.1f}° E{info['elevation']:.1f}°"
                ),
                color=pg.mkColor(255, 255, 255, 255),
                font=QFont("Helvetica", 10),
            )
            self.gl_view.addItem(label)
            self.detection_labels.append(label)

    def _append_log(self, message: str) -> None:
        timestamp = time.strftime("%H:%M:%S")
        entry = f"{timestamp} - {message}"
        self.history.append(entry)
        if len(self.history) > 20:
            self.history.pop(0)
        self.log_widget.setPlainText("\n".join(self.history))


def main() -> None:
    app = QApplication([])
    window = PyQtVisualizer()
    window.show()
    app.exec()


if __name__ == "__main__":
    main()
