import Foundation
import Testing
@testable import ColdbrewKit

#if os(Linux)
import Glibc
#else
import Darwin
#endif

@Test func pathsUseColdbrewHomeEnvironmentOverride() throws {
    let directory = temporaryDirectory()
    setenv("COLDBREW_HOME", directory.path, 1)
    defer { unsetenv("COLDBREW_HOME") }

    let paths = try Paths()

    #expect(paths.root.path == directory.path)
    #expect(paths.dbFile.path == directory.appendingPathComponent("db/coldbrew.sqlite3").path)
}

@Test func pathsMatchRustLayout() throws {
    let root = temporaryDirectory()
    let paths = Paths(root: root)

    #expect(paths.binDir.path == root.appendingPathComponent("bin").path)
    #expect(paths.cellarDir.path == root.appendingPathComponent("cellar").path)
    #expect(paths.cacheDir.path == root.appendingPathComponent("cache").path)
    #expect(paths.downloadsDir.path == root.appendingPathComponent("cache/downloads").path)
    #expect(paths.cacheBlobsDir.path == root.appendingPathComponent("cache/blobs").path)
    #expect(paths.tapsDir.path == root.appendingPathComponent("taps").path)
    #expect(paths.indexDir.path == root.appendingPathComponent("index").path)
    #expect(paths.logsDir.path == root.appendingPathComponent("logs").path)
    #expect(paths.dbDir.path == root.appendingPathComponent("db").path)
    #expect(paths.storeDir.path == root.appendingPathComponent("store").path)
    #expect(paths.locksDir.path == root.appendingPathComponent("locks").path)
    #expect(paths.configFile.path == root.appendingPathComponent("config.toml").path)
    #expect(paths.formulaIndex.path == root.appendingPathComponent("index/formula.json").path)
    #expect(paths.defaultsFile.path == root.appendingPathComponent("defaults.json").path)
    #expect(paths.pinsFile.path == root.appendingPathComponent("pins.json").path)
    #expect(paths.shimsLock.path == root.appendingPathComponent("locks/shims.lock").path)

    #expect(paths.cellarPackage("jq", version: "1.7.1").path == root.appendingPathComponent("cellar/jq/1.7.1").path)
    #expect(paths.tapDir(user: "homebrew", repo: "core").path == root.appendingPathComponent("taps/homebrew/core").path)
    #expect(paths.cacheBottle(name: "jq", version: "1.7.1", tag: "arm64_sequoia").path == root.appendingPathComponent("cache/downloads/jq-1.7.1.arm64_sequoia.bottle.tar.gz").path)
    #expect(paths.cacheBlob(sha256: "abc").path == root.appendingPathComponent("cache/blobs/abc.bottle.tar.gz").path)
    let temp1 = paths.cacheBlobTemp(sha256: "abc")
    let temp2 = paths.cacheBlobTemp(sha256: "abc")
    #expect(temp1.deletingLastPathComponent().path == paths.cacheBlobsDir.path)
    #expect(temp1.lastPathComponent.hasPrefix("abc."))
    #expect(temp1.pathExtension == "part")
    #expect(temp1 != temp2)
    #expect(paths.storeEntry(sha256: "abc").path == root.appendingPathComponent("store/abc").path)
    #expect(paths.storeLock(sha256: "abc").path == root.appendingPathComponent("locks/abc.lock").path)
    #expect(paths.shim("jq").path == root.appendingPathComponent("bin/jq").path)
    #expect(paths.packageMetadata(name: "jq", version: "1.7.1").path == root.appendingPathComponent("cellar/jq/1.7.1/.coldbrew.json").path)
}

@Test func createDirectoriesCreatesRustStorageDirectories() throws {
    let paths = Paths(root: temporaryDirectory())

    try paths.createDirectories()

    for directory in [
        paths.root,
        paths.binDir,
        paths.cellarDir,
        paths.cacheDir,
        paths.cacheBlobsDir,
        paths.tapsDir,
        paths.indexDir,
        paths.logsDir,
        paths.dbDir,
        paths.storeDir,
        paths.locksDir,
    ] {
        var isDirectory = ObjCBool(false)
        #expect(FileManager.default.fileExists(atPath: directory.path, isDirectory: &isDirectory))
        #expect(isDirectory.boolValue)
    }
}

@Test func projectAndLockfilePathHelpersMatchRust() throws {
    let root = temporaryDirectory()
    let project = root.appendingPathComponent("coldbrew.toml")
    let subdirectory = root.appendingPathComponent("src/lib", isDirectory: true)
    try FileManager.default.createDirectory(at: subdirectory, withIntermediateDirectories: true)
    try "".write(to: project, atomically: true, encoding: .utf8)

    #expect(findProjectFile(startDirectory: subdirectory)?.path == project.path)
    #expect(lockfilePath(projectFile: project).path == root.appendingPathComponent("coldbrew.lock").path)
}
