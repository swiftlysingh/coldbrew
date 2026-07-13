# Performance Gates

`harness/perf/crew_perf_gate.py` is the Swift migration performance gate. It is
stdlib Python on purpose: no benchmark framework, no extra package install.

## What It Measures

- `fixture_install`: a fixture-registry install command supplied by the harness.
- `real_install_cold:<pkg>` and `real_install_warm:<pkg>`: optional real installs,
  normally `jq` and `ffmpeg`.
- `shim_startup`: a synthetic installed package invoked through a real Coldbrew
  shim script, measuring CLI startup plus shim resolution.
- Rust-vs-Swift install comparison when both binaries are supplied.

## Local Use

Build the binaries first, then run:

```bash
python3 harness/perf/crew_perf_gate.py \
  --rust-bin target/release/crew \
  --swift-bin swift/.build/release/crew \
  --fixture-install-cmd '{crew} update && {crew} install hello' \
  --real-install jq \
  --real-install ffmpeg
```

The fixture command is a shell template. Supported placeholders:

| Placeholder | Value |
|---|---|
| `{crew}` | absolute path to the binary under test |
| `{home}` | isolated HOME for that sample |
| `{tmp}` | isolated temp directory for that sample |

The command is trusted developer/CI configuration and executes through the shell.
CI points `COLDBREW_FORMULA_API_BASE` at the checked-in registry fixtures.

## Thresholds

| Gate | Default |
|---|---:|
| Swift install time vs Rust | Swift median must be within 10% of Rust median |
| Swift shim startup latency | Median must be under 50 ms |
| Timed iterations | 5 |
| Warmups | 1 |

Environment overrides:

| Variable | Purpose |
|---|---|
| `RUST_CREW_BIN` | Rust `crew` path |
| `SWIFT_CREW_BIN` | Swift `crew` path |
| `PERF_FIXTURE_INSTALL_CMD` | Fixture install command template |
| `PERF_REAL_INSTALLS` | Space- or comma-separated packages, e.g. `jq,ffmpeg` |
| `PERF_REAL_MODE` | `cold`, `warm`, or `both` |
| `PERF_INSTALL_THRESHOLD_PCT` | Rust-vs-Swift install budget |
| `PERF_SHIM_STARTUP_MAX_MS` | Absolute shim startup budget |
| `PERF_EXCEPTION_REASON` | Documented temporary exception; report still lists failures |

Shim timing includes process launch and the `/bin/sh` shim itself because that is the
user-visible startup path.

## Exception Path

Do not raise thresholds silently. If a regression is accepted temporarily, set
`PERF_EXCEPTION_REASON` to the tracking issue and reason. The script exits zero,
but the report moves failures under `Exceptions` and prints the reason.
