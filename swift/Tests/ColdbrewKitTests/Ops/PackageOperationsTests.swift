import Foundation
import Testing
@testable import ColdbrewKit

@Test func packageOperationsLinkUnlinkAndWhichShim() throws {
    let paths = Paths(root: temporaryDirectory())
    try paths.createDirectories()
    try installFixturePackage(paths: paths, name: "hello", version: "1.0.0", binaries: ["hello"])
    let ops = PackageOperations(paths: paths)

    let linked = try ops.link("hello")

    #expect(linked == LinkResult(package: "hello", version: "1.0.0", binaries: ["hello"]))
    if case let .shim(binary, package, path, versions) = try ops.which("hello") {
        #expect(binary == "hello")
        #expect(package == "hello")
        #expect(path.lastPathComponent == "hello")
        #expect(versions == ["1.0.0"])
    } else {
        Issue.record("expected shim result")
    }

    let unlinked = try ops.unlink("hello")
    #expect(unlinked.binaries == ["hello"])
    #expect(try ops.which("hello") == .binary(
        binary: "hello",
        package: "hello",
        version: "1.0.0",
        path: paths.cellarPackage("hello", version: "1.0.0").appendingPathComponent("bin/hello")
    ))
}

@Test func packageOperationsRequireForceForExistingShim() throws {
    let paths = Paths(root: temporaryDirectory())
    try paths.createDirectories()
    try installFixturePackage(paths: paths, name: "hello", version: "1.0.0", binaries: ["hello"])
    let ops = PackageOperations(paths: paths)

    _ = try ops.link("hello")
    do {
        _ = try ops.link("hello")
        Issue.record("expected existing shim error")
    } catch let error as ColdbrewError {
        #expect(error.description.contains("Shim already exists"))
    }

    #expect(try ops.link("hello", force: true).binaries == ["hello"])
}

@Test func packageOperationsPinUnpinAndDefaultUseConfigToml() throws {
    let paths = Paths(root: temporaryDirectory())
    try paths.createDirectories()
    try installFixturePackage(paths: paths, name: "hello", version: "1.0.0", binaries: [])
    try installFixturePackage(paths: paths, name: "hello", version: "2.0.0", binaries: [])
    let ops = PackageOperations(paths: paths)
    let settings = Settings(autoUpdate: false, parallelDownloads: 3, keepVersions: 7, analytics: true)
    try GlobalConfig(settings: settings).save(to: paths.configFile)

    try ops.pin("hello")
    try ops.setDefault("hello@1.0.0")

    let config = try String(contentsOf: paths.configFile)
    #expect(config.contains("[defaults]"))
    #expect(config.contains("hello = \"1.0.0\""))
    #expect(config.contains("[pins]"))
    #expect(config.contains("hello = \"2.0.0\""))
    #expect(try ops.defaultVersions("hello").defaultVersion == "1.0.0")
    #expect(try ops.unpin("hello"))
    #expect(try !ops.unpin("hello"))
    #expect(try GlobalConfig.load(from: paths.configFile).settings == settings)
}

@Test func parsePackageSpecMatchesRustFirstAtSplit() {
    let parsed = parsePackageSpec("multi@1@2.0.0")
    #expect(parsed.0 == "multi")
    #expect(parsed.1 == "1@2.0.0")
}

private func installFixturePackage(paths: Paths, name: String, version: String, binaries: [String]) throws {
    let packageDir = paths.cellarPackage(name, version: version)
    let binDir = packageDir.appendingPathComponent("bin", isDirectory: true)
    try FileManager.default.createDirectory(at: binDir, withIntermediateDirectories: true)
    for binary in binaries {
        let binaryURL = binDir.appendingPathComponent(binary)
        try Data("#!/bin/sh\n".utf8).write(to: binaryURL)
    }

    let package = InstalledPackageRecord(
        name: name,
        version: version,
        cellarPath: packageDir.path,
        binaries: binaries
    )
    try Cellar(paths: paths).saveMetadata(PackageMetadataRecord(
        package: package,
        receipt: InstallReceiptRecord(source: "fixture")
    ))
}
