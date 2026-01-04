"""PyQt6 visualization companion for GMTI-Rust."""

import time
from dataclasses import dataclass
from typing import Any, Dict, List, Optional

import numpy as np
import pyqtgraph as pg
import pyqtgraph.opengl as gl
import requests
from PyQt6.QtCore import QObject, QRunnable, QThreadPool, QTimer, pyqtSignal
from PyQt6.QtGui import QPalette, QColor
from PyQt6.QtWidgets import (
    QApplication,
    QGridLayout,
    QGroupBox,
    QHBoxLayout,
    QLabel,
    QLineEdit,
    QPushButton,
    QTableWidget,
    QTableWidgetItem,
    QTextEdit,
    QVBoxLayout,
    QWidget,
)

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

        self.detection_table = QTableWidget(0, 4)
        self.detection_table.setHorizontalHeaderLabels(
            ["#", "Range (m)", "Doppler (m/s)", "SNR (dB)"]
        )
        telemetry_layout.addWidget(self.detection_table, stretch=1)

        self.metadata_label = QLabel("<b>Scenario metadata</b><br>No metadata yet.")
        self.metadata_label.setWordWrap(True)
        telemetry_layout.addWidget(self.metadata_label)

        self.detection_summary_label = QLabel("No detections yet.")
        self.detection_summary_label.setWordWrap(True)
        telemetry_layout.addWidget(self.detection_summary_label)

        self.gl_view = gl.GLViewWidget()
        self.gl_view.opts["distance"] = 5000
        self.gl_view.setBackgroundColor((0, 0, 0, 255))
        grid = gl.GLGridItem()
        grid.setSize(6000, 6000, 1)
        grid.setSpacing(500, 500, 500)
        self.gl_view.addItem(grid)
        self.axis_item = gl.GLAxisItem()
        self.axis_item.setSize(6000, 6000, 6000)
        self.gl_view.addItem(self.axis_item)
        self.scatter = gl.GLScatterPlotItem(size=6, pxMode=True)
        self.scatter.setGLOptions("additive")
        self.gl_view.addItem(self.scatter)
        self.gl_view.setCameraPosition(distance=12000, elevation=30, azimuth=45)
        telemetry_layout.addWidget(self.gl_view, stretch=2)

        self.log_widget = QTextEdit()
        self.log_widget.setReadOnly(True)
        self.log_widget.setFixedHeight(120)
        telemetry_layout.addWidget(self.log_widget)

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
        self._update_power_profile(power_bins)
        self._update_detection_table(detection_records)
        self._update_metadata(metadata)
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

    def _update_detection_table(self, records: List[Dict[str, Any]]) -> None:
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
            self.detection_table.setItem(
                row, 2, QTableWidgetItem(f"{record.get('doppler', 0):.2f}")
            )
            self.detection_table.setItem(
                row, 3, QTableWidgetItem(f"{record.get('snr', 0):.2f}")
            )

    def _update_detection_summary(self, records: List[Dict[str, Any]]) -> None:
        if not records:
            self.detection_summary_label.setText("No detections to visualize.")
            return
        top = sorted(records, key=lambda rec: rec.get("snr", 0), reverse=True)[:3]
        summary = "Top detections:\n"
        summary += "\n".join(
            f"#{idx + 1}: range {rec.get('range', 0):.1f} m, doppler {rec.get('doppler', 0):.2f} m/s, "
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
            f"Area: {metadata.get('area_width_km', 'n/a')} × {metadata.get('area_height_km', 'n/a')} km<br>"
            f"Clutter: {metadata.get('clutter_level', 'n/a')}<br>"
            f"SNR target: {metadata.get('snr_target_db', 'n/a')} dB<br>"
            f"Interference: {metadata.get('interference_db', 'n/a')} dB<br>"
            f"Target motion: {metadata.get('target_motion', 'n/a')}"
        )
        self.metadata_label.setText(html)

    def _update_3d_view(self, records: List[Dict[str, Any]], metadata: Dict[str, Any]) -> None:
        if not records:
            self.scatter.setData(
                pos=np.array([[0, 0, 0]], dtype=float), size=np.array([3.0]), color=np.array([[0.2, 0.4, 0.8, 0.9]])
            )
            return
        area_width = metadata.get("area_width_km") or metadata.get("area_width", 10)
        area_height = metadata.get("area_height_km") or metadata.get("area_height", 10)
        area_m = max(area_width, area_height, 8.0) * 1000.0
        max_range = max((float(record.get("range", 0)) for record in records), default=area_m)
        display_range = max(area_m, max_range, 100.0)
        scale = 5000.0 / display_range

        positions = []
        colors = []
        sizes = []
        for record in records[:256]:
            rng = float(record.get("range", 0))
            doppler = float(record.get("doppler", 0))
            snr = float(record.get("snr", 0))
            normalized_doppler = np.clip(doppler / 80.0, -1.0, 1.0)
            angle = normalized_doppler * np.pi / 2.0
            x = rng * np.cos(angle) * scale
            y = rng * np.sin(angle) * scale
            z = snr
            positions.append((x, y, z))
            colors.append(
                (
                    min(1.0, 0.3 + snr / 40.0),
                    max(0.1, 0.8 - snr / 80.0),
                    0.4,
                    0.9,
                )
            )
            sizes.append(np.clip(4 + snr * 0.17, 2, 14))
        self.scatter.setData(
            pos=np.array(positions, dtype=float),
            color=np.array(colors, dtype=float),
            size=np.array(sizes, dtype=float),
            pxMode=True,
        )
        self.gl_view.update()

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
