#!/bin/sh
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
CONFIG_DIR="$ROOT_DIR/simulator/configs"

echo "Regenerating baselines from $(date)"

for cfg in "$CONFIG_DIR"/*.yaml; do
    echo "Running workflow config: $cfg"
    (cd "$ROOT_DIR" && cargo run --bin simulator -- --offline --workflow "$cfg")
done

echo "Baselines regenerated at $(date)"
