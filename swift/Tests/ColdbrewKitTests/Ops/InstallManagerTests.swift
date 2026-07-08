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

private func makeInstallBottle(root: URL, name: String, version: String, binary: String) throws -> (url: URL, sha: String) {
    let prefix = root.appendingPathComponent("payload-\(name)/\(name)/\(version)", isDirectory: true)
    let bin = prefix.appendingPathComponent("bin", isDirectory: true)
    try FileManager.default.createDirectory(at: bin, withIntermediateDirectories: true)
    try Data("#!/bin/sh\necho \(name)\n".utf8).write(to: bin.appendingPathComponent(binary))
    let archive = root.appendingPathComponent("\(name)-\(version).tar.gz")
    try ProcessRunner.run("tar", ["-czf", archive.path, "-C", root.appendingPathComponent("payload-\(name)").path, name])
    return (archive, try SHA256.hash(file: archive))
}

private func storeRefCount(paths: Paths, package: String, version: String) throws -> Int64 {
    let connection = try Database(paths: paths).connect()
    return try connection.int64("SELECT COUNT(*) FROM store_refs WHERE package = '\(package)' AND version = '\(version)'") ?? 0
}
