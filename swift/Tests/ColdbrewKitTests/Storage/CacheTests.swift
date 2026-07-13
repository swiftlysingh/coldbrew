import Foundation
import Testing
@testable import ColdbrewKit

@Test func cacheStoresListsAndRemovesBlobMetadata() throws {
    let temp = temporaryDirectory()
    let paths = Paths(root: temp)
    try paths.createDirectories()
    let cache = Cache(paths: paths)

    let path = try cache.storeBlob(sha256: "abc123", data: Data("payload".utf8))
    try cache.recordBlobMetadata(
        sha256: "abc123",
        name: "hello",
        version: "1.0.0",
        tag: "arm64_sequoia",
        sizeBytes: 7
    )

    #expect(path == paths.cacheBlob(sha256: "abc123"))
    #expect(cache.isCached(sha256: "abc123"))
    #expect(try cache.totalSize() == 7)
    #expect(try cache.list().map(\.label) == ["hello 1.0.0 (arm64_sequoia)"])

    try cache.remove(sha256: "abc123")
    #expect(!cache.isCached(sha256: "abc123"))
    #expect(try cache.list().isEmpty)
}

@Test func cacheScansBlobDirectoryWhenDbMetadataIsMissing() throws {
    let temp = temporaryDirectory()
    let paths = Paths(root: temp)
    try paths.createDirectories()
    let cache = Cache(paths: paths)

    try cache.storeBlob(sha256: "deadbeef", data: Data("payload".utf8))

    let bottles = try cache.list()
    #expect(bottles.count == 1)
    #expect(bottles[0].sha256 == "deadbeef")
    #expect(bottles[0].label == "blob deadbeef")
}

@Test func cacheListIncludesBlobsMissingFromDbMetadata() throws {
    let paths = Paths(root: temporaryDirectory())
    try paths.createDirectories()
    let cache = Cache(paths: paths)
    try cache.storeBlob(sha256: "metadata", data: Data("one".utf8))
    try cache.recordBlobMetadata(sha256: "metadata", name: "hello", version: "1.0.0", tag: "fixture", sizeBytes: 3)
    try cache.storeBlob(sha256: "orphan", data: Data("two".utf8))

    #expect(try cache.list().map(\.label) == ["blob orphan", "hello 1.0.0 (fixture)"])
}
