#!/usr/bin/env bash
# Run criterion benchmarks and append results to benchmark-results/
#
# Outputs:
#   latest.json    — structured JSON of the most recent run
#   latest.md      — markdown table of the most recent run
#   history.json   — array of all runs
#   history.md     — appended markdown of all runs
#
# Usage: ./scripts/run-benchmarks.sh [-- <extra cargo bench args>]

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
RESULTS_DIR="$PROJECT_DIR/benchmark-results"

mkdir -p "$RESULTS_DIR"

TIMESTAMP=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
RUST_VERSION=$(rustc --version)
COMMIT=$(git -C "$PROJECT_DIR" rev-parse --short HEAD 2>/dev/null || echo "unknown")

echo "=== Jalwa Benchmark Run ==="
echo "Time:   $TIMESTAMP"
echo "Rust:   $RUST_VERSION"
echo "Commit: $COMMIT"
echo ""

# Run all benchmark suites, capture output
BENCH_OUTPUT=$(cargo bench --bench library --bench dsp --bench video "$@" 2>&1) || true

echo "$BENCH_OUTPUT"

# Parse criterion output into structured results
LATEST_JSON="$RESULTS_DIR/latest.json"
LATEST_MD="$RESULTS_DIR/latest.md"
HISTORY_MD="$RESULTS_DIR/history.md"
HISTORY_JSON="$RESULTS_DIR/history.json"

# Build JSON results — handles criterion's multi-line format where
# the bench name appears on one line and "time: [...]" on the next
python3 -c "
import json, re, sys

output = '''$BENCH_OUTPUT'''

results = []
lines = output.split('\n')
current_name = None

for line in lines:
    # Check for a benchmark name line (non-indented, no 'time:')
    name_m = re.match(r'^(\S+)\s*$', line.strip())
    if name_m and 'time:' not in line and 'Benchmarking' not in line and 'warning' not in line.lower():
        candidate = name_m.group(1)
        # Filter out noise lines
        if not candidate.startswith(('Running', 'Compiling', 'Finished', 'Downloaded', 'Downloading', 'Warning', 'Found', 'Gnuplot')):
            current_name = candidate

    # Also check for single-line format: 'name  time: [...]'
    single_m = re.match(r'^\s*(\S+)\s+time:\s+\[', line)
    if single_m:
        current_name = single_m.group(1)

    # Check for time line
    time_m = re.search(r'time:\s+\[([0-9.]+)\s+(\w+)\s+([0-9.]+)\s+(\w+)\s+([0-9.]+)\s+(\w+)\]', line)
    if time_m and current_name:
        low = float(time_m.group(1))
        unit_low = time_m.group(2)
        mid = float(time_m.group(3))
        unit_mid = time_m.group(4)
        high = float(time_m.group(5))
        unit_high = time_m.group(6)

        def to_us(val, unit):
            if unit in ('ns', 'ns/iter'):
                return val / 1000.0
            if unit in ('\u00b5s', 'us'):
                return val
            if unit == 'ms':
                return val * 1000.0
            if unit == 's':
                return val * 1_000_000.0
            return val

        results.append({
            'name': current_name,
            'low_us': round(to_us(low, unit_low), 3),
            'mid_us': round(to_us(mid, unit_mid), 3),
            'high_us': round(to_us(high, unit_high), 3),
            'unit': '\u00b5s',
        })
        current_name = None

report = {
    'timestamp': '$TIMESTAMP',
    'rust_version': '$RUST_VERSION',
    'commit': '$COMMIT',
    'benchmarks': results,
}

# Write latest.json
with open('$LATEST_JSON', 'w') as f:
    json.dump(report, f, indent=2)
    f.write('\n')

# Append to history.json
history = []
try:
    with open('$HISTORY_JSON') as f:
        history = json.load(f)
except (FileNotFoundError, json.JSONDecodeError):
    pass
history.append(report)
with open('$HISTORY_JSON', 'w') as f:
    json.dump(history, f, indent=2)
    f.write('\n')

# Build markdown table
lines = []
lines.append(f'## {report[\"timestamp\"]} \u2014 {report[\"commit\"]}')
lines.append(f'Rust: {report[\"rust_version\"]}')
lines.append('')
lines.append('| Benchmark | Low (\u00b5s) | Mid (\u00b5s) | High (\u00b5s) |')
lines.append('|-----------|----------|----------|-----------|')
for b in results:
    # Use adaptive precision: 3 decimals for sub-1µs, 1 for >100µs, 2 otherwise
    def fmt(v):
        if v < 1.0:
            return f'{v:.3f}'
        elif v >= 100.0:
            return f'{v:.1f}'
        else:
            return f'{v:.2f}'
    lines.append(f'| {b[\"name\"]} | {fmt(b[\"low_us\"])} | {fmt(b[\"mid_us\"])} | {fmt(b[\"high_us\"])} |')
lines.append('')
md_section = '\n'.join(lines)

# Write latest.md
with open('$LATEST_MD', 'w') as f:
    f.write('# Latest Benchmark Results\n\n')
    f.write(md_section)

# Append to history.md
header_needed = True
try:
    with open('$HISTORY_MD') as f:
        existing = f.read()
        header_needed = not existing.strip()
except FileNotFoundError:
    existing = ''
with open('$HISTORY_MD', 'a') as f:
    if header_needed:
        f.write('# Benchmark History\n\n')
    f.write(md_section)

print(f'\nParsed {len(results)} benchmarks.')
print(f'Results saved to $RESULTS_DIR/')
" 2>&1

echo ""
echo "=== Done ==="
