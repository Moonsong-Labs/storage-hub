#!/bin/bash

# Concurrent runner for sdkTest.ts
# Usage: ./scripts/concurrent-sdkTest.sh [NUM_CONCURRENT]

set -euo pipefail

# Configuration
CMD="pnpm test"     # runs tsx src/sdkTest.ts
NUM_CONCURRENT=${1:-5}
OUTPUT_DIR="./concurrent-logs"

mkdir -p "$OUTPUT_DIR"

run_once() {
  local id=$1
  local timestamp=$(date +%s%N)
  local out_file="${OUTPUT_DIR}/sdkTest_${id}_${timestamp}.log"

  echo "[$id] Starting: $CMD"
  # Capture both stdout and stderr into the same log file
  if $CMD &> "$out_file"; then
    echo "[$id] ✓ Succeeded. Log: $out_file"
    return 0
  else
    echo "[$id] ✗ Failed. Log: $out_file"
    # Show last few lines to help debugging
    tail -n 20 "$out_file" || true
    return 1
  fi
}

echo "Starting $NUM_CONCURRENT concurrent runs..."
echo "Logs directory: $OUTPUT_DIR"
echo "---"

pids=()
for i in $(seq 1 $NUM_CONCURRENT); do
  run_once $i &
  pids+=($!)
done

failed=0
for pid in "${pids[@]}"; do
  if ! wait $pid; then
    ((failed++))
  fi
done

echo "---"
echo "Complete! $((NUM_CONCURRENT - failed))/$NUM_CONCURRENT succeeded"
echo "Logs: $OUTPUT_DIR"


