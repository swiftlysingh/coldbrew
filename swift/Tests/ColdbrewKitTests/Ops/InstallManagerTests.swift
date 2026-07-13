import Foundation
import Testing
@testable import ColdbrewKit

@Test func installManagerInstallsFixtureBottlesWithDepsAndShims() async throws {
    let root = temporaryDirectory()
    let paths = Paths(root: root.appendingPathComponent("coldbrew", isDirectory: true))
    let dep = try makeInstallBottle(root: root, name: "dep", version: "1.0.0", binary: "dep")
    let hello = try makeInstallBottle(root: root, name: "hello", version: "1.0.0", binary: "hello")

    let result = try await InstallManager(paths: paths).install([
        InstallRequest(name: "dep", version: "1.0.0", bottleURL: dep.url, sha256: dep.sha, tag: "fixture", binaries: ["dep"], installedAsDependency: true, installedFor: "hello"),
        InstallRequest(
            name: "hello",
            version: "1.0.0",
            bottleURL: hello.url,
            sha256: hello.sha,
            tag: "fixture",
            binaries: ["hello"],
            runtimeDependencies: [RuntimeDependencyRecord(name: "dep", version: "1.0.0", path: paths.cellarPackage("dep", version: "1.0.0").path)]
        ),
    ], options: InstallOptions(maxConcurrentDownloads: 2))

    #expect(result.packages.map(\.name) == ["dep", "hello"])
    #expect(FileManager.default.fileExists(atPath: paths.cellarPackage("hello", version: "1.0.0").appendingPathComponent("bin/hello").path))
    #expect(FileManager.default.fileExists(atPath: paths.shim("hello").path))
    #expect(try Cellar(paths: paths).package(name: "hello", version: "1.0.0").runtimeDependencies.map(\.name) == ["dep"])
    #expect(try storeRefCount(paths: paths, package: "dep", version: "1.0.0") == 1)
    #expect(try storeRefCount(paths: paths, package: "hello", version: "1.0.0") == 1)
}

@Test func installManagerRejectsExistingPackageUnlessForceIsSet() async throws {
    let root = temporaryDirectory()
    let paths = Paths(root: root.appendingPathComponent("coldbrew", isDirectory: true))
    let bottle = try makeInstallBottle(root: root, name: "hello", version: "1.0.0", binary: "hello")
    let request = InstallRequest(name: "hello", version: "1.0.0", bottleURL: bottle.url, sha256: bottle.sha, tag: "fixture", binaries: ["hello"])
    let manager = InstallManager(paths: paths)

    _ = try await manager.install([request])
    do {
        _ = try await manager.install([request])
        Issue.record("expected package already installed")
    } catch let error as ColdbrewError {
        #expect(error == .packageAlreadyInstalled(name: "hello", version: "1.0.0"))
    }

    let forced = try await manager.install([request], options: InstallOptions(force: true))
    #expect(forced.packages.map(\.name) == ["hello"])
}

@Test func installManagerDiscoversBottleBinaryNames() async throws {
    let root = temporaryDirectory()
    let paths = Paths(root: root.appendingPathComponent("coldbrew", isDirectory: true))
    let bottle = try makeInstallBottle(root: root, name: "ripgrep", version: "1.0.0", binary: "rg")

    _ = try await InstallManager(paths: paths).install([
        InstallRequest(name: "ripgrep", version: "1.0.0", bottleURL: bottle.url, sha256: bottle.sha, tag: "fixture", binaries: ["ripgrep"]),
    ])

    #expect(FileManager.default.fileExists(atPath: paths.shim("rg").path))
    #expect(!FileManager.default.fileExists(atPath: paths.shim("ripgrep").path))
    #expect(try Cellar(paths: paths).package(name: "ripgrep", version: "1.0.0").binaries == ["rg"])
    #expect(try FileManager.default.destinationOfSymbolicLink(atPath: paths.root.appendingPathComponent("opt/ripgrep").path) == paths.cellarPackage("ripgrep", version: "1.0.0").path)
}

@Test func forceInstallRestoresWorkingPackageWhenReplacementFailsAfterSwap() async throws {
    let root = temporaryDirectory()
    let paths = Paths(root: root.appendingPathComponent("coldbrew", isDirectory: true))
    let old = try makeInstallBottle(root: root, name: "hello", version: "1.0.0", binary: "hello", output: "old")
    let manager = InstallManager(paths: paths)
    _ = try await manager.install([
        InstallRequest(name: "hello", version: "1.0.0", bottleURL: old.url, sha256: old.sha, tag: "fixture", binaries: ["hello"]),
    ])
    let replacement = try makeInstallBottle(root: root, name: "hello", version: "1.0.0", binary: "hello", output: "new", poisonMetadata: true)

    do {
        _ = try await manager.install([
            InstallRequest(name: "hello", version: "1.0.0", bottleURL: replacement.url, sha256: replacement.sha, tag: "fixture", binaries: ["hello"]),
        ], options: InstallOptions(force: true))
        Issue.record("expected metadata write failure")
    } catch {}

    let binary = paths.cellarPackage("hello", version: "1.0.0").appendingPathComponent("bin/hello")
    #expect(try String(contentsOf: binary).contains("old"))
    #expect(try Cellar(paths: paths).package(name: "hello", version: "1.0.0").bottleSha256 == old.sha)
    #expect(try storeRefCount(paths: paths, package: "hello", version: "1.0.0") == 1)
}

private func makeInstallBottle(root: URL, name: String, version: String, binary: String, output: String? = nil, poisonMetadata: Bool = false) throws -> (url: URL, sha: String) {
    let fixture = "payload-\(name)-\(UUID().uuidString)"
    let prefix = root.appendingPathComponent("\(fixture)/\(name)/\(version)", isDirectory: true)
    let bin = prefix.appendingPathComponent("bin", isDirectory: true)
    try FileManager.default.createDirectory(at: bin, withIntermediateDirectories: true)
    try Data("#!/bin/sh\necho \(output ?? name)\n".utf8).write(to: bin.appendingPathComponent(binary))
    if poisonMetadata {
        try FileManager.default.createDirectory(at: prefix.appendingPathComponent(".coldbrew.json"), withIntermediateDirectories: true)
    }
    let archive = root.appendingPathComponent("\(fixture).tar.gz")
    try ProcessRunner.run("tar", ["-czf", archive.path, "-C", root.appendingPathComponent(fixture).path, name])
    return (archive, try SHA256.hash(file: archive))
}

private func storeRefCount(paths: Paths, package: String, version: String) throws -> Int64 {
    let connection = try Database(paths: paths).connect()
    return try connection.int64("SELECT COUNT(*) FROM store_refs WHERE package = '\(package)' AND version = '\(version)'") ?? 0
}
