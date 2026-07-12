import Foundation
import Testing
@testable import ColdbrewKit

@Test func databaseCreatesRustSchemaAndPragmas() throws {
    let database = Database(paths: Paths(root: temporaryDirectory()))
    let connection = try database.connect()

    #expect(try connection.int64("PRAGMA user_version") == 3)
    #expect(try connection.string("PRAGMA journal_mode")?.lowercased() == "wal")
    #expect(try connection.int64("PRAGMA synchronous") == 1)
    #expect(try connection.int64("PRAGMA foreign_keys") == 1)

    let tables = Set(try connection.rows("SELECT name FROM sqlite_master WHERE type = 'table'").compactMap { $0.text(0) })
    #expect(tables.contains("api_cache"))
    #expect(tables.contains("blob_cache"))
    #expect(tables.contains("store_entries"))
    #expect(tables.contains("store_refs"))
}

@Test func databaseRoundTripsApiAndBlobCacheRows() throws {
    let database = Database(paths: Paths(root: temporaryDirectory()))
    let connection = try database.connect()

    try database.upsertApiCache(connection, url: "https://example.test/formula.json", etag: "abc", lastModified: "today")
    let apiEntry = try database.getApiCache(connection, url: "https://example.test/formula.json")
    #expect(apiEntry?.etag == "abc")
    #expect(apiEntry?.lastModified == "today")
    #expect(apiEntry?.cachedAt ?? 0 > 0)

    try database.upsertBlobCache(
        connection,
        sha256: "sha256_blob",
        name: "jq",
        version: "1.7.1",
        tag: "arm64_sequoia",
        sizeBytes: 123
    )
    let blobs = try database.listBlobCache(connection)
    #expect(blobs == [
        BlobCacheEntry(
            sha256: "sha256_blob",
            name: "jq",
            version: "1.7.1",
            tag: "arm64_sequoia",
            sizeBytes: 123,
            createdAt: blobs.first?.createdAt ?? 0
        ),
    ])

    try database.deleteBlobCache(connection, sha256: "sha256_blob")
    #expect(try database.listBlobCache(connection).isEmpty)
}

@Test func databaseTracksOrphanedStoreEntriesLikeRust() throws {
    let database = Database(paths: Paths(root: temporaryDirectory()))
    let connection = try database.connect()

    try database.upsertStoreEntry(connection, sha256: "sha256_orphan", sizeBytes: 2000)
    try database.upsertStoreEntry(connection, sha256: "sha256_referenced", sizeBytes: 1000)
    try database.addStoreRef(connection, sha256: "sha256_referenced", package: "jq", version: "1.7.1")

    let orphans = try database.listOrphanedStoreEntries(connection)
    #expect(orphans.count == 1)
    #expect(orphans.first?.sha256 == "sha256_orphan")
    #expect(orphans.first?.sizeBytes == 2000)

    try database.removeStoreRef(connection, sha256: "sha256_referenced", package: "jq", version: "1.7.1")
    #expect(try database.listOrphanedStoreEntries(connection).map(\.sha256) == ["sha256_orphan", "sha256_referenced"])

    try database.deleteStoreEntry(connection, sha256: "sha256_orphan")
    #expect(try database.listOrphanedStoreEntries(connection).map(\.sha256) == ["sha256_referenced"])
}
