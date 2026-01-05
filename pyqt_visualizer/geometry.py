"""Geometry helpers for PyQt visualizer."""

import math
from typing import Any, Dict, Tuple


def project_detection_coordinates(
    record: Dict[str, float],
    metadata: Dict[str, Any],
    view_mode: str,
    half_span_x_m: float,
    half_span_y_m: float,
    area_radius: float,
    max_range_value: float,
    max_doppler: float,
) -> Tuple[float, float]:
    rng = float(record.get("range", 0))
    doppler = float(record.get("doppler", 0))
    normalized_doppler = max(-1.0, min(1.0, doppler / max_doppler)) if max_doppler > 0 else 0.0
    bearing_deg = float(record.get("bearing_deg", 0))
    elevation_deg = float(record.get("elevation_deg", 0))
    bearing_rad = math.radians(bearing_deg)
    elevation_rad = math.radians(elevation_deg)
    horizontal_range = rng * math.cos(elevation_rad)
    horizontal_range = min(max(horizontal_range, 0.0), area_radius)
    if view_mode == "polar":
        x = horizontal_range * math.cos(bearing_rad)
        y = horizontal_range * math.sin(bearing_rad)
    else:
        x = (rng / max_range_value) * half_span_x_m
        y = normalized_doppler * half_span_y_m
    return x, y
