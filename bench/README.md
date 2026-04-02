# Benchmarks

This harness compares Coldbrew (`crew`) and Homebrew (`brew`) on common workflows.
It uses `hyperfine` and stores results under `bench/results/<timestamp>`.

## Quick start

```bash
./bench/run.sh
```

## Requirements

- Homebrew (`brew`)
- `hyperfine`
- Rust toolchain (to build `crew`)

## Scenarios

- `index_refresh`: `crew update` vs `brew update`
- `search`: `crew search` vs `brew search`
- `upgrade_check`: `crew upgrade --yes` vs `brew upgrade --dry-run`
- `multi_install_cold`: caches cleared, reinstall N formulas
- `multi_install_warm`: caches kept, reinstall N formulas
- `single_install_cold`: caches cleared, reinstall one formula
- `single_install_warm`: caches kept, reinstall one formula

Coldbrew does not expose a dry-run upgrade. The script uses `--yes` inside an
isolated `COLDBREW_HOME` under `bench/state` to avoid touching your main setup.

## Configuration

Environment variables supported by `bench/run.sh`:

- `RUNS` (default `7`)
- `WARMUP` (default `1`)
- `FORMULA_COUNT` (default `10`)
- `ALLOW_INSTALLED` (default `0`, set to `1` to include installed formulas)
- `ALLOW_DESTRUCTIVE` (default `0`, set to `1` to allow deletes outside `bench/state`)
- `SEARCH_TERM` (default `python`)
- `SINGLE_FORMULA` (default: first selected formula)
- `CREW_BIN` (default `target/release/crew`)
- `BREW_BIN` (default `brew`)
- `HYPERFINE_BIN` (default `hyperfine`)
- `RESULTS_ROOT` (default `bench/results`)
- `LOGS_ROOT` (default `bench/logs`)
- `STATE_ROOT` (default `bench/state`)
- `COLDBREW_HOME` (default `bench/state/coldbrew`)
- `BREW_CACHE` (default `bench/state/brew-cache`)
- `VERIFY_INSTALL` (default `1`, verify selected formulas are installed)
- `VERIFY_BINARIES` (default `1`, verify binaries in `BINARIES_FILE` are on PATH)
- `VERIFY_BINARIES_STRICT` (default `0`, fail if a binary is missing from PATH)
- `BINARIES_FILE` (default `bench/binaries.txt`)

## Notes

- Homebrew installs still target your system prefix. The script isolates only
  Homebrew downloads with `HOMEBREW_CACHE` under `bench/state`.
- `COLDBREW_HOME` and `BREW_CACHE` must live under `bench/state` unless
  `ALLOW_DESTRUCTIVE=1` is set.
- The script disables Homebrew auto-update and cleanup for stability.
- Coldbrew formulas are updated once before installs to ensure the index exists.
- Coldbrew treats `name@version` as a version spec, so avoid `@` formulas unless
  you intend to install by version.
- `meta.json` includes machine specs (OS, CPU, memory) for each run.
- `bench/binaries.txt` maps formulas to binaries for optional verification (uses `command -v`).
- Results are written as JSON/Markdown per scenario. If `python3` is available,
  a `summary.md` table is generated.
