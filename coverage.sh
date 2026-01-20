#!/usr/bin/env bash

set -euo pipefail

# --- Defaults ---
OUT_FILE="coverage.json"
DEFAULT_COV_ARGS=(--workspace --all-features)
CACHE_SECONDS=3600

usage() {
  cat <<'EOF'
Usage:
  coverage.sh [--out path/to/coverage.json] [-- <extra cargo llvm-cov args>]

Examples:
  coverage.sh
  coverage.sh --out badges/coverage.json
  coverage.sh -- --package my_crate
  coverage.sh -- --ignore-filename-regex '(/tests/|/generated/)'
EOF
}

# --- Arg parsing ---
EXTRA_ARGS=()
while [[ $# -gt 0 ]]; do
  case "$1" in
    -o|--out)
      OUT_FILE="${2:-}"
      [[ -n "${OUT_FILE}" ]] || { echo "error: --out requires a value" >&2; exit 2; }
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    --)
      shift
      EXTRA_ARGS+=("$@")
      break
      ;;
    *)
      echo "error: unknown arg: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

# --- Preconditions ---
if ! command -v cargo >/dev/null 2>&1; then
  echo "error: cargo not found in PATH" >&2
  exit 1
fi

if ! command -v cargo-llvm-cov >/dev/null 2>&1 && ! cargo llvm-cov --version >/dev/null 2>&1; then
  echo "error: cargo-llvm-cov not found. Install with: cargo install cargo-llvm-cov" >&2
  exit 1
fi

if command -v rustup >/dev/null 2>&1; then
  if ! rustup component list --installed | grep -q '^llvm-tools'; then
    echo "error: rustup component llvm-tools-preview is not installed." >&2
    echo "       run: rustup component add llvm-tools-preview" >&2
    exit 1
  fi
fi

# --- Generate LCOV and compute total line coverage ---
tmp_lcov="$(mktemp)"
trap 'rm -f "$tmp_lcov"' EXIT

# Put format flags at the end so EXTRA_ARGS can't accidentally override them.
cargo llvm-cov "${DEFAULT_COV_ARGS[@]}" "${EXTRA_ARGS[@]}" --lcov --output-path "$tmp_lcov"

read -r HIT FOUND < <(
  awk -F: '
    $1=="LF" { found += $2 }
    $1=="LH" { hit   += $2 }
    END { printf "%d %d\n", hit, found }
  ' "$tmp_lcov"
)

if [[ "${FOUND}" -le 0 ]]; then
  echo "error: could not compute coverage (LF total was 0). Did tests run / were any files instrumented?" >&2
  exit 1
fi

PCT="$(awk -v h="$HIT" -v f="$FOUND" 'BEGIN { printf "%.1f", (h*100)/f }')"

COLOR="$(awk -v p="$PCT" 'BEGIN {
  if (p >= 90) print "brightgreen";
  else if (p >= 80) print "green";
  else if (p >= 70) print "yellowgreen";
  else if (p >= 60) print "yellow";
  else if (p >= 50) print "orange";
  else print "red";
}')"

# --- Write Shields endpoint JSON (overwrite existing file) ---
cat > "$OUT_FILE" <<EOF
{"schemaVersion":1,"label":"coverage","message":"${PCT}%","color":"${COLOR}","cacheSeconds":${CACHE_SECONDS}}
EOF

echo "Wrote ${OUT_FILE}: ${PCT}% (LH=${HIT}, LF=${FOUND})"
