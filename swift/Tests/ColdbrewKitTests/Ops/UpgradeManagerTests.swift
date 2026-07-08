import Foundation
import Testing
@testable import ColdbrewKit

@Test func upgradeManagerPlansAvailableUnpinnedUpgrades() async throws {
    let root = temporaryDirectory()
    let paths = Paths(root: root.appendingPathComponent("coldbrew", isDirectory: true))
    let old = try makeUpgradeBottle(root: root, name: "hello", version: "1.0.0")
    let new = try makeUpgradeBottle(root: root, name: "hello", version: "2.0.0")
    _ = try await InstallManager(paths: paths).install([
        InstallRequest(name: "hello", version: "1.0.0", bottleURL: old.url, sha256: old.sha, tag: "fixture", binaries: ["hello"]),
    ])

    let manager = UpgradeManager(paths: paths)
    let plan = try manager.plan(available: [
        InstallRequest(name: "hello", version: "2.0.0", bottleURL: new.url, sha256: new.sha, tag: "fixture", binaries: ["hello"]),
    ])

    #expect(plan.upgrades == [UpgradeInfo(name: "hello", currentVersion: "1.0.0", newVersion: "2.0.0", isMajor: true)])

    try PackageOperations(paths: paths).pin("hello@1.0.0")
    #expect(try manager.plan(available: [
        InstallRequest(name: "hello", version: "2.0.0", bottleURL: new.url, sha256: new.sha, tag: "fixture", binaries: ["hello"]),
    ]).upgrades.isEmpty)
}

@Test func upgradeManagerAppliesOnlyWhenYesIsSet() async throws {
    let root = temporaryDirectory()
    let paths = Paths(root: root.appendingPathComponent("coldbrew", isDirectory: true))
    let old = try makeUpgradeBottle(root: root, name: "hello", version: "1.0.0")
    let new = try makeUpgradeBottle(root: root, name: "hello", version: "1.1.0")
    let request = InstallRequest(name: "hello", version: "1.1.0", bottleURL: new.url, sha256: new.sha, tag: "fixture", binaries: ["hello"])
    _ = try await InstallManager(paths: paths).install([
        InstallRequest(name: "hello", version: "1.0.0", bottleURL: old.url, sha256: old.sha, tag: "fixture", binaries: ["hello"]),
    ])
    let manager = UpgradeManager(paths: paths)
    let plan = try manager.plan(available: [request], filter: ["hello"])

    #expect(try await manager.apply(plan, available: [request], yes: false).upgraded.isEmpty)
    #expect(FileManager.default.fileExists(atPath: paths.cellarPackage("hello", version: "1.0.0").path))

    let result = try await manager.apply(plan, available: [request], yes: true)
    #expect(result.upgraded.map(\.newVersion) == ["1.1.0"])
    #expect(FileManager.default.fileExists(atPath: paths.cellarPackage("hello", version: "1.1.0").path))
    #expect(try PackageOperations(paths: paths).defaultVersions("hello").defaultVersion == "1.1.0")
}

private func makeUpgradeBottle(root: URL, name: String, version: String) throws -> (url: URL, sha: String) {
    let prefix = root.appendingPathComponent("payload-\(name)-\(version)/\(name)/\(version)", isDirectory: true)
    let bin = prefix.appendingPathComponent("bin", isDirectory: true)
    try FileManager.default.createDirectory(at: bin, withIntermediateDirectories: true)
    try Data("#!/bin/sh\necho \(name) \(version)\n".utf8).write(to: bin.appendingPathComponent(name))
    let archive = root.appendingPathComponent("\(name)-\(version).tar.gz")
    try ProcessRunner.run("tar", ["-czf", archive.path, "-C", root.appendingPathComponent("payload-\(name)-\(version)").path, name])
    return (archive, try SHA256.hash(file: archive))
}
