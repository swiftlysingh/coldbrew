# Swift Cutover Prep

This branch is not the final Swift cutover. It adds the non-destructive guardrails
that make the final move explicit.

`swift-main` is not ready to move the Swift package to the repository root yet. As
of 2026-07-08, the branch does not contain `swift/Package.swift`, the executable
spec harness, or the parity evidence required by the migration plan.

## Root Move Gate

The final cutover PR may move `swift/Package.swift` to `Package.swift` only after
`.github/swift-cutover-gates.json` sets `rootPackageMoveAllowed` to `true` and all
`rootMovePrerequisites` have `status: "passed"` with evidence.

Until then, `.github/scripts/check-swift-cutover.py` fails if `Package.swift` is
added at the repo root.

## Required Evidence

| Gate | Required evidence |
|---|---|
| `swift-package` | Swift package has landed under `swift/` and builds on macOS. |
| `harness-rust` | Executable-spec harness is green against the Rust binary. |
| `harness-swift` | Same harness is green against the Swift binary. |
| `macos-parity` | Full macOS parity and real install smokes have passed. |
| `linux-static-sdk` | Linux static SDK release support is green. |
| `performance-startup` | Install-time and shim-startup thresholds have passed. |
| `rust-legacy-archive` | Rust tree archive branch is prepared. |

## Workflow Hooks

- Pull requests to `swift-main` that touch cutover-sensitive files run the cutover
  metadata validator.
- Tag releases run the same validator before publishing artifacts.

The current release workflow still builds the Rust `crew` binary. Swift release
artifacts are intentionally not enabled until the gate above is flipped.
