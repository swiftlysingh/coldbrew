import Foundation
import Testing
@testable import ColdbrewKit

@Test func storeCreatesAndMaterializesStoreEntry() throws {
    let temp = temporaryDirectory()
    let paths = Paths(root: temp.appendingPathComponent("coldbrew", isDirectory: true))
    try paths.createDirectories()
    let source = temp.appendingPathComponent("payload", isDirectory: true)
    try FileManager.default.createDirectory(at: source, withIntermediateDirectories: true)
    try Data("hello".utf8).write(to: source.appendingPathComponent("README"))

    let store = Store(paths: paths)
    let entry = try store.ensureEntry(sha256: "abc123", sourceDirectory: source)

    #expect(entry.created)
    #expect(store.entryExists(sha256: "abc123"))
    #expect(try store.entrySize(sha256: "abc123") == 5)

    let cellarPath = try store.materialize(sha256: "abc123", name: "hello", version: "1.0.0")
    #expect(FileManager.default.fileExists(atPath: cellarPath.appendingPathComponent("README").path))
}

@Test func storeRemoveEntryDeletesStoreDirectoryAndLock() throws {
    let temp = temporaryDirectory()
    let paths = Paths(root: temp)
    try paths.createDirectories()
    let store = Store(paths: paths)

    _ = try store.ensureEntry(sha256: "abc123")
    try Data().write(to: paths.storeLock(sha256: "abc123"))
    try store.removeEntry(sha256: "abc123")

    #expect(!store.entryExists(sha256: "abc123"))
    #expect(!FileManager.default.fileExists(atPath: paths.storeLock(sha256: "abc123").path))
}

@Test func storePreservesRelativeSymbolicLinks() throws {
    let temp = temporaryDirectory()
    let paths = Paths(root: temp.appendingPathComponent("coldbrew", isDirectory: true))
    try paths.createDirectories()
    let source = temp.appendingPathComponent("payload", isDirectory: true)
    let libexec = source.appendingPathComponent("libexec", isDirectory: true)
    let bin = source.appendingPathComponent("bin", isDirectory: true)
    try FileManager.default.createDirectory(at: libexec, withIntermediateDirectories: true)
    try FileManager.default.createDirectory(at: bin, withIntermediateDirectories: true)
    try Data("tool".utf8).write(to: libexec.appendingPathComponent("tool"))
    try FileManager.default.createSymbolicLink(
        atPath: bin.appendingPathComponent("tool").path,
        withDestinationPath: "../libexec/tool"
    )

    let store = Store(paths: paths)
    _ = try store.ensureEntry(sha256: "symlink", sourceDirectory: source)
    let cellar = try store.materialize(sha256: "symlink", name: "hello", version: "1.0.0")

    #expect(
        try FileManager.default.destinationOfSymbolicLink(
            atPath: cellar.appendingPathComponent("bin/tool").path
        ) == "../libexec/tool"
    )
}
