import Foundation
import Testing
@testable import ColdbrewKit

@Test func uninstallRemovesCellarShimsConfigAndStoreRef() async throws {
    let root = temporaryDirectory()
    let paths = Paths(root: root.appendingPathComponent("coldbrew", isDirectory: true))
    let bottle = try makeCleanupBottle(root: root, name: "hello", version: "1.0.0", binary: "hello")
    _ = try await InstallManager(paths: paths).install([
        InstallRequest(name: "hello", version: "1.0.0", bottleURL: bottle.url, sha256: bottle.sha, tag: "fixture", binaries: ["hello"]),
    ])
    try PackageOperations(paths: paths).setDefault("hello@1.0.0")
    try PackageOperations(paths: paths).pin("hello")

    let result = try UninstallCleanupManager(paths: paths).uninstall("hello")

    #expect(result.removed == [RemovedPackage(name: "hello", version: "1.0.0")])
    #expect(!FileManager.default.fileExists(atPath: paths.cellarPackage("hello", version: "1.0.0").path))
    #expect(!FileManager.default.fileExists(atPath: paths.shim("hello").path))
    #expect(try cleanupStoreRefCount(paths: paths, package: "hello", version: "1.0.0") == 0)
    let config = try String(contentsOf: paths.configFile)
    #expect(!config.contains("\"hello\" = \"1.0.0\""))
}

@Test func uninstallWithDepsRemovesOrphanDependency() async throws {
    let root = temporaryDirectory()
    let paths = Paths(root: root.appendingPathComponent("coldbrew", isDirectory: true))
    let dep = try makeCleanupBottle(root: root, name: "dep", version: "1.0.0", binary: "dep")
    let app = try makeCleanupBottle(root: root, name: "app", version: "1.0.0", binary: "app")
    _ = try await InstallManager(paths: paths).install([
        InstallRequest(name: "dep", version: "1.0.0", bottleURL: dep.url, sha256: dep.sha, tag: "fixture", binaries: ["dep"], installedAsDependency: true, installedFor: "app"),
        InstallRequest(
            name: "app",
            version: "1.0.0",
            bottleURL: app.url,
            sha256: app.sha,
            tag: "fixture",
            binaries: ["app"],
            runtimeDependencies: [RuntimeDependencyRecord(name: "dep", version: "1.0.0", path: paths.cellarPackage("dep", version: "1.0.0").path)]
        ),
    ])

    let manager = UninstallCleanupManager(paths: paths)
    #expect(try manager.dependents(of: "dep") == ["app"])
    _ = try manager.uninstall("app", options: UninstallOptions(withDeps: true))

    #expect(!FileManager.default.fileExists(atPath: paths.cellarPackage("app", version: "1.0.0").path))
    #expect(!FileManager.default.fileExists(atPath: paths.cellarPackage("dep", version: "1.0.0").path))
}

@Test func uninstallWithDepsRecursivelyRemovesOrphanChain() async throws {
    let root = temporaryDirectory()
    let paths = Paths(root: root.appendingPathComponent("coldbrew", isDirectory: true))
    let leaf = try makeCleanupBottle(root: root, name: "leaf", version: "1.0.0", binary: "leaf")
    let middle = try makeCleanupBottle(root: root, name: "middle", version: "1.0.0", binary: "middle")
    let app = try makeCleanupBottle(root: root, name: "app-chain", version: "1.0.0", binary: "app-chain")
    _ = try await InstallManager(paths: paths).install([
        InstallRequest(name: "leaf", version: "1.0.0", bottleURL: leaf.url, sha256: leaf.sha, tag: "fixture", installedAsDependency: true),
        InstallRequest(name: "middle", version: "1.0.0", bottleURL: middle.url, sha256: middle.sha, tag: "fixture", runtimeDependencies: [RuntimeDependencyRecord(name: "leaf", version: "1.0.0", path: paths.cellarPackage("leaf", version: "1.0.0").path)], installedAsDependency: true),
        InstallRequest(name: "app-chain", version: "1.0.0", bottleURL: app.url, sha256: app.sha, tag: "fixture", runtimeDependencies: [RuntimeDependencyRecord(name: "middle", version: "1.0.0", path: paths.cellarPackage("middle", version: "1.0.0").path)]),
    ])

    let result = try UninstallCleanupManager(paths: paths).uninstall("app-chain", options: UninstallOptions(withDeps: true))

    #expect(result.removed.map(\.name) == ["app-chain", "middle", "leaf"])
    #expect(try Cellar(paths: paths).listPackages().isEmpty)
}

@Test func cleanupCollectsAndRemovesCacheAndOrphanedStore() throws {
    let root = temporaryDirectory()
    let paths = Paths(root: root)
    try paths.createDirectories()
    let cache = Cache(paths: paths)
    try cache.storeBlob(sha256: "cache-sha", data: Data("cache".utf8))
    try cache.recordBlobMetadata(sha256: "cache-sha", name: "hello", version: "1.0.0", tag: "fixture", sizeBytes: 5)
    try FileManager.default.createDirectory(at: paths.storeEntry(sha256: "orphan-sha"), withIntermediateDirectories: true)
    try Data("store".utf8).write(to: paths.storeEntry(sha256: "orphan-sha").appendingPathComponent("file"))
    let db = Database(paths: paths)
    try db.upsertStoreEntry(db.connect(), sha256: "orphan-sha", sizeBytes: 5)
    let manager = UninstallCleanupManager(paths: paths)

    let summary = try manager.spaceSummary()
    #expect(summary.categories.flatMap(\.items).map(\.kind).sorted { $0.rawValue < $1.rawValue } == [.cacheDownloads, .orphanedStore])
    let dryRun = try manager.clean(kinds: Set(CleanupKind.allCases), dryRun: true)
    #expect(dryRun.removed == 2)
    #expect(FileManager.default.fileExists(atPath: paths.cacheBlob(sha256: "cache-sha").path))

    let cleaned = try manager.clean(kinds: Set(CleanupKind.allCases))
    #expect(cleaned.removed == 2)
    #expect(!FileManager.default.fileExists(atPath: paths.cacheBlob(sha256: "cache-sha").path))
    #expect(!FileManager.default.fileExists(atPath: paths.storeEntry(sha256: "orphan-sha").path))
}

private func makeCleanupBottle(root: URL, name: String, version: String, binary: String) throws -> (url: URL, sha: String) {
    let prefix = root.appendingPathComponent("payload-\(name)/\(name)/\(version)", isDirectory: true)
    let bin = prefix.appendingPathComponent("bin", isDirectory: true)
    try FileManager.default.createDirectory(at: bin, withIntermediateDirectories: true)
    try Data("#!/bin/sh\necho \(name)\n".utf8).write(to: bin.appendingPathComponent(binary))
    let archive = root.appendingPathComponent("\(name)-\(version).tar.gz")
    try ProcessRunner.run("tar", ["-czf", archive.path, "-C", root.appendingPathComponent("payload-\(name)").path, name])
    return (archive, try SHA256.hash(file: archive))
}

private func cleanupStoreRefCount(paths: Paths, package: String, version: String) throws -> Int64 {
    let connection = try Database(paths: paths).connect()
    return try connection.int64("SELECT COUNT(*) FROM store_refs WHERE package = '\(package)' AND version = '\(version)'") ?? 0
}
