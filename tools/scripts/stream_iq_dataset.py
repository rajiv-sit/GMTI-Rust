import argparse
import http.client
import json
import math
import random
import time
from pathlib import Path

SCENARIOS = {
    "airborne": {
        "name": "airborne_intel",
        "platform_type": "Airborne ISR",
        "platform_velocity_kmh": 750.0,
        "altitude_m": 8200.0,
        "area_width_km": 10.0,
        "area_height_km": 10.0,
        "clutter_level": 0.45,
        "snr_target_db": 18.0,
        "interference_db": -10.0,
        "target_motion": "Cruise, gentle zig-zag",
        "taps": 4,
        "range_bins": 1024,
        "doppler_bins": 256,
        "frequency": 1050000000.0,
        "noise": 0.07,
        "seed": 1337,
        "mode": "AdvGmtiScan",
        "frame_rate": 2.0,
    },
    "land": {
        "name": "land_ambush",
        "platform_type": "Land-based Radar",
        "platform_velocity_kmh": 40.0,
        "altitude_m": 30.0,
        "area_width_km": 10.0,
        "area_height_km": 10.0,
        "clutter_level": 0.6,
        "snr_target_db": 14.0,
        "interference_db": -6.0,
        "target_motion": "Tactical convoy moving east",
        "taps": 6,
        "range_bins": 768,
        "doppler_bins": 192,
        "frequency": 900000000.0,
        "noise": 0.08,
        "seed": 404,
        "mode": "AdvDmtiStare",
        "frame_rate": 1.5,
    },
}


def generate_samples(config, frame_index):
    taps = config["taps"]
    range_bins = config["range_bins"]
    frequency = config["frequency"]
    noise_floor = config["noise"]
    rng = random.Random(config["seed"] + frame_index)
    samples = []
    time_offset = frame_index / config["frame_rate"]
    velocity_factor = config["platform_velocity_kmh"] / 500.0

    for tap in range(taps):
        phase_offset = tap * 0.35
        for range_idx in range(range_bins):
            normalized_range = range_idx / range_bins
            base_phase = (
                (normalized_range + time_offset * 0.001 + phase_offset * 0.01)
                * 2.0
                * math.pi
                * frequency
                * 1e-6
            )
            envelope = 0.25 + 0.75 * (1.0 - normalized_range)
            jitter = rng.uniform(-noise_floor, noise_floor)
            motion_wave = math.sin(normalized_range * 7.0 - time_offset * velocity_factor)
            snr_linear = 10 ** (config["snr_target_db"] / 20.0)
            interference = (10 ** (config["interference_db"] / 20.0)) * math.cos(
                normalized_range * 5.0 + time_offset * 0.5
            )
            clutter = config["clutter_level"] * rng.uniform(-1.0, 1.0)
            value = (
                math.sin(base_phase) * envelope * (1.0 + 0.2 * motion_wave)
                + clutter
                + snr_linear * (motion_wave * (1.0 - normalized_range * 0.6))
                + interference
                + jitter
            )
            samples.append(value)
    return samples


def build_payload(config, frame_index, timestamp):
    samples = generate_samples(config, frame_index)
    ancillary = {
        "timestamp": timestamp,
        "mode": config["mode"],
        "pulse_count": config["taps"],
        "dwell": 45.0,
        "range_start": 0.0,
        "range_end": 30_000.0,
        "metadata": {
            "name": config["name"],
            "platform_type": config["platform_type"],
            "platform_velocity_kmh": config["platform_velocity_kmh"],
            "altitude_m": config["altitude_m"],
            "area_width_km": config["area_width_km"],
            "area_height_km": config["area_height_km"],
            "clutter_level": config["clutter_level"],
            "snr_target_db": config["snr_target_db"],
            "interference_db": config["interference_db"],
            "target_motion": config["target_motion"],
            "description": f"{config['name']} run {frame_index}",
            "timestamp_start": timestamp - frame_index / config["frame_rate"],
        },
    }
    return {"samples": samples, "ancillary": ancillary}


def stream_scenario(
    name,
    config,
    duration_minutes,
    host,
    port,
    dry_run,
    real_time,
    metadata_dir,
):
    frames = int(duration_minutes * 60 * config["frame_rate"])
    connection = http.client.HTTPConnection(host, port, timeout=10)
    metadata = {
        "scenario": name,
        "platform_type": config["platform_type"],
        "platform_velocity_kmh": config["platform_velocity_kmh"],
        "altitude_m": config["altitude_m"],
        "area_width_km": config["area_width_km"],
        "area_height_km": config["area_height_km"],
        "clutter_level": config["clutter_level"],
        "snr_target_db": config["snr_target_db"],
        "interference_db": config["interference_db"],
        "target_motion": config["target_motion"],
        "duration_minutes": duration_minutes,
        "frame_rate": config["frame_rate"],
        "taps": config["taps"],
        "range_bins": config["range_bins"],
        "doppler_bins": config["doppler_bins"],
        "frequency": config["frequency"],
        "noise": config["noise"],
        "created_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
    }
    if dry_run:
        print(f"[DRY RUN] Prepared {frames} frames for {name}")
    else:
        print(f"Streaming {frames} frames for {name} to http://{host}:{port}/ingest")

    start_time = time.time()
    for idx in range(frames):
        timestamp = time.time()
        payload = build_payload(config, idx, timestamp)
        if not dry_run:
            try:
                connection.request(
                    "POST",
                    "/ingest",
                    body=json.dumps(payload),
                    headers={"Content-Type": "application/json"},
                )
                response = connection.getresponse()
                response_text = response.read().decode(errors="ignore")
                print(
                    f"[{idx + 1}/{frames}] {response.status} {response.reason} -> {response_text}"
                )
            except Exception as exc:  # pragma: no cover
                print(f"[{idx + 1}/{frames}] failed to POST: {exc}")
                break
        if real_time:
            elapsed = time.time() - start_time
            target = (idx + 1) / config["frame_rate"]
            delay = target - elapsed
            if delay > 0:
                time.sleep(delay)
    if not dry_run:
        connection.close()

    meta_path = Path(metadata_dir) / f"iq_dataset_{name}_metadata.json"
    meta_path.parent.mkdir(parents=True, exist_ok=True)
    meta_path.write_text(json.dumps(metadata, indent=2))
    print(f"Scenario metadata saved to {meta_path}")


def parse_args():
    parser = argparse.ArgumentParser(
        description="Generate 10-minute IQ streams for GMTI ingestion."
    )
    parser.add_argument(
        "--scenario",
        choices=list(SCENARIOS),
        default="airborne",
        help="Baseline scenario to run (airborne or land).",
    )
    parser.add_argument(
        "--duration",
        type=float,
        default=10.0,
        help="Duration of the stream in minutes (minimum 10).",
    )
    parser.add_argument(
        "--host", default="127.0.0.1", help="Host running the GMTI bridge."
    )
    parser.add_argument("--port", type=int, default=9000, help="HTTP port for /ingest.")
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Build the dataset but skip HTTP POST (useful for validation).",
    )
    parser.add_argument(
        "--real-time",
        action="store_true",
        help="Throttle the generation to real-time pacing based on frame rate.",
    )
    parser.add_argument(
        "--metadata-dir",
        default="tools/data",
        help="Directory where scenario metadata files are stored.",
    )
    return parser.parse_args()


def main():
    args = parse_args()
    duration = max(10.0, args.duration)
    scenario = SCENARIOS[args.scenario]
    stream_scenario(
        args.scenario,
        scenario,
        duration,
        args.host,
        args.port,
        args.dry_run,
        args.real_time,
        args.metadata_dir,
    )


if __name__ == "__main__":
    main()
