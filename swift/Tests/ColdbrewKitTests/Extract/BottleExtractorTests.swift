import Foundation
import Testing
@testable import ColdbrewKit

@Test func bottleExtractorUsesTarStripComponentsAndPreservesHardLinks() throws {
    let root = temporaryDirectory()
    let archive = try makeFixtureBottle(in: root)
    let destination = root.appendingPathComponent("extract", isDirectory: true)

    let result = try BottleExtractor().extract(archive, to: destination)

    #expect(result.sizeBytes > 0)
    #expect(FileManager.default.fileExists(atPath: destination.appendingPathComponent("bin/hello").path))
    #expect(!FileManager.default.fileExists(atPath: destination.appendingPathComponent("hello/1.0.0/bin/hello").path))

    let readme = destination.appendingPathComponent("README")
    let license = destination.appendingPathComponent("LICENSE")
    #expect(try inode(readme) == inode(license))
}

@Test func storeCanCreateEntryFromBottleArchive() throws {
    let root = temporaryDirectory()
    let archive = try makeFixtureBottle(in: root)
    let paths = Paths(root: root.appendingPathComponent("coldbrew", isDirectory: true))
    try paths.createDirectories()

    let store = Store(paths: paths)
    let entry = try store.ensureEntry(sha256: "fixture-sha", bottleArchive: archive)

    #expect(entry.created)
    #expect(FileManager.default.fileExists(atPath: paths.storeEntry(sha256: "fixture-sha").appendingPathComponent("bin/hello").path))
}

private func makeFixtureBottle(in root: URL) throws -> URL {
    let prefix = root.appendingPathComponent("payload/hello/1.0.0", isDirectory: true)
    let bin = prefix.appendingPathComponent("bin", isDirectory: true)
    try FileManager.default.createDirectory(at: bin, withIntermediateDirectories: true)
    try Data("#!/bin/sh\necho hello\n".utf8).write(to: bin.appendingPathComponent("hello"))
    try Data("same inode".utf8).write(to: prefix.appendingPathComponent("README"))
    try FileManager.default.linkItem(at: prefix.appendingPathComponent("README"), to: prefix.appendingPathComponent("LICENSE"))

    let archive = root.appendingPathComponent("hello.tar.gz")
    try ProcessRunner.run("tar", ["-czf", archive.path, "-C", root.appendingPathComponent("payload").path, "hello"])
    return archive
}

private func inode(_ url: URL) throws -> UInt64 {
    let attrs = try FileManager.default.attributesOfItem(atPath: url.path)
    return (attrs[.systemFileNumber] as? NSNumber)?.uint64Value ?? 0
}
