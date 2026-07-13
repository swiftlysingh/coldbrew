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

@Test func copyTreePreservesRelativeSymbolicLinks() throws {
    let temp = temporaryDirectory()
    let source = temp.appendingPathComponent("source", isDirectory: true)
    let destination = temp.appendingPathComponent("destination", isDirectory: true)
    try FileManager.default.createDirectory(at: source.appendingPathComponent("real"), withIntermediateDirectories: true)
    try Data("hello".utf8).write(to: source.appendingPathComponent("real/tool"))
    try FileManager.default.createSymbolicLink(atPath: source.appendingPathComponent("tool").path, withDestinationPath: "real/tool")

    try copyTree(from: source, to: destination)

    #expect(try FileManager.default.destinationOfSymbolicLink(atPath: destination.appendingPathComponent("tool").path) == "real/tool")
}
