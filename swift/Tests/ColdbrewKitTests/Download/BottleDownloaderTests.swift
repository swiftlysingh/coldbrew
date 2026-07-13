import Foundation
import Testing
@testable import ColdbrewKit

@Test func bottleDownloaderCopiesFileUrlIntoVerifiedCache() async throws {
    let root = temporaryDirectory()
    let source = root.appendingPathComponent("hello.tar.gz")
    let data = Data("fixture bottle".utf8)
    try data.write(to: source)
    let sha = SHA256.hash(data)

    let paths = Paths(root: root.appendingPathComponent("coldbrew", isDirectory: true))
    try paths.createDirectories()
    let cache = Cache(paths: paths)
    let downloader = BottleDownloader(cache: cache)

    let result = try await downloader.downloadToCache(
        BottleDownloadRequest(url: source, sha256: sha, name: "hello", version: "1.0.0", tag: "fixture")
    )

    #expect(result.downloaded)
    #expect(result.bytesDownloaded == UInt64(data.count))
    #expect(result.path == paths.cacheBlob(sha256: sha))
    #expect(try SHA256.verify(file: result.path, expected: sha))
    #expect(try cache.list().map(\.label) == ["hello 1.0.0 (fixture)"])
}

@Test func bottleDownloaderReturnsCacheHitWithoutDownloading() async throws {
    let root = temporaryDirectory()
    let paths = Paths(root: root.appendingPathComponent("coldbrew", isDirectory: true))
    try paths.createDirectories()
    let cache = Cache(paths: paths)
    let data = Data("fixture bottle".utf8)
    let sha = SHA256.hash(data)
    try cache.storeBlob(sha256: sha, data: data)
    let missingSource = root.appendingPathComponent("missing.tar.gz")

    let result = try await BottleDownloader(cache: cache).downloadToCache(
        BottleDownloadRequest(url: missingSource, sha256: sha, name: "hello", version: "1.0.0", tag: "fixture")
    )

    #expect(!result.downloaded)
    #expect(result.bytesDownloaded == 0)
    #expect(result.path == paths.cacheBlob(sha256: sha))
}

@Test func bottleDownloaderHandlesConcurrentDownloadsOfSameBottle() async throws {
    let root = temporaryDirectory()
    let source = root.appendingPathComponent("hello.tar.gz")
    let data = Data(repeating: 42, count: 1_000_000)
    try data.write(to: source)
    let sha = SHA256.hash(data)
    let paths = Paths(root: root.appendingPathComponent("coldbrew", isDirectory: true))
    try paths.createDirectories()
    let downloader = BottleDownloader(cache: Cache(paths: paths))
    let request = BottleDownloadRequest(url: source, sha256: sha, name: "hello")

    async let first = downloader.downloadToCache(request)
    async let second = downloader.downloadToCache(request)
    let results = try await [first, second]

    #expect(results.allSatisfy { $0.path == paths.cacheBlob(sha256: sha) })
    #expect(try SHA256.verify(file: results[0].path, expected: sha))
}

@Test func bottleDownloaderRemovesTempFileOnChecksumMismatch() async throws {
    let root = temporaryDirectory()
    let source = root.appendingPathComponent("hello.tar.gz")
    try Data("fixture bottle".utf8).write(to: source)
    let paths = Paths(root: root.appendingPathComponent("coldbrew", isDirectory: true))
    try paths.createDirectories()
    let cache = Cache(paths: paths)

    do {
        _ = try await BottleDownloader(cache: cache).downloadToCache(
            BottleDownloadRequest(url: source, sha256: "wrong_hash", name: "hello")
        )
        Issue.record("expected checksum mismatch")
    } catch let error as ColdbrewError {
        #expect(error.description.contains("Checksum mismatch for 'hello'"))
    }

    #expect(!FileManager.default.fileExists(atPath: paths.cacheBlobTemp(sha256: "wrong_hash").path))
    #expect(!FileManager.default.fileExists(atPath: paths.cacheBlob(sha256: "wrong_hash").path))
}

@Test func bottleDownloaderParsesGhcrRepositoryFromBottleUrl() throws {
    let url = URL(string: "https://ghcr.io/v2/homebrew/core/jq/blobs/sha256:abc123")!

    #expect(try BottleDownloader.repository(fromBottleURL: url) == "homebrew/core/jq")
}
