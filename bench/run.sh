#!/usr/bin/env bash
set -euo pipefail
IFS=$'\n\t'

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BENCH_DIR="$ROOT_DIR/bench"

FORMULAS_FILE="${FORMULAS_FILE:-$BENCH_DIR/formulas.txt}"
RUNS="${RUNS:-7}"
WARMUP="${WARMUP:-1}"
FORMULA_COUNT="${FORMULA_COUNT:-10}"
SEARCH_TERM="${SEARCH_TERM:-python}"

CREW_BIN="${CREW_BIN:-$ROOT_DIR/target/release/crew}"
BREW_BIN="${BREW_BIN:-brew}"
HYPERFINE_BIN="${HYPERFINE_BIN:-hyperfine}"

RESULTS_ROOT="${RESULTS_ROOT:-$BENCH_DIR/results}"
LOGS_ROOT="${LOGS_ROOT:-$BENCH_DIR/logs}"
STATE_ROOT="${STATE_ROOT:-$BENCH_DIR/state}"

COLDBREW_HOME="${COLDBREW_HOME:-$STATE_ROOT/coldbrew}"
BREW_CACHE="${BREW_CACHE:-$STATE_ROOT/brew-cache}"

timestamp="$(date +"%Y%m%d-%H%M%S")"
RESULTS_DIR="$RESULTS_ROOT/$timestamp"
LOG_DIR="$LOGS_ROOT/$timestamp"

die() {
  echo "error: $*" >&2
  exit 1
}

log() {
  echo "==> $*"
}

shell_join() {
  local out=""
  local arg
  for arg in "$@"; do
    if [ -z "$out" ]; then
      out="$(printf '%q' "$arg")"
    else
      out+=" $(printf '%q' "$arg")"
    fi
  done
  printf '%s' "$out"
}

json_escape() {
  local value="$1"
  value="${value//\\/\\\\}"
  value="${value//\"/\\\"}"
  value="${value//$'\n'/\\n}"
  printf '%s' "$value"
}

mkdir -p "$RESULTS_DIR" "$LOG_DIR" "$STATE_ROOT" "$BREW_CACHE"

command -v "$BREW_BIN" >/dev/null 2>&1 || die "brew not found"
command -v "$HYPERFINE_BIN" >/dev/null 2>&1 || die "hyperfine not found"

if [ ! -x "$CREW_BIN" ]; then
  log "Building release crew binary"
  (cd "$ROOT_DIR" && cargo build --release)
fi
if [ ! -x "$CREW_BIN" ]; then
  die "crew binary not found at $CREW_BIN"
fi

if [ ! -f "$FORMULAS_FILE" ]; then
  die "formulas file not found at $FORMULAS_FILE"
fi

primary_formulas=()
fallback_formulas=()
section="primary"
while IFS= read -r line || [ -n "$line" ]; do
  line="${line%%#*}"
  line="${line#"${line%%[![:space:]]*}"}"
  line="${line%"${line##*[![:space:]]}"}"
  [ -z "$line" ] && continue
  if [ "$line" = "--fallback--" ]; then
    section="fallback"
    continue
  fi
  if [ "$section" = "primary" ]; then
    primary_formulas+=("$line")
  else
    fallback_formulas+=("$line")
  fi
done < "$FORMULAS_FILE"

installed_formulas="$($BREW_BIN list --formula 2>/dev/null || true)"
is_installed() {
  printf '%s\n' "$installed_formulas" | grep -qx "$1"
}

selected=()
for formula in "${primary_formulas[@]}"; do
  if ! is_installed "$formula"; then
    selected+=("$formula")
  fi
  if [ "${#selected[@]}" -ge "$FORMULA_COUNT" ]; then
    break
  fi
done

if [ "${#selected[@]}" -lt "$FORMULA_COUNT" ]; then
  for formula in "${fallback_formulas[@]}"; do
    if ! is_installed "$formula"; then
      selected+=("$formula")
    fi
    if [ "${#selected[@]}" -ge "$FORMULA_COUNT" ]; then
      break
    fi
  done
fi

if [ "${#selected[@]}" -eq 0 ]; then
  die "no formulas available for benchmarking"
fi

if [ "${#selected[@]}" -lt "$FORMULA_COUNT" ]; then
  echo "warning: only ${#selected[@]} formulas selected (requested $FORMULA_COUNT)" >&2
fi

SINGLE_FORMULA="${SINGLE_FORMULA:-${selected[0]}}"

printf '%s\n' "${selected[@]}" > "$RESULTS_DIR/formulas.txt"

rm -rf "$COLDBREW_HOME"
mkdir -p "$COLDBREW_HOME"

BREW_ENV=(
  "HOMEBREW_NO_AUTO_UPDATE=1"
  "HOMEBREW_NO_INSTALL_CLEANUP=1"
  "HOMEBREW_NO_INSTALLED_DEPENDENTS_CHECK=1"
  "HOMEBREW_CACHE=$BREW_CACHE"
)

crew_cmd() {
  env "COLDBREW_HOME=$COLDBREW_HOME" "$CREW_BIN" "$@"
}

brew_cmd() {
  env "${BREW_ENV[@]}" "$BREW_BIN" "$@"
}

log "Updating coldbrew index"
crew_cmd update

log "Preinstalling formulas"
crew_cmd install "${selected[@]}"
brew_cmd install --formula --force-bottle "${selected[@]}"

log "Writing metadata"
{
  echo "{"
  echo "  \"timestamp\": \"$(json_escape "$timestamp")\","
  echo "  \"runs\": $RUNS,"
  echo "  \"warmup\": $WARMUP,"
  echo "  \"formula_count\": ${#selected[@]},"
  echo "  \"single_formula\": \"$(json_escape "$SINGLE_FORMULA")\","
  echo "  \"search_term\": \"$(json_escape "$SEARCH_TERM")\","
  echo "  \"crew_version\": \"$(json_escape "$($CREW_BIN --version)")\","
  echo "  \"brew_version\": \"$(json_escape "$($BREW_BIN --version | head -n 1)")\","
  echo "  \"hyperfine_version\": \"$(json_escape "$($HYPERFINE_BIN --version)")\","
  echo "  \"formulas\": ["
  for i in "${!selected[@]}"; do
    if [ "$i" -gt 0 ]; then
      echo "    ,\"$(json_escape "${selected[$i]}")\""
    else
      echo "    \"$(json_escape "${selected[$i]}")\""
    fi
  done
  echo "  ]"
  echo "}"
} > "$RESULTS_DIR/meta.json"

prepare_cold_cmd="rm -rf \"$BREW_CACHE\" && mkdir -p \"$BREW_CACHE\" && env COLDBREW_HOME=\"$COLDBREW_HOME\" \"$CREW_BIN\" cache clean"

run_hyperfine() {
  local name="$1"
  local prepare_cmd="$2"
  local crew_command="$3"
  local brew_command="$4"

  local json_path="$RESULTS_DIR/${name}.json"
  local md_path="$RESULTS_DIR/${name}.md"
  local crew_log="$LOG_DIR/${name}-crew.log"
  local brew_log="$LOG_DIR/${name}-brew.log"

  log "Running ${name}"

  local args=(--warmup "$WARMUP" --runs "$RUNS" --export-json "$json_path" --export-markdown "$md_path")
  if [ -n "$prepare_cmd" ]; then
    args+=(--prepare "$prepare_cmd")
  fi

  "$HYPERFINE_BIN" "${args[@]}" \
    --command-name "coldbrew" "$crew_command >> \"$crew_log\" 2>&1" \
    --command-name "homebrew" "$brew_command >> \"$brew_log\" 2>&1"
}

crew_update_cmd="$(shell_join env "COLDBREW_HOME=$COLDBREW_HOME" "$CREW_BIN" update)"
brew_update_cmd="$(shell_join env "${BREW_ENV[@]}" "$BREW_BIN" update)"

crew_search_cmd="$(shell_join env "COLDBREW_HOME=$COLDBREW_HOME" "$CREW_BIN" search "$SEARCH_TERM")"
brew_search_cmd="$(shell_join env "${BREW_ENV[@]}" "$BREW_BIN" search "$SEARCH_TERM")"

crew_upgrade_cmd="$(shell_join env "COLDBREW_HOME=$COLDBREW_HOME" "$CREW_BIN" upgrade --yes)"
brew_upgrade_cmd="$(shell_join env "${BREW_ENV[@]}" "$BREW_BIN" upgrade --dry-run)"

crew_multi_cmd="$(shell_join env "COLDBREW_HOME=$COLDBREW_HOME" "$CREW_BIN" install --force "${selected[@]}")"
brew_multi_cmd="$(shell_join env "${BREW_ENV[@]}" "$BREW_BIN" reinstall --force-bottle --formula "${selected[@]}")"

crew_single_cmd="$(shell_join env "COLDBREW_HOME=$COLDBREW_HOME" "$CREW_BIN" install --force "$SINGLE_FORMULA")"
brew_single_cmd="$(shell_join env "${BREW_ENV[@]}" "$BREW_BIN" reinstall --force-bottle --formula "$SINGLE_FORMULA")"

run_hyperfine "index_refresh" "" "$crew_update_cmd" "$brew_update_cmd"
run_hyperfine "search" "" "$crew_search_cmd" "$brew_search_cmd"
run_hyperfine "upgrade_check" "" "$crew_upgrade_cmd" "$brew_upgrade_cmd"
run_hyperfine "multi_install_cold" "$prepare_cold_cmd" "$crew_multi_cmd" "$brew_multi_cmd"
run_hyperfine "multi_install_warm" "" "$crew_multi_cmd" "$brew_multi_cmd"
run_hyperfine "single_install_cold" "$prepare_cold_cmd" "$crew_single_cmd" "$brew_single_cmd"
run_hyperfine "single_install_warm" "" "$crew_single_cmd" "$brew_single_cmd"

if command -v python3 >/dev/null 2>&1; then
  log "Writing summary"
  python3 - "$RESULTS_DIR" <<'PY'
import json
import os
import sys

results_dir = sys.argv[1]
rows = []
for name in sorted(os.listdir(results_dir)):
    if not name.endswith(".json") or name == "meta.json":
        continue
    path = os.path.join(results_dir, name)
    with open(path, "r", encoding="utf-8") as handle:
        data = json.load(handle)
    scenario = os.path.splitext(name)[0]
    for result in data.get("results", []):
        rows.append(
            (
                scenario,
                result.get("command", ""),
                result.get("median"),
                result.get("stddev"),
            )
        )

summary_path = os.path.join(results_dir, "summary.md")
with open(summary_path, "w", encoding="utf-8") as handle:
    handle.write("| Scenario | Command | Median (s) | Stddev (s) |\n")
    handle.write("| --- | --- | --- | --- |\n")
    for scenario, command, median, stddev in rows:
        median_str = f"{median:.3f}" if isinstance(median, (int, float)) else "-"
        stddev_str = f"{stddev:.3f}" if isinstance(stddev, (int, float)) else "-"
        handle.write(f"| {scenario} | {command} | {median_str} | {stddev_str} |\n")
PY
fi

log "Done. Results written to $RESULTS_DIR"
