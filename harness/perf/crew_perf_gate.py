#!/usr/bin/env python3
"""Performance gate for the Swift migration.

Measures the Rust crew binary, optionally measures the Swift crew binary, and
fails when Swift install timings exceed Rust by more than the configured budget
or shim startup exceeds the absolute startup budget.
"""

from __future__ import annotations

import argparse
import json
import os
import shlex
import stat
import subprocess
import sys
import tempfile
import time
from pathlib import Path
from statistics import mean, median


def env_int(name: str, default: int) -> int:
    value = os.environ.get(name)
    return int(value) if value else default


def env_float(name: str, default: float) -> float:
    value = os.environ.get(name)
    return float(value) if value else default


def require_bin(path: str | None, label: str) -> Path | None:
    if not path:
        return None
    binary = Path(path).expanduser().resolve()
    if not binary.is_file():
        raise SystemExit(f"{label} binary not found: {binary}")
    if not os.access(binary, os.X_OK):
        raise SystemExit(f"{label} binary is not executable: {binary}")
    return binary


def run_checked(command, *, env: dict[str, str], cwd: Path | None = None, shell: bool = False, timeout: int = 900) -> float:
    start = time.perf_counter_ns()
    result = subprocess.run(
        command,
        cwd=cwd,
        env=env,
        shell=shell,
        executable="/bin/bash" if shell else None,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        timeout=timeout,
    )
    elapsed_ms = (time.perf_counter_ns() - start) / 1_000_000
    if result.returncode != 0:
        rendered = command if isinstance(command, str) else " ".join(map(shlex.quote, command))
        raise RuntimeError(
            f"command failed ({result.returncode}): {rendered}\n"
            f"stdout:\n{result.stdout[-4000:]}\n"
            f"stderr:\n{result.stderr[-4000:]}"
        )
    return elapsed_ms


def base_env(home: Path) -> dict[str, str]:
    env = os.environ.copy()
    env["HOME"] = str(home)
    env["COLDBREW_HOME"] = str(home / ".coldbrew")
    env.setdefault("CI", "1")
    return env


def sample(name: str, iterations: int, warmups: int, fn) -> dict:
    values: list[float] = []
    for _ in range(warmups):
        fn()
    for _ in range(iterations):
        values.append(fn())
    return {
        "scenario": name,
        "samples_ms": [round(v, 2) for v in values],
        "mean_ms": round(mean(values), 2),
        "median_ms": round(median(values), 2),
        "min_ms": round(min(values), 2),
        "max_ms": round(max(values), 2),
    }


def fixture_install(binary: Path, command_template: str, timeout: int) -> float:
    with tempfile.TemporaryDirectory(prefix="crew-fixture-install-") as tmp:
        tmp_path = Path(tmp)
        home = tmp_path / "home"
        home.mkdir()
        env = base_env(home)
        command = (
            command_template
            .replace("{crew}", shlex.quote(str(binary)))
            .replace("{home}", shlex.quote(str(home)))
            .replace("{tmp}", shlex.quote(str(tmp_path)))
        )
        return run_checked(command, env=env, shell=True, timeout=timeout)


def real_install(binary: Path, package: str, warm: bool, timeout: int) -> float:
    with tempfile.TemporaryDirectory(prefix=f"crew-real-{package}-") as tmp:
        home = Path(tmp) / "home"
        home.mkdir()
        env = base_env(home)
        if warm:
            run_checked([str(binary), "install", package], env=env, timeout=timeout)
            return run_checked([str(binary), "install", "--force", package], env=env, timeout=timeout)
        return run_checked([str(binary), "install", package], env=env, timeout=timeout)


def write_executable(path: Path, content: str) -> None:
    path.write_text(content)
    path.chmod(path.stat().st_mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)


def shim_startup(binary: Path, timeout: int) -> float:
    with tempfile.TemporaryDirectory(prefix="crew-shim-startup-") as tmp:
        tmp_path = Path(tmp)
        home = tmp_path / "home"
        coldbrew = home / ".coldbrew"
        cellar = coldbrew / "cellar" / "shimbench" / "1.0.0"
        bin_dir = cellar / "bin"
        shim_dir = coldbrew / "bin"
        tool_dir = tmp_path / "tools"
        bin_dir.mkdir(parents=True)
        shim_dir.mkdir(parents=True)
        tool_dir.mkdir()

        target = bin_dir / "shimbench"
        write_executable(target, "#!/bin/sh\nexit 0\n")
        write_executable(shim_dir / "shimbench", '#!/bin/sh\nexec crew exec shimbench shimbench "$@"\n')
        (tool_dir / "crew").symlink_to(binary)

        metadata = {
            "package": {
                "name": "shimbench",
                "version": "1.0.0",
                "tap": "homebrew/core",
                "cellar_path": str(cellar),
                "installed_at": "2026-01-01T00:00:00Z",
                "runtime_dependencies": [],
                "linked": True,
                "pinned": False,
                "bottle_tag": None,
                "bottle_sha256": None,
                "keg_only": False,
                "caveats": None,
                "binaries": ["shimbench"],
                "installed_as_dependency": False,
                "installed_for": None,
            },
            "formula_json": None,
            "receipt": {
                "installed_by": "perf-gate",
                "installed_at": "2026-01-01T00:00:00Z",
                "source": "synthetic",
                "checksum_verified": True,
            },
        }
        (cellar / ".coldbrew.json").write_text(json.dumps(metadata))

        env = base_env(home)
        env["PATH"] = f"{tool_dir}{os.pathsep}{env.get('PATH', '')}"
        return run_checked([str(shim_dir / "shimbench")], env=env, timeout=timeout)


def render_report(args, results: list[dict], skipped: list[str], failures: list[str], exception_reason: str | None) -> str:
    lines = [
        "# Coldbrew Performance Gate",
        "",
        f"- install regression budget: {args.install_threshold_pct:g}%",
        f"- shim startup budget: {args.shim_threshold_ms:g} ms",
        f"- iterations: {args.iterations} timed, {args.warmups} warmup",
        "",
        "| scenario | implementation | mean ms | median ms | min ms | max ms | samples ms |",
        "|---|---:|---:|---:|---:|---:|---|",
    ]
    for item in results:
        lines.append(
            f"| {item['scenario']} | {item['implementation']} | {item['mean_ms']:.2f} | "
            f"{item['median_ms']:.2f} | {item['min_ms']:.2f} | {item['max_ms']:.2f} | "
            f"{', '.join(f'{v:.2f}' for v in item['samples_ms'])} |"
        )

    if skipped:
        lines.extend(["", "## Skipped"])
        lines.extend(f"- {item}" for item in skipped)

    if failures:
        title = "Exceptions" if exception_reason else "Failures"
        lines.extend(["", f"## {title}"])
        if exception_reason:
            lines.append(f"- exception reason: {exception_reason}")
        lines.extend(f"- {item}" for item in failures)

    return "\n".join(lines) + "\n"


def main() -> int:
    parser = argparse.ArgumentParser(description="Run Coldbrew performance gates.")
    parser.add_argument("--rust-bin", default=os.environ.get("RUST_CREW_BIN"))
    parser.add_argument("--swift-bin", default=os.environ.get("SWIFT_CREW_BIN"))
    parser.add_argument("--fixture-install-cmd", default=os.environ.get("PERF_FIXTURE_INSTALL_CMD"))
    parser.add_argument("--real-install", action="append", default=[])
    parser.add_argument("--real-mode", choices=["cold", "warm", "both"], default=os.environ.get("PERF_REAL_MODE", "both"))
    parser.add_argument("--iterations", type=int, default=env_int("PERF_ITERATIONS", 5))
    parser.add_argument("--warmups", type=int, default=env_int("PERF_WARMUPS", 1))
    parser.add_argument("--install-threshold-pct", type=float, default=env_float("PERF_INSTALL_THRESHOLD_PCT", 10.0))
    parser.add_argument("--shim-threshold-ms", type=float, default=env_float("PERF_SHIM_STARTUP_MAX_MS", 50.0))
    parser.add_argument("--timeout", type=int, default=env_int("PERF_COMMAND_TIMEOUT_SECONDS", 900))
    parser.add_argument("--report", type=Path, default=Path(os.environ.get("PERF_REPORT", "perf-report.md")))
    parser.add_argument("--json", dest="json_path", type=Path, default=None)
    parser.add_argument("--exception-reason", default=os.environ.get("PERF_EXCEPTION_REASON"))
    args = parser.parse_args()

    rust = require_bin(args.rust_bin, "Rust")
    swift = require_bin(args.swift_bin, "Swift")
    if not rust and not swift:
        raise SystemExit("provide --rust-bin and/or --swift-bin")

    real_packages = args.real_install or [
        pkg for pkg in os.environ.get("PERF_REAL_INSTALLS", "").replace(",", " ").split() if pkg
    ]
    binaries = [("rust", rust), ("swift", swift)]
    binaries = [(label, binary) for label, binary in binaries if binary is not None]

    results: list[dict] = []
    skipped: list[str] = []
    failures: list[str] = []

    for label, binary in binaries:
        result = sample("shim_startup", args.iterations, args.warmups, lambda b=binary: shim_startup(b, args.timeout))
        result["implementation"] = label
        results.append(result)

        if args.fixture_install_cmd:
            result = sample(
                "fixture_install",
                args.iterations,
                args.warmups,
                lambda b=binary: fixture_install(b, args.fixture_install_cmd, args.timeout),
            )
            result["implementation"] = label
            results.append(result)

        for package in real_packages:
            modes = ["cold", "warm"] if args.real_mode == "both" else [args.real_mode]
            for mode in modes:
                result = sample(
                    f"real_install_{mode}:{package}",
                    args.iterations,
                    args.warmups,
                    lambda b=binary, p=package, m=mode: real_install(b, p, m == "warm", args.timeout),
                )
                result["implementation"] = label
                results.append(result)

    if not args.fixture_install_cmd:
        skipped.append("fixture install timing: set PERF_FIXTURE_INSTALL_CMD with {crew}, {home}, and {tmp} placeholders")
    if not real_packages:
        skipped.append("real jq/ffmpeg timing: pass --real-install jq --real-install ffmpeg or set PERF_REAL_INSTALLS=jq,ffmpeg")
    if not swift:
        skipped.append("Rust-vs-Swift comparison: pass --swift-bin or set SWIFT_CREW_BIN")
        skipped.append("Swift shim startup threshold: pass --swift-bin or set SWIFT_CREW_BIN")

    by_key = {(item["scenario"], item["implementation"]): item for item in results}
    for item in results:
        enforce_shim = item["implementation"] == "swift" or (swift and not rust)
        if item["scenario"] == "shim_startup" and enforce_shim and item["mean_ms"] > args.shim_threshold_ms:
            failures.append(
                f"{item['implementation']} shim startup mean {item['mean_ms']:.2f} ms exceeds {args.shim_threshold_ms:g} ms"
            )

    if rust and swift:
        scenarios = sorted({item["scenario"] for item in results if item["scenario"] != "shim_startup"})
        for scenario in scenarios:
            baseline = by_key.get((scenario, "rust"))
            candidate = by_key.get((scenario, "swift"))
            if not baseline or not candidate:
                continue
            allowed = baseline["mean_ms"] * (1 + args.install_threshold_pct / 100)
            if candidate["mean_ms"] > allowed:
                failures.append(
                    f"{scenario} swift mean {candidate['mean_ms']:.2f} ms exceeds rust mean "
                    f"{baseline['mean_ms']:.2f} ms + {args.install_threshold_pct:g}% ({allowed:.2f} ms)"
                )

    report = render_report(args, results, skipped, failures, args.exception_reason)
    args.report.parent.mkdir(parents=True, exist_ok=True)
    args.report.write_text(report)
    if args.json_path:
        args.json_path.parent.mkdir(parents=True, exist_ok=True)
        args.json_path.write_text(json.dumps({"results": results, "skipped": skipped, "failures": failures}, indent=2))

    print(report)
    if failures and not args.exception_reason:
        return 1
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except RuntimeError as error:
        print(error, file=sys.stderr)
        raise SystemExit(1)
