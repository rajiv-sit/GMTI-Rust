import math
import unittest

from pyqt_visualizer.geometry import project_detection_coordinates


class TestDetectionMapping(unittest.TestCase):
    def setUp(self) -> None:
        self.metadata = {
            "altitude_m": 4000.0,
            "area_width_km": 10.0,
            "area_height_km": 10.0,
        }

    def make_record(self, range_m: float, doppler: float, bearing: float) -> dict:
        return {
            "range": range_m,
            "doppler": doppler,
            "snr": 10.0,
            "bearing_deg": bearing,
            "elevation_deg": 0.0,
        }

    def test_polar_mapping_uses_bearing(self):
        record = self.make_record(5000.0, 0.0, 45.0)
        x, y = project_detection_coordinates(
            record,
            self.metadata,
            "polar",
            half_span_x_m=5000.0,
            half_span_y_m=5000.0,
            area_radius=5000.0,
            max_range_value=5000.0,
            max_doppler=1.0,
        )
        expected = 5000.0 / math.sqrt(2)
        self.assertAlmostEqual(x, expected, places=1)
        self.assertAlmostEqual(y, expected, places=1)

    def test_cartesian_mapping_uses_range_doppler(self):
        record = self.make_record(8000.0, 0.5, 90.0)
        x, y = project_detection_coordinates(
            record,
            self.metadata,
            "cartesian",
            half_span_x_m=5000.0,
            half_span_y_m=5000.0,
            area_radius=5000.0,
            max_range_value=8000.0,
            max_doppler=1.0,
        )
        self.assertAlmostEqual(x, 5000.0, places=1)
        self.assertAlmostEqual(y, 0.5 * 5000.0, places=1)


if __name__ == "__main__":
    unittest.main()
