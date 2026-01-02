import http.client
import json
from pathlib import Path

SCENARIO_FILES = [
    Path("simulator/configs/default.yaml"),
    Path("simulator/configs/stare.yaml"),
    Path("simulator/configs/scan.yaml"),
]


def parse_yaml(path: Path) -> dict:
    data = {}
    for line in path.read_text().splitlines():
        trimmed = line.strip()
        if not trimmed or trimmed.startswith("#"):
            continue
        if ":" not in trimmed:
            continue
        key, value = trimmed.split(":", 1)
        key = key.strip()
        value = value.strip().strip('"')
        if key in {"taps", "range_bins", "doppler_bins", "seed"}:
            data[key] = int(value)
        elif key in {"frequency", "noise"}:
            data[key] = float(value)
        else:
            data[key] = value
    data.setdefault("seed", 0)
    data["scenario"] = path.stem
    return data


def post_scenario(scenario: Path) -> None:
    payload = parse_yaml(scenario)
    body = json.dumps(payload)
    connection = http.client.HTTPConnection("127.0.0.1", 9000, timeout=5)
    connection.request("POST", "/ingest-config", body, headers={"Content-Type": "application/json"})
    response = connection.getresponse()
    print(f"{scenario.name}: {response.status} {response.reason} -> {response.read().decode().strip()}")


def main() -> None:
    for scenario in SCENARIO_FILES:
        if scenario.exists():
            post_scenario(scenario)
        else:
            print(f"missing {scenario}")


if __name__ == "__main__":
    main()
