# Swift Linux Static SDK CI

Status: armed, required once the Swift package exists on `swift-main`.

The `Swift Linux static SDK (x86_64 musl)` CI job is intentionally present before
`swift/Package.swift` lands. On the current `swift-main` baseline it reports a
notice and exits green because there is no Swift package to build. As soon as
`swift/Package.swift` is merged, the same job becomes a required build gate:

- install the pinned Swift 6.3.3 toolchain after verifying its Swift.org PGP signature
- install the matching Swift Static Linux SDK
- run `swift test --swift-sdk x86_64-swift-linux-musl`
- build `crew` with `swift build -c release --swift-sdk x86_64-swift-linux-musl`
- verify the produced x86_64 `crew` binary has no dynamic ELF interpreter
- run the full fixture and cross-binary harness, including ignored lifecycle scenarios

Keep `SWIFT_VERSION`, `SWIFT_TOOLCHAIN_URL`, `SWIFT_STATIC_SDK_URL`, and
`SWIFT_STATIC_SDK_CHECKSUM` in `.github/workflows/ci.yml` together. Swift.org
requires the Static Linux SDK to match the installed Swift toolchain version and
to be installed with its published checksum.

Transition rule: this job may only skip while `swift/Package.swift` is absent.
After the Swift package lands, failures are real Linux/static-SDK regressions, not
allowed-fail migration noise.
